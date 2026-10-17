use neco_algnum::RealAlgebraic;
use neco_bigint::BigInt;
use neco_geometry::{
    GeometryFailure, RationalFunction3, RationalFunctionDomain3, RegionAffineMap3, RegionFunction,
    RegionPolynomial3, RegionPolynomialTerm, MAX_REGION_EXPONENT_OPERATIONS,
    MAX_REGION_POLYNOMIAL_TERMS,
};

fn algebraic(value: i32) -> RealAlgebraic {
    RealAlgebraic::from_integer(BigInt::try_from(value).unwrap()).unwrap()
}

fn term(coefficient: i32, x: u32, y: u32, z: u32) -> RegionPolynomialTerm {
    RegionPolynomialTerm::try_new(algebraic(coefficient), x, y, z).unwrap()
}

fn polynomial(terms: Vec<RegionPolynomialTerm>) -> RegionPolynomial3 {
    RegionPolynomial3::try_from_terms(terms).unwrap()
}

fn monomial(coefficient: i32, x: u32, y: u32, z: u32) -> RegionPolynomial3 {
    polynomial(vec![term(coefficient, x, y, z)])
}

fn evaluate_at(value: &RegionPolynomial3, x: i32, y: i32, z: i32) -> RealAlgebraic {
    let point_x = algebraic(x);
    let point_y = algebraic(y);
    let point_z = algebraic(z);
    let mut result = algebraic(0);
    for item in value.terms() {
        let mut monomial_value = item.coefficient().try_clone().unwrap();
        for _ in 0..item.exponent_x() {
            monomial_value = monomial_value.mul(&point_x).unwrap();
        }
        for _ in 0..item.exponent_y() {
            monomial_value = monomial_value.mul(&point_y).unwrap();
        }
        for _ in 0..item.exponent_z() {
            monomial_value = monomial_value.mul(&point_z).unwrap();
        }
        result = result.add(&monomial_value).unwrap();
    }
    result
}

fn identity_map() -> RegionAffineMap3 {
    RegionAffineMap3::try_new(
        monomial(1, 1, 0, 0),
        monomial(1, 0, 1, 0),
        monomial(1, 0, 0, 1),
    )
    .unwrap()
}

fn translation_map(tx: i32, ty: i32, tz: i32) -> RegionAffineMap3 {
    RegionAffineMap3::try_new(
        polynomial(vec![term(1, 1, 0, 0), term(tx, 0, 0, 0)]),
        polynomial(vec![term(1, 0, 1, 0), term(ty, 0, 0, 0)]),
        polynomial(vec![term(1, 0, 0, 1), term(tz, 0, 0, 0)]),
    )
    .unwrap()
}

fn linear_map() -> RegionAffineMap3 {
    RegionAffineMap3::try_new(
        polynomial(vec![term(2, 1, 0, 0), term(1, 0, 1, 0)]),
        polynomial(vec![term(1, 1, 0, 0), term(3, 0, 1, 0)]),
        monomial(1, 0, 0, 1),
    )
    .unwrap()
}

fn permutation_map() -> RegionAffineMap3 {
    RegionAffineMap3::try_new(
        monomial(1, 0, 1, 0),
        monomial(1, 0, 0, 1),
        monomial(1, 1, 0, 0),
    )
    .unwrap()
}

#[test]
fn api_surface_is_single_and_public() {
    let clone: fn(&RegionPolynomial3) -> Result<RegionPolynomial3, GeometryFailure> =
        RegionPolynomial3::try_clone;
    let add: fn(
        &RegionPolynomial3,
        &RegionPolynomial3,
    ) -> Result<RegionPolynomial3, GeometryFailure> = RegionPolynomial3::try_add;
    let multiply: fn(
        &RegionPolynomial3,
        &RegionPolynomial3,
    ) -> Result<RegionPolynomial3, GeometryFailure> = RegionPolynomial3::try_multiply;
    let scale: fn(
        &RegionPolynomial3,
        &RealAlgebraic,
    ) -> Result<RegionPolynomial3, GeometryFailure> = RegionPolynomial3::try_scale;
    let substitute: fn(
        &RegionPolynomial3,
        &RegionAffineMap3,
    ) -> Result<RegionPolynomial3, GeometryFailure> = RegionPolynomial3::try_affine_substitute;
    let rational_clone: fn(&RationalFunction3) -> Result<RationalFunction3, GeometryFailure> =
        RationalFunction3::try_clone;
    let rational_substitute: fn(
        &RationalFunction3,
        &RegionAffineMap3,
    ) -> Result<RationalFunction3, GeometryFailure> = RationalFunction3::try_affine_substitute;
    let function_clone: fn(&RegionFunction) -> Result<RegionFunction, GeometryFailure> =
        RegionFunction::try_clone;
    let function_substitute: fn(
        &RegionFunction,
        &RegionAffineMap3,
    ) -> Result<RegionFunction, GeometryFailure> = RegionFunction::try_affine_substitute;
    let map_new: fn(
        RegionPolynomial3,
        RegionPolynomial3,
        RegionPolynomial3,
    ) -> Result<RegionAffineMap3, GeometryFailure> = RegionAffineMap3::try_new;
    let _ = (
        clone,
        add,
        multiply,
        scale,
        substitute,
        rational_clone,
        rational_substitute,
        function_clone,
        function_substitute,
        map_new,
        MAX_REGION_POLYNOMIAL_TERMS,
        MAX_REGION_EXPONENT_OPERATIONS,
    );
}

#[test]
fn canonicalizes_terms_independent_of_order() {
    let first = polynomial(vec![term(1, 1, 0, 0), term(-1, 1, 0, 0)]);
    let second = polynomial(vec![term(-1, 1, 0, 0), term(1, 1, 0, 0)]);
    let zero = RegionPolynomial3::zero();
    assert_eq!(first, zero);
    assert_eq!(second, zero);
    assert_eq!(evaluate_at(&first, 2, 3, 5), algebraic(0));
    assert_eq!(evaluate_at(&second, 2, 3, 5), algebraic(0));

    let mixed = polynomial(vec![
        term(2, 0, 1, 0),
        term(0, 3, 0, 0),
        term(1, 1, 0, 0),
        term(3, 0, 1, 0),
        term(-1, 1, 0, 0),
    ]);
    let expected = polynomial(vec![term(5, 0, 1, 0)]);
    assert_eq!(mixed, expected);
    assert_eq!(evaluate_at(&mixed, 1, 2, 3), algebraic(10));
}

#[test]
fn reports_resource_and_algebra_failures() {
    let limit_terms = (0..MAX_REGION_POLYNOMIAL_TERMS)
        .map(|index| term(1, index as u32, 0, 0))
        .collect();
    assert!(RegionPolynomial3::try_from_terms(limit_terms).is_ok());

    let excess_terms = (0..=MAX_REGION_POLYNOMIAL_TERMS)
        .map(|index| term(1, index as u32, 0, 0))
        .collect();
    assert_eq!(
        RegionPolynomial3::try_from_terms(excess_terms),
        Err(GeometryFailure::RegionPolynomialTermLimit {
            requested_terms: MAX_REGION_POLYNOMIAL_TERMS + 1,
            maximum_terms: MAX_REGION_POLYNOMIAL_TERMS,
        })
    );

    let overflow_left = monomial(1, u32::MAX, 0, 0);
    let overflow_right = monomial(1, 1, 0, 0);
    assert_eq!(
        overflow_left.try_multiply(&overflow_right),
        Err(GeometryFailure::RegionExponentOverflow)
    );

    let below_limit = polynomial((0..16).map(|index| term(1, index, 0, 0)).collect())
        .try_multiply(&polynomial(
            (0..16).map(|index| term(1, 0, index, 0)).collect(),
        ))
        .unwrap();
    assert_eq!(below_limit.terms().len(), 256);
    assert_eq!(
        polynomial((0..40).map(|index| term(1, index, 0, 0)).collect()).try_multiply(&polynomial(
            (0..35).map(|index| term(1, 0, index, 0)).collect()
        )),
        Err(GeometryFailure::RegionExponentOperationLimit {
            requested_operations: MAX_REGION_EXPONENT_OPERATIONS + 1,
            maximum_operations: MAX_REGION_EXPONENT_OPERATIONS,
        })
    );

    let summed = monomial(1, 1, 0, 0).try_add(&monomial(2, 1, 0, 0)).unwrap();
    assert_eq!(summed, monomial(3, 1, 0, 0));
}

#[test]
fn validates_affine_map_coordinates() {
    let constant = RegionAffineMap3::try_new(
        monomial(4, 0, 0, 0),
        monomial(-1, 0, 0, 0),
        RegionPolynomial3::zero(),
    );
    assert!(constant.is_ok());

    let univariate = RegionAffineMap3::try_new(
        monomial(1, 1, 0, 0),
        monomial(1, 0, 1, 0),
        monomial(1, 0, 0, 1),
    );
    assert!(univariate.is_ok());

    let multivariate = RegionAffineMap3::try_new(
        polynomial(vec![term(1, 1, 0, 0), term(1, 0, 1, 0), term(1, 0, 0, 1)]),
        monomial(1, 0, 0, 0),
        monomial(1, 0, 0, 0),
    );
    assert!(multivariate.is_ok());

    let quadratic = RegionAffineMap3::try_new(
        monomial(1, 2, 0, 0),
        monomial(1, 0, 1, 0),
        monomial(1, 0, 0, 1),
    );
    assert_eq!(quadratic, Err(GeometryFailure::InvalidRegionAffineMap));

    let bilinear = RegionAffineMap3::try_new(
        monomial(1, 1, 1, 0),
        monomial(1, 0, 1, 0),
        monomial(1, 0, 0, 1),
    );
    assert_eq!(bilinear, Err(GeometryFailure::InvalidRegionAffineMap));
}

#[test]
fn preserves_pointwise_polynomial_operations() {
    let zero = RegionPolynomial3::zero();
    let constant = monomial(4, 0, 0, 0);
    let sparse = polynomial(vec![term(1, 5, 0, 0), term(-2, 0, 0, 3)]);
    let other = polynomial(vec![term(3, 1, 0, 0), term(1, 0, 2, 0)]);
    let points = [(0, 0, 0), (1, -1, 2), (2, 3, -1)];

    for (x, y, z) in points {
        let left = evaluate_at(&zero.try_add(&constant).unwrap(), x, y, z);
        let right = evaluate_at(&zero, x, y, z)
            .add(&evaluate_at(&constant, x, y, z))
            .unwrap();
        assert_eq!(left, right);

        let product = evaluate_at(&sparse.try_multiply(&other).unwrap(), x, y, z);
        let expected = evaluate_at(&sparse, x, y, z)
            .mul(&evaluate_at(&other, x, y, z))
            .unwrap();
        assert_eq!(product, expected);

        let scaled = evaluate_at(&sparse.try_scale(&algebraic(-3)).unwrap(), x, y, z);
        let expected_scale = evaluate_at(&sparse, x, y, z).mul(&algebraic(-3)).unwrap();
        assert_eq!(scaled, expected_scale);
    }
}

#[test]
fn preserves_direct_affine_evaluation() {
    let source = polynomial(vec![term(1, 2, 0, 0), term(-1, 0, 1, 0), term(2, 0, 0, 0)]);
    let points = [(0, 0, 0), (1, 2, 3), (-2, 1, 4)];

    let identity = source.try_affine_substitute(&identity_map()).unwrap();
    assert_eq!(identity, source);

    for (x, y, z) in points {
        assert_eq!(
            evaluate_at(&identity, x, y, z),
            evaluate_at(&source, x, y, z)
        );

        let translated = source
            .try_affine_substitute(&translation_map(1, -2, 3))
            .unwrap();
        assert_eq!(
            evaluate_at(&translated, x, y, z),
            evaluate_at(&source, x + 1, y - 2, z + 3)
        );

        let linear = source.try_affine_substitute(&linear_map()).unwrap();
        let mapped_x = 2 * x + y;
        let mapped_y = x + 3 * y;
        assert_eq!(
            evaluate_at(&linear, x, y, z),
            evaluate_at(&source, mapped_x, mapped_y, z)
        );

        let permuted = source.try_affine_substitute(&permutation_map()).unwrap();
        assert_eq!(
            evaluate_at(&permuted, x, y, z),
            evaluate_at(&source, y, z, x)
        );
    }
}

#[test]
fn preserves_distributivity_and_composition() {
    let left = polynomial(vec![term(1, 1, 0, 0), term(1, 0, 0, 0)]);
    let right = polynomial(vec![term(1, 0, 1, 0), term(-1, 0, 0, 0)]);
    let factor = polynomial(vec![term(2, 0, 0, 1), term(1, 1, 0, 0)]);

    let distributed = left.try_add(&right).unwrap().try_multiply(&factor).unwrap();
    let expanded = left
        .try_multiply(&factor)
        .unwrap()
        .try_add(&right.try_multiply(&factor).unwrap())
        .unwrap();
    assert_eq!(distributed, expanded);

    let first = translation_map(1, 0, 0);
    let second = linear_map();
    let composed = RegionAffineMap3::try_new(
        first.x().try_affine_substitute(&second).unwrap(),
        first.y().try_affine_substitute(&second).unwrap(),
        first.z().try_affine_substitute(&second).unwrap(),
    )
    .unwrap();
    let sequential = left
        .try_affine_substitute(&first)
        .unwrap()
        .try_affine_substitute(&second)
        .unwrap();
    let once = left.try_affine_substitute(&composed).unwrap();
    assert_eq!(sequential, once);

    let permuted = polynomial(vec![term(1, 1, 0, 0), term(1, 1, 0, 0), term(-1, 0, 0, 0)]);
    let canonical = polynomial(vec![term(2, 1, 0, 0), term(-1, 0, 0, 0)]);
    assert_eq!(permuted, canonical);
}

#[test]
fn preserves_rational_domain_and_rejects_zero_denominator() {
    let numerator = monomial(1, 1, 0, 0);
    let denominator = polynomial(vec![term(1, 0, 1, 0), term(1, 0, 0, 0)]);
    let domain = RationalFunctionDomain3::try_new(denominator.try_clone().unwrap()).unwrap();
    let function = RationalFunction3::try_new(
        numerator.try_clone().unwrap(),
        denominator.try_clone().unwrap(),
        domain,
    )
    .unwrap();
    let substituted = function.try_affine_substitute(&identity_map()).unwrap();
    assert_eq!(substituted.numerator(), function.numerator());
    assert_eq!(substituted.denominator(), function.denominator());
    assert_eq!(
        substituted.domain().non_zero_denominator(),
        function.domain().non_zero_denominator()
    );

    let collapse = RegionAffineMap3::try_new(
        RegionPolynomial3::zero(),
        monomial(1, 0, 1, 0),
        monomial(1, 0, 0, 1),
    )
    .unwrap();
    let collapsed = RationalFunction3::try_new(
        monomial(1, 0, 0, 0),
        monomial(1, 1, 0, 0),
        RationalFunctionDomain3::try_new(monomial(1, 1, 0, 0)).unwrap(),
    )
    .unwrap();
    assert_eq!(
        collapsed.try_affine_substitute(&collapse),
        Err(GeometryFailure::MalformedRegionFormula)
    );
}

#[test]
fn preserves_region_function_variants() {
    let polynomial_function = RegionFunction::Polynomial(monomial(1, 1, 0, 0));
    let rational = RegionFunction::RationalFunction(
        RationalFunction3::try_new(
            monomial(1, 0, 0, 0),
            polynomial(vec![term(1, 0, 1, 0), term(1, 0, 0, 0)]),
            RationalFunctionDomain3::try_new(polynomial(vec![term(1, 0, 1, 0), term(1, 0, 0, 0)]))
                .unwrap(),
        )
        .unwrap(),
    );

    match polynomial_function
        .try_affine_substitute(&identity_map())
        .unwrap()
    {
        RegionFunction::Polynomial(value) => assert_eq!(value, monomial(1, 1, 0, 0)),
        RegionFunction::RationalFunction(_) => panic!("polynomial variant must be preserved"),
    }
    match rational.try_affine_substitute(&identity_map()).unwrap() {
        RegionFunction::RationalFunction(value) => {
            assert_eq!(value.numerator(), &monomial(1, 0, 0, 0));
            assert_eq!(value.domain().non_zero_denominator(), value.denominator());
        }
        RegionFunction::Polynomial(_) => panic!("rational variant must be preserved"),
    }
}

#[test]
fn reports_typed_failure_without_panicking() {
    let overflow = std::panic::catch_unwind(|| {
        monomial(1, u32::MAX, 0, 0)
            .try_multiply(&monomial(1, 2, 0, 0))
            .unwrap_err()
    })
    .expect("exponent overflow must not panic");
    assert_eq!(overflow, GeometryFailure::RegionExponentOverflow);

    let terms = std::panic::catch_unwind(|| {
        RegionPolynomial3::try_from_terms(
            (0..=MAX_REGION_POLYNOMIAL_TERMS)
                .map(|index| term(1, index as u32, 0, 0))
                .collect(),
        )
        .unwrap_err()
    })
    .expect("term limit must not panic");
    assert!(matches!(
        terms,
        GeometryFailure::RegionPolynomialTermLimit { .. }
    ));

    let zero_denominator = std::panic::catch_unwind(|| {
        RationalFunctionDomain3::try_new(RegionPolynomial3::zero()).unwrap_err()
    })
    .expect("zero denominator must not panic");
    assert_eq!(zero_denominator, GeometryFailure::MalformedRegionFormula);

    let algebra = monomial(1, 0, 0, 0).try_add(&monomial(1, 0, 0, 0)).unwrap();
    assert_eq!(algebra, monomial(2, 0, 0, 0));
}
