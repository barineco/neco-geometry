use std::string::String;

use neco_algnum::RealAlgebraic;
use neco_bigint::{BigInt, BigUint, Dyadic, RawRational};
use neco_calculus::{
    AbsoluteBits, ConsumerId, Enclosure, ExactDecisionRequest, ExactExpressionRequirement,
    NecoImplementationSource, OwnerId, RationalSurfacePolynomials, SurfacePolynomial, SurfaceTerm,
};
use neco_expr::ExprId;
use neco_geometry::{
    evaluate_surface_witness_checked, evaluate_surface_witness_exact, ChartDisplayName,
    ChartDomainId, ChartPatch, DiskConstraint, GeometryFailure, PairedSurfaceSeam,
    ParametricSurfaceChart2, SeamContact, SeamEnd, SurfaceChartIdentity, SurfaceEvaluationWitness,
    SurfaceOrientation, SurfaceParameterDomain, SurfaceParameterPoint, SurfaceTrimLoop2,
    TrimClassification, TrimLoopRole, TrimmedSurfaceDeclaration,
};

#[path = "support/surface_encoding.rs"]
mod encoding;

fn rational(value: i32) -> neco_bigint::ReducedRational {
    RawRational::new(BigInt::try_from(value).unwrap(), BigUint::one().unwrap())
        .reduce()
        .unwrap()
        .into_reduced()
}

fn algebraic(value: i32) -> RealAlgebraic {
    RealAlgebraic::from_integer(BigInt::try_from(value).unwrap()).unwrap()
}

fn interval(lower: i32, upper: i32) -> neco_geometry::ParameterInterval {
    neco_geometry::ParameterInterval::new(algebraic(lower), algebraic(upper), true, true).unwrap()
}

fn domain() -> SurfaceParameterDomain {
    SurfaceParameterDomain::new(interval(0, 8), interval(0, 8))
}

fn point(u: i32, v: i32) -> SurfaceParameterPoint {
    SurfaceParameterPoint::new(algebraic(u), algebraic(v))
}

fn surface_term(coefficient: i32, exponent_u: u32, exponent_v: u32) -> SurfaceTerm {
    SurfaceTerm {
        coefficient: rational(coefficient),
        exponent_u,
        exponent_v,
    }
}

fn surface_polynomial(coefficient: i32, exponent_u: u32, exponent_v: u32) -> SurfacePolynomial {
    SurfacePolynomial::try_from_terms(vec![surface_term(coefficient, exponent_u, exponent_v)])
        .unwrap()
}

fn constant(coefficient: i32) -> SurfacePolynomial {
    surface_polynomial(coefficient, 0, 0)
}

fn polynomials(
    x: SurfacePolynomial,
    y: SurfacePolynomial,
    z: SurfacePolynomial,
    denominator: SurfacePolynomial,
) -> RationalSurfacePolynomials {
    RationalSurfacePolynomials {
        x,
        y,
        z,
        denominator,
    }
}

fn chart_identity(name: &str, version: u64) -> (ChartDomainId, ChartDisplayName, u64) {
    (
        ChartDomainId::new(String::from(name)).unwrap(),
        ChartDisplayName::new(String::from(name)).unwrap(),
        version,
    )
}

fn polynomial_chart() -> ParametricSurfaceChart2 {
    let (domain_id, display_name, version) = chart_identity("surface-D", 1);
    ParametricSurfaceChart2::try_new(
        domain_id,
        display_name,
        version,
        SurfaceOrientation::Preserve,
        polynomials(
            surface_polynomial(1, 1, 0),
            surface_polynomial(1, 0, 1),
            constant(1),
            constant(1),
        ),
    )
    .unwrap()
}

fn rational_chart() -> ParametricSurfaceChart2 {
    let (domain_id, display_name, version) = chart_identity("surface-D", 1);
    ParametricSurfaceChart2::try_new(
        domain_id,
        display_name,
        version,
        SurfaceOrientation::Preserve,
        polynomials(
            surface_polynomial(2, 1, 0),
            surface_polynomial(2, 0, 1),
            constant(2),
            constant(2),
        ),
    )
    .unwrap()
}

fn loop_vertices(vertices: &[(i32, i32)]) -> Vec<SurfaceParameterPoint> {
    vertices.iter().copied().map(|(u, v)| point(u, v)).collect()
}

fn outer_loop(vertices: &[(i32, i32)]) -> SurfaceTrimLoop2 {
    SurfaceTrimLoop2::try_new(TrimLoopRole::Outer, loop_vertices(vertices), &domain()).unwrap()
}

fn hole_loop(vertices: &[(i32, i32)]) -> SurfaceTrimLoop2 {
    SurfaceTrimLoop2::try_new(TrimLoopRole::Hole, loop_vertices(vertices), &domain()).unwrap()
}

fn identity_of(chart: &ParametricSurfaceChart2) -> SurfaceChartIdentity {
    SurfaceChartIdentity::new(
        chart.identity().domain_id().try_clone().unwrap(),
        chart.identity().version(),
    )
}

fn seam_end(
    chart: &ParametricSurfaceChart2,
    u: i32,
    v: i32,
    orientation: SurfaceOrientation,
) -> SeamEnd {
    SeamEnd::new(identity_of(chart), point(u, v), orientation)
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

fn lineage() -> (
    OwnerId,
    ExactExpressionRequirement,
    NecoImplementationSource,
) {
    (
        OwnerId::new(1),
        requirement(),
        NecoImplementationSource::current(),
    )
}

#[test]
fn constructs_polynomial_and_rational_surface_charts() {
    let polynomial = polynomial_chart();
    let rational = rational_chart();
    assert_eq!(
        polynomial.identity().domain_id().as_str(),
        rational.identity().domain_id().as_str()
    );
    assert_eq!(polynomial.identity().version(), 1);
    assert_eq!(polynomial.orientation(), SurfaceOrientation::Preserve);
    assert!(domain().contains(&algebraic(2), &algebraic(3)).unwrap());

    let exact_polynomial =
        evaluate_surface_witness_exact(&polynomial, &domain(), &algebraic(2), &algebraic(3))
            .unwrap();
    let exact_rational =
        evaluate_surface_witness_exact(&rational, &domain(), &algebraic(2), &algebraic(3)).unwrap();
    match (&exact_polynomial, &exact_rational) {
        (
            SurfaceEvaluationWitness::Exact {
                point: first,
                identity: first_identity,
            },
            SurfaceEvaluationWitness::Exact {
                point: second,
                identity: second_identity,
            },
        ) => {
            assert_eq!(first.x, algebraic(2));
            assert_eq!(first.y, algebraic(3));
            assert_eq!(first.z, algebraic(1));
            assert_eq!(second.x, algebraic(2));
            assert_eq!(second.y, algebraic(3));
            assert_eq!(second.z, algebraic(1));
            assert_eq!(
                first_identity.domain_id().as_str(),
                second_identity.domain_id().as_str()
            );
        }
        _ => panic!("exact evaluation produced a certified witness"),
    }
    let (owner, requirement, source) = lineage();
    let checked_rational = evaluate_surface_witness_checked(
        &rational,
        &domain(),
        &enclosure(2, 2),
        &enclosure(3, 3),
        owner,
        requirement,
        source,
    )
    .unwrap();
    match checked_rational {
        SurfaceEvaluationWitness::Certified { x, y, z, .. } => {
            assert!(x.contains(&Dyadic::new(BigInt::try_from(2).unwrap(), 0)));
            assert!(y.contains(&Dyadic::new(BigInt::try_from(3).unwrap(), 0)));
            assert!(z.contains(&Dyadic::new(BigInt::try_from(1).unwrap(), 0)));
        }
        _ => panic!("checked evaluation produced an exact witness"),
    }

    assert_eq!(
        evaluate_surface_witness_exact(&polynomial, &domain(), &algebraic(9), &algebraic(1)),
        Err(GeometryFailure::ParameterOutsideDomain)
    );

    let (domain_id, display_name, version) = chart_identity("surface-D", 1);
    assert_eq!(
        ParametricSurfaceChart2::try_new(
            domain_id,
            display_name,
            version,
            SurfaceOrientation::Preserve,
            polynomials(
                constant(1),
                constant(1),
                constant(1),
                SurfacePolynomial::try_from_terms(Vec::new()).unwrap(),
            ),
        ),
        Err(GeometryFailure::InvalidSurfaceChart)
    );
}

#[test]
fn classifies_outer_hole_and_trim_regions() {
    let declaration = TrimmedSurfaceDeclaration::try_new(
        polynomial_chart(),
        domain(),
        outer_loop(&[(1, 1), (7, 1), (7, 7), (1, 7)]),
        vec![hole_loop(&[(2, 2), (4, 2), (4, 4), (2, 4)])],
        Vec::new(),
    )
    .unwrap();

    assert_eq!(
        declaration.classify(&algebraic(6), &algebraic(6)).unwrap(),
        TrimClassification::Outer
    );
    assert_eq!(
        declaration.classify(&algebraic(3), &algebraic(3)).unwrap(),
        TrimClassification::Hole
    );
    assert_eq!(
        declaration.classify(&algebraic(0), &algebraic(0)).unwrap(),
        TrimClassification::TrimExterior
    );
    assert_eq!(
        declaration.classify(&algebraic(1), &algebraic(4)).unwrap(),
        TrimClassification::TrimBoundary
    );
    assert_eq!(
        declaration.classify(&algebraic(2), &algebraic(3)).unwrap(),
        TrimClassification::TrimBoundary
    );
    assert_eq!(
        declaration.classify(&algebraic(9), &algebraic(1)),
        Err(GeometryFailure::ParameterOutsideDomain)
    );
}

#[test]
fn rejects_invalid_trim_geometry() {
    let chart = polynomial_chart();
    let crossing = SurfaceTrimLoop2::try_new(
        TrimLoopRole::Outer,
        loop_vertices(&[(1, 1), (3, 3), (3, 1), (1, 3)]),
        &domain(),
    );
    assert_eq!(crossing, Err(GeometryFailure::SelfIntersectingTrimLoop));

    let outside = SurfaceTrimLoop2::try_new(
        TrimLoopRole::Outer,
        loop_vertices(&[(1, 1), (9, 1), (9, 3), (1, 3)]),
        &domain(),
    );
    assert_eq!(outside, Err(GeometryFailure::PCurveOutsideDomain));

    let intersecting_hole = TrimmedSurfaceDeclaration::try_new(
        chart,
        domain(),
        outer_loop(&[(1, 1), (5, 1), (5, 5), (1, 5)]),
        vec![hole_loop(&[(4, 2), (7, 2), (7, 4), (4, 4)])],
        Vec::new(),
    );
    assert_eq!(
        intersecting_hole,
        Err(GeometryFailure::HoleIntersectsOuterLoop)
    );
}

#[test]
fn preserves_paired_seam_identity_and_reports_unresolved_contact() {
    let chart = polynomial_chart();
    let paired = PairedSurfaceSeam::new(
        seam_end(&chart, 1, 2, SurfaceOrientation::Preserve),
        seam_end(&chart, 7, 2, SurfaceOrientation::Reverse),
    );
    let (foreign_id, _, _) = chart_identity("other-D", 1);
    let unresolved = PairedSurfaceSeam::new(
        seam_end(&chart, 1, 6, SurfaceOrientation::Preserve),
        SeamEnd::new(
            SurfaceChartIdentity::new(foreign_id, 1),
            point(7, 6),
            SurfaceOrientation::Preserve,
        ),
    );
    let declaration = TrimmedSurfaceDeclaration::try_new(
        chart,
        domain(),
        outer_loop(&[(1, 1), (7, 1), (7, 7), (1, 7)]),
        Vec::new(),
        vec![unresolved, paired],
    )
    .unwrap();

    assert!(matches!(
        declaration.seams(),
        [SeamContact::Paired(_), SeamContact::Undetermined { .. }]
    ));
    match &declaration.seams()[0] {
        SeamContact::Paired(seam) => {
            assert_eq!(seam.first().identity().domain_id().as_str(), "surface-D");
            assert_eq!(seam.second().identity().domain_id().as_str(), "surface-D");
            assert_eq!(seam.first().orientation(), SurfaceOrientation::Preserve);
            assert_eq!(seam.second().orientation(), SurfaceOrientation::Reverse);
        }
        _ => panic!("canonical seam order lost the paired contact"),
    }
    match &declaration.seams()[1] {
        SeamContact::Undetermined { first, second } => {
            assert_eq!(first.identity().domain_id().as_str(), "other-D");
            assert_eq!(second.identity().domain_id().as_str(), "surface-D");
        }
        _ => panic!("unresolved contact was classified as paired"),
    }
}

#[test]
fn exact_and_checked_witnesses_share_identity_and_checked_keeps_lineage() {
    let chart = polynomial_chart();
    let exact =
        evaluate_surface_witness_exact(&chart, &domain(), &algebraic(2), &algebraic(3)).unwrap();
    let (owner, checked_requirement, source) = lineage();
    let checked = evaluate_surface_witness_checked(
        &chart,
        &domain(),
        &enclosure(2, 2),
        &enclosure(3, 3),
        owner,
        checked_requirement,
        source,
    )
    .unwrap();

    match (&exact, &checked) {
        (
            SurfaceEvaluationWitness::Exact { identity, point },
            SurfaceEvaluationWitness::Certified {
                identity: checked_identity,
                x,
                y,
                z,
                owner: checked_owner,
                requirement: checked_requirement,
                source: checked_source,
            },
        ) => {
            assert_eq!(identity.domain_id().as_str(), "surface-D");
            assert_eq!(identity.version(), checked_identity.version());
            assert_eq!(
                identity.domain_id().as_str(),
                checked_identity.domain_id().as_str()
            );
            assert_eq!(point.x, algebraic(2));
            assert_eq!(point.y, algebraic(3));
            assert_eq!(point.z, algebraic(1));
            assert!(x.contains(&Dyadic::new(BigInt::try_from(2).unwrap(), 0)));
            assert!(y.contains(&Dyadic::new(BigInt::try_from(3).unwrap(), 0)));
            assert!(z.contains(&Dyadic::new(BigInt::try_from(1).unwrap(), 0)));
            assert_eq!(*checked_owner, OwnerId::new(1));
            assert_eq!(checked_requirement, &requirement());
            assert_eq!(*checked_source, NecoImplementationSource::current());
        }
        _ => panic!("exact and checked witnesses used different producers"),
    }
}

#[test]
fn preserves_trim_canonicalization_under_permutation() {
    let chart = polynomial_chart();
    let first_hole = [(3, 3), (5, 3), (5, 5), (3, 5)];
    let second_hole = [(2, 2), (3, 2), (3, 3), (2, 3)];
    let seam_pair = |first: (i32, i32, SurfaceOrientation),
                     second: (i32, i32, SurfaceOrientation)| {
        PairedSurfaceSeam::new(
            seam_end(&chart, first.0, first.1, first.2),
            seam_end(&chart, second.0, second.1, second.2),
        )
    };
    let baseline = TrimmedSurfaceDeclaration::try_new(
        polynomial_chart(),
        domain(),
        outer_loop(&[(1, 1), (7, 1), (7, 7), (1, 7)]),
        vec![hole_loop(&first_hole), hole_loop(&second_hole)],
        vec![
            seam_pair(
                (7, 4, SurfaceOrientation::Reverse),
                (1, 4, SurfaceOrientation::Preserve),
            ),
            seam_pair(
                (4, 7, SurfaceOrientation::Preserve),
                (4, 1, SurfaceOrientation::Preserve),
            ),
        ],
    )
    .unwrap();
    let rotated = TrimmedSurfaceDeclaration::try_new(
        polynomial_chart(),
        domain(),
        outer_loop(&[(7, 1), (7, 7), (1, 7), (1, 1)]),
        vec![hole_loop(&second_hole), hole_loop(&first_hole)],
        vec![
            seam_pair(
                (4, 7, SurfaceOrientation::Preserve),
                (4, 1, SurfaceOrientation::Preserve),
            ),
            seam_pair(
                (7, 4, SurfaceOrientation::Reverse),
                (1, 4, SurfaceOrientation::Preserve),
            ),
        ],
    )
    .unwrap();
    let reversed = TrimmedSurfaceDeclaration::try_new(
        polynomial_chart(),
        domain(),
        outer_loop(&[(1, 1), (1, 7), (7, 7), (7, 1)]),
        vec![
            hole_loop(&[(3, 2), (3, 3), (2, 3), (2, 2)]),
            hole_loop(&[(5, 3), (5, 5), (3, 5), (3, 3)]),
        ],
        vec![
            seam_pair(
                (4, 1, SurfaceOrientation::Preserve),
                (4, 7, SurfaceOrientation::Preserve),
            ),
            seam_pair(
                (1, 4, SurfaceOrientation::Preserve),
                (7, 4, SurfaceOrientation::Reverse),
            ),
        ],
    )
    .unwrap();

    assert_eq!(baseline.outer().vertices(), rotated.outer().vertices());
    assert_eq!(baseline.outer().vertices(), reversed.outer().vertices());
    assert_eq!(baseline.holes().len(), rotated.holes().len());
    assert_eq!(baseline.holes().len(), reversed.holes().len());
    assert_eq!(baseline.seams().len(), rotated.seams().len());
    assert_eq!(baseline.seams().len(), reversed.seams().len());
    for sample in [(6, 6), (2, 2), (3, 4), (0, 0), (1, 4)] {
        let expected = baseline
            .classify(&algebraic(sample.0), &algebraic(sample.1))
            .unwrap();
        assert_eq!(
            rotated
                .classify(&algebraic(sample.0), &algebraic(sample.1))
                .unwrap(),
            expected
        );
        assert_eq!(
            reversed
                .classify(&algebraic(sample.0), &algebraic(sample.1))
                .unwrap(),
            expected
        );
    }
    assert_eq!(
        baseline.chart().identity().domain_id().as_str(),
        reversed.chart().identity().domain_id().as_str()
    );
}

fn algebraic_ratio(numerator: i32, denominator: i32) -> RealAlgebraic {
    algebraic(numerator).div(&algebraic(denominator)).unwrap()
}

fn stereographic_chart() -> ParametricSurfaceChart2 {
    let (domain_id, display_name, version) = chart_identity("stereographic-sphere", 1);
    ParametricSurfaceChart2::try_new(
        domain_id,
        display_name,
        version,
        SurfaceOrientation::Preserve,
        polynomials(
            surface_polynomial(2, 1, 0),
            surface_polynomial(2, 0, 1),
            SurfacePolynomial::try_from_terms(vec![
                surface_term(1, 0, 0),
                surface_term(-1, 2, 0),
                surface_term(-1, 0, 2),
            ])
            .unwrap(),
            SurfacePolynomial::try_from_terms(vec![
                surface_term(1, 0, 0),
                surface_term(1, 2, 0),
                surface_term(1, 0, 2),
            ])
            .unwrap(),
        ),
    )
    .unwrap()
}

#[test]
fn full_patch_preserves_stereographic_equator_and_borrowed_declarations() {
    let patch = ChartPatch::Full {
        chart: stereographic_chart(),
        domain: SurfaceParameterDomain::new(interval(-1, 1), interval(-1, 1)),
        disk: Some(DiskConstraint),
    };
    if let ChartPatch::Full {
        chart,
        domain,
        disk,
    } = &patch
    {
        assert!(core::ptr::eq(patch.chart(), chart));
        assert!(core::ptr::eq(patch.domain(), domain));
        assert!(core::ptr::eq(
            patch.disk_constraint().unwrap(),
            disk.as_ref().unwrap()
        ));
    } else {
        panic!("full patch changed variant");
    }
    assert!(patch.trimmed().is_none());
    assert_eq!(patch.chart().orientation(), SurfaceOrientation::Preserve);
    assert_eq!(
        patch.domain(),
        &SurfaceParameterDomain::new(interval(-1, 1), interval(-1, 1))
    );
    let u = algebraic_ratio(3, 5);
    let v = algebraic_ratio(4, 5);
    assert!(patch.disk_constraint().unwrap().contains(&u, &v).unwrap());
    match evaluate_surface_witness_exact(patch.chart(), patch.domain(), &u, &v).unwrap() {
        SurfaceEvaluationWitness::Exact { identity, point } => {
            assert_eq!(&identity, patch.chart().identity());
            assert_eq!(point.x, u);
            assert_eq!(point.y, v);
            assert_eq!(point.z, algebraic(0));
        }
        _ => panic!("exact evaluation returned a non-exact witness"),
    }

    let unconstrained = ChartPatch::Full {
        chart: polynomial_chart(),
        domain: domain(),
        disk: None,
    };
    assert!(unconstrained.disk_constraint().is_none());
    assert!(unconstrained.trimmed().is_none());
    assert!(unconstrained
        .domain()
        .contains(&algebraic(2), &algebraic(2))
        .unwrap());
}

#[test]
fn disk_membership_is_closed_exact_and_invariant_under_reflections_and_permutation() {
    let denominator = 1 << 24;
    for (u, v, expected) in [
        (algebraic(0), algebraic(0), true),
        (algebraic(1), algebraic(0), true),
        (algebraic_ratio(3, 5), algebraic_ratio(4, 5), true),
        (algebraic(2), algebraic(0), false),
        (
            algebraic_ratio(1, 2).nth_root(2).unwrap(),
            algebraic_ratio(1, 2).nth_root(2).unwrap(),
            true,
        ),
        (algebraic(2).nth_root(2).unwrap(), algebraic(0), false),
        (
            algebraic_ratio(denominator - 1, denominator),
            algebraic(0),
            true,
        ),
        (
            algebraic_ratio(denominator + 1, denominator),
            algebraic(0),
            false,
        ),
    ] {
        assert_eq!(DiskConstraint.contains(&u, &v).unwrap(), expected);
        assert_eq!(
            DiskConstraint
                .contains(&algebraic(0).sub(&u).unwrap(), &v)
                .unwrap(),
            expected
        );
        assert_eq!(
            DiskConstraint
                .contains(&u, &algebraic(0).sub(&v).unwrap())
                .unwrap(),
            expected
        );
        assert_eq!(DiskConstraint.contains(&v, &u).unwrap(), expected);
    }
}

#[test]
fn trimmed_patch_borrows_holes_seams_and_preserves_parameter_classification() {
    let chart = polynomial_chart();
    let seam = PairedSurfaceSeam::new(
        seam_end(&chart, 1, 2, SurfaceOrientation::Preserve),
        seam_end(&chart, 7, 2, SurfaceOrientation::Reverse),
    );
    let declaration = TrimmedSurfaceDeclaration::try_new(
        chart,
        domain(),
        outer_loop(&[(1, 1), (7, 1), (7, 7), (1, 7)]),
        vec![hole_loop(&[(2, 2), (4, 2), (4, 4), (2, 4)])],
        vec![seam],
    )
    .unwrap();
    let samples = [(6, 6), (3, 3), (1, 4), (2, 3), (0, 0), (9, 0)];
    let expected = samples.map(|(u, v)| declaration.classify(&algebraic(u), &algebraic(v)));
    assert_eq!(
        expected,
        [
            Ok(TrimClassification::Outer),
            Ok(TrimClassification::Hole),
            Ok(TrimClassification::TrimBoundary),
            Ok(TrimClassification::TrimBoundary),
            Ok(TrimClassification::TrimExterior),
            Err(GeometryFailure::ParameterOutsideDomain),
        ]
    );
    let patch = ChartPatch::Trimmed { declaration };
    let trimmed = patch.trimmed().unwrap();
    assert!(core::ptr::eq(patch.chart(), trimmed.chart()));
    assert!(core::ptr::eq(patch.domain(), trimmed.domain()));
    assert!(patch.disk_constraint().is_none());
    assert_eq!(trimmed.holes().len(), 1);
    assert!(matches!(trimmed.seams(), [SeamContact::Paired(_)]));
    if let SeamContact::Paired(seam) = &trimmed.seams()[0] {
        assert_eq!(seam.first().identity().domain_id().as_str(), "surface-D");
        assert_eq!(seam.second().identity().domain_id().as_str(), "surface-D");
        assert_eq!(seam.first().orientation(), SurfaceOrientation::Preserve);
        assert_eq!(seam.second().orientation(), SurfaceOrientation::Reverse);
    }
    for ((u, v), expected) in samples.into_iter().zip(expected) {
        assert_eq!(trimmed.classify(&algebraic(u), &algebraic(v)), expected);
    }
}
