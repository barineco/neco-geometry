use alloc::vec::Vec;

use crate::{EncodedSeamAssociation, EncodedSurfacePatch, EncodingBounds, SurfaceOrientation};

/// Supplies WGSL contact, estimated oriented contact normals, and crossing queries with finite-surface candidates.
/// Candidate bounds enclose the parameter rectangle and its encoding error.
/// Unbounded or unsupported candidate computations preserve the ray for contact evaluation.
pub const GPU_CROSSING_WGSL: &str = concat!(
    include_str!("gpu_crossing.wgsl"),
    "\n",
    include_str!("gpu_contact_predicates.wgsl"),
    "\n",
    include_str!("gpu_surface_candidates.wgsl")
);
pub const MAX_GPU_TRIM_VERTICES: usize = 16;
pub const MAX_GPU_TRIM_HOLES: usize = 8;

/// Classifies a ray result; lower evaluation failures remain separate from uncertainty.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum GpuCrossingClassification {
    Hit = 0,
    Miss = 1,
    Unknown = 2,
    LowerEvaluationFailure = 3,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GpuCrossingFailure {
    NonFiniteEncodingBound,
    InvalidEncodingBound,
    UnsupportedCoefficientForm {
        polynomial: usize,
        term: usize,
    },
    UnsupportedDenominator,
    UnsupportedNumericalRange,
    TrimCapacityExceeded {
        hole: Option<usize>,
        vertices: usize,
    },
    HoleCapacityExceeded {
        holes: usize,
    },
    SeamSideOutOfRange {
        side: usize,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GpuCrossingDomain {
    minimum: [f32; 2],
    maximum: [f32; 2],
    closed: [[bool; 2]; 2],
    disk_radius: Option<f32>,
    orientation: i32,
    encoded_error_bound: f32,
}

impl GpuCrossingDomain {
    pub fn minimum(self) -> [f32; 2] {
        self.minimum
    }
    pub fn maximum(self) -> [f32; 2] {
        self.maximum
    }
    pub fn closed(self) -> [[bool; 2]; 2] {
        self.closed
    }
    pub fn disk_radius(self) -> Option<f32> {
        self.disk_radius
    }
    pub fn orientation(self) -> i32 {
        self.orientation
    }
}
impl GpuCrossingDomain {
    pub fn uniform_bytes(self, has_trim: bool, has_seam: bool) -> [u8; 48] {
        let disk = self.disk_radius.unwrap_or(0.0);
        let flags = u32::from(self.disk_radius.is_some())
            | (u32::from(has_trim) << 1)
            | (u32::from(has_seam) << 2);
        let words = [
            self.minimum[0].to_bits(),
            self.minimum[1].to_bits(),
            self.maximum[0].to_bits(),
            self.maximum[1].to_bits(),
            u32::from(self.closed[0][0]),
            u32::from(self.closed[0][1]),
            u32::from(self.closed[1][0]),
            u32::from(self.closed[1][1]),
            disk.to_bits(),
            flags,
            self.orientation as u32,
            self.encoded_error_bound.to_bits(),
        ];
        let mut bytes = [0; 48];
        for (index, word) in words.into_iter().enumerate() {
            bytes[index * 4..index * 4 + 4].copy_from_slice(&word.to_le_bytes());
        }
        bytes
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GpuCrossingPrimitive {
    /// Stores the origin in xyz and the squared sphere radius in w; affine planes use zero in w.
    pub origin_radius: [f32; 4],
    pub axis_u_mode: [f32; 4],
    pub axis_v_pad: [f32; 4],
    pub normal_mode: [f32; 4],
}
impl GpuCrossingPrimitive {
    pub fn uniform_bytes(self) -> [u8; 64] {
        let words = self
            .origin_radius
            .into_iter()
            .chain(self.axis_u_mode)
            .chain(self.axis_v_pad)
            .chain(self.normal_mode);
        let mut bytes = [0; 64];
        for (index, value) in words.enumerate() {
            bytes[index * 4..index * 4 + 4].copy_from_slice(&value.to_le_bytes());
        }
        bytes
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct GpuCrossingTrim {
    outer: Vec<[f32; 2]>,
    holes: Vec<Vec<[f32; 2]>>,
}

impl GpuCrossingTrim {
    pub fn new(
        outer: Vec<[f32; 2]>,
        holes: Vec<Vec<[f32; 2]>>,
    ) -> Result<Self, GpuCrossingFailure> {
        if outer.len() > MAX_GPU_TRIM_VERTICES {
            return Err(GpuCrossingFailure::TrimCapacityExceeded {
                hole: None,
                vertices: outer.len(),
            });
        }
        if holes.len() > MAX_GPU_TRIM_HOLES {
            return Err(GpuCrossingFailure::HoleCapacityExceeded { holes: holes.len() });
        }
        for (hole, vertices) in holes.iter().enumerate() {
            if vertices.len() > MAX_GPU_TRIM_VERTICES {
                return Err(GpuCrossingFailure::TrimCapacityExceeded {
                    hole: Some(hole),
                    vertices: vertices.len(),
                });
            }
        }
        Ok(Self { outer, holes })
    }
    pub fn outer(&self) -> &[[f32; 2]] {
        &self.outer
    }
    pub fn holes(&self) -> &[Vec<[f32; 2]>] {
        &self.holes
    }
    pub fn uniform_bytes(&self) -> Vec<u8> {
        fn ring(vertices: &[[f32; 2]]) -> Vec<u8> {
            let mut bytes = Vec::with_capacity(272);
            for index in 0..MAX_GPU_TRIM_VERTICES {
                let point = vertices.get(index).copied().unwrap_or([0.0; 2]);
                for word in [point[0].to_bits(), point[1].to_bits(), 0, 0] {
                    bytes.extend_from_slice(&word.to_le_bytes());
                }
            }
            for word in [vertices.len() as u32, 0, 0, 0] {
                bytes.extend_from_slice(&word.to_le_bytes());
            }
            bytes
        }
        let mut bytes = ring(&self.outer);
        for word in [self.holes.len() as u32, 0, 0, 0] {
            bytes.extend_from_slice(&word.to_le_bytes());
        }
        for index in 0..MAX_GPU_TRIM_HOLES {
            bytes.extend(ring(self.holes.get(index).map_or(&[], Vec::as_slice)));
        }
        bytes
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GpuSeamClassification {
    Off,
    Contact,
    Unknown,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GpuCrossingSeam {
    endpoint: [f32; 2],
    orientation: i32,
    encoded_error_bound: f32,
}
impl GpuCrossingSeam {
    pub fn from_encoded(
        association: EncodedSeamAssociation,
        bounds: EncodingBounds,
        side: usize,
    ) -> Result<Self, GpuCrossingFailure> {
        if side > 1 {
            return Err(GpuCrossingFailure::SeamSideOutOfRange { side });
        }
        if !bounds.round_bound.is_finite() {
            return Err(GpuCrossingFailure::NonFiniteEncodingBound);
        }
        if bounds.round_bound < 0.0 {
            return Err(GpuCrossingFailure::InvalidEncodingBound);
        }
        if bounds.round_bound > 1.0e-6 {
            return Err(GpuCrossingFailure::UnsupportedNumericalRange);
        }
        let mut encoded_error_bound = bounds.round_bound as f32;
        if f64::from(encoded_error_bound) < bounds.round_bound {
            encoded_error_bound = f32::from_bits(encoded_error_bound.to_bits() + 1);
        }
        let endpoint = association.parameters()[side];
        let orientation = if association.orientations()[side] == SurfaceOrientation::Preserve {
            1
        } else {
            -1
        };
        Ok(Self {
            endpoint,
            orientation,
            encoded_error_bound,
        })
    }

    pub fn classify_parameter(self, uv: [f32; 2]) -> GpuSeamClassification {
        let dx = uv[0] - self.endpoint[0];
        let dy = uv[1] - self.endpoint[1];
        let distance_squared = dx * dx + dy * dy;
        let uncertainty = self.encoded_error_bound.max(1.0e-5);
        let contact_squared = uncertainty * uncertainty;
        if distance_squared <= contact_squared {
            GpuSeamClassification::Contact
        } else if distance_squared <= 4.0 * contact_squared {
            GpuSeamClassification::Unknown
        } else {
            GpuSeamClassification::Off
        }
    }

    pub fn uniform_bytes(self) -> [u8; 32] {
        let words = [
            self.endpoint[0].to_bits(),
            self.endpoint[1].to_bits(),
            self.encoded_error_bound.to_bits(),
            0,
            self.orientation as u32,
            self.encoded_error_bound.to_bits(),
            0,
            0,
        ];
        let mut bytes = [0; 32];
        for (index, word) in words.into_iter().enumerate() {
            bytes[index * 4..index * 4 + 4].copy_from_slice(&word.to_le_bytes());
        }
        bytes
    }
}

/// Encoded data for affine planes and rational stereographic spheres with denominator `1 + u² + v²`.
/// Supports coefficients through 65,536, encoding bounds through 1e-6, domain endpoints in `[-1, 1]`, primitives through 1e6, and ray components through 1e4.
/// Hit denotes an approximate candidate checked by residual and classification tests; Unknown is indeterminate.
#[derive(Clone, Debug, PartialEq)]
pub struct GpuCrossingSurface {
    pub primitive: GpuCrossingPrimitive,
    pub domain: GpuCrossingDomain,
    trim: GpuCrossingTrim,
    seam: Option<GpuCrossingSeam>,
}

impl GpuCrossingSurface {
    pub fn from_encoded(
        patch: &EncodedSurfacePatch,
        bounds: EncodingBounds,
    ) -> Result<Self, GpuCrossingFailure> {
        if !bounds.round_bound.is_finite() {
            return Err(GpuCrossingFailure::NonFiniteEncodingBound);
        }
        if bounds.round_bound < 0.0 {
            return Err(GpuCrossingFailure::InvalidEncodingBound);
        }
        if bounds.round_bound > 1.0e-6 {
            return Err(GpuCrossingFailure::UnsupportedNumericalRange);
        }
        let mut encoded_error_bound = bounds.round_bound as f32;
        if f64::from(encoded_error_bound) < bounds.round_bound {
            encoded_error_bound = f32::from_bits(encoded_error_bound.to_bits() + 1);
        }
        let (coefficients, domain, disk, orientation, restriction) = match patch {
            EncodedSurfacePatch::Full {
                coefficients,
                domain,
                disk,
                orientation,
            } => (coefficients, *domain, disk.is_some(), *orientation, None),
            EncodedSurfacePatch::Trimmed {
                coefficients,
                domain,
                restriction,
                orientation,
            } => (
                coefficients,
                *domain,
                false,
                *orientation,
                Some(restriction),
            ),
        };
        let trim = match restriction {
            Some(restriction) => {
                GpuCrossingTrim::new(restriction.outer().to_vec(), restriction.holes().to_vec())?
            }
            None => GpuCrossingTrim::new(Vec::new(), Vec::new())?,
        };
        let polys = coefficients.polynomials();
        let mut terms = [[0.0f32; 6]; 3];
        for (poly_index, poly) in polys[..3].iter().enumerate() {
            for (term_index, term) in poly.iter().enumerate() {
                let slot = match (term.exponent_u, term.exponent_v) {
                    (0, 0) => 0,
                    (1, 0) => 1,
                    (0, 1) => 2,
                    (2, 0) => 3,
                    (1, 1) => 4,
                    (0, 2) => 5,
                    _ => {
                        return Err(GpuCrossingFailure::UnsupportedCoefficientForm {
                            polynomial: poly_index,
                            term: term_index,
                        })
                    }
                };
                if term.coefficient.abs() > 65_536.0 {
                    return Err(GpuCrossingFailure::UnsupportedNumericalRange);
                }
                terms[poly_index][slot] += term.coefficient;
            }
        }
        let denominator = &polys[3];
        let plane_denominator = denominator.len() == 1
            && denominator[0].exponent_u == 0
            && denominator[0].exponent_v == 0
            && denominator[0].coefficient == 1.0;
        let sphere_denominator = denominator.len() == 3
            && denominator.iter().any(|term| {
                term.exponent_u == 0 && term.exponent_v == 0 && term.coefficient == 1.0
            })
            && denominator.iter().any(|term| {
                term.exponent_u == 2 && term.exponent_v == 0 && term.coefficient == 1.0
            })
            && denominator.iter().any(|term| {
                term.exponent_u == 0 && term.exponent_v == 2 && term.coefficient == 1.0
            });
        let (origin, axis_u, axis_v, normal, mode) = if plane_denominator
            && terms
                .iter()
                .all(|poly| poly[3..].iter().all(|value| *value == 0.0))
        {
            (
                [terms[0][0], terms[1][0], terms[2][0]],
                [terms[0][1], terms[1][1], terms[2][1]],
                [terms[0][2], terms[1][2], terms[2][2]],
                [0.0; 3],
                0.0,
            )
        } else if sphere_denominator {
            if terms
                .iter()
                .any(|poly| poly[4] != 0.0 || poly[3] != poly[5])
            {
                return Err(GpuCrossingFailure::UnsupportedCoefficientForm {
                    polynomial: 3,
                    term: 0,
                });
            }
            let origin = core::array::from_fn(|axis| (terms[axis][0] + terms[axis][3]) * 0.5);
            let normal = core::array::from_fn(|axis| (terms[axis][0] - terms[axis][3]) * 0.5);
            let axis_u = core::array::from_fn(|axis| terms[axis][1] * 0.5);
            let axis_v = core::array::from_fn(|axis| terms[axis][2] * 0.5);
            let dot = |left: [f32; 3], right: [f32; 3]| {
                left[0] * right[0] + left[1] * right[1] + left[2] * right[2]
            };
            let lu = dot(axis_u, axis_u);
            let lv = dot(axis_v, axis_v);
            let ln = dot(normal, normal);
            let margin = lu.max(lv).max(ln).max(1.0) * 1.0e-5;
            if lu <= 1.0e-6
                || (lu - lv).abs() > margin
                || (lu - ln).abs() > margin
                || dot(axis_u, axis_v).abs() > margin
                || dot(axis_u, normal).abs() > margin
                || dot(axis_v, normal).abs() > margin
            {
                return Err(GpuCrossingFailure::UnsupportedNumericalRange);
            }
            (origin, axis_u, axis_v, normal, 1.0)
        } else {
            return Err(GpuCrossingFailure::UnsupportedDenominator);
        };
        let endpoints = domain.endpoints();
        if endpoints
            .into_iter()
            .flatten()
            .any(|value| !value.is_finite() || value.abs() > 1.0)
        {
            return Err(GpuCrossingFailure::UnsupportedNumericalRange);
        }
        let closed = domain.closed();
        let orientation_value = if orientation == SurfaceOrientation::Preserve {
            1
        } else {
            -1
        };
        let disk_radius = if disk { Some(1.0) } else { None };
        let radius_squared = axis_u[0] * axis_u[0] + axis_u[1] * axis_u[1] + axis_u[2] * axis_u[2];
        let result = Self {
            primitive: GpuCrossingPrimitive {
                origin_radius: [
                    origin[0],
                    origin[1],
                    origin[2],
                    if mode == 1.0 { radius_squared } else { 0.0 },
                ],
                axis_u_mode: [axis_u[0], axis_u[1], axis_u[2], mode],
                axis_v_pad: [axis_v[0], axis_v[1], axis_v[2], 0.0],
                normal_mode: [normal[0], normal[1], normal[2], 0.0],
            },
            domain: GpuCrossingDomain {
                minimum: [endpoints[0][0], endpoints[1][0]],
                maximum: [endpoints[0][1], endpoints[1][1]],
                closed,
                disk_radius,
                orientation: orientation_value,
                encoded_error_bound,
            },
            trim,
            seam: None,
        };
        let values = result
            .primitive
            .origin_radius
            .into_iter()
            .chain(result.primitive.axis_u_mode)
            .chain(result.primitive.axis_v_pad)
            .chain(result.primitive.normal_mode);
        if values
            .into_iter()
            .any(|value| !value.is_finite() || value.abs() > 1.0e6)
        {
            return Err(GpuCrossingFailure::UnsupportedNumericalRange);
        }
        Ok(result)
    }
    pub fn trim(&self) -> &GpuCrossingTrim {
        &self.trim
    }
    pub fn primitive_bytes(&self) -> [u8; 64] {
        self.primitive.uniform_bytes()
    }
    pub fn domain_bytes(&self) -> [u8; 48] {
        self.domain
            .uniform_bytes(!self.trim.outer.is_empty(), self.seam.is_some())
    }
    pub fn trim_bytes(&self) -> Vec<u8> {
        self.trim.uniform_bytes()
    }
    pub fn with_seam(mut self, seam: GpuCrossingSeam) -> Self {
        self.seam = Some(seam);
        self
    }
    pub fn seam(&self) -> Option<GpuCrossingSeam> {
        self.seam
    }
    pub fn seam_bytes(&self) -> [u8; 32] {
        self.seam.map_or([0; 32], GpuCrossingSeam::uniform_bytes)
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trim_capacity_failures_report_payload_location() {
        let too_many = vec![[0.0; 2]; MAX_GPU_TRIM_VERTICES + 1];
        assert_eq!(
            GpuCrossingTrim::new(too_many, Vec::new()),
            Err(GpuCrossingFailure::TrimCapacityExceeded {
                hole: None,
                vertices: MAX_GPU_TRIM_VERTICES + 1
            })
        );
        let holes = vec![Vec::new(); MAX_GPU_TRIM_HOLES + 1];
        assert_eq!(
            GpuCrossingTrim::new(Vec::new(), holes),
            Err(GpuCrossingFailure::HoleCapacityExceeded {
                holes: MAX_GPU_TRIM_HOLES + 1
            })
        );
    }
}
