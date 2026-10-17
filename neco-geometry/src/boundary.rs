use alloc::{format, string::String, vec, vec::Vec};
use core::cmp::Ordering;

use neco_algnum::RealAlgebraic;
use neco_bigint::{BigInt, BigUint, BigintError, Dyadic, RawRational};
use neco_calculus::{
    expose_membership_transitions, expose_region_sign, CalculusFailure, DerivativeOrder, Enclosure,
    MembershipJudgmentRequest as CalculusMembershipJudgmentRequest, MembershipTransitionReport,
    Polynomial, PolynomialFailure, RegionSign, RegionSignJudgment,
};
use neco_geometry_foundation::{Point2, Point3};

use crate::intersection::{
    clone_rank_evidence, clone_residual_function, clone_root_evidence, clone_support_relation,
};
use crate::{
    AlgebraicRegion, BoundaryResource, CanonicalAtom, ContactOrderKind, GeometryFailure,
    IntersectionSet, MembershipClassification, RankEvidence, RegionFunction,
    RegionMembershipRequest, RegionPoint, RegionPolynomial3, ResidualFunction, RootEvidence,
    SignRelation, SupportRelation,
};

pub const MAX_BOUNDARY_CANDIDATE_LOCI: usize = 64;
pub const MAX_BOUNDARY_CANDIDATE_PARAMETERS: usize = 4_096;
pub const MAX_BOUNDARY_SIDE_SAMPLES: usize = 4_096;
pub const MAX_BOUNDARY_INTERSECTIONS: usize = 16;
pub const BOUNDARY_ZERO_EXCLUSION_PRECISION_BITS: u32 = 64;

#[derive(Debug, Eq, PartialEq)]
pub struct BoundaryTypes {
    algebraic_boundary: AlgebraicBoundary,
    stratification: BoundaryStratification,
}
impl BoundaryTypes {
    pub fn algebraic_boundary(&self) -> &AlgebraicBoundary {
        &self.algebraic_boundary
    }
    pub fn stratification(&self) -> &BoundaryStratification {
        &self.stratification
    }
    pub fn into_parts(self) -> (AlgebraicBoundary, BoundaryStratification) {
        (self.algebraic_boundary, self.stratification)
    }
}
#[derive(Debug, Eq, PartialEq)]
pub struct BoundaryAtomLocus {
    atom_id: String,
    zero_relation: SignRelation,
    ambient_dimension: i64,
}
impl BoundaryAtomLocus {
    pub fn atom_id(&self) -> &str {
        &self.atom_id
    }
    pub fn zero_relation(&self) -> SignRelation {
        self.zero_relation
    }
    pub fn ambient_dimension(&self) -> i64 {
        self.ambient_dimension
    }
}
#[derive(Debug, Eq, PartialEq)]
pub enum BoundaryEvidenceStatus {
    Proved,
    Undetermined { reason: BoundaryUndeterminedReason },
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum BoundaryUndeterminedReason {
    RestrictedRationalDenominatorNotProvedNonzero,
    FormulaSensitiveTransitionNotProved,
    FormulaInsensitiveCandidatesOmitted,
    StrataEvidenceLocal,
    DisjointnessNotEstablished,
    IntersectionEvidenceUnresolved,
    ActiveAtomDimensionEvidenceInsufficient,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BoundaryEvidenceSource {
    ExactChartLocalCalculusAndRegionMembership,
}
#[derive(Debug, Eq, PartialEq)]
pub struct BoundaryTransitionWitness {
    atom_id: String,
    candidate_count: usize,
    inside_side_observed: bool,
    outside_side_observed: bool,
    evidence_source: BoundaryEvidenceSource,
}
impl BoundaryTransitionWitness {
    pub fn atom_id(&self) -> &str {
        &self.atom_id
    }
    pub fn candidate_count(&self) -> usize {
        self.candidate_count
    }
    pub fn inside_side_observed(&self) -> bool {
        self.inside_side_observed
    }
    pub fn outside_side_observed(&self) -> bool {
        self.outside_side_observed
    }
    pub fn evidence_source(&self) -> BoundaryEvidenceSource {
        self.evidence_source
    }
}
#[derive(Debug, Eq, PartialEq)]
pub enum BoundaryLocalEvidence {
    Proved {
        sign_judgment: RegionSignJudgment,
        transition_report: MembershipTransitionReport,
        witness: BoundaryTransitionWitness,
    },
    Undetermined {
        reason: BoundaryUndeterminedReason,
        sign_judgment: Option<RegionSignJudgment>,
        transition_report: Option<MembershipTransitionReport>,
    },
}
#[derive(Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum BoundaryStratumDimension {
    Dimension(i64),
    UndeterminedDimension { reason: BoundaryUndeterminedReason },
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IntersectionEvidenceKind {
    Empty,
    Point,
    FinitePoints,
    Curve,
    CurveBranches,
    Coincident,
    Undetermined,
}
#[derive(Debug, Eq, PartialEq)]
pub struct BoundaryBranchEvidence {
    branch_id: String,
    chart_id: String,
    support_id: String,
    contact_order: usize,
    contact_kind: ContactOrderKind,
}
impl BoundaryBranchEvidence {
    pub fn branch_id(&self) -> &str {
        &self.branch_id
    }
    pub fn chart_id(&self) -> &str {
        &self.chart_id
    }
    pub fn support_id(&self) -> &str {
        &self.support_id
    }
    pub fn contact_order(&self) -> usize {
        self.contact_order
    }
    pub fn contact_kind(&self) -> ContactOrderKind {
        self.contact_kind
    }
}
#[derive(Debug, Eq, PartialEq)]
pub struct BoundaryRootEvidence {
    lower: RealAlgebraic,
    upper: RealAlgebraic,
    sign_change: bool,
    contact_order: usize,
    contact_kind: ContactOrderKind,
    evidence: RootEvidence,
}
impl BoundaryRootEvidence {
    pub fn lower(&self) -> &RealAlgebraic {
        &self.lower
    }
    pub fn upper(&self) -> &RealAlgebraic {
        &self.upper
    }
    pub fn sign_change(&self) -> bool {
        self.sign_change
    }
    pub fn contact_order(&self) -> usize {
        self.contact_order
    }
    pub fn contact_kind(&self) -> ContactOrderKind {
        self.contact_kind
    }
    pub fn evidence(&self) -> &RootEvidence {
        &self.evidence
    }
}
#[derive(Debug, Eq, PartialEq)]
pub struct IntersectionEvidenceRef {
    source_index: usize,
    kind: IntersectionEvidenceKind,
    branch_evidence: Vec<BoundaryBranchEvidence>,
    root_evidence: Vec<BoundaryRootEvidence>,
    support_relation: SupportRelation,
    rank_evidence: RankEvidence,
    residual_function: ResidualFunction,
}
impl IntersectionEvidenceRef {
    pub fn source_index(&self) -> usize {
        self.source_index
    }
    pub fn kind(&self) -> IntersectionEvidenceKind {
        self.kind
    }
    pub fn branch_evidence(&self) -> &[BoundaryBranchEvidence] {
        &self.branch_evidence
    }
    pub fn root_evidence(&self) -> &[BoundaryRootEvidence] {
        &self.root_evidence
    }
    pub fn support_relation(&self) -> &SupportRelation {
        &self.support_relation
    }
    pub fn rank_evidence(&self) -> &RankEvidence {
        &self.rank_evidence
    }
    pub fn residual_function(&self) -> &ResidualFunction {
        &self.residual_function
    }
}
pub type BoundaryCandidateSet = Vec<BoundaryCandidate>;
pub type BoundaryStratumSet = Vec<BoundaryStratum>;
pub type StringSet = Vec<String>;
pub type IntersectionEvidenceSet = Vec<IntersectionEvidenceRef>;
#[derive(Debug, Eq, PartialEq)]
pub struct BoundaryCandidate {
    locus: BoundaryAtomLocus,
    active_atom_ids: StringSet,
    local_evidence: BoundaryLocalEvidence,
    intersection_evidence: Option<IntersectionEvidenceRef>,
}
impl BoundaryCandidate {
    pub fn locus(&self) -> &BoundaryAtomLocus {
        &self.locus
    }
    pub fn active_atom_ids(&self) -> &[String] {
        &self.active_atom_ids
    }
    pub fn local_evidence(&self) -> &BoundaryLocalEvidence {
        &self.local_evidence
    }
    pub fn intersection_evidence(&self) -> Option<&IntersectionEvidenceRef> {
        self.intersection_evidence.as_ref()
    }
}
#[derive(Debug, Eq, PartialEq)]
pub struct BoundaryCoverageEvidence {
    supplied_candidate_count: usize,
    retained_candidate_count: usize,
    omitted_candidate_count: usize,
    status: BoundaryEvidenceStatus,
}
impl BoundaryCoverageEvidence {
    pub fn supplied_candidate_count(&self) -> usize {
        self.supplied_candidate_count
    }
    pub fn retained_candidate_count(&self) -> usize {
        self.retained_candidate_count
    }
    pub fn omitted_candidate_count(&self) -> usize {
        self.omitted_candidate_count
    }
    pub fn status(&self) -> &BoundaryEvidenceStatus {
        &self.status
    }
}
#[derive(Debug, Eq, PartialEq)]
pub struct AlgebraicBoundary {
    ambient_dimension: i64,
    source_region: AlgebraicRegion,
    candidate_loci: BoundaryCandidateSet,
    retained_loci: BoundaryCandidateSet,
    coverage_evidence: BoundaryCoverageEvidence,
}
impl AlgebraicBoundary {
    pub fn ambient_dimension(&self) -> i64 {
        self.ambient_dimension
    }
    pub fn source_region(&self) -> &AlgebraicRegion {
        &self.source_region
    }
    pub fn candidate_loci(&self) -> &[BoundaryCandidate] {
        &self.candidate_loci
    }
    pub fn retained_loci(&self) -> &[BoundaryCandidate] {
        &self.retained_loci
    }
    pub fn coverage_evidence(&self) -> &BoundaryCoverageEvidence {
        &self.coverage_evidence
    }
}
#[derive(Debug, Eq, PartialEq)]
pub struct StratificationEvidence {
    dimension_partition_status: BoundaryEvidenceStatus,
    disjointness_status: BoundaryEvidenceStatus,
    source_evidence_count: usize,
}
impl StratificationEvidence {
    pub fn dimension_partition_status(&self) -> &BoundaryEvidenceStatus {
        &self.dimension_partition_status
    }
    pub fn disjointness_status(&self) -> &BoundaryEvidenceStatus {
        &self.disjointness_status
    }
    pub fn source_evidence_count(&self) -> usize {
        self.source_evidence_count
    }
}
#[derive(Debug, Eq, PartialEq)]
pub struct BoundaryStratum {
    stratum_id: String,
    dimension: BoundaryStratumDimension,
    active_atom_ids: StringSet,
    candidates: BoundaryCandidateSet,
    local_evidence: BoundaryLocalEvidence,
    source_intersections: IntersectionEvidenceSet,
}
impl BoundaryStratum {
    pub fn stratum_id(&self) -> &str {
        &self.stratum_id
    }
    pub fn dimension(&self) -> &BoundaryStratumDimension {
        &self.dimension
    }
    pub fn active_atom_ids(&self) -> &[String] {
        &self.active_atom_ids
    }
    pub fn candidates(&self) -> &[BoundaryCandidate] {
        &self.candidates
    }
    pub fn local_evidence(&self) -> &BoundaryLocalEvidence {
        &self.local_evidence
    }
    pub fn source_intersections(&self) -> &[IntersectionEvidenceRef] {
        &self.source_intersections
    }
}
#[derive(Debug, Eq, PartialEq)]
#[must_use]
pub struct BoundaryStratification {
    ambient_dimension: i64,
    boundary: AlgebraicBoundary,
    strata: BoundaryStratumSet,
    stratification_evidence: StratificationEvidence,
}
impl BoundaryStratification {
    pub fn ambient_dimension(&self) -> i64 {
        self.ambient_dimension
    }
    pub fn boundary(&self) -> &AlgebraicBoundary {
        &self.boundary
    }
    pub fn strata(&self) -> &[BoundaryStratum] {
        &self.strata
    }
    pub fn stratification_evidence(&self) -> &StratificationEvidence {
        &self.stratification_evidence
    }
}
#[derive(Debug, Eq, PartialEq)]
pub struct BoundaryRestrictionChart {
    chart_id: String,
    parameter_enclosure: Enclosure,
    coordinate_x: Polynomial,
    coordinate_y: Polynomial,
    coordinate_z: Polynomial,
}
impl BoundaryRestrictionChart {
    pub fn try_new(
        chart_id: impl Into<String>,
        parameter_enclosure: Enclosure,
        coordinate_x: Polynomial,
        coordinate_y: Polynomial,
        coordinate_z: Polynomial,
    ) -> Result<Self, GeometryFailure> {
        let chart_id = chart_id.into();
        if chart_id.is_empty() {
            return Err(GeometryFailure::MalformedRegionFormula);
        }
        Ok(Self {
            chart_id,
            parameter_enclosure,
            coordinate_x,
            coordinate_y,
            coordinate_z,
        })
    }
    pub fn chart_id(&self) -> &str {
        &self.chart_id
    }
    pub fn parameter_enclosure(&self) -> &Enclosure {
        &self.parameter_enclosure
    }
    pub fn coordinate_x(&self) -> &Polynomial {
        &self.coordinate_x
    }
    pub fn coordinate_y(&self) -> &Polynomial {
        &self.coordinate_y
    }
    pub fn coordinate_z(&self) -> &Polynomial {
        &self.coordinate_z
    }
}
#[derive(Debug, Eq, PartialEq)]
pub struct BoundarySideSample {
    parameter: Dyadic,
}
impl BoundarySideSample {
    pub fn new(parameter: Dyadic) -> Self {
        Self { parameter }
    }
    pub fn parameter(&self) -> &Dyadic {
        &self.parameter
    }
}
#[derive(Debug, Eq, PartialEq)]
pub struct BoundaryIntersectionSelector {
    intersection_index: usize,
    branch_id: Option<String>,
}
impl BoundaryIntersectionSelector {
    pub fn try_new(
        intersection_index: usize,
        branch_id: Option<String>,
    ) -> Result<Self, GeometryFailure> {
        if branch_id.as_ref().is_some_and(String::is_empty) {
            return Err(GeometryFailure::MalformedRegionFormula);
        }
        Ok(Self {
            intersection_index,
            branch_id,
        })
    }
    pub fn intersection_index(&self) -> usize {
        self.intersection_index
    }
    pub fn branch_id(&self) -> Option<&str> {
        self.branch_id.as_deref()
    }
}
#[derive(Debug, Eq, PartialEq)]
pub struct BoundaryCandidateSeed {
    atom_id: String,
    chart: BoundaryRestrictionChart,
    candidate_parameters: Vec<Dyadic>,
    side_samples: Vec<BoundarySideSample>,
    intersection: Option<BoundaryIntersectionSelector>,
}
impl BoundaryCandidateSeed {
    pub fn try_new(
        atom_id: impl Into<String>,
        chart: BoundaryRestrictionChart,
        mut candidate_parameters: Vec<Dyadic>,
        mut side_samples: Vec<BoundarySideSample>,
        intersection: Option<BoundaryIntersectionSelector>,
    ) -> Result<Self, GeometryFailure> {
        let atom_id = atom_id.into();
        if candidate_parameters.len() > MAX_BOUNDARY_CANDIDATE_PARAMETERS {
            return Err(GeometryFailure::BoundaryResourceLimit {
                resource: BoundaryResource::CandidateParameters,
                requested: candidate_parameters.len(),
                maximum: MAX_BOUNDARY_CANDIDATE_PARAMETERS,
            });
        }
        if side_samples.len() > MAX_BOUNDARY_SIDE_SAMPLES {
            return Err(GeometryFailure::BoundaryResourceLimit {
                resource: BoundaryResource::SideSamples,
                requested: side_samples.len(),
                maximum: MAX_BOUNDARY_SIDE_SAMPLES,
            });
        }
        if atom_id.is_empty()
            || candidate_parameters.is_empty()
            || candidate_parameters
                .iter()
                .any(|p| !chart.parameter_enclosure.contains(p))
            || side_samples
                .iter()
                .any(|p| !chart.parameter_enclosure.contains(&p.parameter))
        {
            return Err(GeometryFailure::MalformedRegionFormula);
        }
        candidate_parameters.sort();
        side_samples.sort_by(|left, right| left.parameter.cmp(&right.parameter));
        if candidate_parameters
            .windows(2)
            .any(|pair| pair[0] == pair[1])
            || side_samples
                .windows(2)
                .any(|pair| pair[0].parameter == pair[1].parameter)
        {
            return Err(GeometryFailure::MalformedRegionFormula);
        }
        Ok(Self {
            atom_id,
            chart,
            candidate_parameters,
            side_samples,
            intersection,
        })
    }
    pub fn atom_id(&self) -> &str {
        &self.atom_id
    }
    pub fn chart(&self) -> &BoundaryRestrictionChart {
        &self.chart
    }
    pub fn candidate_parameters(&self) -> &[Dyadic] {
        &self.candidate_parameters
    }
    pub fn side_samples(&self) -> &[BoundarySideSample] {
        &self.side_samples
    }
    pub fn intersection(&self) -> Option<&BoundaryIntersectionSelector> {
        self.intersection.as_ref()
    }
}

fn compare_candidate_seeds(
    left: &BoundaryCandidateSeed,
    right: &BoundaryCandidateSeed,
) -> Ordering {
    left.atom_id
        .cmp(&right.atom_id)
        .then_with(|| left.chart.chart_id.cmp(&right.chart.chart_id))
        .then_with(|| left.candidate_parameters.cmp(&right.candidate_parameters))
        .then_with(|| {
            left.side_samples
                .iter()
                .map(|sample| &sample.parameter)
                .cmp(right.side_samples.iter().map(|sample| &sample.parameter))
        })
        .then_with(|| match (&left.intersection, &right.intersection) {
            (Some(left), Some(right)) => left
                .intersection_index
                .cmp(&right.intersection_index)
                .then_with(|| left.branch_id.cmp(&right.branch_id)),
            (None, Some(_)) => Ordering::Less,
            (Some(_), None) => Ordering::Greater,
            (None, None) => Ordering::Equal,
        })
}
#[derive(Debug, Eq, PartialEq)]
pub struct BoundaryDerivationRequest {
    pub region: AlgebraicRegion,
    pub intersections: Vec<IntersectionSet>,
    pub candidate_loci: Vec<BoundaryCandidateSeed>,
    pub max_order: DerivativeOrder,
}

struct RestrictedAtomFunction {
    numerator: Polynomial,
    denominator: Option<Polynomial>,
}

pub fn derive_boundary(
    request: BoundaryDerivationRequest,
) -> Result<BoundaryTypes, GeometryFailure> {
    let BoundaryDerivationRequest {
        region,
        intersections,
        mut candidate_loci,
        max_order,
    } = request;
    if candidate_loci.len() > MAX_BOUNDARY_CANDIDATE_LOCI {
        return Err(GeometryFailure::BoundaryResourceLimit {
            resource: BoundaryResource::CandidateLoci,
            requested: candidate_loci.len(),
            maximum: MAX_BOUNDARY_CANDIDATE_LOCI,
        });
    }
    if intersections.len() > MAX_BOUNDARY_INTERSECTIONS {
        return Err(GeometryFailure::BoundaryResourceLimit {
            resource: BoundaryResource::Intersections,
            requested: intersections.len(),
            maximum: MAX_BOUNDARY_INTERSECTIONS,
        });
    }
    candidate_loci.sort_by(compare_candidate_seeds);
    let ambient = region.ambient_dimension();
    let source = clone_region(&region)?;
    let mut all = Vec::with_capacity(candidate_loci.len());
    let mut retained = Vec::with_capacity(candidate_loci.len());
    for seed in candidate_loci {
        let atom = region
            .canonical_atoms()
            .binary_search_by(|atom| atom.atom_id().cmp(seed.atom_id()))
            .map(|index| &region.canonical_atoms()[index])
            .map_err(|_| GeometryFailure::MalformedRegionFormula)?;
        let evidence = match seed.intersection() {
            Some(selector) => Some(snapshot_selector(&intersections, selector)?),
            None => None,
        };
        let restricted = restrict_atom_to_chart(atom, seed.chart())?;
        let sign_request = make_calculus_judgment_request(
            restricted.numerator,
            seed.chart(),
            seed.candidate_parameters(),
            max_order,
        )?;
        let sign = expose_region_sign(&sign_request).map_err(calculus_failure)?;
        let report = expose_membership_transitions(&sign_request).map_err(calculus_failure)?;
        let denominator_valid = if let Some(denominator) = restricted.denominator {
            let request = make_calculus_judgment_request(
                denominator,
                seed.chart(),
                seed.candidate_parameters(),
                max_order,
            )?;
            matches!(
                expose_region_sign(&request)
                    .map_err(calculus_failure)?
                    .sign(),
                RegionSign::ConstantPositive | RegionSign::ConstantNegative
            )
        } else {
            true
        };
        let mut candidates_zero = true;
        let mut active = Vec::new();
        for parameter in seed.candidate_parameters() {
            let result = membership_at(&region, seed.chart(), parameter)?;
            let selected = atom_relation(&result.atom_signs, seed.atom_id())?;
            candidates_zero &= selected == SignRelation::Zero;
            for value in result.atom_signs {
                if value.relation == SignRelation::Zero {
                    active.push(value.atom_id)
                }
            }
        }
        active.sort();
        active.dedup();
        let mut inside = None;
        let mut outside = None;
        for sample in seed.side_samples() {
            let result = membership_at(&region, seed.chart(), sample.parameter())?;
            let relation = atom_relation(&result.atom_signs, seed.atom_id())?;
            match result.classification {
                MembershipClassification::Inside => {
                    if inside.is_none() {
                        inside = Some(relation)
                    }
                }
                MembershipClassification::Outside => {
                    if outside.is_none() {
                        outside = Some(relation)
                    }
                }
                MembershipClassification::Boundary => {}
            }
        }
        let opposite = matches!(
            (inside, outside),
            (Some(SignRelation::Positive), Some(SignRelation::Negative))
                | (Some(SignRelation::Negative), Some(SignRelation::Positive))
        );
        let transitioned = report
            .loci()
            .iter()
            .any(|l| matches!(l.kind(), neco_calculus::MembershipLocusKind::Transition));
        let proved = candidates_zero
            && denominator_valid
            && inside.is_some()
            && outside.is_some()
            && opposite
            && transitioned
            && intersection_proves(evidence.as_ref());
        let local = if proved {
            BoundaryLocalEvidence::Proved {
                sign_judgment: sign,
                transition_report: report,
                witness: BoundaryTransitionWitness {
                    atom_id: String::from(seed.atom_id()),
                    candidate_count: seed.candidate_parameters().len(),
                    inside_side_observed: inside.is_some(),
                    outside_side_observed: outside.is_some(),
                    evidence_source:
                        BoundaryEvidenceSource::ExactChartLocalCalculusAndRegionMembership,
                },
            }
        } else {
            let reason = if !denominator_valid {
                BoundaryUndeterminedReason::RestrictedRationalDenominatorNotProvedNonzero
            } else {
                BoundaryUndeterminedReason::FormulaSensitiveTransitionNotProved
            };
            BoundaryLocalEvidence::Undetermined {
                reason,
                sign_judgment: Some(sign),
                transition_report: Some(report),
            }
        };
        let candidate = BoundaryCandidate {
            locus: BoundaryAtomLocus {
                atom_id: String::from(seed.atom_id()),
                zero_relation: SignRelation::Zero,
                ambient_dimension: ambient,
            },
            active_atom_ids: active,
            local_evidence: local,
            intersection_evidence: evidence,
        };
        let keep = !matches!(
            candidate.local_evidence,
            BoundaryLocalEvidence::Undetermined { .. }
        ) || candidates_zero;
        if keep {
            retained.push(clone_candidate(&candidate)?)
        }
        all.push(candidate);
    }
    let coverage = BoundaryCoverageEvidence {
        supplied_candidate_count: all.len(),
        retained_candidate_count: retained.len(),
        omitted_candidate_count: all.len() - retained.len(),
        status: if all.len() == retained.len() {
            BoundaryEvidenceStatus::Proved
        } else {
            BoundaryEvidenceStatus::Undetermined {
                reason: BoundaryUndeterminedReason::FormulaInsensitiveCandidatesOmitted,
            }
        },
    };
    let boundary = AlgebraicBoundary {
        ambient_dimension: ambient,
        source_region: source,
        candidate_loci: all,
        retained_loci: retained,
        coverage_evidence: coverage,
    };
    let strata = make_strata(&boundary)?;
    let strat_boundary = clone_boundary(&boundary)?;
    Ok(BoundaryTypes {
        algebraic_boundary: boundary,
        stratification: BoundaryStratification {
            ambient_dimension: ambient,
            boundary: strat_boundary,
            strata,
            stratification_evidence: StratificationEvidence {
                dimension_partition_status: BoundaryEvidenceStatus::Undetermined {
                    reason: BoundaryUndeterminedReason::StrataEvidenceLocal,
                },
                disjointness_status: BoundaryEvidenceStatus::Undetermined {
                    reason: BoundaryUndeterminedReason::DisjointnessNotEstablished,
                },
                source_evidence_count: intersections.len(),
            },
        },
    })
}

fn restrict_atom_to_chart(
    atom: &CanonicalAtom,
    chart: &BoundaryRestrictionChart,
) -> Result<RestrictedAtomFunction, GeometryFailure> {
    match atom.function() {
        RegionFunction::Polynomial(p) => Ok(RestrictedAtomFunction {
            numerator: compose(p, chart)?,
            denominator: None,
        }),
        RegionFunction::RationalFunction(r) => Ok(RestrictedAtomFunction {
            numerator: compose(r.numerator(), chart)?,
            denominator: Some(compose(r.denominator(), chart)?),
        }),
    }
}
fn make_calculus_judgment_request(
    target: Polynomial,
    chart: &BoundaryRestrictionChart,
    candidates: &[Dyadic],
    max_order: DerivativeOrder,
) -> Result<CalculusMembershipJudgmentRequest, GeometryFailure> {
    CalculusMembershipJudgmentRequest::new(
        target,
        clone_enclosure(chart.parameter_enclosure())?,
        clone_dyadics(candidates)?,
        max_order,
        BOUNDARY_ZERO_EXCLUSION_PRECISION_BITS,
    )
    .map_err(|error| calculus_failure(CalculusFailure::Root(error)))
}
fn compose(
    p: &RegionPolynomial3,
    chart: &BoundaryRestrictionChart,
) -> Result<Polynomial, GeometryFailure> {
    let mut acc = zero_poly()?;
    for term in p.terms() {
        let mut value = constant(term.coefficient())?;
        value = mul_poly(&value, &power(chart.coordinate_x(), term.exponent_x())?)?;
        value = mul_poly(&value, &power(chart.coordinate_y(), term.exponent_y())?)?;
        value = mul_poly(&value, &power(chart.coordinate_z(), term.exponent_z())?)?;
        acc = add_poly(&acc, &value)?;
    }
    Ok(acc)
}
fn zero_poly() -> Result<Polynomial, GeometryFailure> {
    Polynomial::zero().map_err(poly_failure)
}
fn constant(v: &RealAlgebraic) -> Result<Polynomial, GeometryFailure> {
    Polynomial::constant(v.try_clone().map_err(GeometryFailure::AlgebraFailure)?)
        .map_err(poly_failure)
}
fn add_poly(a: &Polynomial, b: &Polynomial) -> Result<Polynomial, GeometryFailure> {
    let n = a.coefficients().len().max(b.coefficients().len());
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let x = coefficient(a, i)?;
        let y = coefficient(b, i)?;
        out.push(x.add(&y).map_err(GeometryFailure::AlgebraFailure)?)
    }
    Polynomial::from_coefficients(out).map_err(poly_failure)
}
fn mul_poly(a: &Polynomial, b: &Polynomial) -> Result<Polynomial, GeometryFailure> {
    let mut out = Vec::with_capacity(a.coefficients().len() + b.coefficients().len() - 1);
    for _ in 0..a.coefficients().len() + b.coefficients().len() - 1 {
        out.push(zero_algebraic()?)
    }
    for (i, x) in a.coefficients().iter().enumerate() {
        for (j, y) in b.coefficients().iter().enumerate() {
            let product = x.mul(y).map_err(GeometryFailure::AlgebraFailure)?;
            out[i + j] = out[i + j]
                .add(&product)
                .map_err(GeometryFailure::AlgebraFailure)?;
        }
    }
    Polynomial::from_coefficients(out).map_err(poly_failure)
}
fn power(p: &Polynomial, exponent: u32) -> Result<Polynomial, GeometryFailure> {
    let mut base = clone_polynomial(p)?;
    let mut result = constant_one()?;
    let mut n = exponent;
    while n > 0 {
        if n & 1 == 1 {
            result = mul_poly(&result, &base)?
        }
        n >>= 1;
        if n > 0 {
            base = mul_poly(&base, &base)?
        }
    }
    Ok(result)
}
fn coefficient(p: &Polynomial, index: usize) -> Result<RealAlgebraic, GeometryFailure> {
    match p.coefficients().get(index) {
        Some(x) => x.try_clone().map_err(GeometryFailure::AlgebraFailure),
        None => zero_algebraic(),
    }
}
fn zero_algebraic() -> Result<RealAlgebraic, GeometryFailure> {
    RealAlgebraic::from_integer(BigInt::zero()).map_err(GeometryFailure::AlgebraFailure)
}
fn constant_one() -> Result<Polynomial, GeometryFailure> {
    Polynomial::constant(
        RealAlgebraic::from_integer(BigInt::one().map_err(bigint_failure)?)
            .map_err(GeometryFailure::AlgebraFailure)?,
    )
    .map_err(poly_failure)
}
fn membership_at(
    region: &AlgebraicRegion,
    chart: &BoundaryRestrictionChart,
    p: &Dyadic,
) -> Result<crate::RegionMembershipResult, GeometryFailure> {
    let t = dyadic_to_real(p)?;
    let x = chart.coordinate_x().evaluate(&t).map_err(poly_failure)?;
    let y = chart.coordinate_y().evaluate(&t).map_err(poly_failure)?;
    let point = if region.ambient_dimension() == 2 {
        RegionPoint::Planar(Point2 { x, y })
    } else {
        RegionPoint::Spatial(Point3 {
            x,
            y,
            z: chart.coordinate_z().evaluate(&t).map_err(poly_failure)?,
        })
    };
    region.membership(RegionMembershipRequest {
        region: clone_region(region)?,
        point,
    })
}
fn atom_relation(
    values: &[crate::AtomSignResult],
    id: &str,
) -> Result<SignRelation, GeometryFailure> {
    values
        .iter()
        .find(|v| v.atom_id == id)
        .map(|v| v.relation)
        .ok_or(GeometryFailure::MalformedRegionFormula)
}
fn snapshot_selector(
    sets: &[IntersectionSet],
    selector: &BoundaryIntersectionSelector,
) -> Result<IntersectionEvidenceRef, GeometryFailure> {
    let set = sets
        .get(selector.intersection_index())
        .ok_or(GeometryFailure::MalformedRegionFormula)?;
    if let Some(id) = selector.branch_id() {
        let matches = match set.component_view() {
            crate::IntersectionComponentView::Components(mut entries) => entries.any(|entry| {
                entry
                    .evidence()
                    .branch_identity()
                    .is_some_and(|identity| identity.branch_id() == id)
            }),
            _ => false,
        };
        if !matches {
            return Err(GeometryFailure::MalformedRegionFormula);
        }
    }
    snapshot(set, selector.intersection_index())
}
fn snapshot(
    set: &IntersectionSet,
    index: usize,
) -> Result<IntersectionEvidenceRef, GeometryFailure> {
    let kind = match set.kind() {
        crate::IntersectionKind::Empty => IntersectionEvidenceKind::Empty,
        crate::IntersectionKind::Point => IntersectionEvidenceKind::Point,
        crate::IntersectionKind::FinitePoints => IntersectionEvidenceKind::FinitePoints,
        crate::IntersectionKind::Curve => IntersectionEvidenceKind::Curve,
        crate::IntersectionKind::CurveBranches => IntersectionEvidenceKind::CurveBranches,
        crate::IntersectionKind::Coincident => IntersectionEvidenceKind::Coincident,
        crate::IntersectionKind::Undetermined => IntersectionEvidenceKind::Undetermined,
    };
    let witness = set.witness();
    let branches = match set.component_view() {
        crate::IntersectionComponentView::Components(entries) => entries
            .filter_map(|entry| {
                let evidence = entry.evidence();
                Some(branch(
                    evidence.branch_identity()?,
                    evidence.contact_order()?,
                ))
            })
            .collect(),
        _ => Vec::new(),
    };
    let roots = witness
        .root_isolation()
        .brackets()
        .iter()
        .map(|b| {
            Ok::<BoundaryRootEvidence, GeometryFailure>(BoundaryRootEvidence {
                lower: b
                    .lower()
                    .try_clone()
                    .map_err(GeometryFailure::AlgebraFailure)?,
                upper: b
                    .upper()
                    .try_clone()
                    .map_err(GeometryFailure::AlgebraFailure)?,
                sign_change: b.sign_change(),
                contact_order: b.contact_order().order(),
                contact_kind: b.contact_order().kind(),
                evidence: clone_root_evidence(b.evidence())?,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let c = witness.constraint();
    Ok(IntersectionEvidenceRef {
        source_index: index,
        kind,
        branch_evidence: branches,
        root_evidence: roots,
        support_relation: clone_support_relation(c.support_relation()),
        rank_evidence: clone_rank_evidence(c.rank_evidence()),
        residual_function: clone_residual_function(c.residual_function()),
    })
}
fn branch(b: &crate::BranchIdentity, contact: &crate::ContactOrder) -> BoundaryBranchEvidence {
    BoundaryBranchEvidence {
        branch_id: String::from(b.branch_id()),
        chart_id: String::from(b.chart_id()),
        support_id: String::from(b.support_id()),
        contact_order: contact.order(),
        contact_kind: contact.kind(),
    }
}
fn intersection_proves(evidence: Option<&IntersectionEvidenceRef>) -> bool {
    !matches!(
        evidence.map(IntersectionEvidenceRef::kind),
        Some(IntersectionEvidenceKind::Coincident | IntersectionEvidenceKind::Undetermined)
    )
}
fn dimension(c: &BoundaryCandidate) -> BoundaryStratumDimension {
    match c
        .intersection_evidence
        .as_ref()
        .map(IntersectionEvidenceRef::kind)
    {
        Some(IntersectionEvidenceKind::Point | IntersectionEvidenceKind::FinitePoints) => {
            BoundaryStratumDimension::Dimension(0)
        }
        Some(IntersectionEvidenceKind::Curve | IntersectionEvidenceKind::CurveBranches) => {
            BoundaryStratumDimension::Dimension(1)
        }
        Some(IntersectionEvidenceKind::Coincident | IntersectionEvidenceKind::Undetermined) => {
            BoundaryStratumDimension::UndeterminedDimension {
                reason: BoundaryUndeterminedReason::IntersectionEvidenceUnresolved,
            }
        }
        Some(IntersectionEvidenceKind::Empty) | None => {
            if c.active_atom_ids.len() == 1 {
                BoundaryStratumDimension::Dimension(c.locus.ambient_dimension - 1)
            } else {
                BoundaryStratumDimension::UndeterminedDimension {
                    reason: BoundaryUndeterminedReason::ActiveAtomDimensionEvidenceInsufficient,
                }
            }
        }
    }
}
fn make_strata(boundary: &AlgebraicBoundary) -> Result<Vec<BoundaryStratum>, GeometryFailure> {
    let mut strata = Vec::new();
    for (i, c) in boundary.retained_loci.iter().enumerate() {
        strata.push(BoundaryStratum {
            stratum_id: format!("boundary-stratum-{i}"),
            dimension: dimension(c),
            active_atom_ids: c.active_atom_ids.clone(),
            candidates: vec![clone_candidate(c)?],
            local_evidence: clone_local(&c.local_evidence)?,
            source_intersections: c
                .intersection_evidence
                .as_ref()
                .map(clone_intersection)
                .transpose()?
                .into_iter()
                .collect(),
        })
    }
    Ok(strata)
}
fn clone_region(r: &AlgebraicRegion) -> Result<AlgebraicRegion, GeometryFailure> {
    let atoms = r
        .canonical_atoms()
        .iter()
        .map(clone_atom)
        .collect::<Result<Vec<_>, _>>()?;
    AlgebraicRegion::try_new(
        r.ambient_dimension(),
        atoms,
        r.normalized_region_formula().clone(),
        r.region_kind(),
    )
}
fn clone_atom(a: &CanonicalAtom) -> Result<CanonicalAtom, GeometryFailure> {
    CanonicalAtom::try_new(a.atom_id(), a.function().try_clone()?, a.relation())
}
fn clone_candidate(c: &BoundaryCandidate) -> Result<BoundaryCandidate, GeometryFailure> {
    Ok(BoundaryCandidate {
        locus: BoundaryAtomLocus {
            atom_id: c.locus.atom_id.clone(),
            zero_relation: c.locus.zero_relation,
            ambient_dimension: c.locus.ambient_dimension,
        },
        active_atom_ids: c.active_atom_ids.clone(),
        local_evidence: clone_local(&c.local_evidence)?,
        intersection_evidence: c
            .intersection_evidence
            .as_ref()
            .map(clone_intersection)
            .transpose()?,
    })
}
fn clone_local(e: &BoundaryLocalEvidence) -> Result<BoundaryLocalEvidence, GeometryFailure> {
    Ok(match e {
        BoundaryLocalEvidence::Proved {
            sign_judgment,
            transition_report,
            witness,
        } => BoundaryLocalEvidence::Proved {
            sign_judgment: sign_judgment.clone(),
            transition_report: clone_report(transition_report),
            witness: BoundaryTransitionWitness {
                atom_id: witness.atom_id.clone(),
                candidate_count: witness.candidate_count,
                inside_side_observed: witness.inside_side_observed,
                outside_side_observed: witness.outside_side_observed,
                evidence_source: witness.evidence_source,
            },
        },
        BoundaryLocalEvidence::Undetermined {
            reason,
            sign_judgment,
            transition_report,
        } => BoundaryLocalEvidence::Undetermined {
            reason: *reason,
            sign_judgment: sign_judgment.clone(),
            transition_report: transition_report.as_ref().map(clone_report),
        },
    })
}
fn clone_report(r: &MembershipTransitionReport) -> MembershipTransitionReport {
    r.clone()
}
fn clone_intersection(
    e: &IntersectionEvidenceRef,
) -> Result<IntersectionEvidenceRef, GeometryFailure> {
    Ok(IntersectionEvidenceRef {
        source_index: e.source_index,
        kind: e.kind,
        branch_evidence: e
            .branch_evidence
            .iter()
            .map(|b| BoundaryBranchEvidence {
                branch_id: b.branch_id.clone(),
                chart_id: b.chart_id.clone(),
                support_id: b.support_id.clone(),
                contact_order: b.contact_order,
                contact_kind: b.contact_kind,
            })
            .collect(),
        root_evidence: e
            .root_evidence
            .iter()
            .map(|r| {
                Ok::<BoundaryRootEvidence, GeometryFailure>(BoundaryRootEvidence {
                    lower: r
                        .lower
                        .try_clone()
                        .map_err(GeometryFailure::AlgebraFailure)?,
                    upper: r
                        .upper
                        .try_clone()
                        .map_err(GeometryFailure::AlgebraFailure)?,
                    sign_change: r.sign_change,
                    contact_order: r.contact_order,
                    contact_kind: r.contact_kind,
                    evidence: clone_root_evidence(&r.evidence)?,
                })
            })
            .collect::<Result<Vec<_>, _>>()?,
        support_relation: clone_support_relation(&e.support_relation),
        rank_evidence: clone_rank_evidence(&e.rank_evidence),
        residual_function: clone_residual_function(&e.residual_function),
    })
}
fn clone_boundary(b: &AlgebraicBoundary) -> Result<AlgebraicBoundary, GeometryFailure> {
    Ok(AlgebraicBoundary {
        ambient_dimension: b.ambient_dimension,
        source_region: clone_region(&b.source_region)?,
        candidate_loci: b
            .candidate_loci
            .iter()
            .map(clone_candidate)
            .collect::<Result<Vec<_>, _>>()?,
        retained_loci: b
            .retained_loci
            .iter()
            .map(clone_candidate)
            .collect::<Result<Vec<_>, _>>()?,
        coverage_evidence: BoundaryCoverageEvidence {
            supplied_candidate_count: b.coverage_evidence.supplied_candidate_count,
            retained_candidate_count: b.coverage_evidence.retained_candidate_count,
            omitted_candidate_count: b.coverage_evidence.omitted_candidate_count,
            status: match &b.coverage_evidence.status {
                BoundaryEvidenceStatus::Proved => BoundaryEvidenceStatus::Proved,
                BoundaryEvidenceStatus::Undetermined { reason } => {
                    BoundaryEvidenceStatus::Undetermined { reason: *reason }
                }
            },
        },
    })
}
fn clone_enclosure(e: &Enclosure) -> Result<Enclosure, GeometryFailure> {
    Enclosure::new(
        e.lower().try_clone().map_err(bigint_failure)?,
        e.upper().try_clone().map_err(bigint_failure)?,
    )
    .map_err(|x| GeometryFailure::CalculusFailure {
        error: CalculusFailure::Interval(x),
    })
}
fn clone_dyadics(v: &[Dyadic]) -> Result<Vec<Dyadic>, GeometryFailure> {
    v.iter()
        .map(|x| x.try_clone().map_err(bigint_failure))
        .collect()
}
fn clone_polynomial(p: &Polynomial) -> Result<Polynomial, GeometryFailure> {
    p.try_clone().map_err(poly_failure)
}
fn dyadic_to_real(v: &Dyadic) -> Result<RealAlgebraic, GeometryFailure> {
    if v.is_zero() {
        return zero_algebraic();
    }
    let denominator = BigUint::one()
        .map_err(bigint_failure)?
        .shl_bits(v.exponent() as usize)
        .map_err(bigint_failure)?;
    let rational = RawRational::new(
        v.integer().try_clone().map_err(bigint_failure)?,
        denominator,
    )
    .reduce()
    .map_err(bigint_failure)?
    .into_reduced();
    RealAlgebraic::from_reduced_rational(&rational).map_err(GeometryFailure::AlgebraFailure)
}
fn calculus_failure(error: CalculusFailure) -> GeometryFailure {
    GeometryFailure::CalculusFailure { error }
}
fn poly_failure(error: PolynomialFailure) -> GeometryFailure {
    GeometryFailure::CalculusFailure {
        error: CalculusFailure::Polynomial(error),
    }
}
fn bigint_failure(error: BigintError) -> GeometryFailure {
    GeometryFailure::CalculusFailure {
        error: CalculusFailure::Polynomial(PolynomialFailure::Arithmetic(error)),
    }
}
