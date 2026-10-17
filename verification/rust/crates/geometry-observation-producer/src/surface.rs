use geometry_observation_types::{
    decimal, object, observation, success, AdapterFailure, GeneratedRequestCase, Value,
};
use neco_bigint::{BigUint, Dyadic, RawRational};
use neco_calculus::{
    AbsoluteBits, ConsumerId, Enclosure, ExactDecisionRequest, ExactExpressionRequirement,
    NecoImplementationSource, OwnerId, RationalSurfacePolynomials, SurfacePolynomial,
    SurfaceTerm,
};
use neco_expr::ExprId;
use neco_geometry::{
    evaluate_surface_witness_checked, evaluate_surface_witness_exact, ChartDisplayName,
    ChartDomainId, ParameterInterval, ParametricSurfaceChart2, SurfaceEvaluationWitness,
    SurfaceOrientation, SurfaceParameterDomain,
};

use crate::encode::{
    algebraic_rational, dyadic_value, geometry_failure_observation, integer_algebraic,
    integer_bigint, reduced_rational_value,
};
use crate::projection::{requests_typed_failure, selector, PublicApiInvocation, ProjectedRequest};

fn lineage() -> Result<
    (
        OwnerId,
        ExactExpressionRequirement,
        NecoImplementationSource,
    ),
    AdapterFailure,
> {
    Ok((
        OwnerId::new(1),
        ExactExpressionRequirement::new(
            ConsumerId::new(11),
            &[ExprId::new(20)],
            ExactDecisionRequest::Zero(ExprId::new(20)),
            AbsoluteBits::new(8),
        )
        .map_err(|_error| api_failure(neco_geometry::GeometryFailure::EvaluationIndeterminacy {
            family: neco_geometry::ShapeFamily::ParametricSurface,
            reason: neco_geometry::ShapeEvaluationIndeterminacy::CalculusResult,
        }))?,
        NecoImplementationSource::current(),
    ))
}

pub(crate) fn project_exact(
    case: &GeneratedRequestCase,
) -> Result<PublicApiInvocation, AdapterFailure> {
    Ok(PublicApiInvocation {
        operation: "surface-witness-exact",
        arguments: object([
            (
                "familyParameter",
                decimal(parameter_from_selectors(
                    selector(case, "familyParameter")?,
                    selector(case, "formulaVariant")?,
                    requests_typed_failure(&case.requested_observation_kind),
                )),
            ),
            (
                "formulaVariant",
                decimal(selector(case, "formulaVariant")? % 4),
            ),
            (
                "value",
                decimal(parameter_from_selectors(
                    selector(case, "value")?,
                    selector(case, "formulaVariant")?,
                    requests_typed_failure(&case.requested_observation_kind),
                )),
            ),
        ]),
    })
}

pub(crate) fn observe_exact(projected: &ProjectedRequest) -> Result<Value, AdapterFailure> {
    let u = argument_i64(&projected.invocation.arguments, "value")?;
    let v = argument_i64(&projected.invocation.arguments, "familyParameter")?;
    let variant = argument_u64(&projected.invocation.arguments, "formulaVariant")?;
    let chart = surface_chart(variant)?;
    let domain = parameter_domain()?;
    let input = surface_input_value(&chart, u, v)?;
    let observation_value = match evaluate_surface_witness_exact(
        &chart,
        &domain,
        &integer_algebraic(u)?,
        &integer_algebraic(v)?,
    ) {
        Ok(witness) => success(exact_witness_value(&witness)?) ,
        Err(failure) => geometry_failure_observation(&failure),
    };
    Ok(observation(
        projected.case_id.clone(),
        input,
        observation_value,
        "surface-witness-exact",
    ))
}

pub(crate) fn project_checked(
    case: &GeneratedRequestCase,
) -> Result<PublicApiInvocation, AdapterFailure> {
    Ok(PublicApiInvocation {
        operation: "surface-witness-checked",
        arguments: object([
            (
                "familyParameter",
                decimal(parameter_from_selectors(
                    selector(case, "familyParameter")?,
                    selector(case, "formulaVariant")?,
                    requests_typed_failure(&case.requested_observation_kind),
                )),
            ),
            (
                "formulaVariant",
                decimal(selector(case, "formulaVariant")? % 4),
            ),
            (
                "intervalWidth",
                decimal(selector(case, "intervalWidth")? % 4),
            ),
            (
                "value",
                decimal(parameter_from_selectors(
                    selector(case, "value")?,
                    selector(case, "formulaVariant")?,
                    requests_typed_failure(&case.requested_observation_kind),
                )),
            ),
        ]),
    })
}

pub(crate) fn observe_checked(projected: &ProjectedRequest) -> Result<Value, AdapterFailure> {
    let u = argument_i64(&projected.invocation.arguments, "value")?;
    let v = argument_i64(&projected.invocation.arguments, "familyParameter")?;
    let width = argument_u64(&projected.invocation.arguments, "intervalWidth")?;
    let variant = argument_u64(&projected.invocation.arguments, "formulaVariant")?;
    let chart = surface_chart(variant)?;
    let domain = parameter_domain()?;
    let u_enclosure = parameter_enclosure(u, width)?;
    let v_enclosure = parameter_enclosure(v, width)?;
    let (input_owner, input_requirement, input_source) = lineage()?;
    let input = checked_input_value(
        &chart,
        &u_enclosure,
        &v_enclosure,
        input_owner,
        &input_requirement,
        input_source,
    )?;
    let (owner, requirement, source) = lineage()?;
    let observation_value = match evaluate_surface_witness_checked(
        &chart,
        &domain,
        &u_enclosure,
        &v_enclosure,
        owner,
        requirement,
        source,
    ) {
        Ok(witness) => success(checked_witness_value(&witness)?) ,
        Err(failure) => geometry_failure_observation(&failure),
    };
    Ok(observation(
        projected.case_id.clone(),
        input,
        observation_value,
        "surface-witness-checked",
    ))
}

fn parameter_from_selectors(value: u64, _formula: u64, inject_failure: bool) -> i64 {
    let base = [0, 1, 2, 3][(value % 4) as usize];
    if inject_failure {
        base + 32
    } else {
        base
    }
}

fn parameter_domain() -> Result<SurfaceParameterDomain, AdapterFailure> {
    Ok(SurfaceParameterDomain::new(
        ParameterInterval::new(integer_algebraic(0)?, integer_algebraic(8)?, true, true)
            .map_err(api_failure)?,
        ParameterInterval::new(integer_algebraic(0)?, integer_algebraic(8)?, true, true)
            .map_err(api_failure)?,
    ))
}

fn surface_chart(variant: u64) -> Result<ParametricSurfaceChart2, AdapterFailure> {
    let (scale, denominator) = match variant {
        1 => (2, 2),
        3 => (3, 3),
        _ => (1, 1),
    };
    ParametricSurfaceChart2::try_new(
        ChartDomainId::new("surface-D".to_owned()).map_err(|error| api_failure(error.into()))?,
        ChartDisplayName::new("surface-D".to_owned()).map_err(|error| api_failure(error.into()))?,
        1,
        SurfaceOrientation::Preserve,
        RationalSurfacePolynomials {
            x: monomial(scale, 1, 0)?,
            y: monomial(scale, 0, 1)?,
            z: monomial(scale, 0, 0)?,
            denominator: monomial(denominator, 0, 0)?,
        },
    )
    .map_err(api_failure)
}

fn monomial(coefficient: i32, exponent_u: u32, exponent_v: u32) -> Result<SurfacePolynomial, AdapterFailure> {
    SurfacePolynomial::try_from_terms(vec![SurfaceTerm {
        coefficient: integer_rational(coefficient)?,
        exponent_u,
        exponent_v,
    }])
    .map_err(|error| crate::encode::encode_failure(format!("{error:?}")))
}

fn integer_rational(value: i32) -> Result<neco_bigint::ReducedRational, AdapterFailure> {
    let one = BigUint::one().map_err(|error| crate::encode::encode_failure(format!("{error:?}")))?;
    Ok(RawRational::new(integer_bigint(i64::from(value))?, one)
        .reduce()
        .map_err(|error| crate::encode::encode_failure(format!("{error:?}")))?
        .into_reduced())
}

fn parameter_enclosure(origin: i64, width: u64) -> Result<Enclosure, AdapterFailure> {
    let lower = origin;
    let upper = origin + width as i64;
    Enclosure::new(
        Dyadic::new(integer_bigint(lower)?, 0),
        Dyadic::new(integer_bigint(upper)?, 0),
    )
    .map_err(|error| crate::encode::encode_failure(format!("{error:?}")))
}

fn surface_input_value(
    chart: &ParametricSurfaceChart2,
    u: i64,
    v: i64,
) -> Result<Value, AdapterFailure> {
    Ok(object([
        (
            "denominator",
            surface_polynomial_value(&chart.surface().denominator)?,
        ),
        (
            "identity",
            object([
                (
                    "domainId",
                    Value::String(chart.identity().domain_id().as_str().to_owned()),
                ),
                ("version", decimal(chart.identity().version())),
            ]),
        ),
        ("u", decimal(u)),
        ("v", decimal(v)),
        ("x", surface_polynomial_value(&chart.surface().x)?),
        ("y", surface_polynomial_value(&chart.surface().y)?),
        ("z", surface_polynomial_value(&chart.surface().z)?),
    ]))
}

fn checked_input_value(
    chart: &ParametricSurfaceChart2,
    u: &Enclosure,
    v: &Enclosure,
    owner: OwnerId,
    requirement: &ExactExpressionRequirement,
    source: NecoImplementationSource,
) -> Result<Value, AdapterFailure> {
    Ok(object([
        (
            "denominator",
            surface_polynomial_value(&chart.surface().denominator)?,
        ),
        (
            "identity",
            object([
                (
                    "domainId",
                    Value::String(chart.identity().domain_id().as_str().to_owned()),
                ),
                ("version", decimal(chart.identity().version())),
            ]),
        ),
        (
            "provenance",
            provenance_value(owner, requirement, source),
        ),
        ("u", enclosure_value(u)?),
        ("v", enclosure_value(v)?),
        ("x", surface_polynomial_value(&chart.surface().x)?),
        ("y", surface_polynomial_value(&chart.surface().y)?),
        ("z", surface_polynomial_value(&chart.surface().z)?),
    ]))
}

fn surface_polynomial_value(polynomial: &SurfacePolynomial) -> Result<Value, AdapterFailure> {
    let terms = polynomial
        .terms()
        .iter()
        .map(|term| {
            Ok(object([
                ("coefficient", reduced_rational_value(&term.coefficient)?),
                ("exponentU", decimal(term.exponent_u)),
                ("exponentV", decimal(term.exponent_v)),
            ]))
        })
        .collect::<Result<Vec<_>, AdapterFailure>>()?;
    Ok(Value::Array(terms))
}

fn exact_witness_value(witness: &SurfaceEvaluationWitness) -> Result<Value, AdapterFailure> {
    match witness {
        SurfaceEvaluationWitness::Exact { identity, point } => Ok(object([
            (
                "identity",
                object([
                    (
                        "domainId",
                        Value::String(identity.domain_id().as_str().to_owned()),
                    ),
                    ("version", decimal(identity.version())),
                ]),
            ),
            (
                "point",
                object([
                    ("x", algebraic_rational(&point.x)?),
                    ("y", algebraic_rational(&point.y)?),
                    ("z", algebraic_rational(&point.z)?),
                ]),
            ),
        ])),
        SurfaceEvaluationWitness::Certified { .. } => Err(crate::encode::unexpected_outcome(
            "exact evaluation produced a certified witness",
        )),
    }
}

fn checked_witness_value(witness: &SurfaceEvaluationWitness) -> Result<Value, AdapterFailure> {
    match witness {
        SurfaceEvaluationWitness::Certified {
            identity,
            x,
            y,
            z,
            owner,
            requirement,
            source,
        } => Ok(object([
            ("identity", object([
                (
                    "domainId",
                    Value::String(identity.domain_id().as_str().to_owned()),
                ),
                ("version", decimal(identity.version())),
            ])),
            (
                "provenance",
                provenance_value(*owner, requirement, *source),
            ),
            ("x", enclosure_value(x)?),
            ("y", enclosure_value(y)?),
            ("z", enclosure_value(z)?),
        ])),
        SurfaceEvaluationWitness::Exact { .. } => Err(crate::encode::unexpected_outcome(
            "checked evaluation produced an exact witness",
        )),
    }
}

fn enclosure_value(enclosure: &Enclosure) -> Result<Value, AdapterFailure> {
    Ok(object([
        ("lower", dyadic_value(enclosure.lower())?),
        ("upper", dyadic_value(enclosure.upper())?),
    ]))
}

fn provenance_value(
    owner: OwnerId,
    requirement: &ExactExpressionRequirement,
    source: NecoImplementationSource,
) -> Value {
    object([
        ("consumerId", decimal(requirement.consumer_id().get())),
        ("decision", encode_decision(requirement.decision())),
        (
            "expressions",
            Value::Array(
                requirement
                    .expressions()
                    .iter()
                    .map(|expression| decimal(expression.get()))
                    .collect(),
            ),
        ),
        ("owner", decimal(owner.get())),
        ("precision", decimal(requirement.precision().get())),
        (
            "source",
            object([
                (
                    "publicApiIdentity",
                    Value::String(source.public_api_identity().to_owned()),
                ),
                (
                    "revisionIdentity",
                    Value::String(source.revision_identity().to_owned()),
                ),
            ]),
        ),
    ])
}

fn encode_decision(decision: ExactDecisionRequest) -> Value {
    match decision {
        ExactDecisionRequest::Zero(expression) => object([
            ("expression", decimal(expression.get())),
            ("kind", Value::String("zero".to_owned())),
        ]),
        ExactDecisionRequest::Equality(left, right) => object([
            (
                "expressions",
                Value::Array(vec![decimal(left.get()), decimal(right.get())]),
            ),
            ("kind", Value::String("equality".to_owned())),
        ]),
        ExactDecisionRequest::Sign(expression) => object([
            ("expression", decimal(expression.get())),
            ("kind", Value::String("sign".to_owned())),
        ]),
        ExactDecisionRequest::Degeneracy(expression) => object([
            ("expression", decimal(expression.get())),
            ("kind", Value::String("degeneracy".to_owned())),
        ]),
    }
}

fn argument_i64(value: &Value, field: &str) -> Result<i64, AdapterFailure> {
    value
        .get(field)
        .and_then(Value::as_str)
        .and_then(|value| value.parse().ok())
        .ok_or_else(|| crate::encode::encode_failure(format!("{field} is not a decimal integer")))
}

fn argument_u64(value: &Value, field: &str) -> Result<u64, AdapterFailure> {
    value
        .get(field)
        .and_then(Value::as_str)
        .and_then(|value| value.parse().ok())
        .ok_or_else(|| crate::encode::encode_failure(format!("{field} is not a decimal integer")))
}

fn api_failure(failure: neco_geometry::GeometryFailure) -> AdapterFailure {
    crate::encode::encode_failure(format!("{failure:?}"))
}
