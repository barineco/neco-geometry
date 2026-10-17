use crate::encoding_scalar::ScalarEncoder;
use crate::{
    EncodingBounds, EncodingFailure, EncodingLocation, EncodingPrecision, PairedSurfaceSeam,
    SurfaceOrientation,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EncodedSeamAssociation {
    parameters: [[f32; 2]; 2],
    orientations: [SurfaceOrientation; 2],
}

impl EncodedSeamAssociation {
    pub fn parameters(self) -> [[f32; 2]; 2] {
        self.parameters
    }

    pub fn orientations(self) -> [SurfaceOrientation; 2] {
        self.orientations
    }
}

pub fn encode_seam_association(
    seam: &PairedSurfaceSeam,
    precision: EncodingPrecision,
) -> Result<(EncodedSeamAssociation, EncodingBounds), EncodingFailure> {
    let mut encoder = ScalarEncoder::new(precision);
    let mut parameters = [[0.0; 2]; 2];
    let ends = [seam.first(), seam.second()];
    for (endpoint, source) in ends.into_iter().enumerate() {
        for (axis, value) in [source.parameter().u(), source.parameter().v()]
            .into_iter()
            .enumerate()
        {
            parameters[endpoint][axis] =
                encoder.algebraic(value, EncodingLocation::SeamEndpoint { endpoint, axis })?;
        }
    }
    Ok((
        EncodedSeamAssociation {
            parameters,
            orientations: [seam.first().orientation(), seam.second().orientation()],
        },
        encoder.bounds(),
    ))
}
