#![cfg_attr(not(feature = "std"), no_std)]
#![forbid(unsafe_code)]
#![doc = include_str!("../README.md")]

extern crate alloc;

mod construction;
mod types;

pub use construction::construct_exact_geometry_value;
pub use types::{
    ChartDefinition2, ChartDefinition3, ChartDisplayName, ChartDomainId, CheckedRay2, CheckedRay3,
    Direction2, Direction3, ExactGeometryConstructionRequest, FoundationGeometryTypes,
    GeometryFailure, OrthogonalMatrix2, OrthogonalMatrix3, Point2, Point3, Quaternion3, Ray2, Ray3,
    Transform2, Transform3, Vector2, Vector3,
};
