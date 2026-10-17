use neco_algnum::{AlgnumError, RationalPolynomial, RealAlgebraic};
use neco_bigint::{BigInt, BigUint, Dyadic, RawRational};
use neco_calculus::{CalculusFailure, DerivativeOrder, Enclosure, Polynomial};
use neco_geometry::{
    construct_intersection, construct_region_types, derive_boundary, AlgebraicRegion,
    BoundaryCandidateSeed, BoundaryDerivationRequest, BoundaryIntersectionSelector,
    BoundaryLocalEvidence, BoundaryRestrictionChart, BoundarySideSample, BoundaryStratumDimension,
    CanonicalAtom, ContactOrderKind, CurveChart, ExactShapeEvaluation, GeometryFailure,
    ImplicitSurfaceDeclaration, IntersectionComponentView, IntersectionConstructionRequest,
    IntersectionConstructionRequestBody, IntersectionKind, IntersectionOperand, IntersectionSet,
    MembershipClassification, MultivariatePolynomial3, NormalizedRegionFormula,
    ParametricCurveDeclaration, PolynomialTerm3, RegionConstructionRequest, RegionFunction,
    RegionKind, RegionMembershipRequest, RegionPoint, RegionPolynomial3, RegionPolynomialTerm,
    RootIsolationPolicy, ShapeDeclaration, ShapeEvaluation, ShapeFamily, SignRelation,
    WitnessStatus,
};
use neco_geometry_foundation::{ChartDefinition3, ChartDisplayName, ChartDomainId, Point3};

fn algebraic(value: i32) -> RealAlgebraic {
    RealAlgebraic::from_integer(BigInt::try_from(value).unwrap()).unwrap()
}

fn rational_algebraic(numerator: i32, denominator: i32) -> RealAlgebraic {
    let value = RawRational::new(
        BigInt::try_from(numerator).unwrap(),
        BigUint::try_from(u32::try_from(denominator).unwrap()).unwrap(),
    )
    .reduce()
    .unwrap()
    .into_reduced();
    RealAlgebraic::from_reduced_rational(&value).unwrap()
}

fn dyadic(value: i32) -> Dyadic {
    Dyadic::new(BigInt::try_from(value).unwrap(), 0)
}

fn enclosure() -> Enclosure {
    Enclosure::new(dyadic(-1), dyadic(1)).unwrap()
}

fn polynomial1(coefficients: &[i32]) -> Polynomial {
    Polynomial::from_coefficients(coefficients.iter().map(|value| algebraic(*value)).collect())
        .unwrap()
}

fn rational_polynomial(coefficients: &[i32]) -> RationalPolynomial {
    RationalPolynomial::from_coefficients(
        coefficients
            .iter()
            .map(|value| {
                RawRational::new(BigInt::try_from(*value).unwrap(), BigUint::one().unwrap())
                    .reduce()
                    .unwrap()
                    .into_reduced()
            })
            .collect(),
    )
}

fn term(coefficient: i32, x: u32, y: u32, z: u32) -> RegionPolynomialTerm {
    RegionPolynomialTerm::try_new(algebraic(coefficient), x, y, z).unwrap()
}

fn declaration_term(coefficient: i32, x: u32, y: u32, z: u32) -> PolynomialTerm3 {
    PolynomialTerm3::new(algebraic(coefficient), x, y, z).unwrap()
}

fn nonzero_terms(terms: Vec<(i32, u32, u32, u32)>) -> Vec<RegionPolynomialTerm> {
    terms
        .into_iter()
        .filter(|(coefficient, _, _, _)| *coefficient != 0)
        .map(|(coefficient, x, y, z)| term(coefficient, x, y, z))
        .collect()
}

fn nonzero_declaration_terms(terms: Vec<(i32, u32, u32, u32)>) -> Vec<PolynomialTerm3> {
    terms
        .into_iter()
        .filter(|(coefficient, _, _, _)| *coefficient != 0)
        .map(|(coefficient, x, y, z)| declaration_term(coefficient, x, y, z))
        .collect()
}

fn sphere_coefficients(center: (i32, i32, i32), radius: i32) -> Vec<(i32, u32, u32, u32)> {
    let (cx, cy, cz) = center;
    vec![
        (1, 2, 0, 0),
        (1, 0, 2, 0),
        (1, 0, 0, 2),
        (-2 * cx, 1, 0, 0),
        (-2 * cy, 0, 1, 0),
        (-2 * cz, 0, 0, 1),
        (cx * cx + cy * cy + cz * cz - radius * radius, 0, 0, 0),
    ]
}

fn torus_coefficients(
    major_radius: i32,
    minor_numerator: i32,
    minor_denominator: i32,
) -> Vec<(i32, u32, u32, u32)> {
    assert!(minor_denominator > 0);
    let d2 = minor_denominator * minor_denominator;
    let d4 = d2 * d2;
    let a = major_radius * major_radius * d2 - minor_numerator * minor_numerator;
    vec![
        (a * a, 0, 0, 0),
        (
            -2 * d2 * (major_radius * major_radius * d2 + minor_numerator * minor_numerator),
            2,
            0,
            0,
        ),
        (
            -2 * d2 * (major_radius * major_radius * d2 + minor_numerator * minor_numerator),
            0,
            2,
            0,
        ),
        (2 * d2 * a, 0, 0, 2),
        (d4, 4, 0, 0),
        (d4, 0, 4, 0),
        (d4, 0, 0, 4),
        (2 * d4, 2, 2, 0),
        (2 * d4, 2, 0, 2),
        (2 * d4, 0, 2, 2),
    ]
}

fn region(id: &str, terms: Vec<RegionPolynomialTerm>, kind: RegionKind) -> AlgebraicRegion {
    let relation = match kind {
        RegionKind::RegularClosedVolume => SignRelation::Negative,
        RegionKind::LowerDimensionalSet => SignRelation::Zero,
        RegionKind::GeneralSemiAlgebraicSet => panic!("fixture requires a solid or support"),
    };
    let atom = CanonicalAtom::try_new(
        id,
        RegionFunction::Polynomial(RegionPolynomial3::try_from_terms(terms).unwrap()),
        relation,
    )
    .unwrap();
    construct_region_types(RegionConstructionRequest {
        ambient_dimension: 3,
        canonical_atoms: vec![atom],
        normalized_region_formula: NormalizedRegionFormula::Atom(id.into()),
        region_kind: kind,
    })
    .unwrap()
    .into_region()
}

fn sphere_region(
    id: &str,
    center: (i32, i32, i32),
    radius: i32,
    kind: RegionKind,
) -> AlgebraicRegion {
    region(id, nonzero_terms(sphere_coefficients(center, radius)), kind)
}

fn torus_region(
    id: &str,
    major_radius: i32,
    minor_numerator: i32,
    minor_denominator: i32,
    kind: RegionKind,
) -> AlgebraicRegion {
    region(
        id,
        nonzero_terms(torus_coefficients(
            major_radius,
            minor_numerator,
            minor_denominator,
        )),
        kind,
    )
}

fn point(x: i32, y: i32, z: i32) -> RegionPoint {
    RegionPoint::Spatial(Point3 {
        x: algebraic(x),
        y: algebraic(y),
        z: algebraic(z),
    })
}

fn sphere_membership(
    id: &str,
    center: (i32, i32, i32),
    radius: i32,
    kind: RegionKind,
    candidate: RegionPoint,
) -> MembershipClassification {
    sphere_region(id, center, radius, kind)
        .membership(RegionMembershipRequest {
            region: sphere_region(id, center, radius, kind),
            point: candidate,
        })
        .unwrap()
        .classification
}

fn torus_membership(
    id: &str,
    major_radius: i32,
    minor_numerator: i32,
    minor_denominator: i32,
    kind: RegionKind,
    candidate: RegionPoint,
) -> MembershipClassification {
    torus_region(id, major_radius, minor_numerator, minor_denominator, kind)
        .membership(RegionMembershipRequest {
            region: torus_region(id, major_radius, minor_numerator, minor_denominator, kind),
            point: candidate,
        })
        .unwrap()
        .classification
}

fn implicit_operand(terms: Vec<PolynomialTerm3>) -> IntersectionOperand {
    IntersectionOperand::Declaration(ShapeDeclaration::ImplicitSurface(
        ImplicitSurfaceDeclaration::new(MultivariatePolynomial3::try_from_terms(terms).unwrap()),
    ))
}

fn sphere_operand(center: (i32, i32, i32), radius: i32) -> IntersectionOperand {
    implicit_operand(nonzero_declaration_terms(sphere_coefficients(
        center, radius,
    )))
}

fn torus_operand(
    major_radius: i32,
    minor_numerator: i32,
    minor_denominator: i32,
) -> IntersectionOperand {
    implicit_operand(nonzero_declaration_terms(torus_coefficients(
        major_radius,
        minor_numerator,
        minor_denominator,
    )))
}

fn curve(name: &str, numerator_x: &[i32]) -> IntersectionOperand {
    IntersectionOperand::Declaration(ShapeDeclaration::ParametricCurve(
        ParametricCurveDeclaration::new(
            CurveChart::Chart3(
                ChartDefinition3::new(
                    ChartDomainId::new(name.into()).unwrap(),
                    ChartDisplayName::new(format!("chart {name}")).unwrap(),
                    rational_polynomial(&[1]),
                    rational_polynomial(numerator_x),
                    rational_polynomial(&[1]),
                    rational_polynomial(&[1]),
                    rational_polynomial(&[1]),
                )
                .unwrap(),
            ),
            neco_geometry::ParameterInterval::new(algebraic(0), algebraic(1), true, true).unwrap(),
        ),
    ))
}

fn evaluated_point() -> IntersectionOperand {
    IntersectionOperand::Evaluated(ShapeEvaluation::Exact(
        ExactShapeEvaluation::ParametricSurface {
            coordinate: Point3 {
                x: algebraic(0),
                y: algebraic(0),
                z: algebraic(0),
            },
        },
    ))
}

fn undetermined_operand() -> IntersectionOperand {
    IntersectionOperand::Evaluated(ShapeEvaluation::Undetermined {
        family: ShapeFamily::ImplicitSurface,
    })
}

fn implicit_zero() -> IntersectionOperand {
    implicit_operand(Vec::new())
}

fn policy() -> RootIsolationPolicy {
    RootIsolationPolicy::new(
        8,
        neco_geometry::RootRefinementMethod::Bisection,
        enclosure(),
    )
    .unwrap()
}

fn intersection(
    left: IntersectionOperand,
    right: IntersectionOperand,
) -> Result<IntersectionSet, GeometryFailure> {
    construct_intersection(IntersectionConstructionRequest::new(
        IntersectionConstructionRequestBody::new(left, right, policy()),
    ))
}

fn witness(result: &IntersectionSet) -> &neco_geometry::IntersectionWitness {
    result.witness()
}

fn assert_proved(result: &IntersectionSet) {
    let witness = witness(result);
    assert_eq!(witness.existence().status(), &WitnessStatus::Proved);
    assert_eq!(
        witness.root_isolation().completeness().status(),
        &WitnessStatus::Proved
    );
}

fn assert_root_incomplete(result: &IntersectionSet) {
    assert_eq!(result.kind(), IntersectionKind::Undetermined);
    assert!(matches!(
        result.component_view(),
        IntersectionComponentView::Undetermined(_)
    ));
    assert!(matches!(
        result.witness().root_isolation().completeness().status(),
        WitnessStatus::Undetermined { .. }
    ));
}

fn assert_empty_or_undetermined(result: &IntersectionSet) {
    match result.kind() {
        IntersectionKind::Empty => assert_proved(result),
        IntersectionKind::Undetermined => assert_root_incomplete(result),
        other => panic!("expected Empty or Undetermined, got {other:?}"),
    }
}

fn assert_branch_identity(identity: &neco_geometry::BranchIdentity) {
    assert!(!identity.branch_id().is_empty());
    assert!(!identity.chart_id().is_empty());
    assert!(!identity.support_id().is_empty());
}

fn assert_complete_or_undetermined(result: &IntersectionSet) {
    match result.kind() {
        IntersectionKind::Undetermined => assert_root_incomplete(result),
        IntersectionKind::Point
        | IntersectionKind::FinitePoints
        | IntersectionKind::Curve
        | IntersectionKind::CurveBranches => {
            assert_proved(result);
            if let IntersectionComponentView::Components(entries) = result.component_view() {
                assert!(entries.len() > 0);
                for entry in entries {
                    assert_branch_identity(entry.evidence().branch_identity().unwrap());
                }
            }
        }
        IntersectionKind::Empty | IntersectionKind::Coincident => assert_proved(result),
    }
}

fn assert_undefined_zero_power(error: GeometryFailure) {
    assert_eq!(
        error,
        GeometryFailure::CalculusFailure {
            error: CalculusFailure::Algebra(AlgnumError::UndefinedZeroPower),
        }
    );
}

fn assert_curve_or_incomplete(result: Result<IntersectionSet, GeometryFailure>) {
    match result {
        Ok(result)
            if matches!(
                result.kind(),
                IntersectionKind::Curve
                    | IntersectionKind::CurveBranches
                    | IntersectionKind::Undetermined
            ) =>
        {
            assert_complete_or_undetermined(&result)
        }
        Err(error @ GeometryFailure::CalculusFailure { .. }) => assert_undefined_zero_power(error),
        Ok(other) => panic!("expected curve evidence or incompleteness, got {other:?}"),
        Err(other) => panic!("unexpected implicit intersection failure: {other:?}"),
    }
}

fn assert_contact_or_incomplete(result: Result<IntersectionSet, GeometryFailure>) {
    match result {
        Ok(result)
            if matches!(
                result.kind(),
                IntersectionKind::Point
                    | IntersectionKind::FinitePoints
                    | IntersectionKind::Curve
                    | IntersectionKind::CurveBranches
                    | IntersectionKind::Undetermined
            ) =>
        {
            assert_complete_or_undetermined(&result)
        }
        Err(error @ GeometryFailure::CalculusFailure { .. }) => assert_undefined_zero_power(error),
        Ok(other) => panic!("expected contact evidence or incompleteness, got {other:?}"),
        Err(other) => panic!("unexpected implicit intersection failure: {other:?}"),
    }
}

fn assert_empty_or_incomplete(result: Result<IntersectionSet, GeometryFailure>) {
    match result {
        Ok(result) => assert_empty_or_undetermined(&result),
        Err(error @ GeometryFailure::CalculusFailure { .. }) => assert_undefined_zero_power(error),
        Err(other) => panic!("unexpected implicit intersection failure: {other:?}"),
    }
}

fn assert_tangent_or_incomplete(result: Result<IntersectionSet, GeometryFailure>) {
    match result {
        Ok(result)
            if matches!(
                result.kind(),
                IntersectionKind::Point | IntersectionKind::FinitePoints
            ) =>
        {
            assert_complete_or_undetermined(&result);
            if let IntersectionComponentView::Components(entries) = result.component_view() {
                for entry in entries {
                    assert_eq!(
                        entry.evidence().contact_order().unwrap().kind(),
                        ContactOrderKind::Tangent
                    );
                    assert_eq!(entry.evidence().contact_order().unwrap().order(), 2);
                }
            }
        }
        Ok(result) if result.kind() == IntersectionKind::Undetermined => {
            assert_root_incomplete(&result)
        }
        Err(error @ GeometryFailure::CalculusFailure { .. }) => assert_undefined_zero_power(error),
        Ok(other) => panic!("expected tangent point evidence or incompleteness, got {other:?}"),
        Err(other) => panic!("unexpected implicit intersection failure: {other:?}"),
    }
}

fn chart(x: &[i32], y: &[i32], z: &[i32]) -> BoundaryRestrictionChart {
    BoundaryRestrictionChart::try_new(
        "chart",
        Enclosure::new(dyadic(-1), dyadic(2)).unwrap(),
        polynomial1(x),
        polynomial1(y),
        polynomial1(z),
    )
    .unwrap()
}

fn offset_chart(offset_numerator: i32, offset_denominator: i32) -> BoundaryRestrictionChart {
    BoundaryRestrictionChart::try_new(
        "offset-chart",
        Enclosure::new(dyadic(-1), dyadic(2)).unwrap(),
        Polynomial::from_coefficients(vec![
            rational_algebraic(offset_numerator, offset_denominator),
            algebraic(1),
        ])
        .unwrap(),
        polynomial1(&[0]),
        polynomial1(&[0]),
    )
    .unwrap()
}

fn seed_with_chart(
    atom_id: &str,
    chart: BoundaryRestrictionChart,
    intersection: Option<BoundaryIntersectionSelector>,
) -> BoundaryCandidateSeed {
    BoundaryCandidateSeed::try_new(
        atom_id,
        chart,
        vec![dyadic(1)],
        vec![
            BoundarySideSample::new(dyadic(0)),
            BoundarySideSample::new(dyadic(2)),
        ],
        intersection,
    )
    .unwrap()
}

fn seed(
    atom_id: &str,
    intersection: Option<BoundaryIntersectionSelector>,
) -> BoundaryCandidateSeed {
    seed_with_chart(atom_id, chart(&[0, 1], &[0], &[0]), intersection)
}

fn boundary(
    region: AlgebraicRegion,
    intersections: Vec<IntersectionSet>,
    atom_id: &str,
    selector: Option<BoundaryIntersectionSelector>,
) -> neco_geometry::BoundaryTypes {
    derive_boundary(BoundaryDerivationRequest {
        region,
        intersections,
        candidate_loci: vec![seed(atom_id, selector)],
        max_order: DerivativeOrder::new(3),
    })
    .unwrap()
}

fn boundary_with_chart(
    region: AlgebraicRegion,
    atom_id: &str,
    chart: BoundaryRestrictionChart,
) -> neco_geometry::BoundaryTypes {
    derive_boundary(BoundaryDerivationRequest {
        region,
        intersections: Vec::new(),
        candidate_loci: vec![seed_with_chart(atom_id, chart, None)],
        max_order: DerivativeOrder::new(3),
    })
    .unwrap()
}

fn assert_face(boundary: &neco_geometry::BoundaryTypes) {
    assert_eq!(boundary.algebraic_boundary().candidate_loci().len(), 1);
    assert_eq!(boundary.algebraic_boundary().retained_loci().len(), 1);
    assert_eq!(boundary.stratification().strata().len(), 1);
    assert!(matches!(
        boundary.algebraic_boundary().candidate_loci()[0].local_evidence(),
        BoundaryLocalEvidence::Proved { .. }
    ));
    assert!(matches!(
        boundary.stratification().strata()[0].dimension(),
        BoundaryStratumDimension::Dimension(2)
    ));
}

fn point_evidence() -> IntersectionSet {
    intersection(evaluated_point(), curve("point", &[-1, 3])).unwrap()
}
fn curve_evidence() -> IntersectionSet {
    intersection(curve("curve", &[-1, 3]), curve("zero", &[0])).unwrap()
}
fn undetermined_evidence() -> IntersectionSet {
    intersection(undetermined_operand(), implicit_zero()).unwrap()
}

#[test]
fn sphere_regions_construct_with_exact_membership_and_kind() {
    let solid = sphere_region("sphere", (0, 0, 0), 1, RegionKind::RegularClosedVolume);
    let support = sphere_region(
        "sphere-support",
        (2, 0, 0),
        1,
        RegionKind::LowerDimensionalSet,
    );
    assert_eq!(solid.ambient_dimension(), 3);
    assert_eq!(solid.canonical_atoms().len(), 1);
    assert_eq!(solid.region_kind(), RegionKind::RegularClosedVolume);
    assert_eq!(support.region_kind(), RegionKind::LowerDimensionalSet);
    assert_eq!(
        solid.canonical_atoms()[0].relation(),
        SignRelation::Negative
    );
    assert_eq!(support.canonical_atoms()[0].relation(), SignRelation::Zero);
    assert_eq!(
        sphere_membership(
            "sphere",
            (0, 0, 0),
            1,
            RegionKind::RegularClosedVolume,
            point(0, 0, 0)
        ),
        MembershipClassification::Inside
    );
    assert_eq!(
        sphere_membership(
            "sphere",
            (0, 0, 0),
            1,
            RegionKind::RegularClosedVolume,
            point(1, 0, 0)
        ),
        MembershipClassification::Boundary
    );
    assert_eq!(
        sphere_membership(
            "sphere",
            (0, 0, 0),
            1,
            RegionKind::RegularClosedVolume,
            point(2, 0, 0)
        ),
        MembershipClassification::Outside
    );
    assert_eq!(
        solid,
        sphere_region("sphere", (0, 0, 0), 1, RegionKind::RegularClosedVolume)
    );
    assert_eq!(
        construct_region_types(RegionConstructionRequest {
            ambient_dimension: 4,
            canonical_atoms: Vec::new(),
            normalized_region_formula: NormalizedRegionFormula::True,
            region_kind: RegionKind::GeneralSemiAlgebraicSet
        }),
        Err(GeometryFailure::RegionAmbientDimensionMismatch)
    );
}

#[test]
fn sphere_concentric_different_observes_empty_or_undetermined() {
    assert_empty_or_incomplete(intersection(
        sphere_operand((0, 0, 0), 2),
        sphere_operand((0, 0, 0), 1),
    ));
    assert_face(&boundary_with_chart(
        sphere_region("concentric", (0, 0, 0), 2, RegionKind::RegularClosedVolume),
        "concentric",
        chart(&[0, 2], &[0], &[0]),
    ));
}

#[test]
fn sphere_external_tangent_preserves_order_two_or_undetermined() {
    assert_tangent_or_incomplete(intersection(
        sphere_operand((0, 0, 0), 1),
        sphere_operand((2, 0, 0), 1),
    ));
    let evidence = point_evidence();
    let boundary = boundary(
        sphere_region("external", (0, 0, 0), 1, RegionKind::RegularClosedVolume),
        vec![evidence],
        "external",
        Some(BoundaryIntersectionSelector::try_new(0, None).unwrap()),
    );
    assert!(matches!(
        boundary.stratification().strata()[0].dimension(),
        BoundaryStratumDimension::Dimension(0)
    ));
}

#[test]
fn sphere_internal_tangent_preserves_order_two_or_undetermined() {
    assert_tangent_or_incomplete(intersection(
        sphere_operand((0, 0, 0), 2),
        sphere_operand((1, 0, 0), 1),
    ));
}

#[test]
fn sphere_separated_observes_empty_or_undetermined_without_false_nonempty() {
    assert_empty_or_incomplete(intersection(
        sphere_operand((0, 0, 0), 1),
        sphere_operand((100, 0, 0), 1),
    ));
    assert_face(&boundary(
        sphere_region("separated", (0, 0, 0), 1, RegionKind::RegularClosedVolume),
        Vec::new(),
        "separated",
        None,
    ));
}

#[test]
fn sphere_circle_slice_observes_curve_or_undetermined_and_curve_stratum() {
    assert_curve_or_incomplete(intersection(
        sphere_operand((0, 0, 0), 1),
        curve("slice", &[-1, 3]),
    ));
    let evidence = curve_evidence();
    let boundary = boundary(
        sphere_region("slice", (0, 0, 0), 1, RegionKind::RegularClosedVolume),
        vec![evidence],
        "slice",
        Some(BoundaryIntersectionSelector::try_new(0, None).unwrap()),
    );
    assert!(matches!(
        boundary.stratification().strata()[0].dimension(),
        BoundaryStratumDimension::Dimension(1)
    ));
}

#[test]
fn sphere_torus_graze_preserves_contact_or_undetermined() {
    assert_contact_or_incomplete(intersection(
        sphere_operand((2, 0, 0), 1),
        torus_operand(2, 1, 1),
    ));
    let point_boundary = boundary(
        sphere_region("graze", (0, 0, 0), 1, RegionKind::RegularClosedVolume),
        vec![point_evidence()],
        "graze",
        Some(BoundaryIntersectionSelector::try_new(0, None).unwrap()),
    );
    assert!(matches!(
        point_boundary.stratification().strata()[0].dimension(),
        BoundaryStratumDimension::Dimension(0)
    ));
    let curve_boundary = boundary(
        sphere_region("graze-curve", (0, 0, 0), 1, RegionKind::RegularClosedVolume),
        vec![curve_evidence()],
        "graze-curve",
        Some(BoundaryIntersectionSelector::try_new(0, None).unwrap()),
    );
    assert!(matches!(
        curve_boundary.stratification().strata()[0].dimension(),
        BoundaryStratumDimension::Dimension(1)
    ));
    let unresolved = boundary(
        sphere_region(
            "graze-unresolved",
            (0, 0, 0),
            1,
            RegionKind::RegularClosedVolume,
        ),
        vec![undetermined_evidence()],
        "graze-unresolved",
        Some(BoundaryIntersectionSelector::try_new(0, None).unwrap()),
    );
    assert!(matches!(
        unresolved.stratification().strata()[0].dimension(),
        BoundaryStratumDimension::UndeterminedDimension { .. }
    ));
}

#[test]
fn regular_torus_constructs_exact_boundary_face() {
    let torus = torus_region("regular", 2, 1, 1, RegionKind::RegularClosedVolume);
    assert_eq!(torus.canonical_atoms().len(), 1);
    assert_eq!(
        torus_membership(
            "regular",
            2,
            1,
            1,
            RegionKind::RegularClosedVolume,
            point(1, 0, 0)
        ),
        MembershipClassification::Boundary
    );
    assert_eq!(
        torus_membership(
            "regular",
            2,
            1,
            1,
            RegionKind::RegularClosedVolume,
            point(0, 0, 0)
        ),
        MembershipClassification::Outside
    );
    assert_eq!(
        torus_membership(
            "regular",
            2,
            1,
            1,
            RegionKind::RegularClosedVolume,
            point(2, 0, 0)
        ),
        MembershipClassification::Inside
    );
    assert_face(&boundary(torus, Vec::new(), "regular", None));
}

#[test]
fn horn_torus_preserves_self_contact_uncertainty() {
    assert_contact_or_incomplete(intersection(torus_operand(1, 1, 1), torus_operand(1, 1, 1)));
    let boundary = boundary(
        torus_region("horn", 1, 1, 1, RegionKind::RegularClosedVolume),
        vec![undetermined_evidence()],
        "horn",
        Some(BoundaryIntersectionSelector::try_new(0, None).unwrap()),
    );
    assert_eq!(boundary.algebraic_boundary().candidate_loci().len(), 1);
}

#[test]
fn spindle_torus_requires_proved_branch_identity_or_undetermined() {
    assert_curve_or_incomplete(intersection(torus_operand(1, 2, 1), torus_operand(1, 2, 1)));
}

#[test]
fn thin_tube_constructs_with_exact_membership() {
    let first = torus_region("thin-tube", 1, 1, 100, RegionKind::RegularClosedVolume);
    assert_eq!(
        first,
        torus_region("thin-tube", 1, 1, 100, RegionKind::RegularClosedVolume)
    );
    assert_eq!(
        torus_membership(
            "thin-tube",
            1,
            1,
            100,
            RegionKind::RegularClosedVolume,
            point(1, 0, 0)
        ),
        MembershipClassification::Inside
    );
    assert_eq!(
        torus_membership(
            "thin-tube",
            1,
            1,
            100,
            RegionKind::RegularClosedVolume,
            point(0, 0, 0)
        ),
        MembershipClassification::Outside
    );
    let boundary = boundary_with_chart(first, "thin-tube", offset_chart(-1, 100));
    assert_eq!(boundary.algebraic_boundary().candidate_loci().len(), 1);
}

#[test]
fn thin_ring_constructs_with_exact_membership() {
    let first = torus_region("thin-ring", 10, 1, 2, RegionKind::RegularClosedVolume);
    assert_eq!(
        first,
        torus_region("thin-ring", 10, 1, 2, RegionKind::RegularClosedVolume)
    );
    assert_eq!(
        torus_membership(
            "thin-ring",
            10,
            1,
            2,
            RegionKind::RegularClosedVolume,
            point(10, 0, 0)
        ),
        MembershipClassification::Inside
    );
    assert_eq!(
        torus_membership(
            "thin-ring",
            10,
            1,
            2,
            RegionKind::RegularClosedVolume,
            point(0, 0, 0)
        ),
        MembershipClassification::Outside
    );
    let boundary = boundary_with_chart(first, "thin-ring", offset_chart(17, 2));
    assert_eq!(boundary.algebraic_boundary().candidate_loci().len(), 1);
}

#[test]
fn torus_self_support_is_coincident_only_if_proved() {
    match intersection(torus_operand(2, 1, 1), torus_operand(2, 1, 1)) {
        Ok(result) if result.kind() == IntersectionKind::Coincident => assert_proved(&result),
        Ok(result) if result.kind() == IntersectionKind::Undetermined => {
            assert_root_incomplete(&result)
        }
        Err(error @ GeometryFailure::CalculusFailure { .. }) => assert_undefined_zero_power(error),
        Ok(other) => panic!("expected Coincident or Undetermined, got {other:?}"),
        Err(other) => panic!("unexpected implicit intersection failure: {other:?}"),
    }
    let boundary = boundary(
        torus_region("self-support", 2, 1, 1, RegionKind::RegularClosedVolume),
        vec![undetermined_evidence()],
        "self-support",
        Some(BoundaryIntersectionSelector::try_new(0, None).unwrap()),
    );
    assert!(matches!(
        boundary.stratification().strata()[0].dimension(),
        BoundaryStratumDimension::UndeterminedDimension { .. }
    ));
}
