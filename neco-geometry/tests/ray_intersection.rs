use neco_algnum::RealAlgebraic;
use neco_bigint::BigInt;
use neco_geometry::{Direction2, Point2, Ray2, Vector2};

fn algebraic(value: i32) -> RealAlgebraic {
    RealAlgebraic::from_integer(BigInt::try_from(value).unwrap()).unwrap()
}

#[test]
fn ray2_exposes_origin_and_direction() {
    let ray = Ray2::new(
        Point2 {
            x: algebraic(0),
            y: algebraic(0),
        },
        Direction2::try_from_vector(Vector2 {
            x: algebraic(1),
            y: algebraic(0),
        })
        .unwrap(),
    );
    assert_eq!(ray.origin().x, algebraic(0));
    assert_eq!(ray.direction().vector().x, algebraic(1));
}
