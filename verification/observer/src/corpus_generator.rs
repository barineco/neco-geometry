use crate::protocol::{
    encode_generated_request_batch, encode_u64_hex, hamilton_apportionment, CoverageMinimum,
    DecodedProjection, DomainRequirement, GeneratedRequestBatch, GeneratedRequestCase,
    LogicalManifest, ProcessFailure, ProducerDescriptor, SELECTOR_NAMES, DRAW_SCHEDULE_REVISION,
    GENERATOR_ALGORITHM,
};
use crate::coverage::projection_bucket;
use std::collections::{BTreeMap, BTreeSet};

pub fn splitmix64_draw(seed: u64, case_index: u64, slot: u64) -> u64 {
    let mixed = 9u64
        .wrapping_mul(case_index)
        .wrapping_add(slot)
        .wrapping_add(1);
    let x = seed.wrapping_add(0x9E3779B97F4A7C15u64.wrapping_mul(mixed));
    let mut z = x;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^ (z >> 31)
}

#[derive(Clone, Debug)]
pub struct FamilyCatalogEntry {
    pub family: String,
    pub weight: u64,
    pub minimum: u64,
    pub requested_observation_kinds: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct DomainPlan {
    pub domain: String,
    pub adapter_revision: String,
    pub case_count: u64,
    pub families: Vec<FamilyCatalogEntry>,
    pub family_counts: BTreeMap<String, u64>,
}

pub fn family_catalog(
    domain: &DomainRequirement,
    declarations: Option<&[DecodedProjection]>,
) -> Result<Vec<FamilyCatalogEntry>, ProcessFailure> {
    match declarations {
        Some(declarations) => {
            let mut catalog = Vec::new();
            for declaration in declarations {
                let requirement = domain.families.iter().find(|family| family.family == declaration.family).ok_or_else(|| {
                    ProcessFailure::new(
                        "UnboundDescriptorReference",
                        "familyCatalog",
                        format!("LogicalManifest has no family {}", declaration.family),
                    )
                })?;
                if declaration.requested_observation_kinds.is_empty() {
                    return Err(ProcessFailure::new(
                        "UnboundDescriptorReference",
                        "familyCatalog",
                        format!("{} requestedObservationKinds is empty", declaration.family),
                    ));
                }
                catalog.push(FamilyCatalogEntry {
                    family: declaration.family.clone(),
                    weight: requirement.weight,
                    minimum: requirement.minimum,
                    requested_observation_kinds: declaration.requested_observation_kinds.clone(),
                });
            }
            for family in &domain.families {
                if !declarations.iter().any(|item| item.family == family.family) {
                    return Err(ProcessFailure::new(
                        "UnboundDescriptorReference",
                        "familyCatalog",
                        format!("ProducerDescriptor has no family {}", family.family),
                    ));
                }
            }
            Ok(catalog)
        }
        None => {
            for family in &domain.families {
                if family.requested_observation_kinds.is_empty() {
                    return Err(ProcessFailure::new(
                        "UnboundDescriptorReference",
                        "familyCatalog",
                        format!("{} requestedObservationKinds is empty", family.family),
                    ));
                }
            }
            Ok(domain
                .families
                .iter()
                .map(|family| FamilyCatalogEntry {
                    family: family.family.clone(),
                    weight: family.weight,
                    minimum: family.minimum,
                    requested_observation_kinds: family.requested_observation_kinds.clone(),
                })
                .collect())
        }
    }
}

pub fn allocate_domain_families(
    case_count: u64,
    families: &[FamilyCatalogEntry],
    coverage_minimum: &[CoverageMinimum],
) -> Result<BTreeMap<String, u64>, ProcessFailure> {
    let mut reserved = BTreeMap::new();
    for family in families {
        let bucket_minimum: u64 = coverage_minimum
            .iter()
            .filter(|minimum| minimum.family == family.family)
            .map(|minimum| minimum.minimum)
            .sum();
        reserved.insert(family.family.clone(), family.minimum.saturating_add(bucket_minimum));
    }
    let reserved_total: u64 = reserved.values().sum();
    if reserved_total > case_count {
        return Err(ProcessFailure::new(
            "InfeasibleMinimum",
            "allocateDomainFamilies",
            "family and coverage minima exceed the domain case count",
        ));
    }
    let remaining = case_count - reserved_total;
    let weights: Vec<(String, u64)> = families
        .iter()
        .map(|family| (family.family.clone(), family.weight))
        .collect();
    let extra = if remaining == 0 || weights.is_empty() {
        BTreeMap::new()
    } else {
        hamilton_apportionment(remaining, &weights)?
    };
    let mut counts = BTreeMap::new();
    for family in families {
        let count = reserved.get(&family.family).copied().unwrap_or(0)
            + extra.get(&family.family).copied().unwrap_or(0);
        counts.insert(family.family.clone(), count);
    }
    Ok(counts)
}

pub const DERIVED_FAMILY_CASE_LIMIT: u64 = 1_000_000;

fn case_selectors(seed: u64, global_index: u64) -> BTreeMap<String, u64> {
    SELECTOR_NAMES
        .iter()
        .enumerate()
        .map(|(slot, name)| {
            (
                (*name).to_owned(),
                splitmix64_draw(seed, global_index, slot as u64),
            )
        })
        .collect()
}

fn requested_kind_index(
    selectors: &BTreeMap<String, u64>,
    kind_count: usize,
) -> Result<usize, ProcessFailure> {
    let observation_class = *selectors.get("observationClass").ok_or_else(|| {
        ProcessFailure::new(
            "UnboundDescriptorReference",
            "globalCase",
            "observationClass selector is required",
        )
    })?;
    Ok((observation_class as usize) % kind_count)
}

pub fn declared_buckets(declaration: &DecodedProjection) -> BTreeMap<(String, String), BTreeSet<String>> {
    let mut buckets = BTreeMap::new();
    for axis in &declaration.applicable_coverage_axes {
        if let Some(binding) = declaration.bindings.iter().find(|binding| &binding.axis == axis) {
            buckets.insert(
                (declaration.family.clone(), axis.clone()),
                binding.buckets_by_remainder.values().cloned().collect(),
            );
        }
    }
    buckets
}

fn derive_family_count(
    seed: u64,
    start: u64,
    entry: &FamilyCatalogEntry,
    declaration: Option<&DecodedProjection>,
    coverage_minimum: &[CoverageMinimum],
) -> Result<u64, ProcessFailure> {
    let family_minimum: u64 = coverage_minimum
        .iter()
        .filter(|minimum| minimum.family == entry.family)
        .map(|minimum| minimum.minimum)
        .sum();
    let mut required: BTreeMap<(String, String), u64> = BTreeMap::new();
    if let Some(declaration) = declaration {
        for ((_, axis), buckets) in declared_buckets(declaration) {
            for bucket in buckets {
                let declared = coverage_minimum
                    .iter()
                    .filter(|minimum| {
                        minimum.family == entry.family && minimum.axis == axis && minimum.bucket == bucket
                    })
                    .map(|minimum| minimum.minimum)
                    .max()
                    .unwrap_or(0);
                required.insert((axis.clone(), bucket), declared.max(1));
            }
        }
    }
    let floor = if declaration.is_some() {
        entry.minimum
    } else {
        entry.minimum.saturating_add(family_minimum)
    };
    let kind_count = entry.requested_observation_kinds.len();
    let mut seen_buckets: BTreeMap<(String, String), u64> = BTreeMap::new();
    let mut seen_kinds = vec![0u64; kind_count];
    let mut count = 0u64;
    loop {
        let covered = count >= floor
            && seen_kinds.iter().all(|seen| *seen > 0)
            && required
                .iter()
                .all(|(key, need)| seen_buckets.get(key).copied().unwrap_or(0) >= *need);
        if covered {
            return Ok(count);
        }
        if count >= DERIVED_FAMILY_CASE_LIMIT {
            return Err(ProcessFailure::new(
                "InfeasibleMinimum",
                "deriveFamilyCount",
                format!("{} coverage is not reached within the derivation limit", entry.family),
            ));
        }
        let selectors = case_selectors(seed, start + count);
        seen_kinds[requested_kind_index(&selectors, kind_count)?] += 1;
        if let Some(declaration) = declaration {
            for axis in &declaration.applicable_coverage_axes {
                if let Some(bucket) = projection_bucket(declaration, axis, &selectors)? {
                    *seen_buckets.entry((axis.clone(), bucket)).or_default() += 1;
                }
            }
        }
        count += 1;
    }
}

pub fn domain_plans(
    manifest: &LogicalManifest,
    declarations_by_domain: &BTreeMap<String, Vec<DecodedProjection>>,
) -> Result<Vec<DomainPlan>, ProcessFailure> {
    let mut plans = Vec::new();
    let mut cursor = 0u64;
    for domain in &manifest.domains {
        let declarations = declarations_by_domain.get(&domain.domain);
        let families = family_catalog(domain, declarations.map(Vec::as_slice))?;
        let (case_count, family_counts) = match &manifest.generator.explicit_allocation {
            Some(explicit) => {
                let case_count = *explicit.allocation.get(&domain.domain).ok_or_else(|| {
                    ProcessFailure::new(
                        "ManifestValidationFailure",
                        "domainPlans",
                        format!("allocation is missing {}", domain.domain),
                    )
                })?;
                let counts =
                    allocate_domain_families(case_count, &families, &domain.coverage_minimum)?;
                (case_count, counts)
            }
            None => {
                let mut ordered: Vec<&FamilyCatalogEntry> = families.iter().collect();
                ordered.sort_by(|left, right| left.family.as_bytes().cmp(right.family.as_bytes()));
                let mut counts = BTreeMap::new();
                let mut family_start = cursor;
                for entry in ordered {
                    let declaration = declarations.and_then(|items| {
                        items.iter().find(|item| item.family == entry.family)
                    });
                    let count = derive_family_count(
                        manifest.generator.seed_value,
                        family_start,
                        entry,
                        declaration,
                        &domain.coverage_minimum,
                    )?;
                    counts.insert(entry.family.clone(), count);
                    family_start += count;
                }
                (family_start - cursor, counts)
            }
        };
        cursor += case_count;
        plans.push(DomainPlan {
            domain: domain.domain.clone(),
            adapter_revision: domain.adapter_revision.clone(),
            case_count,
            families,
            family_counts,
        });
    }
    Ok(plans)
}

fn ordered_family_for_domain_index(
    plan: &DomainPlan,
    domain_index: u64,
) -> Result<&FamilyCatalogEntry, ProcessFailure> {
    let mut ordered: Vec<&FamilyCatalogEntry> = plan.families.iter().collect();
    ordered.sort_by(|left, right| left.family.as_bytes().cmp(right.family.as_bytes()));
    let mut cursor = 0u64;
    for family in ordered {
        let count = *plan.family_counts.get(&family.family).ok_or_else(|| {
            ProcessFailure::new(
                "UnboundDescriptorReference",
                "familyForDomainIndex",
                format!("{} has no allocated count", family.family),
            )
        })?;
        if domain_index < cursor + count {
            return Ok(family);
        }
        cursor += count;
    }
    Err(ProcessFailure::new(
        "UnboundDescriptorReference",
        "familyForDomainIndex",
        "domain case index is outside the allocated family ranges",
    ))
}

pub fn global_case(
    manifest: &LogicalManifest,
    plans: &[DomainPlan],
    global_index: u64,
) -> Result<(String, u64, GeneratedRequestCase), ProcessFailure> {
    let mut cursor = 0u64;
    for plan in plans {
        if global_index < cursor + plan.case_count {
            let domain_index = global_index - cursor;
            let family = ordered_family_for_domain_index(plan, domain_index)?;
            let selectors = case_selectors(manifest.generator.seed_value, global_index);
            if family.requested_observation_kinds.is_empty() {
                return Err(ProcessFailure::new(
                    "UnboundDescriptorReference",
                    "globalCase",
                    format!("{} requestedObservationKinds is empty", family.family),
                ));
            }
            let kind_index =
                requested_kind_index(&selectors, family.requested_observation_kinds.len())?;
            let requested_observation_kind = family.requested_observation_kinds[kind_index].clone();
            let case = GeneratedRequestCase {
                case_id: format!("case-{}", encode_u64_hex(global_index)),
                global_case_index: global_index,
                domain_case_index: domain_index,
                family: family.family.clone(),
                requested_observation_kind,
                selectors,
            };
            return Ok((plan.domain.clone(), domain_index, case));
        }
        cursor += plan.case_count;
    }
    Err(ProcessFailure::new(
        "ManifestValidationFailure",
        "globalCase",
        "global case index is outside the logical corpus",
    ))
}

pub fn generate_all_cases(
    manifest: &LogicalManifest,
    declarations_by_domain: &BTreeMap<String, Vec<DecodedProjection>>,
) -> Result<Vec<(String, GeneratedRequestCase)>, ProcessFailure> {
    let plans = domain_plans(manifest, declarations_by_domain)?;
    let mut cases = Vec::new();
    for global_index in 0..plans.iter().map(|plan| plan.case_count).sum() {
        let (domain, _, case) = global_case(manifest, &plans, global_index)?;
        cases.push((domain, case));
    }
    Ok(cases)
}

pub fn shard_batches(
    manifest: &LogicalManifest,
    declarations_by_domain: &BTreeMap<String, Vec<DecodedProjection>>,
    shard_size: u64,
) -> Result<Vec<GeneratedRequestBatch>, ProcessFailure> {
    if shard_size == 0 {
        return Err(ProcessFailure::new(
            "ShardExceedsProducerMaxBatchCases",
            "shardBatches",
            "shard size must be positive",
        ));
    }
    let plans = domain_plans(manifest, declarations_by_domain)?;
    let mut batches = Vec::new();
    for plan in &plans {
        let mut domain_cases = Vec::new();
        for domain_index in 0..plan.case_count {
            let global_index = plans
                .iter()
                .take_while(|item| item.domain != plan.domain)
                .map(|item| item.case_count)
                .sum::<u64>()
                + domain_index;
            let (_, _, case) = global_case(manifest, &plans, global_index)?;
            domain_cases.push(case);
        }
        for chunk in domain_cases.chunks(shard_size as usize) {
            if chunk.len() as u64 > shard_size {
                return Err(ProcessFailure::new(
                    "ShardExceedsProducerMaxBatchCases",
                    "shardBatches",
                    "shard exceeds the requested shard size",
                ));
            }
            batches.push(GeneratedRequestBatch {
                adapter_revision: plan.adapter_revision.clone(),
                domain: plan.domain.clone(),
                algorithm: GENERATOR_ALGORITHM.to_owned(),
                draw_schedule_revision: DRAW_SCHEDULE_REVISION.to_owned(),
                seed: manifest.generator.seed.clone(),
                cases: chunk.to_vec(),
            });
        }
    }
    Ok(batches)
}

pub fn request_digest(
    manifest: &LogicalManifest,
    declarations_by_domain: &BTreeMap<String, Vec<DecodedProjection>>,
) -> Result<String, ProcessFailure> {
    let cases = generate_all_cases(manifest, declarations_by_domain)?;
    let mut concatenated = Vec::new();
    for (_, case) in cases {
        let encoded = crate::canonical_json(&crate::protocol::generated_request_case_value(&case)).map_err(
            |error| ProcessFailure::new("CanonicalDecodeFailure", "requestDigest", error),
        )?;
        concatenated.extend_from_slice(&encoded);
    }
    Ok(crate::sha256(&concatenated))
}

pub fn write_shards(
    batches: &[GeneratedRequestBatch],
    out: &std::path::Path,
) -> Result<String, ProcessFailure> {
    std::fs::create_dir_all(out).map_err(|error| {
        ProcessFailure::new(
            "ShardEventWriteFailure",
            "writeShards",
            format!("{}: {error}", out.display()),
        )
    })?;
    let mut concatenated = Vec::new();
    for (index, batch) in batches.iter().enumerate() {
        let encoded = encode_generated_request_batch(batch)?;
        let path = out.join(format!("shard-{index:04}.json"));
        crate::protocol::atomic_write(&path, &encoded)?;
        for case in &batch.cases {
            let case_bytes = crate::canonical_json(&crate::protocol::generated_request_case_value(case))
                .map_err(|error| ProcessFailure::new("CanonicalDecodeFailure", "writeShards", error))?;
            concatenated.extend_from_slice(&case_bytes);
        }
    }
    Ok(crate::sha256(&concatenated))
}

pub fn enforce_shard_at_most_producer_max(
    shard_size: u64,
    descriptors: &BTreeMap<String, ProducerDescriptor>,
) -> Result<(), ProcessFailure> {
    if descriptors.is_empty() {
        return Err(ProcessFailure::new(
            "UnboundBuildMetadata",
            "enforceShardAtMostProducerMax",
            "ProducerDescriptor is required to bound shardCaseCount",
        ));
    }
    for descriptor in descriptors.values() {
        if shard_size > descriptor.max_batch_cases {
            return Err(ProcessFailure::new(
                "ShardExceedsProducerMaxBatchCases",
                "enforceShardAtMostProducerMax",
                format!(
                    "shardCaseCount {shard_size} exceeds maxBatchCases {}",
                    descriptor.max_batch_cases
                ),
            ));
        }
    }
    Ok(())
}

pub fn infeasible_minimum_failure(manifest: &LogicalManifest) -> Result<ProcessFailure, ProcessFailure> {
    for domain in &manifest.domains {
        let case_count = manifest
            .generator
            .explicit_allocation
            .as_ref()
            .and_then(|explicit| explicit.allocation.get(&domain.domain).copied())
            .unwrap_or(0);
        let families: Vec<FamilyCatalogEntry> = domain
            .families
            .iter()
            .map(|family| FamilyCatalogEntry {
                family: family.family.clone(),
                weight: family.weight,
                minimum: case_count.saturating_add(1),
                requested_observation_kinds: family.requested_observation_kinds.clone(),
            })
            .collect();
        if let Err(failure) = allocate_domain_families(case_count, &families, &[]) {
            return Ok(failure);
        }
    }
    Err(ProcessFailure::new(
        "ManifestValidationFailure",
        "testInfeasibleMinimum",
        "infeasible minima were not detected",
    ))
}
