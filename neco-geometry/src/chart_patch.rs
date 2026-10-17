use core::cmp::Ordering;

use neco_algnum::{AlgnumError, RealAlgebraic};
use neco_bigint::BigInt;

use crate::{
    GeometryFailure, ParametricSurfaceChart2, SurfaceParameterDomain, TrimmedSurfaceDeclaration,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DiskConstraint;

impl DiskConstraint {
    pub fn contains(&self, u: &RealAlgebraic, v: &RealAlgebraic) -> Result<bool, GeometryFailure> {
        let one = RealAlgebraic::from_integer(BigInt::one().map_err(AlgnumError::from)?)?;
        let squared_radius = u.mul(u)?.add(&v.mul(v)?)?;
        Ok(matches!(
            squared_radius.compare(&one)?,
            Ordering::Less | Ordering::Equal
        ))
    }
}

#[derive(Debug, Eq, PartialEq)]
pub enum ChartPatch {
    Full {
        chart: ParametricSurfaceChart2,
        domain: SurfaceParameterDomain,
        disk: Option<DiskConstraint>,
    },
    Trimmed {
        declaration: TrimmedSurfaceDeclaration,
    },
}

impl ChartPatch {
    pub fn chart(&self) -> &ParametricSurfaceChart2 {
        match self {
            Self::Full { chart, .. } => chart,
            Self::Trimmed { declaration } => declaration.chart(),
        }
    }

    pub fn domain(&self) -> &SurfaceParameterDomain {
        match self {
            Self::Full { domain, .. } => domain,
            Self::Trimmed { declaration } => declaration.domain(),
        }
    }

    pub fn disk_constraint(&self) -> Option<&DiskConstraint> {
        match self {
            Self::Full { disk, .. } => disk.as_ref(),
            Self::Trimmed { .. } => None,
        }
    }

    pub fn trimmed(&self) -> Option<&TrimmedSurfaceDeclaration> {
        match self {
            Self::Full { .. } => None,
            Self::Trimmed { declaration } => Some(declaration),
        }
    }
}
