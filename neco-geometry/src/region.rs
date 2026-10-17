use alloc::{boxed::Box, string::String, vec, vec::Vec};
use core::cmp::Ordering;

use neco_algnum::{AlgnumError, RealAlgebraic};
use neco_bigint::{BigInt, BigintError, Sign};
use neco_calculus::{CalculusFailure, PolynomialFailure, RationalFailure};
use neco_geometry_foundation::{Point2, Point3};

use crate::types::GeometryFailure;

pub const MAX_REGION_CANONICAL_ATOMS: usize = 256;
pub const MAX_REGION_POLYNOMIAL_TERMS: usize = 256;
pub const MAX_REGION_EXPONENT_OPERATIONS: usize = 4_096;

#[derive(Debug, Eq, PartialEq)]
pub struct RegionPolynomialTerm {
    coefficient: RealAlgebraic,
    exponent_x: u32,
    exponent_y: u32,
    exponent_z: u32,
}

impl RegionPolynomialTerm {
    pub fn try_new(
        coefficient: RealAlgebraic,
        exponent_x: u32,
        exponent_y: u32,
        exponent_z: u32,
    ) -> Result<Self, GeometryFailure> {
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

    fn try_clone(&self) -> Result<Self, GeometryFailure> {
        Ok(Self {
            coefficient: self
                .coefficient
                .try_clone()
                .map_err(GeometryFailure::AlgebraFailure)?,
            exponent_x: self.exponent_x,
            exponent_y: self.exponent_y,
            exponent_z: self.exponent_z,
        })
    }
}

pub type RegionPolynomialTermSet = Vec<RegionPolynomialTerm>;

#[derive(Debug, Eq, PartialEq)]
pub struct RegionPolynomial3 {
    terms: RegionPolynomialTermSet,
}

impl RegionPolynomial3 {
    pub fn try_from_terms(mut terms: RegionPolynomialTermSet) -> Result<Self, GeometryFailure> {
        terms.sort_unstable_by_key(|term| (term.exponent_x, term.exponent_y, term.exponent_z));
        let mut normalized: Vec<RegionPolynomialTerm> = Vec::with_capacity(terms.len());
        for term in terms {
            if term.coefficient.is_zero() {
                continue;
            }
            if let Some(previous) = normalized.last_mut() {
                if (
                    previous.exponent_x,
                    previous.exponent_y,
                    previous.exponent_z,
                ) == (term.exponent_x, term.exponent_y, term.exponent_z)
                {
                    previous.coefficient = previous
                        .coefficient
                        .add(&term.coefficient)
                        .map_err(GeometryFailure::AlgebraFailure)?;
                    if previous.coefficient.is_zero() {
                        normalized.pop();
                    }
                    continue;
                }
            }
            normalized.push(term);
        }
        if normalized.len() > MAX_REGION_POLYNOMIAL_TERMS {
            return Err(GeometryFailure::RegionPolynomialTermLimit {
                requested_terms: normalized.len(),
                maximum_terms: MAX_REGION_POLYNOMIAL_TERMS,
            });
        }
        Ok(Self { terms: normalized })
    }

    pub fn zero() -> Self {
        Self { terms: Vec::new() }
    }

    pub fn terms(&self) -> &[RegionPolynomialTerm] {
        &self.terms
    }

    fn is_zero(&self) -> bool {
        self.terms.iter().all(|term| term.coefficient.is_zero())
    }

    pub fn try_clone(&self) -> Result<Self, GeometryFailure> {
        Ok(Self {
            terms: self
                .terms
                .iter()
                .map(RegionPolynomialTerm::try_clone)
                .collect::<Result<Vec<_>, _>>()?,
        })
    }

    pub fn try_add(&self, other: &Self) -> Result<Self, GeometryFailure> {
        let mut terms = Vec::with_capacity(self.terms.len() + other.terms.len());
        for term in &self.terms {
            terms.push(term.try_clone()?);
        }
        for term in &other.terms {
            terms.push(term.try_clone()?);
        }
        Self::try_from_terms(terms)
    }

    pub fn try_scale(&self, scalar: &RealAlgebraic) -> Result<Self, GeometryFailure> {
        let mut terms = Vec::with_capacity(self.terms.len());
        for term in &self.terms {
            terms.push(RegionPolynomialTerm {
                coefficient: term
                    .coefficient
                    .mul(scalar)
                    .map_err(GeometryFailure::AlgebraFailure)?,
                exponent_x: term.exponent_x,
                exponent_y: term.exponent_y,
                exponent_z: term.exponent_z,
            });
        }
        Self::try_from_terms(terms)
    }

    pub fn try_multiply(&self, other: &Self) -> Result<Self, GeometryFailure> {
        let mut operations = 0;
        self.try_multiply_counted(other, &mut operations)
    }

    pub fn try_affine_substitute(&self, map: &RegionAffineMap3) -> Result<Self, GeometryFailure> {
        let mut operations = 0;
        let mut result = Self::zero();
        for term in &self.terms {
            let powered_x = map.x.try_pow(term.exponent_x, &mut operations)?;
            let powered_y = map.y.try_pow(term.exponent_y, &mut operations)?;
            let powered_z = map.z.try_pow(term.exponent_z, &mut operations)?;
            let mut substituted = powered_x.try_multiply_counted(&powered_y, &mut operations)?;
            substituted = substituted.try_multiply_counted(&powered_z, &mut operations)?;
            substituted = substituted.try_scale(&term.coefficient)?;
            result = result.try_add(&substituted)?;
        }
        Ok(result)
    }

    fn try_multiply_counted(
        &self,
        other: &Self,
        operations: &mut usize,
    ) -> Result<Self, GeometryFailure> {
        if self.is_zero() || other.is_zero() {
            return Ok(Self::zero());
        }
        let mut terms = Vec::new();
        for left in &self.terms {
            for right in &other.terms {
                let exponent_x = add_exponents(left.exponent_x, right.exponent_x, operations)?;
                let exponent_y = add_exponents(left.exponent_y, right.exponent_y, operations)?;
                let exponent_z = add_exponents(left.exponent_z, right.exponent_z, operations)?;
                let coefficient = left
                    .coefficient
                    .mul(&right.coefficient)
                    .map_err(GeometryFailure::AlgebraFailure)?;
                terms.push(RegionPolynomialTerm {
                    coefficient,
                    exponent_x,
                    exponent_y,
                    exponent_z,
                });
            }
        }
        Self::try_from_terms(terms)
    }

    fn try_pow(&self, exponent: u32, operations: &mut usize) -> Result<Self, GeometryFailure> {
        if exponent == 0 {
            return constant_one();
        }
        if exponent == 1 {
            return self.try_clone();
        }
        if self.terms.len() == 1 {
            return self.try_monomial_pow(exponent, operations);
        }
        let mut result = self.try_clone()?;
        for _ in 1..exponent {
            result = result.try_multiply_counted(self, operations)?;
        }
        Ok(result)
    }

    fn try_monomial_pow(
        &self,
        exponent: u32,
        operations: &mut usize,
    ) -> Result<Self, GeometryFailure> {
        let term = &self.terms[0];
        let exponent_x = multiply_exponents(term.exponent_x, exponent, operations)?;
        let exponent_y = multiply_exponents(term.exponent_y, exponent, operations)?;
        let exponent_z = multiply_exponents(term.exponent_z, exponent, operations)?;
        let power = BigInt::try_from(exponent).map_err(calculus_from_bigint)?;
        Self::try_from_terms(vec![RegionPolynomialTerm {
            coefficient: algebraic_power(&term.coefficient, &power)?,
            exponent_x,
            exponent_y,
            exponent_z,
        }])
    }

    fn evaluate(
        &self,
        x: &RealAlgebraic,
        y: &RealAlgebraic,
        z: &RealAlgebraic,
    ) -> Result<RealAlgebraic, GeometryFailure> {
        let zero = BigInt::zero();
        let mut result =
            RealAlgebraic::from_integer(zero).map_err(GeometryFailure::AlgebraFailure)?;
        for term in &self.terms {
            let exponent_x = BigInt::try_from(term.exponent_x).map_err(calculus_from_bigint)?;
            let exponent_y = BigInt::try_from(term.exponent_y).map_err(calculus_from_bigint)?;
            let exponent_z = BigInt::try_from(term.exponent_z).map_err(calculus_from_bigint)?;
            let value = algebraic_power(x, &exponent_x)?
                .mul(&algebraic_power(y, &exponent_y)?)
                .map_err(GeometryFailure::AlgebraFailure)?
                .mul(&algebraic_power(z, &exponent_z)?)
                .map_err(GeometryFailure::AlgebraFailure)?
                .mul(&term.coefficient)
                .map_err(GeometryFailure::AlgebraFailure)?;
            result = result
                .add(&value)
                .map_err(GeometryFailure::AlgebraFailure)?;
        }
        Ok(result)
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct RegionAffineMap3 {
    x: RegionPolynomial3,
    y: RegionPolynomial3,
    z: RegionPolynomial3,
}

impl RegionAffineMap3 {
    pub fn try_new(
        x: RegionPolynomial3,
        y: RegionPolynomial3,
        z: RegionPolynomial3,
    ) -> Result<Self, GeometryFailure> {
        validate_affine_coordinate(&x)?;
        validate_affine_coordinate(&y)?;
        validate_affine_coordinate(&z)?;
        Ok(Self { x, y, z })
    }

    pub fn x(&self) -> &RegionPolynomial3 {
        &self.x
    }

    pub fn y(&self) -> &RegionPolynomial3 {
        &self.y
    }

    pub fn z(&self) -> &RegionPolynomial3 {
        &self.z
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct RationalFunctionDomain3 {
    non_zero_denominator: RegionPolynomial3,
}

impl RationalFunctionDomain3 {
    pub fn try_new(non_zero_denominator: RegionPolynomial3) -> Result<Self, GeometryFailure> {
        if non_zero_denominator.is_zero() {
            return Err(GeometryFailure::MalformedRegionFormula);
        }
        Ok(Self {
            non_zero_denominator,
        })
    }

    pub fn non_zero_denominator(&self) -> &RegionPolynomial3 {
        &self.non_zero_denominator
    }

    fn try_clone(&self) -> Result<Self, GeometryFailure> {
        Ok(Self {
            non_zero_denominator: self.non_zero_denominator.try_clone()?,
        })
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct RationalFunction3 {
    numerator: RegionPolynomial3,
    denominator: RegionPolynomial3,
    domain: RationalFunctionDomain3,
}

impl RationalFunction3 {
    pub fn try_new(
        numerator: RegionPolynomial3,
        denominator: RegionPolynomial3,
        domain: RationalFunctionDomain3,
    ) -> Result<Self, GeometryFailure> {
        if denominator != *domain.non_zero_denominator() {
            return Err(GeometryFailure::MalformedRegionFormula);
        }
        Ok(Self {
            numerator,
            denominator,
            domain,
        })
    }

    pub fn numerator(&self) -> &RegionPolynomial3 {
        &self.numerator
    }

    pub fn denominator(&self) -> &RegionPolynomial3 {
        &self.denominator
    }

    pub fn domain(&self) -> &RationalFunctionDomain3 {
        &self.domain
    }

    pub fn try_clone(&self) -> Result<Self, GeometryFailure> {
        Ok(Self {
            numerator: self.numerator.try_clone()?,
            denominator: self.denominator.try_clone()?,
            domain: self.domain.try_clone()?,
        })
    }

    pub fn try_affine_substitute(&self, map: &RegionAffineMap3) -> Result<Self, GeometryFailure> {
        let numerator = self.numerator.try_affine_substitute(map)?;
        let denominator = self.denominator.try_affine_substitute(map)?;
        let domain = RationalFunctionDomain3::try_new(denominator.try_clone()?)?;
        Self::try_new(numerator, denominator, domain)
    }

    fn evaluate(
        &self,
        x: &RealAlgebraic,
        y: &RealAlgebraic,
        z: &RealAlgebraic,
    ) -> Result<RealAlgebraic, GeometryFailure> {
        let denominator = self.domain.non_zero_denominator.evaluate(x, y, z)?;
        if denominator.is_zero() {
            return Err(GeometryFailure::CalculusFailure {
                error: CalculusFailure::Rational(RationalFailure::UndefinedDomain),
            });
        }
        let numerator = self.numerator.evaluate(x, y, z)?;
        numerator.div(&denominator).map_err(|error| match error {
            AlgnumError::DivisionByZero => GeometryFailure::CalculusFailure {
                error: CalculusFailure::Polynomial(PolynomialFailure::Algebra(error)),
            },
            error => GeometryFailure::AlgebraFailure(error),
        })
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum RegionFunction {
    Polynomial(RegionPolynomial3),
    RationalFunction(RationalFunction3),
}

impl RegionFunction {
    pub fn try_clone(&self) -> Result<Self, GeometryFailure> {
        match self {
            Self::Polynomial(polynomial) => Ok(Self::Polynomial(polynomial.try_clone()?)),
            Self::RationalFunction(function) => Ok(Self::RationalFunction(function.try_clone()?)),
        }
    }

    pub fn try_affine_substitute(&self, map: &RegionAffineMap3) -> Result<Self, GeometryFailure> {
        match self {
            Self::Polynomial(polynomial) => {
                Ok(Self::Polynomial(polynomial.try_affine_substitute(map)?))
            }
            Self::RationalFunction(function) => {
                Ok(Self::RationalFunction(function.try_affine_substitute(map)?))
            }
        }
    }

    fn evaluate(
        &self,
        x: &RealAlgebraic,
        y: &RealAlgebraic,
        z: &RealAlgebraic,
    ) -> Result<RealAlgebraic, GeometryFailure> {
        match self {
            Self::Polynomial(polynomial) => polynomial.evaluate(x, y, z),
            Self::RationalFunction(function) => function.evaluate(x, y, z),
        }
    }

    fn compatible_with_dimension(&self, ambient_dimension: i64) -> bool {
        if ambient_dimension == 3 {
            return true;
        }
        let has_z = |polynomial: &RegionPolynomial3| {
            polynomial.terms().iter().any(|term| term.exponent_z != 0)
        };
        match self {
            Self::Polynomial(polynomial) => !has_z(polynomial),
            Self::RationalFunction(function) => {
                !has_z(function.numerator()) && !has_z(function.denominator())
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SignRelation {
    Negative,
    Zero,
    Positive,
}

#[derive(Debug, Eq, PartialEq)]
pub struct CanonicalAtom {
    atom_id: String,
    function: RegionFunction,
    relation: SignRelation,
}

impl CanonicalAtom {
    pub fn try_new(
        atom_id: impl Into<String>,
        function: RegionFunction,
        relation: SignRelation,
    ) -> Result<Self, GeometryFailure> {
        let atom_id = atom_id.into();
        if atom_id.is_empty() {
            return Err(GeometryFailure::MalformedRegionFormula);
        }
        Ok(Self {
            atom_id,
            function,
            relation,
        })
    }

    pub fn atom_id(&self) -> &str {
        &self.atom_id
    }

    pub fn function(&self) -> &RegionFunction {
        &self.function
    }

    pub fn relation(&self) -> SignRelation {
        self.relation
    }

    pub fn try_clone(&self) -> Result<Self, GeometryFailure> {
        Ok(Self {
            atom_id: self.atom_id.clone(),
            function: self.function.try_clone()?,
            relation: self.relation,
        })
    }
}

pub type CanonicalAtomSet = Vec<CanonicalAtom>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NormalizedRegionFormula {
    Atom(String),
    Not(Box<NormalizedRegionFormula>),
    And(NormalizedRegionFormulaSet),
    Or(NormalizedRegionFormulaSet),
    True,
    False,
}

pub type NormalizedRegionFormulaSet = Vec<NormalizedRegionFormula>;

impl NormalizedRegionFormula {
    pub fn normalize(self) -> Result<Self, GeometryFailure> {
        match self {
            Self::Atom(atom_id) if atom_id.is_empty() => {
                Err(GeometryFailure::MalformedRegionFormula)
            }
            Self::Atom(atom_id) => Ok(Self::Atom(atom_id)),
            Self::Not(formula) => match formula.normalize()? {
                Self::True => Ok(Self::False),
                Self::False => Ok(Self::True),
                Self::Not(inner) => Ok(*inner),
                normalized => Ok(Self::Not(Box::new(normalized))),
            },
            Self::And(operands) => normalize_operands(operands, true),
            Self::Or(operands) => normalize_operands(operands, false),
            Self::True => Ok(Self::True),
            Self::False => Ok(Self::False),
        }
    }
}

fn normalize_operands(
    operands: NormalizedRegionFormulaSet,
    is_and: bool,
) -> Result<NormalizedRegionFormula, GeometryFailure> {
    if operands.is_empty() {
        return Err(GeometryFailure::MalformedRegionFormula);
    }
    let mut normalized = Vec::new();
    for operand in operands {
        let operand = operand.normalize()?;
        if (is_and && operand == NormalizedRegionFormula::False)
            || (!is_and && operand == NormalizedRegionFormula::True)
        {
            return Ok(if is_and {
                NormalizedRegionFormula::False
            } else {
                NormalizedRegionFormula::True
            });
        }
        if (is_and && operand == NormalizedRegionFormula::True)
            || (!is_and && operand == NormalizedRegionFormula::False)
        {
            continue;
        }
        normalized.push(operand);
    }
    normalized.sort_by(compare_formulas);
    normalized.dedup();
    match normalized.len() {
        0 => Ok(if is_and {
            NormalizedRegionFormula::True
        } else {
            NormalizedRegionFormula::False
        }),
        1 => Ok(normalized.remove(0)),
        _ if is_and => Ok(NormalizedRegionFormula::And(normalized)),
        _ => Ok(NormalizedRegionFormula::Or(normalized)),
    }
}

fn compare_formulas(left: &NormalizedRegionFormula, right: &NormalizedRegionFormula) -> Ordering {
    let rank = |formula: &NormalizedRegionFormula| match formula {
        NormalizedRegionFormula::Atom(_) => 0,
        NormalizedRegionFormula::Not(_) => 1,
        NormalizedRegionFormula::And(_) => 2,
        NormalizedRegionFormula::Or(_) => 3,
        NormalizedRegionFormula::True => 4,
        NormalizedRegionFormula::False => 5,
    };
    match rank(left).cmp(&rank(right)) {
        Ordering::Equal => match (left, right) {
            (NormalizedRegionFormula::Atom(left), NormalizedRegionFormula::Atom(right)) => {
                left.cmp(right)
            }
            (NormalizedRegionFormula::Not(left), NormalizedRegionFormula::Not(right)) => {
                compare_formulas(left, right)
            }
            (NormalizedRegionFormula::And(left), NormalizedRegionFormula::And(right))
            | (NormalizedRegionFormula::Or(left), NormalizedRegionFormula::Or(right)) => {
                compare_formula_sets(left, right)
            }
            _ => Ordering::Equal,
        },
        ordering => ordering,
    }
}

fn compare_formula_sets(
    left: &NormalizedRegionFormulaSet,
    right: &NormalizedRegionFormulaSet,
) -> Ordering {
    for (left, right) in left.iter().zip(right) {
        match compare_formulas(left, right) {
            Ordering::Equal => {}
            ordering => return ordering,
        }
    }
    left.len().cmp(&right.len())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RegionKind {
    RegularClosedVolume,
    GeneralSemiAlgebraicSet,
    LowerDimensionalSet,
}

#[derive(Debug, Eq, PartialEq)]
#[must_use]
pub struct AlgebraicRegion {
    ambient_dimension: i64,
    canonical_atoms: CanonicalAtomSet,
    normalized_region_formula: NormalizedRegionFormula,
    region_kind: RegionKind,
}

impl AlgebraicRegion {
    pub fn try_new(
        ambient_dimension: i64,
        mut canonical_atoms: CanonicalAtomSet,
        normalized_region_formula: NormalizedRegionFormula,
        region_kind: RegionKind,
    ) -> Result<Self, GeometryFailure> {
        if ambient_dimension != 2 && ambient_dimension != 3 {
            return Err(GeometryFailure::RegionAmbientDimensionMismatch);
        }
        if canonical_atoms.len() > MAX_REGION_CANONICAL_ATOMS {
            return Err(GeometryFailure::RegionCanonicalAtomLimit {
                requested_atoms: canonical_atoms.len(),
                maximum_atoms: MAX_REGION_CANONICAL_ATOMS,
            });
        }
        canonical_atoms.sort_unstable_by(|left, right| left.atom_id.cmp(&right.atom_id));
        for pair in canonical_atoms.windows(2) {
            if pair[0].atom_id == pair[1].atom_id {
                return Err(GeometryFailure::DuplicateRegionAtom);
            }
        }
        if canonical_atoms.iter().enumerate().any(|(index, atom)| {
            canonical_atoms[index + 1..]
                .iter()
                .any(|other| atom.function == other.function && atom.relation == other.relation)
        }) {
            return Err(GeometryFailure::DuplicateRegionCanonicalKey);
        }
        if canonical_atoms
            .iter()
            .any(|atom| !atom.function.compatible_with_dimension(ambient_dimension))
        {
            return Err(GeometryFailure::RegionAmbientDimensionMismatch);
        }
        let normalized_region_formula = normalized_region_formula.normalize()?;
        if !formula_references_known_atoms(&normalized_region_formula, &canonical_atoms) {
            return Err(GeometryFailure::MalformedRegionFormula);
        }
        Ok(Self {
            ambient_dimension,
            canonical_atoms,
            normalized_region_formula,
            region_kind,
        })
    }

    pub fn ambient_dimension(&self) -> i64 {
        self.ambient_dimension
    }

    pub fn canonical_atoms(&self) -> &[CanonicalAtom] {
        &self.canonical_atoms
    }

    pub fn normalized_region_formula(&self) -> &NormalizedRegionFormula {
        &self.normalized_region_formula
    }

    pub fn region_kind(&self) -> RegionKind {
        self.region_kind
    }

    pub fn membership(
        &self,
        request: RegionMembershipRequest,
    ) -> Result<RegionMembershipResult, GeometryFailure> {
        self.membership_at(&request.point)
    }

    pub fn membership_judgment(
        &self,
        request: MembershipJudgmentRequest,
    ) -> Result<MembershipJudgmentResult, GeometryFailure> {
        let mut candidate_points = request.candidate_points.into_iter();
        let mut judgment = None;
        let mut mismatch = false;

        for point in candidate_points.by_ref() {
            let result = self.membership_at(&point)?;
            if let Some(expected) = judgment.as_ref() {
                if expected != &result {
                    mismatch = true;
                }
            } else {
                judgment = Some(result);
            }
        }

        if mismatch {
            return Err(GeometryFailure::MalformedRegionFormula);
        }
        match judgment {
            Some(result) => Ok(result),
            None => match self.normalized_region_formula {
                NormalizedRegionFormula::True => Ok(RegionMembershipResult {
                    classification: MembershipClassification::Inside,
                    atom_signs: Vec::new(),
                }),
                NormalizedRegionFormula::False => Ok(RegionMembershipResult {
                    classification: MembershipClassification::Outside,
                    atom_signs: Vec::new(),
                }),
                _ => Err(GeometryFailure::MalformedRegionFormula),
            },
        }
    }

    pub fn set_boolean(
        &self,
        other: &Self,
        formula: NormalizedRegionFormula,
    ) -> Result<Self, GeometryFailure> {
        if self.ambient_dimension != other.ambient_dimension {
            return Err(GeometryFailure::RegionAmbientDimensionMismatch);
        }
        let mut atoms = Vec::new();
        for atom in &self.canonical_atoms {
            atoms.push(atom.try_clone()?);
        }
        for atom in &other.canonical_atoms {
            atoms.push(atom.try_clone()?);
        }
        Self::try_new(
            self.ambient_dimension,
            atoms,
            formula,
            combined_region_kind(self.region_kind, other.region_kind),
        )
    }

    pub fn solid_boolean(
        &self,
        other: &Self,
        formula: NormalizedRegionFormula,
    ) -> Result<Self, GeometryFailure> {
        if self.region_kind != RegionKind::RegularClosedVolume
            || other.region_kind != RegionKind::RegularClosedVolume
        {
            return Err(GeometryFailure::RegionKindMismatch);
        }
        let result = self.set_boolean(other, formula)?;
        Self::try_new(
            result.ambient_dimension,
            result.canonical_atoms,
            result.normalized_region_formula,
            RegionKind::RegularClosedVolume,
        )
    }

    fn membership_at(
        &self,
        point: &RegionPoint,
    ) -> Result<RegionMembershipResult, GeometryFailure> {
        if point.dimension() != self.ambient_dimension {
            return Err(GeometryFailure::RegionPointDimensionMismatch);
        }
        let (x, y, z) = point.coordinates()?;
        let mut atom_signs = Vec::new();
        for atom in &self.canonical_atoms {
            let relation = SignRelation::from_sign(
                atom.function
                    .evaluate(&x, &y, &z)?
                    .sign()
                    .map_err(GeometryFailure::AlgebraFailure)?,
            );
            atom_signs.push(AtomSignResult {
                atom_id: atom.atom_id.clone(),
                relation,
            });
        }
        let formula_true = evaluate_formula(
            &self.normalized_region_formula,
            &atom_signs,
            &self.canonical_atoms,
        )?;
        let classification = if atom_signs
            .iter()
            .any(|sign| sign.relation == SignRelation::Zero)
        {
            MembershipClassification::Boundary
        } else if formula_true {
            MembershipClassification::Inside
        } else {
            MembershipClassification::Outside
        };
        Ok(RegionMembershipResult {
            classification,
            atom_signs,
        })
    }
}

fn combined_region_kind(left: RegionKind, right: RegionKind) -> RegionKind {
    if left == RegionKind::LowerDimensionalSet && right == RegionKind::LowerDimensionalSet {
        RegionKind::LowerDimensionalSet
    } else {
        RegionKind::GeneralSemiAlgebraicSet
    }
}

fn formula_references_known_atoms(
    formula: &NormalizedRegionFormula,
    atoms: &CanonicalAtomSet,
) -> bool {
    match formula {
        NormalizedRegionFormula::Atom(atom_id) => atoms
            .binary_search_by(|atom| atom.atom_id.as_str().cmp(atom_id.as_str()))
            .is_ok(),
        NormalizedRegionFormula::Not(inner) => formula_references_known_atoms(inner, atoms),
        NormalizedRegionFormula::And(operands) | NormalizedRegionFormula::Or(operands) => operands
            .iter()
            .all(|operand| formula_references_known_atoms(operand, atoms)),
        NormalizedRegionFormula::True | NormalizedRegionFormula::False => true,
    }
}

fn evaluate_formula(
    formula: &NormalizedRegionFormula,
    atom_signs: &AtomSignResultSet,
    atoms: &CanonicalAtomSet,
) -> Result<bool, GeometryFailure> {
    match formula {
        NormalizedRegionFormula::Atom(atom_id) => {
            let atom = atoms
                .binary_search_by(|atom| atom.atom_id.as_str().cmp(atom_id.as_str()))
                .map(|index| &atoms[index])
                .map_err(|_| GeometryFailure::MalformedRegionFormula)?;
            let sign = atom_signs
                .binary_search_by(|sign| sign.atom_id.as_str().cmp(atom_id.as_str()))
                .map(|index| &atom_signs[index])
                .map_err(|_| GeometryFailure::MalformedRegionFormula)?;
            Ok(atom.relation == sign.relation)
        }
        NormalizedRegionFormula::Not(inner) => Ok(!evaluate_formula(inner, atom_signs, atoms)?),
        NormalizedRegionFormula::And(operands) => {
            operands.iter().try_fold(true, |result, operand| {
                Ok(result && evaluate_formula(operand, atom_signs, atoms)?)
            })
        }
        NormalizedRegionFormula::Or(operands) => {
            operands.iter().try_fold(false, |result, operand| {
                Ok(result || evaluate_formula(operand, atom_signs, atoms)?)
            })
        }
        NormalizedRegionFormula::True => Ok(true),
        NormalizedRegionFormula::False => Ok(false),
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum RegionPoint {
    Planar(Point2),
    Spatial(Point3),
}

impl RegionPoint {
    pub fn dimension(&self) -> i64 {
        match self {
            Self::Planar(_) => 2,
            Self::Spatial(_) => 3,
        }
    }

    fn coordinates(
        &self,
    ) -> Result<(RealAlgebraic, RealAlgebraic, RealAlgebraic), GeometryFailure> {
        match self {
            Self::Planar(point) => Ok((
                point
                    .x
                    .try_clone()
                    .map_err(GeometryFailure::AlgebraFailure)?,
                point
                    .y
                    .try_clone()
                    .map_err(GeometryFailure::AlgebraFailure)?,
                RealAlgebraic::from_integer(BigInt::zero())
                    .map_err(GeometryFailure::AlgebraFailure)?,
            )),
            Self::Spatial(point) => Ok((
                point
                    .x
                    .try_clone()
                    .map_err(GeometryFailure::AlgebraFailure)?,
                point
                    .y
                    .try_clone()
                    .map_err(GeometryFailure::AlgebraFailure)?,
                point
                    .z
                    .try_clone()
                    .map_err(GeometryFailure::AlgebraFailure)?,
            )),
        }
    }
}

pub type RegionPointSet = Vec<RegionPoint>;

#[derive(Debug, Eq, PartialEq)]
pub struct RegionMembershipRequest {
    pub region: AlgebraicRegion,
    pub point: RegionPoint,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MembershipClassification {
    Inside,
    Boundary,
    Outside,
}

#[derive(Debug, Eq, PartialEq)]
pub struct AtomSignResult {
    pub atom_id: String,
    pub relation: SignRelation,
}

pub type AtomSignResultSet = Vec<AtomSignResult>;

#[derive(Debug, Eq, PartialEq)]
pub struct RegionMembershipResult {
    pub classification: MembershipClassification,
    pub atom_signs: AtomSignResultSet,
}

pub type MembershipJudgmentResult = RegionMembershipResult;

#[derive(Debug, Eq, PartialEq)]
pub struct MembershipJudgmentRequest {
    pub region: AlgebraicRegion,
    pub candidate_points: RegionPointSet,
}

#[derive(Debug, Eq, PartialEq)]
pub struct RegionConstructionRequest {
    pub ambient_dimension: i64,
    pub canonical_atoms: CanonicalAtomSet,
    pub normalized_region_formula: NormalizedRegionFormula,
    pub region_kind: RegionKind,
}

#[derive(Debug, Eq, PartialEq)]
pub struct RegionTypes {
    region: AlgebraicRegion,
}

impl RegionTypes {
    pub fn region(&self) -> &AlgebraicRegion {
        &self.region
    }

    pub fn into_region(self) -> AlgebraicRegion {
        self.region
    }
}

pub fn construct_region_types(
    request: RegionConstructionRequest,
) -> Result<RegionTypes, GeometryFailure> {
    Ok(RegionTypes {
        region: AlgebraicRegion::try_new(
            request.ambient_dimension,
            request.canonical_atoms,
            request.normalized_region_formula,
            request.region_kind,
        )?,
    })
}

fn validate_affine_coordinate(polynomial: &RegionPolynomial3) -> Result<(), GeometryFailure> {
    for term in polynomial.terms() {
        let plane = match term.exponent_x.checked_add(term.exponent_y) {
            Some(value) => value,
            None => return Err(GeometryFailure::InvalidRegionAffineMap),
        };
        let degree = match plane.checked_add(term.exponent_z) {
            Some(value) => value,
            None => return Err(GeometryFailure::InvalidRegionAffineMap),
        };
        if degree > 1 {
            return Err(GeometryFailure::InvalidRegionAffineMap);
        }
    }
    Ok(())
}

fn constant_one() -> Result<RegionPolynomial3, GeometryFailure> {
    let one = BigInt::one().map_err(calculus_from_bigint)?;
    RegionPolynomial3::try_from_terms(vec![RegionPolynomialTerm::try_new(
        RealAlgebraic::from_integer(one).map_err(GeometryFailure::AlgebraFailure)?,
        0,
        0,
        0,
    )?])
}

fn record_exponent_operation(operations: &mut usize) -> Result<(), GeometryFailure> {
    *operations = operations.saturating_add(1);
    if *operations > MAX_REGION_EXPONENT_OPERATIONS {
        return Err(GeometryFailure::RegionExponentOperationLimit {
            requested_operations: *operations,
            maximum_operations: MAX_REGION_EXPONENT_OPERATIONS,
        });
    }
    Ok(())
}

fn add_exponents(left: u32, right: u32, operations: &mut usize) -> Result<u32, GeometryFailure> {
    record_exponent_operation(operations)?;
    left.checked_add(right)
        .ok_or(GeometryFailure::RegionExponentOverflow)
}

fn multiply_exponents(
    left: u32,
    right: u32,
    operations: &mut usize,
) -> Result<u32, GeometryFailure> {
    record_exponent_operation(operations)?;
    left.checked_mul(right)
        .ok_or(GeometryFailure::RegionExponentOverflow)
}

fn algebraic_power(
    value: &RealAlgebraic,
    exponent: &BigInt,
) -> Result<RealAlgebraic, GeometryFailure> {
    if exponent.is_zero() && value.is_zero() {
        return RealAlgebraic::from_integer(BigInt::one().map_err(calculus_from_bigint)?)
            .map_err(GeometryFailure::AlgebraFailure);
    }
    value
        .pow_integer(exponent)
        .map_err(GeometryFailure::AlgebraFailure)
}

fn calculus_from_bigint(error: BigintError) -> GeometryFailure {
    GeometryFailure::CalculusFailure {
        error: CalculusFailure::from(error),
    }
}

impl SignRelation {
    fn from_sign(sign: Sign) -> Self {
        match sign {
            Sign::Negative => Self::Negative,
            Sign::Zero => Self::Zero,
            Sign::Positive => Self::Positive,
        }
    }
}
