use neco_algnum::{AlgnumError, RealAlgebraic};
use neco_bigint::{BigUint, BigintError, Dyadic, RawRational, ReducedRational};
use neco_calculus::MAX_PRECISION_BITS;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EncodingFormat {
    F32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EncodingBounds {
    pub format: EncodingFormat,
    pub round_bound: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EncodingPrecision {
    enclosure_bits: u32,
    maximum_round_bound: f64,
}

impl EncodingPrecision {
    pub fn new(enclosure_bits: u32, maximum_round_bound: f64) -> Result<Self, EncodingFailure> {
        if enclosure_bits == 0 || enclosure_bits > MAX_PRECISION_BITS {
            return Err(EncodingFailure::PrecisionLimitExceeded {
                requested: enclosure_bits,
                maximum: MAX_PRECISION_BITS,
            });
        }
        if !maximum_round_bound.is_finite() || maximum_round_bound < 0.0 {
            return Err(EncodingFailure::InvalidRoundBound {
                requested: maximum_round_bound,
            });
        }
        Ok(Self {
            enclosure_bits,
            maximum_round_bound,
        })
    }

    pub fn enclosure_bits(self) -> u32 {
        self.enclosure_bits
    }
    pub fn maximum_round_bound(self) -> f64 {
        self.maximum_round_bound
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EncodingLocation {
    Coefficient {
        polynomial: usize,
        term: usize,
    },
    DomainEndpoint {
        axis: usize,
        upper: bool,
    },
    TrimVertex {
        hole: Option<usize>,
        vertex: usize,
        axis: usize,
    },
    SeamEndpoint {
        endpoint: usize,
        axis: usize,
    },
}

#[derive(Debug, PartialEq)]
pub enum EncodingFailure {
    PrecisionLimitExceeded {
        requested: u32,
        maximum: u32,
    },
    InvalidRoundBound {
        requested: f64,
    },
    Arithmetic {
        location: EncodingLocation,
        cause: BigintError,
    },
    Algebraic {
        location: EncodingLocation,
        cause: AlgnumError,
    },
    NonFiniteEncoding {
        location: EncodingLocation,
        approximation: f64,
    },
    PrecisionShortage {
        location: EncodingLocation,
        established: f64,
        maximum: f64,
    },
}

pub(crate) struct ScalarEncoder {
    precision: EncodingPrecision,
    round_bound: f64,
}

impl ScalarEncoder {
    pub(crate) fn new(precision: EncodingPrecision) -> Self {
        Self {
            precision,
            round_bound: 0.0,
        }
    }

    pub(crate) fn bounds(&self) -> EncodingBounds {
        EncodingBounds {
            format: EncodingFormat::F32,
            round_bound: self.round_bound,
        }
    }
    pub(crate) fn rational(
        &mut self,
        value: &ReducedRational,
        location: EncodingLocation,
    ) -> Result<f32, EncodingFailure> {
        let arithmetic = |cause| EncodingFailure::Arithmetic { location, cause };
        let lower = value
            .dyadic_floor(self.precision.enclosure_bits)
            .map_err(arithmetic)?;
        let upper = value
            .dyadic_ceil(self.precision.enclosure_bits)
            .map_err(arithmetic)?;
        self.enclosure(&lower, &upper, location, None)
    }
    pub(crate) fn algebraic(
        &mut self,
        value: &RealAlgebraic,
        location: EncodingLocation,
    ) -> Result<f32, EncodingFailure> {
        let enclosed = value
            .enclose(self.precision.enclosure_bits)
            .map_err(|cause| EncodingFailure::Algebraic { location, cause })?;
        self.enclosure(
            enclosed.enclosure().lower(),
            enclosed.enclosure().upper(),
            location,
            Some(value),
        )
    }
    fn enclosure(
        &mut self,
        lower: &Dyadic,
        upper: &Dyadic,
        location: EncodingLocation,
        source: Option<&RealAlgebraic>,
    ) -> Result<f32, EncodingFailure> {
        let arithmetic = |cause| EncodingFailure::Arithmetic { location, cause };
        let approximation = lower
            .midpoint(upper)
            .map_err(arithmetic)?
            .round_to_f64_ties_even()
            .map_err(arithmetic)?;
        let encoded = approximation as f32;
        if !encoded.is_finite() {
            return Err(EncodingFailure::NonFiniteEncoding {
                location,
                approximation,
            });
        }
        let represented = Dyadic::from_f64_exact(f64::from(encoded)).map_err(arithmetic)?;
        let distance = |endpoint: &Dyadic| {
            if &represented >= endpoint {
                represented.sub(endpoint)
            } else {
                endpoint.sub(&represented)
            }
        };
        let lower_error = distance(lower).map_err(arithmetic)?;
        let upper_error = distance(upper).map_err(arithmetic)?;
        let error = core::cmp::max(lower_error, upper_error);
        let source_matches = match source {
            Some(source) if !error.is_zero() && lower <= &represented && &represented <= upper => {
                let denominator = BigUint::one()
                    .map_err(arithmetic)?
                    .shl_bits(represented.exponent() as usize)
                    .map_err(arithmetic)?;
                let rational = RawRational::new(
                    represented.integer().try_clone().map_err(arithmetic)?,
                    denominator,
                )
                .reduce()
                .map_err(arithmetic)?
                .into_reduced();
                let algebraic = |cause| EncodingFailure::Algebraic { location, cause };
                let candidate =
                    RealAlgebraic::from_reduced_rational(&rational).map_err(algebraic)?;
                source.compare(&candidate).map_err(algebraic)?.is_eq()
            }
            _ => false,
        };
        let rounded = error.round_to_f64_ties_even().map_err(arithmetic)?;
        let established = if error.is_zero() || source_matches {
            0.0
        } else {
            f64::from_bits(rounded.to_bits() + 1)
        };
        if !established.is_finite() || established > self.precision.maximum_round_bound {
            return Err(EncodingFailure::PrecisionShortage {
                location,
                established,
                maximum: self.precision.maximum_round_bound,
            });
        }
        self.round_bound = self.round_bound.max(established);
        Ok(encoded)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use neco_bigint::BigInt;

    fn value(numerator: i32, denominator: u32) -> RealAlgebraic {
        let rational = RawRational::new(
            BigInt::try_from(numerator).unwrap(),
            BigUint::try_from(denominator).unwrap(),
        )
        .reduce()
        .unwrap()
        .into_reduced();
        RealAlgebraic::from_reduced_rational(&rational).unwrap()
    }

    #[test]
    fn exact_algebraic_candidate_satisfies_zero_round_bound() {
        let location = EncodingLocation::DomainEndpoint {
            axis: 0,
            upper: true,
        };
        for (numerator, denominator, expected) in [(-1, 1, -1.0), (1, 1, 1.0), (7, 8, 0.875)] {
            let mut encoder = ScalarEncoder::new(EncodingPrecision::new(48, 0.0).unwrap());
            assert_eq!(
                encoder
                    .algebraic(&value(numerator, denominator), location)
                    .unwrap(),
                expected
            );
            assert_eq!(encoder.bounds().round_bound, 0.0);
        }
    }

    #[test]
    fn non_dyadic_source_keeps_positive_bound_and_precision_shortage() {
        let source = value(1, 3);
        let location = EncodingLocation::SeamEndpoint {
            endpoint: 1,
            axis: 0,
        };
        let mut encoder = ScalarEncoder::new(EncodingPrecision::new(48, 1e-6).unwrap());
        let encoded = encoder.algebraic(&source, location).unwrap();
        assert_eq!(encoded, 1.0f32 / 3.0);
        assert!(encoder.bounds().round_bound > 0.0);
        let mut exact_only = ScalarEncoder::new(EncodingPrecision::new(48, 0.0).unwrap());
        assert!(matches!(
            exact_only.algebraic(&source, location),
            Err(EncodingFailure::PrecisionShortage { location: observed, maximum: 0.0, .. })
                if observed == location
        ));
    }

    #[test]
    fn precision_validation_keeps_requested_values() {
        for requested in [0, MAX_PRECISION_BITS + 1] {
            assert_eq!(
                EncodingPrecision::new(requested, 1.0),
                Err(EncodingFailure::PrecisionLimitExceeded {
                    requested,
                    maximum: MAX_PRECISION_BITS
                })
            );
        }
        for requested in [-1.0, f64::INFINITY, f64::NAN] {
            assert!(matches!(
                EncodingPrecision::new(32, requested),
                Err(EncodingFailure::InvalidRoundBound { .. })
            ));
        }
        let precision = EncodingPrecision::new(32, 0.0).unwrap();
        assert_eq!(precision.enclosure_bits(), 32);
        assert_eq!(precision.maximum_round_bound(), 0.0);
    }
}
