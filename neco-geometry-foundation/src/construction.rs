use alloc::boxed::Box;

use neco_algnum::{AlgnumError, RealAlgebraic};

use crate::types::{
    ChartDefinition2, ChartDefinition3, Direction2, Direction3, ExactGeometryConstructionRequest,
    FoundationGeometryTypes, GeometryFailure, OrthogonalMatrix2, OrthogonalMatrix3, Point2, Point3,
    Quaternion3, Transform2, Transform3, Vector2, Vector3,
};

fn algebra_failure(error: AlgnumError) -> GeometryFailure {
    GeometryFailure::AlgebraFailure(error)
}

fn add(lhs: &RealAlgebraic, rhs: &RealAlgebraic) -> Result<RealAlgebraic, GeometryFailure> {
    lhs.add(rhs).map_err(algebra_failure)
}

fn square(value: &RealAlgebraic) -> Result<RealAlgebraic, GeometryFailure> {
    value.mul(value).map_err(algebra_failure)
}

fn sum_of_squares(values: &[&RealAlgebraic]) -> Result<RealAlgebraic, GeometryFailure> {
    let mut result = square(values[0])?;
    for value in values.iter().skip(1) {
        result = add(&result, &square(value)?)?;
    }
    Ok(result)
}

fn dot(lhs: &[&RealAlgebraic], rhs: &[&RealAlgebraic]) -> Result<RealAlgebraic, GeometryFailure> {
    let mut result = lhs[0].mul(rhs[0]).map_err(algebra_failure)?;
    for (left, right) in lhs.iter().zip(rhs.iter()).skip(1) {
        result = add(&result, &left.mul(right).map_err(algebra_failure)?)?;
    }
    Ok(result)
}

fn construct_direction2(vector: Vector2) -> Result<Direction2, GeometryFailure> {
    Direction2::try_from_vector(vector)
}

fn construct_direction3(vector: Vector3) -> Result<Direction3, GeometryFailure> {
    Direction3::try_from_vector(vector)
}

fn construct_matrix2(
    m00: RealAlgebraic,
    m01: RealAlgebraic,
    m10: RealAlgebraic,
    m11: RealAlgebraic,
) -> Result<OrthogonalMatrix2, GeometryFailure> {
    let column0 = [&m00, &m10];
    let column1 = [&m01, &m11];
    if !sum_of_squares(&column0)?.is_one()
        || !sum_of_squares(&column1)?.is_one()
        || !dot(&column0, &column1)?.is_zero()
    {
        return Err(GeometryFailure::NonOrthogonalMatrix);
    }
    Ok(OrthogonalMatrix2::from_entries(m00, m01, m10, m11))
}

fn construct_matrix3(entries: [RealAlgebraic; 9]) -> Result<OrthogonalMatrix3, GeometryFailure> {
    let [m00, m01, m02, m10, m11, m12, m20, m21, m22] = &entries;
    let column0 = [m00, m10, m20];
    let column1 = [m01, m11, m21];
    let column2 = [m02, m12, m22];
    if !sum_of_squares(&column0)?.is_one()
        || !sum_of_squares(&column1)?.is_one()
        || !sum_of_squares(&column2)?.is_one()
        || !dot(&column0, &column1)?.is_zero()
        || !dot(&column0, &column2)?.is_zero()
        || !dot(&column1, &column2)?.is_zero()
    {
        return Err(GeometryFailure::NonOrthogonalMatrix);
    }
    Ok(OrthogonalMatrix3::from_entries(entries))
}

fn construct_quaternion3(
    w: RealAlgebraic,
    x: RealAlgebraic,
    y: RealAlgebraic,
    z: RealAlgebraic,
) -> Result<Quaternion3, GeometryFailure> {
    if !sum_of_squares(&[&w, &x, &y, &z])?.is_one() {
        return Err(GeometryFailure::NonUnitQuaternion);
    }
    Ok(Quaternion3::from_components(w, x, y, z))
}

pub fn construct_exact_geometry_value(
    request: ExactGeometryConstructionRequest,
) -> Result<FoundationGeometryTypes, GeometryFailure> {
    match request {
        ExactGeometryConstructionRequest::Point2 { x, y } => Ok(FoundationGeometryTypes::Point2 {
            value: Point2 { x, y },
        }),
        ExactGeometryConstructionRequest::Point3 { x, y, z } => {
            Ok(FoundationGeometryTypes::Point3 {
                value: Point3 { x, y, z },
            })
        }
        ExactGeometryConstructionRequest::Vector2 { x, y } => {
            Ok(FoundationGeometryTypes::Vector2 {
                value: Vector2 { x, y },
            })
        }
        ExactGeometryConstructionRequest::Vector3 { x, y, z } => {
            Ok(FoundationGeometryTypes::Vector3 {
                value: Vector3 { x, y, z },
            })
        }
        ExactGeometryConstructionRequest::Direction2 { vector } => {
            Ok(FoundationGeometryTypes::Direction2 {
                value: construct_direction2(vector)?,
            })
        }
        ExactGeometryConstructionRequest::Direction3 { vector } => {
            Ok(FoundationGeometryTypes::Direction3 {
                value: construct_direction3(vector)?,
            })
        }
        ExactGeometryConstructionRequest::OrthogonalMatrix2 { m00, m01, m10, m11 } => {
            Ok(FoundationGeometryTypes::OrthogonalMatrix2 {
                value: construct_matrix2(m00, m01, m10, m11)?,
            })
        }
        ExactGeometryConstructionRequest::OrthogonalMatrix3 { entries } => {
            Ok(FoundationGeometryTypes::OrthogonalMatrix3 {
                value: Box::new(construct_matrix3(*entries)?),
            })
        }
        ExactGeometryConstructionRequest::Quaternion3 { w, x, y, z } => {
            Ok(FoundationGeometryTypes::Quaternion3 {
                value: construct_quaternion3(w, x, y, z)?,
            })
        }
        ExactGeometryConstructionRequest::Transform2 {
            matrix,
            translation,
        } => Ok(FoundationGeometryTypes::Transform2 {
            value: Transform2 {
                matrix,
                translation,
            },
        }),
        ExactGeometryConstructionRequest::Transform3 {
            matrix,
            rotation,
            translation,
        } => Ok(FoundationGeometryTypes::Transform3 {
            value: Box::new(Transform3 {
                matrix: *matrix,
                rotation,
                translation,
            }),
        }),
        ExactGeometryConstructionRequest::ChartDefinition2 {
            domain_id,
            display_name,
            coefficients,
            numerator_x,
            numerator_y,
            denominator,
        } => Ok(FoundationGeometryTypes::ChartDefinition2 {
            value: ChartDefinition2::new(
                domain_id,
                display_name,
                coefficients,
                numerator_x,
                numerator_y,
                denominator,
            )?,
        }),
        ExactGeometryConstructionRequest::ChartDefinition3 {
            domain_id,
            display_name,
            coefficients,
            numerator_x,
            numerator_y,
            numerator_z,
            denominator,
        } => Ok(FoundationGeometryTypes::ChartDefinition3 {
            value: ChartDefinition3::new(
                domain_id,
                display_name,
                coefficients,
                numerator_x,
                numerator_y,
                numerator_z,
                denominator,
            )?,
        }),
    }
}

#[cfg(test)]
mod tests {
    use alloc::{boxed::Box, string::String, vec, vec::Vec};

    use neco_algnum::{AlgnumError, Polynomial, RationalPolynomial, RealAlgebraic};
    use neco_bigint::{BigInt, BigUint, Dyadic, RawRational};

    use super::*;
    use crate::{ChartDisplayName, ChartDomainId};

    fn root(coefficients: &[i32], lower: i32, upper: i32) -> RealAlgebraic {
        Polynomial::from_coefficients(
            coefficients
                .iter()
                .map(|value| BigInt::try_from(*value).unwrap())
                .collect(),
        )
        .candidate()
        .unwrap()
        .square_free()
        .unwrap()
        .factor()
        .unwrap()
        .pop()
        .unwrap()
        .certify_root(
            Dyadic::new(BigInt::try_from(lower).unwrap(), 0),
            Dyadic::new(BigInt::try_from(upper).unwrap(), 0),
        )
        .unwrap()
        .into_value()
    }

    fn empty_polynomial() -> RationalPolynomial {
        RationalPolynomial::from_coefficients(Vec::new())
    }

    fn unit_polynomial() -> RationalPolynomial {
        RationalPolynomial::from_coefficients(vec![RawRational::new(
            BigInt::one().unwrap(),
            BigUint::one().unwrap(),
        )
        .reduce()
        .unwrap()
        .into_reduced()])
    }

    fn assert_real_eq(actual: RealAlgebraic, expected: &RealAlgebraic) {
        assert_eq!(actual, expected.try_clone().unwrap());
    }

    fn assert_real_ref_eq(actual: &RealAlgebraic, expected: &RealAlgebraic) {
        assert_eq!(actual, expected);
    }

    #[test]
    fn successful_requests_preserve_all_thirteen_foundation_variants() {
        let zero = root(&[0, 1], -1, 1);
        let one = root(&[-1, 1], 0, 2);
        let two = root(&[-2, 1], 1, 3);
        let vector2 = Vector2 {
            x: two.try_clone().unwrap(),
            y: zero.try_clone().unwrap(),
        };
        let vector3 = Vector3 {
            x: two.try_clone().unwrap(),
            y: zero.try_clone().unwrap(),
            z: one.try_clone().unwrap(),
        };
        let matrix2 = (one.try_clone().unwrap(), zero.try_clone().unwrap());
        let matrix3 = (
            one.try_clone().unwrap(),
            zero.try_clone().unwrap(),
            zero.try_clone().unwrap(),
            zero.try_clone().unwrap(),
            one.try_clone().unwrap(),
            zero.try_clone().unwrap(),
            zero.try_clone().unwrap(),
            zero.try_clone().unwrap(),
            one.try_clone().unwrap(),
        );
        let quaternion = (
            one.try_clone().unwrap(),
            zero.try_clone().unwrap(),
            zero.try_clone().unwrap(),
            zero.try_clone().unwrap(),
        );

        let requests = [
            ExactGeometryConstructionRequest::Point2 {
                x: two.try_clone().unwrap(),
                y: zero.try_clone().unwrap(),
            },
            ExactGeometryConstructionRequest::Point3 {
                x: two.try_clone().unwrap(),
                y: zero.try_clone().unwrap(),
                z: one.try_clone().unwrap(),
            },
            ExactGeometryConstructionRequest::Vector2 {
                x: two.try_clone().unwrap(),
                y: zero.try_clone().unwrap(),
            },
            ExactGeometryConstructionRequest::Vector3 {
                x: two.try_clone().unwrap(),
                y: zero.try_clone().unwrap(),
                z: one.try_clone().unwrap(),
            },
            ExactGeometryConstructionRequest::Direction2 { vector: vector2 },
            ExactGeometryConstructionRequest::Direction3 { vector: vector3 },
            ExactGeometryConstructionRequest::OrthogonalMatrix2 {
                m00: matrix2.0.try_clone().unwrap(),
                m01: matrix2.1.try_clone().unwrap(),
                m10: matrix2.1.try_clone().unwrap(),
                m11: matrix2.0.try_clone().unwrap(),
            },
            ExactGeometryConstructionRequest::OrthogonalMatrix3 {
                entries: Box::new([
                    matrix3.0.try_clone().unwrap(),
                    matrix3.1.try_clone().unwrap(),
                    matrix3.2.try_clone().unwrap(),
                    matrix3.3.try_clone().unwrap(),
                    matrix3.4.try_clone().unwrap(),
                    matrix3.5.try_clone().unwrap(),
                    matrix3.6.try_clone().unwrap(),
                    matrix3.7.try_clone().unwrap(),
                    matrix3.8.try_clone().unwrap(),
                ]),
            },
            ExactGeometryConstructionRequest::Quaternion3 {
                w: quaternion.0.try_clone().unwrap(),
                x: quaternion.1.try_clone().unwrap(),
                y: quaternion.2.try_clone().unwrap(),
                z: quaternion.3.try_clone().unwrap(),
            },
            ExactGeometryConstructionRequest::Transform2 {
                matrix: construct_matrix2(
                    matrix2.0.try_clone().unwrap(),
                    matrix2.1.try_clone().unwrap(),
                    matrix2.1.try_clone().unwrap(),
                    matrix2.0.try_clone().unwrap(),
                )
                .unwrap(),
                translation: Vector2 {
                    x: two.try_clone().unwrap(),
                    y: zero.try_clone().unwrap(),
                },
            },
            ExactGeometryConstructionRequest::Transform3 {
                matrix: Box::new(
                    construct_matrix3([
                        matrix3.0.try_clone().unwrap(),
                        matrix3.1.try_clone().unwrap(),
                        matrix3.2.try_clone().unwrap(),
                        matrix3.3.try_clone().unwrap(),
                        matrix3.4.try_clone().unwrap(),
                        matrix3.5.try_clone().unwrap(),
                        matrix3.6.try_clone().unwrap(),
                        matrix3.7.try_clone().unwrap(),
                        matrix3.8.try_clone().unwrap(),
                    ])
                    .unwrap(),
                ),
                rotation: construct_quaternion3(
                    quaternion.0.try_clone().unwrap(),
                    quaternion.1.try_clone().unwrap(),
                    quaternion.2.try_clone().unwrap(),
                    quaternion.3.try_clone().unwrap(),
                )
                .unwrap(),
                translation: Vector3 {
                    x: two.try_clone().unwrap(),
                    y: zero.try_clone().unwrap(),
                    z: one.try_clone().unwrap(),
                },
            },
            ExactGeometryConstructionRequest::ChartDefinition2 {
                domain_id: ChartDomainId::new(String::from("D2")).unwrap(),
                display_name: ChartDisplayName::new(String::from("chart D2")).unwrap(),
                coefficients: empty_polynomial(),
                numerator_x: empty_polynomial(),
                numerator_y: empty_polynomial(),
                denominator: unit_polynomial(),
            },
            ExactGeometryConstructionRequest::ChartDefinition3 {
                domain_id: ChartDomainId::new(String::from("D3")).unwrap(),
                display_name: ChartDisplayName::new(String::from("chart D3")).unwrap(),
                coefficients: empty_polynomial(),
                numerator_x: empty_polynomial(),
                numerator_y: empty_polynomial(),
                numerator_z: empty_polynomial(),
                denominator: unit_polynomial(),
            },
        ];

        for request in requests {
            match construct_exact_geometry_value(request).unwrap() {
                FoundationGeometryTypes::Point2 { value } => {
                    assert_real_eq(value.x, &two);
                    assert_real_eq(value.y, &zero);
                }
                FoundationGeometryTypes::Point3 { value } => {
                    assert_real_eq(value.x, &two);
                    assert_real_eq(value.y, &zero);
                    assert_real_eq(value.z, &one);
                }
                FoundationGeometryTypes::Vector2 { value } => {
                    assert_real_eq(value.x, &two);
                    assert_real_eq(value.y, &zero);
                }
                FoundationGeometryTypes::Vector3 { value } => {
                    assert_real_eq(value.x, &two);
                    assert_real_eq(value.y, &zero);
                    assert_real_eq(value.z, &one);
                }
                FoundationGeometryTypes::Direction2 { value } => {
                    assert_real_ref_eq(&value.vector().x, &two);
                    assert_real_ref_eq(&value.vector().y, &zero);
                }
                FoundationGeometryTypes::Direction3 { value } => {
                    assert_real_ref_eq(&value.vector().x, &two);
                    assert_real_ref_eq(&value.vector().y, &zero);
                    assert_real_ref_eq(&value.vector().z, &one);
                }
                FoundationGeometryTypes::OrthogonalMatrix2 { value } => {
                    assert_eq!(
                        value,
                        construct_matrix2(
                            one.try_clone().unwrap(),
                            zero.try_clone().unwrap(),
                            zero.try_clone().unwrap(),
                            one.try_clone().unwrap(),
                        )
                        .unwrap()
                    );
                }
                FoundationGeometryTypes::OrthogonalMatrix3 { value } => {
                    assert_eq!(
                        *value,
                        construct_matrix3([
                            one.try_clone().unwrap(),
                            zero.try_clone().unwrap(),
                            zero.try_clone().unwrap(),
                            zero.try_clone().unwrap(),
                            one.try_clone().unwrap(),
                            zero.try_clone().unwrap(),
                            zero.try_clone().unwrap(),
                            zero.try_clone().unwrap(),
                            one.try_clone().unwrap(),
                        ])
                        .unwrap()
                    );
                }
                FoundationGeometryTypes::Quaternion3 { value } => {
                    assert_eq!(
                        value,
                        construct_quaternion3(
                            one.try_clone().unwrap(),
                            zero.try_clone().unwrap(),
                            zero.try_clone().unwrap(),
                            zero.try_clone().unwrap(),
                        )
                        .unwrap()
                    );
                }
                FoundationGeometryTypes::Transform2 { value } => {
                    assert_eq!(
                        value.matrix,
                        construct_matrix2(
                            one.try_clone().unwrap(),
                            zero.try_clone().unwrap(),
                            zero.try_clone().unwrap(),
                            one.try_clone().unwrap(),
                        )
                        .unwrap()
                    );
                    assert_real_eq(value.translation.x, &two);
                    assert_real_eq(value.translation.y, &zero);
                }
                FoundationGeometryTypes::Transform3 { value } => {
                    assert_eq!(
                        value.matrix,
                        construct_matrix3([
                            one.try_clone().unwrap(),
                            zero.try_clone().unwrap(),
                            zero.try_clone().unwrap(),
                            zero.try_clone().unwrap(),
                            one.try_clone().unwrap(),
                            zero.try_clone().unwrap(),
                            zero.try_clone().unwrap(),
                            zero.try_clone().unwrap(),
                            one.try_clone().unwrap(),
                        ])
                        .unwrap()
                    );
                    assert_eq!(
                        value.rotation,
                        construct_quaternion3(
                            one.try_clone().unwrap(),
                            zero.try_clone().unwrap(),
                            zero.try_clone().unwrap(),
                            zero.try_clone().unwrap(),
                        )
                        .unwrap()
                    );
                    assert_real_eq(value.translation.x, &two);
                    assert_real_eq(value.translation.y, &zero);
                    assert_real_eq(value.translation.z, &one);
                }
                FoundationGeometryTypes::ChartDefinition2 { value } => {
                    assert_eq!(value.domain_id().as_str(), "D2");
                    assert_eq!(value.display_name().as_str(), "chart D2");
                    assert_eq!(value.coefficients(), &empty_polynomial());
                    assert_eq!(value.numerator_x(), &empty_polynomial());
                    assert_eq!(value.numerator_y(), &empty_polynomial());
                    assert_eq!(value.denominator(), &unit_polynomial());
                }
                FoundationGeometryTypes::ChartDefinition3 { value } => {
                    assert_eq!(value.domain_id().as_str(), "D3");
                    assert_eq!(value.display_name().as_str(), "chart D3");
                    assert_eq!(value.coefficients(), &empty_polynomial());
                    assert_eq!(value.numerator_x(), &empty_polynomial());
                    assert_eq!(value.numerator_y(), &empty_polynomial());
                    assert_eq!(value.numerator_z(), &empty_polynomial());
                    assert_eq!(value.denominator(), &unit_polynomial());
                }
            }
        }
    }

    #[test]
    fn invalid_requests_observe_zero_direction_and_non_orthogonal_matrix_failures() {
        let zero = root(&[0, 1], -1, 1);
        let two = root(&[-2, 1], 1, 3);
        assert_eq!(
            construct_exact_geometry_value(ExactGeometryConstructionRequest::Direction2 {
                vector: Vector2 {
                    x: zero.try_clone().unwrap(),
                    y: zero.try_clone().unwrap(),
                },
            }),
            Err(GeometryFailure::ZeroDirection)
        );
        assert_eq!(
            construct_exact_geometry_value(ExactGeometryConstructionRequest::Direction3 {
                vector: Vector3 {
                    x: zero.try_clone().unwrap(),
                    y: zero.try_clone().unwrap(),
                    z: zero.try_clone().unwrap(),
                },
            }),
            Err(GeometryFailure::ZeroDirection)
        );
        assert_eq!(
            construct_exact_geometry_value(ExactGeometryConstructionRequest::OrthogonalMatrix2 {
                m00: two.try_clone().unwrap(),
                m01: zero.try_clone().unwrap(),
                m10: zero.try_clone().unwrap(),
                m11: two.try_clone().unwrap(),
            }),
            Err(GeometryFailure::NonOrthogonalMatrix)
        );
        assert_eq!(
            construct_exact_geometry_value(ExactGeometryConstructionRequest::OrthogonalMatrix3 {
                entries: Box::new([
                    two.try_clone().unwrap(),
                    zero.try_clone().unwrap(),
                    zero.try_clone().unwrap(),
                    zero.try_clone().unwrap(),
                    two.try_clone().unwrap(),
                    zero.try_clone().unwrap(),
                    zero.try_clone().unwrap(),
                    zero.try_clone().unwrap(),
                    two.try_clone().unwrap(),
                ]),
            }),
            Err(GeometryFailure::NonOrthogonalMatrix)
        );
    }

    #[test]
    fn quaternion_requests_observe_unit_success_and_non_unit_failure() {
        let zero = root(&[0, 1], -1, 1);
        let one = root(&[-1, 1], 0, 2);
        assert!(matches!(
            construct_exact_geometry_value(ExactGeometryConstructionRequest::Quaternion3 {
                w: one.try_clone().unwrap(),
                x: zero.try_clone().unwrap(),
                y: zero.try_clone().unwrap(),
                z: zero.try_clone().unwrap(),
            }),
            Ok(FoundationGeometryTypes::Quaternion3 { .. })
        ));
        assert_eq!(
            construct_exact_geometry_value(ExactGeometryConstructionRequest::Quaternion3 {
                w: one.try_clone().unwrap(),
                x: one.try_clone().unwrap(),
                y: zero.try_clone().unwrap(),
                z: zero.try_clone().unwrap(),
            }),
            Err(GeometryFailure::NonUnitQuaternion)
        );
    }

    #[test]
    fn geometry_failure_variants_preserve_the_declared_error_values() {
        let failures = [
            GeometryFailure::ZeroDirection,
            GeometryFailure::NonOrthogonalMatrix,
            GeometryFailure::NonUnitQuaternion,
            GeometryFailure::AlgebraFailure(AlgnumError::ZeroPolynomial),
        ];
        for failure in failures {
            match failure {
                GeometryFailure::ZeroDirection => {}
                GeometryFailure::NonOrthogonalMatrix => {}
                GeometryFailure::NonUnitQuaternion => {}
                GeometryFailure::AlgebraFailure(AlgnumError::ZeroPolynomial) => {}
                GeometryFailure::AlgebraFailure(_) => panic!("unexpected algebra failure"),
                GeometryFailure::InvalidChartDomainId
                | GeometryFailure::InvalidChartDisplayName
                | GeometryFailure::InvalidChartDenominator
                | GeometryFailure::ChartDomainMismatch
                | GeometryFailure::NonFiniteRayInput
                | GeometryFailure::InvalidRayTolerance
                | GeometryFailure::ZeroDirectionWithinTolerance => {
                    panic!("unexpected chart failure")
                }
            }
        }
    }
}
