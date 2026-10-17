use geometry_observation_types::{
    decimal, failure_observation, object, rational, AdapterFailure, OperandProvenance, Value,
};
use neco_algnum::RealAlgebraic;
use neco_bigint::{BigInt, BigUint, Dyadic, ReducedRational, Sign};
use neco_geometry::GeometryFailure;

pub(crate) fn integer_algebraic(value: i64) -> Result<RealAlgebraic, AdapterFailure> {
    let integer = BigInt::try_from(value).map_err(|error| encode_failure(error.to_string()))?;
    RealAlgebraic::from_integer(integer).map_err(|error| encode_failure(error.to_string()))
}

pub(crate) fn integer_bigint(value: i64) -> Result<BigInt, AdapterFailure> {
    BigInt::try_from(value).map_err(|error| encode_failure(error.to_string()))
}

pub(crate) fn big_decimal(value: &BigUint) -> Result<String, AdapterFailure> {
    if value.is_zero() {
        return Ok("0".to_owned());
    }
    let base = BigUint::try_from(1_000_000_000_u32).map_err(|error| encode_failure(error.to_string()))?;
    let mut power = BigUint::one().map_err(|error| encode_failure(error.to_string()))?;
    while power.mul(&base).is_ok_and(|next| next <= *value) {
        power = power.mul(&base).map_err(|error| encode_failure(error.to_string()))?;
    }
    let (leading, mut rest) = value
        .div_rem(&power)
        .map_err(|error| encode_failure(error.to_string()))?;
    let mut text = leading
        .to_u32()
        .ok_or_else(|| encode_failure("decimal chunk exceeds u32"))?
        .to_string();
    let one = BigUint::one().map_err(|error| encode_failure(error.to_string()))?;
    while power > one {
        power = power
            .exact_div(&base)
            .map_err(|error| encode_failure(error.to_string()))?;
        let (chunk, next) = rest
            .div_rem(&power)
            .map_err(|error| encode_failure(error.to_string()))?;
        text.push_str(&format!(
            "{:09}",
            chunk
                .to_u32()
                .ok_or_else(|| encode_failure("decimal chunk exceeds u32"))?
        ));
        rest = next;
    }
    Ok(text)
}

pub(crate) fn int_decimal(value: &BigInt) -> Result<String, AdapterFailure> {
    let magnitude = big_decimal(value.magnitude())?;
    Ok(match value.sign() {
        Sign::Negative => format!("-{magnitude}"),
        Sign::Zero | Sign::Positive => magnitude,
    })
}

pub(crate) fn algebraic_rational(value: &RealAlgebraic) -> Result<Value, AdapterFailure> {
    let coefficients = value.minimal_polynomial().polynomial().coefficients();
    if coefficients.len() != 2 {
        return Err(encode_failure(
            "observed algebraic coefficient is not a rational of degree one",
        ));
    }
    let constant = coefficients[0]
        .try_clone()
        .map_err(|error| encode_failure(error.to_string()))?;
    let linear = coefficients[1]
        .try_clone()
        .map_err(|error| encode_failure(error.to_string()))?;
    let numerator = constant
        .negated()
        .map_err(|error| encode_failure(error.to_string()))?;
    let denominator_sign = linear.sign();
    let denominator = linear
        .magnitude()
        .try_clone()
        .map_err(|error| encode_failure(error.to_string()))?;
    if denominator.is_zero() {
        return Err(encode_failure("rational denominator is zero"));
    }
    let numerator = match denominator_sign {
        Sign::Negative => numerator
            .negated()
            .map_err(|error| encode_failure(error.to_string()))?,
        Sign::Zero | Sign::Positive => numerator,
    };
    let reduced = neco_bigint::RawRational::new(numerator, denominator)
        .reduce()
        .map_err(|error| encode_failure(error.to_string()))?
        .into_reduced();
    reduced_rational_value(&reduced)
}

pub(crate) fn reduced_rational_value(value: &ReducedRational) -> Result<Value, AdapterFailure> {
    Ok(rational(
        int_decimal(value.numerator())?,
        big_decimal(value.denominator())?,
    ))
}

pub(crate) fn dyadic_value(value: &Dyadic) -> Result<Value, AdapterFailure> {
    Ok(geometry_observation_types::dyadic(
        int_decimal(value.integer())?,
        value.exponent(),
    ))
}

pub(crate) fn geometry_failure_observation(failure: &GeometryFailure) -> Value {
    failure_observation(geometry_failure_value(failure))
}

pub(crate) fn geometry_failure_value(failure: &GeometryFailure) -> Value {
    match failure {
        GeometryFailure::ParameterOutsideDomain => variant("parameter-outside-domain"),
        GeometryFailure::InvalidRegionAffineMap => variant("invalid-region-affine-map"),
        GeometryFailure::RegionPolynomialTermLimit {
            requested_terms,
            maximum_terms,
        } => object([
            ("maximumTerms", decimal(*maximum_terms)),
            ("requestedTerms", decimal(*requested_terms)),
            (
                "variant",
                Value::String("region-polynomial-term-limit".to_owned()),
            ),
        ]),
        GeometryFailure::RegionExponentOverflow => variant("region-exponent-overflow"),
        GeometryFailure::RegionExponentOperationLimit {
            requested_operations,
            maximum_operations,
        } => object([
            ("maximumOperations", decimal(*maximum_operations)),
            ("requestedOperations", decimal(*requested_operations)),
            (
                "variant",
                Value::String("region-exponent-operation-limit".to_owned()),
            ),
        ]),
        GeometryFailure::MalformedRegionFormula => variant("malformed-region-formula"),
        GeometryFailure::InvalidSurfaceChart => variant("invalid-surface-chart"),
        GeometryFailure::DuplicateRegionAtom => variant("duplicate-region-atom"),
        GeometryFailure::DuplicateRegionCanonicalKey => variant("duplicate-region-canonical-key"),
        GeometryFailure::RegionAmbientDimensionMismatch => {
            variant("region-ambient-dimension-mismatch")
        }
        GeometryFailure::RegionPointDimensionMismatch => variant("region-point-dimension-mismatch"),
        GeometryFailure::RegionKindMismatch => variant("region-kind-mismatch"),
        GeometryFailure::DenominatorValidity { .. } => variant("denominator-validity"),
        GeometryFailure::CalculusFailure { .. } => variant("calculus-failure"),
        GeometryFailure::AlgebraFailure(_) => variant("algebra-failure"),
        GeometryFailure::EvaluationIndeterminacy { .. } => variant("evaluation-indeterminacy"),
        GeometryFailure::RegionCanonicalAtomLimit {
            requested_atoms,
            maximum_atoms,
        } => object([
            ("maximumAtoms", decimal(*maximum_atoms)),
            ("requestedAtoms", decimal(*requested_atoms)),
            (
                "variant",
                Value::String("region-canonical-atom-limit".to_owned()),
            ),
        ]),
        GeometryFailure::FoundationFailure { .. } => variant("foundation-failure"),
        GeometryFailure::ZeroDirection => variant("zero-direction"),
        GeometryFailure::NonOrthogonalMatrix => variant("non-orthogonal-matrix"),
        GeometryFailure::NonUnitQuaternion => variant("non-unit-quaternion"),
        GeometryFailure::ShapeFamilyMismatch => variant("shape-family-mismatch"),
        GeometryFailure::InvalidRayParameterRange => variant("invalid-ray-parameter-range"),
        GeometryFailure::RootIsolationPolicyInvalid => variant("root-isolation-policy-invalid"),
        GeometryFailure::RootIsolationResourceLimit { .. } => {
            variant("root-isolation-resource-limit")
        }
        GeometryFailure::RootPartitionCellLimit { .. } => variant("root-partition-cell-limit"),
        GeometryFailure::RootPartitionIdentifierOverflow { .. } => {
            variant("root-partition-identifier-overflow")
        }
        GeometryFailure::BoundaryResourceLimit { .. } => variant("boundary-resource-limit"),
        GeometryFailure::EmptyFinitePointComponents => variant("empty-finite-point-components"),
        GeometryFailure::EmptyCurveBranchComponents => variant("empty-curve-branch-components"),
        GeometryFailure::DuplicateIntersectionComponentIdentity => {
            variant("duplicate-intersection-component-identity")
        }
        GeometryFailure::ComponentIdentityEvidenceMismatch => {
            variant("component-identity-evidence-mismatch")
        }
        GeometryFailure::ComponentIdentityCanonicalizationFailure => {
            variant("component-identity-canonicalization-failure")
        }
        GeometryFailure::SelfIntersectingTrimLoop => variant("self-intersecting-trim-loop"),
        GeometryFailure::HoleIntersectsOuterLoop => variant("hole-intersects-outer-loop"),
        GeometryFailure::PCurveOutsideDomain => variant("pcurve-outside-domain"),
        GeometryFailure::EmptyRayCrossingTargetSet => variant("empty-ray-crossing-target-set"),
        GeometryFailure::DuplicateRayCrossingTargetIdentity => {
            variant("duplicate-ray-crossing-target-identity")
        }
        GeometryFailure::MixedRayCrossingTargetFamily => variant("mixed-ray-crossing-target-family"),
        GeometryFailure::DegenerateTriangle => variant("degenerate-triangle"),
        GeometryFailure::InvalidRayCrossingPolicy => variant("invalid-ray-crossing-policy"),
        GeometryFailure::RayCrossingTargetLimitExceeded => {
            variant("ray-crossing-target-limit-exceeded")
        }
        GeometryFailure::RayCrossingHitLimitExceeded => variant("ray-crossing-hit-limit-exceeded"),
    }
}

fn variant(name: &'static str) -> Value {
    object([("variant", Value::String(name.to_owned()))])
}

pub(crate) fn encode_failure(condition: impl Into<String>) -> AdapterFailure {
    adapter_failure("publicApiConstructionFailure", "encodeObservation", condition)
}

pub(crate) fn unexpected_outcome(condition: impl Into<String>) -> AdapterFailure {
    adapter_failure(
        "unexpectedPublicApiOutcome",
        "observePublicApi",
        condition,
    )
}

fn adapter_failure(
    kind: &'static str,
    failing_operation: &'static str,
    condition: impl Into<String>,
) -> AdapterFailure {
    AdapterFailure::new(
        kind,
        failing_operation,
        OperandProvenance {
            case_id: None,
            family: None,
            requested_observation_kind: None,
            selectors: Default::default(),
            projection_revision: None,
            resource_revision: None,
            build_identity: None,
        },
        condition,
    )
}
