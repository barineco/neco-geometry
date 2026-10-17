use super::{
    algebraic, chart_identity, constant, domain, hole_loop, identity_of, interval, outer_loop,
    polynomial_chart, polynomials, rational, seam_end, stereographic_chart, surface_polynomial,
};
use neco_algnum::RealAlgebraic;
use neco_bigint::{BigUint, BigintError, Dyadic, RawRational, ReducedRational};
use neco_calculus::{SurfacePolynomial, SurfaceTerm};
use neco_geometry::{
    encode_chart_patch, encode_seam_association, ChartPatch, DiskConstraint, EncodedSurfacePatch,
    EncodingFailure, EncodingFormat, EncodingLocation, EncodingPrecision, PairedSurfaceSeam,
    ParameterInterval, ParametricSurfaceChart2, SeamEnd, SurfaceOrientation,
    SurfaceParameterDomain, SurfaceParameterPoint, TrimmedSurfaceDeclaration,
};

fn precision() -> EncodingPrecision {
    EncodingPrecision::new(48, 0.000001).unwrap()
}

fn coefficient_patch(coefficient: ReducedRational, orientation: SurfaceOrientation) -> ChartPatch {
    let (id, name, version) = chart_identity("encoded-scalar", 1);
    ChartPatch::Full {
        chart: ParametricSurfaceChart2::try_new(
            id,
            name,
            version,
            orientation,
            polynomials(
                SurfacePolynomial::try_from_terms(vec![SurfaceTerm {
                    coefficient,
                    exponent_u: 1,
                    exponent_v: 0,
                }])
                .unwrap(),
                surface_polynomial(1, 0, 1),
                constant(0),
                constant(1),
            ),
        )
        .unwrap(),
        domain: SurfaceParameterDomain::new(interval(-1, 1), interval(-1, 1)),
        disk: None,
    }
}

fn represented_rational(value: f64) -> ReducedRational {
    let dyadic = Dyadic::from_f64_exact(value).unwrap();
    RawRational::new(
        dyadic.integer().try_clone().unwrap(),
        BigUint::one()
            .unwrap()
            .shl_bits(dyadic.exponent() as usize)
            .unwrap(),
    )
    .reduce()
    .unwrap()
    .into_reduced()
}

fn assert_algebraic_error(value: &RealAlgebraic, encoded: f32, bound: f64) {
    if bound == 0.0 {
        let represented =
            RealAlgebraic::from_reduced_rational(&represented_rational(f64::from(encoded)))
                .unwrap();
        assert!(value.compare(&represented).unwrap().is_eq());
        return;
    }
    let actual = value.enclose(80).unwrap();
    let encoded = Dyadic::from_f64_exact(f64::from(encoded)).unwrap();
    let bound = Dyadic::from_f64_exact(bound).unwrap();
    assert!(encoded.sub(&bound).unwrap() <= *actual.enclosure().lower());
    assert!(encoded.add(&bound).unwrap() >= *actual.enclosure().upper());
}

#[test]
fn full_encoding_preserves_plane_sphere_disk_and_orientation() {
    for (chart, source_domain, disk) in [
        (polynomial_chart(), domain(), None),
        (
            stereographic_chart(),
            SurfaceParameterDomain::new(interval(-1, 1), interval(-1, 1)),
            Some(DiskConstraint),
        ),
    ] {
        let patch = ChartPatch::Full {
            chart,
            domain: source_domain,
            disk,
        };
        let (encoded, bounds) = encode_chart_patch(&patch, precision()).unwrap();
        assert_eq!(bounds.format, EncodingFormat::F32);
        assert!(
            bounds.round_bound.is_finite()
                && bounds.round_bound <= precision().maximum_round_bound()
        );
        match encoded {
            EncodedSurfacePatch::Full {
                coefficients,
                domain,
                disk,
                orientation,
            } => {
                assert_eq!(disk.is_some(), patch.disk_constraint().is_some());
                assert_eq!(orientation, patch.chart().orientation());
                let source = patch.chart().surface();
                for (numeric, exact) in coefficients.polynomials().iter().zip([
                    &source.x,
                    &source.y,
                    &source.z,
                    &source.denominator,
                ]) {
                    assert_eq!(numeric.len(), exact.terms().len());
                    for (numeric, exact) in numeric.iter().zip(exact.terms()) {
                        assert_eq!(
                            (numeric.exponent_u, numeric.exponent_v),
                            (exact.exponent_u, exact.exponent_v)
                        );
                        let difference = represented_rational(f64::from(numeric.coefficient))
                            .sub(&exact.coefficient)
                            .unwrap();
                        let zero = rational(0);
                        let magnitude = if difference < zero {
                            zero.sub(&difference).unwrap()
                        } else {
                            difference
                        };
                        assert!(magnitude <= represented_rational(bounds.round_bound));
                    }
                }
                for (axis, source) in [patch.domain().u(), patch.domain().v()]
                    .into_iter()
                    .enumerate()
                {
                    assert_eq!(
                        domain.closed()[axis],
                        [source.lower_closed(), source.upper_closed()]
                    );
                    assert_algebraic_error(
                        source.lower(),
                        domain.endpoints()[axis][0],
                        bounds.round_bound,
                    );
                    assert_algebraic_error(
                        source.upper(),
                        domain.endpoints()[axis][1],
                        bounds.round_bound,
                    );
                }
            }
            _ => panic!("full patch changed variant"),
        }
    }
    let patch = coefficient_patch(rational(1), SurfaceOrientation::Reverse);
    let (encoded, _) = encode_chart_patch(&patch, precision()).unwrap();
    assert!(matches!(
        encoded,
        EncodedSurfacePatch::Full {
            orientation: SurfaceOrientation::Reverse,
            ..
        }
    ));
}

#[test]
fn trimmed_encoding_preserves_outer_holes_and_vertex_order() {
    let declaration = TrimmedSurfaceDeclaration::try_new(
        polynomial_chart(),
        domain(),
        outer_loop(&[(1, 1), (7, 1), (7, 7), (1, 7)]),
        vec![hole_loop(&[(2, 2), (4, 2), (4, 4), (2, 4)])],
        vec![],
    )
    .unwrap();
    let patch = ChartPatch::Trimmed { declaration };
    let (encoded, bounds) = encode_chart_patch(&patch, precision()).unwrap();
    match encoded {
        EncodedSurfacePatch::Trimmed {
            restriction,
            domain,
            orientation,
            ..
        } => {
            let source = patch.trimmed().unwrap();
            assert_eq!(restriction.holes().len(), source.holes().len());
            assert_eq!(orientation, patch.chart().orientation());
            assert_eq!(domain.closed(), [[true; 2]; 2]);
            for (numeric, exact) in core::iter::once(restriction.outer())
                .chain(restriction.holes().iter().map(Vec::as_slice))
                .zip(core::iter::once(source.outer()).chain(source.holes()))
            {
                assert_eq!(numeric.len(), exact.vertices().len());
                for (numeric, exact) in numeric.iter().zip(exact.vertices()) {
                    assert_algebraic_error(exact.u(), numeric[0], bounds.round_bound);
                    assert_algebraic_error(exact.v(), numeric[1], bounds.round_bound);
                }
            }
        }
        _ => panic!("trimmed patch changed variant"),
    }
}

#[test]
fn encoding_bounds_cover_fraction_and_irrational_open_domain() {
    let third = rational(1).div(&rational(3)).unwrap();
    let mut patch = coefficient_patch(third.try_clone().unwrap(), SurfaceOrientation::Preserve);
    if let ChartPatch::Full { domain, .. } = &mut patch {
        *domain = SurfaceParameterDomain::new(
            ParameterInterval::new(algebraic(2).nth_root(2).unwrap(), algebraic(2), false, true)
                .unwrap(),
            interval(-1, 1),
        );
    }
    let (encoded, bounds) = encode_chart_patch(&patch, precision()).unwrap();
    match encoded {
        EncodedSurfacePatch::Full {
            coefficients,
            domain,
            ..
        } => {
            let encoded = f64::from(coefficients.polynomials()[0][0].coefficient);
            let difference = represented_rational(encoded).sub(&third).unwrap();
            assert!(difference > rational(0));
            assert!(difference <= represented_rational(bounds.round_bound));
            assert_eq!(domain.closed()[0], [false, true]);
            assert_algebraic_error(
                patch.domain().u().lower(),
                domain.endpoints()[0][0],
                bounds.round_bound,
            );
            assert!(bounds.round_bound > 0.0);
            println!(
                "fraction_and_irrational_round_bound={:.12e}",
                bounds.round_bound
            );
        }
        _ => panic!("full patch changed variant"),
    }
}

#[test]
fn encoding_failures_keep_precision_location_and_lower_cause() {
    let third = coefficient_patch(
        rational(1).div(&rational(3)).unwrap(),
        SurfaceOrientation::Preserve,
    );
    assert!(
        matches!(encode_chart_patch(&third, EncodingPrecision::new(64, 1e-12).unwrap()),
        Err(EncodingFailure::PrecisionShortage { location: EncodingLocation::Coefficient { polynomial: 0, term: 0 }, established, maximum })
        if established > maximum && maximum == 1e-12)
    );
    let huge = coefficient_patch(
        rational(2).pow_i32(200).unwrap(),
        SurfaceOrientation::Preserve,
    );
    assert!(matches!(encode_chart_patch(&huge, precision()),
        Err(EncodingFailure::NonFiniteEncoding { location: EncodingLocation::Coefficient { polynomial: 0, term: 0 }, approximation })
        if approximation.is_finite() && approximation > f64::from(f32::MAX)));
    let beyond_double = coefficient_patch(
        rational(2).pow_i32(1400).unwrap(),
        SurfaceOrientation::Preserve,
    );
    assert!(matches!(
        encode_chart_patch(&beyond_double, precision()),
        Err(EncodingFailure::Arithmetic {
            location: EncodingLocation::Coefficient {
                polynomial: 0,
                term: 0
            },
            cause: BigintError::FloatOutOfRange
        })
    ));
}

#[test]
fn seam_encoding_preserves_endpoint_order_orientation_and_error_bounds() {
    let chart = polynomial_chart();
    let seam = PairedSurfaceSeam::new(
        SeamEnd::new(
            identity_of(&chart),
            SurfaceParameterPoint::new(
                RealAlgebraic::from_reduced_rational(&rational(1).div(&rational(3)).unwrap())
                    .unwrap(),
                algebraic(2).nth_root(2).unwrap(),
            ),
            SurfaceOrientation::Preserve,
        ),
        seam_end(&chart, 7, 2, SurfaceOrientation::Reverse),
    );
    let (encoded, bounds) = encode_seam_association(&seam, precision()).unwrap();
    assert_eq!(bounds.format, EncodingFormat::F32);
    assert_eq!(
        encoded.orientations(),
        [SurfaceOrientation::Preserve, SurfaceOrientation::Reverse]
    );
    for (endpoint, exact) in [seam.first(), seam.second()].into_iter().enumerate() {
        for (axis, exact) in [exact.parameter().u(), exact.parameter().v()]
            .into_iter()
            .enumerate()
        {
            assert_algebraic_error(
                exact,
                encoded.parameters()[endpoint][axis],
                bounds.round_bound,
            );
        }
    }
    assert!(bounds.round_bound > 0.0 && bounds.round_bound <= precision().maximum_round_bound());
}

#[test]
fn seam_encoding_failures_identify_endpoint_and_axis() {
    let chart = polynomial_chart();
    let seam = PairedSurfaceSeam::new(
        seam_end(&chart, 1, 2, SurfaceOrientation::Preserve),
        SeamEnd::new(
            identity_of(&chart),
            SurfaceParameterPoint::new(
                algebraic(1),
                RealAlgebraic::from_reduced_rational(&rational(1).div(&rational(3)).unwrap())
                    .unwrap(),
            ),
            SurfaceOrientation::Reverse,
        ),
    );
    assert!(matches!(
        encode_seam_association(&seam, EncodingPrecision::new(64, 1e-12).unwrap()),
        Err(EncodingFailure::PrecisionShortage {
            location: EncodingLocation::SeamEndpoint { endpoint: 1, axis: 1 },
            established,
            maximum,
        }) if established > maximum && maximum == 1e-12
    ));
    let huge = PairedSurfaceSeam::new(
        seam_end(&chart, 1, 2, SurfaceOrientation::Preserve),
        SeamEnd::new(
            identity_of(&chart),
            SurfaceParameterPoint::new(
                RealAlgebraic::from_reduced_rational(&rational(2).pow_i32(200).unwrap()).unwrap(),
                algebraic(1),
            ),
            SurfaceOrientation::Reverse,
        ),
    );
    assert!(matches!(
        encode_seam_association(&huge, precision()),
        Err(EncodingFailure::NonFiniteEncoding {
            location: EncodingLocation::SeamEndpoint { endpoint: 1, axis: 0 },
            approximation,
        }) if approximation.is_finite() && approximation > f64::from(f32::MAX)
    ));
}
