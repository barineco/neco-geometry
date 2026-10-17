use neco_algnum::RealAlgebraic;
use neco_bigint::BigInt;
use neco_geometry::{
    construct_region_types, AlgebraicRegion, CanonicalAtom, GeometryFailure,
    MembershipClassification, MembershipJudgmentRequest, NormalizedRegionFormula,
    RationalFunction3, RationalFunctionDomain3, RegionConstructionRequest, RegionFunction,
    RegionKind, RegionMembershipRequest, RegionPoint, RegionPolynomial3, RegionPolynomialTerm,
    SignRelation, MAX_REGION_CANONICAL_ATOMS,
};
use std::boxed::Box;

fn algebraic(value: i32) -> RealAlgebraic {
    RealAlgebraic::from_integer(BigInt::try_from(value).unwrap()).unwrap()
}

fn term(coefficient: i32, x: u32, y: u32, z: u32) -> RegionPolynomialTerm {
    RegionPolynomialTerm::try_new(algebraic(coefficient), x, y, z).unwrap()
}

fn polynomial(terms: Vec<RegionPolynomialTerm>) -> RegionPolynomial3 {
    RegionPolynomial3::try_from_terms(terms).unwrap()
}

fn atom(id: &str, polynomial: RegionPolynomial3, relation: SignRelation) -> CanonicalAtom {
    CanonicalAtom::try_new(id, RegionFunction::Polynomial(polynomial), relation).unwrap()
}

fn point(x: i32, y: i32, z: i32) -> RegionPoint {
    RegionPoint::Spatial(neco_geometry::Point3 {
        x: algebraic(x),
        y: algebraic(y),
        z: algebraic(z),
    })
}

#[test]
fn canonical_atom_limit_rejects_the_first_unbounded_input() {
    let atoms = (0..=MAX_REGION_CANONICAL_ATOMS)
        .map(|index| {
            atom(
                &format!("atom-{index:03}"),
                polynomial(vec![term(1, 1, 0, 0)]),
                SignRelation::Positive,
            )
        })
        .collect();
    assert_eq!(
        AlgebraicRegion::try_new(
            3,
            atoms,
            NormalizedRegionFormula::True,
            RegionKind::GeneralSemiAlgebraicSet,
        ),
        Err(GeometryFailure::RegionCanonicalAtomLimit {
            requested_atoms: MAX_REGION_CANONICAL_ATOMS + 1,
            maximum_atoms: MAX_REGION_CANONICAL_ATOMS,
        })
    );
}

fn region_request_with_kind(
    atoms: Vec<CanonicalAtom>,
    formula: NormalizedRegionFormula,
    region_kind: RegionKind,
) -> RegionConstructionRequest {
    RegionConstructionRequest {
        ambient_dimension: 3,
        canonical_atoms: atoms,
        normalized_region_formula: formula,
        region_kind,
    }
}

fn region_request(
    atoms: Vec<CanonicalAtom>,
    formula: NormalizedRegionFormula,
) -> RegionConstructionRequest {
    region_request_with_kind(atoms, formula, RegionKind::RegularClosedVolume)
}

fn make_region(atoms: Vec<CanonicalAtom>, formula: NormalizedRegionFormula) -> AlgebraicRegion {
    construct_region_types(region_request(atoms, formula))
        .unwrap()
        .into_region()
}

fn make_region_of_kind(
    atoms: Vec<CanonicalAtom>,
    formula: NormalizedRegionFormula,
    region_kind: RegionKind,
) -> AlgebraicRegion {
    construct_region_types(region_request_with_kind(atoms, formula, region_kind))
        .unwrap()
        .into_region()
}

#[test]
fn construct_region_types_rejects_invalid_dimension() {
    let result = construct_region_types(RegionConstructionRequest {
        ambient_dimension: 4,
        canonical_atoms: Vec::new(),
        normalized_region_formula: NormalizedRegionFormula::True,
        region_kind: RegionKind::GeneralSemiAlgebraicSet,
    });
    assert_eq!(result, Err(GeometryFailure::RegionAmbientDimensionMismatch));
}

#[test]
fn canonical_atom_permutation_and_duplicates() {
    let first = make_region(
        vec![
            atom(
                "b",
                polynomial(vec![term(1, 0, 1, 0)]),
                SignRelation::Positive,
            ),
            atom(
                "a",
                polynomial(vec![term(1, 1, 0, 0)]),
                SignRelation::Positive,
            ),
        ],
        NormalizedRegionFormula::And(vec![
            NormalizedRegionFormula::Atom("b".into()),
            NormalizedRegionFormula::Atom("a".into()),
        ]),
    );
    let second = make_region(
        vec![
            atom(
                "a",
                polynomial(vec![term(1, 1, 0, 0)]),
                SignRelation::Positive,
            ),
            atom(
                "b",
                polynomial(vec![term(1, 0, 1, 0)]),
                SignRelation::Positive,
            ),
        ],
        NormalizedRegionFormula::And(vec![
            NormalizedRegionFormula::Atom("a".into()),
            NormalizedRegionFormula::Atom("b".into()),
        ]),
    );
    assert_eq!(first, second);

    let duplicate = construct_region_types(region_request(
        vec![
            atom(
                "a",
                polynomial(vec![term(1, 1, 0, 0)]),
                SignRelation::Positive,
            ),
            atom(
                "a",
                polynomial(vec![term(1, 0, 1, 0)]),
                SignRelation::Positive,
            ),
        ],
        NormalizedRegionFormula::Atom("a".into()),
    ));
    assert_eq!(duplicate, Err(GeometryFailure::DuplicateRegionAtom));
}

#[test]
fn normalized_formula_is_deterministic() {
    let first = NormalizedRegionFormula::And(vec![
        NormalizedRegionFormula::Atom("b".into()),
        NormalizedRegionFormula::Atom("a".into()),
        NormalizedRegionFormula::Atom("a".into()),
    ])
    .normalize()
    .unwrap();
    let second = NormalizedRegionFormula::And(vec![
        NormalizedRegionFormula::Atom("a".into()),
        NormalizedRegionFormula::Atom("b".into()),
    ])
    .normalize()
    .unwrap();
    assert_eq!(first, second);
    assert_eq!(
        NormalizedRegionFormula::And(vec![NormalizedRegionFormula::True])
            .normalize()
            .unwrap(),
        NormalizedRegionFormula::True
    );
}

fn boolean_region(formula: NormalizedRegionFormula) -> AlgebraicRegion {
    make_region(
        vec![
            atom(
                "x",
                polynomial(vec![term(1, 1, 0, 0)]),
                SignRelation::Positive,
            ),
            atom(
                "y",
                polynomial(vec![term(1, 0, 1, 0)]),
                SignRelation::Positive,
            ),
        ],
        formula,
    )
}

fn boolean_membership(
    formula: NormalizedRegionFormula,
    candidate: RegionPoint,
) -> MembershipClassification {
    let region = boolean_region(formula.clone());
    region
        .membership(RegionMembershipRequest {
            region: boolean_region(formula),
            point: candidate,
        })
        .unwrap()
        .classification
}

#[test]
fn region_boolean_laws_preserve_membership_semantics() {
    let x = || NormalizedRegionFormula::Atom("x".into());
    let y = || NormalizedRegionFormula::Atom("y".into());

    assert_eq!(
        NormalizedRegionFormula::And(vec![x(), y()])
            .normalize()
            .unwrap(),
        NormalizedRegionFormula::And(vec![y(), x()])
            .normalize()
            .unwrap()
    );
    assert_eq!(
        NormalizedRegionFormula::Or(vec![x(), y()])
            .normalize()
            .unwrap(),
        NormalizedRegionFormula::Or(vec![y(), x()])
            .normalize()
            .unwrap()
    );
    assert_eq!(
        NormalizedRegionFormula::And(vec![x(), x()])
            .normalize()
            .unwrap(),
        x()
    );
    assert_eq!(
        NormalizedRegionFormula::Or(vec![x(), x()])
            .normalize()
            .unwrap(),
        x()
    );
    assert_eq!(
        NormalizedRegionFormula::And(vec![x(), NormalizedRegionFormula::True])
            .normalize()
            .unwrap(),
        x()
    );
    assert_eq!(
        NormalizedRegionFormula::Or(vec![x(), NormalizedRegionFormula::False])
            .normalize()
            .unwrap(),
        x()
    );
    assert_eq!(
        NormalizedRegionFormula::And(vec![x(), NormalizedRegionFormula::False])
            .normalize()
            .unwrap(),
        NormalizedRegionFormula::False
    );
    assert_eq!(
        NormalizedRegionFormula::Or(vec![x(), NormalizedRegionFormula::True])
            .normalize()
            .unwrap(),
        NormalizedRegionFormula::True
    );

    for (point_x, point_y) in [(1, 1), (1, -1), (-1, 1), (-1, -1), (0, 1)] {
        let not_and =
            NormalizedRegionFormula::Not(Box::new(NormalizedRegionFormula::And(vec![x(), y()])));
        let or_not = NormalizedRegionFormula::Or(vec![
            NormalizedRegionFormula::Not(Box::new(x())),
            NormalizedRegionFormula::Not(Box::new(y())),
        ]);
        assert_eq!(
            boolean_membership(not_and, point(point_x, point_y, 0)),
            boolean_membership(or_not, point(point_x, point_y, 0))
        );

        let not_or =
            NormalizedRegionFormula::Not(Box::new(NormalizedRegionFormula::Or(vec![x(), y()])));
        let and_not = NormalizedRegionFormula::And(vec![
            NormalizedRegionFormula::Not(Box::new(x())),
            NormalizedRegionFormula::Not(Box::new(y())),
        ]);
        assert_eq!(
            boolean_membership(not_or, point(point_x, point_y, 0)),
            boolean_membership(and_not, point(point_x, point_y, 0))
        );
    }
}

#[test]
fn exact_membership_classification() {
    let region = make_region(
        vec![atom(
            "x_positive",
            polynomial(vec![term(1, 1, 0, 0)]),
            SignRelation::Positive,
        )],
        NormalizedRegionFormula::Atom("x_positive".into()),
    );
    for (candidate, expected) in [
        (point(1, 0, 0), MembershipClassification::Inside),
        (point(0, 0, 0), MembershipClassification::Boundary),
        (point(-1, 0, 0), MembershipClassification::Outside),
    ] {
        assert_eq!(
            region
                .membership(RegionMembershipRequest {
                    region: make_region(
                        vec![atom(
                            "x_positive",
                            polynomial(vec![term(1, 1, 0, 0)]),
                            SignRelation::Positive,
                        )],
                        NormalizedRegionFormula::Atom("x_positive".into()),
                    ),
                    point: candidate,
                })
                .unwrap()
                .classification,
            expected
        );
    }
}

#[test]
fn sphere_box_and_difference_membership() {
    let sphere = make_region(
        vec![atom(
            "sphere",
            polynomial(vec![
                term(1, 2, 0, 0),
                term(1, 0, 2, 0),
                term(1, 0, 0, 2),
                term(-1, 0, 0, 0),
            ]),
            SignRelation::Negative,
        )],
        NormalizedRegionFormula::Atom("sphere".into()),
    );
    for (candidate, expected) in [
        (point(0, 0, 0), MembershipClassification::Inside),
        (point(1, 0, 0), MembershipClassification::Boundary),
        (point(2, 0, 0), MembershipClassification::Outside),
    ] {
        assert_eq!(
            sphere
                .membership(RegionMembershipRequest {
                    region: make_region(
                        vec![atom(
                            "sphere",
                            polynomial(vec![
                                term(1, 2, 0, 0),
                                term(1, 0, 2, 0),
                                term(1, 0, 0, 2),
                                term(-1, 0, 0, 0),
                            ]),
                            SignRelation::Negative,
                        )],
                        NormalizedRegionFormula::Atom("sphere".into()),
                    ),
                    point: candidate,
                })
                .unwrap()
                .classification,
            expected
        );
    }

    let box_region = make_region(
        vec![
            atom(
                "x_lower",
                polynomial(vec![term(-1, 1, 0, 0), term(-1, 0, 0, 0)]),
                SignRelation::Negative,
            ),
            atom(
                "x_upper",
                polynomial(vec![term(1, 1, 0, 0), term(-1, 0, 0, 0)]),
                SignRelation::Negative,
            ),
            atom(
                "y_lower",
                polynomial(vec![term(-1, 0, 1, 0), term(-1, 0, 0, 0)]),
                SignRelation::Negative,
            ),
            atom(
                "y_upper",
                polynomial(vec![term(1, 0, 1, 0), term(-1, 0, 0, 0)]),
                SignRelation::Negative,
            ),
            atom(
                "z_lower",
                polynomial(vec![term(-1, 0, 0, 1), term(-1, 0, 0, 0)]),
                SignRelation::Negative,
            ),
            atom(
                "z_upper",
                polynomial(vec![term(1, 0, 0, 1), term(-1, 0, 0, 0)]),
                SignRelation::Negative,
            ),
        ],
        NormalizedRegionFormula::And(vec![
            NormalizedRegionFormula::Atom("x_lower".into()),
            NormalizedRegionFormula::Atom("x_upper".into()),
            NormalizedRegionFormula::Atom("y_lower".into()),
            NormalizedRegionFormula::Atom("y_upper".into()),
            NormalizedRegionFormula::Atom("z_lower".into()),
            NormalizedRegionFormula::Atom("z_upper".into()),
        ]),
    );
    assert_eq!(
        box_region
            .membership(RegionMembershipRequest {
                region: make_region_of_kind(
                    vec![
                        atom(
                            "x_lower",
                            polynomial(vec![term(-1, 1, 0, 0), term(-1, 0, 0, 0)]),
                            SignRelation::Negative,
                        ),
                        atom(
                            "x_upper",
                            polynomial(vec![term(1, 1, 0, 0), term(-1, 0, 0, 0)]),
                            SignRelation::Negative,
                        ),
                        atom(
                            "y_lower",
                            polynomial(vec![term(-1, 0, 1, 0), term(-1, 0, 0, 0)]),
                            SignRelation::Negative,
                        ),
                        atom(
                            "y_upper",
                            polynomial(vec![term(1, 0, 1, 0), term(-1, 0, 0, 0)]),
                            SignRelation::Negative,
                        ),
                        atom(
                            "z_lower",
                            polynomial(vec![term(-1, 0, 0, 1), term(-1, 0, 0, 0)]),
                            SignRelation::Negative,
                        ),
                        atom(
                            "z_upper",
                            polynomial(vec![term(1, 0, 0, 1), term(-1, 0, 0, 0)]),
                            SignRelation::Negative,
                        ),
                    ],
                    NormalizedRegionFormula::And(vec![
                        NormalizedRegionFormula::Atom("x_lower".into()),
                        NormalizedRegionFormula::Atom("x_upper".into()),
                        NormalizedRegionFormula::Atom("y_lower".into()),
                        NormalizedRegionFormula::Atom("y_upper".into()),
                        NormalizedRegionFormula::Atom("z_lower".into()),
                        NormalizedRegionFormula::Atom("z_upper".into()),
                    ]),
                    RegionKind::RegularClosedVolume,
                ),
                point: point(0, 0, 0),
            })
            .unwrap()
            .classification,
        MembershipClassification::Inside
    );
    assert_eq!(
        box_region
            .membership(RegionMembershipRequest {
                region: make_region_of_kind(
                    vec![
                        atom(
                            "x_lower",
                            polynomial(vec![term(-1, 1, 0, 0), term(-1, 0, 0, 0)]),
                            SignRelation::Negative,
                        ),
                        atom(
                            "x_upper",
                            polynomial(vec![term(1, 1, 0, 0), term(-1, 0, 0, 0)]),
                            SignRelation::Negative,
                        ),
                        atom(
                            "y_lower",
                            polynomial(vec![term(-1, 0, 1, 0), term(-1, 0, 0, 0)]),
                            SignRelation::Negative,
                        ),
                        atom(
                            "y_upper",
                            polynomial(vec![term(1, 0, 1, 0), term(-1, 0, 0, 0)]),
                            SignRelation::Negative,
                        ),
                        atom(
                            "z_lower",
                            polynomial(vec![term(-1, 0, 0, 1), term(-1, 0, 0, 0)]),
                            SignRelation::Negative,
                        ),
                        atom(
                            "z_upper",
                            polynomial(vec![term(1, 0, 0, 1), term(-1, 0, 0, 0)]),
                            SignRelation::Negative,
                        ),
                    ],
                    NormalizedRegionFormula::And(vec![
                        NormalizedRegionFormula::Atom("x_lower".into()),
                        NormalizedRegionFormula::Atom("x_upper".into()),
                        NormalizedRegionFormula::Atom("y_lower".into()),
                        NormalizedRegionFormula::Atom("y_upper".into()),
                        NormalizedRegionFormula::Atom("z_lower".into()),
                        NormalizedRegionFormula::Atom("z_upper".into()),
                    ]),
                    RegionKind::RegularClosedVolume,
                ),
                point: point(2, 0, 0),
            })
            .unwrap()
            .classification,
        MembershipClassification::Outside
    );

    let difference = sphere
        .set_boolean(
            &box_region,
            NormalizedRegionFormula::And(vec![
                NormalizedRegionFormula::Atom("sphere".into()),
                NormalizedRegionFormula::Not(Box::new(NormalizedRegionFormula::Atom(
                    "x_lower".into(),
                ))),
            ]),
        )
        .unwrap();
    assert_eq!(
        difference.region_kind(),
        RegionKind::GeneralSemiAlgebraicSet
    );
    assert_eq!(
        difference.normalized_region_formula(),
        &NormalizedRegionFormula::And(vec![
            NormalizedRegionFormula::Atom("sphere".into()),
            NormalizedRegionFormula::Not(Box::new(
                NormalizedRegionFormula::Atom("x_lower".into(),)
            )),
        ])
    );
}

#[test]
fn caller_candidate_set_is_preserved() {
    let region = make_region(
        vec![atom(
            "x",
            polynomial(vec![term(1, 1, 0, 0)]),
            SignRelation::Positive,
        )],
        NormalizedRegionFormula::Atom("x".into()),
    );
    assert_eq!(
        region
            .membership_judgment(MembershipJudgmentRequest {
                region: make_region(
                    vec![atom(
                        "x",
                        polynomial(vec![term(1, 1, 0, 0)]),
                        SignRelation::Positive,
                    )],
                    NormalizedRegionFormula::Atom("x".into()),
                ),
                candidate_points: vec![point(1, 0, 0), point(2, 0, 0)],
            })
            .unwrap()
            .classification,
        MembershipClassification::Inside
    );
    assert_eq!(
        region.membership_judgment(MembershipJudgmentRequest {
            region: make_region(
                vec![atom(
                    "x",
                    polynomial(vec![term(1, 1, 0, 0)]),
                    SignRelation::Positive,
                )],
                NormalizedRegionFormula::Atom("x".into()),
            ),
            candidate_points: vec![point(1, 0, 0), point(-1, 0, 0)],
        }),
        Err(GeometryFailure::MalformedRegionFormula)
    );

    let true_region = make_region(Vec::new(), NormalizedRegionFormula::True);
    assert_eq!(
        true_region.membership_judgment(MembershipJudgmentRequest {
            region: make_region(Vec::new(), NormalizedRegionFormula::True),
            candidate_points: Vec::new(),
        }),
        Ok(neco_geometry::RegionMembershipResult {
            classification: MembershipClassification::Inside,
            atom_signs: Vec::new(),
        })
    );
    let false_region = make_region(Vec::new(), NormalizedRegionFormula::False);
    assert_eq!(
        false_region.membership_judgment(MembershipJudgmentRequest {
            region: make_region(Vec::new(), NormalizedRegionFormula::False),
            candidate_points: Vec::new(),
        }),
        Ok(neco_geometry::RegionMembershipResult {
            classification: MembershipClassification::Outside,
            atom_signs: Vec::new(),
        })
    );
}

fn rational_region(
    atom_id: &str,
    numerator: i32,
    denominator_terms: Vec<RegionPolynomialTerm>,
) -> AlgebraicRegion {
    let denominator = polynomial(denominator_terms);
    make_region(
        vec![CanonicalAtom::try_new(
            atom_id,
            RegionFunction::RationalFunction(
                RationalFunction3::try_new(
                    polynomial(vec![term(numerator, 0, 0, 0)]),
                    denominator,
                    RationalFunctionDomain3::try_new(polynomial(vec![term(1, 1, 0, 0)])).unwrap(),
                )
                .unwrap(),
            ),
            SignRelation::Positive,
        )
        .unwrap()],
        NormalizedRegionFormula::Atom(atom_id.into()),
    )
}

#[test]
fn canonical_atom_owned_snapshot_preserves_polynomial_and_rational_functions() {
    let polynomial_atom = atom(
        "polynomial",
        polynomial(vec![term(2, 2, 0, 0), term(-3, 0, 0, 0)]),
        SignRelation::Negative,
    );
    let polynomial_snapshot = polynomial_atom.try_clone().unwrap();
    assert_eq!(polynomial_snapshot, polynomial_atom);
    assert_eq!(polynomial_atom.atom_id(), "polynomial");

    let rational_atom = CanonicalAtom::try_new(
        "rational",
        RegionFunction::RationalFunction(
            RationalFunction3::try_new(
                polynomial(vec![term(1, 0, 1, 0)]),
                polynomial(vec![term(1, 1, 0, 0)]),
                RationalFunctionDomain3::try_new(polynomial(vec![term(1, 1, 0, 0)])).unwrap(),
            )
            .unwrap(),
        ),
        SignRelation::Positive,
    )
    .unwrap();
    let rational_snapshot = rational_atom.try_clone().unwrap();
    assert_eq!(rational_snapshot, rational_atom);
    assert_eq!(rational_atom.atom_id(), "rational");
}

#[test]
fn rational_domain_validity() {
    let region = rational_region("r", 1, vec![term(1, 1, 0, 0)]);
    assert!(region
        .membership(RegionMembershipRequest {
            region: rational_region("r", 1, vec![term(1, 1, 0, 0)]),
            point: point(1, 0, 0),
        })
        .is_ok());
    assert_eq!(
        region.membership(RegionMembershipRequest {
            region: rational_region("r", 1, vec![term(1, 1, 0, 0)]),
            point: point(0, 0, 0),
        }),
        Err(GeometryFailure::CalculusFailure {
            error: neco_calculus::CalculusFailure::Rational(
                neco_calculus::RationalFailure::UndefinedDomain,
            ),
        })
    );
}

#[test]
fn rational_atom_permutation_is_canonical() {
    let denominator = polynomial(vec![term(1, 1, 0, 0)]);
    let first = make_region(
        vec![
            CanonicalAtom::try_new(
                "b",
                RegionFunction::RationalFunction(
                    RationalFunction3::try_new(
                        polynomial(vec![term(2, 0, 0, 0)]),
                        denominator,
                        RationalFunctionDomain3::try_new(polynomial(vec![term(1, 1, 0, 0)]))
                            .unwrap(),
                    )
                    .unwrap(),
                ),
                SignRelation::Positive,
            )
            .unwrap(),
            atom(
                "a",
                polynomial(vec![term(1, 1, 0, 0)]),
                SignRelation::Positive,
            ),
        ],
        NormalizedRegionFormula::And(vec![
            NormalizedRegionFormula::Atom("b".into()),
            NormalizedRegionFormula::Atom("a".into()),
        ]),
    );
    let denominator = polynomial(vec![term(1, 1, 0, 0)]);
    let second = make_region(
        vec![
            atom(
                "a",
                polynomial(vec![term(1, 1, 0, 0)]),
                SignRelation::Positive,
            ),
            CanonicalAtom::try_new(
                "b",
                RegionFunction::RationalFunction(
                    RationalFunction3::try_new(
                        polynomial(vec![term(2, 0, 0, 0)]),
                        denominator,
                        RationalFunctionDomain3::try_new(polynomial(vec![term(1, 1, 0, 0)]))
                            .unwrap(),
                    )
                    .unwrap(),
                ),
                SignRelation::Positive,
            )
            .unwrap(),
        ],
        NormalizedRegionFormula::And(vec![
            NormalizedRegionFormula::Atom("a".into()),
            NormalizedRegionFormula::Atom("b".into()),
        ]),
    );
    assert_eq!(first, second);
}

#[test]
fn region_kind_boolean_observes_all_ordered_pairs() {
    let kinds = [
        RegionKind::RegularClosedVolume,
        RegionKind::GeneralSemiAlgebraicSet,
        RegionKind::LowerDimensionalSet,
    ];
    let mut pair_count = 0;

    for left_kind in kinds {
        for right_kind in kinds {
            pair_count += 1;
            let left = make_region_of_kind(
                vec![atom(
                    "left",
                    polynomial(vec![term(1, 1, 0, 0)]),
                    SignRelation::Positive,
                )],
                NormalizedRegionFormula::Atom("left".into()),
                left_kind,
            );
            let right = make_region_of_kind(
                vec![atom(
                    "right",
                    polynomial(vec![term(1, 0, 1, 0)]),
                    SignRelation::Positive,
                )],
                NormalizedRegionFormula::Atom("right".into()),
                right_kind,
            );
            let formula = NormalizedRegionFormula::Or(vec![
                NormalizedRegionFormula::Atom("left".into()),
                NormalizedRegionFormula::Atom("right".into()),
            ]);
            let expected_set_kind = if left_kind == RegionKind::LowerDimensionalSet
                && right_kind == RegionKind::LowerDimensionalSet
            {
                RegionKind::LowerDimensionalSet
            } else {
                RegionKind::GeneralSemiAlgebraicSet
            };

            assert_eq!(
                left.set_boolean(&right, formula.clone())
                    .unwrap()
                    .region_kind(),
                expected_set_kind
            );
            if left_kind == RegionKind::RegularClosedVolume
                && right_kind == RegionKind::RegularClosedVolume
            {
                assert_eq!(
                    left.solid_boolean(&right, formula).unwrap().region_kind(),
                    RegionKind::RegularClosedVolume
                );
            } else {
                assert_eq!(
                    left.solid_boolean(&right, formula),
                    Err(GeometryFailure::RegionKindMismatch)
                );
            }
        }
    }

    assert_eq!(pair_count, 9);
}

#[test]
fn region_construction_is_repeatable() {
    let request = region_request(
        vec![atom(
            "x",
            polynomial(vec![term(1, 1, 0, 0)]),
            SignRelation::Positive,
        )],
        NormalizedRegionFormula::Atom("x".into()),
    );
    assert_eq!(
        construct_region_types(request),
        construct_region_types(region_request(
            vec![atom(
                "x",
                polynomial(vec![term(1, 1, 0, 0)]),
                SignRelation::Positive,
            )],
            NormalizedRegionFormula::Atom("x".into()),
        ))
    );
}

#[test]
fn membership_request_is_repeatable() {
    let region = make_region(
        vec![atom(
            "x",
            polynomial(vec![term(1, 1, 0, 0)]),
            SignRelation::Positive,
        )],
        NormalizedRegionFormula::Atom("x".into()),
    );
    let first = region.membership(RegionMembershipRequest {
        region: make_region(
            vec![atom(
                "x",
                polynomial(vec![term(1, 1, 0, 0)]),
                SignRelation::Positive,
            )],
            NormalizedRegionFormula::Atom("x".into()),
        ),
        point: point(1, 0, 0),
    });
    let second = region.membership(RegionMembershipRequest {
        region: make_region(
            vec![atom(
                "x",
                polynomial(vec![term(1, 1, 0, 0)]),
                SignRelation::Positive,
            )],
            NormalizedRegionFormula::Atom("x".into()),
        ),
        point: point(1, 0, 0),
    });
    assert_eq!(first, second);
}

#[test]
fn reachable_failures_are_public_and_typed() {
    let invalid_dimension = construct_region_types(RegionConstructionRequest {
        ambient_dimension: 4,
        canonical_atoms: Vec::new(),
        normalized_region_formula: NormalizedRegionFormula::True,
        region_kind: RegionKind::GeneralSemiAlgebraicSet,
    });
    assert_eq!(
        invalid_dimension,
        Err(GeometryFailure::RegionAmbientDimensionMismatch)
    );

    let malformed_formula = construct_region_types(RegionConstructionRequest {
        ambient_dimension: 3,
        canonical_atoms: Vec::new(),
        normalized_region_formula: NormalizedRegionFormula::And(Vec::new()),
        region_kind: RegionKind::GeneralSemiAlgebraicSet,
    });
    assert_eq!(
        malformed_formula,
        Err(GeometryFailure::MalformedRegionFormula)
    );

    let invalid_domain = RationalFunctionDomain3::try_new(RegionPolynomial3::zero());
    assert_eq!(invalid_domain, Err(GeometryFailure::MalformedRegionFormula));

    let unknown_atom = construct_region_types(region_request(
        vec![atom(
            "known",
            polynomial(vec![term(1, 1, 0, 0)]),
            SignRelation::Positive,
        )],
        NormalizedRegionFormula::Atom("unknown".into()),
    ));
    assert_eq!(unknown_atom, Err(GeometryFailure::MalformedRegionFormula));

    let duplicate_atom = construct_region_types(region_request(
        vec![
            atom(
                "duplicate",
                polynomial(vec![term(1, 1, 0, 0)]),
                SignRelation::Positive,
            ),
            atom(
                "duplicate",
                polynomial(vec![term(1, 0, 1, 0)]),
                SignRelation::Positive,
            ),
        ],
        NormalizedRegionFormula::Atom("duplicate".into()),
    ));
    assert_eq!(duplicate_atom, Err(GeometryFailure::DuplicateRegionAtom));

    let duplicate_canonical_key = construct_region_types(region_request(
        vec![
            atom(
                "first",
                polynomial(vec![term(1, 1, 0, 0)]),
                SignRelation::Positive,
            ),
            atom(
                "second",
                polynomial(vec![term(1, 1, 0, 0)]),
                SignRelation::Positive,
            ),
        ],
        NormalizedRegionFormula::And(vec![
            NormalizedRegionFormula::Atom("first".into()),
            NormalizedRegionFormula::Atom("second".into()),
        ]),
    ));
    assert_eq!(
        duplicate_canonical_key,
        Err(GeometryFailure::DuplicateRegionCanonicalKey)
    );

    let region = make_region(
        vec![atom(
            "x",
            polynomial(vec![term(1, 1, 0, 0)]),
            SignRelation::Positive,
        )],
        NormalizedRegionFormula::Atom("x".into()),
    );
    assert_eq!(
        region.membership(RegionMembershipRequest {
            region: make_region(
                vec![atom(
                    "x",
                    polynomial(vec![term(1, 1, 0, 0)]),
                    SignRelation::Positive,
                )],
                NormalizedRegionFormula::Atom("x".into()),
            ),
            point: RegionPoint::Planar(neco_geometry::Point2 {
                x: algebraic(1),
                y: algebraic(0),
            }),
        }),
        Err(GeometryFailure::RegionPointDimensionMismatch)
    );

    let rational = rational_region("r", 1, vec![term(1, 1, 0, 0)]);
    assert_eq!(
        rational.membership(RegionMembershipRequest {
            region: rational_region("r", 1, vec![term(1, 1, 0, 0)]),
            point: point(0, 0, 0),
        }),
        Err(GeometryFailure::CalculusFailure {
            error: neco_calculus::CalculusFailure::Rational(
                neco_calculus::RationalFailure::UndefinedDomain,
            ),
        })
    );
}
