use core::mem::size_of;

use neco_geometry::GeometryFailure;

const MAXIMUM_BYTES: usize = 96;

#[test]
fn geometry_failure_size_stays_within_bound() {
    let actual = size_of::<GeometryFailure>();
    assert!(
        actual <= MAXIMUM_BYTES,
        "GeometryFailure is {actual} bytes; the bound is {MAXIMUM_BYTES}"
    );
}
