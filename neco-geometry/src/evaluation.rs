use alloc::boxed::Box;
use alloc::vec::Vec;

use neco_algnum::{AlgnumError, RealAlgebraic};
use neco_bigint::BigintError;
use neco_calculus::{
    evaluate_implicit, evaluate_rational_polynomial, evaluate_surface, evaluate_surface_checked,
    CalculusFailure, Enclosure, EvaluationFailure, ExactExpressionRequirement,
    ImplicitEvaluationInput, ImplicitEvaluationProducer, NecoImplementationSource, OwnerId,
    PolynomialFailure, RationalPolynomialInput, RationalSurfacePolynomials, SurfaceEvaluationInput,
    SurfaceEvaluationInput2, SurfaceEvaluationProducer, SurfacePolynomial, SurfaceTerm,
};
use neco_geometry_foundation::{
    ChartDefinition2, ChartDefinition3, CheckedRay2, CheckedRay3, Point2, Point3, Ray2, Ray3,
};

use crate::types::{
    ExactShapeEvaluation, GeometryFailure, MultivariatePolynomial3, ParametricSurfaceChart2,
    ShapeCoordinate, ShapeDeclaration, ShapeEvaluation, ShapeEvaluationIndeterminacy,
    ShapeEvaluationInput, ShapeEvaluationRequest, ShapeFamily, SurfaceChartIdentity,
    SurfaceEvaluationWitness, SurfaceParameterDomain,
};

pub fn evaluate_ray2(
    ray: &Ray2,
    parameter: &RealAlgebraic,
    range: &crate::types::Ray2ParameterRange,
) -> Result<crate::types::ExactRayEvaluation2, GeometryFailure> {
    if parameter.compare(&crate::intersection::dyadic_to_real(range.lower())?)?
        == core::cmp::Ordering::Less
        || parameter.compare(&crate::intersection::dyadic_to_real(range.upper())?)?
            == core::cmp::Ordering::Greater
    {
        return Err(GeometryFailure::ParameterOutsideDomain);
    }
    let x = ray
        .origin()
        .x
        .add(&ray.direction().vector().x.mul(parameter)?)?;
    let y = ray
        .origin()
        .y
        .add(&ray.direction().vector().y.mul(parameter)?)?;
    Ok(crate::types::ExactRayEvaluation2::Point(Point2 { x, y }))
}

pub fn evaluate_ray3(
    ray: &Ray3,
    parameter: &RealAlgebraic,
    range: &crate::types::PositiveRayParameterRange,
) -> Result<crate::types::ExactRayEvaluation3, GeometryFailure> {
    if !range.contains_exact(parameter)? {
        return Err(GeometryFailure::ParameterOutsideDomain);
    }
    let x = ray
        .origin()
        .x
        .add(&ray.direction().vector().x.mul(parameter)?)?;
    let y = ray
        .origin()
        .y
        .add(&ray.direction().vector().y.mul(parameter)?)?;
    let z = ray
        .origin()
        .z
        .add(&ray.direction().vector().z.mul(parameter)?)?;
    Ok(crate::types::ExactRayEvaluation3::Point(Point3 { x, y, z }))
}

pub fn evaluate_checked_ray2(
    ray: &CheckedRay2,
    parameter: f64,
    range: &crate::types::CheckedRay2ParameterRange,
) -> Result<crate::types::CheckedRayEvaluation2, GeometryFailure> {
    if !parameter.is_finite() || parameter < range.lower() || parameter > range.upper() {
        return Err(GeometryFailure::ParameterOutsideDomain);
    }
    let point = [
        ray.origin()[0] + parameter * ray.direction()[0],
        ray.origin()[1] + parameter * ray.direction()[1],
    ];
    if point.iter().all(|value| value.is_finite()) {
        Ok(crate::types::CheckedRayEvaluation2::Candidate(point))
    } else {
        Ok(crate::types::CheckedRayEvaluation2::Undetermined)
    }
}

pub fn evaluate_checked_ray3(
    ray: &CheckedRay3,
    parameter: f64,
    range: &crate::types::PositiveRayParameterRange,
) -> Result<crate::types::CheckedRayEvaluation3, GeometryFailure> {
    if !range.contains_checked(parameter)? {
        return Err(GeometryFailure::ParameterOutsideDomain);
    }
    let point = [
        ray.origin()[0] + parameter * ray.direction()[0],
        ray.origin()[1] + parameter * ray.direction()[1],
        ray.origin()[2] + parameter * ray.direction()[2],
    ];
    if point.iter().all(|value| value.is_finite()) {
        Ok(crate::types::CheckedRayEvaluation3::Candidate(point))
    } else {
        Ok(crate::types::CheckedRayEvaluation3::Undetermined)
    }
}

pub fn evaluate_shape(request: ShapeEvaluationRequest) -> Result<ShapeEvaluation, GeometryFailure> {
    if request.declaration().family() != request.input().family() {
        return Err(GeometryFailure::ShapeFamilyMismatch);
    }
    match (request.declaration(), request.input()) {
        (
            ShapeDeclaration::ParametricCurve(declaration),
            ShapeEvaluationInput::ParametricCurve { parameter },
        ) => {
            if !declaration.domain().contains(parameter)? {
                return Err(GeometryFailure::ParameterOutsideDomain);
            }
            let coordinate = match declaration.chart() {
                crate::types::CurveChart::Chart2(chart) => ShapeCoordinate::Planar(Point2 {
                    x: eval_chart2(chart, parameter, false)?,
                    y: eval_chart2(chart, parameter, true)?,
                }),
                crate::types::CurveChart::Chart3(chart) => ShapeCoordinate::Spatial(Point3 {
                    x: eval_chart3(chart, parameter, 0)?,
                    y: eval_chart3(chart, parameter, 1)?,
                    z: eval_chart3(chart, parameter, 2)?,
                }),
            };
            Ok(ShapeEvaluation::Exact(
                ExactShapeEvaluation::ParametricCurve { coordinate },
            ))
        }
        (
            ShapeDeclaration::ParametricSurface(declaration),
            ShapeEvaluationInput::ParametricSurface { u, v },
        ) => {
            if !declaration.domain().contains(u, v)? {
                return Err(GeometryFailure::ParameterOutsideDomain);
            }
            match evaluate_surface(surface_input(declaration.chart(), u, v)?)
                .map_err(map_calculus_failure)?
            {
                SurfaceEvaluationProducer::Exact { x, y, z } => Ok(ShapeEvaluation::Exact(
                    ExactShapeEvaluation::ParametricSurface {
                        coordinate: Point3 { x, y, z },
                    },
                )),
                SurfaceEvaluationProducer::Certified { coordinates } => {
                    Ok(ShapeEvaluation::CertifiedEnclosure(
                        crate::types::CertifiedShapeEvaluation::ParametricSurface {
                            coordinate: Box::new(crate::types::CoordinateEnclosure3 {
                                x: clone_enclosure(coordinates.x())?,
                                y: clone_enclosure(coordinates.y())?,
                                z: clone_enclosure(coordinates.z())?,
                            }),
                            owner: coordinates.owner(),
                            requirement: clone_requirement(coordinates.requirement())?,
                            source: coordinates.source(),
                        },
                    ))
                }
            }
        }
        (
            ShapeDeclaration::ImplicitSurface(declaration),
            ShapeEvaluationInput::ImplicitSurface { point },
        ) => {
            let equation = transport_implicit_equation(declaration.equation())?;
            match evaluate_implicit(ImplicitEvaluationInput {
                equation,
                x: point.x.try_clone().map_err(map_algebra_failure)?,
                y: point.y.try_clone().map_err(map_algebra_failure)?,
                z: point.z.try_clone().map_err(map_algebra_failure)?,
            })
            .map_err(map_calculus_failure)?
            {
                ImplicitEvaluationProducer::Exact { scalar } => Ok(ShapeEvaluation::Exact(
                    ExactShapeEvaluation::ImplicitSurface { scalar },
                )),
            }
        }
        _ => Err(GeometryFailure::ShapeFamilyMismatch),
    }
}

fn eval_chart2(
    chart: &ChartDefinition2,
    point: &RealAlgebraic,
    second: bool,
) -> Result<RealAlgebraic, GeometryFailure> {
    let polynomial = if second {
        chart.numerator_y()
    } else {
        chart.numerator_x()
    };
    let numerator = evaluate_rational_polynomial(RationalPolynomialInput {
        polynomial: polynomial.try_clone().map_err(map_algebra_failure)?,
        point: point.try_clone().map_err(map_algebra_failure)?,
    })
    .map_err(map_calculus_failure)?
    .into_value();
    let denominator = evaluate_rational_polynomial(RationalPolynomialInput {
        polynomial: chart
            .denominator()
            .try_clone()
            .map_err(map_algebra_failure)?,
        point: point.try_clone().map_err(map_algebra_failure)?,
    })
    .map_err(map_calculus_failure)?
    .into_value();
    divide_chart_values(numerator, denominator)
}

fn eval_chart3(
    chart: &ChartDefinition3,
    point: &RealAlgebraic,
    index: usize,
) -> Result<RealAlgebraic, GeometryFailure> {
    let polynomial = match index {
        0 => chart.numerator_x(),
        1 => chart.numerator_y(),
        _ => chart.numerator_z(),
    };
    let numerator = evaluate_rational_polynomial(RationalPolynomialInput {
        polynomial: polynomial.try_clone().map_err(map_algebra_failure)?,
        point: point.try_clone().map_err(map_algebra_failure)?,
    })
    .map_err(map_calculus_failure)?
    .into_value();
    let denominator = evaluate_rational_polynomial(RationalPolynomialInput {
        polynomial: chart
            .denominator()
            .try_clone()
            .map_err(map_algebra_failure)?,
        point: point.try_clone().map_err(map_algebra_failure)?,
    })
    .map_err(map_calculus_failure)?
    .into_value();
    divide_chart_values(numerator, denominator)
}

fn divide_chart_values(
    numerator: RealAlgebraic,
    denominator: RealAlgebraic,
) -> Result<RealAlgebraic, GeometryFailure> {
    numerator.div(&denominator).map_err(|error| match error {
        AlgnumError::DivisionByZero => GeometryFailure::DenominatorValidity {
            error: CalculusFailure::Polynomial(PolynomialFailure::Algebra(error)),
        },
        error => GeometryFailure::AlgebraFailure(error),
    })
}

fn transport_implicit_equation(
    equation: &MultivariatePolynomial3,
) -> Result<neco_calculus::ImplicitPolynomial3, GeometryFailure> {
    let terms = equation
        .terms()
        .iter()
        .map(|term| {
            Ok(neco_calculus::ImplicitTerm {
                coefficient: term
                    .coefficient()
                    .try_clone()
                    .map_err(map_algebra_failure)?,
                exponent_x: term.exponent_x(),
                exponent_y: term.exponent_y(),
                exponent_z: term.exponent_z(),
            })
        })
        .collect::<Result<Vec<_>, GeometryFailure>>()?;
    neco_calculus::ImplicitPolynomial3::try_from_terms(terms).map_err(map_calculus_failure)
}

fn surface_input(
    chart: &ParametricSurfaceChart2,
    u: &RealAlgebraic,
    v: &RealAlgebraic,
) -> Result<SurfaceEvaluationInput, GeometryFailure> {
    Ok(SurfaceEvaluationInput {
        x: clone_surface_polynomial(&chart.surface().x)?,
        y: clone_surface_polynomial(&chart.surface().y)?,
        z: clone_surface_polynomial(&chart.surface().z)?,
        denominator: clone_surface_polynomial(&chart.surface().denominator)?,
        u: u.try_clone().map_err(map_algebra_failure)?,
        v: v.try_clone().map_err(map_algebra_failure)?,
    })
}

pub fn evaluate_surface_witness_exact(
    chart: &ParametricSurfaceChart2,
    domain: &SurfaceParameterDomain,
    u: &RealAlgebraic,
    v: &RealAlgebraic,
) -> Result<SurfaceEvaluationWitness, GeometryFailure> {
    if !domain.contains(u, v)? {
        return Err(GeometryFailure::ParameterOutsideDomain);
    }
    match evaluate_surface(surface_input(chart, u, v)?).map_err(map_calculus_failure)? {
        SurfaceEvaluationProducer::Exact { x, y, z } => Ok(SurfaceEvaluationWitness::Exact {
            identity: SurfaceChartIdentity::new(
                chart.identity().domain_id().try_clone()?,
                chart.identity().version(),
            ),
            point: Point3 { x, y, z },
        }),
        SurfaceEvaluationProducer::Certified { .. } => {
            Err(GeometryFailure::EvaluationIndeterminacy {
                family: ShapeFamily::ParametricSurface,
                reason: ShapeEvaluationIndeterminacy::CalculusResult,
            })
        }
    }
}

pub fn evaluate_surface_witness_checked(
    chart: &ParametricSurfaceChart2,
    domain: &SurfaceParameterDomain,
    u: &Enclosure,
    v: &Enclosure,
    owner: OwnerId,
    requirement: ExactExpressionRequirement,
    source: NecoImplementationSource,
) -> Result<SurfaceEvaluationWitness, GeometryFailure> {
    let u_lower = crate::intersection::dyadic_to_real(u.lower())?;
    let u_upper = crate::intersection::dyadic_to_real(u.upper())?;
    let v_lower = crate::intersection::dyadic_to_real(v.lower())?;
    let v_upper = crate::intersection::dyadic_to_real(v.upper())?;
    if !domain.contains(&u_lower, &v_lower)?
        || !domain.contains(&u_upper, &v_upper)?
        || !domain.contains(&u_lower, &v_upper)?
        || !domain.contains(&u_upper, &v_lower)?
    {
        return Err(GeometryFailure::ParameterOutsideDomain);
    }
    match evaluate_surface_checked(
        SurfaceEvaluationInput2::new(
            clone_rational_surface(chart.surface())?,
            clone_enclosure(u)?,
            clone_enclosure(v)?,
            owner,
            requirement,
            source,
        )
        .map_err(map_calculus_failure)?,
    )
    .map_err(map_calculus_failure)?
    {
        SurfaceEvaluationProducer::Certified { coordinates } => {
            Ok(SurfaceEvaluationWitness::Certified {
                identity: SurfaceChartIdentity::new(
                    chart.identity().domain_id().try_clone()?,
                    chart.identity().version(),
                ),
                x: clone_enclosure(coordinates.x())?,
                y: clone_enclosure(coordinates.y())?,
                z: Box::new(clone_enclosure(coordinates.z())?),
                owner: coordinates.owner(),
                requirement: clone_requirement(coordinates.requirement())?,
                source: coordinates.source(),
            })
        }
        SurfaceEvaluationProducer::Exact { .. } => Err(GeometryFailure::EvaluationIndeterminacy {
            family: ShapeFamily::ParametricSurface,
            reason: ShapeEvaluationIndeterminacy::CalculusResult,
        }),
    }
}

pub(crate) fn clone_rational_surface(
    surface: &RationalSurfacePolynomials,
) -> Result<RationalSurfacePolynomials, GeometryFailure> {
    Ok(RationalSurfacePolynomials {
        x: clone_surface_polynomial(&surface.x)?,
        y: clone_surface_polynomial(&surface.y)?,
        z: clone_surface_polynomial(&surface.z)?,
        denominator: clone_surface_polynomial(&surface.denominator)?,
    })
}

fn clone_enclosure(enclosure: &Enclosure) -> Result<Enclosure, GeometryFailure> {
    Enclosure::new(
        enclosure.lower().try_clone().map_err(map_bigint_failure)?,
        enclosure.upper().try_clone().map_err(map_bigint_failure)?,
    )
    .map_err(|error| GeometryFailure::CalculusFailure {
        error: CalculusFailure::Interval(error),
    })
}

fn clone_requirement(
    requirement: &ExactExpressionRequirement,
) -> Result<ExactExpressionRequirement, GeometryFailure> {
    ExactExpressionRequirement::new(
        requirement.consumer_id(),
        requirement.expressions(),
        requirement.decision(),
        requirement.precision(),
    )
    .map_err(|_| GeometryFailure::EvaluationIndeterminacy {
        family: ShapeFamily::ParametricSurface,
        reason: ShapeEvaluationIndeterminacy::CalculusResult,
    })
}

fn clone_surface_polynomial(
    polynomial: &SurfacePolynomial,
) -> Result<SurfacePolynomial, GeometryFailure> {
    let terms = polynomial
        .terms()
        .iter()
        .map(|term| {
            Ok(SurfaceTerm {
                coefficient: term.coefficient.try_clone().map_err(map_bigint_failure)?,
                exponent_u: term.exponent_u,
                exponent_v: term.exponent_v,
            })
        })
        .collect::<Result<Vec<_>, GeometryFailure>>()?;
    SurfacePolynomial::try_from_terms(terms).map_err(map_calculus_failure)
}

fn map_bigint_failure(_error: BigintError) -> GeometryFailure {
    GeometryFailure::EvaluationIndeterminacy {
        family: ShapeFamily::ImplicitSurface,
        reason: ShapeEvaluationIndeterminacy::CalculusResult,
    }
}

fn map_algebra_failure(error: AlgnumError) -> GeometryFailure {
    GeometryFailure::AlgebraFailure(error)
}
fn map_calculus_failure(error: CalculusFailure) -> GeometryFailure {
    let denominator_validity_failure = match &error {
        CalculusFailure::Evaluation(failure) => {
            matches!(**failure, EvaluationFailure::DomainViolation { .. })
        }
        CalculusFailure::Rational(failure) => matches!(
            failure,
            neco_calculus::RationalFailure::ZeroDenominatorPolynomial
                | neco_calculus::RationalFailure::UndefinedDomain
                | neco_calculus::RationalFailure::DomainContainsZero
        ),
        _ => false,
    };
    if denominator_validity_failure {
        GeometryFailure::DenominatorValidity { error }
    } else {
        GeometryFailure::CalculusFailure { error }
    }
}
