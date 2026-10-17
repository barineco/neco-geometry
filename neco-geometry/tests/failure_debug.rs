use neco_algnum::RealAlgebraic;
use neco_bigint::{BigInt, BigUint, Dyadic, RawRational};
use neco_calculus::{
    AbsoluteBits, CalculusFailure, ConsumerId, Enclosure, EvaluationFailure, ExactDecisionRequest,
    ExactExpressionRequirement, NecoImplementationSource, OwnerId, RationalSurfacePolynomials,
    SurfacePolynomial, SurfaceTerm,
};
use neco_expr::ExprId;
use neco_geometry::{
    evaluate_surface_witness_checked, evaluate_surface_witness_exact, CertifiedShapeEvaluation,
    ChartDisplayName, ChartDomainId, CoordinateEnclosure3, GeometryFailure, ParameterInterval,
    ParametricSurfaceChart2, ShapeCoordinateEnclosure, ShapeEvaluation, SurfaceOrientation,
    SurfaceParameterDomain,
};

const CHECKED_CERTIFIED_WITNESS: &str = r#"Certified { identity: SurfaceChartIdentity { domain_id: ChartDomainId("surface-D"), version: 1 }, x: Enclosure { value: DyadicEnclosure { lower: Dyadic { integer: BigInt { sign: Positive, magnitude: BigUint { limbs: [131071] } }, exponent: 16 }, upper: Dyadic { integer: BigInt { sign: Positive, magnitude: BigUint { limbs: [131073] } }, exponent: 16 } } }, y: Enclosure { value: DyadicEnclosure { lower: Dyadic { integer: BigInt { sign: Positive, magnitude: BigUint { limbs: [98303] } }, exponent: 15 }, upper: Dyadic { integer: BigInt { sign: Positive, magnitude: BigUint { limbs: [98305] } }, exponent: 15 } } }, z: Enclosure { value: DyadicEnclosure { lower: Dyadic { integer: BigInt { sign: Positive, magnitude: BigUint { limbs: [65535] } }, exponent: 16 }, upper: Dyadic { integer: BigInt { sign: Positive, magnitude: BigUint { limbs: [65537] } }, exponent: 16 } } }, owner: OwnerId(1), requirement: ExactExpressionRequirement { consumer_id: ConsumerId(11), expressions: [ExprId(500)], decision: Zero(ExprId(500)), precision: AbsoluteBits(16) }, source: NecoImplementationSource { revision_identity: "rev", public_api_identity: "api" } }"#;
const SHAPE_COORDINATE_ENCLOSURE_SPATIAL: &str = r#"Spatial(CoordinateEnclosure3 { x: Enclosure { value: DyadicEnclosure { lower: Dyadic { integer: BigInt { sign: Zero, magnitude: BigUint { limbs: [] } }, exponent: 0 }, upper: Dyadic { integer: BigInt { sign: Positive, magnitude: BigUint { limbs: [1] } }, exponent: 0 } } }, y: Enclosure { value: DyadicEnclosure { lower: Dyadic { integer: BigInt { sign: Positive, magnitude: BigUint { limbs: [1] } }, exponent: 0 }, upper: Dyadic { integer: BigInt { sign: Positive, magnitude: BigUint { limbs: [2] } }, exponent: 0 } } }, z: Enclosure { value: DyadicEnclosure { lower: Dyadic { integer: BigInt { sign: Positive, magnitude: BigUint { limbs: [2] } }, exponent: 0 }, upper: Dyadic { integer: BigInt { sign: Positive, magnitude: BigUint { limbs: [3] } }, exponent: 0 } } } })"#;
const CERTIFIED_PARAMETRIC_SURFACE: &str = r#"ParametricSurface { coordinate: CoordinateEnclosure3 { x: Enclosure { value: DyadicEnclosure { lower: Dyadic { integer: BigInt { sign: Zero, magnitude: BigUint { limbs: [] } }, exponent: 0 }, upper: Dyadic { integer: BigInt { sign: Positive, magnitude: BigUint { limbs: [1] } }, exponent: 0 } } }, y: Enclosure { value: DyadicEnclosure { lower: Dyadic { integer: BigInt { sign: Positive, magnitude: BigUint { limbs: [1] } }, exponent: 0 }, upper: Dyadic { integer: BigInt { sign: Positive, magnitude: BigUint { limbs: [2] } }, exponent: 0 } } }, z: Enclosure { value: DyadicEnclosure { lower: Dyadic { integer: BigInt { sign: Positive, magnitude: BigUint { limbs: [2] } }, exponent: 0 }, upper: Dyadic { integer: BigInt { sign: Positive, magnitude: BigUint { limbs: [3] } }, exponent: 0 } } } }, owner: OwnerId(1), requirement: ExactExpressionRequirement { consumer_id: ConsumerId(11), expressions: [ExprId(500)], decision: Zero(ExprId(500)), precision: AbsoluteBits(16) }, source: NecoImplementationSource { revision_identity: "rev", public_api_identity: "api" } }"#;
const CERTIFIED_PARAMETRIC_CURVE_SPATIAL: &str = r#"CertifiedEnclosure(ParametricCurve { coordinate: Spatial(CoordinateEnclosure3 { x: Enclosure { value: DyadicEnclosure { lower: Dyadic { integer: BigInt { sign: Zero, magnitude: BigUint { limbs: [] } }, exponent: 0 }, upper: Dyadic { integer: BigInt { sign: Positive, magnitude: BigUint { limbs: [1] } }, exponent: 0 } } }, y: Enclosure { value: DyadicEnclosure { lower: Dyadic { integer: BigInt { sign: Positive, magnitude: BigUint { limbs: [1] } }, exponent: 0 }, upper: Dyadic { integer: BigInt { sign: Positive, magnitude: BigUint { limbs: [2] } }, exponent: 0 } } }, z: Enclosure { value: DyadicEnclosure { lower: Dyadic { integer: BigInt { sign: Positive, magnitude: BigUint { limbs: [2] } }, exponent: 0 }, upper: Dyadic { integer: BigInt { sign: Positive, magnitude: BigUint { limbs: [3] } }, exponent: 0 } } } }), owner: OwnerId(1), requirement: ExactExpressionRequirement { consumer_id: ConsumerId(11), expressions: [ExprId(500)], decision: Zero(ExprId(500)), precision: AbsoluteBits(16) }, source: NecoImplementationSource { revision_identity: "rev", public_api_identity: "api" } })"#;
const CHECKED_DENOMINATOR_CONTAINS_ZERO: &str = r#"DenominatorValidity { error: Evaluation(DomainViolation { operation: SurfaceChecked, owner: OwnerId(1), requirement: ExactExpressionRequirement { consumer_id: ConsumerId(11), expressions: [ExprId(500)], decision: Zero(ExprId(500)), precision: AbsoluteBits(16) }, source: NecoImplementationSource { revision_identity: "rev", public_api_identity: "api" }, input: SurfaceChecked { surface: RationalSurfacePolynomials { x: SurfacePolynomial { terms: [SurfaceTerm { coefficient: ReducedRational { numerator: BigInt { sign: Positive, magnitude: BigUint { limbs: [2] } }, denominator: BigUint { limbs: [1] } }, exponent_u: 1, exponent_v: 0 }] }, y: SurfacePolynomial { terms: [SurfaceTerm { coefficient: ReducedRational { numerator: BigInt { sign: Positive, magnitude: BigUint { limbs: [2] } }, denominator: BigUint { limbs: [1] } }, exponent_u: 0, exponent_v: 1 }] }, z: SurfacePolynomial { terms: [SurfaceTerm { coefficient: ReducedRational { numerator: BigInt { sign: Positive, magnitude: BigUint { limbs: [2] } }, denominator: BigUint { limbs: [1] } }, exponent_u: 0, exponent_v: 0 }] }, denominator: SurfacePolynomial { terms: [SurfaceTerm { coefficient: ReducedRational { numerator: BigInt { sign: Positive, magnitude: BigUint { limbs: [1] } }, denominator: BigUint { limbs: [1] } }, exponent_u: 1, exponent_v: 0 }] } }, u: Enclosure { value: DyadicEnclosure { lower: Dyadic { integer: BigInt { sign: Zero, magnitude: BigUint { limbs: [] } }, exponent: 0 }, upper: Dyadic { integer: BigInt { sign: Positive, magnitude: BigUint { limbs: [1] } }, exponent: 0 } } }, v: Enclosure { value: DyadicEnclosure { lower: Dyadic { integer: BigInt { sign: Positive, magnitude: BigUint { limbs: [3] } }, exponent: 0 }, upper: Dyadic { integer: BigInt { sign: Positive, magnitude: BigUint { limbs: [3] } }, exponent: 0 } } } }, witness: DomainContainsZero { denominator: Enclosure { value: DyadicEnclosure { lower: Dyadic { integer: BigInt { sign: Zero, magnitude: BigUint { limbs: [] } }, exponent: 0 }, upper: Dyadic { integer: BigInt { sign: Positive, magnitude: BigUint { limbs: [131073] } }, exponent: 17 } } } }, lower: DomainContainsZero }) }"#;
const EXACT_DENOMINATOR_ZERO: &str = r#"DenominatorValidity { error: Rational(UndefinedDomain) }"#;
const CALCULUS_FAILURE_ZERO_IMPLICIT_TERM: &str =
    r#"CalculusFailure { error: Evaluation(ZeroImplicitTerm) }"#;
const DENOMINATOR_VALIDITY_DUPLICATE_EXPONENT: &str =
    r#"DenominatorValidity { error: Evaluation(DuplicateImplicitExponent) }"#;

fn rational(value: i32) -> neco_bigint::ReducedRational {
    RawRational::new(BigInt::try_from(value).unwrap(), BigUint::one().unwrap())
        .reduce()
        .unwrap()
        .into_reduced()
}

fn algebraic(value: i32) -> RealAlgebraic {
    RealAlgebraic::from_integer(BigInt::try_from(value).unwrap()).unwrap()
}

fn interval(lower: i32, upper: i32) -> ParameterInterval {
    ParameterInterval::new(algebraic(lower), algebraic(upper), true, true).unwrap()
}

fn domain() -> SurfaceParameterDomain {
    SurfaceParameterDomain::new(interval(0, 8), interval(0, 8))
}

fn surface_polynomial(coefficient: i32, exponent_u: u32, exponent_v: u32) -> SurfacePolynomial {
    SurfacePolynomial::try_from_terms(vec![SurfaceTerm {
        coefficient: rational(coefficient),
        exponent_u,
        exponent_v,
    }])
    .unwrap()
}

fn chart(denominator: SurfacePolynomial) -> ParametricSurfaceChart2 {
    ParametricSurfaceChart2::try_new(
        ChartDomainId::new(String::from("surface-D")).unwrap(),
        ChartDisplayName::new(String::from("surface-D")).unwrap(),
        1,
        SurfaceOrientation::Preserve,
        RationalSurfacePolynomials {
            x: surface_polynomial(2, 1, 0),
            y: surface_polynomial(2, 0, 1),
            z: surface_polynomial(2, 0, 0),
            denominator,
        },
    )
    .unwrap()
}

fn enclosure(lower: i32, upper: i32) -> Enclosure {
    Enclosure::new(
        Dyadic::new(BigInt::try_from(lower).unwrap(), 0),
        Dyadic::new(BigInt::try_from(upper).unwrap(), 0),
    )
    .unwrap()
}

fn requirement() -> ExactExpressionRequirement {
    ExactExpressionRequirement::new(
        ConsumerId::new(11),
        &[ExprId::new(500)],
        ExactDecisionRequest::Zero(ExprId::new(500)),
        AbsoluteBits::new(16),
    )
    .unwrap()
}

fn source() -> NecoImplementationSource {
    NecoImplementationSource::new("rev", "api")
}

fn coordinate() -> CoordinateEnclosure3 {
    CoordinateEnclosure3 {
        x: enclosure(0, 1),
        y: enclosure(1, 2),
        z: enclosure(2, 3),
    }
}

#[test]
fn geometry_failure_debug_text_keeps_the_unboxed_form() {
    let crossing = chart(surface_polynomial(1, 1, 0));
    let enclosure_failure = evaluate_surface_witness_checked(
        &crossing,
        &domain(),
        &enclosure(0, 1),
        &enclosure(3, 3),
        OwnerId::new(1),
        requirement(),
        source(),
    )
    .unwrap_err();
    assert_eq!(
        format!("{enclosure_failure:?}"),
        CHECKED_DENOMINATOR_CONTAINS_ZERO
    );
    let point_failure =
        evaluate_surface_witness_exact(&crossing, &domain(), &algebraic(0), &algebraic(3))
            .unwrap_err();
    assert_eq!(format!("{point_failure:?}"), EXACT_DENOMINATOR_ZERO);
    let calculus = GeometryFailure::CalculusFailure {
        error: CalculusFailure::from(EvaluationFailure::ZeroImplicitTerm),
    };
    assert_eq!(format!("{calculus:?}"), CALCULUS_FAILURE_ZERO_IMPLICIT_TERM);
    let denominator = GeometryFailure::DenominatorValidity {
        error: CalculusFailure::from(EvaluationFailure::DuplicateImplicitExponent),
    };
    assert_eq!(
        format!("{denominator:?}"),
        DENOMINATOR_VALIDITY_DUPLICATE_EXPONENT
    );
}

#[test]
fn certified_evaluation_debug_text_keeps_the_unboxed_form() {
    let witness = evaluate_surface_witness_checked(
        &chart(surface_polynomial(2, 0, 0)),
        &domain(),
        &enclosure(2, 2),
        &enclosure(3, 3),
        OwnerId::new(1),
        requirement(),
        source(),
    )
    .unwrap();
    assert_eq!(format!("{witness:?}"), CHECKED_CERTIFIED_WITNESS);
    let spatial = ShapeCoordinateEnclosure::Spatial(Box::new(coordinate()));
    assert_eq!(format!("{spatial:?}"), SHAPE_COORDINATE_ENCLOSURE_SPATIAL);
    let surface = CertifiedShapeEvaluation::ParametricSurface {
        coordinate: Box::new(coordinate()),
        owner: OwnerId::new(1),
        requirement: requirement(),
        source: source(),
    };
    assert_eq!(format!("{surface:?}"), CERTIFIED_PARAMETRIC_SURFACE);
    let curve = ShapeEvaluation::CertifiedEnclosure(CertifiedShapeEvaluation::ParametricCurve {
        coordinate: ShapeCoordinateEnclosure::Spatial(Box::new(coordinate())),
        owner: OwnerId::new(1),
        requirement: requirement(),
        source: source(),
    });
    assert_eq!(format!("{curve:?}"), CERTIFIED_PARAMETRIC_CURVE_SPATIAL);
}
