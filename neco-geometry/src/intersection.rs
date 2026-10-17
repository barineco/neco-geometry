use alloc::boxed::Box;
use alloc::format;
use alloc::string::String;
use alloc::vec;
use alloc::vec::Vec;
use core::cmp::Ordering;
use core::num::NonZeroUsize;

use neco_algnum::RealAlgebraic;
use neco_bigint::{BigInt, BigUint, BigintError, Dyadic, RawRational, Sign};
use neco_calculus::{
    CalculusFailure, ConvergenceStatus as CalculusConvergenceStatus, DerivativeOrder, Enclosure,
    ExactExpressionRequirement, NecoImplementationSource, OwnerId, Polynomial, PolynomialFailure,
    RootBracket as CalculusRootBracket, RootEvidence as CalculusRootEvidence,
    RootIsolation as CalculusRootIsolation, RootMultiplicity, RootRefinementMethod,
    SurfacePolynomial,
};
use neco_geometry_foundation::{CheckedRay3, Direction3, Point2, Point3, Ray3, Vector3};

use crate::evaluation::{
    evaluate_shape, evaluate_surface_witness_checked, evaluate_surface_witness_exact,
};
use crate::types::{
    point_on_loop, CertifiedShapeEvaluation, CoordinateEnclosure3, ExactShapeEvaluation,
    GeometryFailure, ImplicitSurfaceDeclaration, SeamContact, ShapeCoordinate, ShapeDeclaration,
    ShapeEvaluation, ShapeEvaluationRequest, SurfaceCrossingCandidate, SurfaceEvaluationWitness,
    SurfaceParameterPoint, TrimClassification,
};

pub type BranchIdentitySet = Vec<BranchIdentity>;
pub type CurveBranchSet = Vec<CurveBranch>;
pub type IntersectionPointSet = Vec<IntersectionPoint>;
pub type RootBracketSet = Vec<RootBracket>;
pub type BranchIdentityRef = Option<BranchIdentity>;

#[derive(Debug, Eq, PartialEq)]
pub struct IntersectionDomain {
    parameterization: String,
    restriction: String,
    support: String,
}

impl IntersectionDomain {
    fn new(parameterization: String, restriction: String, support: String) -> Self {
        Self {
            parameterization,
            restriction,
            support,
        }
    }

    pub fn parameterization(&self) -> &str {
        &self.parameterization
    }

    pub fn restriction(&self) -> &str {
        &self.restriction
    }

    pub fn support(&self) -> &str {
        &self.support
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct ResidualFunction {
    left_expression: String,
    right_expression: String,
    combined_expression: String,
}

impl ResidualFunction {
    fn new(left_expression: String, right_expression: String) -> Self {
        let combined_expression = format!("({left_expression}) - ({right_expression})");
        Self {
            left_expression,
            right_expression,
            combined_expression,
        }
    }

    pub fn left_expression(&self) -> &str {
        &self.left_expression
    }

    pub fn right_expression(&self) -> &str {
        &self.right_expression
    }

    pub fn combined_expression(&self) -> &str {
        &self.combined_expression
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SupportEvidence {
    CanonicalSupportAndChart,
    CompatibleParametricSupports,
    DistinctEvaluatedScalars,
    EqualEvaluatedScalars,
    EvaluatedSpatialCoordinates,
    ImplicitSupportPair,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SupportUndeterminedReason {
    IncompleteEvaluatedScalar,
    NoCommonParameterization,
    UnclassifiedEvidence,
}

#[derive(Debug, Eq, PartialEq)]
pub enum SupportRelation {
    Coincident { evidence: SupportEvidence },
    Curve { evidence: SupportEvidence },
    Disjoint { evidence: SupportEvidence },
    Finite { evidence: SupportEvidence },
    Undetermined { reason: SupportUndeterminedReason },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RankDegeneracyReason {
    InsufficientCommonRank,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RankUndeterminedReason {
    SupportRelationUnresolved,
}

#[derive(Debug, Eq, PartialEq)]
pub enum RankEvidence {
    Sufficient { rank: usize },
    Degenerate { reason: RankDegeneracyReason },
    Undetermined { reason: RankUndeterminedReason },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RootSign {
    Negative,
    Positive,
    Zero,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RootUndeterminedReason {
    ExactEvidenceUnavailable,
    ExactMultiplicityUnavailable,
    SignChangePointUnavailable,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResidualSourceId(String);

impl ResidualSourceId {
    fn new(value: &str) -> Self {
        Self(String::from(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum RootEvidence {
    SignChange {
        lower_sign: RootSign,
        upper_sign: RootSign,
        derivative_enclosure: Enclosure,
        source: ResidualSourceId,
    },
    EvenMultiplicityTouch {
        order: usize,
        point: RealAlgebraic,
        source: ResidualSourceId,
    },
    HigherOrder {
        order: usize,
        point: RealAlgebraic,
        source: ResidualSourceId,
    },
    SimpleRoot {
        point: RealAlgebraic,
        source: ResidualSourceId,
    },
    Undetermined {
        reason: RootUndeterminedReason,
        source: ResidualSourceId,
    },
}

#[derive(Debug, Eq, PartialEq)]
pub struct IntersectionConstraint {
    left_operand: IntersectionOperand,
    right_operand: IntersectionOperand,
    residual_function: ResidualFunction,
    domain: IntersectionDomain,
    support_relation: SupportRelation,
    rank_evidence: RankEvidence,
}

impl IntersectionConstraint {
    fn new(
        left_operand: IntersectionOperand,
        right_operand: IntersectionOperand,
        residual_function: ResidualFunction,
        domain: IntersectionDomain,
        support_relation: SupportRelation,
        rank_evidence: RankEvidence,
    ) -> Self {
        Self {
            left_operand,
            right_operand,
            residual_function,
            domain,
            support_relation,
            rank_evidence,
        }
    }

    pub fn left_operand(&self) -> &IntersectionOperand {
        &self.left_operand
    }

    pub fn right_operand(&self) -> &IntersectionOperand {
        &self.right_operand
    }

    pub fn residual_function(&self) -> &ResidualFunction {
        &self.residual_function
    }

    pub fn domain(&self) -> &IntersectionDomain {
        &self.domain
    }

    pub fn support_relation(&self) -> &SupportRelation {
        &self.support_relation
    }

    pub fn rank_evidence(&self) -> &RankEvidence {
        &self.rank_evidence
    }

    fn rank_is_undetermined(&self) -> bool {
        matches!(
            self.rank_evidence,
            RankEvidence::Degenerate { .. } | RankEvidence::Undetermined { .. }
        )
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OperandSource {
    Declaration,
    Evaluated,
}

#[derive(Debug, Eq, PartialEq)]
struct OperandDescriptor {
    source: OperandSource,
    family: crate::types::ShapeFamily,
    support_id: String,
    chart_id: String,
    domain: String,
    parameter_bounds: Option<(RealAlgebraic, RealAlgebraic)>,
    expression: String,
    polynomial: Option<Polynomial>,
    exact_scalar: Option<RealAlgebraic>,
    exact_point: Option<Point3>,
    certified_scalar: Option<(
        Enclosure,
        OwnerId,
        ExactExpressionRequirement,
        NecoImplementationSource,
    )>,
}

#[derive(Debug, Eq, PartialEq)]
struct DerivedRootProblem {
    constraint: IntersectionConstraint,
    polynomial: Option<Polynomial>,
    domain: Option<Enclosure>,
}

type PartitionCellId = usize;

#[derive(Debug, Eq, PartialEq)]
struct PartitionCell {
    id: PartitionCellId,
    depth: usize,
    enclosure: Enclosure,
    disposition: CellDisposition,
}

#[derive(Debug, Eq, PartialEq)]
enum CellDisposition {
    Candidate,
    Coincident,
    RootFree,
    Unresolved { reason: PartitionUnresolvedReason },
}

#[derive(Debug, Eq, PartialEq)]
enum RootCandidateBracket {
    Exact { parameter: RealAlgebraic },
    Certified(CalculusRootBracket),
    Unresolved { enclosure: Enclosure },
}

#[derive(Debug, Eq, PartialEq)]
struct RootCandidate {
    bracket: RootCandidateBracket,
    evidence: RootEvidence,
    source_cell: PartitionCellId,
}

#[derive(Debug, Eq, PartialEq)]
struct RootEnumeration {
    candidates: Vec<RootCandidate>,
    cells: Vec<PartitionCell>,
}

impl RootEnumeration {
    fn incomplete(domain: Option<&Enclosure>) -> Self {
        let cells = domain
            .and_then(|enclosure| {
                Some(vec![PartitionCell {
                    id: 0,
                    depth: 0,
                    enclosure: clone_enclosure(enclosure).ok()?,
                    disposition: CellDisposition::Unresolved {
                        reason: PartitionUnresolvedReason::DomainUnavailable,
                    },
                }])
            })
            .unwrap_or_default();
        Self {
            candidates: Vec::new(),
            cells,
        }
    }
}

type RootCandidateSet = RootEnumeration;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PartitionUnresolvedReason {
    DomainUnavailable,
    MultipleExactDyadicRoots,
    SignChangeUniqueness,
    UncertifiedRoot,
}

enum CellAssessment {
    Candidate(Vec<RootCandidate>),
    ExactTouch {
        candidate: Box<RootCandidate>,
        pivot: Dyadic,
    },
    RootFree,
    Unresolved(PartitionUnresolvedReason),
}

struct PendingPartitionCell {
    id: PartitionCellId,
    depth: usize,
    enclosure: Enclosure,
    suppressed_exact_endpoints: Vec<Dyadic>,
}

const ROOT_INTERVAL_PRECISION_BITS: u32 = 32;
pub const MAX_ROOT_PARTITION_DEPTH: usize = 19;
pub const MAX_ROOT_PARTITION_CELLS: usize = 65_535;

fn partition_cells_for_depth(depth: usize) -> usize {
    1usize
        .checked_shl((depth + 1) as u32)
        .and_then(|value| value.checked_sub(1))
        .unwrap_or(MAX_ROOT_PARTITION_CELLS)
        .min(MAX_ROOT_PARTITION_CELLS)
}

fn ensure_partition_cell_capacity(
    completed_cells: usize,
    pending_cells: usize,
    child_cells: usize,
    maximum_cells: usize,
) -> Result<(), GeometryFailure> {
    let requested_cells = completed_cells
        .checked_add(pending_cells)
        .and_then(|value| value.checked_add(child_cells))
        .ok_or(GeometryFailure::RootPartitionIdentifierOverflow {
            current_identifier: usize::MAX,
            increment: child_cells,
        })?;
    if requested_cells > maximum_cells {
        return Err(GeometryFailure::RootPartitionCellLimit {
            requested_cells,
            maximum_cells,
        });
    }
    Ok(())
}

fn take_partition_cell_id(next_cell_id: &mut usize) -> Result<usize, GeometryFailure> {
    let current_identifier = *next_cell_id;
    *next_cell_id = current_identifier.checked_add(1).ok_or(
        GeometryFailure::RootPartitionIdentifierOverflow {
            current_identifier,
            increment: 1,
        },
    )?;
    Ok(current_identifier)
}

#[derive(Debug, Eq, PartialEq)]
pub struct RootIsolation {
    brackets: RootBracketSet,
    completeness: ExistenceWitness,
    method: RootRefinementMethod,
    source_domain: IntersectionDomain,
    source_function: ResidualFunction,
    non_overlap: UniquenessWitness,
}

impl RootIsolation {
    fn new(
        brackets: RootBracketSet,
        completeness: ExistenceWitness,
        method: RootRefinementMethod,
        source_domain: IntersectionDomain,
        source_function: ResidualFunction,
        non_overlap: UniquenessWitness,
    ) -> Self {
        Self {
            brackets,
            completeness,
            method,
            source_domain,
            source_function,
            non_overlap,
        }
    }

    pub fn brackets(&self) -> &[RootBracket] {
        &self.brackets
    }

    pub fn completeness(&self) -> &ExistenceWitness {
        &self.completeness
    }

    pub fn method(&self) -> RootRefinementMethod {
        self.method
    }

    pub fn source_domain(&self) -> &IntersectionDomain {
        &self.source_domain
    }

    pub fn source_function(&self) -> &ResidualFunction {
        &self.source_function
    }

    pub fn non_overlap(&self) -> &UniquenessWitness {
        &self.non_overlap
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct BranchIdentity {
    branch_id: String,
    chart_id: String,
    support_id: String,
}

impl BranchIdentity {
    fn new(branch_id: String, chart_id: String, support_id: String) -> Self {
        Self {
            branch_id,
            chart_id,
            support_id,
        }
    }

    pub fn branch_id(&self) -> &str {
        &self.branch_id
    }

    pub fn chart_id(&self) -> &str {
        &self.chart_id
    }

    pub fn support_id(&self) -> &str {
        &self.support_id
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct IntersectionCurve {
    branch_identity: BranchIdentity,
    contact_order: ContactOrder,
    residual: IntersectionResidual,
    support_id: String,
    witness: IntersectionWitness,
}

impl IntersectionCurve {
    fn new(
        branch_identity: BranchIdentity,
        contact_order: ContactOrder,
        residual: IntersectionResidual,
        support_id: String,
        witness: IntersectionWitness,
    ) -> Self {
        Self {
            branch_identity,
            contact_order,
            residual,
            support_id,
            witness,
        }
    }

    pub fn branch_identity(&self) -> &BranchIdentity {
        &self.branch_identity
    }

    pub fn contact_order(&self) -> &ContactOrder {
        &self.contact_order
    }

    pub fn residual(&self) -> &IntersectionResidual {
        &self.residual
    }

    pub fn support_id(&self) -> &str {
        &self.support_id
    }

    pub fn witness(&self) -> &IntersectionWitness {
        &self.witness
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct CurveBranch {
    branch_identity: BranchIdentity,
    contact_order: ContactOrder,
    residual: IntersectionResidual,
    witness: IntersectionWitness,
}

impl CurveBranch {
    fn new(
        branch_identity: BranchIdentity,
        contact_order: ContactOrder,
        residual: IntersectionResidual,
        witness: IntersectionWitness,
    ) -> Self {
        Self {
            branch_identity,
            contact_order,
            residual,
            witness,
        }
    }

    pub fn branch_identity(&self) -> &BranchIdentity {
        &self.branch_identity
    }

    pub fn contact_order(&self) -> &ContactOrder {
        &self.contact_order
    }

    pub fn residual(&self) -> &IntersectionResidual {
        &self.residual
    }

    pub fn witness(&self) -> &IntersectionWitness {
        &self.witness
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct IntersectionPoint {
    branch_identity: BranchIdentityRef,
    contact_order: ContactOrder,
    point: Point3,
    residual: IntersectionResidual,
    witness: IntersectionWitness,
}

impl IntersectionPoint {
    fn new(
        branch_identity: BranchIdentityRef,
        contact_order: ContactOrder,
        point: Point3,
        residual: IntersectionResidual,
        witness: IntersectionWitness,
    ) -> Self {
        Self {
            branch_identity,
            contact_order,
            point,
            residual,
            witness,
        }
    }

    pub fn branch_identity(&self) -> Option<&BranchIdentity> {
        self.branch_identity.as_ref()
    }

    pub fn contact_order(&self) -> &ContactOrder {
        &self.contact_order
    }

    pub fn point(&self) -> &Point3 {
        &self.point
    }

    pub fn residual(&self) -> &IntersectionResidual {
        &self.residual
    }

    pub fn witness(&self) -> &IntersectionWitness {
        &self.witness
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct IntersectionResidual {
    combined: ResidualValue,
    left: ResidualValue,
    right: ResidualValue,
}

impl IntersectionResidual {
    fn new(combined: ResidualValue, left: ResidualValue, right: ResidualValue) -> Self {
        Self {
            combined,
            left,
            right,
        }
    }

    pub fn combined(&self) -> &ResidualValue {
        &self.combined
    }

    pub fn left(&self) -> &ResidualValue {
        &self.left
    }

    pub fn right(&self) -> &ResidualValue {
        &self.right
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct RootBracket {
    contact_order: ContactOrder,
    convergence: ConvergenceWitness,
    lower: RealAlgebraic,
    sign_change: bool,
    upper: RealAlgebraic,
    evidence: RootEvidence,
    exact_parameter: Option<RealAlgebraic>,
}

impl RootBracket {
    fn from_calculus(
        bracket: &CalculusRootBracket,
        isolation: &CalculusRootIsolation,
        evidence: RootEvidence,
    ) -> Result<Self, GeometryFailure> {
        let contact_order = contact_order_from_evidence(&evidence);
        Ok(Self {
            contact_order,
            convergence: ConvergenceWitness::from_calculus(isolation.witness())?,
            lower: dyadic_to_real(bracket.enclosure().lower())?,
            sign_change: matches!(bracket.evidence(), CalculusRootEvidence::SignChange { .. }),
            upper: dyadic_to_real(bracket.enclosure().upper())?,
            evidence,
            exact_parameter: None,
        })
    }

    fn from_unresolved(
        enclosure: &Enclosure,
        evidence: RootEvidence,
        policy: &RootIsolationPolicy,
    ) -> Result<Self, GeometryFailure> {
        let lower = dyadic_to_real(enclosure.lower())?;
        let upper = dyadic_to_real(enclosure.upper())?;
        Ok(Self {
            contact_order: contact_order_from_evidence(&evidence),
            convergence: ConvergenceWitness::new(
                lower.try_clone()?,
                upper.try_clone()?,
                lower.try_clone()?,
                upper.try_clone()?,
                0,
                policy.method,
                CalculusConvergenceStatus::IterationLimit,
            ),
            lower,
            sign_change: false,
            upper,
            evidence,
            exact_parameter: None,
        })
    }

    fn from_exact(
        parameter: RealAlgebraic,
        evidence: RootEvidence,
        policy: &RootIsolationPolicy,
    ) -> Result<Self, GeometryFailure> {
        let lower = parameter.try_clone()?;
        let upper = parameter.try_clone()?;
        Ok(Self {
            contact_order: contact_order_from_evidence(&evidence),
            convergence: ConvergenceWitness::new(
                parameter.try_clone()?,
                parameter.try_clone()?,
                parameter.try_clone()?,
                parameter.try_clone()?,
                0,
                policy.method,
                CalculusConvergenceStatus::ExactDyadicRoot,
            ),
            lower,
            sign_change: false,
            upper,
            evidence,
            exact_parameter: Some(parameter),
        })
    }

    pub fn contact_order(&self) -> &ContactOrder {
        &self.contact_order
    }

    pub fn convergence(&self) -> &ConvergenceWitness {
        &self.convergence
    }

    pub fn lower(&self) -> &RealAlgebraic {
        &self.lower
    }

    pub fn sign_change(&self) -> bool {
        self.sign_change
    }

    pub fn upper(&self) -> &RealAlgebraic {
        &self.upper
    }

    pub fn evidence(&self) -> &RootEvidence {
        &self.evidence
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct ExistenceWitness {
    evidence: ExistenceEvidence,
    method: WitnessMethod,
    status: WitnessStatus,
}

impl ExistenceWitness {
    fn new(evidence: ExistenceEvidence, method: WitnessMethod, status: WitnessStatus) -> Self {
        Self {
            evidence,
            method,
            status,
        }
    }

    pub fn evidence(&self) -> ExistenceEvidence {
        self.evidence
    }

    pub fn method(&self) -> WitnessMethod {
        self.method
    }

    pub fn status(&self) -> &WitnessStatus {
        &self.status
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct UniquenessWitness {
    evidence: UniquenessEvidence,
    method: WitnessMethod,
    status: WitnessStatus,
}

impl UniquenessWitness {
    fn new(evidence: UniquenessEvidence, method: WitnessMethod, status: WitnessStatus) -> Self {
        Self {
            evidence,
            method,
            status,
        }
    }

    pub fn evidence(&self) -> UniquenessEvidence {
        self.evidence
    }

    pub fn method(&self) -> WitnessMethod {
        self.method
    }

    pub fn status(&self) -> &WitnessStatus {
        &self.status
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct IntersectionWitness {
    existence: ExistenceWitness,
    uniqueness: UniquenessWitness,
    root_isolation: RootIsolation,
    constraint: IntersectionConstraint,
}

impl IntersectionWitness {
    fn from_isolation_and_constraint(
        isolation: RootIsolation,
        constraint: IntersectionConstraint,
    ) -> Self {
        Self {
            existence: clone_existence(&isolation.completeness),
            uniqueness: clone_uniqueness(&isolation.non_overlap),
            root_isolation: isolation,
            constraint,
        }
    }

    pub fn existence(&self) -> &ExistenceWitness {
        &self.existence
    }

    pub fn uniqueness(&self) -> &UniquenessWitness {
        &self.uniqueness
    }

    pub fn root_isolation(&self) -> &RootIsolation {
        &self.root_isolation
    }

    pub fn constraint(&self) -> &IntersectionConstraint {
        &self.constraint
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct ConvergenceWitness {
    final_lower: RealAlgebraic,
    final_upper: RealAlgebraic,
    initial_lower: RealAlgebraic,
    initial_upper: RealAlgebraic,
    iterations: usize,
    method: RootRefinementMethod,
    status: CalculusConvergenceStatus,
}

impl ConvergenceWitness {
    fn new(
        final_lower: RealAlgebraic,
        final_upper: RealAlgebraic,
        initial_lower: RealAlgebraic,
        initial_upper: RealAlgebraic,
        iterations: usize,
        method: RootRefinementMethod,
        status: CalculusConvergenceStatus,
    ) -> Self {
        Self {
            final_lower,
            final_upper,
            initial_lower,
            initial_upper,
            iterations,
            method,
            status,
        }
    }

    pub fn final_lower(&self) -> &RealAlgebraic {
        &self.final_lower
    }

    pub fn final_upper(&self) -> &RealAlgebraic {
        &self.final_upper
    }

    pub fn initial_lower(&self) -> &RealAlgebraic {
        &self.initial_lower
    }

    pub fn initial_upper(&self) -> &RealAlgebraic {
        &self.initial_upper
    }

    pub fn iterations(&self) -> usize {
        self.iterations
    }

    pub fn method(&self) -> RootRefinementMethod {
        self.method
    }

    pub fn status(&self) -> CalculusConvergenceStatus {
        self.status
    }

    fn from_calculus(witness: &neco_calculus::ConvergenceWitness) -> Result<Self, GeometryFailure> {
        Ok(Self::new(
            dyadic_to_real(witness.final_enclosure().lower())?,
            dyadic_to_real(witness.final_enclosure().upper())?,
            dyadic_to_real(witness.initial().lower())?,
            dyadic_to_real(witness.initial().upper())?,
            witness.iterations(),
            witness.method(),
            witness.status(),
        ))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContactOrder {
    kind: ContactOrderKind,
    order: usize,
}

impl ContactOrder {
    fn new(kind: ContactOrderKind, order: usize) -> Self {
        Self { kind, order }
    }

    pub fn kind(&self) -> ContactOrderKind {
        self.kind
    }

    pub fn order(&self) -> usize {
        self.order
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct IntersectionConstructionRequestBody {
    left_operand: IntersectionOperand,
    right_operand: IntersectionOperand,
    root_isolation_policy: RootIsolationPolicy,
}

impl IntersectionConstructionRequestBody {
    pub fn new(
        left_operand: IntersectionOperand,
        right_operand: IntersectionOperand,
        root_isolation_policy: RootIsolationPolicy,
    ) -> Self {
        Self {
            left_operand,
            right_operand,
            root_isolation_policy,
        }
    }

    pub fn left_operand(&self) -> &IntersectionOperand {
        &self.left_operand
    }

    pub fn right_operand(&self) -> &IntersectionOperand {
        &self.right_operand
    }

    pub fn root_isolation_policy(&self) -> &RootIsolationPolicy {
        &self.root_isolation_policy
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct IntersectionConstructionRequest {
    body: IntersectionConstructionRequestBody,
}

impl IntersectionConstructionRequest {
    pub fn new(body: IntersectionConstructionRequestBody) -> Self {
        Self { body }
    }

    pub fn body(&self) -> &IntersectionConstructionRequestBody {
        &self.body
    }

    fn into_body(self) -> IntersectionConstructionRequestBody {
        self.body
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct CoincidentSupport {
    left_support: ShapeDeclaration,
    residual: IntersectionResidual,
    right_support: ShapeDeclaration,
    witness: IntersectionWitness,
}

impl CoincidentSupport {
    fn new(
        left_support: ShapeDeclaration,
        residual: IntersectionResidual,
        right_support: ShapeDeclaration,
        witness: IntersectionWitness,
    ) -> Self {
        Self {
            left_support,
            residual,
            right_support,
            witness,
        }
    }

    pub fn left_support(&self) -> &ShapeDeclaration {
        &self.left_support
    }

    pub fn residual(&self) -> &IntersectionResidual {
        &self.residual
    }

    pub fn right_support(&self) -> &ShapeDeclaration {
        &self.right_support
    }

    pub fn witness(&self) -> &IntersectionWitness {
        &self.witness
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct RootIsolationPolicy {
    max_cells: usize,
    max_iterations: usize,
    method: RootRefinementMethod,
    target_width: Enclosure,
}

impl RootIsolationPolicy {
    pub fn new(
        max_iterations: usize,
        method: RootRefinementMethod,
        target_width: Enclosure,
    ) -> Result<Self, GeometryFailure> {
        if max_iterations == 0 {
            return Err(GeometryFailure::RootIsolationPolicyInvalid);
        }
        if max_iterations > MAX_ROOT_PARTITION_DEPTH {
            return Err(GeometryFailure::RootIsolationResourceLimit {
                requested_depth: max_iterations,
                maximum_depth: MAX_ROOT_PARTITION_DEPTH,
            });
        }
        Ok(Self {
            max_cells: partition_cells_for_depth(max_iterations),
            max_iterations,
            method,
            target_width,
        })
    }

    pub fn with_limits(
        max_iterations: usize,
        max_cells: usize,
        method: RootRefinementMethod,
        target_width: Enclosure,
    ) -> Result<Self, GeometryFailure> {
        if max_cells == 0 {
            return Err(GeometryFailure::RootIsolationPolicyInvalid);
        }
        if max_cells > MAX_ROOT_PARTITION_CELLS {
            return Err(GeometryFailure::RootPartitionCellLimit {
                requested_cells: max_cells,
                maximum_cells: MAX_ROOT_PARTITION_CELLS,
            });
        }
        let mut policy = Self::new(max_iterations, method, target_width)?;
        policy.max_cells = max_cells;
        Ok(policy)
    }

    pub fn max_cells(&self) -> usize {
        self.max_cells
    }

    pub fn max_iterations(&self) -> usize {
        self.max_iterations
    }

    pub fn method(&self) -> RootRefinementMethod {
        self.method
    }

    pub fn target_width(&self) -> &Enclosure {
        &self.target_width
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IntersectionUndeterminedReason {
    BranchIdentityUndetermined,
    CoincidenceUndetermined,
    EvaluationUndetermined,
    MultiplicityUndetermined,
    RootIsolationIncomplete,
}

#[derive(Debug, Eq, PartialEq)]
pub enum ResidualValue {
    Certified {
        owner: OwnerId,
        requirement: ExactExpressionRequirement,
        source: NecoImplementationSource,
        value: Enclosure,
    },
    Exact {
        value: RealAlgebraic,
    },
    Undetermined {
        reason: ResidualUndeterminedReason,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResidualUndeterminedReason {
    OperandEvidenceUnresolved,
    ScalarResidualUnavailable,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct IntersectionComponentIdentity(usize);

impl IntersectionComponentIdentity {
    pub fn ordinal(&self) -> usize {
        self.0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IntersectionKind {
    Coincident,
    Curve,
    CurveBranches,
    Empty,
    FinitePoints,
    Point,
    Undetermined,
}

pub enum ComponentEvidenceRef<'a> {
    Point(&'a IntersectionPoint),
    Curve(&'a IntersectionCurve),
    CurveBranch(&'a CurveBranch),
    Coincident(&'a CoincidentSupport),
}

impl<'a> ComponentEvidenceRef<'a> {
    pub fn branch_identity(&self) -> Option<&'a BranchIdentity> {
        match self {
            Self::Point(value) => value.branch_identity(),
            Self::Curve(value) => Some(value.branch_identity()),
            Self::CurveBranch(value) => Some(value.branch_identity()),
            Self::Coincident(_) => None,
        }
    }

    pub fn contact_order(&self) -> Option<&'a ContactOrder> {
        match self {
            Self::Point(value) => Some(value.contact_order()),
            Self::Curve(value) => Some(value.contact_order()),
            Self::CurveBranch(value) => Some(value.contact_order()),
            Self::Coincident(_) => None,
        }
    }
}

pub struct IntersectionComponentViewEntry<'a> {
    identity: IntersectionComponentIdentity,
    evidence: ComponentEvidenceRef<'a>,
}

impl<'a> IntersectionComponentViewEntry<'a> {
    pub fn identity(&self) -> IntersectionComponentIdentity {
        self.identity
    }
    pub fn evidence(&self) -> &ComponentEvidenceRef<'a> {
        &self.evidence
    }
}

pub struct IntersectionComponentIter<'a> {
    inner: core::slice::Iter<'a, OwnedIntersectionComponent>,
}

impl<'a> Iterator for IntersectionComponentIter<'a> {
    type Item = IntersectionComponentViewEntry<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|entry| {
            let evidence = match &entry.evidence {
                OwnedIntersectionEvidence::Point(value) => {
                    ComponentEvidenceRef::Point(value.as_ref())
                }
                OwnedIntersectionEvidence::Curve(value) => {
                    ComponentEvidenceRef::Curve(value.as_ref())
                }
                OwnedIntersectionEvidence::CurveBranch(value) => {
                    ComponentEvidenceRef::CurveBranch(value.as_ref())
                }
                OwnedIntersectionEvidence::Coincident(value) => {
                    ComponentEvidenceRef::Coincident(value.as_ref())
                }
            };
            IntersectionComponentViewEntry {
                identity: entry.identity,
                evidence,
            }
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl ExactSizeIterator for IntersectionComponentIter<'_> {}

pub enum IntersectionComponentView<'a> {
    Components(IntersectionComponentIter<'a>),
    NoComponents,
    Undetermined(IntersectionUndeterminedReason),
}

#[derive(Debug, Eq, PartialEq)]
enum OwnedIntersectionEvidence {
    Point(Box<IntersectionPoint>),
    Curve(Box<IntersectionCurve>),
    CurveBranch(Box<CurveBranch>),
    Coincident(Box<CoincidentSupport>),
}

#[derive(Debug, Eq, PartialEq)]
struct OwnedIntersectionComponent {
    identity: IntersectionComponentIdentity,
    evidence: OwnedIntersectionEvidence,
}

impl OwnedIntersectionComponent {
    fn validate(&self) -> Result<(), GeometryFailure> {
        match self.evidence {
            OwnedIntersectionEvidence::Point(_)
            | OwnedIntersectionEvidence::Curve(_)
            | OwnedIntersectionEvidence::CurveBranch(_)
            | OwnedIntersectionEvidence::Coincident(_) => Ok(()),
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
#[must_use]
pub struct IntersectionSet {
    kind: IntersectionKind,
    witness: IntersectionWitness,
    residual: IntersectionResidual,
    reason: Option<IntersectionUndeterminedReason>,
    components: Vec<OwnedIntersectionComponent>,
}

impl IntersectionSet {
    pub fn kind(&self) -> IntersectionKind {
        self.kind
    }
    pub fn witness(&self) -> &IntersectionWitness {
        &self.witness
    }
    pub fn residual(&self) -> &IntersectionResidual {
        &self.residual
    }
    pub fn component_view(&self) -> IntersectionComponentView<'_> {
        if let Some(reason) = self.reason {
            return IntersectionComponentView::Undetermined(reason);
        }
        if self.components.is_empty() {
            return IntersectionComponentView::NoComponents;
        }
        IntersectionComponentView::Components(IntersectionComponentIter {
            inner: self.components.iter(),
        })
    }

    fn empty(residual: IntersectionResidual, witness: IntersectionWitness) -> Self {
        Self {
            kind: IntersectionKind::Empty,
            witness,
            residual,
            reason: None,
            components: Vec::new(),
        }
    }

    fn undetermined(
        reason: IntersectionUndeterminedReason,
        residual: IntersectionResidual,
        witness: IntersectionWitness,
    ) -> Self {
        Self {
            kind: IntersectionKind::Undetermined,
            witness,
            residual,
            reason: Some(reason),
            components: Vec::new(),
        }
    }

    fn from_components(
        kind: IntersectionKind,
        residual: IntersectionResidual,
        witness: IntersectionWitness,
        evidence: Vec<OwnedIntersectionEvidence>,
    ) -> Result<Self, GeometryFailure> {
        let expected = match kind {
            IntersectionKind::Coincident | IntersectionKind::Point | IntersectionKind::Curve => 1,
            IntersectionKind::FinitePoints | IntersectionKind::CurveBranches => {
                if evidence.is_empty() {
                    return Err(if kind == IntersectionKind::FinitePoints {
                        GeometryFailure::EmptyFinitePointComponents
                    } else {
                        GeometryFailure::EmptyCurveBranchComponents
                    });
                }
                evidence.len()
            }
            IntersectionKind::Empty | IntersectionKind::Undetermined => {
                return Err(GeometryFailure::ComponentIdentityEvidenceMismatch);
            }
        };
        if evidence.len() != expected {
            return Err(GeometryFailure::ComponentIdentityEvidenceMismatch);
        }
        let components = evidence
            .into_iter()
            .enumerate()
            .map(|(ordinal, evidence)| OwnedIntersectionComponent {
                identity: IntersectionComponentIdentity(ordinal),
                evidence,
            })
            .collect::<Vec<_>>();
        if components
            .windows(2)
            .any(|pair| pair[0].identity >= pair[1].identity)
        {
            return Err(GeometryFailure::DuplicateIntersectionComponentIdentity);
        }
        for component in &components {
            component
                .validate()
                .map_err(|_| GeometryFailure::ComponentIdentityEvidenceMismatch)?;
        }
        if components.len() != expected {
            return Err(GeometryFailure::ComponentIdentityCanonicalizationFailure);
        }
        let kind_matches = match kind {
            IntersectionKind::Coincident => matches!(
                components[0].evidence,
                OwnedIntersectionEvidence::Coincident(_)
            ),
            IntersectionKind::Point | IntersectionKind::FinitePoints => components
                .iter()
                .all(|entry| matches!(entry.evidence, OwnedIntersectionEvidence::Point(_))),
            IntersectionKind::Curve => {
                matches!(components[0].evidence, OwnedIntersectionEvidence::Curve(_))
            }
            IntersectionKind::CurveBranches => components
                .iter()
                .all(|entry| matches!(entry.evidence, OwnedIntersectionEvidence::CurveBranch(_))),
            IntersectionKind::Empty | IntersectionKind::Undetermined => false,
        };
        if !kind_matches {
            return Err(GeometryFailure::ComponentIdentityEvidenceMismatch);
        }
        Ok(Self {
            kind,
            witness,
            residual,
            reason: None,
            components,
        })
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum IntersectionOperand {
    Declaration(ShapeDeclaration),
    Evaluated(ShapeEvaluation),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ContactOrderKind {
    HigherOrder,
    Tangent,
    Transverse,
    Undetermined,
}

#[derive(Debug, Eq, PartialEq)]
pub enum WitnessStatus {
    Proved,
    Undetermined { reason: WitnessUndeterminedReason },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WitnessMethod {
    CanonicalLowerEndpointOrdering,
    DeterministicDyadicCandidatePartition,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExistenceEvidence {
    CompleteCandidatePartition,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UniquenessEvidence {
    PairwiseNonOverlappingBrackets,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WitnessUndeterminedReason {
    BracketSetInvalid,
    PartitionOrCandidatesIncomplete,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct RayCrossingTargetId(String);

impl RayCrossingTargetId {
    pub fn new(value: impl Into<String>) -> Result<Self, GeometryFailure> {
        let value = value.into();
        if value.is_empty() {
            return Err(GeometryFailure::InvalidRayCrossingPolicy);
        }
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct ExactTriangleTarget {
    pub id: RayCrossingTargetId,
    pub a: Point3,
    pub b: Point3,
    pub c: Point3,
}

impl ExactTriangleTarget {
    pub fn new(
        id: RayCrossingTargetId,
        a: Point3,
        b: Point3,
        c: Point3,
    ) -> Result<Self, GeometryFailure> {
        if exact_triangle_is_degenerate(&a, &b, &c)? {
            return Err(GeometryFailure::DegenerateTriangle);
        }
        Ok(Self { id, a, b, c })
    }
}

#[derive(Debug, PartialEq)]
pub struct CheckedTriangleTarget {
    pub id: RayCrossingTargetId,
    pub vertices: [[f64; 3]; 3],
    owner: OwnerId,
    requirement: ExactExpressionRequirement,
    source: NecoImplementationSource,
}

impl CheckedTriangleTarget {
    pub fn new(
        id: RayCrossingTargetId,
        vertices: [[f64; 3]; 3],
        owner: OwnerId,
        requirement: ExactExpressionRequirement,
        source: NecoImplementationSource,
    ) -> Result<Self, GeometryFailure> {
        if vertices.iter().flatten().any(|value| !value.is_finite()) {
            return Err(GeometryFailure::DegenerateTriangle);
        }
        if checked_triangle_is_degenerate(vertices) {
            return Err(GeometryFailure::DegenerateTriangle);
        }
        Ok(Self {
            id,
            vertices,
            owner,
            requirement,
            source,
        })
    }

    pub const fn owner(&self) -> OwnerId {
        self.owner
    }

    pub fn requirement(&self) -> &ExactExpressionRequirement {
        &self.requirement
    }

    pub const fn source(&self) -> NecoImplementationSource {
        self.source
    }
}

fn exact_triangle_is_degenerate(
    a: &Point3,
    b: &Point3,
    c: &Point3,
) -> Result<bool, GeometryFailure> {
    let ab = [a.x.sub(&b.x)?, a.y.sub(&b.y)?, a.z.sub(&b.z)?];
    let ac = [a.x.sub(&c.x)?, a.y.sub(&c.y)?, a.z.sub(&c.z)?];
    let cross = [
        ab[1].mul(&ac[2])?.sub(&ab[2].mul(&ac[1])?)?,
        ab[2].mul(&ac[0])?.sub(&ab[0].mul(&ac[2])?)?,
        ab[0].mul(&ac[1])?.sub(&ab[1].mul(&ac[0])?)?,
    ];
    Ok(cross.iter().all(RealAlgebraic::is_zero))
}

fn checked_triangle_is_degenerate(vertices: [[f64; 3]; 3]) -> bool {
    let ab = [
        vertices[1][0] - vertices[0][0],
        vertices[1][1] - vertices[0][1],
        vertices[1][2] - vertices[0][2],
    ];
    let ac = [
        vertices[2][0] - vertices[0][0],
        vertices[2][1] - vertices[0][1],
        vertices[2][2] - vertices[0][2],
    ];
    let cross = [
        ab[1] * ac[2] - ab[2] * ac[1],
        ab[2] * ac[0] - ab[0] * ac[2],
        ab[0] * ac[1] - ab[1] * ac[0],
    ];
    cross == [0.0; 3]
}

#[derive(Debug, Eq, PartialEq)]
pub struct TrimmedParametricSurfaceTarget {
    id: RayCrossingTargetId,
    candidate: SurfaceCrossingCandidate,
}

impl TrimmedParametricSurfaceTarget {
    pub fn new(id: RayCrossingTargetId, candidate: SurfaceCrossingCandidate) -> Self {
        Self { id, candidate }
    }
    pub fn id(&self) -> &RayCrossingTargetId {
        &self.id
    }
    pub fn candidate(&self) -> &SurfaceCrossingCandidate {
        &self.candidate
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct CheckedTrimmedParametricSurfaceTarget {
    id: RayCrossingTargetId,
    candidate: SurfaceCrossingCandidate,
    owner: OwnerId,
    requirement: ExactExpressionRequirement,
    source: NecoImplementationSource,
}

impl CheckedTrimmedParametricSurfaceTarget {
    pub fn new(
        id: RayCrossingTargetId,
        candidate: SurfaceCrossingCandidate,
        owner: OwnerId,
        requirement: ExactExpressionRequirement,
        source: NecoImplementationSource,
    ) -> Self {
        Self {
            id,
            candidate,
            owner,
            requirement,
            source,
        }
    }

    pub fn id(&self) -> &RayCrossingTargetId {
        &self.id
    }

    pub fn candidate(&self) -> &SurfaceCrossingCandidate {
        &self.candidate
    }

    pub const fn owner(&self) -> OwnerId {
        self.owner
    }

    pub fn requirement(&self) -> &ExactExpressionRequirement {
        &self.requirement
    }

    pub const fn source(&self) -> NecoImplementationSource {
        self.source
    }
}

#[derive(Debug, PartialEq)]
pub enum ExactRayCrossingTarget {
    Triangle(ExactTriangleTarget),
    TrimmedSurface(TrimmedParametricSurfaceTarget),
    ImplicitSurface {
        id: RayCrossingTargetId,
        surface: ImplicitSurfaceDeclaration,
    },
}
#[derive(Debug, PartialEq)]
pub enum CheckedRayCrossingTarget {
    Triangle(CheckedTriangleTarget),
    TrimmedSurface(Box<CheckedTrimmedParametricSurfaceTarget>),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RayCrossingTargetFamily {
    Triangle,
    TrimmedSurface,
    ImplicitSurface,
}

#[derive(Debug, PartialEq)]
pub struct ExactRayCrossingTargetSet(Vec<ExactRayCrossingTarget>);
impl ExactRayCrossingTargetSet {
    pub fn new(targets: Vec<ExactRayCrossingTarget>) -> Result<Self, GeometryFailure> {
        if targets.is_empty() {
            return Err(GeometryFailure::EmptyRayCrossingTargetSet);
        }
        let families = targets.iter().map(exact_target_family).collect::<Vec<_>>();
        if families.windows(2).any(|pair| pair[0] != pair[1]) {
            return Err(GeometryFailure::MixedRayCrossingTargetFamily);
        }
        let ids = targets.iter().map(exact_target_id).collect::<Vec<_>>();
        validate_target_ids(ids)?;
        Ok(Self(targets))
    }
    pub fn targets(&self) -> &[ExactRayCrossingTarget] {
        &self.0
    }
}
#[derive(Debug, PartialEq)]
pub struct CheckedRayCrossingTargetSet(Vec<CheckedRayCrossingTarget>);
impl CheckedRayCrossingTargetSet {
    pub fn new(targets: Vec<CheckedRayCrossingTarget>) -> Result<Self, GeometryFailure> {
        if targets.is_empty() {
            return Err(GeometryFailure::EmptyRayCrossingTargetSet);
        }
        let families = targets
            .iter()
            .map(checked_target_family)
            .collect::<Vec<_>>();
        if families.windows(2).any(|pair| pair[0] != pair[1]) {
            return Err(GeometryFailure::MixedRayCrossingTargetFamily);
        }
        let ids = targets.iter().map(checked_target_id).collect::<Vec<_>>();
        validate_target_ids(ids)?;
        Ok(Self(targets))
    }
    pub fn targets(&self) -> &[CheckedRayCrossingTarget] {
        &self.0
    }
}

fn exact_target_family(target: &ExactRayCrossingTarget) -> RayCrossingTargetFamily {
    match target {
        ExactRayCrossingTarget::Triangle(_) => RayCrossingTargetFamily::Triangle,
        ExactRayCrossingTarget::TrimmedSurface(_) => RayCrossingTargetFamily::TrimmedSurface,
        ExactRayCrossingTarget::ImplicitSurface { .. } => RayCrossingTargetFamily::ImplicitSurface,
    }
}
fn checked_target_family(target: &CheckedRayCrossingTarget) -> RayCrossingTargetFamily {
    match target {
        CheckedRayCrossingTarget::Triangle(_) => RayCrossingTargetFamily::Triangle,
        CheckedRayCrossingTarget::TrimmedSurface(_) => RayCrossingTargetFamily::TrimmedSurface,
    }
}
fn exact_target_id(target: &ExactRayCrossingTarget) -> &RayCrossingTargetId {
    match target {
        ExactRayCrossingTarget::Triangle(target) => &target.id,
        ExactRayCrossingTarget::TrimmedSurface(target) => target.id(),
        ExactRayCrossingTarget::ImplicitSurface { id, .. } => id,
    }
}
fn checked_target_id(target: &CheckedRayCrossingTarget) -> &RayCrossingTargetId {
    match target {
        CheckedRayCrossingTarget::Triangle(target) => &target.id,
        CheckedRayCrossingTarget::TrimmedSurface(target) => target.id(),
    }
}
fn validate_target_ids(mut ids: Vec<&RayCrossingTargetId>) -> Result<(), GeometryFailure> {
    ids.sort_unstable();
    if ids.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(GeometryFailure::DuplicateRayCrossingTargetIdentity);
    }
    Ok(())
}

#[derive(Debug, Eq, PartialEq)]
pub struct RayCrossingPolicy {
    pub root_isolation: RootIsolationPolicy,
    pub max_targets: NonZeroUsize,
    pub max_hits: NonZeroUsize,
}
impl RayCrossingPolicy {
    pub fn new(
        root_isolation: RootIsolationPolicy,
        max_targets: usize,
        max_hits: usize,
    ) -> Result<Self, GeometryFailure> {
        Ok(Self {
            root_isolation,
            max_targets: NonZeroUsize::new(max_targets)
                .ok_or(GeometryFailure::InvalidRayCrossingPolicy)?,
            max_hits: NonZeroUsize::new(max_hits)
                .ok_or(GeometryFailure::InvalidRayCrossingPolicy)?,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RayOriginContact {
    None,
    Interior,
    Boundary,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RayCrossingLocation {
    Interior,
    TrimBoundary,
    HoleBoundary,
    Seam,
    Edge,
    Vertex,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CrossingMultiplicity {
    Simple,
    Odd(usize),
    Even(usize),
    Undetermined,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CrossingContribution {
    Toggle,
    NoToggle,
    Undetermined,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CrossingParity {
    Even,
    Odd,
    Undetermined,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RayCrossingUndeterminedReason {
    Edge,
    Vertex,
    TrimBoundary,
    HoleBoundary,
    DuplicateParameter,
    CoplanarOverlap,
    Coincident,
    InvalidMultiplicity,
    UnresolvedSeam,
    NormalIndeterminate,
}

#[derive(Debug, Eq, PartialEq)]
pub enum RayNormal {
    Regular(Direction3),
    Undetermined,
}

#[derive(Debug, Eq, PartialEq)]
pub struct RayCrossingHit {
    target_id: RayCrossingTargetId,
    parameter: RealAlgebraic,
    point: Point3,
    location: RayCrossingLocation,
    multiplicity: CrossingMultiplicity,
    contribution: CrossingContribution,
    normal: RayNormal,
}

impl RayCrossingHit {
    pub fn new(
        target_id: RayCrossingTargetId,
        parameter: RealAlgebraic,
        point: Point3,
        location: RayCrossingLocation,
        multiplicity: CrossingMultiplicity,
        contribution: CrossingContribution,
        normal: RayNormal,
    ) -> Self {
        Self {
            target_id,
            parameter,
            point,
            location,
            multiplicity,
            contribution,
            normal,
        }
    }
    pub fn target_id(&self) -> &RayCrossingTargetId {
        &self.target_id
    }
    pub fn parameter(&self) -> &RealAlgebraic {
        &self.parameter
    }
    pub fn point(&self) -> &Point3 {
        &self.point
    }
    pub fn location(&self) -> RayCrossingLocation {
        self.location
    }
    pub fn multiplicity(&self) -> CrossingMultiplicity {
        self.multiplicity
    }
    pub fn contribution(&self) -> CrossingContribution {
        self.contribution
    }

    pub fn normal(&self) -> &RayNormal {
        &self.normal
    }
}

#[derive(Debug, PartialEq)]
pub struct CheckedRayCrossingHit {
    target_id: RayCrossingTargetId,
    parameter: Enclosure,
    point: CoordinateEnclosure3,
    location: RayCrossingLocation,
    multiplicity: CrossingMultiplicity,
    contribution: CrossingContribution,
    owner: OwnerId,
    requirement: ExactExpressionRequirement,
    source: NecoImplementationSource,
    normal: RayNormal,
}

struct CheckedHitGeometry {
    location: RayCrossingLocation,
    multiplicity: CrossingMultiplicity,
    contribution: CrossingContribution,
    owner: OwnerId,
    requirement: ExactExpressionRequirement,
    source: NecoImplementationSource,
    normal: RayNormal,
}

impl CheckedRayCrossingHit {
    fn from_exact_hit(
        target_id: RayCrossingTargetId,
        parameter: Enclosure,
        point: CoordinateEnclosure3,
        geometry: CheckedHitGeometry,
    ) -> Self {
        Self {
            target_id,
            parameter,
            point,
            location: geometry.location,
            multiplicity: geometry.multiplicity,
            contribution: geometry.contribution,
            owner: geometry.owner,
            requirement: geometry.requirement,
            source: geometry.source,
            normal: geometry.normal,
        }
    }
    pub fn target_id(&self) -> &RayCrossingTargetId {
        &self.target_id
    }
    pub fn parameter(&self) -> &Enclosure {
        &self.parameter
    }
    pub fn point(&self) -> &CoordinateEnclosure3 {
        &self.point
    }
    pub fn location(&self) -> RayCrossingLocation {
        self.location
    }
    pub fn multiplicity(&self) -> CrossingMultiplicity {
        self.multiplicity
    }
    pub fn contribution(&self) -> CrossingContribution {
        self.contribution
    }
    pub const fn owner(&self) -> OwnerId {
        self.owner
    }

    pub fn requirement(&self) -> &ExactExpressionRequirement {
        &self.requirement
    }

    pub const fn source(&self) -> NecoImplementationSource {
        self.source
    }
    pub fn normal(&self) -> &RayNormal {
        &self.normal
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct RayCrossingWitness {
    target_count: usize,
    target_ids: Vec<RayCrossingTargetId>,
    parity: CrossingParity,
}
impl RayCrossingWitness {
    pub fn new(target_ids: Vec<RayCrossingTargetId>, parity: CrossingParity) -> Self {
        let target_count = target_ids.len();
        Self {
            target_count,
            target_ids,
            parity,
        }
    }
    pub fn target_count(&self) -> usize {
        self.target_count
    }
    pub fn target_ids(&self) -> &[RayCrossingTargetId] {
        &self.target_ids
    }
    pub fn parity(&self) -> CrossingParity {
        self.parity
    }
}

#[derive(Debug, PartialEq)]
pub struct CheckedRayCrossingWitness {
    target_count: usize,
    target_ids: Vec<RayCrossingTargetId>,
    parity: CrossingParity,
}
impl CheckedRayCrossingWitness {
    pub fn new(target_ids: Vec<RayCrossingTargetId>, parity: CrossingParity) -> Self {
        let target_count = target_ids.len();
        Self {
            target_count,
            target_ids,
            parity,
        }
    }
    pub fn target_count(&self) -> usize {
        self.target_count
    }
    pub fn target_ids(&self) -> &[RayCrossingTargetId] {
        &self.target_ids
    }
    pub fn parity(&self) -> CrossingParity {
        self.parity
    }
}
#[derive(Debug, Eq, PartialEq)]
pub enum RayCrossingResult {
    Proved {
        origin_contact: RayOriginContact,
        parity: CrossingParity,
        hits: Vec<RayCrossingHit>,
        witness: RayCrossingWitness,
    },
    Undetermined {
        reason: RayCrossingUndeterminedReason,
        origin_contact: RayOriginContact,
        hits: Vec<RayCrossingHit>,
        witness: RayCrossingWitness,
    },
}

#[derive(Debug, PartialEq)]
pub enum CheckedRayCrossingResult {
    Proved {
        origin_contact: RayOriginContact,
        parity: CrossingParity,
        hits: Vec<CheckedRayCrossingHit>,
        witness: CheckedRayCrossingWitness,
    },
    Undetermined {
        reason: RayCrossingUndeterminedReason,
        origin_contact: RayOriginContact,
        hits: Vec<CheckedRayCrossingHit>,
        witness: CheckedRayCrossingWitness,
    },
}

pub fn cross_ray3(
    ray: Ray3,
    targets: ExactRayCrossingTargetSet,
    range: crate::types::PositiveRayParameterRange,
    policy: RayCrossingPolicy,
) -> Result<RayCrossingResult, GeometryFailure> {
    validate_policy(&policy.root_isolation)?;
    if targets.targets().len() > policy.max_targets.get() {
        return Err(GeometryFailure::RayCrossingTargetLimitExceeded);
    }

    let mut origin_contact = RayOriginContact::None;
    let mut hits = Vec::new();
    let mut undetermined = None;
    for target in targets.targets() {
        match target {
            ExactRayCrossingTarget::Triangle(triangle) => {
                let evaluation = cross_exact_triangle(&ray, triangle, &range)?;
                origin_contact = combine_origin_contact(origin_contact, evaluation.origin_contact);
                if let Some(reason) = evaluation.reason {
                    undetermined.get_or_insert(reason);
                }
                if let Some(hit) = evaluation.hit {
                    hits.push(hit);
                }
            }
            ExactRayCrossingTarget::ImplicitSurface { id, surface } => {
                let evaluation =
                    cross_exact_implicit(&ray, id, surface, &range, &policy.root_isolation)?;
                origin_contact = combine_origin_contact(origin_contact, evaluation.origin_contact);
                if let Some(reason) = evaluation.reason {
                    undetermined.get_or_insert(reason);
                }
                hits.extend(evaluation.hits);
            }
            ExactRayCrossingTarget::TrimmedSurface(surface) => {
                let evaluation =
                    cross_exact_trimmed_surface(&ray, surface.id(), surface.candidate(), &range)?;
                origin_contact = combine_origin_contact(origin_contact, evaluation.origin_contact);
                if let Some(reason) = evaluation.reason {
                    undetermined.get_or_insert(reason);
                }
                if let Some(hit) = evaluation.hit {
                    hits.push(hit);
                }
            }
        }
        if hits.len() > policy.max_hits.get() {
            return Err(GeometryFailure::RayCrossingHitLimitExceeded);
        }
    }
    hits.sort_by(
        |left, right| match left.parameter.compare(&right.parameter) {
            Ok(Ordering::Equal) => left.target_id.cmp(&right.target_id),
            Ok(ordering) => ordering,
            Err(_) => Ordering::Equal,
        },
    );
    if hits.windows(2).any(|pair| {
        pair[0]
            .parameter
            .compare(&pair[1].parameter)
            .is_ok_and(|ordering| ordering == Ordering::Equal)
    }) {
        undetermined.get_or_insert(RayCrossingUndeterminedReason::DuplicateParameter);
    }
    let parity = if undetermined.is_some() {
        CrossingParity::Undetermined
    } else if hits
        .iter()
        .filter(|hit| hit.contribution == CrossingContribution::Toggle)
        .count()
        % 2
        == 0
    {
        CrossingParity::Even
    } else {
        CrossingParity::Odd
    };
    let target_ids = targets
        .targets()
        .iter()
        .map(exact_target_id)
        .map(|id| RayCrossingTargetId(id.0.clone()))
        .collect();
    let witness = RayCrossingWitness::new(target_ids, parity);
    Ok(match undetermined {
        Some(reason) => RayCrossingResult::Undetermined {
            reason,
            origin_contact,
            hits,
            witness,
        },
        None => RayCrossingResult::Proved {
            origin_contact,
            parity,
            hits,
            witness,
        },
    })
}

pub fn cross_checked_ray3(
    ray: CheckedRay3,
    targets: CheckedRayCrossingTargetSet,
    range: crate::types::PositiveRayParameterRange,
    policy: RayCrossingPolicy,
) -> Result<CheckedRayCrossingResult, GeometryFailure> {
    validate_policy(&policy.root_isolation)?;
    if targets.targets().len() > policy.max_targets.get() {
        return Err(GeometryFailure::RayCrossingTargetLimitExceeded);
    }
    let exact_ray = match exact_ray_from_checked(&ray) {
        Some(exact_ray) => exact_ray,
        None => {
            return Ok(checked_undetermined_result(
                &targets,
                RayCrossingUndeterminedReason::InvalidMultiplicity,
            ));
        }
    };
    let mut origin_contact = RayOriginContact::None;
    let mut hits = Vec::new();
    let mut undetermined = None;
    for target in targets.targets() {
        match target {
            CheckedRayCrossingTarget::Triangle(triangle) => {
                let exact_triangle = match exact_triangle_from_checked(triangle) {
                    Some(exact_triangle) => exact_triangle,
                    None => {
                        undetermined
                            .get_or_insert(RayCrossingUndeterminedReason::InvalidMultiplicity);
                        continue;
                    }
                };
                let evaluation = cross_exact_triangle(&exact_ray, &exact_triangle, &range)?;
                origin_contact = combine_origin_contact(origin_contact, evaluation.origin_contact);
                if let Some(reason) = evaluation.reason {
                    undetermined.get_or_insert(reason);
                }
                if let Some(hit) = evaluation.hit {
                    hits.push(checked_hit_from_exact(
                        hit,
                        triangle.owner(),
                        clone_requirement(triangle.requirement())?,
                        triangle.source(),
                    )?);
                }
            }
            CheckedRayCrossingTarget::TrimmedSurface(surface) => {
                let evaluation = cross_exact_trimmed_surface(
                    &exact_ray,
                    surface.id(),
                    surface.candidate(),
                    &range,
                )?;
                origin_contact = combine_origin_contact(origin_contact, evaluation.origin_contact);
                if let Some(reason) = evaluation.reason {
                    undetermined.get_or_insert(reason);
                }
                if let Some(hit) = evaluation.hit {
                    hits.push(checked_surface_hit(hit, surface)?);
                }
            }
        }
        if hits.len() > policy.max_hits.get() {
            return Err(GeometryFailure::RayCrossingHitLimitExceeded);
        }
    }
    hits.sort_by(
        |left, right| match compare_enclosures(left.parameter(), right.parameter()) {
            Ok(ordering) => ordering.then(left.target_id().cmp(right.target_id())),
            Err(_) => left.target_id().cmp(right.target_id()),
        },
    );
    if hits
        .windows(2)
        .any(|pair| enclosures_overlap(pair[0].parameter(), pair[1].parameter()).unwrap_or(false))
    {
        undetermined.get_or_insert(RayCrossingUndeterminedReason::DuplicateParameter);
    }
    let parity = if undetermined.is_some() {
        CrossingParity::Undetermined
    } else if hits
        .iter()
        .filter(|hit| hit.contribution() == CrossingContribution::Toggle)
        .count()
        % 2
        == 0
    {
        CrossingParity::Even
    } else {
        CrossingParity::Odd
    };
    let target_ids = targets
        .targets()
        .iter()
        .map(checked_target_id)
        .map(|id| RayCrossingTargetId(id.0.clone()))
        .collect();
    let witness = CheckedRayCrossingWitness::new(target_ids, parity);
    Ok(match undetermined {
        Some(reason) => CheckedRayCrossingResult::Undetermined {
            reason,
            origin_contact,
            hits,
            witness,
        },
        None => CheckedRayCrossingResult::Proved {
            origin_contact,
            parity,
            hits,
            witness,
        },
    })
}

struct ExactTriangleCrossing {
    origin_contact: RayOriginContact,
    hit: Option<RayCrossingHit>,
    reason: Option<RayCrossingUndeterminedReason>,
}

struct ExactImplicitCrossing {
    origin_contact: RayOriginContact,
    hits: Vec<RayCrossingHit>,
    reason: Option<RayCrossingUndeterminedReason>,
}

fn cross_exact_implicit(
    ray: &Ray3,
    id: &RayCrossingTargetId,
    surface: &ImplicitSurfaceDeclaration,
    range: &crate::types::PositiveRayParameterRange,
    policy: &RootIsolationPolicy,
) -> Result<ExactImplicitCrossing, GeometryFailure> {
    let polynomial = ray_polynomial(ray, surface)?;
    if polynomial.degree() == 0 {
        return Ok(ExactImplicitCrossing {
            origin_contact: RayOriginContact::None,
            hits: Vec::new(),
            reason: if polynomial.coefficients()[0].is_zero() {
                Some(RayCrossingUndeterminedReason::Coincident)
            } else {
                None
            },
        });
    }

    let bound = isolation_bound(&polynomial)?;
    let domain = Enclosure::new(
        Dyadic::new(
            bound.integer().negated().map_err(map_bigint_failure)?,
            bound.exponent(),
        ),
        bound,
    )
    .map_err(|error| GeometryFailure::CalculusFailure {
        error: CalculusFailure::Interval(error),
    })?;
    let constraint = ray_constraint_positive(surface)?;
    let problem = DerivedRootProblem {
        constraint,
        polynomial: Some(
            polynomial
                .try_clone()
                .map_err(calculus_polynomial_failure)?,
        ),
        domain: Some(domain),
    };
    let candidates = enumerate_root_candidates(&problem, policy)?;
    let isolation = build_root_isolation(&problem, candidates, policy)?;
    let mut hits = Vec::new();
    let mut reason = None;
    for parameter in exact_positive_integer_roots(&polynomial)? {
        if !range.contains_exact(&parameter)? {
            continue;
        }
        let (multiplicity, contribution) = implicit_root_multiplicity(&polynomial, &parameter)?;
        let point = evaluate_ray_point(ray, &parameter)?;
        let normal = implicit_hit_normal(surface, &point)?;
        hits.push(RayCrossingHit::new(
            RayCrossingTargetId(id.0.clone()),
            parameter,
            point,
            RayCrossingLocation::Interior,
            multiplicity,
            contribution,
            normal,
        ));
    }
    for bracket in isolation.brackets() {
        let parameter = match bracket.evidence() {
            RootEvidence::EvenMultiplicityTouch { point, .. }
            | RootEvidence::HigherOrder { point, .. }
            | RootEvidence::SimpleRoot { point, .. } => point.try_clone()?,
            RootEvidence::SignChange { .. } | RootEvidence::Undetermined { .. } => {
                if bracket_overlaps_positive(bracket)?
                    && !hits.iter().any(|hit| {
                        hit.parameter
                            .compare(bracket.lower())
                            .is_ok_and(|ordering| ordering != Ordering::Less)
                            && hit
                                .parameter
                                .compare(bracket.upper())
                                .is_ok_and(|ordering| ordering != Ordering::Greater)
                    })
                {
                    reason.get_or_insert(RayCrossingUndeterminedReason::InvalidMultiplicity);
                }
                continue;
            }
        };
        if parameter.is_zero() || !range.contains_exact(&parameter)? {
            continue;
        }
        if hits.iter().any(|hit| {
            hit.parameter
                .compare(&parameter)
                .is_ok_and(|ordering| ordering == Ordering::Equal)
        }) {
            continue;
        }
        let point = evaluate_ray_point(ray, &parameter)?;
        let (multiplicity, contribution) = match bracket.evidence() {
            RootEvidence::SimpleRoot { .. } => {
                (CrossingMultiplicity::Simple, CrossingContribution::Toggle)
            }
            RootEvidence::HigherOrder { order, .. } if order % 2 == 1 => (
                CrossingMultiplicity::Odd(*order),
                CrossingContribution::Toggle,
            ),
            RootEvidence::EvenMultiplicityTouch { order, .. }
            | RootEvidence::HigherOrder { order, .. } => (
                CrossingMultiplicity::Even(*order),
                CrossingContribution::NoToggle,
            ),
            _ => (
                CrossingMultiplicity::Undetermined,
                CrossingContribution::Undetermined,
            ),
        };
        let normal = implicit_hit_normal(surface, &point)?;
        hits.push(RayCrossingHit::new(
            RayCrossingTargetId(id.0.clone()),
            parameter,
            point,
            RayCrossingLocation::Interior,
            multiplicity,
            contribution,
            normal,
        ));
    }
    if hits.is_empty()
        && matches!(
            isolation.completeness().status(),
            WitnessStatus::Undetermined { .. }
        )
    {
        reason.get_or_insert(RayCrossingUndeterminedReason::InvalidMultiplicity);
    }
    Ok(ExactImplicitCrossing {
        origin_contact: RayOriginContact::None,
        hits,
        reason,
    })
}

fn combine_origin_contact(left: RayOriginContact, right: RayOriginContact) -> RayOriginContact {
    match (left, right) {
        (RayOriginContact::Boundary, _) | (_, RayOriginContact::Boundary) => {
            RayOriginContact::Boundary
        }
        (RayOriginContact::Interior, _) | (_, RayOriginContact::Interior) => {
            RayOriginContact::Interior
        }
        _ => RayOriginContact::None,
    }
}

fn cross_exact_trimmed_surface(
    ray: &Ray3,
    id: &RayCrossingTargetId,
    candidate: &SurfaceCrossingCandidate,
    range: &crate::types::PositiveRayParameterRange,
) -> Result<ExactTriangleCrossing, GeometryFailure> {
    let Some(chart) = affine_chart(candidate.declaration().chart().surface())? else {
        return Ok(ExactTriangleCrossing {
            origin_contact: RayOriginContact::None,
            hit: None,
            reason: Some(RayCrossingUndeterminedReason::InvalidMultiplicity),
        });
    };
    let normal = cross_vector(&chart.du, &chart.dv)?;
    if vector_is_zero(&normal)? {
        return Ok(ExactTriangleCrossing {
            origin_contact: RayOriginContact::None,
            hit: None,
            reason: Some(RayCrossingUndeterminedReason::InvalidMultiplicity),
        });
    }
    let origin_offset = Vector3 {
        x: chart.origin.x.sub(&ray.origin().x)?,
        y: chart.origin.y.sub(&ray.origin().y)?,
        z: chart.origin.z.sub(&ray.origin().z)?,
    };
    let denominator = dot_vector(&normal, ray.direction().vector())?;
    let numerator = dot_vector(&normal, &origin_offset)?;
    if denominator.is_zero() {
        if !numerator.is_zero() {
            return Ok(ExactTriangleCrossing {
                origin_contact: RayOriginContact::None,
                hit: None,
                reason: None,
            });
        }
        let Some((u, v)) = plane_parameters(&chart, ray.origin())? else {
            return Ok(ExactTriangleCrossing {
                origin_contact: RayOriginContact::None,
                hit: None,
                reason: Some(RayCrossingUndeterminedReason::Coincident),
            });
        };
        let (origin_contact, reason) = surface_origin_classification(candidate, &u, &v)?;
        return Ok(ExactTriangleCrossing {
            origin_contact,
            hit: None,
            reason: reason.or(Some(RayCrossingUndeterminedReason::Coincident)),
        });
    }
    let parameter = numerator.div(&denominator)?;
    let point_on_ray = evaluate_ray_point(ray, &parameter)?;
    let Some((u, v)) = plane_parameters(&chart, &point_on_ray)? else {
        return Ok(ExactTriangleCrossing {
            origin_contact: RayOriginContact::None,
            hit: None,
            reason: Some(RayCrossingUndeterminedReason::InvalidMultiplicity),
        });
    };
    let (origin_contact, origin_reason) = if parameter.is_zero() {
        surface_origin_classification(candidate, &u, &v)?
    } else {
        (RayOriginContact::None, None)
    };
    if !range.contains_exact(&parameter)? {
        return Ok(ExactTriangleCrossing {
            origin_contact,
            hit: None,
            reason: origin_reason,
        });
    }
    if !candidate.domain().contains(&u, &v)? {
        return Ok(ExactTriangleCrossing {
            origin_contact,
            hit: None,
            reason: origin_reason,
        });
    }
    let (location, hit_reason) = surface_hit_classification(candidate, &u, &v)?;
    let Some(location) = location else {
        return Ok(ExactTriangleCrossing {
            origin_contact,
            hit: None,
            reason: origin_reason,
        });
    };
    let reason = origin_reason.or(hit_reason);
    let contribution = if reason.is_some() {
        CrossingContribution::Undetermined
    } else {
        CrossingContribution::Toggle
    };
    let witness = evaluate_surface_witness_exact(
        candidate.declaration().chart(),
        candidate.domain(),
        &u,
        &v,
    )?;
    let point = match witness {
        SurfaceEvaluationWitness::Exact { point, .. } => point,
        SurfaceEvaluationWitness::Certified { .. } => {
            return Ok(ExactTriangleCrossing {
                origin_contact,
                hit: None,
                reason: Some(RayCrossingUndeterminedReason::InvalidMultiplicity),
            });
        }
    };
    Ok(ExactTriangleCrossing {
        origin_contact,
        hit: Some(RayCrossingHit::new(
            RayCrossingTargetId(id.0.clone()),
            parameter,
            point,
            location,
            CrossingMultiplicity::Simple,
            contribution,
            ray_normal_from_vector(normal)?,
        )),
        reason,
    })
}

struct AffineSurfaceChart {
    origin: Point3,
    du: Vector3,
    dv: Vector3,
}

fn affine_chart(
    surface: &neco_calculus::RationalSurfacePolynomials,
) -> Result<Option<AffineSurfaceChart>, GeometryFailure> {
    let Some([origin_x, du_x, dv_x]) = affine_form(&surface.x)? else {
        return Ok(None);
    };
    let Some([origin_y, du_y, dv_y]) = affine_form(&surface.y)? else {
        return Ok(None);
    };
    let Some([origin_z, du_z, dv_z]) = affine_form(&surface.z)? else {
        return Ok(None);
    };
    Ok(Some(AffineSurfaceChart {
        origin: Point3 {
            x: origin_x,
            y: origin_y,
            z: origin_z,
        },
        du: Vector3 {
            x: du_x,
            y: du_y,
            z: du_z,
        },
        dv: Vector3 {
            x: dv_x,
            y: dv_y,
            z: dv_z,
        },
    }))
}

fn affine_form(
    polynomial: &SurfacePolynomial,
) -> Result<Option<[RealAlgebraic; 3]>, GeometryFailure> {
    let mut constant = real_zero()?;
    let mut du = real_zero()?;
    let mut dv = real_zero()?;
    for term in polynomial.terms() {
        if term.exponent_u > 1 || term.exponent_v > 1 || term.exponent_u + term.exponent_v > 1 {
            return Ok(None);
        }
        let coefficient = RealAlgebraic::from_reduced_rational(&term.coefficient)
            .map_err(GeometryFailure::AlgebraFailure)?;
        match (term.exponent_u, term.exponent_v) {
            (0, 0) => constant = coefficient,
            (1, 0) => du = coefficient,
            (0, 1) => dv = coefficient,
            _ => return Ok(None),
        }
    }
    Ok(Some([constant, du, dv]))
}

fn plane_parameters(
    chart: &AffineSurfaceChart,
    point: &Point3,
) -> Result<Option<(RealAlgebraic, RealAlgebraic)>, GeometryFailure> {
    let offset = Vector3 {
        x: point.x.sub(&chart.origin.x)?,
        y: point.y.sub(&chart.origin.y)?,
        z: point.z.sub(&chart.origin.z)?,
    };
    let pairs = [
        (&chart.du.x, &chart.dv.x, &offset.x),
        (&chart.du.y, &chart.dv.y, &offset.y),
        (&chart.du.z, &chart.dv.z, &offset.z),
    ];
    for first in 0..3 {
        for second in (first + 1)..3 {
            if let Some(values) = solve_2x2(
                [pairs[first].0, pairs[first].1, pairs[first].2],
                [pairs[second].0, pairs[second].1, pairs[second].2],
            )? {
                return Ok(Some(values));
            }
        }
    }
    Ok(None)
}

fn solve_2x2(
    first: [&RealAlgebraic; 3],
    second: [&RealAlgebraic; 3],
) -> Result<Option<(RealAlgebraic, RealAlgebraic)>, GeometryFailure> {
    let det = first[0].mul(second[1])?.sub(&first[1].mul(second[0])?)?;
    if det.is_zero() {
        return Ok(None);
    }
    let u = first[2]
        .mul(second[1])?
        .sub(&first[1].mul(second[2])?)?
        .div(&det)?;
    let v = first[0]
        .mul(second[2])?
        .sub(&first[2].mul(second[0])?)?
        .div(&det)?;
    Ok(Some((u, v)))
}

fn surface_origin_classification(
    candidate: &SurfaceCrossingCandidate,
    u: &RealAlgebraic,
    v: &RealAlgebraic,
) -> Result<(RayOriginContact, Option<RayCrossingUndeterminedReason>), GeometryFailure> {
    if !candidate.domain().contains(u, v)? {
        return Ok((RayOriginContact::None, None));
    }
    let (location, reason) = surface_hit_classification(candidate, u, v)?;
    Ok(match location {
        Some(RayCrossingLocation::Interior) => (RayOriginContact::Interior, reason),
        Some(_) => (
            RayOriginContact::Boundary,
            reason.or(Some(RayCrossingUndeterminedReason::TrimBoundary)),
        ),
        None => (RayOriginContact::None, None),
    })
}

fn surface_hit_classification(
    candidate: &SurfaceCrossingCandidate,
    u: &RealAlgebraic,
    v: &RealAlgebraic,
) -> Result<
    (
        Option<RayCrossingLocation>,
        Option<RayCrossingUndeterminedReason>,
    ),
    GeometryFailure,
> {
    if let Some(reason) = unresolved_seam_reason(candidate.declaration(), u, v)? {
        return Ok((Some(RayCrossingLocation::Seam), Some(reason)));
    }
    if paired_seam_contains(candidate.declaration(), u, v)? {
        return Ok((Some(RayCrossingLocation::Interior), None));
    }
    match candidate.classify(u, v)? {
        TrimClassification::Outer => Ok((Some(RayCrossingLocation::Interior), None)),
        TrimClassification::Hole | TrimClassification::TrimExterior => Ok((None, None)),
        TrimClassification::TrimBoundary => {
            if hole_boundary_contains(candidate.declaration(), u, v)? {
                Ok((
                    Some(RayCrossingLocation::HoleBoundary),
                    Some(RayCrossingUndeterminedReason::HoleBoundary),
                ))
            } else {
                Ok((
                    Some(RayCrossingLocation::TrimBoundary),
                    Some(RayCrossingUndeterminedReason::TrimBoundary),
                ))
            }
        }
    }
}

fn unresolved_seam_reason(
    declaration: &crate::types::TrimmedSurfaceDeclaration,
    u: &RealAlgebraic,
    v: &RealAlgebraic,
) -> Result<Option<RayCrossingUndeterminedReason>, GeometryFailure> {
    for seam in declaration.seams() {
        if let SeamContact::Undetermined { first, second } = seam {
            if parameter_equals(first.parameter(), u, v)?
                || parameter_equals(second.parameter(), u, v)?
            {
                return Ok(Some(RayCrossingUndeterminedReason::UnresolvedSeam));
            }
        }
    }
    Ok(None)
}

fn paired_seam_contains(
    declaration: &crate::types::TrimmedSurfaceDeclaration,
    u: &RealAlgebraic,
    v: &RealAlgebraic,
) -> Result<bool, GeometryFailure> {
    for seam in declaration.seams() {
        if let SeamContact::Paired(paired) = seam {
            if parameter_equals(paired.first().parameter(), u, v)?
                || parameter_equals(paired.second().parameter(), u, v)?
            {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn hole_boundary_contains(
    declaration: &crate::types::TrimmedSurfaceDeclaration,
    u: &RealAlgebraic,
    v: &RealAlgebraic,
) -> Result<bool, GeometryFailure> {
    for hole in declaration.holes() {
        if point_on_loop(u, v, hole.vertices())? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn parameter_equals(
    point: &SurfaceParameterPoint,
    u: &RealAlgebraic,
    v: &RealAlgebraic,
) -> Result<bool, GeometryFailure> {
    Ok(point.u().compare(u)? == Ordering::Equal && point.v().compare(v)? == Ordering::Equal)
}

fn vector_is_zero(vector: &Vector3) -> Result<bool, GeometryFailure> {
    Ok(vector.x.is_zero() && vector.y.is_zero() && vector.z.is_zero())
}

fn real_zero() -> Result<RealAlgebraic, GeometryFailure> {
    RealAlgebraic::from_integer(BigInt::zero()).map_err(GeometryFailure::AlgebraFailure)
}

fn real_abs(value: &RealAlgebraic) -> Result<RealAlgebraic, GeometryFailure> {
    let zero = real_zero()?;
    if value.compare(&zero)? == Ordering::Less {
        zero.sub(value).map_err(GeometryFailure::AlgebraFailure)
    } else {
        value.try_clone().map_err(GeometryFailure::AlgebraFailure)
    }
}

fn exact_positive_integer_roots(
    polynomial: &Polynomial,
) -> Result<Vec<RealAlgebraic>, GeometryFailure> {
    let mut roots = Vec::new();
    for value in 1..=32 {
        let parameter =
            RealAlgebraic::from_integer(BigInt::try_from(value).map_err(map_bigint_failure)?)?;
        if polynomial
            .evaluate(&parameter)
            .map_err(calculus_polynomial_failure)?
            .is_zero()
        {
            roots.push(parameter);
        }
    }
    Ok(roots)
}

fn implicit_root_multiplicity(
    polynomial: &Polynomial,
    parameter: &RealAlgebraic,
) -> Result<(CrossingMultiplicity, CrossingContribution), GeometryFailure> {
    let mut order = 1_usize;
    let mut current = polynomial
        .try_clone()
        .map_err(calculus_polynomial_failure)?;
    loop {
        let derivative = current
            .derivative(DerivativeOrder::new(1))
            .map_err(calculus_polynomial_failure)?;
        if !derivative
            .evaluate(parameter)
            .map_err(calculus_polynomial_failure)?
            .is_zero()
        {
            break;
        }
        order += 1;
        if order > 16 {
            return Ok((
                CrossingMultiplicity::Undetermined,
                CrossingContribution::Undetermined,
            ));
        }
        current = derivative;
    }
    Ok(match order {
        1 => (CrossingMultiplicity::Simple, CrossingContribution::Toggle),
        order if order % 2 == 1 => (
            CrossingMultiplicity::Odd(order),
            CrossingContribution::Toggle,
        ),
        order => (
            CrossingMultiplicity::Even(order),
            CrossingContribution::NoToggle,
        ),
    })
}

fn bracket_overlaps_positive(bracket: &RootBracket) -> Result<bool, GeometryFailure> {
    Ok(bracket.upper().compare(&real_zero()?)? == Ordering::Greater)
}

fn isolation_bound(polynomial: &Polynomial) -> Result<Dyadic, GeometryFailure> {
    let fallback = Dyadic::new(BigInt::try_from(1024_i32).map_err(map_bigint_failure)?, 0);
    let coefficients = polynomial.coefficients();
    let Some(degree) = coefficients.iter().rposition(|value| !value.is_zero()) else {
        return Ok(fallback);
    };
    let leading = real_abs(&coefficients[degree])?;
    if leading.is_zero() {
        return Ok(fallback);
    }
    let mut max_ratio = real_zero()?;
    for coefficient in &coefficients[..degree] {
        let ratio = real_abs(coefficient)?.div(&leading)?;
        if ratio.compare(&max_ratio)? == Ordering::Greater {
            max_ratio = ratio;
        }
    }
    let bound =
        RealAlgebraic::from_integer(BigInt::one().map_err(map_bigint_failure)?)?.add(&max_ratio)?;
    Ok(real_to_dyadic(&bound).unwrap_or(fallback))
}

fn checked_undetermined_result(
    targets: &CheckedRayCrossingTargetSet,
    reason: RayCrossingUndeterminedReason,
) -> CheckedRayCrossingResult {
    let target_ids = targets
        .targets()
        .iter()
        .map(checked_target_id)
        .map(|id| RayCrossingTargetId(id.0.clone()))
        .collect();
    CheckedRayCrossingResult::Undetermined {
        reason,
        origin_contact: RayOriginContact::None,
        hits: Vec::new(),
        witness: CheckedRayCrossingWitness::new(target_ids, CrossingParity::Undetermined),
    }
}

fn exact_ray_from_checked(ray: &CheckedRay3) -> Option<Ray3> {
    let origin = Point3 {
        x: f64_to_real(ray.origin()[0])?,
        y: f64_to_real(ray.origin()[1])?,
        z: f64_to_real(ray.origin()[2])?,
    };
    let direction = Vector3 {
        x: f64_to_real(ray.direction()[0])?,
        y: f64_to_real(ray.direction()[1])?,
        z: f64_to_real(ray.direction()[2])?,
    };
    Some(Ray3::new(
        origin,
        neco_geometry_foundation::Direction3::try_from_vector(direction).ok()?,
    ))
}

fn exact_triangle_from_checked(triangle: &CheckedTriangleTarget) -> Option<ExactTriangleTarget> {
    ExactTriangleTarget::new(
        RayCrossingTargetId(triangle.id.0.clone()),
        Point3 {
            x: f64_to_real(triangle.vertices[0][0])?,
            y: f64_to_real(triangle.vertices[0][1])?,
            z: f64_to_real(triangle.vertices[0][2])?,
        },
        Point3 {
            x: f64_to_real(triangle.vertices[1][0])?,
            y: f64_to_real(triangle.vertices[1][1])?,
            z: f64_to_real(triangle.vertices[1][2])?,
        },
        Point3 {
            x: f64_to_real(triangle.vertices[2][0])?,
            y: f64_to_real(triangle.vertices[2][1])?,
            z: f64_to_real(triangle.vertices[2][2])?,
        },
    )
    .ok()
}

fn f64_to_real(value: f64) -> Option<RealAlgebraic> {
    if !value.is_finite() {
        return None;
    }
    let integer = i32::try_from(value as i64).ok()?;
    if value != f64::from(integer) {
        return None;
    }
    RealAlgebraic::from_integer(BigInt::try_from(integer).ok()?).ok()
}

fn checked_hit_from_exact(
    hit: RayCrossingHit,
    owner: OwnerId,
    requirement: ExactExpressionRequirement,
    source: NecoImplementationSource,
) -> Result<CheckedRayCrossingHit, GeometryFailure> {
    Ok(CheckedRayCrossingHit::from_exact_hit(
        RayCrossingTargetId(hit.target_id.0.clone()),
        real_to_enclosure(hit.parameter())?,
        CoordinateEnclosure3 {
            x: real_to_enclosure(&hit.point().x)?,
            y: real_to_enclosure(&hit.point().y)?,
            z: real_to_enclosure(&hit.point().z)?,
        },
        CheckedHitGeometry {
            location: hit.location(),
            multiplicity: hit.multiplicity(),
            contribution: hit.contribution(),
            owner,
            requirement,
            source,
            normal: hit.normal,
        },
    ))
}

fn checked_surface_hit(
    hit: RayCrossingHit,
    surface: &CheckedTrimmedParametricSurfaceTarget,
) -> Result<CheckedRayCrossingHit, GeometryFailure> {
    let parameter = real_to_enclosure(hit.parameter())?;
    let point = match recover_checked_surface_point(&hit, surface)? {
        Some(point) => point,
        None => CoordinateEnclosure3 {
            x: real_to_enclosure(&hit.point().x)?,
            y: real_to_enclosure(&hit.point().y)?,
            z: real_to_enclosure(&hit.point().z)?,
        },
    };
    Ok(CheckedRayCrossingHit::from_exact_hit(
        RayCrossingTargetId(hit.target_id.0.clone()),
        parameter,
        point,
        CheckedHitGeometry {
            location: hit.location(),
            multiplicity: hit.multiplicity(),
            contribution: hit.contribution(),
            owner: surface.owner(),
            requirement: clone_requirement(surface.requirement())?,
            source: surface.source(),
            normal: hit.normal,
        },
    ))
}

fn recover_checked_surface_point(
    hit: &RayCrossingHit,
    surface: &CheckedTrimmedParametricSurfaceTarget,
) -> Result<Option<CoordinateEnclosure3>, GeometryFailure> {
    let Some(chart) = affine_chart(surface.candidate().declaration().chart().surface())? else {
        return Ok(None);
    };
    let Some((u, v)) = plane_parameters(&chart, hit.point())? else {
        return Ok(None);
    };
    let u_enclosure = real_to_enclosure(&u)?;
    let v_enclosure = real_to_enclosure(&v)?;
    match evaluate_surface_witness_checked(
        surface.candidate().declaration().chart(),
        surface.candidate().domain(),
        &u_enclosure,
        &v_enclosure,
        surface.owner(),
        clone_requirement(surface.requirement())?,
        surface.source(),
    ) {
        Ok(SurfaceEvaluationWitness::Certified { x, y, z, .. }) => {
            Ok(Some(CoordinateEnclosure3 { x, y, z: *z }))
        }
        _ => Ok(None),
    }
}

fn real_to_enclosure(value: &RealAlgebraic) -> Result<Enclosure, GeometryFailure> {
    let dyadic = real_to_dyadic(value)?;
    Enclosure::new(dyadic.try_clone().map_err(map_bigint_failure)?, dyadic).map_err(|error| {
        GeometryFailure::CalculusFailure {
            error: CalculusFailure::Interval(error),
        }
    })
}

fn compare_enclosures(left: &Enclosure, right: &Enclosure) -> Result<Ordering, GeometryFailure> {
    dyadic_to_real(left.lower())?
        .compare(&dyadic_to_real(right.lower())?)
        .map_err(GeometryFailure::AlgebraFailure)
}

fn enclosures_overlap(left: &Enclosure, right: &Enclosure) -> Result<bool, GeometryFailure> {
    let left_lower = dyadic_to_real(left.lower())?;
    let left_upper = dyadic_to_real(left.upper())?;
    let right_lower = dyadic_to_real(right.lower())?;
    let right_upper = dyadic_to_real(right.upper())?;
    Ok(left_lower.compare(&right_upper)? != Ordering::Greater
        && right_lower.compare(&left_upper)? != Ordering::Greater)
}

fn cross_exact_triangle(
    ray: &Ray3,
    triangle: &ExactTriangleTarget,
    range: &crate::types::PositiveRayParameterRange,
) -> Result<ExactTriangleCrossing, GeometryFailure> {
    let ab = subtract_point(&triangle.b, &triangle.a)?;
    let ac = subtract_point(&triangle.c, &triangle.a)?;
    let normal = cross_vector(&ab, &ac)?;
    let origin_offset = subtract_point(&triangle.a, ray.origin())?;
    let denominator = dot_vector(&normal, ray.direction().vector())?;
    let numerator = dot_vector(&normal, &origin_offset)?;
    if denominator.is_zero() {
        return Ok(ExactTriangleCrossing {
            origin_contact: RayOriginContact::None,
            hit: None,
            reason: if numerator.is_zero() {
                Some(RayCrossingUndeterminedReason::CoplanarOverlap)
            } else {
                None
            },
        });
    }
    let parameter = numerator.div(&denominator)?;
    let point = evaluate_ray_point(ray, &parameter)?;
    let location = triangle_location(&point, triangle, &normal)?;
    let origin_location = if numerator.is_zero() {
        triangle_location(ray.origin(), triangle, &normal)?
    } else {
        None
    };
    let origin_contact = match origin_location {
        Some(RayCrossingLocation::Interior) => RayOriginContact::Interior,
        Some(_) => RayOriginContact::Boundary,
        None => RayOriginContact::None,
    };
    if !range.contains_exact(&parameter)? {
        return Ok(ExactTriangleCrossing {
            origin_contact,
            hit: None,
            reason: match origin_location {
                Some(RayCrossingLocation::Edge) => Some(RayCrossingUndeterminedReason::Edge),
                Some(RayCrossingLocation::Vertex) => Some(RayCrossingUndeterminedReason::Vertex),
                _ => None,
            },
        });
    }
    let Some(location) = location else {
        return Ok(ExactTriangleCrossing {
            origin_contact,
            hit: None,
            reason: match origin_location {
                Some(RayCrossingLocation::Edge) => Some(RayCrossingUndeterminedReason::Edge),
                Some(RayCrossingLocation::Vertex) => Some(RayCrossingUndeterminedReason::Vertex),
                _ => None,
            },
        });
    };
    let reason = match location {
        RayCrossingLocation::Interior => None,
        RayCrossingLocation::Edge => Some(RayCrossingUndeterminedReason::Edge),
        RayCrossingLocation::Vertex => Some(RayCrossingUndeterminedReason::Vertex),
        _ => Some(RayCrossingUndeterminedReason::InvalidMultiplicity),
    };
    let contribution = if reason.is_some() {
        CrossingContribution::Undetermined
    } else {
        CrossingContribution::Toggle
    };
    Ok(ExactTriangleCrossing {
        origin_contact,
        hit: Some(RayCrossingHit::new(
            RayCrossingTargetId(triangle.id.0.clone()),
            parameter,
            point,
            location,
            CrossingMultiplicity::Simple,
            contribution,
            ray_normal_from_vector(normal)?,
        )),
        reason,
    })
}

fn ray_normal_from_vector(vector: Vector3) -> Result<RayNormal, GeometryFailure> {
    match Direction3::try_from_vector(vector) {
        Ok(direction) => Ok(RayNormal::Regular(direction)),
        Err(neco_geometry_foundation::GeometryFailure::ZeroDirection) => {
            Ok(RayNormal::Undetermined)
        }
        Err(error) => Err(GeometryFailure::FoundationFailure { error }),
    }
}

fn implicit_hit_normal(
    surface: &ImplicitSurfaceDeclaration,
    point: &Point3,
) -> Result<RayNormal, GeometryFailure> {
    ray_normal_from_vector(implicit_gradient(surface, point)?)
}

fn implicit_gradient(
    surface: &ImplicitSurfaceDeclaration,
    point: &Point3,
) -> Result<Vector3, GeometryFailure> {
    let mut components = [
        RealAlgebraic::from_integer(BigInt::zero())?,
        RealAlgebraic::from_integer(BigInt::zero())?,
        RealAlgebraic::from_integer(BigInt::zero())?,
    ];
    for term in surface.equation().terms() {
        let values = [&point.x, &point.y, &point.z];
        let exponents = [term.exponent_x(), term.exponent_y(), term.exponent_z()];
        for component in 0..3 {
            if exponents[component] == 0 {
                continue;
            }
            let mut value = term.coefficient().try_clone()?;
            value = value.mul(&RealAlgebraic::from_integer(
                BigInt::try_from(exponents[component]).map_err(map_bigint_failure)?,
            )?)?;
            for axis in 0..3 {
                let power = if axis == component {
                    exponents[axis] - 1
                } else {
                    exponents[axis]
                };
                for _ in 0..power {
                    value = value.mul(values[axis])?;
                }
            }
            components[component] = components[component].add(&value)?;
        }
    }
    Ok(Vector3 {
        x: components[0].try_clone()?,
        y: components[1].try_clone()?,
        z: components[2].try_clone()?,
    })
}

fn subtract_point(left: &Point3, right: &Point3) -> Result<Vector3, GeometryFailure> {
    Ok(Vector3 {
        x: left.x.sub(&right.x)?,
        y: left.y.sub(&right.y)?,
        z: left.z.sub(&right.z)?,
    })
}

fn cross_vector(left: &Vector3, right: &Vector3) -> Result<Vector3, GeometryFailure> {
    Ok(Vector3 {
        x: left.y.mul(&right.z)?.sub(&left.z.mul(&right.y)?)?,
        y: left.z.mul(&right.x)?.sub(&left.x.mul(&right.z)?)?,
        z: left.x.mul(&right.y)?.sub(&left.y.mul(&right.x)?)?,
    })
}

fn dot_vector(left: &Vector3, right: &Vector3) -> Result<RealAlgebraic, GeometryFailure> {
    Ok(left
        .x
        .mul(&right.x)?
        .add(&left.y.mul(&right.y)?)?
        .add(&left.z.mul(&right.z)?)?)
}

fn triangle_location(
    point: &Point3,
    triangle: &ExactTriangleTarget,
    normal: &Vector3,
) -> Result<Option<RayCrossingLocation>, GeometryFailure> {
    let pa = subtract_point(&triangle.a, point)?;
    let pb = subtract_point(&triangle.b, point)?;
    let pc = subtract_point(&triangle.c, point)?;
    let a = dot_vector(normal, &cross_vector(&pb, &pc)?)?;
    let b = dot_vector(normal, &cross_vector(&pc, &pa)?)?;
    let c = dot_vector(normal, &cross_vector(&pa, &pb)?)?;
    let signs = [a, b, c];
    let zero = real_zero()?;
    let positive = signs
        .iter()
        .filter(|value| value.compare(&zero).is_ok_and(|v| v == Ordering::Greater))
        .count();
    let negative = signs
        .iter()
        .filter(|value| value.compare(&zero).is_ok_and(|v| v == Ordering::Less))
        .count();
    if positive != 0 && negative != 0 {
        return Ok(None);
    }
    let zeroes = signs.iter().filter(|value| value.is_zero()).count();
    Ok(Some(match zeroes {
        0 => RayCrossingLocation::Interior,
        1 => RayCrossingLocation::Edge,
        _ => RayCrossingLocation::Vertex,
    }))
}

fn ray_polynomial(
    ray: &Ray3,
    surface: &ImplicitSurfaceDeclaration,
) -> Result<Polynomial, GeometryFailure> {
    let mut result = Polynomial::zero().map_err(calculus_polynomial_failure)?;
    for term in surface.equation().terms() {
        let x = affine_polynomial(
            &ray.origin().x,
            &ray.direction().vector().x,
            term.exponent_x(),
        )?;
        let y = affine_polynomial(
            &ray.origin().y,
            &ray.direction().vector().y,
            term.exponent_y(),
        )?;
        let z = affine_polynomial(
            &ray.origin().z,
            &ray.direction().vector().z,
            term.exponent_z(),
        )?;
        let term_polynomial = polynomial_mul(&polynomial_mul(&x, &y)?, &z)?;
        let coefficient = term.coefficient().try_clone()?;
        let scaled = term_polynomial
            .coefficients()
            .iter()
            .map(|value| {
                value
                    .mul(&coefficient)
                    .map_err(GeometryFailure::AlgebraFailure)
            })
            .collect::<Result<Vec<_>, _>>()?;
        result = polynomial_add(
            &result,
            &Polynomial::from_coefficients(scaled).map_err(calculus_polynomial_failure)?,
        )?;
    }
    Ok(result)
}

fn polynomial_add(left: &Polynomial, right: &Polynomial) -> Result<Polynomial, GeometryFailure> {
    let length = left.coefficients().len().max(right.coefficients().len());
    let zero = RealAlgebraic::from_integer(BigInt::zero())?;
    let mut coefficients = Vec::with_capacity(length);
    for index in 0..length {
        let lhs = left.coefficients().get(index).unwrap_or(&zero);
        let rhs = right.coefficients().get(index).unwrap_or(&zero);
        coefficients.push(lhs.add(rhs)?);
    }
    Polynomial::from_coefficients(coefficients).map_err(calculus_polynomial_failure)
}

fn polynomial_mul(left: &Polynomial, right: &Polynomial) -> Result<Polynomial, GeometryFailure> {
    let mut coefficients = Vec::with_capacity(left.degree() + right.degree() + 1);
    for _ in 0..=left.degree() + right.degree() {
        coefficients.push(RealAlgebraic::from_integer(BigInt::zero())?);
    }
    for (left_index, lhs) in left.coefficients().iter().enumerate() {
        for (right_index, rhs) in right.coefficients().iter().enumerate() {
            let product = lhs.mul(rhs)?;
            coefficients[left_index + right_index] =
                coefficients[left_index + right_index].add(&product)?;
        }
    }
    Polynomial::from_coefficients(coefficients).map_err(calculus_polynomial_failure)
}

fn affine_polynomial(
    origin: &RealAlgebraic,
    direction: &RealAlgebraic,
    exponent: u32,
) -> Result<Polynomial, GeometryFailure> {
    let mut result = Polynomial::constant(RealAlgebraic::from_integer(
        BigInt::one().map_err(map_bigint_failure)?,
    )?)
    .map_err(calculus_polynomial_failure)?;
    let affine =
        Polynomial::from_coefficients(alloc::vec![origin.try_clone()?, direction.try_clone()?])
            .map_err(calculus_polynomial_failure)?;
    for _ in 0..exponent {
        result = polynomial_mul(&result, &affine)?;
    }
    Ok(result)
}

fn ray_constraint_positive(
    surface: &ImplicitSurfaceDeclaration,
) -> Result<IntersectionConstraint, GeometryFailure> {
    let declaration = ShapeDeclaration::ImplicitSurface(ImplicitSurfaceDeclaration::new(
        clone_polynomial3(surface.equation())?,
    ));
    let left = clone_declaration(&declaration)?;
    Ok(IntersectionConstraint::new(
        IntersectionOperand::Declaration(left),
        IntersectionOperand::Declaration(declaration),
        ResidualFunction::new(String::from("ray implicit"), String::from("0")),
        IntersectionDomain::new(
            String::from("ray-parameter"),
            String::from("0 < t < +infinity"),
            String::from("ray"),
        ),
        SupportRelation::Finite {
            evidence: SupportEvidence::ImplicitSupportPair,
        },
        RankEvidence::Sufficient { rank: 0 },
    ))
}

fn evaluate_ray_point(ray: &Ray3, parameter: &RealAlgebraic) -> Result<Point3, GeometryFailure> {
    Ok(Point3 {
        x: ray
            .origin()
            .x
            .add(&ray.direction().vector().x.mul(parameter)?)?,
        y: ray
            .origin()
            .y
            .add(&ray.direction().vector().y.mul(parameter)?)?,
        z: ray
            .origin()
            .z
            .add(&ray.direction().vector().z.mul(parameter)?)?,
    })
}

pub fn construct_intersection(
    request: IntersectionConstructionRequest,
) -> Result<IntersectionSet, GeometryFailure> {
    let body = request.into_body();
    let IntersectionConstructionRequestBody {
        left_operand,
        right_operand,
        root_isolation_policy,
    } = body;
    validate_policy(&root_isolation_policy)?;

    let left_descriptor = resolve_operand(&left_operand)?;
    let right_descriptor = resolve_operand(&right_operand)?;
    let problem = derive_intersection_constraint(
        left_operand,
        right_operand,
        left_descriptor,
        right_descriptor,
    )?;
    let candidates = enumerate_root_candidates(&problem, &root_isolation_policy)?;
    let isolation = build_root_isolation(&problem, candidates, &root_isolation_policy)?;
    classify_constraint(problem, &isolation)
}

fn validate_policy(policy: &RootIsolationPolicy) -> Result<(), GeometryFailure> {
    if policy.max_iterations == 0 || policy.max_cells == 0 {
        return Err(GeometryFailure::RootIsolationPolicyInvalid);
    }
    if policy.max_iterations > MAX_ROOT_PARTITION_DEPTH {
        return Err(GeometryFailure::RootIsolationResourceLimit {
            requested_depth: policy.max_iterations,
            maximum_depth: MAX_ROOT_PARTITION_DEPTH,
        });
    }
    if policy.max_cells > MAX_ROOT_PARTITION_CELLS {
        return Err(GeometryFailure::RootPartitionCellLimit {
            requested_cells: policy.max_cells,
            maximum_cells: MAX_ROOT_PARTITION_CELLS,
        });
    }
    Ok(())
}

fn resolve_operand(operand: &IntersectionOperand) -> Result<OperandDescriptor, GeometryFailure> {
    match operand {
        IntersectionOperand::Declaration(declaration) => {
            let evaluation = evaluate_declaration_capability(declaration)?;
            descriptor_from_declaration(declaration, evaluation)
        }
        IntersectionOperand::Evaluated(evaluation) => descriptor_from_evaluation(evaluation),
    }
}

fn derive_intersection_constraint(
    left_operand: IntersectionOperand,
    right_operand: IntersectionOperand,
    left: OperandDescriptor,
    right: OperandDescriptor,
) -> Result<DerivedRootProblem, GeometryFailure> {
    let residual_function =
        ResidualFunction::new(left.expression.clone(), right.expression.clone());
    let support = derive_support_relation(&left, &right);
    let rank = derive_rank_evidence(&left, &right, &support);
    let domain = IntersectionDomain::new(
        format!("{} x {}", left.chart_id, right.chart_id),
        format!("{} ∩ {}", left.domain, right.domain),
        format!("{} <> {}", left.support_id, right.support_id),
    );
    let polynomial = derive_residual_polynomial(&left, &right)?;
    let numeric_domain = derive_numeric_domain(&left, &right)?;
    let constraint = IntersectionConstraint::new(
        left_operand,
        right_operand,
        residual_function,
        domain,
        support,
        rank,
    );
    Ok(DerivedRootProblem {
        constraint,
        polynomial,
        domain: numeric_domain,
    })
}

fn enumerate_root_candidates(
    problem: &DerivedRootProblem,
    policy: &RootIsolationPolicy,
) -> Result<RootCandidateSet, GeometryFailure> {
    let Some(polynomial) = problem.polynomial.as_ref() else {
        return Ok(RootEnumeration::incomplete(problem.domain.as_ref()));
    };
    let Some(domain) = problem.domain.as_ref() else {
        return Ok(RootEnumeration::incomplete(None));
    };

    if polynomial.coefficients().iter().all(RealAlgebraic::is_zero)
        && matches!(
            problem.constraint.support_relation(),
            SupportRelation::Coincident { .. }
        )
    {
        return Ok(RootEnumeration {
            candidates: Vec::new(),
            cells: vec![PartitionCell {
                id: 0,
                depth: 0,
                enclosure: clone_enclosure(domain)?,
                disposition: CellDisposition::Coincident,
            }],
        });
    }

    let source = problem.constraint.residual_function().combined_expression();
    let order_bound = DerivativeOrder::new(policy.max_iterations.max(2));
    let derivative = polynomial
        .derivative(DerivativeOrder::new(1))
        .map_err(calculus_polynomial_failure)?;
    let mut pending = vec![PendingPartitionCell {
        id: 0,
        depth: 0,
        enclosure: clone_enclosure(domain)?,
        suppressed_exact_endpoints: Vec::new(),
    }];
    let mut next_cell_id = 1;
    let mut cells = Vec::new();
    let mut candidates = Vec::new();

    while let Some(PendingPartitionCell {
        id: cell_id,
        depth,
        enclosure,
        suppressed_exact_endpoints,
    }) = pending.pop()
    {
        match assess_partition_cell(
            polynomial,
            &derivative,
            &enclosure,
            cell_id,
            &order_bound,
            source,
            &suppressed_exact_endpoints,
        )? {
            CellAssessment::ExactTouch { candidate, pivot } => {
                cells.push(PartitionCell {
                    id: cell_id,
                    depth,
                    enclosure: clone_enclosure(&enclosure)?,
                    disposition: CellDisposition::Candidate,
                });
                let pivot_exhausts_polynomial =
                    exact_touch_exhausts_polynomial(&candidate, polynomial);
                let pivot_is_unresolved =
                    matches!(&candidate.bracket, RootCandidateBracket::Unresolved { .. });
                candidates.push(*candidate);
                if pivot_exhausts_polynomial || pivot_is_unresolved {
                    continue;
                }

                let has_lower_child = pivot != *enclosure.lower();
                let has_upper_child = pivot != *enclosure.upper();
                ensure_partition_cell_capacity(
                    cells.len(),
                    pending.len(),
                    usize::from(has_lower_child) + usize::from(has_upper_child),
                    policy.max_cells,
                )?;
                if has_lower_child {
                    let lower = Enclosure::new(
                        enclosure.lower().try_clone().map_err(map_bigint_failure)?,
                        pivot.try_clone().map_err(map_bigint_failure)?,
                    )
                    .map_err(|error| GeometryFailure::CalculusFailure {
                        error: CalculusFailure::Interval(error),
                    })?;
                    let lower_id = take_partition_cell_id(&mut next_cell_id)?;
                    pending.push(PendingPartitionCell {
                        id: lower_id,
                        depth: depth + 1,
                        suppressed_exact_endpoints: suppressed_endpoints_for_child(
                            &suppressed_exact_endpoints,
                            lower.lower(),
                            lower.upper(),
                            &pivot,
                        )?,
                        enclosure: lower,
                    });
                }
                if has_upper_child {
                    let upper = Enclosure::new(
                        pivot.try_clone().map_err(map_bigint_failure)?,
                        enclosure.upper().try_clone().map_err(map_bigint_failure)?,
                    )
                    .map_err(|error| GeometryFailure::CalculusFailure {
                        error: CalculusFailure::Interval(error),
                    })?;
                    let upper_id = take_partition_cell_id(&mut next_cell_id)?;
                    pending.push(PendingPartitionCell {
                        id: upper_id,
                        depth: depth + 1,
                        suppressed_exact_endpoints: suppressed_endpoints_for_child(
                            &suppressed_exact_endpoints,
                            upper.lower(),
                            upper.upper(),
                            &pivot,
                        )?,
                        enclosure: upper,
                    });
                }
            }
            CellAssessment::Candidate(cell_candidates) => {
                cells.push(PartitionCell {
                    id: cell_id,
                    depth,
                    enclosure,
                    disposition: CellDisposition::Candidate,
                });
                candidates.extend(cell_candidates);
            }
            CellAssessment::RootFree => cells.push(PartitionCell {
                id: cell_id,
                depth,
                enclosure,
                disposition: CellDisposition::RootFree,
            }),
            CellAssessment::Unresolved(reason) if depth < policy.max_iterations => {
                let midpoint =
                    enclosure
                        .midpoint()
                        .map_err(|error| GeometryFailure::CalculusFailure {
                            error: CalculusFailure::Interval(error),
                        })?;
                if midpoint == *enclosure.lower() || midpoint == *enclosure.upper() {
                    cells.push(PartitionCell {
                        id: cell_id,
                        depth,
                        enclosure,
                        disposition: CellDisposition::Unresolved { reason },
                    });
                    continue;
                }
                ensure_partition_cell_capacity(cells.len(), pending.len(), 2, policy.max_cells)?;
                let lower_id = take_partition_cell_id(&mut next_cell_id)?;
                let upper_id = take_partition_cell_id(&mut next_cell_id)?;
                let lower = Enclosure::new(
                    enclosure.lower().try_clone().map_err(map_bigint_failure)?,
                    midpoint.try_clone().map_err(map_bigint_failure)?,
                )
                .map_err(|error| GeometryFailure::CalculusFailure {
                    error: CalculusFailure::Interval(error),
                })?;
                let upper = Enclosure::new(
                    midpoint,
                    enclosure.upper().try_clone().map_err(map_bigint_failure)?,
                )
                .map_err(|error| GeometryFailure::CalculusFailure {
                    error: CalculusFailure::Interval(error),
                })?;
                let lower_suppressed = inherited_suppressed_endpoints(
                    &suppressed_exact_endpoints,
                    lower.lower(),
                    lower.upper(),
                )?;
                let upper_suppressed = inherited_suppressed_endpoints(
                    &suppressed_exact_endpoints,
                    upper.lower(),
                    upper.upper(),
                )?;
                pending.push(PendingPartitionCell {
                    id: upper_id,
                    depth: depth + 1,
                    enclosure: upper,
                    suppressed_exact_endpoints: upper_suppressed,
                });
                pending.push(PendingPartitionCell {
                    id: lower_id,
                    depth: depth + 1,
                    enclosure: lower,
                    suppressed_exact_endpoints: lower_suppressed,
                });
            }
            CellAssessment::Unresolved(reason) => cells.push(PartitionCell {
                id: cell_id,
                depth,
                enclosure,
                disposition: CellDisposition::Unresolved { reason },
            }),
        }
    }

    Ok(RootEnumeration { candidates, cells })
}

fn exact_touch_exhausts_polynomial(candidate: &RootCandidate, polynomial: &Polynomial) -> bool {
    let order = match candidate.evidence {
        RootEvidence::EvenMultiplicityTouch { order, .. }
        | RootEvidence::HigherOrder { order, .. } => order,
        RootEvidence::SignChange { .. }
        | RootEvidence::SimpleRoot { .. }
        | RootEvidence::Undetermined { .. } => return false,
    };
    order >= polynomial.coefficients().len().saturating_sub(1)
}

fn inherited_suppressed_endpoints(
    inherited: &[Dyadic],
    lower: &Dyadic,
    upper: &Dyadic,
) -> Result<Vec<Dyadic>, GeometryFailure> {
    inherited
        .iter()
        .filter(|point| *point == lower || *point == upper)
        .map(|point| point.try_clone().map_err(map_bigint_failure))
        .collect()
}

fn suppressed_endpoints_for_child(
    inherited: &[Dyadic],
    lower: &Dyadic,
    upper: &Dyadic,
    pivot: &Dyadic,
) -> Result<Vec<Dyadic>, GeometryFailure> {
    let mut endpoints = inherited_suppressed_endpoints(inherited, lower, upper)?;
    if !endpoints.iter().any(|point| point == pivot) {
        endpoints.push(pivot.try_clone().map_err(map_bigint_failure)?);
    }
    Ok(endpoints)
}

fn assess_partition_cell(
    polynomial: &Polynomial,
    derivative: &Polynomial,
    enclosure: &Enclosure,
    cell_id: PartitionCellId,
    order_bound: &DerivativeOrder,
    source: &str,
    suppressed_exact_endpoints: &[Dyadic],
) -> Result<CellAssessment, GeometryFailure> {
    let lower_sign = polynomial
        .evaluate(&dyadic_to_real(enclosure.lower())?)
        .map_err(calculus_polynomial_failure)?
        .sign()
        .map_err(GeometryFailure::AlgebraFailure)?;
    let upper_sign = polynomial
        .evaluate(&dyadic_to_real(enclosure.upper())?)
        .map_err(calculus_polynomial_failure)?
        .sign()
        .map_err(GeometryFailure::AlgebraFailure)?;
    let midpoint = enclosure
        .midpoint()
        .map_err(|error| GeometryFailure::CalculusFailure {
            error: CalculusFailure::Interval(error),
        })?;
    let midpoint_sign = polynomial
        .evaluate(&dyadic_to_real(&midpoint)?)
        .map_err(calculus_polynomial_failure)?
        .sign()
        .map_err(GeometryFailure::AlgebraFailure)?;

    let mut exact_points = Vec::new();
    if lower_sign == Sign::Zero
        && !suppressed_exact_endpoints
            .iter()
            .any(|point| point == enclosure.lower())
    {
        exact_points.push(enclosure.lower().try_clone().map_err(map_bigint_failure)?);
    }
    if upper_sign == Sign::Zero
        && enclosure.upper() != enclosure.lower()
        && !suppressed_exact_endpoints
            .iter()
            .any(|point| point == enclosure.upper())
    {
        exact_points.push(enclosure.upper().try_clone().map_err(map_bigint_failure)?);
    }
    if midpoint_sign == Sign::Zero && !exact_points.iter().any(|point| point == &midpoint) {
        exact_points.push(midpoint.try_clone().map_err(map_bigint_failure)?);
    }

    if exact_points.len() > 1 {
        return Ok(CellAssessment::Unresolved(
            PartitionUnresolvedReason::MultipleExactDyadicRoots,
        ));
    }
    if let Some(pivot) = exact_points.into_iter().next() {
        let candidate = exact_touch_candidate(
            polynomial,
            enclosure,
            pivot.try_clone().map_err(map_bigint_failure)?,
            cell_id,
            order_bound,
            source,
        )?;
        return Ok(CellAssessment::ExactTouch {
            candidate: Box::new(candidate),
            pivot,
        });
    }

    if opposite_nonzero(lower_sign, upper_sign) {
        let derivative_enclosure = derivative
            .evaluate_enclosure(enclosure, ROOT_INTERVAL_PRECISION_BITS)
            .map_err(calculus_polynomial_failure)?;
        if derivative_enclosure.contains_zero() {
            return Ok(CellAssessment::Unresolved(
                PartitionUnresolvedReason::SignChangeUniqueness,
            ));
        }
        let calculus_bracket = CalculusRootBracket::sign_change(
            polynomial
                .try_clone()
                .map_err(calculus_polynomial_failure)?,
            clone_enclosure(enclosure)?,
        )
        .map_err(|error| GeometryFailure::CalculusFailure {
            error: CalculusFailure::Root(error),
        })?;
        return Ok(CellAssessment::Candidate(vec![RootCandidate {
            bracket: RootCandidateBracket::Certified(calculus_bracket),
            evidence: RootEvidence::SignChange {
                lower_sign: root_sign(lower_sign),
                upper_sign: root_sign(upper_sign),
                derivative_enclosure,
                source: ResidualSourceId::new(source),
            },
            source_cell: cell_id,
        }]));
    }

    let value_enclosure = polynomial
        .evaluate_enclosure(enclosure, ROOT_INTERVAL_PRECISION_BITS)
        .map_err(calculus_polynomial_failure)?;
    if !value_enclosure.contains_zero() {
        return Ok(CellAssessment::RootFree);
    }
    Ok(CellAssessment::Unresolved(
        PartitionUnresolvedReason::UncertifiedRoot,
    ))
}

fn exact_touch_candidate(
    polynomial: &Polynomial,
    enclosure: &Enclosure,
    point: Dyadic,
    cell_id: PartitionCellId,
    order_bound: &DerivativeOrder,
    source: &str,
) -> Result<RootCandidate, GeometryFailure> {
    let multiplicity = match neco_calculus::classify_multiplicity(polynomial, &point, *order_bound)
    {
        Ok(multiplicity) => multiplicity,
        Err(error) if root_failure_is_undetermined(&error) => {
            return Ok(RootCandidate {
                bracket: RootCandidateBracket::Unresolved {
                    enclosure: clone_enclosure(enclosure)?,
                },
                evidence: RootEvidence::Undetermined {
                    reason: RootUndeterminedReason::ExactMultiplicityUnavailable,
                    source: ResidualSourceId::new(source),
                },
                source_cell: cell_id,
            });
        }
        Err(error) => {
            return Err(GeometryFailure::CalculusFailure {
                error: CalculusFailure::Root(error),
            });
        }
    };
    let point = dyadic_to_real(&point)?;
    let order = match multiplicity {
        RootMultiplicity::Multiple(value) => value.get(),
        RootMultiplicity::Simple => 1,
    };
    let evidence = if order == 1 {
        RootEvidence::SimpleRoot {
            point: point.try_clone()?,
            source: ResidualSourceId::new(source),
        }
    } else if order > 2 {
        RootEvidence::HigherOrder {
            order,
            point: point.try_clone()?,
            source: ResidualSourceId::new(source),
        }
    } else {
        RootEvidence::EvenMultiplicityTouch {
            order,
            point: point.try_clone()?,
            source: ResidualSourceId::new(source),
        }
    };
    Ok(RootCandidate {
        bracket: RootCandidateBracket::Exact { parameter: point },
        evidence,
        source_cell: cell_id,
    })
}

fn root_failure_is_undetermined(error: &neco_calculus::RootFailure) -> bool {
    matches!(
        error,
        neco_calculus::RootFailure::NoRootAtPoint
            | neco_calculus::RootFailure::NoEvenMultiplicityTouch
            | neco_calculus::RootFailure::MultiplicityUndetermined
            | neco_calculus::RootFailure::PointOutsideEnclosure
    )
}

fn build_root_isolation(
    problem: &DerivedRootProblem,
    candidates: RootCandidateSet,
    policy: &RootIsolationPolicy,
) -> Result<RootIsolation, GeometryFailure> {
    let RootEnumeration { candidates, cells } = candidates;
    let mut candidate_brackets = Vec::new();
    let mut unresolved_candidate = false;
    for candidate in candidates {
        let source_cell = candidate.source_cell;
        let bracket = match candidate.bracket {
            RootCandidateBracket::Exact { parameter } => {
                RootBracket::from_exact(parameter, candidate.evidence, policy)?
            }
            RootCandidateBracket::Certified(calculus_bracket) => {
                let calculus_isolation = refine_root(&calculus_bracket, policy)?;
                RootBracket::from_calculus(
                    &calculus_bracket,
                    &calculus_isolation,
                    candidate.evidence,
                )?
            }
            RootCandidateBracket::Unresolved { enclosure } => {
                unresolved_candidate = true;
                RootBracket::from_unresolved(&enclosure, candidate.evidence, policy)?
            }
        };
        candidate_brackets.push((source_cell, bracket));
    }
    let candidate_cell_ids: Vec<_> = cells
        .iter()
        .filter(|cell| matches!(cell.disposition, CellDisposition::Candidate))
        .map(|cell| cell.id)
        .collect();
    let candidate_source_cells: Vec<_> = candidate_brackets
        .iter()
        .map(|(source_cell, _)| *source_cell)
        .collect();
    let candidate_brackets_match = candidate_cell_ids.len() == candidate_brackets.len()
        && candidate_cell_ids.iter().all(|cell_id| {
            candidate_source_cells
                .iter()
                .filter(|source_cell| **source_cell == *cell_id)
                .count()
                == 1
        })
        && candidate_source_cells.iter().all(|source_cell| {
            candidate_cell_ids
                .iter()
                .any(|cell_id| cell_id == source_cell)
        });
    let duplicate_candidate = candidate_source_cells
        .iter()
        .enumerate()
        .any(|(index, cell_id)| {
            candidate_source_cells[..index]
                .iter()
                .any(|previous| previous == cell_id)
        });
    let mut ordering_failed = false;
    candidate_brackets.sort_by(
        |(_, left), (_, right)| match left.lower().compare(right.lower()) {
            Ok(ordering) => ordering,
            Err(_) => {
                ordering_failed = true;
                Ordering::Equal
            }
        },
    );

    let mut overlap = false;
    let mut outside_domain = false;
    let brackets: Vec<_> = candidate_brackets
        .into_iter()
        .map(|(_, bracket)| bracket)
        .collect();
    if let Some(domain) = problem.domain.as_ref() {
        for bracket in &brackets {
            let lower_inside = bracket.lower().compare(&dyadic_to_real(domain.lower())?);
            let upper_inside = bracket.upper().compare(&dyadic_to_real(domain.upper())?);
            match (lower_inside, upper_inside) {
                (Ok(lower), Ok(upper)) if lower != Ordering::Less && upper != Ordering::Greater => {
                }
                (Ok(_), Ok(_)) => outside_domain = true,
                _ => ordering_failed = true,
            }
        }
    } else {
        outside_domain = true;
    }
    for pair in brackets.windows(2) {
        match pair[0].lower().compare(pair[1].lower()) {
            Ok(Ordering::Equal) => overlap = true,
            Ok(_) => {}
            Err(_) => ordering_failed = true,
        }
        match pair[0].upper().compare(pair[1].lower()) {
            Ok(Ordering::Greater) => overlap = true,
            Ok(_) => {}
            Err(_) => ordering_failed = true,
        }
    }

    let partition_complete = !cells.is_empty()
        && cells.iter().all(|cell| {
            matches!(
                cell.disposition,
                CellDisposition::Candidate
                    | CellDisposition::Coincident
                    | CellDisposition::RootFree
            )
        });
    let structural_complete = partition_complete
        && candidate_brackets_match
        && !unresolved_candidate
        && !duplicate_candidate
        && !overlap
        && !outside_domain
        && !ordering_failed;
    let source_domain = clone_domain(problem.constraint.domain());
    let source_function = clone_residual_function(problem.constraint.residual_function());
    let completeness_status = if structural_complete {
        WitnessStatus::Proved
    } else {
        WitnessStatus::Undetermined {
            reason: WitnessUndeterminedReason::PartitionOrCandidatesIncomplete,
        }
    };
    let non_overlap_status =
        if !duplicate_candidate && !overlap && !outside_domain && !ordering_failed {
            WitnessStatus::Proved
        } else {
            WitnessStatus::Undetermined {
                reason: WitnessUndeterminedReason::BracketSetInvalid,
            }
        };
    let completeness = ExistenceWitness::new(
        ExistenceEvidence::CompleteCandidatePartition,
        WitnessMethod::DeterministicDyadicCandidatePartition,
        completeness_status,
    );
    let non_overlap = UniquenessWitness::new(
        UniquenessEvidence::PairwiseNonOverlappingBrackets,
        WitnessMethod::CanonicalLowerEndpointOrdering,
        non_overlap_status,
    );
    Ok(RootIsolation::new(
        brackets,
        completeness,
        policy.method,
        source_domain,
        source_function,
        non_overlap,
    ))
}

fn classify_constraint(
    problem: DerivedRootProblem,
    isolation: &RootIsolation,
) -> Result<IntersectionSet, GeometryFailure> {
    let residual = residual_from_constraint(&problem.constraint)?;
    let root_witness = IntersectionWitness::from_isolation_and_constraint(
        clone_isolation(isolation)?,
        problem.constraint,
    );
    let constraint = root_witness.constraint();
    if constraint.rank_is_undetermined() {
        return Ok(undetermined_with(
            IntersectionUndeterminedReason::EvaluationUndetermined,
            residual,
            root_witness,
        ));
    }
    let unresolved_isolation = !matches!(
        root_witness.root_isolation().completeness().status(),
        WitnessStatus::Proved
    ) || !matches!(
        root_witness.root_isolation().non_overlap().status(),
        WitnessStatus::Proved
    );
    if unresolved_isolation {
        return Ok(undetermined_with(
            IntersectionUndeterminedReason::RootIsolationIncomplete,
            residual,
            root_witness,
        ));
    }
    match constraint.support_relation() {
        SupportRelation::Coincident { .. } => {
            let Ok((left, right)) = declaration_pair(constraint) else {
                return Ok(undetermined_with(
                    IntersectionUndeterminedReason::CoincidenceUndetermined,
                    residual,
                    root_witness,
                ));
            };
            IntersectionSet::from_components(
                IntersectionKind::Coincident,
                clone_residual(&residual)?,
                clone_witness(&root_witness)?,
                vec![OwnedIntersectionEvidence::Coincident(Box::new(
                    CoincidentSupport::new(
                        left,
                        clone_residual(&residual)?,
                        right,
                        clone_witness(&root_witness)?,
                    ),
                ))],
            )
        }
        SupportRelation::Disjoint { .. } => Ok(IntersectionSet::empty(residual, root_witness)),
        SupportRelation::Finite { .. } => {
            let points = derive_points(constraint, isolation, &residual, &root_witness)?;
            match points.len() {
                0 => Ok(IntersectionSet::empty(residual, root_witness)),
                1 => {
                    let point = points.into_iter().next().ok_or({
                        GeometryFailure::CalculusFailure {
                            error: CalculusFailure::Polynomial(
                                PolynomialFailure::EmptyCoefficients,
                            ),
                        }
                    })?;
                    IntersectionSet::from_components(
                        IntersectionKind::Point,
                        residual,
                        root_witness,
                        vec![OwnedIntersectionEvidence::Point(Box::new(point))],
                    )
                }
                _ => IntersectionSet::from_components(
                    IntersectionKind::FinitePoints,
                    residual,
                    root_witness,
                    points
                        .into_iter()
                        .map(|point| OwnedIntersectionEvidence::Point(Box::new(point)))
                        .collect(),
                ),
            }
        }
        SupportRelation::Curve { .. } => {
            let identities = derive_branch_identities(constraint, isolation)?;
            if identities.is_empty() {
                return Ok(undetermined_with(
                    IntersectionUndeterminedReason::BranchIdentityUndetermined,
                    residual,
                    root_witness,
                ));
            }
            if identities.len() != isolation.brackets().len() {
                return Ok(undetermined_with(
                    IntersectionUndeterminedReason::BranchIdentityUndetermined,
                    residual,
                    root_witness,
                ));
            }
            let mut branches = Vec::new();
            for (ordinal, identity) in identities.into_iter().enumerate() {
                let Some(bracket) = isolation.brackets().get(ordinal) else {
                    return Ok(undetermined_with(
                        IntersectionUndeterminedReason::BranchIdentityUndetermined,
                        residual,
                        root_witness,
                    ));
                };
                branches.push(CurveBranch::new(
                    identity,
                    bracket.contact_order().clone(),
                    clone_residual(&residual)?,
                    clone_witness(&root_witness)?,
                ));
            }
            if branches.len() == 1 {
                let branch = branches.into_iter().next().ok_or({
                    GeometryFailure::CalculusFailure {
                        error: CalculusFailure::Polynomial(PolynomialFailure::EmptyCoefficients),
                    }
                })?;
                let identity = clone_identity(branch.branch_identity())?;
                let curve = IntersectionCurve::new(
                    identity,
                    branch.contact_order().clone(),
                    branch.residual().clone_for_operation()?,
                    constraint_support_id(constraint),
                    clone_witness(&root_witness)?,
                );
                IntersectionSet::from_components(
                    IntersectionKind::Curve,
                    residual,
                    root_witness,
                    vec![OwnedIntersectionEvidence::Curve(Box::new(curve))],
                )
            } else {
                IntersectionSet::from_components(
                    IntersectionKind::CurveBranches,
                    residual,
                    root_witness,
                    branches
                        .into_iter()
                        .map(|branch| OwnedIntersectionEvidence::CurveBranch(Box::new(branch)))
                        .collect(),
                )
            }
        }
        SupportRelation::Undetermined { .. } => Ok(undetermined_with(
            IntersectionUndeterminedReason::CoincidenceUndetermined,
            residual,
            root_witness,
        )),
    }
}

fn derive_support_relation(left: &OperandDescriptor, right: &OperandDescriptor) -> SupportRelation {
    if left.support_id == right.support_id && left.chart_id == right.chart_id {
        return SupportRelation::Coincident {
            evidence: SupportEvidence::CanonicalSupportAndChart,
        };
    }
    let left_evaluated = matches!(left.source, OperandSource::Evaluated);
    let right_evaluated = matches!(right.source, OperandSource::Evaluated);
    if (left_evaluated && left.exact_point.is_some())
        || (right_evaluated && right.exact_point.is_some())
    {
        return SupportRelation::Finite {
            evidence: SupportEvidence::EvaluatedSpatialCoordinates,
        };
    }
    if left_evaluated && right_evaluated {
        if let (Some(left), Some(right)) = (&left.exact_scalar, &right.exact_scalar) {
            return if left == right {
                SupportRelation::Coincident {
                    evidence: SupportEvidence::EqualEvaluatedScalars,
                }
            } else {
                SupportRelation::Disjoint {
                    evidence: SupportEvidence::DistinctEvaluatedScalars,
                }
            };
        }
        if left.exact_scalar.is_some() || right.exact_scalar.is_some() {
            return SupportRelation::Undetermined {
                reason: SupportUndeterminedReason::IncompleteEvaluatedScalar,
            };
        }
    }
    if matches!(left.source, OperandSource::Declaration)
        && matches!(right.source, OperandSource::Declaration)
    {
        return match (left.family, right.family) {
            (
                crate::types::ShapeFamily::ParametricCurve,
                crate::types::ShapeFamily::ParametricCurve,
            )
            | (
                crate::types::ShapeFamily::ParametricSurface,
                crate::types::ShapeFamily::ParametricSurface,
            ) => SupportRelation::Curve {
                evidence: SupportEvidence::CompatibleParametricSupports,
            },
            (
                crate::types::ShapeFamily::ImplicitSurface,
                crate::types::ShapeFamily::ImplicitSurface,
            ) => SupportRelation::Curve {
                evidence: SupportEvidence::ImplicitSupportPair,
            },
            _ => SupportRelation::Undetermined {
                reason: SupportUndeterminedReason::NoCommonParameterization,
            },
        };
    }
    SupportRelation::Undetermined {
        reason: SupportUndeterminedReason::UnclassifiedEvidence,
    }
}

fn derive_rank_evidence(
    left: &OperandDescriptor,
    right: &OperandDescriptor,
    support: &SupportRelation,
) -> RankEvidence {
    if matches!(support, SupportRelation::Undetermined { .. }) {
        return RankEvidence::Undetermined {
            reason: RankUndeterminedReason::SupportRelationUnresolved,
        };
    }
    if left.family == right.family {
        RankEvidence::Sufficient { rank: 1 }
    } else if (matches!(left.source, OperandSource::Evaluated) && left.exact_point.is_some())
        || (matches!(right.source, OperandSource::Evaluated) && right.exact_point.is_some())
    {
        RankEvidence::Sufficient { rank: 0 }
    } else {
        RankEvidence::Degenerate {
            reason: RankDegeneracyReason::InsufficientCommonRank,
        }
    }
}

fn derive_residual_polynomial(
    left: &OperandDescriptor,
    right: &OperandDescriptor,
) -> Result<Option<Polynomial>, GeometryFailure> {
    match (left.source, right.source) {
        (OperandSource::Evaluated, OperandSource::Evaluated) => {
            if let (Some(left), Some(right)) = (&left.exact_scalar, &right.exact_scalar) {
                return Ok(Some(
                    Polynomial::constant(left.sub(right).map_err(GeometryFailure::AlgebraFailure)?)
                        .map_err(calculus_polynomial_failure)?,
                ));
            }
            if let (Some(left), Some(right)) = (&left.exact_point, &right.exact_point) {
                return Ok(Some(
                    Polynomial::constant(
                        left.x
                            .sub(&right.x)
                            .map_err(GeometryFailure::AlgebraFailure)?,
                    )
                    .map_err(calculus_polynomial_failure)?,
                ));
            }
            Ok(None)
        }
        (OperandSource::Evaluated, OperandSource::Declaration) => {
            let Some(polynomial) = right.polynomial.as_ref() else {
                return Ok(None);
            };
            if let Some(point) = left.exact_point.as_ref() {
                return negate_polynomial_plus_constant(polynomial, &point.x);
            }
            if let Some(value) = left.exact_scalar.as_ref() {
                return negate_polynomial_plus_constant(polynomial, value);
            }
            Ok(None)
        }
        (OperandSource::Declaration, OperandSource::Evaluated) => {
            let Some(polynomial) = left.polynomial.as_ref() else {
                return Ok(None);
            };
            if let Some(point) = right.exact_point.as_ref() {
                return subtract_constant_from_polynomial(polynomial, &point.x);
            }
            if let Some(value) = right.exact_scalar.as_ref() {
                return subtract_constant_from_polynomial(polynomial, value);
            }
            Ok(None)
        }
        (OperandSource::Declaration, OperandSource::Declaration) => {
            match (&left.polynomial, &right.polynomial) {
                (Some(left), Some(right)) => {
                    let max = left.degree().max(right.degree());
                    let mut coefficients = Vec::with_capacity(max + 1);
                    for index in 0..=max {
                        let left_value = left
                            .coefficients()
                            .get(index)
                            .map(RealAlgebraic::try_clone)
                            .transpose()?;
                        let right_value = right
                            .coefficients()
                            .get(index)
                            .map(RealAlgebraic::try_clone)
                            .transpose()?;
                        let zero = RealAlgebraic::from_integer(BigInt::zero())?;
                        let left_value = left_value.unwrap_or(zero.try_clone()?);
                        let right_value = right_value.unwrap_or(zero);
                        coefficients.push(
                            left_value
                                .sub(&right_value)
                                .map_err(GeometryFailure::AlgebraFailure)?,
                        );
                    }
                    Ok(Some(
                        Polynomial::from_coefficients(coefficients)
                            .map_err(calculus_polynomial_failure)?,
                    ))
                }
                _ => Ok(None),
            }
        }
    }
}

fn subtract_constant_from_polynomial(
    polynomial: &Polynomial,
    value: &RealAlgebraic,
) -> Result<Option<Polynomial>, GeometryFailure> {
    let mut coefficients = Vec::with_capacity(polynomial.coefficients().len());
    for (index, coefficient) in polynomial.coefficients().iter().enumerate() {
        let adjusted = if index == 0 {
            coefficient
                .sub(value)
                .map_err(GeometryFailure::AlgebraFailure)?
        } else {
            coefficient.try_clone()?
        };
        coefficients.push(adjusted);
    }
    Ok(Some(
        Polynomial::from_coefficients(coefficients).map_err(calculus_polynomial_failure)?,
    ))
}

fn negate_polynomial_plus_constant(
    polynomial: &Polynomial,
    value: &RealAlgebraic,
) -> Result<Option<Polynomial>, GeometryFailure> {
    let mut coefficients = Vec::with_capacity(polynomial.coefficients().len());
    for (index, coefficient) in polynomial.coefficients().iter().enumerate() {
        let negated = RealAlgebraic::from_integer(BigInt::zero())?
            .sub(coefficient)
            .map_err(GeometryFailure::AlgebraFailure)?;
        let adjusted = if index == 0 {
            value
                .sub(coefficient)
                .map_err(GeometryFailure::AlgebraFailure)?
        } else {
            negated
        };
        coefficients.push(adjusted);
    }
    Ok(Some(
        Polynomial::from_coefficients(coefficients).map_err(calculus_polynomial_failure)?,
    ))
}

fn derive_numeric_domain(
    left: &OperandDescriptor,
    right: &OperandDescriptor,
) -> Result<Option<Enclosure>, GeometryFailure> {
    match (&left.parameter_bounds, &right.parameter_bounds) {
        (Some((left_lower, left_upper)), Some((right_lower, right_upper))) => {
            let lower = if left_lower.compare(right_lower)? == Ordering::Greater {
                left_lower
            } else {
                right_lower
            };
            let upper = if left_upper.compare(right_upper)? == Ordering::Less {
                left_upper
            } else {
                right_upper
            };
            if lower.compare(upper)? == Ordering::Greater {
                return Err(GeometryFailure::ParameterOutsideDomain);
            }
            let lower = real_to_dyadic(lower)?;
            let upper = real_to_dyadic(upper)?;
            Ok(Some(Enclosure::new(lower, upper).map_err(|error| {
                GeometryFailure::CalculusFailure {
                    error: CalculusFailure::Interval(error),
                }
            })?))
        }
        (Some((lower, upper)), None) | (None, Some((lower, upper))) => {
            let lower = real_to_dyadic(lower)?;
            let upper = real_to_dyadic(upper)?;
            Ok(Some(Enclosure::new(lower, upper).map_err(|error| {
                GeometryFailure::CalculusFailure {
                    error: CalculusFailure::Interval(error),
                }
            })?))
        }
        (None, None) if left.exact_scalar.is_some() && right.exact_scalar.is_some() => Ok(Some(
            Enclosure::new(
                Dyadic::new(BigInt::zero(), 0),
                Dyadic::new(BigInt::zero(), 0),
            )
            .map_err(|error| GeometryFailure::CalculusFailure {
                error: CalculusFailure::Interval(error),
            })?,
        )),
        (None, None) => Ok(None),
    }
}

fn descriptor_from_declaration(
    declaration: &ShapeDeclaration,
    evaluation: ShapeEvaluation,
) -> Result<OperandDescriptor, GeometryFailure> {
    let family = declaration.family();
    let (support_id, chart_id, domain, parameter_bounds, expression, polynomial) = match declaration
    {
        ShapeDeclaration::ParametricCurve(value) => {
            let chart_id = match value.chart() {
                crate::types::CurveChart::Chart2(_) => String::from("chart-2"),
                crate::types::CurveChart::Chart3(_) => String::from("chart-3"),
            };
            let expression = curve_expression(value);
            let polynomial = curve_polynomial(value)?;
            (
                format!("curve:{}", expression),
                chart_id,
                String::from(value.chart_domain()),
                interval_bounds(value.domain()),
                expression,
                polynomial,
            )
        }
        ShapeDeclaration::ParametricSurface(value) => {
            let expression = surface_expression(value);
            (
                format!("surface:{}", expression),
                String::from("surface-chart"),
                String::from(value.chart().identity().domain_id().as_str()),
                None,
                expression,
                None,
            )
        }
        ShapeDeclaration::ImplicitSurface(value) => {
            let expression = implicit_expression(value);
            (
                format!("implicit:{}", expression),
                String::from("implicit-chart"),
                String::from("support-domain"),
                None,
                expression,
                implicit_polynomial(value)?,
            )
        }
    };
    let (exact_scalar, exact_point, certified_scalar) = evaluation_evidence(&evaluation)?;
    Ok(OperandDescriptor {
        source: OperandSource::Declaration,
        family,
        support_id,
        chart_id,
        domain,
        parameter_bounds,
        expression,
        polynomial,
        exact_scalar,
        exact_point,
        certified_scalar,
    })
}

fn descriptor_from_evaluation(
    evaluation: &ShapeEvaluation,
) -> Result<OperandDescriptor, GeometryFailure> {
    let family = match evaluation {
        ShapeEvaluation::Exact(ExactShapeEvaluation::ParametricCurve { .. })
        | ShapeEvaluation::CertifiedEnclosure(CertifiedShapeEvaluation::ParametricCurve {
            ..
        }) => crate::types::ShapeFamily::ParametricCurve,
        ShapeEvaluation::Exact(ExactShapeEvaluation::ParametricSurface { .. })
        | ShapeEvaluation::CertifiedEnclosure(CertifiedShapeEvaluation::ParametricSurface {
            ..
        }) => crate::types::ShapeFamily::ParametricSurface,
        ShapeEvaluation::Exact(ExactShapeEvaluation::ImplicitSurface { .. })
        | ShapeEvaluation::CertifiedEnclosure(CertifiedShapeEvaluation::ImplicitSurface {
            ..
        }) => crate::types::ShapeFamily::ImplicitSurface,
        ShapeEvaluation::Undetermined { family } => *family,
    };
    let (exact_scalar, exact_point, certified_scalar) = evaluation_evidence(evaluation)?;
    let expression = format!("evaluated:{family:?}:{evaluation:?}");
    Ok(OperandDescriptor {
        source: OperandSource::Evaluated,
        family,
        support_id: expression.clone(),
        chart_id: format!("evaluated-{family:?}"),
        domain: String::from("evaluated-domain"),
        parameter_bounds: None,
        expression,
        polynomial: None,
        exact_scalar,
        exact_point,
        certified_scalar,
    })
}

type EvaluationEvidence = (
    Option<RealAlgebraic>,
    Option<Point3>,
    Option<(
        Enclosure,
        OwnerId,
        ExactExpressionRequirement,
        NecoImplementationSource,
    )>,
);

fn evaluation_evidence(
    evaluation: &ShapeEvaluation,
) -> Result<EvaluationEvidence, GeometryFailure> {
    match evaluation {
        ShapeEvaluation::Exact(ExactShapeEvaluation::ImplicitSurface { scalar }) => {
            Ok((Some(scalar.try_clone()?), None, None))
        }
        ShapeEvaluation::Exact(ExactShapeEvaluation::ParametricCurve { coordinate }) => {
            let point = match coordinate {
                ShapeCoordinate::Spatial(point) => clone_point(point)?,
                ShapeCoordinate::Planar(point) => Point3 {
                    x: point.x.try_clone()?,
                    y: point.y.try_clone()?,
                    z: RealAlgebraic::from_integer(BigInt::zero())?,
                },
            };
            Ok((None, Some(point), None))
        }
        ShapeEvaluation::Exact(ExactShapeEvaluation::ParametricSurface { coordinate }) => {
            Ok((None, Some(clone_point(coordinate)?), None))
        }
        ShapeEvaluation::CertifiedEnclosure(CertifiedShapeEvaluation::ImplicitSurface {
            scalar,
            owner,
            requirement,
            source,
        }) => Ok((
            None,
            None,
            Some((
                clone_enclosure(scalar)?,
                *owner,
                clone_requirement(requirement)?,
                *source,
            )),
        )),
        ShapeEvaluation::CertifiedEnclosure(_) | ShapeEvaluation::Undetermined { .. } => {
            Ok((None, None, None))
        }
    }
}

fn derive_points(
    constraint: &IntersectionConstraint,
    isolation: &RootIsolation,
    residual: &IntersectionResidual,
    witness: &IntersectionWitness,
) -> Result<IntersectionPointSet, GeometryFailure> {
    let Some(point) = operand_point(constraint.left_operand())
        .or_else(|| operand_point(constraint.right_operand()))
    else {
        return Ok(Vec::new());
    };
    isolation
        .brackets()
        .iter()
        .enumerate()
        .map(|(ordinal, bracket)| {
            Ok(IntersectionPoint::new(
                Some(derive_branch_identity(constraint, ordinal)),
                bracket.contact_order().clone(),
                clone_point(&point)?,
                clone_residual(residual)?,
                clone_witness(witness)?,
            ))
        })
        .collect()
}

fn derive_branch_identities(
    constraint: &IntersectionConstraint,
    isolation: &RootIsolation,
) -> Result<BranchIdentitySet, GeometryFailure> {
    if constraint.support_relation().is_undetermined() {
        return Ok(Vec::new());
    }
    let count = isolation.brackets().len();
    Ok((0..count)
        .map(|ordinal| derive_branch_identity(constraint, ordinal))
        .collect())
}

fn derive_branch_identity(constraint: &IntersectionConstraint, ordinal: usize) -> BranchIdentity {
    let support_id = constraint_support_id(constraint);
    let chart_id = constraint_chart_id(constraint);
    BranchIdentity::new(
        format!("{support_id}|{chart_id}|candidate-{ordinal}"),
        chart_id,
        support_id,
    )
}

fn declaration_pair(
    constraint: &IntersectionConstraint,
) -> Result<(ShapeDeclaration, ShapeDeclaration), GeometryFailure> {
    match (constraint.left_operand(), constraint.right_operand()) {
        (IntersectionOperand::Declaration(left), IntersectionOperand::Declaration(right)) => {
            Ok((clone_declaration(left)?, clone_declaration(right)?))
        }
        _ => Err(GeometryFailure::EvaluationIndeterminacy {
            family: crate::types::ShapeFamily::ImplicitSurface,
            reason: crate::types::ShapeEvaluationIndeterminacy::DomainValidity,
        }),
    }
}

fn operand_point(operand: &IntersectionOperand) -> Option<Point3> {
    match operand {
        IntersectionOperand::Evaluated(ShapeEvaluation::Exact(
            ExactShapeEvaluation::ParametricSurface { coordinate },
        )) => Some(clone_point(coordinate).ok()?),
        IntersectionOperand::Evaluated(ShapeEvaluation::Exact(
            ExactShapeEvaluation::ParametricCurve {
                coordinate: ShapeCoordinate::Spatial(coordinate),
            },
        )) => Some(clone_point(coordinate).ok()?),
        _ => None,
    }
}

fn residual_from_constraint(
    constraint: &IntersectionConstraint,
) -> Result<IntersectionResidual, GeometryFailure> {
    let left = operand_residual_value(constraint.left_operand())?;
    let right = operand_residual_value(constraint.right_operand())?;
    let combined = match (&left, &right) {
        (ResidualValue::Exact { value: left }, ResidualValue::Exact { value: right }) => {
            ResidualValue::Exact {
                value: left.sub(right).map_err(GeometryFailure::AlgebraFailure)?,
            }
        }
        (
            ResidualValue::Certified {
                owner: left_owner,
                requirement: left_requirement,
                source: left_source,
                value: left,
            },
            ResidualValue::Certified { value: right, .. },
        ) => ResidualValue::Certified {
            owner: *left_owner,
            requirement: clone_requirement(left_requirement)?,
            source: *left_source,
            value: subtract_enclosures(left, right)?,
        },
        _ => ResidualValue::Undetermined {
            reason: ResidualUndeterminedReason::OperandEvidenceUnresolved,
        },
    };
    Ok(IntersectionResidual::new(combined, left, right))
}

fn operand_residual_value(operand: &IntersectionOperand) -> Result<ResidualValue, GeometryFailure> {
    match operand {
        IntersectionOperand::Evaluated(ShapeEvaluation::Exact(
            ExactShapeEvaluation::ImplicitSurface { scalar },
        )) => Ok(ResidualValue::Exact {
            value: scalar.try_clone()?,
        }),
        IntersectionOperand::Evaluated(ShapeEvaluation::CertifiedEnclosure(
            CertifiedShapeEvaluation::ImplicitSurface {
                scalar,
                owner,
                requirement,
                source,
            },
        )) => Ok(ResidualValue::Certified {
            owner: *owner,
            requirement: clone_requirement(requirement)?,
            source: *source,
            value: clone_enclosure(scalar)?,
        }),
        _ => Ok(ResidualValue::Undetermined {
            reason: ResidualUndeterminedReason::ScalarResidualUnavailable,
        }),
    }
}

fn undetermined_with(
    reason: IntersectionUndeterminedReason,
    residual: IntersectionResidual,
    witness: IntersectionWitness,
) -> IntersectionSet {
    IntersectionSet::undetermined(reason, residual, witness)
}

fn refine_root(
    bracket: &CalculusRootBracket,
    policy: &RootIsolationPolicy,
) -> Result<CalculusRootIsolation, GeometryFailure> {
    match policy.method {
        RootRefinementMethod::Bisection => {
            neco_calculus::refine_root_bisection(bracket, policy.max_iterations)
                .map_err(map_calculus_failure)
        }
        RootRefinementMethod::NewtonBisection => {
            neco_calculus::refine_root_newton_bisection(bracket, policy.max_iterations, 32)
                .map_err(map_calculus_failure)
        }
    }
}

fn contact_order_from_evidence(evidence: &RootEvidence) -> ContactOrder {
    match evidence {
        RootEvidence::SignChange { .. } | RootEvidence::SimpleRoot { .. } => {
            ContactOrder::new(ContactOrderKind::Transverse, 1)
        }
        RootEvidence::EvenMultiplicityTouch { order, .. } => {
            if *order == 2 {
                ContactOrder::new(ContactOrderKind::Tangent, 2)
            } else if *order > 2 {
                ContactOrder::new(ContactOrderKind::HigherOrder, *order)
            } else {
                ContactOrder::new(ContactOrderKind::Undetermined, 0)
            }
        }
        RootEvidence::HigherOrder { order, .. } => {
            ContactOrder::new(ContactOrderKind::HigherOrder, *order)
        }
        RootEvidence::Undetermined { .. } => ContactOrder::new(ContactOrderKind::Undetermined, 0),
    }
}

fn clone_isolation(isolation: &RootIsolation) -> Result<RootIsolation, GeometryFailure> {
    let brackets = isolation
        .brackets
        .iter()
        .map(clone_bracket)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(RootIsolation::new(
        brackets,
        clone_existence(&isolation.completeness),
        isolation.method,
        clone_domain(&isolation.source_domain),
        clone_residual_function(&isolation.source_function),
        clone_uniqueness(&isolation.non_overlap),
    ))
}

fn clone_bracket(bracket: &RootBracket) -> Result<RootBracket, GeometryFailure> {
    Ok(RootBracket {
        contact_order: bracket.contact_order.clone(),
        convergence: ConvergenceWitness::new(
            bracket.convergence.final_lower.try_clone()?,
            bracket.convergence.final_upper.try_clone()?,
            bracket.convergence.initial_lower.try_clone()?,
            bracket.convergence.initial_upper.try_clone()?,
            bracket.convergence.iterations,
            bracket.convergence.method,
            bracket.convergence.status,
        ),
        lower: bracket.lower.try_clone()?,
        sign_change: bracket.sign_change,
        upper: bracket.upper.try_clone()?,
        evidence: clone_root_evidence(&bracket.evidence)?,
        exact_parameter: bracket
            .exact_parameter
            .as_ref()
            .map(|parameter| parameter.try_clone())
            .transpose()?,
    })
}

pub(crate) fn clone_root_evidence(
    evidence: &RootEvidence,
) -> Result<RootEvidence, GeometryFailure> {
    Ok(match evidence {
        RootEvidence::SignChange {
            lower_sign,
            upper_sign,
            derivative_enclosure,
            source,
        } => RootEvidence::SignChange {
            lower_sign: *lower_sign,
            upper_sign: *upper_sign,
            derivative_enclosure: clone_enclosure(derivative_enclosure)?,
            source: source.clone(),
        },
        RootEvidence::EvenMultiplicityTouch {
            order,
            point,
            source,
        } => RootEvidence::EvenMultiplicityTouch {
            order: *order,
            point: point.try_clone()?,
            source: source.clone(),
        },
        RootEvidence::HigherOrder {
            order,
            point,
            source,
        } => RootEvidence::HigherOrder {
            order: *order,
            point: point.try_clone()?,
            source: source.clone(),
        },
        RootEvidence::SimpleRoot { point, source } => RootEvidence::SimpleRoot {
            point: point.try_clone()?,
            source: source.clone(),
        },
        RootEvidence::Undetermined { reason, source } => RootEvidence::Undetermined {
            reason: *reason,
            source: source.clone(),
        },
    })
}

fn clone_existence(value: &ExistenceWitness) -> ExistenceWitness {
    ExistenceWitness::new(
        value.evidence,
        value.method,
        clone_witness_status(&value.status),
    )
}

fn clone_uniqueness(value: &UniquenessWitness) -> UniquenessWitness {
    UniquenessWitness::new(
        value.evidence,
        value.method,
        clone_witness_status(&value.status),
    )
}

fn clone_witness_status(status: &WitnessStatus) -> WitnessStatus {
    match status {
        WitnessStatus::Proved => WitnessStatus::Proved,
        WitnessStatus::Undetermined { reason } => WitnessStatus::Undetermined { reason: *reason },
    }
}

fn clone_witness(value: &IntersectionWitness) -> Result<IntersectionWitness, GeometryFailure> {
    Ok(IntersectionWitness {
        existence: clone_existence(&value.existence),
        uniqueness: clone_uniqueness(&value.uniqueness),
        root_isolation: clone_isolation(&value.root_isolation)?,
        constraint: clone_constraint(&value.constraint)?,
    })
}

fn clone_constraint(
    value: &IntersectionConstraint,
) -> Result<IntersectionConstraint, GeometryFailure> {
    Ok(IntersectionConstraint::new(
        clone_operand(value.left_operand())?,
        clone_operand(value.right_operand())?,
        clone_residual_function(value.residual_function()),
        clone_domain(value.domain()),
        clone_support_relation(value.support_relation()),
        clone_rank_evidence(value.rank_evidence()),
    ))
}

fn clone_operand(value: &IntersectionOperand) -> Result<IntersectionOperand, GeometryFailure> {
    match value {
        IntersectionOperand::Declaration(declaration) => Ok(IntersectionOperand::Declaration(
            clone_declaration(declaration)?,
        )),
        IntersectionOperand::Evaluated(evaluation) => Ok(IntersectionOperand::Evaluated(
            clone_evaluation(evaluation)?,
        )),
    }
}

fn clone_evaluation(value: &ShapeEvaluation) -> Result<ShapeEvaluation, GeometryFailure> {
    Ok(match value {
        ShapeEvaluation::Exact(evaluation) => ShapeEvaluation::Exact(match evaluation {
            ExactShapeEvaluation::ParametricCurve { coordinate } => {
                ExactShapeEvaluation::ParametricCurve {
                    coordinate: clone_shape_coordinate(coordinate)?,
                }
            }
            ExactShapeEvaluation::ParametricSurface { coordinate } => {
                ExactShapeEvaluation::ParametricSurface {
                    coordinate: clone_point(coordinate)?,
                }
            }
            ExactShapeEvaluation::ImplicitSurface { scalar } => {
                ExactShapeEvaluation::ImplicitSurface {
                    scalar: scalar.try_clone()?,
                }
            }
        }),
        ShapeEvaluation::CertifiedEnclosure(evaluation) => {
            ShapeEvaluation::CertifiedEnclosure(match evaluation {
                CertifiedShapeEvaluation::ParametricCurve {
                    coordinate,
                    owner,
                    requirement,
                    source,
                } => CertifiedShapeEvaluation::ParametricCurve {
                    coordinate: clone_shape_coordinate_enclosure(coordinate)?,
                    owner: *owner,
                    requirement: clone_requirement(requirement)?,
                    source: *source,
                },
                CertifiedShapeEvaluation::ParametricSurface {
                    coordinate,
                    owner,
                    requirement,
                    source,
                } => CertifiedShapeEvaluation::ParametricSurface {
                    coordinate: Box::new(clone_coordinate_enclosure3(coordinate)?),
                    owner: *owner,
                    requirement: clone_requirement(requirement)?,
                    source: *source,
                },
                CertifiedShapeEvaluation::ImplicitSurface {
                    scalar,
                    owner,
                    requirement,
                    source,
                } => CertifiedShapeEvaluation::ImplicitSurface {
                    scalar: clone_enclosure(scalar)?,
                    owner: *owner,
                    requirement: clone_requirement(requirement)?,
                    source: *source,
                },
            })
        }
        ShapeEvaluation::Undetermined { family } => {
            ShapeEvaluation::Undetermined { family: *family }
        }
    })
}

fn clone_shape_coordinate(value: &ShapeCoordinate) -> Result<ShapeCoordinate, GeometryFailure> {
    Ok(match value {
        ShapeCoordinate::Planar(point) => ShapeCoordinate::Planar(Point2 {
            x: point.x.try_clone()?,
            y: point.y.try_clone()?,
        }),
        ShapeCoordinate::Spatial(point) => ShapeCoordinate::Spatial(clone_point(point)?),
    })
}

fn clone_shape_coordinate_enclosure(
    value: &crate::types::ShapeCoordinateEnclosure,
) -> Result<crate::types::ShapeCoordinateEnclosure, GeometryFailure> {
    Ok(match value {
        crate::types::ShapeCoordinateEnclosure::Planar(coordinate) => {
            crate::types::ShapeCoordinateEnclosure::Planar(crate::types::CoordinateEnclosure2 {
                x: clone_enclosure(&coordinate.x)?,
                y: clone_enclosure(&coordinate.y)?,
            })
        }
        crate::types::ShapeCoordinateEnclosure::Spatial(coordinate) => {
            crate::types::ShapeCoordinateEnclosure::Spatial(Box::new(clone_coordinate_enclosure3(
                coordinate,
            )?))
        }
    })
}

fn clone_coordinate_enclosure3(
    value: &crate::types::CoordinateEnclosure3,
) -> Result<crate::types::CoordinateEnclosure3, GeometryFailure> {
    Ok(crate::types::CoordinateEnclosure3 {
        x: clone_enclosure(&value.x)?,
        y: clone_enclosure(&value.y)?,
        z: clone_enclosure(&value.z)?,
    })
}

pub(crate) fn clone_support_relation(value: &SupportRelation) -> SupportRelation {
    match value {
        SupportRelation::Coincident { evidence } => SupportRelation::Coincident {
            evidence: *evidence,
        },
        SupportRelation::Curve { evidence } => SupportRelation::Curve {
            evidence: *evidence,
        },
        SupportRelation::Disjoint { evidence } => SupportRelation::Disjoint {
            evidence: *evidence,
        },
        SupportRelation::Finite { evidence } => SupportRelation::Finite {
            evidence: *evidence,
        },
        SupportRelation::Undetermined { reason } => {
            SupportRelation::Undetermined { reason: *reason }
        }
    }
}

pub(crate) fn clone_rank_evidence(value: &RankEvidence) -> RankEvidence {
    match value {
        RankEvidence::Sufficient { rank } => RankEvidence::Sufficient { rank: *rank },
        RankEvidence::Degenerate { reason } => RankEvidence::Degenerate { reason: *reason },
        RankEvidence::Undetermined { reason } => RankEvidence::Undetermined { reason: *reason },
    }
}

fn clone_residual(value: &IntersectionResidual) -> Result<IntersectionResidual, GeometryFailure> {
    Ok(IntersectionResidual::new(
        clone_residual_value(&value.combined)?,
        clone_residual_value(&value.left)?,
        clone_residual_value(&value.right)?,
    ))
}

impl IntersectionResidual {
    fn clone_for_operation(&self) -> Result<Self, GeometryFailure> {
        clone_residual(self)
    }
}

fn clone_residual_value(value: &ResidualValue) -> Result<ResidualValue, GeometryFailure> {
    Ok(match value {
        ResidualValue::Certified {
            owner,
            requirement,
            source,
            value,
        } => ResidualValue::Certified {
            owner: *owner,
            requirement: clone_requirement(requirement)?,
            source: *source,
            value: clone_enclosure(value)?,
        },
        ResidualValue::Exact { value } => ResidualValue::Exact {
            value: value.try_clone()?,
        },
        ResidualValue::Undetermined { reason } => ResidualValue::Undetermined { reason: *reason },
    })
}

fn clone_domain(value: &IntersectionDomain) -> IntersectionDomain {
    IntersectionDomain::new(
        value.parameterization.clone(),
        value.restriction.clone(),
        value.support.clone(),
    )
}

pub(crate) fn clone_residual_function(value: &ResidualFunction) -> ResidualFunction {
    ResidualFunction::new(
        value.left_expression.clone(),
        value.right_expression.clone(),
    )
}

fn clone_identity(value: &BranchIdentity) -> Result<BranchIdentity, GeometryFailure> {
    Ok(BranchIdentity::new(
        value.branch_id.clone(),
        value.chart_id.clone(),
        value.support_id.clone(),
    ))
}

fn clone_point(value: &Point3) -> Result<Point3, GeometryFailure> {
    Ok(Point3 {
        x: value.x.try_clone()?,
        y: value.y.try_clone()?,
        z: value.z.try_clone()?,
    })
}

fn clone_enclosure(value: &Enclosure) -> Result<Enclosure, GeometryFailure> {
    Enclosure::new(
        value
            .lower()
            .try_clone()
            .map_err(|error| GeometryFailure::CalculusFailure {
                error: CalculusFailure::Polynomial(PolynomialFailure::Arithmetic(error)),
            })?,
        value
            .upper()
            .try_clone()
            .map_err(|error| GeometryFailure::CalculusFailure {
                error: CalculusFailure::Polynomial(PolynomialFailure::Arithmetic(error)),
            })?,
    )
    .map_err(|error| GeometryFailure::CalculusFailure {
        error: CalculusFailure::Interval(error),
    })
}

fn clone_requirement(
    requirement: &ExactExpressionRequirement,
) -> Result<ExactExpressionRequirement, GeometryFailure> {
    ExactExpressionRequirement::new(
        requirement.consumer_id(),
        requirement.expressions(),
        requirement.decision(),
        requirement.precision(),
    )
    .map_err(|_| GeometryFailure::EvaluationIndeterminacy {
        family: crate::types::ShapeFamily::ImplicitSurface,
        reason: crate::types::ShapeEvaluationIndeterminacy::CalculusResult,
    })
}

fn subtract_enclosures(left: &Enclosure, right: &Enclosure) -> Result<Enclosure, GeometryFailure> {
    Enclosure::new(
        left.lower()
            .sub(right.upper())
            .map_err(map_bigint_failure)?,
        left.upper()
            .sub(right.lower())
            .map_err(map_bigint_failure)?,
    )
    .map_err(|error| GeometryFailure::CalculusFailure {
        error: CalculusFailure::Interval(error),
    })
}

pub(crate) fn dyadic_to_real(value: &Dyadic) -> Result<RealAlgebraic, GeometryFailure> {
    if value.is_zero() {
        return Ok(RealAlgebraic::from_integer(BigInt::zero())?);
    }
    let denominator = BigUint::one()
        .map_err(map_bigint_failure)?
        .shl_bits(value.exponent() as usize)
        .map_err(map_bigint_failure)?;
    let rational = RawRational::new(
        value.integer().try_clone().map_err(map_bigint_failure)?,
        denominator,
    )
    .reduce()
    .map_err(map_bigint_failure)?
    .into_reduced();
    RealAlgebraic::from_reduced_rational(&rational).map_err(GeometryFailure::AlgebraFailure)
}

fn real_to_dyadic(value: &RealAlgebraic) -> Result<Dyadic, GeometryFailure> {
    let minimal = value.minimal_polynomial().polynomial();
    if minimal.degree() == Some(1) {
        let coefficients = minimal.coefficients();
        let mut numerator = coefficients[0].negated().map_err(map_bigint_failure)?;
        let leading = &coefficients[1];
        if matches!(leading.sign(), Sign::Negative) {
            numerator = numerator.negated().map_err(map_bigint_failure)?;
        }
        let denominator = leading.magnitude();
        let bit_len = denominator.bit_len();
        if bit_len > 0 && (0..bit_len - 1).all(|bit| !denominator.bit(bit)) {
            return Ok(Dyadic::new(numerator, (bit_len - 1) as u32));
        }
    }
    if value.is_zero() {
        return Ok(Dyadic::new(BigInt::zero(), 0));
    }
    if value.is_one() {
        return Ok(Dyadic::new(BigInt::one().map_err(map_bigint_failure)?, 0));
    }
    let sign = value.sign().map_err(GeometryFailure::AlgebraFailure)?;
    if matches!(sign, Sign::Negative) {
        let negative_one = BigInt::zero()
            .sub(&BigInt::one().map_err(map_bigint_failure)?)
            .map_err(map_bigint_failure)?;
        let magnitude = value.mul(&RealAlgebraic::from_integer(negative_one)?)?;
        let mut dyadic = real_to_dyadic(&magnitude)?;
        dyadic = Dyadic::new(
            dyadic.integer().negated().map_err(map_bigint_failure)?,
            dyadic.exponent(),
        );
        return Ok(dyadic);
    }
    let enclosure = value
        .enclose(0)
        .map_err(GeometryFailure::AlgebraFailure)?
        .enclosure()
        .try_clone()
        .map_err(map_bigint_failure)?;
    if enclosure.lower() == enclosure.upper() {
        return enclosure.lower().try_clone().map_err(map_bigint_failure);
    }
    Err(GeometryFailure::EvaluationIndeterminacy {
        family: crate::types::ShapeFamily::ParametricCurve,
        reason: crate::types::ShapeEvaluationIndeterminacy::DomainValidity,
    })
}

fn map_bigint_failure(error: BigintError) -> GeometryFailure {
    GeometryFailure::CalculusFailure {
        error: CalculusFailure::Polynomial(PolynomialFailure::Arithmetic(error)),
    }
}

fn map_calculus_failure(error: CalculusFailure) -> GeometryFailure {
    GeometryFailure::CalculusFailure { error }
}

fn calculus_polynomial_failure(error: PolynomialFailure) -> GeometryFailure {
    GeometryFailure::CalculusFailure {
        error: CalculusFailure::Polynomial(error),
    }
}

fn opposite_nonzero(left: Sign, right: Sign) -> bool {
    matches!(
        (left, right),
        (Sign::Negative, Sign::Positive) | (Sign::Positive, Sign::Negative)
    )
}

fn root_sign(sign: Sign) -> RootSign {
    match sign {
        Sign::Negative => RootSign::Negative,
        Sign::Positive => RootSign::Positive,
        Sign::Zero => RootSign::Zero,
    }
}

fn constraint_support_id(constraint: &IntersectionConstraint) -> String {
    format!(
        "{}|{}",
        operand_support_id(constraint.left_operand()),
        operand_support_id(constraint.right_operand())
    )
}

fn constraint_chart_id(constraint: &IntersectionConstraint) -> String {
    format!(
        "{}|{}",
        operand_chart_id(constraint.left_operand()),
        operand_chart_id(constraint.right_operand())
    )
}

fn operand_support_id(operand: &IntersectionOperand) -> String {
    match operand {
        IntersectionOperand::Declaration(value) => format!("declaration-{:?}", value.family()),
        IntersectionOperand::Evaluated(value) => format!("evaluated-{:?}", value),
    }
}

fn operand_chart_id(operand: &IntersectionOperand) -> String {
    match operand {
        IntersectionOperand::Declaration(value) => format!("chart-{:?}", value.family()),
        IntersectionOperand::Evaluated(value) => format!("evaluation-{:?}", value),
    }
}

fn curve_expression(value: &crate::types::ParametricCurveDeclaration) -> String {
    format!("curve-domain:{}", value.chart_domain())
}

fn surface_expression(value: &crate::types::ParametricSurfaceDeclaration) -> String {
    format!("surface-domain:{}", value.chart().display_name().as_str())
}

fn implicit_expression(value: &crate::types::ImplicitSurfaceDeclaration) -> String {
    format!("implicit-terms:{}", value.equation().terms().len())
}

fn interval_bounds(
    interval: &crate::types::ParameterInterval,
) -> Option<(RealAlgebraic, RealAlgebraic)> {
    Some((
        interval.lower().try_clone().ok()?,
        interval.upper().try_clone().ok()?,
    ))
}

fn curve_polynomial(
    value: &crate::types::ParametricCurveDeclaration,
) -> Result<Option<Polynomial>, GeometryFailure> {
    let chart = match value.chart() {
        crate::types::CurveChart::Chart2(chart) => chart.numerator_x(),
        crate::types::CurveChart::Chart3(chart) => chart.numerator_x(),
    };
    let coefficients = chart
        .to_real_algebraic_coefficients()
        .map_err(|error| GeometryFailure::CalculusFailure {
            error: CalculusFailure::Polynomial(PolynomialFailure::Algebra(error)),
        })?
        .into_coefficients();
    if coefficients.is_empty() {
        return Ok(Some(
            Polynomial::zero().map_err(calculus_polynomial_failure)?,
        ));
    }
    Ok(Some(
        Polynomial::from_coefficients(coefficients).map_err(calculus_polynomial_failure)?,
    ))
}

fn implicit_polynomial(
    value: &crate::types::ImplicitSurfaceDeclaration,
) -> Result<Option<Polynomial>, GeometryFailure> {
    let mut coefficients = Vec::new();
    for term in value.equation().terms() {
        if term.exponent_y() != 0 || term.exponent_z() != 0 {
            return Ok(None);
        }
        while coefficients.len() <= term.exponent_x() as usize {
            coefficients.push(RealAlgebraic::from_integer(BigInt::zero())?);
        }
        coefficients[term.exponent_x() as usize] = term.coefficient().try_clone()?;
    }
    if coefficients.is_empty() {
        return Ok(Some(
            Polynomial::zero().map_err(calculus_polynomial_failure)?,
        ));
    }
    Ok(Some(
        Polynomial::from_coefficients(coefficients).map_err(calculus_polynomial_failure)?,
    ))
}

fn clone_declaration(value: &ShapeDeclaration) -> Result<ShapeDeclaration, GeometryFailure> {
    match value {
        ShapeDeclaration::ParametricCurve(value) => Ok(ShapeDeclaration::ParametricCurve(
            crate::types::ParametricCurveDeclaration::new(
                clone_curve_chart(value.chart())?,
                crate::types::ParameterInterval::new(
                    value.domain().lower().try_clone()?,
                    value.domain().upper().try_clone()?,
                    value.domain().lower_closed(),
                    value.domain().upper_closed(),
                )?,
            ),
        )),
        ShapeDeclaration::ParametricSurface(value) => Ok(ShapeDeclaration::ParametricSurface(
            crate::types::ParametricSurfaceDeclaration::new(
                clone_surface_chart(value.chart())?,
                crate::types::SurfaceParameterDomain::new(
                    clone_interval(value.domain().u())?,
                    clone_interval(value.domain().v())?,
                ),
            ),
        )),
        ShapeDeclaration::ImplicitSurface(value) => Ok(ShapeDeclaration::ImplicitSurface(
            crate::types::ImplicitSurfaceDeclaration::new(clone_polynomial3(value.equation())?),
        )),
    }
}

fn clone_curve_chart(
    value: &crate::types::CurveChart,
) -> Result<crate::types::CurveChart, GeometryFailure> {
    match value {
        crate::types::CurveChart::Chart2(chart) => Ok(crate::types::CurveChart::Chart2(
            neco_geometry_foundation::ChartDefinition2::new(
                chart.domain_id().try_clone()?,
                chart.display_name().try_clone()?,
                chart.coefficients().try_clone()?,
                chart.numerator_x().try_clone()?,
                chart.numerator_y().try_clone()?,
                chart.denominator().try_clone()?,
            )?,
        )),
        crate::types::CurveChart::Chart3(chart) => {
            Ok(crate::types::CurveChart::Chart3(clone_chart3(chart)?))
        }
    }
}

fn clone_surface_chart(
    chart: &crate::types::ParametricSurfaceChart2,
) -> Result<crate::types::ParametricSurfaceChart2, GeometryFailure> {
    crate::types::ParametricSurfaceChart2::try_new(
        chart.identity().domain_id().try_clone()?,
        chart.display_name().try_clone()?,
        chart.identity().version(),
        chart.orientation(),
        crate::evaluation::clone_rational_surface(chart.surface())?,
    )
}

fn clone_chart3(
    chart: &neco_geometry_foundation::ChartDefinition3,
) -> Result<neco_geometry_foundation::ChartDefinition3, GeometryFailure> {
    Ok(neco_geometry_foundation::ChartDefinition3::new(
        chart.domain_id().try_clone()?,
        chart.display_name().try_clone()?,
        chart.coefficients().try_clone()?,
        chart.numerator_x().try_clone()?,
        chart.numerator_y().try_clone()?,
        chart.numerator_z().try_clone()?,
        chart.denominator().try_clone()?,
    )?)
}

fn clone_interval(
    value: &crate::types::ParameterInterval,
) -> Result<crate::types::ParameterInterval, GeometryFailure> {
    crate::types::ParameterInterval::new(
        value.lower().try_clone()?,
        value.upper().try_clone()?,
        value.lower_closed(),
        value.upper_closed(),
    )
}

fn clone_polynomial3(
    value: &crate::types::MultivariatePolynomial3,
) -> Result<crate::types::MultivariatePolynomial3, GeometryFailure> {
    let terms = value
        .terms()
        .iter()
        .map(|term| {
            crate::types::PolynomialTerm3::new(
                term.coefficient().try_clone()?,
                term.exponent_x(),
                term.exponent_y(),
                term.exponent_z(),
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    crate::types::MultivariatePolynomial3::try_from_terms(terms)
}

fn evaluate_declaration_capability(
    declaration: &ShapeDeclaration,
) -> Result<ShapeEvaluation, GeometryFailure> {
    match evaluate_shape(declaration_probe_request(declaration)?) {
        Ok(evaluation) => Ok(evaluation),
        Err(GeometryFailure::CalculusFailure {
            error: CalculusFailure::Polynomial(PolynomialFailure::EmptyCoefficients),
        }) => Ok(ShapeEvaluation::Undetermined {
            family: declaration.family(),
        }),
        Err(error) => Err(error),
    }
}

fn declaration_probe_request(
    declaration: &ShapeDeclaration,
) -> Result<ShapeEvaluationRequest, GeometryFailure> {
    match declaration {
        ShapeDeclaration::ParametricCurve(value) => Ok(ShapeEvaluationRequest::new(
            clone_declaration(declaration)?,
            match value.chart() {
                crate::types::CurveChart::Chart2(_) | crate::types::CurveChart::Chart3(_) => {
                    crate::types::ShapeEvaluationInput::ParametricCurve {
                        parameter: value.domain().lower().try_clone()?,
                    }
                }
            },
        )),
        ShapeDeclaration::ParametricSurface(value) => Ok(ShapeEvaluationRequest::new(
            clone_declaration(declaration)?,
            crate::types::ShapeEvaluationInput::ParametricSurface {
                u: value.domain().u().lower().try_clone()?,
                v: value.domain().v().lower().try_clone()?,
            },
        )),
        ShapeDeclaration::ImplicitSurface(_) => Ok(ShapeEvaluationRequest::new(
            clone_declaration(declaration)?,
            crate::types::ShapeEvaluationInput::ImplicitSurface {
                point: Point3 {
                    x: RealAlgebraic::from_integer(BigInt::zero())?,
                    y: RealAlgebraic::from_integer(BigInt::zero())?,
                    z: RealAlgebraic::from_integer(BigInt::zero())?,
                },
            },
        )),
    }
}

trait ChartDomain {
    fn chart_domain(&self) -> &str;
}

impl ChartDomain for crate::types::ParametricCurveDeclaration {
    fn chart_domain(&self) -> &str {
        match self.chart() {
            crate::types::CurveChart::Chart2(chart) => chart.domain_id().as_str(),
            crate::types::CurveChart::Chart3(chart) => chart.domain_id().as_str(),
        }
    }
}

trait SupportRelationExt {
    fn is_undetermined(&self) -> bool;
}

impl SupportRelationExt for SupportRelation {
    fn is_undetermined(&self) -> bool {
        matches!(self, SupportRelation::Undetermined { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn algebraic(value: i32) -> RealAlgebraic {
        RealAlgebraic::from_integer(BigInt::try_from(value).unwrap()).unwrap()
    }

    fn unit_enclosure() -> Enclosure {
        Enclosure::new(
            Dyadic::new(BigInt::zero(), 0),
            Dyadic::new(BigInt::one().unwrap(), 0),
        )
        .unwrap()
    }

    fn polynomial(coefficients: &[i32]) -> Polynomial {
        Polynomial::from_coefficients(coefficients.iter().map(|value| algebraic(*value)).collect())
            .unwrap()
    }

    fn problem(polynomial: Polynomial) -> DerivedRootProblem {
        let operand = IntersectionOperand::Evaluated(ShapeEvaluation::Undetermined {
            family: crate::types::ShapeFamily::ParametricCurve,
        });
        DerivedRootProblem {
            constraint: IntersectionConstraint::new(
                operand,
                IntersectionOperand::Evaluated(ShapeEvaluation::Undetermined {
                    family: crate::types::ShapeFamily::ImplicitSurface,
                }),
                ResidualFunction::new(String::from("left"), String::from("right")),
                IntersectionDomain::new(
                    String::from("chart"),
                    String::from("domain"),
                    String::from("support"),
                ),
                SupportRelation::Finite {
                    evidence: SupportEvidence::EvaluatedSpatialCoordinates,
                },
                RankEvidence::Sufficient { rank: 1 },
            ),
            polynomial: Some(polynomial),
            domain: Some(unit_enclosure()),
        }
    }

    fn policy(max_iterations: usize) -> RootIsolationPolicy {
        RootIsolationPolicy::new(
            max_iterations,
            RootRefinementMethod::Bisection,
            unit_enclosure(),
        )
        .unwrap()
    }

    #[test]
    fn sign_change_with_derivative_zero_is_unresolved() {
        let polynomial = polynomial(&[-1, 36, -96, 64]);
        let enclosure = unit_enclosure();
        let derivative = polynomial.derivative(DerivativeOrder::new(1)).unwrap();
        assert!(derivative
            .evaluate_enclosure(&enclosure, ROOT_INTERVAL_PRECISION_BITS)
            .unwrap()
            .contains_zero());
        assert!(matches!(
            assess_partition_cell(
                &polynomial,
                &derivative,
                &enclosure,
                0,
                &DerivativeOrder::new(2),
                "test",
                &[],
            )
            .unwrap(),
            CellAssessment::Unresolved(PartitionUnresolvedReason::SignChangeUniqueness)
        ));
    }

    #[test]
    fn exact_touch_pivot_keeps_distinct_root_region_pending() {
        let polynomial = polynomial(&[-3, 22, -52, 40]);
        let enclosure = unit_enclosure();
        let derivative = polynomial.derivative(DerivativeOrder::new(1)).unwrap();
        let assessment = assess_partition_cell(
            &polynomial,
            &derivative,
            &enclosure,
            0,
            &DerivativeOrder::new(8),
            "test",
            &[],
        )
        .unwrap();
        let pivot = match assessment {
            CellAssessment::ExactTouch { candidate, pivot } => {
                assert!(matches!(
                    candidate.evidence,
                    RootEvidence::EvenMultiplicityTouch { .. }
                ));
                pivot
            }
            _ => panic!("expected the exact touch pivot"),
        };
        let lower = Enclosure::new(
            enclosure.lower().try_clone().unwrap(),
            pivot.try_clone().unwrap(),
        )
        .unwrap();
        assert!(!matches!(
            assess_partition_cell(
                &polynomial,
                &derivative,
                &lower,
                1,
                &DerivativeOrder::new(8),
                "test",
                &[pivot],
            )
            .unwrap(),
            CellAssessment::RootFree
        ));

        let root_problem = problem(polynomial);
        let candidates = enumerate_root_candidates(&root_problem, &policy(2)).unwrap();
        let isolation = build_root_isolation(&root_problem, candidates, &policy(2)).unwrap();
        assert!(
            isolation.brackets().len() >= 2
                || matches!(
                    isolation.completeness().status(),
                    WitnessStatus::Undetermined { .. }
                )
        );
    }

    #[test]
    fn suppressed_exact_endpoint_is_not_repeated_as_candidate() {
        let root_problem = problem(polynomial(&[0, 0, 1]));
        let enumeration = enumerate_root_candidates(&root_problem, &policy(4)).unwrap();
        assert_eq!(enumeration.candidates.len(), 1);
        assert!(matches!(
            enumeration.candidates[0].evidence,
            RootEvidence::EvenMultiplicityTouch { .. }
        ));
    }
}
