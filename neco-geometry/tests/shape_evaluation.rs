use std::string::String;

use neco_algnum::{AlgnumError, RationalPolynomial, RealAlgebraic};
use neco_bigint::{BigInt, BigUint, RawRational};
use neco_calculus::{
    CalculusFailure, PolynomialFailure, RationalFailure, RationalSurfacePolynomials,
    SurfacePolynomial, SurfaceTerm,
};
use neco_geometry::{
    evaluate_shape, ChartDefinition3, ChartDisplayName, ChartDomainId, CurveChart,
    ExactShapeEvaluation, GeometryFailure, ImplicitSurfaceDeclaration, MultivariatePolynomial3,
    ParameterInterval, ParametricCurveDeclaration, ParametricSurfaceChart2,
    ParametricSurfaceDeclaration, Point3, PolynomialTerm3, ShapeDeclaration, ShapeEvaluation,
    ShapeEvaluationInput, ShapeEvaluationRequest, SurfaceOrientation, SurfaceParameterDomain,
};

fn rational(value: i32) -> neco_bigint::ReducedRational {
    RawRational::new(BigInt::try_from(value).unwrap(), BigUint::one().unwrap())
        .reduce()
        .unwrap()
        .into_reduced()
}

fn polynomial(coefficients: &[i32]) -> RationalPolynomial {
    RationalPolynomial::from_coefficients(coefficients.iter().copied().map(rational).collect())
}

fn algebraic(value: i32) -> RealAlgebraic {
    RealAlgebraic::from_integer(BigInt::try_from(value).unwrap()).unwrap()
}

fn sqrt_two() -> RealAlgebraic {
    algebraic(2).nth_root(2).unwrap()
}

fn term(
    coefficient: RealAlgebraic,
    exponent_x: u32,
    exponent_y: u32,
    exponent_z: u32,
) -> PolynomialTerm3 {
    PolynomialTerm3::new(coefficient, exponent_x, exponent_y, exponent_z).unwrap()
}

fn integer_term(
    coefficient: i32,
    exponent_x: u32,
    exponent_y: u32,
    exponent_z: u32,
) -> PolynomialTerm3 {
    term(algebraic(coefficient), exponent_x, exponent_y, exponent_z)
}

fn interval(lower: i32, upper: i32) -> ParameterInterval {
    ParameterInterval::new(algebraic(lower), algebraic(upper), true, true).unwrap()
}

fn surface_polynomial(coefficients: &[i32]) -> SurfacePolynomial {
    let terms = coefficients
        .iter()
        .enumerate()
        .filter(|(_, coefficient)| **coefficient != 0)
        .map(|(exponent_u, coefficient)| SurfaceTerm {
            coefficient: rational(*coefficient),
            exponent_u: u32::try_from(exponent_u).unwrap(),
            exponent_v: 0,
        })
        .collect();
    SurfacePolynomial::try_from_terms(terms).unwrap()
}

fn surface_chart(
    numerator_x: &[i32],
    numerator_y: &[i32],
    numerator_z: &[i32],
    denominator: &[i32],
) -> ParametricSurfaceChart2 {
    ParametricSurfaceChart2::try_new(
        ChartDomainId::new(String::from("D")).unwrap(),
        ChartDisplayName::new(String::from("chart D")).unwrap(),
        1,
        SurfaceOrientation::Preserve,
        RationalSurfacePolynomials {
            x: surface_polynomial(numerator_x),
            y: surface_polynomial(numerator_y),
            z: surface_polynomial(numerator_z),
            denominator: surface_polynomial(denominator),
        },
    )
    .unwrap()
}

fn chart(
    numerator_x: &[i32],
    numerator_y: &[i32],
    numerator_z: &[i32],
    denominator: &[i32],
) -> ChartDefinition3 {
    ChartDefinition3::new(
        ChartDomainId::new(String::from("D")).unwrap(),
        ChartDisplayName::new(String::from("chart D")).unwrap(),
        polynomial(&[]),
        polynomial(numerator_x),
        polynomial(numerator_y),
        polynomial(numerator_z),
        polynomial(denominator),
    )
    .unwrap()
}

fn implicit_request(terms: Vec<PolynomialTerm3>, point: Point3) -> ShapeEvaluationRequest {
    ShapeEvaluationRequest::new(
        ShapeDeclaration::ImplicitSurface(ImplicitSurfaceDeclaration::new(
            MultivariatePolynomial3::try_from_terms(terms).unwrap(),
        )),
        ShapeEvaluationInput::ImplicitSurface { point },
    )
}

fn point(x: i32, y: i32, z: i32) -> Point3 {
    Point3 {
        x: algebraic(x),
        y: algebraic(y),
        z: algebraic(z),
    }
}

#[test]
fn preserves_non_rational_implicit_coefficient() {
    let coefficient = sqrt_two();
    let expected = coefficient.mul(&algebraic(2)).unwrap();
    let request = implicit_request(vec![term(coefficient, 1, 0, 0)], point(2, 1, 1));

    assert_eq!(
        evaluate_shape(request),
        Ok(ShapeEvaluation::Exact(
            ExactShapeEvaluation::ImplicitSurface { scalar: expected },
        ))
    );
}

#[test]
fn implicit_term_permutation_has_the_same_public_result() {
    let first = implicit_request(
        vec![
            integer_term(2, 1, 0, 0),
            integer_term(3, 0, 1, 0),
            integer_term(5, 0, 0, 1),
        ],
        point(2, 3, 5),
    );
    let second = implicit_request(
        vec![
            integer_term(5, 0, 0, 1),
            integer_term(2, 1, 0, 0),
            integer_term(3, 0, 1, 0),
        ],
        point(2, 3, 5),
    );

    assert_eq!(evaluate_shape(first), evaluate_shape(second));
}

#[test]
fn preserves_family_and_denominator_failure_payloads() {
    let family_mismatch = evaluate_shape(ShapeEvaluationRequest::new(
        ShapeDeclaration::ImplicitSurface(ImplicitSurfaceDeclaration::new(
            MultivariatePolynomial3::zero(),
        )),
        ShapeEvaluationInput::ParametricCurve {
            parameter: algebraic(0),
        },
    ));
    assert_eq!(family_mismatch, Err(GeometryFailure::ShapeFamilyMismatch));

    let denominator_failure = evaluate_shape(ShapeEvaluationRequest::new(
        ShapeDeclaration::ParametricSurface(ParametricSurfaceDeclaration::new(
            surface_chart(&[1], &[1], &[1], &[1, -1]),
            SurfaceParameterDomain::new(interval(0, 1), interval(0, 1)),
        )),
        ShapeEvaluationInput::ParametricSurface {
            u: algebraic(1),
            v: algebraic(1),
        },
    ));
    assert_eq!(
        denominator_failure,
        Err(GeometryFailure::DenominatorValidity {
            error: CalculusFailure::Rational(RationalFailure::UndefinedDomain),
        })
    );

    let curve_denominator_failure = evaluate_shape(ShapeEvaluationRequest::new(
        ShapeDeclaration::ParametricCurve(ParametricCurveDeclaration::new(
            CurveChart::Chart3(chart(&[1], &[1], &[1], &[1, -1])),
            interval(0, 1),
        )),
        ShapeEvaluationInput::ParametricCurve {
            parameter: algebraic(1),
        },
    ));
    assert_eq!(
        curve_denominator_failure,
        Err(GeometryFailure::DenominatorValidity {
            error: CalculusFailure::Polynomial(PolynomialFailure::Algebra(
                AlgnumError::DivisionByZero,
            )),
        })
    );
}

#[test]
fn preserves_non_denominator_calculus_failure_payload() {
    let request = implicit_request(vec![integer_term(1, 0, 0, 0)], point(0, 1, 1));

    assert_eq!(
        evaluate_shape(request),
        Err(GeometryFailure::CalculusFailure {
            error: CalculusFailure::Algebra(AlgnumError::UndefinedZeroPower),
        })
    );
}

#[test]
fn evaluate_shape_is_pure_and_repeatable() {
    let first = evaluate_shape(implicit_request(
        vec![integer_term(7, 1, 0, 0)],
        point(3, 0, 0),
    ));
    let second = evaluate_shape(implicit_request(
        vec![integer_term(7, 1, 0, 0)],
        point(3, 0, 0),
    ));

    assert_eq!(first, second);
}

#[test]
fn evaluates_curve_surface_and_implicit_families() {
    let curve = evaluate_shape(ShapeEvaluationRequest::new(
        ShapeDeclaration::ParametricCurve(ParametricCurveDeclaration::new(
            CurveChart::Chart3(chart(&[0, 1], &[2], &[3], &[1])),
            interval(0, 1),
        )),
        ShapeEvaluationInput::ParametricCurve {
            parameter: algebraic(1),
        },
    ));
    assert_eq!(
        curve,
        Ok(ShapeEvaluation::Exact(
            ExactShapeEvaluation::ParametricCurve {
                coordinate: neco_geometry::ShapeCoordinate::Spatial(point(1, 2, 3)),
            },
        ))
    );

    let surface = evaluate_shape(ShapeEvaluationRequest::new(
        ShapeDeclaration::ParametricSurface(ParametricSurfaceDeclaration::new(
            surface_chart(&[1], &[2], &[3], &[1]),
            SurfaceParameterDomain::new(interval(0, 1), interval(0, 1)),
        )),
        ShapeEvaluationInput::ParametricSurface {
            u: algebraic(1),
            v: algebraic(1),
        },
    ));
    assert_eq!(
        surface,
        Ok(ShapeEvaluation::Exact(
            ExactShapeEvaluation::ParametricSurface {
                coordinate: point(1, 2, 3),
            },
        ))
    );

    let implicit = evaluate_shape(implicit_request(
        vec![integer_term(2, 1, 0, 0)],
        point(3, 1, 1),
    ));
    assert_eq!(
        implicit,
        Ok(ShapeEvaluation::Exact(
            ExactShapeEvaluation::ImplicitSurface {
                scalar: algebraic(6),
            },
        ))
    );
}

#[test]
fn invalid_zero_and_duplicate_terms_fail_at_public_constructors() {
    assert!(PolynomialTerm3::new(algebraic(0), 0, 0, 0).is_err());

    let duplicate = MultivariatePolynomial3::try_from_terms(vec![
        integer_term(1, 1, 0, 0),
        integer_term(2, 1, 0, 0),
    ]);
    assert!(duplicate.is_err());
}

#[test]
fn public_constructors_preserve_canonical_term_order() {
    let equation = MultivariatePolynomial3::try_from_terms(vec![
        integer_term(2, 1, 0, 0),
        integer_term(3, 0, 1, 0),
    ])
    .unwrap();

    assert_eq!(
        equation
            .terms()
            .iter()
            .map(|term| (term.exponent_x(), term.exponent_y(), term.exponent_z()))
            .collect::<Vec<_>>(),
        vec![(0, 1, 0), (1, 0, 0)]
    );
}

#[test]
fn public_chart_composition_preserves_coordinates_and_domain_identity() {
    let outer = ChartDefinition3::new(
        ChartDomainId::new(String::from("D")).unwrap(),
        ChartDisplayName::new(String::from("outer")).unwrap(),
        polynomial(&[1, 1]),
        polynomial(&[1, 1]),
        polynomial(&[2]),
        polynomial(&[3]),
        polynomial(&[1]),
    )
    .unwrap();
    let inner = ChartDefinition3::new(
        ChartDomainId::new(String::from("D")).unwrap(),
        ChartDisplayName::new(String::from("inner")).unwrap(),
        polynomial(&[1, 1]),
        polynomial(&[1]),
        polynomial(&[1]),
        polynomial(&[1]),
        polynomial(&[1]),
    )
    .unwrap();
    let composed = outer.compose(&inner).unwrap();
    assert_eq!(composed.domain_id().as_str(), "D");
    assert_eq!(composed.display_name().as_str(), "outer");
    assert_eq!(composed.numerator_x(), &polynomial(&[2, 1]));

    let other = ChartDefinition3::new(
        ChartDomainId::new(String::from("other")).unwrap(),
        ChartDisplayName::new(String::from("other")).unwrap(),
        polynomial(&[1]),
        polynomial(&[1]),
        polynomial(&[1]),
        polynomial(&[1]),
        polynomial(&[1]),
    )
    .unwrap();
    assert_eq!(
        outer.compose(&other),
        Err(neco_geometry_foundation::GeometryFailure::ChartDomainMismatch)
    );
}
