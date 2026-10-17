use alloc::boxed::Box;
use alloc::vec::Vec;
use core::cmp::Ordering;

use neco_algnum::{AlgnumError, RealAlgebraic};
use neco_bigint::{BigInt, Dyadic};
use neco_calculus::{
    CalculusFailure, Enclosure, ExactExpressionRequirement, NecoImplementationSource, OwnerId,
    RationalSurfacePolynomials,
};
use neco_geometry_foundation::{
    ChartDefinition2, ChartDefinition3, ChartDisplayName, ChartDomainId, Point2, Point3,
};

#[derive(Debug, Eq, PartialEq)]
pub struct ParameterInterval {
    lower: RealAlgebraic,
    upper: RealAlgebraic,
    lower_closed: bool,
    upper_closed: bool,
}

impl ParameterInterval {
    pub fn new(
        lower: RealAlgebraic,
        upper: RealAlgebraic,
        lower_closed: bool,
        upper_closed: bool,
    ) -> Result<Self, GeometryFailure> {
        let ordering = lower
            .compare(&upper)
            .map_err(GeometryFailure::AlgebraFailure)?;
        if ordering == Ordering::Greater {
            return Err(GeometryFailure::ParameterOutsideDomain);
        }
        if ordering == Ordering::Equal && !(lower_closed && upper_closed) {
            return Err(GeometryFailure::ParameterOutsideDomain);
        }
        Ok(Self {
            lower,
            upper,
            lower_closed,
            upper_closed,
        })
    }

    pub fn contains(&self, value: &RealAlgebraic) -> Result<bool, GeometryFailure> {
        let lower = value
            .compare(&self.lower)
            .map_err(GeometryFailure::AlgebraFailure)?;
        let upper = value
            .compare(&self.upper)
            .map_err(GeometryFailure::AlgebraFailure)?;
        Ok(
            (lower == Ordering::Greater || (lower == Ordering::Equal && self.lower_closed))
                && (upper == Ordering::Less || (upper == Ordering::Equal && self.upper_closed)),
        )
    }

    pub fn lower(&self) -> &RealAlgebraic {
        &self.lower
    }
    pub fn upper(&self) -> &RealAlgebraic {
        &self.upper
    }
    pub fn lower_closed(&self) -> bool {
        self.lower_closed
    }
    pub fn upper_closed(&self) -> bool {
        self.upper_closed
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct SurfaceParameterDomain {
    u: ParameterInterval,
    v: ParameterInterval,
}

impl SurfaceParameterDomain {
    pub fn new(u: ParameterInterval, v: ParameterInterval) -> Self {
        Self { u, v }
    }
    pub fn u(&self) -> &ParameterInterval {
        &self.u
    }
    pub fn v(&self) -> &ParameterInterval {
        &self.v
    }
    pub fn contains(&self, u: &RealAlgebraic, v: &RealAlgebraic) -> Result<bool, GeometryFailure> {
        Ok(self.u.contains(u)? && self.v.contains(v)?)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SurfaceOrientation {
    Preserve,
    Reverse,
}

#[derive(Debug, Eq, PartialEq)]
pub struct SurfaceChartIdentity {
    domain_id: ChartDomainId,
    version: u64,
}

impl SurfaceChartIdentity {
    pub fn new(domain_id: ChartDomainId, version: u64) -> Self {
        Self { domain_id, version }
    }
    pub fn domain_id(&self) -> &ChartDomainId {
        &self.domain_id
    }
    pub fn version(&self) -> u64 {
        self.version
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct ParametricSurfaceChart2 {
    identity: SurfaceChartIdentity,
    display_name: ChartDisplayName,
    orientation: SurfaceOrientation,
    surface: RationalSurfacePolynomials,
}

impl ParametricSurfaceChart2 {
    pub fn try_new(
        domain_id: ChartDomainId,
        display_name: ChartDisplayName,
        version: u64,
        orientation: SurfaceOrientation,
        surface: RationalSurfacePolynomials,
    ) -> Result<Self, GeometryFailure> {
        if surface.denominator.terms().is_empty() {
            return Err(GeometryFailure::InvalidSurfaceChart);
        }
        Ok(Self {
            identity: SurfaceChartIdentity::new(domain_id, version),
            display_name,
            orientation,
            surface,
        })
    }
    pub fn identity(&self) -> &SurfaceChartIdentity {
        &self.identity
    }
    pub fn display_name(&self) -> &ChartDisplayName {
        &self.display_name
    }
    pub fn orientation(&self) -> SurfaceOrientation {
        self.orientation
    }
    pub fn surface(&self) -> &RationalSurfacePolynomials {
        &self.surface
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum CurveChart {
    Chart2(ChartDefinition2),
    Chart3(ChartDefinition3),
}

#[derive(Debug, Eq, PartialEq)]
pub struct ParametricCurveDeclaration {
    chart: CurveChart,
    domain: ParameterInterval,
}

impl ParametricCurveDeclaration {
    pub fn new(chart: CurveChart, domain: ParameterInterval) -> Self {
        Self { chart, domain }
    }
    pub fn chart(&self) -> &CurveChart {
        &self.chart
    }
    pub fn domain(&self) -> &ParameterInterval {
        &self.domain
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct ParametricSurfaceDeclaration {
    chart: ParametricSurfaceChart2,
    domain: SurfaceParameterDomain,
}

impl ParametricSurfaceDeclaration {
    pub fn new(chart: ParametricSurfaceChart2, domain: SurfaceParameterDomain) -> Self {
        Self { chart, domain }
    }
    pub fn chart(&self) -> &ParametricSurfaceChart2 {
        &self.chart
    }
    pub fn domain(&self) -> &SurfaceParameterDomain {
        &self.domain
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrimLoopRole {
    Outer,
    Hole,
}

#[derive(Debug, Eq, PartialEq)]
pub struct SurfaceParameterPoint {
    u: RealAlgebraic,
    v: RealAlgebraic,
}

impl SurfaceParameterPoint {
    pub fn new(u: RealAlgebraic, v: RealAlgebraic) -> Self {
        Self { u, v }
    }
    pub fn u(&self) -> &RealAlgebraic {
        &self.u
    }
    pub fn v(&self) -> &RealAlgebraic {
        &self.v
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct SurfaceTrimLoop2 {
    role: TrimLoopRole,
    vertices: Vec<SurfaceParameterPoint>,
}

impl SurfaceTrimLoop2 {
    pub fn try_new(
        role: TrimLoopRole,
        vertices: Vec<SurfaceParameterPoint>,
        domain: &SurfaceParameterDomain,
    ) -> Result<Self, GeometryFailure> {
        if vertices.len() < 3 {
            return Err(GeometryFailure::SelfIntersectingTrimLoop);
        }
        for vertex in &vertices {
            if !domain.contains(vertex.u(), vertex.v())? {
                return Err(GeometryFailure::PCurveOutsideDomain);
            }
        }
        if loop_self_intersects(&vertices)? {
            return Err(GeometryFailure::SelfIntersectingTrimLoop);
        }
        Ok(Self { role, vertices })
    }
    pub fn role(&self) -> TrimLoopRole {
        self.role
    }
    pub fn vertices(&self) -> &[SurfaceParameterPoint] {
        &self.vertices
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct PairedSurfaceSeam {
    first: SeamEnd,
    second: SeamEnd,
}

#[derive(Debug, Eq, PartialEq)]
pub struct SeamEnd {
    identity: SurfaceChartIdentity,
    parameter: SurfaceParameterPoint,
    orientation: SurfaceOrientation,
}

impl SeamEnd {
    pub fn new(
        identity: SurfaceChartIdentity,
        parameter: SurfaceParameterPoint,
        orientation: SurfaceOrientation,
    ) -> Self {
        Self {
            identity,
            parameter,
            orientation,
        }
    }
    pub fn identity(&self) -> &SurfaceChartIdentity {
        &self.identity
    }
    pub fn parameter(&self) -> &SurfaceParameterPoint {
        &self.parameter
    }
    pub fn orientation(&self) -> SurfaceOrientation {
        self.orientation
    }
}

impl PairedSurfaceSeam {
    pub fn new(first: SeamEnd, second: SeamEnd) -> Self {
        Self { first, second }
    }
    pub fn first(&self) -> &SeamEnd {
        &self.first
    }
    pub fn second(&self) -> &SeamEnd {
        &self.second
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TrimClassification {
    Outer,
    Hole,
    TrimExterior,
    TrimBoundary,
}

#[derive(Debug, Eq, PartialEq)]
pub enum SeamContact {
    Paired(PairedSurfaceSeam),
    Undetermined { first: SeamEnd, second: SeamEnd },
}

#[derive(Debug, Eq, PartialEq)]
pub struct TrimmedSurfaceDeclaration {
    chart: ParametricSurfaceChart2,
    domain: SurfaceParameterDomain,
    outer: SurfaceTrimLoop2,
    holes: Vec<SurfaceTrimLoop2>,
    seams: Vec<SeamContact>,
}

impl TrimmedSurfaceDeclaration {
    pub fn try_new(
        chart: ParametricSurfaceChart2,
        domain: SurfaceParameterDomain,
        mut outer: SurfaceTrimLoop2,
        mut holes: Vec<SurfaceTrimLoop2>,
        seams: Vec<PairedSurfaceSeam>,
    ) -> Result<Self, GeometryFailure> {
        if outer.role != TrimLoopRole::Outer {
            return Err(GeometryFailure::SelfIntersectingTrimLoop);
        }
        for hole in &holes {
            if hole.role != TrimLoopRole::Hole {
                return Err(GeometryFailure::SelfIntersectingTrimLoop);
            }
            if loops_intersect(&outer.vertices, &hole.vertices)? {
                return Err(GeometryFailure::HoleIntersectsOuterLoop);
            }
        }
        canonicalize_loop(&mut outer.vertices)?;
        for hole in &mut holes {
            canonicalize_loop(&mut hole.vertices)?;
        }
        holes.sort_by(|left, right| compare_loop_keys(&left.vertices, &right.vertices));
        let mut seams = seams
            .into_iter()
            .map(|seam| classify_seam(&chart.identity, canonicalize_seam(seam)))
            .collect::<Vec<_>>();
        seams.sort_by(compare_seam_contacts);
        Ok(Self {
            chart,
            domain,
            outer,
            holes,
            seams,
        })
    }
    pub fn chart(&self) -> &ParametricSurfaceChart2 {
        &self.chart
    }
    pub fn domain(&self) -> &SurfaceParameterDomain {
        &self.domain
    }
    pub fn outer(&self) -> &SurfaceTrimLoop2 {
        &self.outer
    }
    pub fn holes(&self) -> &[SurfaceTrimLoop2] {
        &self.holes
    }
    pub fn seams(&self) -> &[SeamContact] {
        &self.seams
    }
    pub fn classify(
        &self,
        u: &RealAlgebraic,
        v: &RealAlgebraic,
    ) -> Result<TrimClassification, GeometryFailure> {
        if !self.domain.contains(u, v)? {
            return Err(GeometryFailure::ParameterOutsideDomain);
        }
        if point_on_loop(u, v, &self.outer.vertices)? {
            return Ok(TrimClassification::TrimBoundary);
        }
        for hole in &self.holes {
            if point_on_loop(u, v, &hole.vertices)? {
                return Ok(TrimClassification::TrimBoundary);
            }
        }
        let inside_outer = point_in_loop(u, v, &self.outer.vertices)?;
        if !inside_outer {
            return Ok(TrimClassification::TrimExterior);
        }
        for hole in &self.holes {
            if point_in_loop(u, v, &hole.vertices)? {
                return Ok(TrimClassification::Hole);
            }
        }
        Ok(TrimClassification::Outer)
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum SurfaceEvaluationWitness {
    Exact {
        identity: SurfaceChartIdentity,
        point: Point3,
    },
    Certified {
        identity: SurfaceChartIdentity,
        x: Enclosure,
        y: Enclosure,
        z: Box<Enclosure>,
        owner: OwnerId,
        requirement: ExactExpressionRequirement,
        source: NecoImplementationSource,
    },
}

#[derive(Debug, Eq, PartialEq)]
pub struct SurfaceCrossingCandidate {
    declaration: TrimmedSurfaceDeclaration,
}

impl SurfaceCrossingCandidate {
    pub fn new(declaration: TrimmedSurfaceDeclaration) -> Self {
        Self { declaration }
    }
    pub fn identity(&self) -> &SurfaceChartIdentity {
        self.declaration.chart.identity()
    }
    pub fn domain(&self) -> &SurfaceParameterDomain {
        self.declaration.domain()
    }
    pub fn orientation(&self) -> SurfaceOrientation {
        self.declaration.chart.orientation()
    }
    pub fn declaration(&self) -> &TrimmedSurfaceDeclaration {
        &self.declaration
    }
    pub fn classify(
        &self,
        u: &RealAlgebraic,
        v: &RealAlgebraic,
    ) -> Result<TrimClassification, GeometryFailure> {
        self.declaration.classify(u, v)
    }
}

fn classify_seam(chart_identity: &SurfaceChartIdentity, seam: PairedSurfaceSeam) -> SeamContact {
    if identities_match(chart_identity, seam.first.identity())
        && identities_match(chart_identity, seam.second.identity())
    {
        SeamContact::Paired(seam)
    } else {
        SeamContact::Undetermined {
            first: seam.first,
            second: seam.second,
        }
    }
}

fn identities_match(left: &SurfaceChartIdentity, right: &SurfaceChartIdentity) -> bool {
    left.domain_id().as_str() == right.domain_id().as_str() && left.version() == right.version()
}

fn canonicalize_loop(vertices: &mut [SurfaceParameterPoint]) -> Result<(), GeometryFailure> {
    rotate_loop_to_minimum(vertices);
    if loop_is_clockwise(vertices)? {
        vertices[1..].reverse();
    }
    Ok(())
}

fn rotate_loop_to_minimum(vertices: &mut [SurfaceParameterPoint]) {
    let mut minimum = 0;
    for index in 1..vertices.len() {
        if compare_points(&vertices[index], &vertices[minimum]) == Ordering::Less {
            minimum = index;
        }
    }
    vertices.rotate_left(minimum);
}

fn loop_is_clockwise(vertices: &[SurfaceParameterPoint]) -> Result<bool, GeometryFailure> {
    let zero = RealAlgebraic::from_integer(neco_bigint::BigInt::zero())?;
    let mut twice_area = zero.try_clone()?;
    for index in 0..vertices.len() {
        let current = &vertices[index];
        let next = &vertices[(index + 1) % vertices.len()];
        let term = current
            .u()
            .mul(next.v())?
            .sub(&next.u().mul(current.v())?)?;
        twice_area = twice_area.add(&term)?;
    }
    Ok(twice_area.compare(&zero)? == Ordering::Less)
}

fn canonicalize_seam(seam: PairedSurfaceSeam) -> PairedSurfaceSeam {
    if compare_seam_ends(seam.first(), seam.second()) == Ordering::Greater {
        PairedSurfaceSeam::new(seam.second, seam.first)
    } else {
        seam
    }
}

fn compare_seam_contacts(left: &SeamContact, right: &SeamContact) -> Ordering {
    match (left, right) {
        (SeamContact::Paired(left), SeamContact::Paired(right)) => compare_seams(left, right),
        (
            SeamContact::Undetermined {
                first: left_first,
                second: left_second,
            },
            SeamContact::Undetermined {
                first: right_first,
                second: right_second,
            },
        ) => compare_seam_ends(left_first, right_first)
            .then(compare_seam_ends(left_second, right_second)),
        (SeamContact::Paired(_), SeamContact::Undetermined { .. }) => Ordering::Less,
        (SeamContact::Undetermined { .. }, SeamContact::Paired(_)) => Ordering::Greater,
    }
}

fn compare_seams(left: &PairedSurfaceSeam, right: &PairedSurfaceSeam) -> Ordering {
    compare_seam_ends(left.first(), right.first())
        .then(compare_seam_ends(left.second(), right.second()))
}

fn compare_seam_ends(left: &SeamEnd, right: &SeamEnd) -> Ordering {
    left.identity()
        .domain_id()
        .as_str()
        .cmp(right.identity().domain_id().as_str())
        .then(left.identity().version().cmp(&right.identity().version()))
        .then(compare_points(left.parameter(), right.parameter()))
        .then(orientation_order(left.orientation()).cmp(&orientation_order(right.orientation())))
}

fn orientation_order(orientation: SurfaceOrientation) -> u8 {
    match orientation {
        SurfaceOrientation::Preserve => 0,
        SurfaceOrientation::Reverse => 1,
    }
}

fn compare_loop_keys(left: &[SurfaceParameterPoint], right: &[SurfaceParameterPoint]) -> Ordering {
    match (left.first(), right.first()) {
        (Some(first), Some(second)) => compare_points(first, second),
        (None, Some(_)) => Ordering::Less,
        (Some(_), None) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

fn compare_points(left: &SurfaceParameterPoint, right: &SurfaceParameterPoint) -> Ordering {
    match left.u().compare(right.u()) {
        Ok(Ordering::Equal) => left.v().compare(right.v()).unwrap_or(Ordering::Equal),
        Ok(ordering) => ordering,
        Err(_) => Ordering::Equal,
    }
}

fn loop_self_intersects(vertices: &[SurfaceParameterPoint]) -> Result<bool, GeometryFailure> {
    let count = vertices.len();
    for first in 0..count {
        let first_end = (first + 1) % count;
        for second in (first + 1)..count {
            let second_end = (second + 1) % count;
            if first == second
                || first_end == second
                || second_end == first
                || (first == 0 && second_end == 0)
            {
                continue;
            }
            if segments_intersect(
                &vertices[first],
                &vertices[first_end],
                &vertices[second],
                &vertices[second_end],
            )? {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn loops_intersect(
    left: &[SurfaceParameterPoint],
    right: &[SurfaceParameterPoint],
) -> Result<bool, GeometryFailure> {
    for first in 0..left.len() {
        let first_end = (first + 1) % left.len();
        for second in 0..right.len() {
            let second_end = (second + 1) % right.len();
            if segments_intersect(
                &left[first],
                &left[first_end],
                &right[second],
                &right[second_end],
            )? {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

pub(crate) fn point_on_loop(
    u: &RealAlgebraic,
    v: &RealAlgebraic,
    vertices: &[SurfaceParameterPoint],
) -> Result<bool, GeometryFailure> {
    let point = SurfaceParameterPoint::new(u.try_clone()?, v.try_clone()?);
    for index in 0..vertices.len() {
        let next = (index + 1) % vertices.len();
        if point_on_segment(&point, &vertices[index], &vertices[next])? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn point_in_loop(
    u: &RealAlgebraic,
    v: &RealAlgebraic,
    vertices: &[SurfaceParameterPoint],
) -> Result<bool, GeometryFailure> {
    let mut winding = 0_i32;
    for index in 0..vertices.len() {
        let current = &vertices[index];
        let next = &vertices[(index + 1) % vertices.len()];
        let current_v = current.v().compare(v)?;
        let next_v = next.v().compare(v)?;
        if current_v == Ordering::Greater
            && next_v != Ordering::Greater
            && is_left(u, v, current, next)? == Ordering::Greater
        {
            winding += 1;
        } else if current_v != Ordering::Greater
            && next_v == Ordering::Greater
            && is_left(u, v, current, next)? == Ordering::Less
        {
            winding -= 1;
        }
    }
    Ok(winding != 0)
}

fn is_left(
    u: &RealAlgebraic,
    v: &RealAlgebraic,
    start: &SurfaceParameterPoint,
    end: &SurfaceParameterPoint,
) -> Result<Ordering, GeometryFailure> {
    let dx = end.u().sub(start.u())?;
    let dy = end.v().sub(start.v())?;
    let px = u.sub(start.u())?;
    let py = v.sub(start.v())?;
    Ok(dx
        .mul(&py)?
        .sub(&dy.mul(&px)?)?
        .compare(&RealAlgebraic::from_integer(neco_bigint::BigInt::zero())?)?)
}

fn segments_intersect(
    a: &SurfaceParameterPoint,
    b: &SurfaceParameterPoint,
    c: &SurfaceParameterPoint,
    d: &SurfaceParameterPoint,
) -> Result<bool, GeometryFailure> {
    let abc = is_left(c.u(), c.v(), a, b)?;
    let abd = is_left(d.u(), d.v(), a, b)?;
    let cda = is_left(a.u(), a.v(), c, d)?;
    let cdb = is_left(b.u(), b.v(), c, d)?;
    Ok(abc != abd && cda != cdb)
}

fn point_on_segment(
    point: &SurfaceParameterPoint,
    start: &SurfaceParameterPoint,
    end: &SurfaceParameterPoint,
) -> Result<bool, GeometryFailure> {
    if is_left(point.u(), point.v(), start, end)? != Ordering::Equal {
        return Ok(false);
    }
    let min_u = if start.u().compare(end.u())? == Ordering::Less {
        start.u()
    } else {
        end.u()
    };
    let max_u = if start.u().compare(end.u())? == Ordering::Greater {
        start.u()
    } else {
        end.u()
    };
    let min_v = if start.v().compare(end.v())? == Ordering::Less {
        start.v()
    } else {
        end.v()
    };
    let max_v = if start.v().compare(end.v())? == Ordering::Greater {
        start.v()
    } else {
        end.v()
    };
    Ok(point.u().compare(min_u)? != Ordering::Less
        && point.u().compare(max_u)? != Ordering::Greater
        && point.v().compare(min_v)? != Ordering::Less
        && point.v().compare(max_v)? != Ordering::Greater)
}

#[derive(Debug, Eq, PartialEq)]
pub struct PolynomialTerm3 {
    coefficient: RealAlgebraic,
    exponent_x: u32,
    exponent_y: u32,
    exponent_z: u32,
}

impl PolynomialTerm3 {
    pub fn new(
        coefficient: RealAlgebraic,
        exponent_x: u32,
        exponent_y: u32,
        exponent_z: u32,
    ) -> Result<Self, GeometryFailure> {
        if coefficient.is_zero() {
            return Err(GeometryFailure::EvaluationIndeterminacy {
                family: ShapeFamily::ImplicitSurface,
                reason: ShapeEvaluationIndeterminacy::DomainValidity,
            });
        }
        Ok(Self {
            coefficient,
            exponent_x,
            exponent_y,
            exponent_z,
        })
    }
    pub fn coefficient(&self) -> &RealAlgebraic {
        &self.coefficient
    }
    pub fn exponent_x(&self) -> u32 {
        self.exponent_x
    }
    pub fn exponent_y(&self) -> u32 {
        self.exponent_y
    }
    pub fn exponent_z(&self) -> u32 {
        self.exponent_z
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct MultivariatePolynomial3 {
    terms: Vec<PolynomialTerm3>,
}

impl MultivariatePolynomial3 {
    pub fn try_from_terms(mut terms: Vec<PolynomialTerm3>) -> Result<Self, GeometryFailure> {
        terms.sort_unstable_by_key(|term| (term.exponent_x, term.exponent_y, term.exponent_z));
        if terms.windows(2).any(|pair| {
            (pair[0].exponent_x, pair[0].exponent_y, pair[0].exponent_z)
                == (pair[1].exponent_x, pair[1].exponent_y, pair[1].exponent_z)
        }) {
            return Err(GeometryFailure::EvaluationIndeterminacy {
                family: ShapeFamily::ImplicitSurface,
                reason: ShapeEvaluationIndeterminacy::DomainValidity,
            });
        }
        Ok(Self { terms })
    }
    pub fn zero() -> Self {
        Self { terms: Vec::new() }
    }
    pub fn terms(&self) -> &[PolynomialTerm3] {
        &self.terms
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct ImplicitSurfaceDeclaration {
    equation: MultivariatePolynomial3,
}
impl ImplicitSurfaceDeclaration {
    pub fn new(equation: MultivariatePolynomial3) -> Self {
        Self { equation }
    }
    pub fn equation(&self) -> &MultivariatePolynomial3 {
        &self.equation
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum ShapeDeclaration {
    ParametricCurve(ParametricCurveDeclaration),
    ParametricSurface(ParametricSurfaceDeclaration),
    ImplicitSurface(ImplicitSurfaceDeclaration),
}

impl ShapeDeclaration {
    pub fn family(&self) -> ShapeFamily {
        match self {
            Self::ParametricCurve(_) => ShapeFamily::ParametricCurve,
            Self::ParametricSurface(_) => ShapeFamily::ParametricSurface,
            Self::ImplicitSurface(_) => ShapeFamily::ImplicitSurface,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShapeFamily {
    ParametricCurve,
    ParametricSurface,
    ImplicitSurface,
}

#[derive(Debug, Eq, PartialEq)]
pub enum ShapeEvaluationInput {
    ParametricCurve { parameter: RealAlgebraic },
    ParametricSurface { u: RealAlgebraic, v: RealAlgebraic },
    ImplicitSurface { point: Point3 },
}

impl ShapeEvaluationInput {
    pub fn family(&self) -> ShapeFamily {
        match self {
            Self::ParametricCurve { .. } => ShapeFamily::ParametricCurve,
            Self::ParametricSurface { .. } => ShapeFamily::ParametricSurface,
            Self::ImplicitSurface { .. } => ShapeFamily::ImplicitSurface,
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct ShapeEvaluationRequest {
    declaration: ShapeDeclaration,
    input: ShapeEvaluationInput,
}
impl ShapeEvaluationRequest {
    pub fn new(declaration: ShapeDeclaration, input: ShapeEvaluationInput) -> Self {
        Self { declaration, input }
    }
    pub fn declaration(&self) -> &ShapeDeclaration {
        &self.declaration
    }
    pub fn input(&self) -> &ShapeEvaluationInput {
        &self.input
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum ShapeCoordinate {
    Planar(Point2),
    Spatial(Point3),
}
#[derive(Debug, Eq, PartialEq)]
pub struct CoordinateEnclosure2 {
    pub x: Enclosure,
    pub y: Enclosure,
}
#[derive(Debug, Eq, PartialEq)]
pub struct CoordinateEnclosure3 {
    pub x: Enclosure,
    pub y: Enclosure,
    pub z: Enclosure,
}
#[derive(Debug, Eq, PartialEq)]
pub enum ShapeCoordinateEnclosure {
    Planar(CoordinateEnclosure2),
    Spatial(Box<CoordinateEnclosure3>),
}

#[derive(Debug, Eq, PartialEq)]
pub enum ExactShapeEvaluation {
    ParametricCurve { coordinate: ShapeCoordinate },
    ParametricSurface { coordinate: Point3 },
    ImplicitSurface { scalar: RealAlgebraic },
}

#[derive(Debug, Eq, PartialEq)]
pub enum CertifiedShapeEvaluation {
    ParametricCurve {
        coordinate: ShapeCoordinateEnclosure,
        owner: OwnerId,
        requirement: ExactExpressionRequirement,
        source: NecoImplementationSource,
    },
    ParametricSurface {
        coordinate: Box<CoordinateEnclosure3>,
        owner: OwnerId,
        requirement: ExactExpressionRequirement,
        source: NecoImplementationSource,
    },
    ImplicitSurface {
        scalar: Enclosure,
        owner: OwnerId,
        requirement: ExactExpressionRequirement,
        source: NecoImplementationSource,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ShapeEvaluationIndeterminacy {
    DomainValidity,
    CalculusResult,
}

#[derive(Debug, Eq, PartialEq)]
pub enum ShapeEvaluation {
    Exact(ExactShapeEvaluation),
    CertifiedEnclosure(CertifiedShapeEvaluation),
    Undetermined { family: ShapeFamily },
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PositiveRayParameterRange;

impl PositiveRayParameterRange {
    pub const fn new() -> Self {
        Self
    }

    pub fn contains_exact(&self, value: &RealAlgebraic) -> Result<bool, GeometryFailure> {
        Ok(!value.is_zero()
            && value.compare(&RealAlgebraic::from_integer(BigInt::zero())?)? == Ordering::Greater)
    }

    pub fn contains_checked(&self, value: f64) -> Result<bool, GeometryFailure> {
        Ok(value.is_finite() && value > 0.0)
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct Ray2ParameterRange {
    lower: Dyadic,
    upper: Dyadic,
}

impl Ray2ParameterRange {
    pub fn new(lower: Dyadic, upper: Dyadic) -> Result<Self, GeometryFailure> {
        if lower.integer().sign() == neco_bigint::Sign::Negative || upper < lower {
            return Err(GeometryFailure::InvalidRayParameterRange);
        }
        Ok(Self { lower, upper })
    }

    pub fn lower(&self) -> &Dyadic {
        &self.lower
    }

    pub fn upper(&self) -> &Dyadic {
        &self.upper
    }
}

#[derive(Debug, PartialEq)]
pub struct CheckedRay2ParameterRange {
    lower: f64,
    upper: f64,
}

impl CheckedRay2ParameterRange {
    pub fn new(lower: f64, upper: f64) -> Result<Self, GeometryFailure> {
        if !lower.is_finite() || !upper.is_finite() || lower < 0.0 || upper < lower {
            return Err(GeometryFailure::InvalidRayParameterRange);
        }
        Ok(Self { lower, upper })
    }

    pub fn lower(&self) -> f64 {
        self.lower
    }

    pub fn upper(&self) -> f64 {
        self.upper
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum ExactRayEvaluation2 {
    Point(Point2),
}
#[derive(Debug, Eq, PartialEq)]
pub enum ExactRayEvaluation3 {
    Point(Point3),
}
#[derive(Debug, PartialEq)]
pub enum CheckedRayEvaluation2 {
    Candidate([f64; 2]),
    Undetermined,
}
#[derive(Debug, PartialEq)]
pub enum CheckedRayEvaluation3 {
    Candidate([f64; 3]),
    Undetermined,
}

#[derive(Debug, Eq, PartialEq)]
pub enum GeometryFailure {
    FoundationFailure {
        error: neco_geometry_foundation::GeometryFailure,
    },
    ZeroDirection,
    NonOrthogonalMatrix,
    NonUnitQuaternion,
    AlgebraFailure(AlgnumError),
    ShapeFamilyMismatch,
    ParameterOutsideDomain,
    InvalidRayParameterRange,
    DenominatorValidity {
        error: CalculusFailure,
    },
    CalculusFailure {
        error: CalculusFailure,
    },
    EvaluationIndeterminacy {
        family: ShapeFamily,
        reason: ShapeEvaluationIndeterminacy,
    },
    DuplicateRegionAtom,
    DuplicateRegionCanonicalKey,
    RegionCanonicalAtomLimit {
        requested_atoms: usize,
        maximum_atoms: usize,
    },
    RegionPolynomialTermLimit {
        requested_terms: usize,
        maximum_terms: usize,
    },
    RegionExponentOverflow,
    RegionExponentOperationLimit {
        requested_operations: usize,
        maximum_operations: usize,
    },
    InvalidRegionAffineMap,
    MalformedRegionFormula,
    RegionAmbientDimensionMismatch,
    RegionKindMismatch,
    RegionPointDimensionMismatch,
    RootIsolationPolicyInvalid,
    RootIsolationResourceLimit {
        requested_depth: usize,
        maximum_depth: usize,
    },
    RootPartitionCellLimit {
        requested_cells: usize,
        maximum_cells: usize,
    },
    RootPartitionIdentifierOverflow {
        current_identifier: usize,
        increment: usize,
    },
    BoundaryResourceLimit {
        resource: BoundaryResource,
        requested: usize,
        maximum: usize,
    },
    EmptyFinitePointComponents,
    EmptyCurveBranchComponents,
    DuplicateIntersectionComponentIdentity,
    ComponentIdentityEvidenceMismatch,
    ComponentIdentityCanonicalizationFailure,
    InvalidSurfaceChart,
    SelfIntersectingTrimLoop,
    HoleIntersectsOuterLoop,
    PCurveOutsideDomain,
    EmptyRayCrossingTargetSet,
    DuplicateRayCrossingTargetIdentity,
    MixedRayCrossingTargetFamily,
    DegenerateTriangle,
    InvalidRayCrossingPolicy,
    RayCrossingTargetLimitExceeded,
    RayCrossingHitLimitExceeded,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BoundaryResource {
    CandidateLoci,
    CandidateParameters,
    SideSamples,
    Intersections,
}

impl From<AlgnumError> for GeometryFailure {
    fn from(error: AlgnumError) -> Self {
        Self::AlgebraFailure(error)
    }
}

impl From<neco_geometry_foundation::GeometryFailure> for GeometryFailure {
    fn from(error: neco_geometry_foundation::GeometryFailure) -> Self {
        Self::FoundationFailure { error }
    }
}
