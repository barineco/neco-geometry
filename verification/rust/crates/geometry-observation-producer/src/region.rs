use geometry_observation_types::{
    decimal, object, observation, success, AdapterFailure, GeneratedRequestCase, Value,
};
use neco_geometry::{
    AlgebraicRegion, CanonicalAtom, MembershipClassification, NormalizedRegionFormula, Point2,
    Point3, RegionAffineMap3, RegionFunction, RegionKind, RegionMembershipRequest, RegionPoint,
    RegionPolynomial3, RegionPolynomialTerm, SignRelation, MAX_REGION_POLYNOMIAL_TERMS,
};

use crate::encode::{algebraic_rational, geometry_failure_observation, integer_algebraic};
use crate::projection::{requests_typed_failure, selector, PublicApiInvocation, ProjectedRequest};

pub(crate) fn project_normalization(
    case: &GeneratedRequestCase,
) -> Result<PublicApiInvocation, AdapterFailure> {
    Ok(PublicApiInvocation {
        operation: "region-polynomial-normalization",
        arguments: object([
            (
                "coefficient",
                decimal(coefficient_from_value(selector(case, "value")?)),
            ),
            (
                "formulaVariant",
                decimal(selector(case, "formulaVariant")? % 4),
            ),
        ]),
    })
}

pub(crate) fn observe_normalization(projected: &ProjectedRequest) -> Result<Value, AdapterFailure> {
    let coefficient = argument_i64(&projected.invocation.arguments, "coefficient")?;
    let formula = argument_u64(&projected.invocation.arguments, "formulaVariant")?;
    let terms = raw_polynomial_terms(
        coefficient,
        formula,
        requests_typed_failure(&projected.requested_observation_kind),
    )?;
    let input = object([("rawTerms", polynomial_terms_slice(&terms)?)]);
    let observation_value = match RegionPolynomial3::try_from_terms(terms) {
        Ok(polynomial) => success(object([("terms", polynomial_terms_value(&polynomial)?)])) ,
        Err(failure) => geometry_failure_observation(&failure),
    };
    Ok(observation(
        projected.case_id.clone(),
        input,
        observation_value,
        "region-polynomial-normalization",
    ))
}

pub(crate) fn project_affine(
    case: &GeneratedRequestCase,
) -> Result<PublicApiInvocation, AdapterFailure> {
    Ok(PublicApiInvocation {
        operation: "region-affine-substitution",
        arguments: object([
            (
                "familyParameter",
                decimal(selector(case, "familyParameter")? % 4),
            ),
            (
                "formulaVariant",
                decimal(selector(case, "formulaVariant")? % 4),
            ),
            (
                "value",
                decimal(coefficient_from_value(selector(case, "value")?)),
            ),
        ]),
    })
}

pub(crate) fn observe_affine(projected: &ProjectedRequest) -> Result<Value, AdapterFailure> {
    let map_kind = argument_u64(&projected.invocation.arguments, "familyParameter")?;
    let source_kind = argument_u64(&projected.invocation.arguments, "formulaVariant")?;
    let translation = argument_i64(&projected.invocation.arguments, "value")?;
    let inject_failure = requests_typed_failure(&projected.requested_observation_kind);
    let source_terms = affine_source_terms(source_kind, translation, inject_failure)?;
    let map_terms = affine_map_raw(map_kind, translation, inject_failure)?;
    let input = object([
        (
            "map",
            object([
                ("x", polynomial_terms_slice(&map_terms.0)?),
                ("y", polynomial_terms_slice(&map_terms.1)?),
                ("z", polynomial_terms_slice(&map_terms.2)?),
            ]),
        ),
        ("source", object([("terms", polynomial_terms_slice(&source_terms)?)])) ,
    ]);
    let observation_value = match RegionPolynomial3::try_from_terms(source_terms).and_then(|source| {
        RegionAffineMap3::try_new(
            RegionPolynomial3::try_from_terms(map_terms.0)?,
            RegionPolynomial3::try_from_terms(map_terms.1)?,
            RegionPolynomial3::try_from_terms(map_terms.2)?,
        )
        .and_then(|map| source.try_affine_substitute(&map))
    }) {
        Ok(polynomial) => success(object([("terms", polynomial_terms_value(&polynomial)?)])) ,
        Err(failure) => geometry_failure_observation(&failure),
    };
    Ok(observation(
        projected.case_id.clone(),
        input,
        observation_value,
        "region-affine-substitution",
    ))
}

pub(crate) fn project_formula(
    case: &GeneratedRequestCase,
) -> Result<PublicApiInvocation, AdapterFailure> {
    Ok(PublicApiInvocation {
        operation: "region-formula-construction",
        arguments: object([(
            "formulaVariant",
            decimal(selector(case, "formulaVariant")? % 4),
        )]),
    })
}

pub(crate) fn observe_formula(projected: &ProjectedRequest) -> Result<Value, AdapterFailure> {
    let variant = argument_u64(&projected.invocation.arguments, "formulaVariant")?;
    let (atoms, formula, input) = formula_configuration(
        variant,
        requests_typed_failure(&projected.requested_observation_kind),
    )?;
    let observation_value = match AlgebraicRegion::try_new(
        3,
        atoms,
        formula,
        RegionKind::GeneralSemiAlgebraicSet,
    ) {
        Ok(region) => success(object([(
            "formula",
            formula_value(region.normalized_region_formula())?,
        )])),
        Err(failure) => geometry_failure_observation(&failure),
    };
    Ok(observation(
        projected.case_id.clone(),
        input,
        observation_value,
        "region-formula-construction",
    ))
}

pub(crate) fn project_membership(
    case: &GeneratedRequestCase,
) -> Result<PublicApiInvocation, AdapterFailure> {
    Ok(PublicApiInvocation {
        operation: "region-membership",
        arguments: object([
            (
                "familyParameter",
                decimal(signed_coordinate(selector(case, "familyParameter")?)),
            ),
            (
                "formulaVariant",
                decimal(selector(case, "formulaVariant")? % 4),
            ),
            ("sign", decimal(signed_coordinate(selector(case, "sign")?))),
            ("value", decimal(signed_coordinate(selector(case, "value")?))),
        ]),
    })
}

pub(crate) fn observe_membership(projected: &ProjectedRequest) -> Result<Value, AdapterFailure> {
    let x = argument_i64(&projected.invocation.arguments, "value")?;
    let y = argument_i64(&projected.invocation.arguments, "familyParameter")?;
    let z = argument_i64(&projected.invocation.arguments, "sign")?;
    let variant = argument_u64(&projected.invocation.arguments, "formulaVariant")?;
    let region = AlgebraicRegion::try_new(
        3,
        vec![polynomial_atom("a", 1, 1, 0, 0)?],
        NormalizedRegionFormula::Atom("a".to_owned()),
        RegionKind::GeneralSemiAlgebraicSet,
    );
    let request_region = AlgebraicRegion::try_new(
        3,
        vec![polynomial_atom("a", 1, 1, 0, 0)?],
        NormalizedRegionFormula::Atom("a".to_owned()),
        RegionKind::GeneralSemiAlgebraicSet,
    );
    let point = if requests_typed_failure(&projected.requested_observation_kind) {
        RegionPoint::Planar(Point2 {
            x: integer_algebraic(x)?,
            y: integer_algebraic(y)?,
        })
    } else {
        RegionPoint::Spatial(Point3 {
            x: integer_algebraic(x)?,
            y: integer_algebraic(y)?,
            z: integer_algebraic(z)?,
        })
    };
    let input = object([
        ("formulaVariant", decimal(variant)),
        ("x", decimal(x)),
        ("y", decimal(y)),
        ("z", decimal(z)),
    ]);
    let observation_value = match (region, request_region) {
        (Ok(region), Ok(request_region)) => {
            match region.membership(RegionMembershipRequest {
                region: request_region,
                point,
            }) {
                Ok(result) => success(object([
                    (
                        "atomSigns",
                        Value::Array(
                            result
                                .atom_signs
                                .iter()
                                .map(|sign| {
                                    object([
                                        ("atomId", Value::String(sign.atom_id.clone())),
                                        (
                                            "relation",
                                            Value::String(sign_name(sign.relation).to_owned()),
                                        ),
                                    ])
                                })
                                .collect(),
                        ),
                    ),
                    (
                        "classification",
                        Value::String(classification_name(result.classification).to_owned()),
                    ),
                ])),
                Err(failure) => geometry_failure_observation(&failure),
            }
        }
        (Err(failure), _) | (_, Err(failure)) => geometry_failure_observation(&failure),
    };
    Ok(observation(
        projected.case_id.clone(),
        input,
        observation_value,
        "region-membership",
    ))
}

fn coefficient_from_value(value: u64) -> i64 {
    [1, 2, 3, 5][(value % 4) as usize]
}

fn signed_coordinate(value: u64) -> i64 {
    let magnitude = [0, 1, 2, 3][(value % 4) as usize];
    if (value / 4) % 2 == 1 {
        -magnitude
    } else {
        magnitude
    }
}

fn raw_polynomial_terms(
    coefficient: i64,
    formula: u64,
    inject_failure: bool,
) -> Result<Vec<RegionPolynomialTerm>, AdapterFailure> {
    if inject_failure {
        let mut terms = Vec::new();
        let count = MAX_REGION_POLYNOMIAL_TERMS + 1;
        for index in 0..count {
            terms.push(term(1, index as u32, 0, 0)?);
        }
        return Ok(terms);
    }
    match formula {
        0 => Ok(vec![
            term(coefficient, 1, 0, 0)?,
            term(0, 0, 1, 0)?,
            term(coefficient, 1, 0, 0)?,
        ]),
        1 => Ok(vec![term(coefficient, 0, 1, 0)?, term(coefficient, 1, 0, 0)?]),
        2 => Ok(vec![term(coefficient, 1, 0, 0)?]),
        _ => Ok(vec![term(coefficient, 0, 0, 1)?]),
    }
}

fn affine_source_terms(
    kind: u64,
    coefficient: i64,
    inject_failure: bool,
) -> Result<Vec<RegionPolynomialTerm>, AdapterFailure> {
    if inject_failure {
        return Ok(vec![term(1, 2, 0, 0)?]);
    }
    match kind {
        0 => Ok(vec![term(1, 1, 0, 0)?]),
        1 => Ok(vec![term(coefficient, 1, 0, 0)?, term(1, 0, 1, 0)?]),
        2 => Ok(vec![term(1, 1, 0, 0)?, term(1, 0, 0, 1)?]),
        _ => Ok(vec![term(coefficient, 0, 0, 1)?]),
    }
}

type AffineMapTerms = (
    Vec<RegionPolynomialTerm>,
    Vec<RegionPolynomialTerm>,
    Vec<RegionPolynomialTerm>,
);

fn affine_map_raw(
    kind: u64,
    translation: i64,
    inject_failure: bool,
) -> Result<AffineMapTerms, AdapterFailure> {
    if inject_failure {
        return Ok((
            vec![term(1, 2, 0, 0)?],
            vec![term(1, 0, 1, 0)?],
            vec![term(1, 0, 0, 1)?],
        ));
    }
    match kind {
        0 => Ok((
            vec![term(1, 1, 0, 0)?],
            vec![term(1, 0, 1, 0)?],
            vec![term(1, 0, 0, 1)?],
        )),
        1 => Ok((
            vec![term(1, 1, 0, 0)?, term(translation, 0, 0, 0)?],
            vec![term(1, 0, 1, 0)?],
            vec![term(1, 0, 0, 1)?],
        )),
        2 => Ok((
            vec![term(2, 1, 0, 0)?],
            vec![term(1, 0, 1, 0)?],
            vec![term(1, 0, 0, 1)?],
        )),
        _ => Ok((
            vec![term(3, 1, 0, 0)?],
            vec![term(1, 0, 1, 0)?],
            vec![term(1, 0, 0, 1)?],
        )),
    }
}

fn formula_configuration(
    variant: u64,
    inject_failure: bool,
) -> Result<(Vec<CanonicalAtom>, NormalizedRegionFormula, Value), AdapterFailure> {
    let atom = polynomial_atom("a", 1, 1, 0, 0)?;
    if inject_failure {
        return Ok((
            vec![atom],
            NormalizedRegionFormula::And(Vec::new()),
            object([("variant", Value::String("empty-and".to_owned()))]),
        ));
    }
    match variant {
        0 => Ok((
            vec![atom],
            NormalizedRegionFormula::True,
            object([("variant", Value::String("true".to_owned()))]),
        )),
        1 => Ok((
            vec![atom],
            NormalizedRegionFormula::And(vec![
                NormalizedRegionFormula::Atom("a".to_owned()),
                NormalizedRegionFormula::True,
            ]),
            object([("variant", Value::String("and".to_owned()))]),
        )),
        2 => Ok((
            vec![atom],
            NormalizedRegionFormula::Not(Box::new(NormalizedRegionFormula::Not(Box::new(
                NormalizedRegionFormula::Atom("a".to_owned()),
            )))),
            object([("variant", Value::String("not".to_owned()))]),
        )),
        _ => Ok((
            vec![atom],
            NormalizedRegionFormula::False,
            object([("variant", Value::String("false".to_owned()))]),
        )),
    }
}

fn polynomial_atom(
    atom_id: &str,
    coefficient: i64,
    exponent_x: u32,
    exponent_y: u32,
    exponent_z: u32,
) -> Result<CanonicalAtom, AdapterFailure> {
    CanonicalAtom::try_new(
        atom_id,
        RegionFunction::Polynomial(
            RegionPolynomial3::try_from_terms(vec![term(
                coefficient, exponent_x, exponent_y, exponent_z,
            )?])
            .map_err(api_failure)?,
        ),
        SignRelation::Positive,
    )
    .map_err(api_failure)
}

fn term(
    coefficient: i64,
    exponent_x: u32,
    exponent_y: u32,
    exponent_z: u32,
) -> Result<RegionPolynomialTerm, AdapterFailure> {
    RegionPolynomialTerm::try_new(
        integer_algebraic(coefficient)?,
        exponent_x,
        exponent_y,
        exponent_z,
    )
    .map_err(api_failure)
}

fn polynomial_terms_value(polynomial: &RegionPolynomial3) -> Result<Value, AdapterFailure> {
    polynomial_terms_slice(polynomial.terms())
}

fn polynomial_terms_slice(terms: &[RegionPolynomialTerm]) -> Result<Value, AdapterFailure> {
    let encoded = terms
        .iter()
        .map(|term| {
            Ok(object([
                ("coefficient", algebraic_rational(term.coefficient())?),
                ("exponentX", decimal(term.exponent_x())),
                ("exponentY", decimal(term.exponent_y())),
                ("exponentZ", decimal(term.exponent_z())),
            ]))
        })
        .collect::<Result<Vec<_>, AdapterFailure>>()?;
    Ok(Value::Array(encoded))
}

fn formula_value(formula: &NormalizedRegionFormula) -> Result<Value, AdapterFailure> {
    Ok(match formula {
        NormalizedRegionFormula::Atom(atom_id) => object([
            ("atomId", Value::String(atom_id.clone())),
            ("kind", Value::String("atom".to_owned())),
        ]),
        NormalizedRegionFormula::Not(inner) => object([
            ("kind", Value::String("not".to_owned())),
            ("operand", formula_value(inner)?),
        ]),
        NormalizedRegionFormula::And(operands) => object([
            ("kind", Value::String("and".to_owned())),
            (
                "operands",
                Value::Array(
                    operands
                        .iter()
                        .map(formula_value)
                        .collect::<Result<Vec<_>, _>>()?,
                ),
            ),
        ]),
        NormalizedRegionFormula::Or(operands) => object([
            ("kind", Value::String("or".to_owned())),
            (
                "operands",
                Value::Array(
                    operands
                        .iter()
                        .map(formula_value)
                        .collect::<Result<Vec<_>, _>>()?,
                ),
            ),
        ]),
        NormalizedRegionFormula::True => object([("kind", Value::String("true".to_owned()))]),
        NormalizedRegionFormula::False => object([("kind", Value::String("false".to_owned()))]),
    })
}

fn classification_name(classification: MembershipClassification) -> &'static str {
    match classification {
        MembershipClassification::Inside => "inside",
        MembershipClassification::Boundary => "boundary",
        MembershipClassification::Outside => "outside",
    }
}

fn sign_name(relation: SignRelation) -> &'static str {
    match relation {
        SignRelation::Negative => "negative",
        SignRelation::Zero => "zero",
        SignRelation::Positive => "positive",
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
