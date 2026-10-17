use std::string::String;

use neco_bigint::{BigInt, BigUint, RawRational};
use neco_calculus::{RationalSurfacePolynomials, SurfacePolynomial, SurfaceTerm};
use neco_geometry::{
    evaluate_surface_witness_checked, evaluate_surface_witness_exact, CanonicalAtom,
    ChartDisplayName, ChartDomainId, GeometryFailure, ParametricSurfaceChart2,
    SurfaceCrossingCandidate, SurfaceOrientation, SurfaceParameterDomain, SurfaceTrimLoop2,
    TrimLoopRole, TrimmedSurfaceDeclaration,
};

fn rational(value: i32) -> neco_bigint::ReducedRational {
    RawRational::new(BigInt::try_from(value).unwrap(), BigUint::one().unwrap())
        .reduce()
        .unwrap()
        .into_reduced()
}

fn algebraic(value: i32) -> neco_algnum::RealAlgebraic {
    neco_algnum::RealAlgebraic::from_integer(BigInt::try_from(value).unwrap()).unwrap()
}

fn constant(coefficient: i32) -> SurfacePolynomial {
    SurfacePolynomial::try_from_terms(vec![SurfaceTerm {
        coefficient: rational(coefficient),
        exponent_u: 0,
        exponent_v: 0,
    }])
    .unwrap()
}

#[test]
fn surface_witness_is_crate_root_reachable() {
    let _owned_atom: fn(&CanonicalAtom) -> Result<CanonicalAtom, GeometryFailure> =
        CanonicalAtom::try_clone;
    let chart = ParametricSurfaceChart2::try_new(
        ChartDomainId::new(String::from("public-D")).unwrap(),
        ChartDisplayName::new(String::from("public-D")).unwrap(),
        1,
        SurfaceOrientation::Preserve,
        RationalSurfacePolynomials {
            x: constant(1),
            y: constant(1),
            z: constant(1),
            denominator: constant(1),
        },
    )
    .unwrap();
    let domain = SurfaceParameterDomain::new(
        neco_geometry::ParameterInterval::new(algebraic(0), algebraic(2), true, true).unwrap(),
        neco_geometry::ParameterInterval::new(algebraic(0), algebraic(2), true, true).unwrap(),
    );
    let outer = SurfaceTrimLoop2::try_new(
        TrimLoopRole::Outer,
        vec![
            neco_geometry::SurfaceParameterPoint::new(algebraic(0), algebraic(0)),
            neco_geometry::SurfaceParameterPoint::new(algebraic(2), algebraic(0)),
            neco_geometry::SurfaceParameterPoint::new(algebraic(2), algebraic(2)),
            neco_geometry::SurfaceParameterPoint::new(algebraic(0), algebraic(2)),
        ],
        &domain,
    )
    .unwrap();
    let declaration =
        TrimmedSurfaceDeclaration::try_new(chart, domain, outer, Vec::new(), Vec::new()).unwrap();
    let _candidate = SurfaceCrossingCandidate::new(declaration);
    let _exact: fn(
        &ParametricSurfaceChart2,
        &SurfaceParameterDomain,
        &neco_algnum::RealAlgebraic,
        &neco_algnum::RealAlgebraic,
    ) -> Result<
        neco_geometry::SurfaceEvaluationWitness,
        neco_geometry::GeometryFailure,
    > = evaluate_surface_witness_exact;
    let _checked = evaluate_surface_witness_checked;
}
