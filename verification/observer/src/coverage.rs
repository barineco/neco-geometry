use crate::protocol::{
    DecodedProjection, GeneratedRequestCase, LogicalManifest, ProcessFailure, SELECTOR_NAMES,
};
use serde_json::{Map, Value};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default)]
pub struct CoverageCounts {
    pub buckets: BTreeMap<(String, String, String), u64>,
    pub producer_observation_kinds: BTreeMap<String, u64>,
    pub checker_verdicts: BTreeMap<String, u64>,
    pub process_errors: BTreeMap<String, u64>,
}

pub fn validate_selector_rules(declarations: &[DecodedProjection]) -> Result<(), ProcessFailure> {
    for declaration in declarations {
        for binding in &declaration.bindings {
            if binding.modulus == 0 {
                return Err(ProcessFailure::new(
                    "UnboundDescriptorReference",
                    "validateSelectorRules",
                    "selectorRule.modulus must be positive",
                ));
            }
            if binding.buckets_by_remainder.len() as u64 != binding.modulus {
                return Err(ProcessFailure::new(
                    "UnboundDescriptorReference",
                    "validateSelectorRules",
                    "selectorRule remainder map must be total",
                ));
            }
            for remainder in 0..binding.modulus {
                if !binding.buckets_by_remainder.contains_key(&remainder) {
                    return Err(ProcessFailure::new(
                        "UnboundDescriptorReference",
                        "validateSelectorRules",
                        "each remainder below modulus must map to exactly one bucket",
                    ));
                }
            }
        }
    }
    Ok(())
}

pub fn projection_bucket(
    declaration: &DecodedProjection,
    axis: &str,
    selectors: &BTreeMap<String, u64>,
) -> Result<Option<String>, ProcessFailure> {
    if !declaration
        .applicable_coverage_axes
        .iter()
        .any(|item| item == axis)
    {
        return Ok(None);
    }
    let binding = declaration
        .bindings
        .iter()
        .find(|binding| binding.axis == axis)
        .ok_or_else(|| {
            ProcessFailure::new(
                "UnboundDescriptorReference",
                "projectionBucket",
                format!("selectorRule for axis {axis} is unbound"),
            )
        })?;
    let selector_value = selectors.get(&binding.selector).ok_or_else(|| {
        ProcessFailure::new(
            "UnboundDescriptorReference",
            "projectionBucket",
            format!("selector {} is missing from the request", binding.selector),
        )
    })?;
    let remainder = selector_value % binding.modulus;
    let bucket = binding.buckets_by_remainder.get(&remainder).ok_or_else(|| {
        ProcessFailure::new(
            "UnboundDescriptorReference",
            "projectionBucket",
            "remainder is missing from the total selectorRule map",
        )
    })?;
    Ok(Some(bucket.clone()))
}

pub fn collect_case_buckets(
    declaration: &DecodedProjection,
    case: &GeneratedRequestCase,
    counts: &mut CoverageCounts,
) -> Result<(), ProcessFailure> {
    for axis in &declaration.applicable_coverage_axes {
        if let Some(bucket) = projection_bucket(declaration, axis, &case.selectors)? {
            *counts
                .buckets
                .entry((declaration.family.clone(), axis.clone(), bucket))
                .or_default() += 1;
        }
    }
    Ok(())
}

pub fn ignore_nonapplicable_selectors(declaration: &DecodedProjection) -> Vec<String> {
    SELECTOR_NAMES
        .iter()
        .filter(|name| {
            !declaration
                .bindings
                .iter()
                .any(|binding| binding.selector == **name)
        })
        .map(|name| (*name).to_owned())
        .collect()
}

pub fn record_producer_kind(counts: &mut CoverageCounts, kind: &str) {
    let class = if kind == "success" {
        "success"
    } else {
        "typedFailure"
    };
    *counts
        .producer_observation_kinds
        .entry(class.to_owned())
        .or_default() += 1;
}

pub fn record_checker_verdict(counts: &mut CoverageCounts, verdict: &str) {
    *counts.checker_verdicts.entry(verdict.to_owned()).or_default() += 1;
}

pub fn record_process_error(counts: &mut CoverageCounts, kind: &str) {
    *counts.process_errors.entry(kind.to_owned()).or_default() += 1;
}

pub fn coverage_value(counts: &CoverageCounts) -> Value {
    let mut buckets = Map::new();
    for ((family, axis, bucket), count) in &counts.buckets {
        let key = format!("{family}/{axis}/{bucket}");
        buckets.insert(key, Value::from(*count));
    }
    crate::protocol::object([
        ("checkerVerdicts", map_to_object(&counts.checker_verdicts)),
        ("descriptorBuckets", Value::Object(buckets)),
        ("processErrors", map_to_object(&counts.process_errors)),
        (
            "producerObservationKinds",
            map_to_object(&counts.producer_observation_kinds),
        ),
    ])
}

fn map_to_object(counts: &BTreeMap<String, u64>) -> Value {
    Value::Object(
        counts
            .iter()
            .map(|(key, count)| (key.clone(), Value::from(*count)))
            .collect(),
    )
}

pub fn collect_corpus_coverage(
    declarations: &[DecodedProjection],
    cases: &[GeneratedRequestCase],
) -> Result<CoverageCounts, ProcessFailure> {
    validate_selector_rules(declarations)?;
    let mut counts = CoverageCounts::default();
    for case in cases {
        let declaration = declarations
            .iter()
            .find(|item| item.family == case.family)
            .ok_or_else(|| {
                ProcessFailure::new(
                    "UnboundDescriptorReference",
                    "collectCorpusCoverage",
                    format!("family {} is unbound", case.family),
                )
            })?;
        collect_case_buckets(declaration, case, &mut counts)?;
    }
    Ok(counts)
}

pub fn enforce_declared_bucket_coverage(
    manifest: &LogicalManifest,
    declarations_by_domain: &BTreeMap<String, Vec<DecodedProjection>>,
    counts: &CoverageCounts,
) -> Result<(), ProcessFailure> {
    let mut missing = Vec::new();
    for domain in &manifest.domains {
        let Some(declarations) = declarations_by_domain.get(&domain.domain) else {
            continue;
        };
        for declaration in declarations {
            for ((family, axis), buckets) in crate::corpus_generator::declared_buckets(declaration) {
                for bucket in buckets {
                    let required = domain
                        .coverage_minimum
                        .iter()
                        .filter(|minimum| {
                            minimum.family == family && minimum.axis == axis && minimum.bucket == bucket
                        })
                        .map(|minimum| minimum.minimum)
                        .max()
                        .unwrap_or(0)
                        .max(1);
                    let key = (family.clone(), axis.clone(), bucket.clone());
                    if counts.buckets.get(&key).copied().unwrap_or(0) < required {
                        missing.push(Value::String(format!("{family}/{axis}/{bucket}")));
                    }
                }
            }
        }
    }
    if missing.is_empty() {
        return Ok(());
    }
    Err(ProcessFailure::new(
        "CoverageMissing",
        "enforceDeclaredBucketCoverage",
        "declared selector buckets were not generated",
    )
    .with_witness(crate::protocol::object([("missing", Value::Array(missing))])))
}
