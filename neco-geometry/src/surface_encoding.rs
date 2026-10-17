use alloc::vec::Vec;

use neco_calculus::SurfacePolynomial;

use crate::encoding_scalar::ScalarEncoder;
use crate::{
    ChartPatch, EncodingBounds, EncodingFailure, EncodingLocation, EncodingPrecision,
    SurfaceOrientation, SurfaceTrimLoop2,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EncodedSurfaceTerm {
    pub coefficient: f32,
    pub exponent_u: u32,
    pub exponent_v: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EncodedCoefficients {
    polynomials: [Vec<EncodedSurfaceTerm>; 4],
}

impl EncodedCoefficients {
    pub fn polynomials(&self) -> &[Vec<EncodedSurfaceTerm>; 4] {
        &self.polynomials
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EncodedDomain {
    endpoints: [[f32; 2]; 2],
    closed: [[bool; 2]; 2],
}

impl EncodedDomain {
    pub fn endpoints(self) -> [[f32; 2]; 2] {
        self.endpoints
    }
    pub fn closed(self) -> [[bool; 2]; 2] {
        self.closed
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EncodedDiskConstraint;

#[derive(Clone, Debug, PartialEq)]
pub struct EncodedTrimRestriction {
    outer: Vec<[f32; 2]>,
    holes: Vec<Vec<[f32; 2]>>,
}

impl EncodedTrimRestriction {
    pub fn outer(&self) -> &[[f32; 2]] {
        &self.outer
    }
    pub fn holes(&self) -> &[Vec<[f32; 2]>] {
        &self.holes
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum EncodedSurfacePatch {
    Full {
        coefficients: EncodedCoefficients,
        domain: EncodedDomain,
        disk: Option<EncodedDiskConstraint>,
        orientation: SurfaceOrientation,
    },
    Trimmed {
        coefficients: EncodedCoefficients,
        domain: EncodedDomain,
        restriction: EncodedTrimRestriction,
        orientation: SurfaceOrientation,
    },
}

pub fn encode_chart_patch(
    patch: &ChartPatch,
    precision: EncodingPrecision,
) -> Result<(EncodedSurfacePatch, EncodingBounds), EncodingFailure> {
    let mut encoder = ScalarEncoder::new(precision);
    let source = patch.chart().surface();
    let coefficients = EncodedCoefficients {
        polynomials: [
            encoder.polynomial(&source.x, 0)?,
            encoder.polynomial(&source.y, 1)?,
            encoder.polynomial(&source.z, 2)?,
            encoder.polynomial(&source.denominator, 3)?,
        ],
    };
    let mut endpoints = [[0.0; 2]; 2];
    let mut closed = [[false; 2]; 2];
    for (axis, interval) in [patch.domain().u(), patch.domain().v()]
        .into_iter()
        .enumerate()
    {
        for (endpoint, value) in [interval.lower(), interval.upper()].into_iter().enumerate() {
            endpoints[axis][endpoint] = encoder.algebraic(
                value,
                EncodingLocation::DomainEndpoint {
                    axis,
                    upper: endpoint == 1,
                },
            )?;
        }
        closed[axis] = [interval.lower_closed(), interval.upper_closed()];
    }
    let domain = EncodedDomain { endpoints, closed };
    let orientation = patch.chart().orientation();
    let encoded = match patch {
        ChartPatch::Full { disk, .. } => EncodedSurfacePatch::Full {
            coefficients,
            domain,
            disk: disk.map(|_| EncodedDiskConstraint),
            orientation,
        },
        ChartPatch::Trimmed { declaration } => {
            let outer = encoder.trim_loop(declaration.outer(), None)?;
            let mut holes = Vec::with_capacity(declaration.holes().len());
            for (hole, source) in declaration.holes().iter().enumerate() {
                holes.push(encoder.trim_loop(source, Some(hole))?);
            }
            EncodedSurfacePatch::Trimmed {
                coefficients,
                domain,
                restriction: EncodedTrimRestriction { outer, holes },
                orientation,
            }
        }
    };
    Ok((encoded, encoder.bounds()))
}

impl ScalarEncoder {
    fn polynomial(
        &mut self,
        polynomial: &SurfacePolynomial,
        index: usize,
    ) -> Result<Vec<EncodedSurfaceTerm>, EncodingFailure> {
        polynomial
            .terms()
            .iter()
            .enumerate()
            .map(|(term, source)| {
                Ok(EncodedSurfaceTerm {
                    coefficient: self.rational(
                        &source.coefficient,
                        EncodingLocation::Coefficient {
                            polynomial: index,
                            term,
                        },
                    )?,
                    exponent_u: source.exponent_u,
                    exponent_v: source.exponent_v,
                })
            })
            .collect()
    }

    fn trim_loop(
        &mut self,
        source: &SurfaceTrimLoop2,
        hole: Option<usize>,
    ) -> Result<Vec<[f32; 2]>, EncodingFailure> {
        source
            .vertices()
            .iter()
            .enumerate()
            .map(|(vertex, point)| {
                Ok([
                    self.algebraic(
                        point.u(),
                        EncodingLocation::TrimVertex {
                            hole,
                            vertex,
                            axis: 0,
                        },
                    )?,
                    self.algebraic(
                        point.v(),
                        EncodingLocation::TrimVertex {
                            hole,
                            vertex,
                            axis: 1,
                        },
                    )?,
                ])
            })
            .collect()
    }
}
