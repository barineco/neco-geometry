use neco_algnum::RealAlgebraic;
use neco_bigint::{BigInt, BigUint, Dyadic, RawRational};
use neco_calculus::{
    AbsoluteBits, ConsumerId, Enclosure, ExactDecisionRequest, ExactExpressionRequirement,
    NecoImplementationSource, OwnerId, RationalSurfacePolynomials, SurfacePolynomial,
    SurfaceTerm,
};
use neco_expr::ExprId;
use neco_geometry::{
    evaluate_surface_witness_checked, evaluate_surface_witness_exact, AlgebraicRegion,
    CanonicalAtom, ChartDisplayName, ChartDomainId, GeometryFailure, MembershipClassification,
    NormalizedRegionFormula, ParameterInterval, ParametricSurfaceChart2, Point2, Point3,
    RegionAffineMap3, RegionFunction, RegionKind, RegionMembershipRequest, RegionPoint,
    RegionPolynomial3, RegionPolynomialTerm, SignRelation, SurfaceOrientation,
    SurfaceParameterDomain, MAX_REGION_POLYNOMIAL_TERMS,
};

fn algebraic(value: i64) -> RealAlgebraic {
    RealAlgebraic::from_integer(BigInt::try_from(value).unwrap()).unwrap()
}

fn term(coefficient: i64, x: u32, y: u32, z: u32) -> RegionPolynomialTerm {
    RegionPolynomialTerm::try_new(algebraic(coefficient), x, y, z).unwrap()
}

fn rational(value: i32) -> neco_bigint::ReducedRational {
    RawRational::new(BigInt::try_from(value).unwrap(), BigUint::one().unwrap())
        .reduce()
        .unwrap()
        .into_reduced()
}

fn monomial(coefficient: i32, exponent_u: u32, exponent_v: u32) -> SurfacePolynomial {
    SurfacePolynomial::try_from_terms(vec![SurfaceTerm {
        coefficient: rational(coefficient),
        exponent_u,
        exponent_v,
    }])
    .unwrap()
}

#[test]
fn crate_root_region_polynomial_normalizes_and_reports_term_limit() {
    let merged = RegionPolynomial3::try_from_terms(vec![
        term(1, 1, 0, 0),
        term(0, 0, 1, 0),
        term(1, 1, 0, 0),
    ])
    .unwrap();
    assert_eq!(merged.terms().len(), 1);
    assert_eq!(merged.terms()[0].exponent_x(), 1);
    let overflow = (0..=MAX_REGION_POLYNOMIAL_TERMS)
        .map(|index| term(1, index as u32, 0, 0))
        .collect();
    assert!(matches!(
        RegionPolynomial3::try_from_terms(overflow),
        Err(GeometryFailure::RegionPolynomialTermLimit { .. })
    ));
}

#[test]
fn crate_root_affine_map_rejects_quadratic_coordinate() {
    let source = RegionPolynomial3::try_from_terms(vec![term(1, 1, 0, 0)]).unwrap();
    let identity = RegionAffineMap3::try_new(
        RegionPolynomial3::try_from_terms(vec![term(1, 1, 0, 0)]).unwrap(),
        RegionPolynomial3::try_from_terms(vec![term(1, 0, 1, 0)]).unwrap(),
        RegionPolynomial3::try_from_terms(vec![term(1, 0, 0, 1)]).unwrap(),
    )
    .unwrap();
    assert!(source.try_affine_substitute(&identity).is_ok());
    assert!(matches!(
        RegionAffineMap3::try_new(
            RegionPolynomial3::try_from_terms(vec![term(1, 2, 0, 0)]).unwrap(),
            RegionPolynomial3::try_from_terms(vec![term(1, 0, 1, 0)]).unwrap(),
            RegionPolynomial3::try_from_terms(vec![term(1, 0, 0, 1)]).unwrap(),
        ),
        Err(GeometryFailure::InvalidRegionAffineMap)
    ));
}

#[test]
fn crate_root_formula_and_membership_use_public_constructors() {
    let atom = CanonicalAtom::try_new(
        "a",
        RegionFunction::Polynomial(RegionPolynomial3::try_from_terms(vec![term(1, 1, 0, 0)]).unwrap()),
        SignRelation::Positive,
    )
    .unwrap();
    assert!(AlgebraicRegion::try_new(
        3,
        vec![atom],
        NormalizedRegionFormula::And(Vec::new()),
        RegionKind::GeneralSemiAlgebraicSet,
    )
    .is_err());
    let region = AlgebraicRegion::try_new(
        3,
        vec![CanonicalAtom::try_new(
            "a",
            RegionFunction::Polynomial(
                RegionPolynomial3::try_from_terms(vec![term(1, 1, 0, 0)]).unwrap(),
            ),
            SignRelation::Positive,
        )
        .unwrap()],
        NormalizedRegionFormula::Atom("a".to_owned()),
        RegionKind::GeneralSemiAlgebraicSet,
    )
    .unwrap();
    let request_region = AlgebraicRegion::try_new(
        3,
        vec![CanonicalAtom::try_new(
            "a",
            RegionFunction::Polynomial(
                RegionPolynomial3::try_from_terms(vec![term(1, 1, 0, 0)]).unwrap(),
            ),
            SignRelation::Positive,
        )
        .unwrap()],
        NormalizedRegionFormula::Atom("a".to_owned()),
        RegionKind::GeneralSemiAlgebraicSet,
    )
    .unwrap();
    let inside = region
        .membership(RegionMembershipRequest {
            region: request_region,
            point: RegionPoint::Spatial(Point3 {
                x: algebraic(1),
                y: algebraic(0),
                z: algebraic(0),
            }),
        })
        .unwrap();
    assert_eq!(inside.classification, MembershipClassification::Inside);
    let mismatch = AlgebraicRegion::try_new(
        3,
        vec![CanonicalAtom::try_new(
            "a",
            RegionFunction::Polynomial(
                RegionPolynomial3::try_from_terms(vec![term(1, 1, 0, 0)]).unwrap(),
            ),
            SignRelation::Positive,
        )
        .unwrap()],
        NormalizedRegionFormula::Atom("a".to_owned()),
        RegionKind::GeneralSemiAlgebraicSet,
    )
    .unwrap()
    .membership(RegionMembershipRequest {
        region: AlgebraicRegion::try_new(
            3,
            vec![CanonicalAtom::try_new(
                "a",
                RegionFunction::Polynomial(
                    RegionPolynomial3::try_from_terms(vec![term(1, 1, 0, 0)]).unwrap(),
                ),
                SignRelation::Positive,
            )
            .unwrap()],
            NormalizedRegionFormula::Atom("a".to_owned()),
            RegionKind::GeneralSemiAlgebraicSet,
        )
        .unwrap(),
        point: RegionPoint::Planar(Point2 {
            x: algebraic(1),
            y: algebraic(0),
        }),
    });
    assert!(matches!(
        mismatch,
        Err(GeometryFailure::RegionPointDimensionMismatch)
    ));
}

#[test]
fn crate_root_surface_witnesses_cover_exact_checked_and_domain_failure() {
    let chart = ParametricSurfaceChart2::try_new(
        ChartDomainId::new("surface-D".to_owned()).unwrap(),
        ChartDisplayName::new("surface-D".to_owned()).unwrap(),
        1,
        SurfaceOrientation::Preserve,
        RationalSurfacePolynomials {
            x: monomial(2, 1, 0),
            y: monomial(2, 0, 1),
            z: monomial(2, 0, 0),
            denominator: monomial(2, 0, 0),
        },
    )
    .unwrap();
    let domain = SurfaceParameterDomain::new(
        ParameterInterval::new(algebraic(0), algebraic(8), true, true).unwrap(),
        ParameterInterval::new(algebraic(0), algebraic(8), true, true).unwrap(),
    );
    assert!(evaluate_surface_witness_exact(&chart, &domain, &algebraic(1), &algebraic(2)).is_ok());
    assert!(matches!(
        evaluate_surface_witness_exact(&chart, &domain, &algebraic(32), &algebraic(0)),
        Err(GeometryFailure::ParameterOutsideDomain)
    ));
    let enclosure = Enclosure::new(Dyadic::new(BigInt::try_from(1).unwrap(), 0), Dyadic::new(BigInt::try_from(2).unwrap(), 0)).unwrap();
    let requirement = ExactExpressionRequirement::new(
        ConsumerId::new(11),
        &[ExprId::new(20)],
        ExactDecisionRequest::Zero(ExprId::new(20)),
        AbsoluteBits::new(8),
    )
    .unwrap();
    assert!(evaluate_surface_witness_checked(
        &chart,
        &domain,
        &enclosure,
        &enclosure,
        OwnerId::new(1),
        requirement,
        NecoImplementationSource::current(),
    )
    .is_ok());
}
