use neco_algnum::{RationalPolynomial, RealAlgebraic};
use neco_bigint::Dyadic;
use neco_bigint::{BigInt, BigUint, RawRational};
use neco_calculus::Enclosure;
use neco_geometry::{
    construct_intersection, ChartDefinition3, ChartDisplayName, ChartDomainId,
    ComponentEvidenceRef, CurveChart, ExactShapeEvaluation, ExistenceEvidence, GeometryFailure,
    ImplicitSurfaceDeclaration, IntersectionComponentView, IntersectionConstructionRequest,
    IntersectionConstructionRequestBody, IntersectionKind, IntersectionOperand, IntersectionSet,
    MultivariatePolynomial3, ParameterInterval, ParametricCurveDeclaration, Point3, ResidualValue,
    RootEvidence, RootIsolationPolicy, RootRefinementMethod, ShapeDeclaration, ShapeEvaluation,
    ShapeFamily, UniquenessEvidence, WitnessStatus, WitnessUndeterminedReason,
    MAX_ROOT_PARTITION_CELLS, MAX_ROOT_PARTITION_DEPTH,
};
use std::cmp::Ordering;

fn algebraic(value: i32) -> RealAlgebraic {
    RealAlgebraic::from_integer(BigInt::try_from(value).unwrap()).unwrap()
}

fn rational_algebraic(numerator: i32, denominator: u32) -> RealAlgebraic {
    let value = RawRational::new(
        BigInt::try_from(numerator).unwrap(),
        BigUint::try_from(denominator).unwrap(),
    )
    .reduce()
    .unwrap()
    .into_reduced();
    RealAlgebraic::from_reduced_rational(&value).unwrap()
}

fn exact_residual(value: &ResidualValue) -> &RealAlgebraic {
    match value {
        ResidualValue::Exact { value } => value,
        _ => panic!("expected an exact residual"),
    }
}

fn polynomial(coefficients: &[i32]) -> RationalPolynomial {
    RationalPolynomial::from_coefficients(
        coefficients
            .iter()
            .map(|coefficient| {
                RawRational::new(
                    BigInt::try_from(*coefficient).unwrap(),
                    BigUint::one().unwrap(),
                )
                .reduce()
                .unwrap()
                .into_reduced()
            })
            .collect(),
    )
}

fn policy() -> RootIsolationPolicy {
    RootIsolationPolicy::new(
        8,
        RootRefinementMethod::Bisection,
        Enclosure::new(
            Dyadic::new(BigInt::zero(), 0),
            Dyadic::new(BigInt::one().unwrap(), 0),
        )
        .unwrap(),
    )
    .unwrap()
}

fn curve(name: &str, numerator_x: &[i32]) -> IntersectionOperand {
    IntersectionOperand::Declaration(ShapeDeclaration::ParametricCurve(
        ParametricCurveDeclaration::new(
            CurveChart::Chart3(
                ChartDefinition3::new(
                    ChartDomainId::new(String::from(name)).unwrap(),
                    ChartDisplayName::new(format!("chart {name}")).unwrap(),
                    polynomial(&[1]),
                    polynomial(numerator_x),
                    polynomial(&[1]),
                    polynomial(&[1]),
                    polynomial(&[1]),
                )
                .unwrap(),
            ),
            ParameterInterval::new(algebraic(0), algebraic(1), true, true).unwrap(),
        ),
    ))
}

fn point() -> IntersectionOperand {
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

fn scalar(value: i32) -> IntersectionOperand {
    IntersectionOperand::Evaluated(ShapeEvaluation::Exact(
        ExactShapeEvaluation::ImplicitSurface {
            scalar: algebraic(value),
        },
    ))
}

fn unresolved() -> IntersectionOperand {
    IntersectionOperand::Evaluated(ShapeEvaluation::Undetermined {
        family: ShapeFamily::ImplicitSurface,
    })
}

fn zero_implicit_surface() -> IntersectionOperand {
    IntersectionOperand::Declaration(ShapeDeclaration::ImplicitSurface(
        ImplicitSurfaceDeclaration::new(MultivariatePolynomial3::zero()),
    ))
}

fn intersect(
    left: IntersectionOperand,
    right: IntersectionOperand,
) -> Result<IntersectionSet, GeometryFailure> {
    construct_intersection(IntersectionConstructionRequest::new(
        IntersectionConstructionRequestBody::new(left, right, policy()),
    ))
}

fn component_count(intersection: &IntersectionSet) -> Option<usize> {
    match intersection.component_view() {
        IntersectionComponentView::Components(components) => Some(components.len()),
        IntersectionComponentView::NoComponents => Some(0),
        IntersectionComponentView::Undetermined(_) => None,
    }
}

#[test]
fn classification_determines_the_component_shape() {
    let cases = [
        (
            intersect(scalar(1), scalar(2)).unwrap(),
            IntersectionKind::Empty,
            Some(0),
        ),
        (
            intersect(point(), curve("point", &[-1, 3])).unwrap(),
            IntersectionKind::Point,
            Some(1),
        ),
        (
            intersect(point(), curve("finite", &[2, -9, 9])).unwrap(),
            IntersectionKind::FinitePoints,
            Some(2),
        ),
        (
            intersect(curve("same", &[0]), curve("same", &[0])).unwrap(),
            IntersectionKind::Coincident,
            Some(1),
        ),
        (
            intersect(curve("curve", &[-1, 3]), curve("zero", &[0])).unwrap(),
            IntersectionKind::Curve,
            Some(1),
        ),
        (
            intersect(curve("branches", &[2, -9, 9]), curve("zero-branches", &[0])).unwrap(),
            IntersectionKind::CurveBranches,
            Some(2),
        ),
        (
            intersect(unresolved(), zero_implicit_surface()).unwrap(),
            IntersectionKind::Undetermined,
            None,
        ),
    ];

    for (intersection, expected_kind, expected_components) in cases {
        assert_eq!(intersection.kind(), expected_kind);
        assert_eq!(component_count(&intersection), expected_components);
    }
}

#[test]
fn finite_points_have_canonical_identity_order_and_point_evidence() {
    let intersection = intersect(point(), curve("finite", &[2, -9, 9])).unwrap();
    let IntersectionComponentView::Components(components) = intersection.component_view() else {
        panic!("finite intersection must expose components");
    };
    let components = components.collect::<Vec<_>>();

    assert_eq!(components.len(), 2);
    assert!(components[0].identity() < components[1].identity());
    assert_eq!(components[0].identity().ordinal(), 0);
    assert_eq!(components[1].identity().ordinal(), 1);
    assert!(components
        .iter()
        .all(|component| matches!(component.evidence(), ComponentEvidenceRef::Point(_))));
}

#[test]
fn component_evidence_preserves_geometric_classification() {
    let point_intersection = intersect(point(), curve("point", &[-1, 3])).unwrap();
    let curve_intersection = intersect(curve("curve", &[-1, 3]), curve("zero", &[0])).unwrap();
    let coincident_intersection = intersect(curve("same", &[0]), curve("same", &[0])).unwrap();

    let IntersectionComponentView::Components(mut points) = point_intersection.component_view()
    else {
        panic!("point intersection must expose one point");
    };
    let IntersectionComponentView::Components(mut curves) = curve_intersection.component_view()
    else {
        panic!("curve intersection must expose one curve");
    };
    let IntersectionComponentView::Components(mut coincident) =
        coincident_intersection.component_view()
    else {
        panic!("coincident supports must expose one component");
    };

    let point = points.next().unwrap();
    let curve = curves.next().unwrap();
    let coincident = coincident.next().unwrap();
    assert!(matches!(point.evidence(), ComponentEvidenceRef::Point(_)));
    assert!(matches!(curve.evidence(), ComponentEvidenceRef::Curve(_)));
    assert!(matches!(
        coincident.evidence(),
        ComponentEvidenceRef::Coincident(_)
    ));
    assert!(point.evidence().branch_identity().is_some());
    assert!(curve.evidence().contact_order().is_some());
    assert!(coincident.evidence().branch_identity().is_none());
}

#[test]
fn operand_permutation_preserves_geometry_and_transforms_oriented_residual() {
    let left_right = intersect(scalar(1), scalar(2)).unwrap();
    let right_left = intersect(scalar(2), scalar(1)).unwrap();

    assert_eq!(left_right.kind(), right_left.kind());
    assert_eq!(component_count(&left_right), component_count(&right_left));
    assert_eq!(
        exact_residual(left_right.residual().left()),
        exact_residual(right_left.residual().right())
    );
    assert_eq!(
        exact_residual(left_right.residual().right()),
        exact_residual(right_left.residual().left())
    );
    assert!(exact_residual(left_right.residual().combined())
        .add(exact_residual(right_left.residual().combined()))
        .unwrap()
        .is_zero());

    let left_function = left_right.witness().constraint().residual_function();
    let right_function = right_left.witness().constraint().residual_function();
    assert_eq!(
        left_function.left_expression(),
        right_function.right_expression()
    );
    assert_eq!(
        left_function.right_expression(),
        right_function.left_expression()
    );
}

#[test]
fn repeated_construction_preserves_the_complete_result() {
    let first = intersect(point(), curve("finite", &[2, -9, 9])).unwrap();
    let second = intersect(point(), curve("finite", &[2, -9, 9])).unwrap();

    assert_eq!(first, second);
    assert_eq!(
        first.witness().existence().evidence(),
        ExistenceEvidence::CompleteCandidatePartition
    );
    assert_eq!(
        first.witness().uniqueness().evidence(),
        UniquenessEvidence::PairwiseNonOverlappingBrackets
    );
}

#[test]
fn finite_root_brackets_are_strictly_ordered() {
    let intersection = intersect(point(), curve("finite", &[2, -9, 9])).unwrap();
    let brackets = intersection.witness().root_isolation().brackets();
    let roots = [rational_algebraic(1, 3), rational_algebraic(2, 3)];

    assert_eq!(brackets.len(), 2);
    for pair in brackets.windows(2) {
        assert_eq!(
            pair[0].upper().compare(pair[1].lower()).unwrap(),
            Ordering::Less
        );
    }
    for (bracket, root) in brackets.iter().zip(&roots) {
        assert_ne!(bracket.lower().compare(root).unwrap(), Ordering::Greater);
        assert_ne!(bracket.upper().compare(root).unwrap(), Ordering::Less);
        assert!(matches!(
            bracket.evidence(),
            RootEvidence::SignChange { .. }
        ));
    }
    assert_eq!(
        intersection.witness().existence().status(),
        &WitnessStatus::Proved
    );
    assert_eq!(
        intersection.witness().uniqueness().status(),
        &WitnessStatus::Proved
    );
}

#[test]
fn unresolved_input_remains_undetermined() {
    let intersection = intersect(unresolved(), zero_implicit_surface()).unwrap();

    assert_eq!(intersection.kind(), IntersectionKind::Undetermined);
    assert!(matches!(
        intersection.component_view(),
        IntersectionComponentView::Undetermined(_)
    ));
    assert!(matches!(
        intersection.witness().existence().status(),
        WitnessStatus::Undetermined {
            reason: WitnessUndeterminedReason::PartitionOrCandidatesIncomplete
        }
    ));
}

#[test]
fn invalid_root_isolation_policies_return_typed_failures() {
    let target = Enclosure::new(
        Dyadic::new(BigInt::zero(), 0),
        Dyadic::new(BigInt::one().unwrap(), 0),
    )
    .unwrap();
    assert_eq!(
        RootIsolationPolicy::new(0, RootRefinementMethod::Bisection, target),
        Err(GeometryFailure::RootIsolationPolicyInvalid)
    );

    let target = Enclosure::new(
        Dyadic::new(BigInt::zero(), 0),
        Dyadic::new(BigInt::one().unwrap(), 0),
    )
    .unwrap();
    assert!(RootIsolationPolicy::new(8, RootRefinementMethod::NewtonBisection, target).is_ok());

    let target = Enclosure::new(
        Dyadic::new(BigInt::zero(), 0),
        Dyadic::new(BigInt::one().unwrap(), 0),
    )
    .unwrap();
    assert_eq!(
        RootIsolationPolicy::new(
            MAX_ROOT_PARTITION_DEPTH + 1,
            RootRefinementMethod::Bisection,
            target,
        ),
        Err(GeometryFailure::RootIsolationResourceLimit {
            requested_depth: MAX_ROOT_PARTITION_DEPTH + 1,
            maximum_depth: MAX_ROOT_PARTITION_DEPTH,
        })
    );

    let target = Enclosure::new(
        Dyadic::new(BigInt::zero(), 0),
        Dyadic::new(BigInt::one().unwrap(), 0),
    )
    .unwrap();
    let near_limit = RootIsolationPolicy::with_limits(
        MAX_ROOT_PARTITION_DEPTH,
        MAX_ROOT_PARTITION_CELLS,
        RootRefinementMethod::Bisection,
        target,
    )
    .unwrap();
    assert_eq!(near_limit.max_cells(), MAX_ROOT_PARTITION_CELLS);

    let target = Enclosure::new(
        Dyadic::new(BigInt::zero(), 0),
        Dyadic::new(BigInt::one().unwrap(), 0),
    )
    .unwrap();
    assert_eq!(
        RootIsolationPolicy::with_limits(
            1,
            MAX_ROOT_PARTITION_CELLS + 1,
            RootRefinementMethod::Bisection,
            target,
        ),
        Err(GeometryFailure::RootPartitionCellLimit {
            requested_cells: MAX_ROOT_PARTITION_CELLS + 1,
            maximum_cells: MAX_ROOT_PARTITION_CELLS,
        })
    );
}
