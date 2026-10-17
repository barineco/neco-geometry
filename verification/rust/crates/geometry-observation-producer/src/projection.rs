use std::collections::BTreeSet;

use geometry_observation_types::{
    AdapterFailure, GeneratedRequestCase, OperandProvenance, Value,
};

use crate::region;
use crate::surface;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ProjectionDeclaration {
    pub family: &'static str,
    pub requested_kinds: &'static [&'static str],
    pub applicable_axes: &'static [&'static str],
    pub operation: &'static str,
    pub checker_predicate: &'static str,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ProjectedRequest {
    pub case_id: String,
    pub family: &'static str,
    pub requested_observation_kind: String,
    pub invocation: PublicApiInvocation,
    pub checker_predicate: &'static str,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PublicApiInvocation {
    pub operation: &'static str,
    pub arguments: Value,
}

const DECLARATIONS: [ProjectionDeclaration; 6] = [
    ProjectionDeclaration {
        family: "region-polynomial-normalization",
        requested_kinds: &["success", "failure"],
        applicable_axes: &["formulaVariant", "value"],
        operation: "region-polynomial-normalization",
        checker_predicate: "canonicalRegionPolynomial",
    },
    ProjectionDeclaration {
        family: "region-affine-substitution",
        requested_kinds: &["success", "failure"],
        applicable_axes: &["familyParameter", "formulaVariant", "value"],
        operation: "region-affine-substitution",
        checker_predicate: "affineSubstituteOrTypedFailure",
    },
    ProjectionDeclaration {
        family: "region-formula-construction",
        requested_kinds: &["success", "failure"],
        applicable_axes: &["formulaVariant"],
        operation: "region-formula-construction",
        checker_predicate: "normalizedRegionFormula",
    },
    ProjectionDeclaration {
        family: "region-membership",
        requested_kinds: &["success", "failure"],
        applicable_axes: &["familyParameter", "formulaVariant", "sign", "value"],
        operation: "region-membership",
        checker_predicate: "membershipClassification",
    },
    ProjectionDeclaration {
        family: "surface-witness-exact",
        requested_kinds: &["success", "failure"],
        applicable_axes: &["familyParameter", "formulaVariant", "value"],
        operation: "surface-witness-exact",
        checker_predicate: "surfaceWitnessExact",
    },
    ProjectionDeclaration {
        family: "surface-witness-checked",
        requested_kinds: &["success", "failure"],
        applicable_axes: &["familyParameter", "formulaVariant", "intervalWidth", "value"],
        operation: "surface-witness-checked",
        checker_predicate: "surfaceWitnessChecked",
    },
];

pub(crate) fn declarations() -> &'static [ProjectionDeclaration] {
    &DECLARATIONS
}

pub(crate) fn declaration(family: &str) -> Result<&'static ProjectionDeclaration, AdapterFailure> {
    DECLARATIONS
        .iter()
        .find(|declaration| declaration.family == family)
        .ok_or_else(|| unsupported(family, None, "family is outside the six-family declaration"))
}

pub(crate) fn coverage_bucket(selector: &str, value: u64) -> Result<&'static str, AdapterFailure> {
    match selector {
        "value" | "familyParameter" | "intervalWidth" => {
            Ok(["small", "medium", "large", "extreme"][(value % 4) as usize])
        }
        "sign" => Ok(["positive", "negative", "mixed", "zero"][(value % 4) as usize]),
        "formulaVariant" => Ok(["base", "scaled", "alternate", "failure"][(value % 4) as usize]),
        _ => Err(unsupported(
            "",
            None,
            "selector has no applicable coverage bucket",
        )),
    }
}

pub(crate) fn project(case: &GeneratedRequestCase) -> Result<ProjectedRequest, AdapterFailure> {
    let declaration = declaration(&case.family)?;
    if !declaration
        .requested_kinds
        .contains(&case.requested_observation_kind.as_str())
    {
        return Err(unsupported(
            &case.family,
            Some(case),
            "requested observation kind is outside the declaration",
        ));
    }
    for axis in applicable_axis_set(declaration.family)? {
        let _ = projection_witness(declaration.family, axis, selector(case, axis)?)?;
    }
    let invocation = match declaration.family {
        "region-polynomial-normalization" => region::project_normalization(case)?,
        "region-affine-substitution" => region::project_affine(case)?,
        "region-formula-construction" => region::project_formula(case)?,
        "region-membership" => region::project_membership(case)?,
        "surface-witness-exact" => surface::project_exact(case)?,
        "surface-witness-checked" => surface::project_checked(case)?,
        _ => {
            return Err(unsupported(
                &case.family,
                Some(case),
                "family declaration cannot be projected",
            ))
        }
    };
    Ok(ProjectedRequest {
        case_id: case.case_id.clone(),
        family: declaration.family,
        requested_observation_kind: case.requested_observation_kind.clone(),
        invocation,
        checker_predicate: declaration.checker_predicate,
    })
}

pub(crate) fn observe(projected: &ProjectedRequest) -> Result<Value, AdapterFailure> {
    match projected.family {
        "region-polynomial-normalization" => region::observe_normalization(projected),
        "region-affine-substitution" => region::observe_affine(projected),
        "region-formula-construction" => region::observe_formula(projected),
        "region-membership" => region::observe_membership(projected),
        "surface-witness-exact" => surface::observe_exact(projected),
        "surface-witness-checked" => surface::observe_checked(projected),
        _ => Err(unsupported(
            projected.family,
            None,
            "family declaration cannot be observed",
        )),
    }
}

pub(crate) fn requests_typed_failure(kind: &str) -> bool {
    kind != "success"
}

pub(crate) fn selector(case: &GeneratedRequestCase, name: &str) -> Result<u64, AdapterFailure> {
    case.selectors.get(name).copied().ok_or_else(|| {
        unsupported(
            &case.family,
            Some(case),
            "selector is absent from canonical request",
        )
    })
}

pub(crate) fn applicable_axis_set(family: &str) -> Result<BTreeSet<&'static str>, AdapterFailure> {
    Ok(declaration(family)?
        .applicable_axes
        .iter()
        .copied()
        .collect())
}

pub(crate) fn projection_witness(
    family: &str,
    axis: &str,
    selector_value: u64,
) -> Result<String, AdapterFailure> {
    let bucket = coverage_bucket(axis, selector_value)?;
    Ok(format!("{family}:{axis}:{bucket}"))
}

fn unsupported(
    family: &str,
    case: Option<&GeneratedRequestCase>,
    condition: &str,
) -> AdapterFailure {
    AdapterFailure::new(
        "unsupportedProjection",
        "projectRequest",
        OperandProvenance {
            case_id: case.map(|case| case.case_id.clone()),
            family: (!family.is_empty()).then(|| family.to_owned()),
            requested_observation_kind: case.map(|case| case.requested_observation_kind.clone()),
            selectors: case
                .map(|case| {
                    case.selectors
                        .iter()
                        .map(|(name, value)| (name.clone(), format!("{value:016x}")))
                        .collect()
                })
                .unwrap_or_default(),
            projection_revision: None,
            resource_revision: None,
            build_identity: None,
        },
        condition,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use geometry_observation_types::{GeneratedRequestCase, SELECTOR_NAMES};

    #[test]
    fn six_declarations_have_a_finite_bucket_for_every_applicable_axis() {
        assert_eq!(declarations().len(), 6);
        for declaration in declarations() {
            for axis in declaration.applicable_axes {
                for remainder in 0..4 {
                    assert!(coverage_bucket(axis, remainder).is_ok());
                }
            }
        }
    }

    #[test]
    fn non_applicable_axes_do_not_change_projection() {
        let selectors = SELECTOR_NAMES
            .into_iter()
            .map(|name| (name.to_owned(), 0))
            .collect();
        let case = GeneratedRequestCase {
            case_id: "case-0000000000000000".to_owned(),
            global_case_index: 0,
            domain_case_index: 0,
            family: "region-polynomial-normalization".to_owned(),
            requested_observation_kind: "success".to_owned(),
            selectors,
        };
        let mut changed = case.clone();
        changed.selectors.insert("observationClass".to_owned(), 3);
        changed.selectors.insert("mutation".to_owned(), 2);
        changed.selectors.insert("reserved".to_owned(), 1);
        changed.selectors.insert("sign".to_owned(), 7);
        assert_eq!(
            project(&case).unwrap().invocation,
            project(&changed).unwrap().invocation
        );
    }
}
