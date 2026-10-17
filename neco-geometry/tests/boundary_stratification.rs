use neco_algnum::RealAlgebraic;
use neco_bigint::{BigInt, BigUint, Dyadic, RawRational};
use neco_calculus::{DerivativeOrder, Enclosure, Polynomial, RegionSign};
use neco_geometry::{
    construct_intersection, construct_region_types, derive_boundary, AlgebraicRegion,
    BoundaryCandidateSeed, BoundaryDerivationRequest, BoundaryIntersectionSelector,
    BoundaryLocalEvidence, BoundaryResource, BoundaryRestrictionChart, BoundarySideSample,
    BoundaryStratumDimension, CanonicalAtom, CurveChart, ExactShapeEvaluation, GeometryFailure,
    ImplicitSurfaceDeclaration, IntersectionConstructionRequest,
    IntersectionConstructionRequestBody, IntersectionEvidenceKind, IntersectionOperand,
    MultivariatePolynomial3, NormalizedRegionFormula, ParametricCurveDeclaration,
    RationalFunction3, RationalFunctionDomain3, RegionConstructionRequest, RegionFunction,
    RegionKind, RegionPolynomial3, RegionPolynomialTerm, ShapeDeclaration, ShapeEvaluation,
    ShapeFamily, SignRelation, MAX_BOUNDARY_CANDIDATE_PARAMETERS, MAX_BOUNDARY_SIDE_SAMPLES,
};
use neco_geometry_foundation::{ChartDefinition3, ChartDisplayName, ChartDomainId};

fn algebraic(value: i32) -> RealAlgebraic {
    RealAlgebraic::from_integer(BigInt::try_from(value).unwrap()).unwrap()
}

fn dyadic(value: i32) -> Dyadic {
    Dyadic::new(BigInt::try_from(value).unwrap(), 0)
}

fn polynomial1(coefficients: &[i32]) -> Polynomial {
    Polynomial::from_coefficients(coefficients.iter().map(|value| algebraic(*value)).collect())
        .unwrap()
}

fn term(coefficient: i32, x: u32, y: u32, z: u32) -> RegionPolynomialTerm {
    RegionPolynomialTerm::try_new(algebraic(coefficient), x, y, z).unwrap()
}

fn polynomial3(terms: Vec<RegionPolynomialTerm>) -> RegionPolynomial3 {
    RegionPolynomial3::try_from_terms(terms).unwrap()
}

fn atom(id: &str, terms: Vec<RegionPolynomialTerm>, relation: SignRelation) -> CanonicalAtom {
    CanonicalAtom::try_new(id, RegionFunction::Polynomial(polynomial3(terms)), relation).unwrap()
}

fn region(atoms: Vec<CanonicalAtom>, formula: NormalizedRegionFormula) -> AlgebraicRegion {
    construct_region_types(RegionConstructionRequest {
        ambient_dimension: 3,
        canonical_atoms: atoms,
        normalized_region_formula: formula,
        region_kind: RegionKind::GeneralSemiAlgebraicSet,
    })
    .unwrap()
    .into_region()
}

fn enclosure() -> Enclosure {
    Enclosure::new(dyadic(-1), dyadic(1)).unwrap()
}

fn chart(x: &[i32], y: &[i32], z: &[i32]) -> BoundaryRestrictionChart {
    BoundaryRestrictionChart::try_new(
        "chart",
        enclosure(),
        polynomial1(x),
        polynomial1(y),
        polynomial1(z),
    )
    .unwrap()
}

fn seed(
    atom_id: &str,
    chart: BoundaryRestrictionChart,
    candidates: Vec<i32>,
    sides: Vec<i32>,
    intersection: Option<BoundaryIntersectionSelector>,
) -> BoundaryCandidateSeed {
    BoundaryCandidateSeed::try_new(
        atom_id,
        chart,
        candidates.into_iter().map(dyadic).collect(),
        sides
            .into_iter()
            .map(|parameter| BoundarySideSample::new(dyadic(parameter)))
            .collect(),
        intersection,
    )
    .unwrap()
}

fn request(
    region: AlgebraicRegion,
    candidate_loci: Vec<BoundaryCandidateSeed>,
) -> BoundaryDerivationRequest {
    BoundaryDerivationRequest {
        region,
        intersections: Vec::new(),
        candidate_loci,
        max_order: DerivativeOrder::new(3),
    }
}

fn rational_polynomial(coefficients: &[i32]) -> neco_algnum::RationalPolynomial {
    neco_algnum::RationalPolynomial::from_coefficients(
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

fn intersection_policy() -> neco_geometry::RootIsolationPolicy {
    neco_geometry::RootIsolationPolicy::new(
        8,
        neco_geometry::RootRefinementMethod::Bisection,
        enclosure(),
    )
    .unwrap()
}

fn intersection_request(
    left: IntersectionOperand,
    right: IntersectionOperand,
) -> IntersectionConstructionRequest {
    IntersectionConstructionRequest::new(IntersectionConstructionRequestBody::new(
        left,
        right,
        intersection_policy(),
    ))
}

fn evaluated_point() -> IntersectionOperand {
    IntersectionOperand::Evaluated(ShapeEvaluation::Exact(
        ExactShapeEvaluation::ParametricSurface {
            coordinate: neco_geometry::Point3 {
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
    IntersectionOperand::Declaration(ShapeDeclaration::ImplicitSurface(
        ImplicitSurfaceDeclaration::new(MultivariatePolynomial3::zero()),
    ))
}

fn curve(name: &str, numerator_x: &[i32]) -> IntersectionOperand {
    IntersectionOperand::Declaration(ShapeDeclaration::ParametricCurve(
        ParametricCurveDeclaration::new(
            CurveChart::Chart3(
                ChartDefinition3::new(
                    ChartDomainId::new(String::from(name)).unwrap(),
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

fn transition_region() -> AlgebraicRegion {
    region(
        vec![atom("x", vec![term(1, 1, 0, 0)], SignRelation::Positive)],
        NormalizedRegionFormula::Atom("x".into()),
    )
}

#[test]
fn atom_zero_without_formula_transition_is_not_retained() {
    let region = region(
        vec![
            atom("x", vec![term(1, 1, 0, 0)], SignRelation::Positive),
            atom("always", vec![term(1, 0, 0, 0)], SignRelation::Positive),
        ],
        NormalizedRegionFormula::Atom("always".into()),
    );
    let result = derive_boundary(request(
        region,
        vec![seed(
            "x",
            chart(&[0, 1], &[0], &[0]),
            vec![0],
            vec![-1, 1],
            None,
        )],
    ))
    .unwrap();

    assert!(matches!(
        result.algebraic_boundary().candidate_loci()[0].local_evidence(),
        BoundaryLocalEvidence::Undetermined { .. }
    ));
}

#[test]
fn atom_zero_with_inside_outside_transition_is_retained() {
    let result = derive_boundary(request(
        transition_region(),
        vec![seed(
            "x",
            chart(&[0, 1], &[0], &[0]),
            vec![0],
            vec![-1, 1],
            None,
        )],
    ))
    .unwrap();
    let retained = &result.algebraic_boundary().retained_loci()[0];

    assert_eq!(retained.locus().atom_id(), "x");
    assert!(matches!(
        retained.local_evidence(),
        BoundaryLocalEvidence::Proved { .. }
    ));
}

#[test]
fn boundary_restricts_multivariate_polynomial_through_explicit_chart() {
    let result = derive_boundary(request(
        region(
            vec![atom(
                "mixed",
                vec![term(1, 1, 0, 0), term(1, 0, 1, 1)],
                SignRelation::Positive,
            )],
            NormalizedRegionFormula::Atom("mixed".into()),
        ),
        vec![seed(
            "mixed",
            chart(&[0, 1], &[2], &[0, 1]),
            vec![0],
            vec![-1, 1],
            None,
        )],
    ))
    .unwrap();

    match result.algebraic_boundary().retained_loci()[0].local_evidence() {
        BoundaryLocalEvidence::Proved {
            sign_judgment,
            witness,
            ..
        } => {
            assert_eq!(sign_judgment.sign(), RegionSign::SignVaries);
            assert_eq!(witness.candidate_count(), 1);
        }
        other => panic!("expected proved restriction evidence, got {other:?}"),
    }
}

#[test]
fn rational_boundary_requires_nonzero_restricted_denominator() {
    let rational = CanonicalAtom::try_new(
        "r",
        RegionFunction::RationalFunction(
            RationalFunction3::try_new(
                polynomial3(vec![term(1, 1, 0, 0)]),
                polynomial3(vec![term(-1, 0, 0, 0), term(4, 1, 0, 0)]),
                RationalFunctionDomain3::try_new(polynomial3(vec![
                    term(-1, 0, 0, 0),
                    term(4, 1, 0, 0),
                ]))
                .unwrap(),
            )
            .unwrap(),
        ),
        SignRelation::Positive,
    )
    .unwrap();
    let result = derive_boundary(request(
        region(vec![rational], NormalizedRegionFormula::Atom("r".into())),
        vec![seed(
            "r",
            chart(&[0, 1], &[0], &[0]),
            vec![0],
            vec![-1, 1],
            None,
        )],
    ))
    .unwrap();

    assert!(matches!(
        result.algebraic_boundary().candidate_loci()[0].local_evidence(),
        BoundaryLocalEvidence::Undetermined {
            reason: neco_geometry::BoundaryUndeterminedReason::RestrictedRationalDenominatorNotProvedNonzero,
            ..
        }
    ));
}

#[test]
fn caller_candidate_parameters_are_not_enumerated_or_replaced() {
    let result = derive_boundary(request(
        transition_region(),
        vec![seed(
            "x",
            chart(&[0, 1], &[0], &[0]),
            vec![0],
            vec![-1, 1],
            None,
        )],
    ))
    .unwrap();
    let candidate = &result.algebraic_boundary().candidate_loci()[0];

    assert_eq!(result.algebraic_boundary().candidate_loci().len(), 1);
    match candidate.local_evidence() {
        BoundaryLocalEvidence::Proved { witness, .. } => assert_eq!(witness.candidate_count(), 1),
        other => panic!("expected caller-supplied candidate evidence, got {other:?}"),
    }
}

#[test]
fn intersection_snapshots_preserve_point_curve_coincident_and_undetermined_distinctions() {
    let intersections = vec![
        construct_intersection(intersection_request(
            evaluated_point(),
            curve("point", &[-1, 3]),
        ))
        .unwrap(),
        construct_intersection(intersection_request(
            curve("curve", &[-1, 3]),
            curve("zero", &[0]),
        ))
        .unwrap(),
        construct_intersection(intersection_request(
            curve("same", &[0]),
            curve("same", &[0]),
        ))
        .unwrap(),
        construct_intersection(intersection_request(
            undetermined_operand(),
            implicit_zero(),
        ))
        .unwrap(),
    ];
    let mut boundary_request = request(
        transition_region(),
        (0..4)
            .map(|index| {
                seed(
                    "x",
                    chart(&[0, 1], &[0], &[0]),
                    vec![0],
                    vec![-1, 1],
                    Some(BoundaryIntersectionSelector::try_new(index, None).unwrap()),
                )
            })
            .collect(),
    );
    boundary_request.intersections = intersections;
    let result = derive_boundary(boundary_request).unwrap();
    let kinds: Vec<_> = result
        .algebraic_boundary()
        .candidate_loci()
        .iter()
        .map(|candidate| candidate.intersection_evidence().unwrap().kind())
        .collect();

    assert_eq!(
        kinds,
        vec![
            IntersectionEvidenceKind::Point,
            IntersectionEvidenceKind::Curve,
            IntersectionEvidenceKind::Coincident,
            IntersectionEvidenceKind::Undetermined,
        ]
    );
}

#[test]
fn strata_preserve_point_curve_and_undetermined_dimensions() {
    let intersections = vec![
        construct_intersection(intersection_request(
            evaluated_point(),
            curve("point", &[-1, 3]),
        ))
        .unwrap(),
        construct_intersection(intersection_request(
            curve("curve", &[-1, 3]),
            curve("zero", &[0]),
        ))
        .unwrap(),
        construct_intersection(intersection_request(
            undetermined_operand(),
            implicit_zero(),
        ))
        .unwrap(),
    ];
    let mut boundary_request = request(
        transition_region(),
        (0..3)
            .map(|index| {
                seed(
                    "x",
                    chart(&[0, 1], &[0], &[0]),
                    vec![0],
                    vec![-1, 1],
                    Some(BoundaryIntersectionSelector::try_new(index, None).unwrap()),
                )
            })
            .collect(),
    );
    boundary_request.intersections = intersections;
    let result = derive_boundary(boundary_request).unwrap();
    let dimensions: Vec<_> = result
        .stratification()
        .strata()
        .iter()
        .map(|stratum| stratum.dimension())
        .collect();

    assert!(matches!(
        dimensions[0],
        BoundaryStratumDimension::Dimension(0)
    ));
    assert!(matches!(
        dimensions[1],
        BoundaryStratumDimension::Dimension(1)
    ));
    assert!(matches!(
        dimensions[2],
        BoundaryStratumDimension::UndeterminedDimension { .. }
    ));
}

#[test]
fn boundary_propagates_public_failure_payloads() {
    assert_eq!(
        BoundaryIntersectionSelector::try_new(0, Some(String::new())),
        Err(GeometryFailure::MalformedRegionFormula)
    );

    let excessive_parameters = (0..=MAX_BOUNDARY_CANDIDATE_PARAMETERS)
        .map(|_| dyadic(0))
        .collect();
    assert_eq!(
        BoundaryCandidateSeed::try_new(
            "x",
            chart(&[0, 1], &[0], &[0]),
            excessive_parameters,
            vec![BoundarySideSample::new(dyadic(1))],
            None,
        ),
        Err(GeometryFailure::BoundaryResourceLimit {
            resource: BoundaryResource::CandidateParameters,
            requested: MAX_BOUNDARY_CANDIDATE_PARAMETERS + 1,
            maximum: MAX_BOUNDARY_CANDIDATE_PARAMETERS,
        })
    );

    let excessive_side_samples = (0..=MAX_BOUNDARY_SIDE_SAMPLES)
        .map(|_| BoundarySideSample::new(dyadic(0)))
        .collect();
    assert_eq!(
        BoundaryCandidateSeed::try_new(
            "x",
            chart(&[0, 1], &[0], &[0]),
            vec![dyadic(0)],
            excessive_side_samples,
            None,
        ),
        Err(GeometryFailure::BoundaryResourceLimit {
            resource: BoundaryResource::SideSamples,
            requested: MAX_BOUNDARY_SIDE_SAMPLES + 1,
            maximum: MAX_BOUNDARY_SIDE_SAMPLES,
        })
    );

    let result = derive_boundary(request(
        transition_region(),
        vec![seed(
            "unknown",
            chart(&[0, 1], &[0], &[0]),
            vec![0],
            vec![-1, 1],
            None,
        )],
    ));

    assert_eq!(result, Err(GeometryFailure::MalformedRegionFormula));
}

#[test]
fn boundary_undetermined_evidence_is_successful() {
    let result = derive_boundary(request(
        transition_region(),
        vec![seed(
            "x",
            chart(&[0, 1], &[0], &[0]),
            vec![0],
            vec![1],
            None,
        )],
    ));

    assert!(matches!(
        result.unwrap().algebraic_boundary().candidate_loci()[0].local_evidence(),
        BoundaryLocalEvidence::Undetermined { .. }
    ));
}

#[test]
fn boundary_derivation_is_public_and_repeatable() {
    let first = derive_boundary(request(
        transition_region(),
        vec![seed(
            "x",
            chart(&[0, 1], &[0], &[0]),
            vec![0],
            vec![-1, 1],
            None,
        )],
    ));
    let second = derive_boundary(request(
        transition_region(),
        vec![seed(
            "x",
            chart(&[0, 1], &[0], &[0]),
            vec![0],
            vec![-1, 1],
            None,
        )],
    ));

    assert_eq!(first, second);
}

fn two_atom_boundary_request(reverse: bool) -> BoundaryDerivationRequest {
    let region = region(
        vec![
            atom("x", vec![term(1, 1, 0, 0)], SignRelation::Positive),
            atom("y", vec![term(1, 0, 1, 0)], SignRelation::Positive),
        ],
        NormalizedRegionFormula::Or(vec![
            NormalizedRegionFormula::Atom("x".into()),
            NormalizedRegionFormula::Atom("y".into()),
        ]),
    );
    let x = seed("x", chart(&[0, 1], &[0], &[0]), vec![0], vec![-1, 1], None);
    let y = seed("y", chart(&[0], &[0, 1], &[0]), vec![0], vec![-1, 1], None);
    request(region, if reverse { vec![y, x] } else { vec![x, y] })
}

#[test]
fn candidate_permutation_preserves_boundary_and_strata() {
    let forward = derive_boundary(two_atom_boundary_request(false)).unwrap();
    let reverse = derive_boundary(two_atom_boundary_request(true)).unwrap();

    assert_eq!(forward, reverse);
    assert_eq!(
        forward
            .algebraic_boundary()
            .candidate_loci()
            .iter()
            .map(|candidate| candidate.locus().atom_id())
            .collect::<Vec<_>>(),
        vec!["x", "y"]
    );
}
