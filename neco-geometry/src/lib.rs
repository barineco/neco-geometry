#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]
#![doc = include_str!("../README.md")]

extern crate alloc;

mod boundary;
mod chart_patch;
mod encoding_scalar;
#[cfg(feature = "gpu")]
mod gpu_crossing;

#[cfg(feature = "gpu")]
pub use gpu_crossing::{
    GpuCrossingClassification, GpuCrossingDomain, GpuCrossingFailure, GpuCrossingPrimitive,
    GpuCrossingSeam, GpuCrossingSurface, GpuCrossingTrim, GpuSeamClassification, GPU_CROSSING_WGSL,
    MAX_GPU_TRIM_HOLES, MAX_GPU_TRIM_VERTICES,
};
mod evaluation;
mod intersection;
mod region;
mod seam_encoding;
mod surface_encoding;
mod types;

pub use boundary::{
    derive_boundary, AlgebraicBoundary, BoundaryAtomLocus, BoundaryBranchEvidence,
    BoundaryCandidate, BoundaryCandidateSeed, BoundaryCandidateSet, BoundaryCoverageEvidence,
    BoundaryDerivationRequest, BoundaryEvidenceSource, BoundaryEvidenceStatus,
    BoundaryIntersectionSelector, BoundaryLocalEvidence, BoundaryRestrictionChart,
    BoundaryRootEvidence, BoundarySideSample, BoundaryStratification, BoundaryStratum,
    BoundaryStratumDimension, BoundaryStratumSet, BoundaryTransitionWitness, BoundaryTypes,
    BoundaryUndeterminedReason, IntersectionEvidenceKind, IntersectionEvidenceRef,
    IntersectionEvidenceSet, StratificationEvidence, StringSet,
    BOUNDARY_ZERO_EXCLUSION_PRECISION_BITS, MAX_BOUNDARY_CANDIDATE_LOCI,
    MAX_BOUNDARY_CANDIDATE_PARAMETERS, MAX_BOUNDARY_INTERSECTIONS, MAX_BOUNDARY_SIDE_SAMPLES,
};
pub use chart_patch::{ChartPatch, DiskConstraint};
pub use encoding_scalar::{
    EncodingBounds, EncodingFailure, EncodingFormat, EncodingLocation, EncodingPrecision,
};
pub use evaluation::{
    evaluate_checked_ray2, evaluate_checked_ray3, evaluate_ray2, evaluate_ray3, evaluate_shape,
    evaluate_surface_witness_checked, evaluate_surface_witness_exact,
};
pub use intersection::{
    construct_intersection, cross_checked_ray3, cross_ray3, BranchIdentity, BranchIdentityRef,
    BranchIdentitySet, CheckedRayCrossingHit, CheckedRayCrossingResult, CheckedRayCrossingTarget,
    CheckedRayCrossingTargetSet, CheckedRayCrossingWitness, CheckedTriangleTarget,
    CheckedTrimmedParametricSurfaceTarget, CoincidentSupport, ComponentEvidenceRef, ContactOrder,
    ContactOrderKind, ConvergenceWitness, CrossingContribution, CrossingMultiplicity,
    CrossingParity, CurveBranch, CurveBranchSet, ExactRayCrossingTarget, ExactRayCrossingTargetSet,
    ExactTriangleTarget, ExistenceEvidence, ExistenceWitness, IntersectionComponentIdentity,
    IntersectionComponentIter, IntersectionComponentView, IntersectionComponentViewEntry,
    IntersectionConstraint, IntersectionConstructionRequest, IntersectionConstructionRequestBody,
    IntersectionCurve, IntersectionDomain, IntersectionKind, IntersectionOperand,
    IntersectionPoint, IntersectionPointSet, IntersectionResidual, IntersectionSet,
    IntersectionUndeterminedReason, IntersectionWitness, RankDegeneracyReason, RankEvidence,
    RankUndeterminedReason, RayCrossingHit, RayCrossingLocation, RayCrossingPolicy,
    RayCrossingResult, RayCrossingTargetId, RayCrossingUndeterminedReason, RayCrossingWitness,
    RayNormal, RayOriginContact, ResidualFunction, ResidualSourceId, ResidualUndeterminedReason,
    ResidualValue, RootBracket, RootBracketSet, RootEvidence, RootIsolation, RootIsolationPolicy,
    RootSign, RootUndeterminedReason, SupportEvidence, SupportRelation, SupportUndeterminedReason,
    TrimmedParametricSurfaceTarget, UniquenessEvidence, UniquenessWitness, WitnessMethod,
    WitnessStatus, WitnessUndeterminedReason, MAX_ROOT_PARTITION_CELLS, MAX_ROOT_PARTITION_DEPTH,
};
pub use neco_algnum::RealAlgebraic;
pub use neco_calculus::{
    CalculusFailure, ConvergenceStatus as RootConvergenceStatus, Enclosure,
    ExactExpressionRequirement, NecoImplementationSource, OwnerId, RootRefinementMethod,
};
pub use neco_geometry_foundation::{
    ChartDefinition2, ChartDefinition3, ChartDisplayName, ChartDomainId, CheckedRay2, CheckedRay3,
    Direction2, Direction3, Point2, Point3, Ray2, Ray3, Vector2, Vector3,
};
pub use region::{
    construct_region_types, AlgebraicRegion, AtomSignResult, AtomSignResultSet, CanonicalAtom,
    CanonicalAtomSet, MembershipClassification, MembershipJudgmentRequest,
    MembershipJudgmentResult, NormalizedRegionFormula, NormalizedRegionFormulaSet,
    RationalFunction3, RationalFunctionDomain3, RegionAffineMap3, RegionConstructionRequest,
    RegionFunction, RegionKind, RegionMembershipRequest, RegionMembershipResult, RegionPoint,
    RegionPointSet, RegionPolynomial3, RegionPolynomialTerm, RegionPolynomialTermSet, RegionTypes,
    SignRelation, MAX_REGION_CANONICAL_ATOMS, MAX_REGION_EXPONENT_OPERATIONS,
    MAX_REGION_POLYNOMIAL_TERMS,
};
pub use seam_encoding::{encode_seam_association, EncodedSeamAssociation};
pub use surface_encoding::{
    encode_chart_patch, EncodedCoefficients, EncodedDiskConstraint, EncodedDomain,
    EncodedSurfacePatch, EncodedSurfaceTerm, EncodedTrimRestriction,
};
pub use types::{
    BoundaryResource, CertifiedShapeEvaluation, CheckedRay2ParameterRange, CheckedRayEvaluation2,
    CheckedRayEvaluation3, CoordinateEnclosure2, CoordinateEnclosure3, CurveChart,
    ExactRayEvaluation2, ExactRayEvaluation3, ExactShapeEvaluation, GeometryFailure,
    ImplicitSurfaceDeclaration, MultivariatePolynomial3, PairedSurfaceSeam, ParameterInterval,
    ParametricCurveDeclaration, ParametricSurfaceChart2, ParametricSurfaceDeclaration,
    PolynomialTerm3, PositiveRayParameterRange, Ray2ParameterRange, SeamContact, SeamEnd,
    ShapeCoordinate, ShapeCoordinateEnclosure, ShapeDeclaration, ShapeEvaluation,
    ShapeEvaluationIndeterminacy, ShapeEvaluationInput, ShapeEvaluationRequest, ShapeFamily,
    SurfaceChartIdentity, SurfaceCrossingCandidate, SurfaceEvaluationWitness, SurfaceOrientation,
    SurfaceParameterDomain, SurfaceParameterPoint, SurfaceTrimLoop2, TrimClassification,
    TrimLoopRole, TrimmedSurfaceDeclaration,
};
