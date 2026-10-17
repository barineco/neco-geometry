use std::string::String;

use neco_algnum::RealAlgebraic;
use neco_bigint::{BigInt, BigUint, RawRational};
use neco_calculus::{
    AbsoluteBits, ConsumerId, Enclosure, ExactDecisionRequest, ExactExpressionRequirement,
    NecoImplementationSource, OwnerId, RationalSurfacePolynomials, SurfacePolynomial, SurfaceTerm,
};
use neco_expr::ExprId;
use neco_geometry::{
    cross_checked_ray3, cross_ray3, evaluate_surface_witness_exact, ChartDisplayName,
    ChartDomainId, CheckedRay3, CheckedRayCrossingResult, CheckedRayCrossingTarget,
    CheckedRayCrossingTargetSet, CheckedTriangleTarget, CheckedTrimmedParametricSurfaceTarget,
    CrossingContribution, CrossingMultiplicity, CrossingParity, Direction3, ExactRayCrossingTarget,
    ExactRayCrossingTargetSet, ExactTriangleTarget, GeometryFailure, ImplicitSurfaceDeclaration,
    MultivariatePolynomial3, PairedSurfaceSeam, ParametricSurfaceChart2, Point3, PolynomialTerm3,
    PositiveRayParameterRange, Ray3, RayCrossingLocation, RayCrossingPolicy, RayCrossingResult,
    RayCrossingTargetId, RayCrossingUndeterminedReason, RayNormal, RayOriginContact,
    RootIsolationPolicy, RootRefinementMethod, SeamEnd, SurfaceCrossingCandidate,
    SurfaceOrientation, SurfaceParameterDomain, SurfaceParameterPoint, SurfaceTrimLoop2,
    TrimClassification, TrimLoopRole, TrimmedParametricSurfaceTarget, TrimmedSurfaceDeclaration,
    Vector3,
};

fn algebraic(value: i32) -> RealAlgebraic {
    RealAlgebraic::from_integer(BigInt::try_from(value).unwrap()).unwrap()
}

fn ratio(numerator: i32, denominator: u32) -> RealAlgebraic {
    let rational = RawRational::new(
        BigInt::try_from(numerator).unwrap(),
        BigUint::try_from(denominator).unwrap(),
    )
    .reduce()
    .unwrap()
    .into_reduced();
    RealAlgebraic::from_reduced_rational(&rational).unwrap()
}

fn point(x: i32, y: i32, z: i32) -> Point3 {
    Point3 {
        x: algebraic(x),
        y: algebraic(y),
        z: algebraic(z),
    }
}

fn point_r(x: RealAlgebraic, y: RealAlgebraic, z: RealAlgebraic) -> Point3 {
    Point3 { x, y, z }
}

fn ray(origin: Point3, direction: Vector3) -> Ray3 {
    Ray3::new(origin, Direction3::try_from_vector(direction).unwrap())
}

fn z_ray(origin: Point3) -> Ray3 {
    ray(
        origin,
        Vector3 {
            x: algebraic(0),
            y: algebraic(0),
            z: algebraic(1),
        },
    )
}

fn triangle(id: &str) -> ExactTriangleTarget {
    ExactTriangleTarget::new(
        RayCrossingTargetId::new(id).unwrap(),
        point(0, 0, 0),
        point(1, 0, 0),
        point(0, 1, 0),
    )
    .unwrap()
}

fn xy_triangle(
    id: &str,
    a: (i32, i32, i32),
    b: (i32, i32, i32),
    c: (i32, i32, i32),
) -> ExactTriangleTarget {
    ExactTriangleTarget::new(
        RayCrossingTargetId::new(id).unwrap(),
        point(a.0, a.1, a.2),
        point(b.0, b.1, b.2),
        point(c.0, c.1, c.2),
    )
    .unwrap()
}

fn root_policy() -> RootIsolationPolicy {
    RootIsolationPolicy::new(
        8,
        RootRefinementMethod::Bisection,
        Enclosure::new(
            neco_bigint::Dyadic::new(BigInt::zero(), 0),
            neco_bigint::Dyadic::new(BigInt::one().unwrap(), 8),
        )
        .unwrap(),
    )
    .unwrap()
}

fn policy(max_targets: usize, max_hits: usize) -> RayCrossingPolicy {
    RayCrossingPolicy::new(root_policy(), max_targets, max_hits).unwrap()
}

fn provenance() -> (
    OwnerId,
    ExactExpressionRequirement,
    NecoImplementationSource,
) {
    (
        OwnerId::new(1),
        ExactExpressionRequirement::new(
            ConsumerId::new(11),
            &[ExprId::new(500)],
            ExactDecisionRequest::Zero(ExprId::new(500)),
            AbsoluteBits::new(50),
        )
        .unwrap(),
        NecoImplementationSource::current(),
    )
}
fn requirement_for_test() -> ExactExpressionRequirement {
    provenance().1
}

fn rational(value: i32) -> neco_bigint::ReducedRational {
    RawRational::new(BigInt::try_from(value).unwrap(), BigUint::one().unwrap())
        .reduce()
        .unwrap()
        .into_reduced()
}

fn constant(coefficient: i32) -> SurfacePolynomial {
    SurfacePolynomial::try_from_terms(vec![SurfaceTerm {
        coefficient: rational(coefficient),
        exponent_u: 0,
        exponent_v: 0,
    }])
    .unwrap()
}

fn linear_u(coefficient: i32) -> SurfacePolynomial {
    SurfacePolynomial::try_from_terms(vec![SurfaceTerm {
        coefficient: rational(coefficient),
        exponent_u: 1,
        exponent_v: 0,
    }])
    .unwrap()
}

fn linear_v(coefficient: i32) -> SurfacePolynomial {
    SurfacePolynomial::try_from_terms(vec![SurfaceTerm {
        coefficient: rational(coefficient),
        exponent_u: 0,
        exponent_v: 1,
    }])
    .unwrap()
}

fn parameter(u: i32, v: i32) -> SurfaceParameterPoint {
    SurfaceParameterPoint::new(algebraic(u), algebraic(v))
}

fn domain() -> SurfaceParameterDomain {
    SurfaceParameterDomain::new(
        neco_geometry::ParameterInterval::new(algebraic(0), algebraic(6), true, true).unwrap(),
        neco_geometry::ParameterInterval::new(algebraic(0), algebraic(6), true, true).unwrap(),
    )
}

fn loop2(role: TrimLoopRole, vertices: &[(i32, i32)]) -> SurfaceTrimLoop2 {
    SurfaceTrimLoop2::try_new(
        role,
        vertices.iter().map(|(u, v)| parameter(*u, *v)).collect(),
        &domain(),
    )
    .unwrap()
}

fn polynomial_chart() -> ParametricSurfaceChart2 {
    ParametricSurfaceChart2::try_new(
        ChartDomainId::new(String::from("poly-D")).unwrap(),
        ChartDisplayName::new(String::from("poly-D")).unwrap(),
        1,
        SurfaceOrientation::Preserve,
        RationalSurfacePolynomials {
            x: linear_u(1),
            y: linear_v(1),
            z: constant(1),
            denominator: constant(1),
        },
    )
    .unwrap()
}

fn rational_chart() -> ParametricSurfaceChart2 {
    ParametricSurfaceChart2::try_new(
        ChartDomainId::new(String::from("rat-D")).unwrap(),
        ChartDisplayName::new(String::from("rat-D")).unwrap(),
        2,
        SurfaceOrientation::Preserve,
        RationalSurfacePolynomials {
            x: linear_u(2),
            y: linear_v(2),
            z: constant(2),
            denominator: constant(2),
        },
    )
    .unwrap()
}

fn trimmed_declaration(
    chart: ParametricSurfaceChart2,
    outer: &[(i32, i32)],
    holes: &[&[(i32, i32)]],
    seams: Vec<PairedSurfaceSeam>,
) -> TrimmedSurfaceDeclaration {
    TrimmedSurfaceDeclaration::try_new(
        chart,
        domain(),
        loop2(TrimLoopRole::Outer, outer),
        holes
            .iter()
            .map(|hole| loop2(TrimLoopRole::Hole, hole))
            .collect(),
        seams,
    )
    .unwrap()
}

fn surface_target(id: &str, chart: ParametricSurfaceChart2) -> ExactRayCrossingTarget {
    let declaration = trimmed_declaration(
        chart,
        &[(0, 0), (4, 0), (4, 4), (0, 4)],
        &[&[(1, 1), (2, 1), (2, 2), (1, 2)]],
        Vec::new(),
    );
    ExactRayCrossingTarget::TrimmedSurface(TrimmedParametricSurfaceTarget::new(
        RayCrossingTargetId::new(id).unwrap(),
        SurfaceCrossingCandidate::new(declaration),
    ))
}

fn sphere_surface() -> ImplicitSurfaceDeclaration {
    ImplicitSurfaceDeclaration::new(
        MultivariatePolynomial3::try_from_terms(vec![
            PolynomialTerm3::new(algebraic(1), 2, 0, 0).unwrap(),
            PolynomialTerm3::new(algebraic(1), 0, 2, 0).unwrap(),
            PolynomialTerm3::new(algebraic(1), 0, 0, 2).unwrap(),
            PolynomialTerm3::new(algebraic(-1), 0, 0, 0).unwrap(),
        ])
        .unwrap(),
    )
}

fn implicit_target(id: &str, surface: ImplicitSurfaceDeclaration) -> ExactRayCrossingTarget {
    ExactRayCrossingTarget::ImplicitSurface {
        id: RayCrossingTargetId::new(id).unwrap(),
        surface,
    }
}

fn proved(
    result: RayCrossingResult,
) -> (
    RayOriginContact,
    CrossingParity,
    Vec<neco_geometry::RayCrossingHit>,
) {
    match result {
        RayCrossingResult::Proved {
            origin_contact,
            parity,
            hits,
            ..
        } => (origin_contact, parity, hits),
        other => panic!("expected proved crossing, got {other:?}"),
    }
}

fn undetermined(
    result: RayCrossingResult,
) -> (
    RayCrossingUndeterminedReason,
    RayOriginContact,
    Vec<neco_geometry::RayCrossingHit>,
) {
    match result {
        RayCrossingResult::Undetermined {
            reason,
            origin_contact,
            hits,
            ..
        } => (reason, origin_contact, hits),
        other => panic!("expected undetermined crossing, got {other:?}"),
    }
}

fn cross_triangles(ray: Ray3, triangles: Vec<ExactTriangleTarget>) -> RayCrossingResult {
    let count = triangles.len();
    let targets = ExactRayCrossingTargetSet::new(
        triangles
            .into_iter()
            .map(ExactRayCrossingTarget::Triangle)
            .collect(),
    )
    .unwrap();
    cross_ray3(
        ray,
        targets,
        PositiveRayParameterRange::new(),
        policy(count, 16),
    )
    .unwrap()
}

fn tetrahedron() -> Vec<ExactTriangleTarget> {
    vec![
        xy_triangle("abc", (0, 0, 0), (2, 0, 0), (0, 2, 0)),
        xy_triangle("abd", (0, 0, 0), (2, 0, 0), (0, 0, 2)),
        xy_triangle("acd", (0, 0, 0), (0, 2, 0), (0, 0, 2)),
        xy_triangle("bcd", (2, 0, 0), (0, 2, 0), (0, 0, 2)),
    ]
}

fn octahedron() -> Vec<ExactTriangleTarget> {
    let plus_x = (1, 0, 0);
    let minus_x = (-1, 0, 0);
    let plus_y = (0, 1, 0);
    let minus_y = (0, -1, 0);
    let plus_z = (0, 0, 1);
    let minus_z = (0, 0, -1);
    vec![
        xy_triangle("++z-xy", plus_x, plus_y, plus_z),
        xy_triangle("+-z-xy", plus_x, minus_y, plus_z),
        xy_triangle("-+z-xy", minus_x, plus_y, plus_z),
        xy_triangle("--z-xy", minus_x, minus_y, plus_z),
        xy_triangle("++z+xy", plus_x, plus_y, minus_z),
        xy_triangle("+-z+xy", plus_x, minus_y, minus_z),
        xy_triangle("-+z+xy", minus_x, plus_y, minus_z),
        xy_triangle("--z+xy", minus_x, minus_y, minus_z),
    ]
}

fn cube() -> Vec<ExactTriangleTarget> {
    vec![
        xy_triangle("z0a", (0, 0, 0), (2, 0, 0), (0, 2, 0)),
        xy_triangle("z0b", (2, 0, 0), (2, 2, 0), (0, 2, 0)),
        xy_triangle("z2a", (0, 0, 2), (2, 0, 2), (0, 2, 2)),
        xy_triangle("z2b", (2, 0, 2), (2, 2, 2), (0, 2, 2)),
        xy_triangle("y0a", (0, 0, 0), (2, 0, 0), (0, 0, 2)),
        xy_triangle("y0b", (2, 0, 0), (2, 0, 2), (0, 0, 2)),
        xy_triangle("y2a", (0, 2, 0), (2, 2, 0), (0, 2, 2)),
        xy_triangle("y2b", (2, 2, 0), (2, 2, 2), (0, 2, 2)),
        xy_triangle("x0a", (0, 0, 0), (0, 2, 0), (0, 0, 2)),
        xy_triangle("x0b", (0, 2, 0), (0, 2, 2), (0, 0, 2)),
        xy_triangle("x2a", (2, 0, 0), (2, 2, 0), (2, 0, 2)),
        xy_triangle("x2b", (2, 2, 0), (2, 2, 2), (2, 0, 2)),
    ]
}

fn concave() -> Vec<ExactTriangleTarget> {
    vec![
        xy_triangle("l0", (0, 0, 0), (4, 0, 0), (0, 2, 0)),
        xy_triangle("l1", (4, 0, 0), (4, 2, 0), (0, 2, 0)),
        xy_triangle("l2", (0, 2, 0), (2, 2, 0), (0, 4, 0)),
        xy_triangle("l3", (2, 2, 0), (2, 4, 0), (0, 4, 0)),
        xy_triangle("u0", (0, 0, 2), (4, 0, 2), (0, 2, 2)),
        xy_triangle("u1", (4, 0, 2), (4, 2, 2), (0, 2, 2)),
        xy_triangle("u2", (0, 2, 2), (2, 2, 2), (0, 4, 2)),
        xy_triangle("u3", (2, 2, 2), (2, 4, 2), (0, 4, 2)),
    ]
}

#[test]
fn positive_parameter_domain_is_unbounded_and_excludes_origin() {
    let domain = PositiveRayParameterRange::new();
    assert!(!domain.contains_exact(&algebraic(0)).unwrap());
    assert!(!domain.contains_exact(&algebraic(-1)).unwrap());
    assert!(domain.contains_exact(&algebraic(1_000_000)).unwrap());
    assert!(!domain.contains_checked(0.0).unwrap());
    assert!(domain.contains_checked(f64::MAX).unwrap());
    assert!(!domain.contains_checked(f64::NAN).unwrap());
    assert!(!domain.contains_checked(f64::INFINITY).unwrap());
}

#[test]
fn rejects_empty_target_set() {
    assert_eq!(
        ExactRayCrossingTargetSet::new(Vec::new()),
        Err(GeometryFailure::EmptyRayCrossingTargetSet)
    );
}

#[test]
fn rejects_duplicate_target_identity() {
    assert_eq!(
        ExactRayCrossingTargetSet::new(vec![
            ExactRayCrossingTarget::Triangle(triangle("same")),
            ExactRayCrossingTarget::Triangle(triangle("same")),
        ]),
        Err(GeometryFailure::DuplicateRayCrossingTargetIdentity)
    );
}

#[test]
fn rejects_mixed_target_family() {
    assert_eq!(
        ExactRayCrossingTargetSet::new(vec![
            ExactRayCrossingTarget::Triangle(triangle("triangle")),
            implicit_target(
                "implicit",
                ImplicitSurfaceDeclaration::new(MultivariatePolynomial3::zero())
            ),
        ]),
        Err(GeometryFailure::MixedRayCrossingTargetFamily)
    );
}

#[test]
fn rejects_degenerate_triangle() {
    assert_eq!(
        ExactTriangleTarget::new(
            RayCrossingTargetId::new("exact").unwrap(),
            point(0, 0, 0),
            point(1, 1, 1),
            point(2, 2, 2),
        ),
        Err(GeometryFailure::DegenerateTriangle)
    );
    let (owner, requirement, source) = provenance();
    assert_eq!(
        CheckedTriangleTarget::new(
            RayCrossingTargetId::new("checked").unwrap(),
            [[0.0, 0.0, 0.0], [1.0, 1.0, 1.0], [2.0, 2.0, 2.0]],
            owner,
            requirement,
            source,
        ),
        Err(GeometryFailure::DegenerateTriangle)
    );
}

#[test]
fn rejects_invalid_policy() {
    assert_eq!(
        RayCrossingPolicy::new(root_policy(), 0, 1),
        Err(GeometryFailure::InvalidRayCrossingPolicy)
    );
}

#[test]
fn rejects_target_limit_excess() {
    let targets = ExactRayCrossingTargetSet::new(vec![
        ExactRayCrossingTarget::Triangle(triangle("a")),
        ExactRayCrossingTarget::Triangle(triangle("b")),
    ])
    .unwrap();
    assert_eq!(
        cross_ray3(
            z_ray(point(0, 0, -1)),
            targets,
            PositiveRayParameterRange::new(),
            policy(1, 8),
        ),
        Err(GeometryFailure::RayCrossingTargetLimitExceeded)
    );
}

#[test]
fn rejects_hit_limit_excess() {
    let targets = ExactRayCrossingTargetSet::new(vec![
        ExactRayCrossingTarget::Triangle(xy_triangle("z0", (-1, -1, 0), (2, -1, 0), (-1, 2, 0))),
        ExactRayCrossingTarget::Triangle(xy_triangle("z1", (-1, -1, 1), (2, -1, 1), (-1, 2, 1))),
    ])
    .unwrap();
    assert_eq!(
        cross_ray3(
            z_ray(point(0, 0, -1)),
            targets,
            PositiveRayParameterRange::new(),
            policy(2, 1),
        ),
        Err(GeometryFailure::RayCrossingHitLimitExceeded)
    );
}

#[test]
fn classifies_declared_triangle_target_sets() {
    let interior = proved(cross_triangles(
        z_ray(point_r(ratio(1, 2), ratio(1, 2), ratio(1, 2))),
        tetrahedron(),
    ));
    assert_eq!(interior.0, RayOriginContact::None);
    assert_eq!(interior.1, CrossingParity::Odd);

    let face = proved(cross_triangles(
        z_ray(point_r(ratio(1, 2), ratio(1, 2), algebraic(0))),
        tetrahedron(),
    ));
    assert_eq!(face.0, RayOriginContact::Interior);

    let exterior = proved(cross_triangles(z_ray(point(5, 5, 5)), tetrahedron()));
    assert_eq!(exterior.0, RayOriginContact::None);
    assert_eq!(exterior.1, CrossingParity::Even);

    let octa = proved(cross_triangles(
        ray(
            point(0, 0, 0),
            Vector3 {
                x: algebraic(1),
                y: algebraic(1),
                z: algebraic(1),
            },
        ),
        octahedron(),
    ));
    assert_eq!(octa.1, CrossingParity::Odd);

    let cube_interior = proved(cross_triangles(
        z_ray(point_r(ratio(1, 2), ratio(1, 2), algebraic(1))),
        cube(),
    ));
    assert_eq!(cube_interior.1, CrossingParity::Odd);

    let concave_interior = proved(cross_triangles(z_ray(point(1, 1, 1)), concave()));
    assert_eq!(concave_interior.1, CrossingParity::Odd);
    let concave_notch = proved(cross_triangles(z_ray(point(3, 3, 1)), concave()));
    assert_eq!(concave_notch.1, CrossingParity::Even);
}

#[test]
fn preserves_triangle_crossing_under_target_permutation() {
    let mut triangles = tetrahedron();
    let first = proved(cross_triangles(
        z_ray(point_r(ratio(1, 2), ratio(1, 2), ratio(1, 2))),
        tetrahedron(),
    ));
    triangles.reverse();
    let second = proved(cross_triangles(
        z_ray(point_r(ratio(1, 2), ratio(1, 2), ratio(1, 2))),
        triangles,
    ));
    assert_eq!(first.0, second.0);
    assert_eq!(first.1, second.1);
    assert_eq!(first.2.len(), second.2.len());
    let mut first_ids: Vec<_> = first
        .2
        .iter()
        .map(|hit| hit.target_id().as_str().to_owned())
        .collect();
    let mut second_ids: Vec<_> = second
        .2
        .iter()
        .map(|hit| hit.target_id().as_str().to_owned())
        .collect();
    first_ids.sort();
    second_ids.sort();
    assert_eq!(first_ids, second_ids);
}

#[test]
fn returns_typed_triangle_indeterminacy() {
    let edge = undetermined(cross_triangles(
        z_ray(point(1, 0, -1)),
        vec![xy_triangle("edge", (0, 0, 0), (2, 0, 0), (0, 2, 0))],
    ));
    assert_eq!(edge.0, RayCrossingUndeterminedReason::Edge);

    let vertex = undetermined(cross_triangles(
        z_ray(point(0, 0, -1)),
        vec![xy_triangle("vertex", (0, 0, 0), (2, 0, 0), (0, 2, 0))],
    ));
    assert_eq!(vertex.0, RayCrossingUndeterminedReason::Vertex);

    let duplicate = undetermined(cross_triangles(
        z_ray(point(0, 0, -1)),
        vec![
            xy_triangle("first", (-1, -1, 0), (2, -1, 0), (-1, 2, 0)),
            xy_triangle("second", (-2, -2, 0), (3, -2, 0), (-2, 3, 0)),
        ],
    ));
    assert_eq!(
        duplicate.0,
        RayCrossingUndeterminedReason::DuplicateParameter
    );

    let coplanar = undetermined(cross_triangles(
        ray(
            point(0, 0, 0),
            Vector3 {
                x: algebraic(1),
                y: algebraic(0),
                z: algebraic(0),
            },
        ),
        vec![xy_triangle("plane", (-1, -1, 0), (2, -1, 0), (-1, 2, 0))],
    ));
    assert_eq!(coplanar.0, RayCrossingUndeterminedReason::CoplanarOverlap);
}

#[test]
fn preserves_crossing_under_common_translation_and_positive_scale() {
    let origin = point_r(ratio(1, 2), ratio(1, 2), ratio(1, 2));
    let base = proved(cross_triangles(z_ray(origin), tetrahedron()));
    let translated_targets = vec![
        xy_triangle("abc", (3, 0, 0), (5, 0, 0), (3, 2, 0)),
        xy_triangle("abd", (3, 0, 0), (5, 0, 0), (3, 0, 2)),
        xy_triangle("acd", (3, 0, 0), (3, 2, 0), (3, 0, 2)),
        xy_triangle("bcd", (5, 0, 0), (3, 2, 0), (3, 0, 2)),
    ];
    let translated_result = proved(cross_triangles(
        z_ray(point_r(ratio(7, 2), ratio(1, 2), ratio(1, 2))),
        translated_targets,
    ));
    assert_eq!(base.0, translated_result.0);
    assert_eq!(base.1, translated_result.1);

    let scaled_targets = vec![
        xy_triangle("abc", (0, 0, 0), (4, 0, 0), (0, 4, 0)),
        xy_triangle("abd", (0, 0, 0), (4, 0, 0), (0, 0, 4)),
        xy_triangle("acd", (0, 0, 0), (0, 4, 0), (0, 0, 4)),
        xy_triangle("bcd", (4, 0, 0), (0, 4, 0), (0, 0, 4)),
    ];
    let scaled = proved(cross_triangles(z_ray(point(1, 1, 1)), scaled_targets));
    assert_eq!(base.0, scaled.0);
    assert_eq!(base.1, scaled.1);
}

#[test]
fn evaluates_polynomial_surface_outer_hole_trim() {
    let targets =
        ExactRayCrossingTargetSet::new(vec![surface_target("poly", polynomial_chart())]).unwrap();
    let outer = proved(
        cross_ray3(
            z_ray(point(3, 3, 0)),
            targets,
            PositiveRayParameterRange::new(),
            policy(1, 4),
        )
        .unwrap(),
    );
    assert_eq!(outer.1, CrossingParity::Odd);
    assert_eq!(outer.2[0].location(), RayCrossingLocation::Interior);

    let hole_targets =
        ExactRayCrossingTargetSet::new(vec![surface_target("poly", polynomial_chart())]).unwrap();
    let hole = proved(
        cross_ray3(
            z_ray(point_r(ratio(3, 2), ratio(3, 2), algebraic(0))),
            hole_targets,
            PositiveRayParameterRange::new(),
            policy(1, 4),
        )
        .unwrap(),
    );
    assert_eq!(hole.1, CrossingParity::Even);
    assert!(hole.2.is_empty());

    let exterior_targets =
        ExactRayCrossingTargetSet::new(vec![surface_target("poly", polynomial_chart())]).unwrap();
    let exterior = proved(
        cross_ray3(
            z_ray(point(5, 5, 0)),
            exterior_targets,
            PositiveRayParameterRange::new(),
            policy(1, 4),
        )
        .unwrap(),
    );
    assert_eq!(exterior.1, CrossingParity::Even);

    let boundary_targets =
        ExactRayCrossingTargetSet::new(vec![surface_target("poly", polynomial_chart())]).unwrap();
    let boundary = undetermined(
        cross_ray3(
            z_ray(point(4, 2, 0)),
            boundary_targets,
            PositiveRayParameterRange::new(),
            policy(1, 4),
        )
        .unwrap(),
    );
    assert_eq!(boundary.0, RayCrossingUndeterminedReason::TrimBoundary);
}

#[test]
fn evaluates_rational_surface_outer_hole_trim() {
    let targets =
        ExactRayCrossingTargetSet::new(vec![surface_target("rat", rational_chart())]).unwrap();
    let outer = proved(
        cross_ray3(
            z_ray(point(6, 6, 0)),
            targets,
            PositiveRayParameterRange::new(),
            policy(1, 4),
        )
        .unwrap(),
    );
    assert_eq!(outer.1, CrossingParity::Odd);
}

#[test]
fn rejects_self_intersecting_pcurve() {
    let crossing = SurfaceTrimLoop2::try_new(
        TrimLoopRole::Outer,
        vec![
            parameter(0, 0),
            parameter(3, 3),
            parameter(0, 3),
            parameter(3, 0),
        ],
        &domain(),
    );
    assert_eq!(crossing, Err(GeometryFailure::SelfIntersectingTrimLoop));
}

#[test]
fn rejects_pcurve_outside_surface_domain() {
    let outside = SurfaceTrimLoop2::try_new(
        TrimLoopRole::Outer,
        vec![parameter(0, 0), parameter(8, 0), parameter(0, 3)],
        &domain(),
    );
    assert_eq!(outside, Err(GeometryFailure::PCurveOutsideDomain));
}

#[test]
fn rejects_hole_crossing_outer_loop() {
    let outer = loop2(TrimLoopRole::Outer, &[(0, 0), (4, 0), (4, 4), (0, 4)]);
    let hole = loop2(TrimLoopRole::Hole, &[(3, 1), (5, 1), (5, 2), (3, 2)]);
    let result = TrimmedSurfaceDeclaration::try_new(
        polynomial_chart(),
        domain(),
        outer,
        vec![hole],
        Vec::new(),
    );
    assert_eq!(result.err(), Some(GeometryFailure::HoleIntersectsOuterLoop));
}

#[test]
fn rejects_invalid_trim() {
    assert_eq!(
        SurfaceTrimLoop2::try_new(
            TrimLoopRole::Outer,
            vec![parameter(0, 0), parameter(1, 0)],
            &domain()
        ),
        Err(GeometryFailure::SelfIntersectingTrimLoop)
    );
}

#[test]
fn evaluates_paired_seam_and_reports_unresolved_seam() {
    let chart = polynomial_chart();
    let identity = neco_geometry::SurfaceChartIdentity::new(
        ChartDomainId::new(String::from("poly-D")).unwrap(),
        1,
    );
    let paired = PairedSurfaceSeam::new(
        SeamEnd::new(identity, parameter(1, 2), SurfaceOrientation::Preserve),
        SeamEnd::new(
            neco_geometry::SurfaceChartIdentity::new(
                ChartDomainId::new(String::from("poly-D")).unwrap(),
                1,
            ),
            parameter(3, 2),
            SurfaceOrientation::Reverse,
        ),
    );
    let declaration =
        trimmed_declaration(chart, &[(0, 0), (4, 0), (4, 4), (0, 4)], &[], vec![paired]);
    let targets = ExactRayCrossingTargetSet::new(vec![ExactRayCrossingTarget::TrimmedSurface(
        TrimmedParametricSurfaceTarget::new(
            RayCrossingTargetId::new("paired").unwrap(),
            SurfaceCrossingCandidate::new(declaration),
        ),
    )])
    .unwrap();
    let paired_hit = proved(
        cross_ray3(
            z_ray(point(1, 2, 0)),
            targets,
            PositiveRayParameterRange::new(),
            policy(1, 4),
        )
        .unwrap(),
    );
    assert_eq!(paired_hit.1, CrossingParity::Odd);

    let foreign = PairedSurfaceSeam::new(
        SeamEnd::new(
            neco_geometry::SurfaceChartIdentity::new(
                ChartDomainId::new(String::from("other")).unwrap(),
                9,
            ),
            parameter(1, 2),
            SurfaceOrientation::Preserve,
        ),
        SeamEnd::new(
            neco_geometry::SurfaceChartIdentity::new(
                ChartDomainId::new(String::from("else")).unwrap(),
                8,
            ),
            parameter(3, 2),
            SurfaceOrientation::Preserve,
        ),
    );
    let unresolved = trimmed_declaration(
        polynomial_chart(),
        &[(0, 0), (4, 0), (4, 4), (0, 4)],
        &[],
        vec![foreign],
    );
    let unresolved_targets =
        ExactRayCrossingTargetSet::new(vec![ExactRayCrossingTarget::TrimmedSurface(
            TrimmedParametricSurfaceTarget::new(
                RayCrossingTargetId::new("unresolved").unwrap(),
                SurfaceCrossingCandidate::new(unresolved),
            ),
        )])
        .unwrap();
    let unresolved_hit = undetermined(
        cross_ray3(
            z_ray(point(1, 2, 0)),
            unresolved_targets,
            PositiveRayParameterRange::new(),
            policy(1, 4),
        )
        .unwrap(),
    );
    assert_eq!(
        unresolved_hit.0,
        RayCrossingUndeterminedReason::UnresolvedSeam
    );
}

#[test]
fn preserves_trimmed_surface_crossing_under_permutation() {
    let first = trimmed_declaration(
        polynomial_chart(),
        &[(0, 0), (4, 0), (4, 4), (0, 4)],
        &[&[(1, 1), (2, 1), (2, 2), (1, 2)]],
        Vec::new(),
    );
    let second = trimmed_declaration(
        polynomial_chart(),
        &[(4, 4), (0, 4), (0, 0), (4, 0)],
        &[&[(2, 2), (1, 2), (1, 1), (2, 1)]],
        Vec::new(),
    );
    assert_eq!(
        first.classify(&algebraic(3), &algebraic(3)).unwrap(),
        second.classify(&algebraic(3), &algebraic(3)).unwrap()
    );
    let first_targets =
        ExactRayCrossingTargetSet::new(vec![ExactRayCrossingTarget::TrimmedSurface(
            TrimmedParametricSurfaceTarget::new(
                RayCrossingTargetId::new("surface").unwrap(),
                SurfaceCrossingCandidate::new(first),
            ),
        )])
        .unwrap();
    let second_targets =
        ExactRayCrossingTargetSet::new(vec![ExactRayCrossingTarget::TrimmedSurface(
            TrimmedParametricSurfaceTarget::new(
                RayCrossingTargetId::new("surface").unwrap(),
                SurfaceCrossingCandidate::new(second),
            ),
        )])
        .unwrap();
    let first_result = proved(
        cross_ray3(
            z_ray(point(3, 3, 0)),
            first_targets,
            PositiveRayParameterRange::new(),
            policy(1, 4),
        )
        .unwrap(),
    );
    let second_result = proved(
        cross_ray3(
            z_ray(point(3, 3, 0)),
            second_targets,
            PositiveRayParameterRange::new(),
            policy(1, 4),
        )
        .unwrap(),
    );
    assert_eq!(first_result.1, second_result.1);
    assert_eq!(
        first_result.2[0].parameter(),
        second_result.2[0].parameter()
    );
}

#[test]
fn crosses_implicit_sphere_with_regular_normals() {
    let targets =
        ExactRayCrossingTargetSet::new(vec![implicit_target("sphere", sphere_surface())]).unwrap();
    let result = proved(
        cross_ray3(
            z_ray(point(0, 0, -2)),
            targets,
            PositiveRayParameterRange::new(),
            policy(1, 8),
        )
        .unwrap(),
    );
    assert_eq!(result.0, RayOriginContact::None);
    assert_eq!(result.1, CrossingParity::Even);
    assert_eq!(result.2.len(), 2);
    assert_eq!(result.2[0].parameter(), &algebraic(1));
    assert_eq!(result.2[1].parameter(), &algebraic(3));
    assert_eq!(result.2[0].contribution(), CrossingContribution::Toggle);
    assert_eq!(result.2[1].contribution(), CrossingContribution::Toggle);
    match result.2[0].normal() {
        RayNormal::Regular(direction) => {
            assert_eq!(direction.vector().x, algebraic(0));
            assert_eq!(direction.vector().y, algebraic(0));
            assert_eq!(direction.vector().z, algebraic(-2));
        }
        RayNormal::Undetermined => panic!("sphere entry normal must be regular"),
    }
    match result.2[1].normal() {
        RayNormal::Regular(direction) => {
            assert_eq!(direction.vector().x, algebraic(0));
            assert_eq!(direction.vector().y, algebraic(0));
            assert_eq!(direction.vector().z, algebraic(2));
        }
        RayNormal::Undetermined => panic!("sphere exit normal must be regular"),
    }
}

#[test]
fn classifies_implicit_tangent_multiplicity() {
    let targets =
        ExactRayCrossingTargetSet::new(vec![implicit_target("sphere", sphere_surface())]).unwrap();
    let result = proved(
        cross_ray3(
            ray(
                point(0, 1, -1),
                Vector3 {
                    x: algebraic(0),
                    y: algebraic(0),
                    z: algebraic(1),
                },
            ),
            targets,
            PositiveRayParameterRange::new(),
            policy(1, 8),
        )
        .unwrap(),
    );
    assert_eq!(result.2.len(), 1);
    assert_eq!(result.2[0].multiplicity(), CrossingMultiplicity::Even(2));
    assert_eq!(result.2[0].contribution(), CrossingContribution::NoToggle);
    assert_eq!(result.1, CrossingParity::Even);
}

#[test]
fn preserves_implicit_empty_and_coincident_results() {
    let empty_targets =
        ExactRayCrossingTargetSet::new(vec![implicit_target("sphere", sphere_surface())]).unwrap();
    let empty = proved(
        cross_ray3(
            ray(
                point(3, 0, 0),
                Vector3 {
                    x: algebraic(1),
                    y: algebraic(0),
                    z: algebraic(0),
                },
            ),
            empty_targets,
            PositiveRayParameterRange::new(),
            policy(1, 8),
        )
        .unwrap(),
    );
    assert!(empty.2.is_empty());
    assert_eq!(empty.1, CrossingParity::Even);

    let coincident_targets = ExactRayCrossingTargetSet::new(vec![implicit_target(
        "zero",
        ImplicitSurfaceDeclaration::new(MultivariatePolynomial3::zero()),
    )])
    .unwrap();
    let coincident = undetermined(
        cross_ray3(
            z_ray(point(0, 0, -2)),
            coincident_targets,
            PositiveRayParameterRange::new(),
            policy(1, 8),
        )
        .unwrap(),
    );
    assert_eq!(coincident.0, RayCrossingUndeterminedReason::Coincident);
}

#[test]
fn preserves_implicit_root_indeterminacy() {
    let tight = RootIsolationPolicy::new(
        1,
        RootRefinementMethod::Bisection,
        Enclosure::new(
            neco_bigint::Dyadic::new(BigInt::zero(), 0),
            neco_bigint::Dyadic::new(BigInt::one().unwrap(), 40),
        )
        .unwrap(),
    )
    .unwrap();
    let targets =
        ExactRayCrossingTargetSet::new(vec![implicit_target("sphere", sphere_surface())]).unwrap();
    let result = cross_ray3(
        z_ray(point_r(ratio(1, 2), algebraic(0), algebraic(-2))),
        targets,
        PositiveRayParameterRange::new(),
        RayCrossingPolicy::new(tight, 1, 8).unwrap(),
    )
    .unwrap();
    match result {
        RayCrossingResult::Undetermined { reason, hits, .. } => {
            assert_eq!(reason, RayCrossingUndeterminedReason::InvalidMultiplicity);
            assert!(hits.iter().all(|hit| {
                hit.contribution() != CrossingContribution::Toggle
                    || hit.multiplicity() == CrossingMultiplicity::Undetermined
            }));
        }
        RayCrossingResult::Proved { hits, .. } => {
            panic!("expected isolation indeterminacy, got proved hits {hits:?}");
        }
    }
}

#[test]
fn checked_hits_preserve_enclosures_and_provenance() {
    let ray = CheckedRay3::try_new([0.0, 0.0, -1.0], [0.0, 0.0, 1.0], 0.0).unwrap();
    let (owner, requirement, source) = provenance();
    let targets = CheckedRayCrossingTargetSet::new(vec![CheckedRayCrossingTarget::Triangle(
        CheckedTriangleTarget::new(
            RayCrossingTargetId::new("triangle").unwrap(),
            [[-1.0, -1.0, 0.0], [2.0, -1.0, 0.0], [-1.0, 2.0, 0.0]],
            owner,
            requirement,
            source,
        )
        .unwrap(),
    )])
    .unwrap();
    let result =
        cross_checked_ray3(ray, targets, PositiveRayParameterRange::new(), policy(1, 4)).unwrap();
    let CheckedRayCrossingResult::Proved { hits, parity, .. } = result else {
        panic!("expected proved checked crossing");
    };
    assert_eq!(parity, CrossingParity::Odd);
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].owner(), OwnerId::new(1));
    assert_eq!(hits[0].requirement(), &requirement_for_test());
    assert_eq!(hits[0].source(), NecoImplementationSource::current());
    assert_eq!(hits[0].location(), RayCrossingLocation::Interior);
    let parameter = hits[0].parameter();
    assert!(neco_geometry::evaluate_checked_ray3(
        &CheckedRay3::try_new([0.0, 0.0, -1.0], [0.0, 0.0, 1.0], 0.0).unwrap(),
        1.0,
        &PositiveRayParameterRange::new()
    )
    .is_ok());
    let _ = parameter;

    let (surface_owner, surface_requirement, surface_source) = provenance();
    let surface_targets =
        CheckedRayCrossingTargetSet::new(vec![CheckedRayCrossingTarget::TrimmedSurface(Box::new(
            CheckedTrimmedParametricSurfaceTarget::new(
                RayCrossingTargetId::new("surface").unwrap(),
                SurfaceCrossingCandidate::new(trimmed_declaration(
                    polynomial_chart(),
                    &[(0, 0), (4, 0), (4, 4), (0, 4)],
                    &[],
                    Vec::new(),
                )),
                surface_owner,
                surface_requirement,
                surface_source,
            ),
        ))])
        .unwrap();
    let surface = cross_checked_ray3(
        CheckedRay3::try_new([3.0, 3.0, 0.0], [0.0, 0.0, 1.0], 0.0).unwrap(),
        surface_targets,
        PositiveRayParameterRange::new(),
        policy(1, 4),
    )
    .unwrap();
    let CheckedRayCrossingResult::Proved { hits, .. } = surface else {
        panic!("expected proved checked surface crossing");
    };
    assert_eq!(hits[0].owner(), OwnerId::new(1));
    assert_eq!(hits[0].requirement(), &requirement_for_test());
    assert_eq!(hits[0].source(), NecoImplementationSource::current());
}

#[test]
fn surface_target_connection_is_typed() {
    let chart = polynomial_chart();
    let outer = loop2(TrimLoopRole::Outer, &[(0, 0), (4, 0), (4, 4), (0, 4)]);
    let hole = loop2(TrimLoopRole::Hole, &[(1, 1), (2, 1), (2, 2), (1, 2)]);
    let declaration =
        TrimmedSurfaceDeclaration::try_new(chart, domain(), outer, vec![hole], Vec::new()).unwrap();
    let candidate = SurfaceCrossingCandidate::new(declaration);
    assert_eq!(candidate.identity().domain_id().as_str(), "poly-D");
    assert_eq!(
        candidate.classify(&algebraic(3), &algebraic(3)).unwrap(),
        TrimClassification::Outer
    );
    let witness = evaluate_surface_witness_exact(
        candidate.declaration().chart(),
        candidate.domain(),
        &algebraic(3),
        &algebraic(2),
    )
    .unwrap();
    match witness {
        neco_geometry::SurfaceEvaluationWitness::Exact { point, .. } => {
            assert_eq!(point.x, algebraic(3));
            assert_eq!(point.y, algebraic(2));
            assert_eq!(point.z, algebraic(1));
        }
        _ => panic!("crossing witness producer did not return exact coordinates"),
    }
}
