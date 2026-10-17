use alloc::{boxed::Box, string::String, vec, vec::Vec};

use neco_algnum::{AlgnumError, RationalPolynomial, RealAlgebraic};

#[derive(Debug, Eq, PartialEq)]
pub struct Point2 {
    pub x: RealAlgebraic,
    pub y: RealAlgebraic,
}

#[derive(Debug, Eq, PartialEq)]
pub struct Point3 {
    pub x: RealAlgebraic,
    pub y: RealAlgebraic,
    pub z: RealAlgebraic,
}

#[derive(Debug, Eq, PartialEq)]
pub struct Vector2 {
    pub x: RealAlgebraic,
    pub y: RealAlgebraic,
}

#[derive(Debug, Eq, PartialEq)]
pub struct Vector3 {
    pub x: RealAlgebraic,
    pub y: RealAlgebraic,
    pub z: RealAlgebraic,
}

#[derive(Debug, Eq, PartialEq)]
pub struct Direction2 {
    vector: Vector2,
}

impl Direction2 {
    pub fn try_from_vector(vector: Vector2) -> Result<Self, GeometryFailure> {
        if vector.x.is_zero() && vector.y.is_zero() {
            return Err(GeometryFailure::ZeroDirection);
        }
        Ok(Self { vector })
    }

    pub fn vector(&self) -> &Vector2 {
        &self.vector
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct Direction3 {
    vector: Vector3,
}

impl Direction3 {
    pub fn try_from_vector(vector: Vector3) -> Result<Self, GeometryFailure> {
        if vector.x.is_zero() && vector.y.is_zero() && vector.z.is_zero() {
            return Err(GeometryFailure::ZeroDirection);
        }
        Ok(Self { vector })
    }

    pub fn vector(&self) -> &Vector3 {
        &self.vector
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct Ray2 {
    origin: Point2,
    direction: Direction2,
}

impl Ray2 {
    pub fn new(origin: Point2, direction: Direction2) -> Self {
        Self { origin, direction }
    }

    pub fn origin(&self) -> &Point2 {
        &self.origin
    }

    pub fn direction(&self) -> &Direction2 {
        &self.direction
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct Ray3 {
    origin: Point3,
    direction: Direction3,
}

impl Ray3 {
    pub fn new(origin: Point3, direction: Direction3) -> Self {
        Self { origin, direction }
    }

    pub fn origin(&self) -> &Point3 {
        &self.origin
    }

    pub fn direction(&self) -> &Direction3 {
        &self.direction
    }
}

#[derive(Debug, PartialEq)]
pub struct CheckedRay2 {
    origin: [f64; 2],
    direction: [f64; 2],
}

impl CheckedRay2 {
    pub fn try_new(
        origin: [f64; 2],
        direction: [f64; 2],
        tolerance: f64,
    ) -> Result<Self, GeometryFailure> {
        if !origin
            .iter()
            .chain(direction.iter())
            .all(|value| value.is_finite())
        {
            return Err(GeometryFailure::NonFiniteRayInput);
        }
        if !tolerance.is_finite() || tolerance < 0.0 {
            return Err(GeometryFailure::InvalidRayTolerance);
        }
        if direction
            .iter()
            .map(|value| value.abs())
            .fold(0.0, f64::max)
            <= tolerance
        {
            return Err(GeometryFailure::ZeroDirectionWithinTolerance);
        }
        Ok(Self { origin, direction })
    }

    pub fn origin(&self) -> &[f64; 2] {
        &self.origin
    }

    pub fn direction(&self) -> &[f64; 2] {
        &self.direction
    }
}

#[derive(Debug, PartialEq)]
pub struct CheckedRay3 {
    origin: [f64; 3],
    direction: [f64; 3],
}

impl CheckedRay3 {
    pub fn try_new(
        origin: [f64; 3],
        direction: [f64; 3],
        tolerance: f64,
    ) -> Result<Self, GeometryFailure> {
        if !origin
            .iter()
            .chain(direction.iter())
            .all(|value| value.is_finite())
        {
            return Err(GeometryFailure::NonFiniteRayInput);
        }
        if !tolerance.is_finite() || tolerance < 0.0 {
            return Err(GeometryFailure::InvalidRayTolerance);
        }
        if direction
            .iter()
            .map(|value| value.abs())
            .fold(0.0, f64::max)
            <= tolerance
        {
            return Err(GeometryFailure::ZeroDirectionWithinTolerance);
        }
        Ok(Self { origin, direction })
    }

    pub fn origin(&self) -> &[f64; 3] {
        &self.origin
    }

    pub fn direction(&self) -> &[f64; 3] {
        &self.direction
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct OrthogonalMatrix2 {
    m00: RealAlgebraic,
    m01: RealAlgebraic,
    m10: RealAlgebraic,
    m11: RealAlgebraic,
}

impl OrthogonalMatrix2 {
    pub(crate) fn from_entries(
        m00: RealAlgebraic,
        m01: RealAlgebraic,
        m10: RealAlgebraic,
        m11: RealAlgebraic,
    ) -> Self {
        Self { m00, m01, m10, m11 }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct OrthogonalMatrix3 {
    m00: RealAlgebraic,
    m01: RealAlgebraic,
    m02: RealAlgebraic,
    m10: RealAlgebraic,
    m11: RealAlgebraic,
    m12: RealAlgebraic,
    m20: RealAlgebraic,
    m21: RealAlgebraic,
    m22: RealAlgebraic,
}

impl OrthogonalMatrix3 {
    pub(crate) fn from_entries(entries: [RealAlgebraic; 9]) -> Self {
        let [m00, m01, m02, m10, m11, m12, m20, m21, m22] = entries;
        Self {
            m00,
            m01,
            m02,
            m10,
            m11,
            m12,
            m20,
            m21,
            m22,
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct Quaternion3 {
    w: RealAlgebraic,
    x: RealAlgebraic,
    y: RealAlgebraic,
    z: RealAlgebraic,
}

impl Quaternion3 {
    pub(crate) fn from_components(
        w: RealAlgebraic,
        x: RealAlgebraic,
        y: RealAlgebraic,
        z: RealAlgebraic,
    ) -> Self {
        Self { w, x, y, z }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct Transform2 {
    pub matrix: OrthogonalMatrix2,
    pub translation: Vector2,
}

#[derive(Debug, Eq, PartialEq)]
pub struct Transform3 {
    pub matrix: OrthogonalMatrix3,
    pub rotation: Quaternion3,
    pub translation: Vector3,
}

#[derive(Debug, Eq, PartialEq)]
pub struct ChartDomainId(String);

impl ChartDomainId {
    pub fn new(value: String) -> Result<Self, GeometryFailure> {
        if value.is_empty() {
            return Err(GeometryFailure::InvalidChartDomainId);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn try_clone(&self) -> Result<Self, GeometryFailure> {
        Ok(Self(self.0.clone()))
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct ChartDisplayName(String);

impl ChartDisplayName {
    pub fn new(value: String) -> Result<Self, GeometryFailure> {
        if value.is_empty() {
            return Err(GeometryFailure::InvalidChartDisplayName);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn try_clone(&self) -> Result<Self, GeometryFailure> {
        Ok(Self(self.0.clone()))
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct ChartDefinition2 {
    domain_id: ChartDomainId,
    display_name: ChartDisplayName,
    coefficients: RationalPolynomial,
    numerator_x: RationalPolynomial,
    numerator_y: RationalPolynomial,
    denominator: RationalPolynomial,
}

impl ChartDefinition2 {
    pub fn new(
        domain_id: ChartDomainId,
        display_name: ChartDisplayName,
        coefficients: RationalPolynomial,
        numerator_x: RationalPolynomial,
        numerator_y: RationalPolynomial,
        denominator: RationalPolynomial,
    ) -> Result<Self, GeometryFailure> {
        if denominator.is_zero() {
            return Err(GeometryFailure::InvalidChartDenominator);
        }
        Ok(Self {
            domain_id,
            display_name,
            coefficients,
            numerator_x,
            numerator_y,
            denominator,
        })
    }

    pub fn domain_id(&self) -> &ChartDomainId {
        &self.domain_id
    }
    pub fn display_name(&self) -> &ChartDisplayName {
        &self.display_name
    }
    pub fn coefficients(&self) -> &RationalPolynomial {
        &self.coefficients
    }
    pub fn numerator_x(&self) -> &RationalPolynomial {
        &self.numerator_x
    }
    pub fn numerator_y(&self) -> &RationalPolynomial {
        &self.numerator_y
    }
    pub fn denominator(&self) -> &RationalPolynomial {
        &self.denominator
    }

    pub fn compose(&self, inner: &Self) -> Result<Self, GeometryFailure> {
        if self.domain_id != inner.domain_id {
            return Err(GeometryFailure::ChartDomainMismatch);
        }
        Self::new(
            self.domain_id.try_clone()?,
            self.display_name.try_clone()?,
            compose_rational_polynomial(&self.coefficients, &inner.coefficients)?,
            compose_rational_polynomial(&self.numerator_x, &inner.coefficients)?,
            compose_rational_polynomial(&self.numerator_y, &inner.coefficients)?,
            compose_rational_polynomial(&self.denominator, &inner.coefficients)?,
        )
    }
}

#[derive(Debug, Eq, PartialEq)]
pub struct ChartDefinition3 {
    domain_id: ChartDomainId,
    display_name: ChartDisplayName,
    coefficients: RationalPolynomial,
    numerator_x: RationalPolynomial,
    numerator_y: RationalPolynomial,
    numerator_z: RationalPolynomial,
    denominator: RationalPolynomial,
}

impl ChartDefinition3 {
    pub fn new(
        domain_id: ChartDomainId,
        display_name: ChartDisplayName,
        coefficients: RationalPolynomial,
        numerator_x: RationalPolynomial,
        numerator_y: RationalPolynomial,
        numerator_z: RationalPolynomial,
        denominator: RationalPolynomial,
    ) -> Result<Self, GeometryFailure> {
        if denominator.is_zero() {
            return Err(GeometryFailure::InvalidChartDenominator);
        }
        Ok(Self {
            domain_id,
            display_name,
            coefficients,
            numerator_x,
            numerator_y,
            numerator_z,
            denominator,
        })
    }

    pub fn domain_id(&self) -> &ChartDomainId {
        &self.domain_id
    }
    pub fn display_name(&self) -> &ChartDisplayName {
        &self.display_name
    }
    pub fn coefficients(&self) -> &RationalPolynomial {
        &self.coefficients
    }
    pub fn numerator_x(&self) -> &RationalPolynomial {
        &self.numerator_x
    }
    pub fn numerator_y(&self) -> &RationalPolynomial {
        &self.numerator_y
    }
    pub fn numerator_z(&self) -> &RationalPolynomial {
        &self.numerator_z
    }
    pub fn denominator(&self) -> &RationalPolynomial {
        &self.denominator
    }

    pub fn compose(&self, inner: &Self) -> Result<Self, GeometryFailure> {
        if self.domain_id != inner.domain_id {
            return Err(GeometryFailure::ChartDomainMismatch);
        }
        Self::new(
            self.domain_id.try_clone()?,
            self.display_name.try_clone()?,
            compose_rational_polynomial(&self.coefficients, &inner.coefficients)?,
            compose_rational_polynomial(&self.numerator_x, &inner.coefficients)?,
            compose_rational_polynomial(&self.numerator_y, &inner.coefficients)?,
            compose_rational_polynomial(&self.numerator_z, &inner.coefficients)?,
            compose_rational_polynomial(&self.denominator, &inner.coefficients)?,
        )
    }
}

fn compose_rational_polynomial(
    outer: &RationalPolynomial,
    inner: &RationalPolynomial,
) -> Result<RationalPolynomial, GeometryFailure> {
    let mut result = RationalPolynomial::from_coefficients(Vec::new());
    for coefficient in outer.coefficients().iter().rev() {
        result = result.mul(inner).map_err(GeometryFailure::AlgebraFailure)?;
        let constant = RationalPolynomial::from_coefficients(vec![coefficient
            .try_clone()
            .map_err(|error| GeometryFailure::AlgebraFailure(error.into()))?]);
        result = result
            .add(&constant)
            .map_err(GeometryFailure::AlgebraFailure)?;
    }
    Ok(result)
}

#[derive(Debug, Eq, PartialEq)]
pub enum GeometryFailure {
    ZeroDirection,
    NonFiniteRayInput,
    InvalidRayTolerance,
    ZeroDirectionWithinTolerance,
    NonOrthogonalMatrix,
    NonUnitQuaternion,
    AlgebraFailure(AlgnumError),
    InvalidChartDomainId,
    InvalidChartDisplayName,
    InvalidChartDenominator,
    ChartDomainMismatch,
}

#[derive(Debug, Eq, PartialEq)]
pub enum ExactGeometryConstructionRequest {
    Point2 {
        x: RealAlgebraic,
        y: RealAlgebraic,
    },
    Point3 {
        x: RealAlgebraic,
        y: RealAlgebraic,
        z: RealAlgebraic,
    },
    Vector2 {
        x: RealAlgebraic,
        y: RealAlgebraic,
    },
    Vector3 {
        x: RealAlgebraic,
        y: RealAlgebraic,
        z: RealAlgebraic,
    },
    Direction2 {
        vector: Vector2,
    },
    Direction3 {
        vector: Vector3,
    },
    OrthogonalMatrix2 {
        m00: RealAlgebraic,
        m01: RealAlgebraic,
        m10: RealAlgebraic,
        m11: RealAlgebraic,
    },
    OrthogonalMatrix3 {
        entries: Box<[RealAlgebraic; 9]>,
    },
    Quaternion3 {
        w: RealAlgebraic,
        x: RealAlgebraic,
        y: RealAlgebraic,
        z: RealAlgebraic,
    },
    Transform2 {
        matrix: OrthogonalMatrix2,
        translation: Vector2,
    },
    Transform3 {
        matrix: Box<OrthogonalMatrix3>,
        rotation: Quaternion3,
        translation: Vector3,
    },
    ChartDefinition2 {
        domain_id: ChartDomainId,
        display_name: ChartDisplayName,
        coefficients: RationalPolynomial,
        numerator_x: RationalPolynomial,
        numerator_y: RationalPolynomial,
        denominator: RationalPolynomial,
    },
    ChartDefinition3 {
        domain_id: ChartDomainId,
        display_name: ChartDisplayName,
        coefficients: RationalPolynomial,
        numerator_x: RationalPolynomial,
        numerator_y: RationalPolynomial,
        numerator_z: RationalPolynomial,
        denominator: RationalPolynomial,
    },
}

#[derive(Debug, Eq, PartialEq)]
pub enum FoundationGeometryTypes {
    Point2 { value: Point2 },
    Point3 { value: Point3 },
    Vector2 { value: Vector2 },
    Vector3 { value: Vector3 },
    Direction2 { value: Direction2 },
    Direction3 { value: Direction3 },
    OrthogonalMatrix2 { value: OrthogonalMatrix2 },
    OrthogonalMatrix3 { value: Box<OrthogonalMatrix3> },
    Quaternion3 { value: Quaternion3 },
    Transform2 { value: Transform2 },
    Transform3 { value: Box<Transform3> },
    ChartDefinition2 { value: ChartDefinition2 },
    ChartDefinition3 { value: ChartDefinition3 },
}
