use neco_algnum::RealAlgebraic;
use neco_bigint::{BigInt, Dyadic};
use neco_calculus::{DerivativeOrder, Enclosure, Polynomial};
use neco_geometry::{
    construct_region_types, derive_boundary, AlgebraicRegion, BoundaryCandidateSeed,
    BoundaryDerivationRequest, BoundaryRestrictionChart, BoundarySideSample, CanonicalAtom,
    NormalizedRegionFormula, RegionConstructionRequest, RegionFunction, RegionKind,
    RegionMembershipRequest, RegionPoint, RegionPolynomial3, RegionPolynomialTerm, SignRelation,
};
use neco_geometry_foundation::Point3;

fn algebraic(value: i32) -> RealAlgebraic {
    RealAlgebraic::from_integer(BigInt::try_from(value).unwrap()).unwrap()
}

fn term(coefficient: i32, x: u32, y: u32, z: u32) -> RegionPolynomialTerm {
    RegionPolynomialTerm::try_new(algebraic(coefficient), x, y, z).unwrap()
}

fn region() -> AlgebraicRegion {
    construct_region_types(RegionConstructionRequest {
        ambient_dimension: 3,
        canonical_atoms: vec![CanonicalAtom::try_new(
            "x",
            RegionFunction::Polynomial(
                RegionPolynomial3::try_from_terms(vec![term(1, 1, 0, 0)]).unwrap(),
            ),
            SignRelation::Positive,
        )
        .unwrap()],
        normalized_region_formula: NormalizedRegionFormula::Atom("x".into()),
        region_kind: RegionKind::GeneralSemiAlgebraicSet,
    })
    .unwrap()
    .into_region()
}

fn point(x: i32, y: i32, z: i32) -> RegionPoint {
    RegionPoint::Spatial(Point3 {
        x: algebraic(x),
        y: algebraic(y),
        z: algebraic(z),
    })
}

#[test]
fn preserves_boundary_clone_consumer() {
    let source = region();
    let chart = BoundaryRestrictionChart::try_new(
        "chart",
        Enclosure::new(
            Dyadic::new(BigInt::try_from(-1).unwrap(), 0),
            Dyadic::new(BigInt::try_from(1).unwrap(), 0),
        )
        .unwrap(),
        Polynomial::from_coefficients(vec![algebraic(0), algebraic(1)]).unwrap(),
        Polynomial::from_coefficients(vec![algebraic(0)]).unwrap(),
        Polynomial::from_coefficients(vec![algebraic(0)]).unwrap(),
    )
    .unwrap();
    let derived = derive_boundary(BoundaryDerivationRequest {
        region: region(),
        intersections: Vec::new(),
        candidate_loci: vec![BoundaryCandidateSeed::try_new(
            "x",
            chart,
            vec![Dyadic::new(BigInt::try_from(0).unwrap(), 0)],
            vec![
                BoundarySideSample::new(Dyadic::new(BigInt::try_from(-1).unwrap(), 0)),
                BoundarySideSample::new(Dyadic::new(BigInt::try_from(1).unwrap(), 0)),
            ],
            None,
        )
        .unwrap()],
        max_order: DerivativeOrder::new(3),
    })
    .unwrap();
    let cloned = derived.algebraic_boundary().source_region();

    for coordinates in [(1, 0, 0), (0, 0, 0), (-1, 2, 3)] {
        let original = source
            .membership(RegionMembershipRequest {
                region: region(),
                point: point(coordinates.0, coordinates.1, coordinates.2),
            })
            .unwrap();
        let after_clone = cloned
            .membership(RegionMembershipRequest {
                region: region(),
                point: point(coordinates.0, coordinates.1, coordinates.2),
            })
            .unwrap();
        assert_eq!(original.classification, after_clone.classification);
        assert_eq!(original.atom_signs, after_clone.atom_signs);
    }
}
