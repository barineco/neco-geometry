use crate::{canonical_json, sha256};
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

pub const LOGICAL_MANIFEST_SCHEMA: &str = "neco.verification.logical-manifest.v1";
pub const PROTOCOL_REVISION: &str = "protocol-v1";
pub const GENERATOR_REVISION: &str = "generator-v1";
pub const GENERATOR_ALGORITHM: &str = "splitmix64-v1";
pub const DRAW_SCHEDULE_REVISION: &str = "draw-schedule-v1";
pub const REQUEST_SCHEMA: &str = "neco.verification.generated-request-batch.v1";
pub const PRODUCER_DESCRIPTION_SCHEMA: &str = "neco.verification.producer-description.v1";
pub const DEFAULT_MAX_BATCH_CASES: u64 = 2000;
pub const SELECTOR_NAMES: [&str; 9] = [
    "value",
    "familyParameter",
    "observationClass",
    "sign",
    "multiplicity",
    "intervalWidth",
    "formulaVariant",
    "mutation",
    "reserved",
];
pub const SEVEN_LANES: [&str; 7] = [
    "corpusExecution",
    "observationVerification",
    "soundnessTheorem",
    "publicApiParity",
    "mutationRejection",
    "invalidProtocol",
    "repeatability",
];

const FORBIDDEN_MANIFEST_KEYS: [&str; 16] = [
    "artifactDigest",
    "checkerPath",
    "descriptorDigest",
    "digest",
    "gitObjectId",
    "gitOid",
    "hostPath",
    "hostname",
    "pid",
    "producerPath",
    "reportDirectory",
    "selfDigest",
    "sourceOID",
    "sourceOid",
    "temporaryDirectory",
    "workingDirectory",
];

#[derive(Clone, Debug)]
pub struct ProcessFailure(Box<ProcessFailureRecord>);

#[derive(Clone, Debug)]
struct ProcessFailureRecord {
    kind: String,
    failing_operation: String,
    operand_provenance: Value,
    violated_condition: String,
    witness: Value,
    contributing_failures: Vec<ProcessFailure>,
}

impl ProcessFailure {
    pub fn new(
        kind: impl Into<String>,
        failing_operation: impl Into<String>,
        violated_condition: impl Into<String>,
    ) -> Self {
        Self(Box::new(ProcessFailureRecord {
            kind: kind.into(),
            failing_operation: failing_operation.into(),
            operand_provenance: Value::Object(Map::new()),
            violated_condition: violated_condition.into(),
            witness: Value::Null,
            contributing_failures: Vec::new(),
        }))
    }

    pub fn with_provenance(mut self, provenance: Value) -> Self {
        self.0.operand_provenance = provenance;
        self
    }

    pub fn with_witness(mut self, witness: Value) -> Self {
        self.0.witness = witness;
        self
    }

    pub fn with_contributing(mut self, failures: Vec<ProcessFailure>) -> Self {
        self.0.contributing_failures = failures;
        self
    }

    pub fn kind(&self) -> &str {
        &self.0.kind
    }

    pub fn failing_operation(&self) -> &str {
        &self.0.failing_operation
    }

    pub fn operand_provenance(&self) -> &Value {
        &self.0.operand_provenance
    }

    pub fn violated_condition(&self) -> &str {
        &self.0.violated_condition
    }

    pub fn witness(&self) -> &Value {
        &self.0.witness
    }

    pub fn contributing_failures(&self) -> &[ProcessFailure] {
        &self.0.contributing_failures
    }

    pub fn to_value(&self) -> Value {
        object([
            (
                "contributingFailures",
                Value::Array(
                    self.0
                        .contributing_failures
                        .iter()
                        .map(Self::to_value)
                        .collect(),
                ),
            ),
            (
                "failingOperation",
                Value::String(self.0.failing_operation.clone()),
            ),
            ("kind", Value::String(self.0.kind.clone())),
            ("operandProvenance", self.0.operand_provenance.clone()),
            (
                "violatedCondition",
                Value::String(self.0.violated_condition.clone()),
            ),
            ("witness", self.0.witness.clone()),
        ])
    }

    pub fn render(&self) -> Result<String, String> {
        String::from_utf8(canonical_json(&self.to_value())?).map_err(|error| error.to_string())
    }
}

#[derive(Clone, Debug)]
pub struct LogicalManifest {
    pub protocol_revision: String,
    pub generator: GeneratorDeclaration,
    pub domains: Vec<DomainRequirement>,
    pub coverage: CoverageDeclaration,
    pub lanes: Vec<String>,
    pub timeout_policy: TimeoutPolicy,
    pub bytes: Vec<u8>,
    pub digest: String,
}

#[derive(Clone, Debug)]
pub struct GeneratorDeclaration {
    pub revision: String,
    pub algorithm: String,
    pub draw_schedule_revision: String,
    pub seed: String,
    pub seed_value: u64,
    pub tier: String,
    pub shard_case_count: u64,
    pub explicit_allocation: Option<ExplicitAllocation>,
}

#[derive(Clone, Debug)]
pub struct ExplicitAllocation {
    pub normal_case_count: u64,
    pub allocation: BTreeMap<String, u64>,
}

#[derive(Clone, Debug)]
pub struct DomainRequirement {
    pub domain: String,
    pub request_schema: String,
    pub observation_schema: String,
    pub producer_api_revision: String,
    pub checker_api_revision: String,
    pub adapter_revision: String,
    pub projection_revision: String,
    pub semantics_ref: SemanticsRef,
    pub families: Vec<FamilyRequirement>,
    pub coverage_minimum: Vec<CoverageMinimum>,
}

#[derive(Clone, Debug)]
pub struct FamilyRequirement {
    pub family: String,
    pub weight: u64,
    pub minimum: u64,
    pub requested_observation_kinds: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct CoverageMinimum {
    pub family: String,
    pub axis: String,
    pub bucket: String,
    pub minimum: u64,
}

#[derive(Clone, Debug)]
pub struct CoverageDeclaration {
    pub mutation_expected_verdict: String,
    pub invalid_protocol_expected_verdict: String,
}

#[derive(Clone, Debug)]
pub struct TimeoutPolicy {
    pub shard_timeout_seconds: u64,
    pub lane_timeout_seconds: u64,
}

#[derive(Clone, Debug)]
pub struct SemanticsRef {
    pub declaration_digest: String,
    pub protocol_revision: String,
    pub checker_revision: String,
    pub shared_semantics_refs: Vec<SharedSemanticsRef>,
}

#[derive(Clone, Debug)]
pub struct SharedSemanticsRef {
    pub revision: String,
    pub declaration_digest: String,
}

#[derive(Clone, Debug)]
pub struct HostExecutionConfig {
    pub domains: BTreeMap<String, DomainExecutionPaths>,
    pub working_directory: PathBuf,
    pub temporary_directory: PathBuf,
    pub report_directory: PathBuf,
    pub shard_parallelism: Option<usize>,
    pub bytes: Vec<u8>,
}

impl HostExecutionConfig {
    pub fn parallelism(&self) -> usize {
        self.shard_parallelism
            .unwrap_or_else(|| std::thread::available_parallelism().map_or(1, usize::from))
    }
}

#[derive(Clone, Debug)]
pub struct DomainExecutionPaths {
    pub producer_path: PathBuf,
    pub checker_path: PathBuf,
}

#[derive(Clone, Debug)]
pub struct ProducerDescriptor {
    pub domain: String,
    pub producer_api_revision: String,
    pub adapter_revision: String,
    pub target_triple: String,
    pub rustc_identity: String,
    pub semantics_revision: String,
    pub semantics_declaration_digest: String,
    pub projection_revision: String,
    pub projection_declarations: Vec<Value>,
    pub resource_revision: String,
    pub resource_declaration_digest: String,
    pub max_batch_cases: u64,
    pub bytes: Vec<u8>,
    pub digest: String,
}

#[derive(Clone, Debug)]
pub struct DecodedProjection {
    pub family: String,
    pub requested_observation_kinds: Vec<String>,
    pub applicable_coverage_axes: Vec<String>,
    pub bindings: Vec<SelectorBinding>,
}

#[derive(Clone, Debug)]
pub struct SelectorBinding {
    pub axis: String,
    pub selector: String,
    pub modulus: u64,
    pub buckets_by_remainder: BTreeMap<u64, String>,
    pub bucket_names: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct GeneratedRequestCase {
    pub case_id: String,
    pub global_case_index: u64,
    pub domain_case_index: u64,
    pub family: String,
    pub requested_observation_kind: String,
    pub selectors: BTreeMap<String, u64>,
}

#[derive(Clone, Debug)]
pub struct GeneratedRequestBatch {
    pub adapter_revision: String,
    pub domain: String,
    pub algorithm: String,
    pub draw_schedule_revision: String,
    pub seed: String,
    pub cases: Vec<GeneratedRequestCase>,
}

pub fn object<const N: usize>(fields: [(&str, Value); N]) -> Value {
    Value::Object(
        fields
            .into_iter()
            .map(|(key, value)| (key.to_owned(), value))
            .collect(),
    )
}

pub fn encode_u64_hex(value: u64) -> String {
    format!("{value:016x}")
}

pub fn decode_u64_hex(text: &str, operation: &str, field: &str) -> Result<u64, ProcessFailure> {
    if text.len() != 16
        || !text
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err(ProcessFailure::new(
            "CanonicalDecodeFailure",
            operation,
            format!("{field} must be 16 lowercase hexadecimal digits"),
        ));
    }
    u64::from_str_radix(text, 16).map_err(|error| {
        ProcessFailure::new(
            "CanonicalDecodeFailure",
            operation,
            format!("{field} is not a u64: {error}"),
        )
    })
}

pub fn decode_canonical_bytes(bytes: &[u8], operation: &str) -> Result<Value, ProcessFailure> {
    std::str::from_utf8(bytes).map_err(|error| {
        ProcessFailure::new(
            "CanonicalDecodeFailure",
            operation,
            format!("bytes are not UTF-8: {error}"),
        )
    })?;
    let value: Value = serde_json::from_slice(bytes).map_err(|error| {
        ProcessFailure::new(
            "CanonicalDecodeFailure",
            operation,
            format!("JSON decode failed: {error}"),
        )
    })?;
    let encoded = canonical_json(&value)
        .map_err(|error| ProcessFailure::new("CanonicalDecodeFailure", operation, error))?;
    if encoded != bytes {
        return Err(ProcessFailure::new(
            "CanonicalDecodeFailure",
            operation,
            "decode/re-encode bytes differ",
        ));
    }
    Ok(value)
}

pub fn decode_canonical_file(
    path: &Path,
    operation: &str,
) -> Result<(Value, Vec<u8>), ProcessFailure> {
    let bytes = fs::read(path).map_err(|error| {
        ProcessFailure::new(
            "CanonicalDecodeFailure",
            operation,
            format!("{}: {error}", path.display()),
        )
    })?;
    let value = decode_canonical_bytes(&bytes, operation)?;
    Ok((value, bytes))
}

fn as_object<'a>(
    value: &'a Value,
    operation: &str,
    name: &str,
) -> Result<&'a Map<String, Value>, ProcessFailure> {
    value.as_object().ok_or_else(|| {
        ProcessFailure::new(
            "CanonicalDecodeFailure",
            operation,
            format!("{name} must be an object"),
        )
    })
}

fn exact_keys(
    object: &Map<String, Value>,
    keys: &[&str],
    operation: &str,
    name: &str,
) -> Result<(), ProcessFailure> {
    let expected: BTreeSet<&str> = keys.iter().copied().collect();
    let actual: BTreeSet<&str> = object.keys().map(String::as_str).collect();
    if actual == expected {
        Ok(())
    } else {
        Err(ProcessFailure::new(
            "ManifestValidationFailure",
            operation,
            format!("{name} fields differ from the required set"),
        ))
        .map_err(|failure| {
            failure.with_witness(crate::protocol::object([
                (
                    "actual",
                    Value::Array(
                        actual
                            .into_iter()
                            .map(|key| Value::String(key.to_owned()))
                            .collect(),
                    ),
                ),
                (
                    "expected",
                    Value::Array(
                        expected
                            .into_iter()
                            .map(|key| Value::String(key.to_owned()))
                            .collect(),
                    ),
                ),
            ]))
        })
    }
}

fn required_str(
    object: &Map<String, Value>,
    key: &str,
    operation: &str,
    name: &str,
) -> Result<String, ProcessFailure> {
    object
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| {
            ProcessFailure::new(
                "UnboundBuildMetadata",
                operation,
                format!("{name}.{key} is a required string"),
            )
        })
}

fn required_u64(
    object: &Map<String, Value>,
    key: &str,
    operation: &str,
    name: &str,
) -> Result<u64, ProcessFailure> {
    object.get(key).and_then(Value::as_u64).ok_or_else(|| {
        ProcessFailure::new(
            "UnboundBuildMetadata",
            operation,
            format!("{name}.{key} is a required u64"),
        )
    })
}

fn required_array<'a>(
    object: &'a Map<String, Value>,
    key: &str,
    operation: &str,
    name: &str,
) -> Result<&'a Vec<Value>, ProcessFailure> {
    object.get(key).and_then(Value::as_array).ok_or_else(|| {
        ProcessFailure::new(
            "UnboundBuildMetadata",
            operation,
            format!("{name}.{key} is a required array"),
        )
    })
}

fn required_object<'a>(
    object: &'a Map<String, Value>,
    key: &str,
    operation: &str,
    name: &str,
) -> Result<&'a Map<String, Value>, ProcessFailure> {
    object.get(key).and_then(Value::as_object).ok_or_else(|| {
        ProcessFailure::new(
            "UnboundBuildMetadata",
            operation,
            format!("{name}.{key} is a required object"),
        )
    })
}

fn walk_forbidden(value: &Value, operation: &str) -> Result<(), ProcessFailure> {
    match value {
        Value::Object(fields) => {
            for (key, child) in fields {
                if FORBIDDEN_MANIFEST_KEYS.contains(&key.as_str()) {
                    return Err(ProcessFailure::new(
                        "ManifestValidationFailure",
                        operation,
                        format!("LogicalManifest contains forbidden field {key}"),
                    ));
                }
                walk_forbidden(child, operation)?;
            }
            Ok(())
        }
        Value::Array(values) => {
            for child in values {
                walk_forbidden(child, operation)?;
            }
            Ok(())
        }
        Value::String(text) if text.starts_with('/') => Err(ProcessFailure::new(
            "ManifestValidationFailure",
            operation,
            "LogicalManifest contains an absolute path",
        )
        .with_witness(object([("value", Value::String(text.clone()))]))),
        _ => Ok(()),
    }
}

pub fn decode_semantics_ref(
    value: &Value,
    operation: &str,
) -> Result<SemanticsRef, ProcessFailure> {
    let object = as_object(value, operation, "SemanticsRef")?;
    exact_keys(
        object,
        &[
            "checkerRevision",
            "declarationDigest",
            "protocolRevision",
            "sharedSemanticsRefs",
        ],
        operation,
        "SemanticsRef",
    )?;
    if object.contains_key("gitObjectId") {
        return Err(ProcessFailure::new(
            "ManifestValidationFailure",
            operation,
            "SemanticsRef must not contain GitObjectId",
        ));
    }
    let shared = required_array(object, "sharedSemanticsRefs", operation, "SemanticsRef")?;
    let mut shared_semantics_refs = Vec::new();
    for item in shared {
        let item_object = as_object(item, operation, "sharedSemanticsRef")?;
        exact_keys(
            item_object,
            &["declarationDigest", "revision"],
            operation,
            "sharedSemanticsRef",
        )?;
        shared_semantics_refs.push(SharedSemanticsRef {
            revision: required_str(item_object, "revision", operation, "sharedSemanticsRef")?,
            declaration_digest: required_str(
                item_object,
                "declarationDigest",
                operation,
                "sharedSemanticsRef",
            )?,
        });
    }
    Ok(SemanticsRef {
        declaration_digest: required_str(object, "declarationDigest", operation, "SemanticsRef")?,
        protocol_revision: required_str(object, "protocolRevision", operation, "SemanticsRef")?,
        checker_revision: required_str(object, "checkerRevision", operation, "SemanticsRef")?,
        shared_semantics_refs,
    })
}

pub fn semantics_ref_value(reference: &SemanticsRef) -> Value {
    object([
        (
            "checkerRevision",
            Value::String(reference.checker_revision.clone()),
        ),
        (
            "declarationDigest",
            Value::String(reference.declaration_digest.clone()),
        ),
        (
            "protocolRevision",
            Value::String(reference.protocol_revision.clone()),
        ),
        (
            "sharedSemanticsRefs",
            Value::Array(
                reference
                    .shared_semantics_refs
                    .iter()
                    .map(|item| {
                        object([
                            (
                                "declarationDigest",
                                Value::String(item.declaration_digest.clone()),
                            ),
                            ("revision", Value::String(item.revision.clone())),
                        ])
                    })
                    .collect(),
            ),
        ),
    ])
}

pub fn decode_logical_manifest(bytes: &[u8]) -> Result<LogicalManifest, ProcessFailure> {
    let value = decode_canonical_bytes(bytes, "decodeLogicalManifest")?;
    walk_forbidden(&value, "decodeLogicalManifest")?;
    let object = as_object(&value, "decodeLogicalManifest", "LogicalManifest")?;
    exact_keys(
        object,
        &[
            "coverage",
            "domains",
            "generator",
            "lanes",
            "protocolRevision",
            "schema",
            "timeoutPolicy",
        ],
        "decodeLogicalManifest",
        "LogicalManifest",
    )?;
    let schema = required_str(object, "schema", "decodeLogicalManifest", "LogicalManifest")?;
    if schema != LOGICAL_MANIFEST_SCHEMA {
        return Err(ProcessFailure::new(
            "ManifestValidationFailure",
            "decodeLogicalManifest",
            "schema must be neco.verification.logical-manifest.v1",
        ));
    }
    let protocol_revision = required_str(
        object,
        "protocolRevision",
        "decodeLogicalManifest",
        "LogicalManifest",
    )?;
    if protocol_revision != PROTOCOL_REVISION {
        return Err(ProcessFailure::new(
            "ManifestValidationFailure",
            "decodeLogicalManifest",
            "protocolRevision must be protocol-v1",
        ));
    }
    let generator = decode_generator(required_object(
        object,
        "generator",
        "decodeLogicalManifest",
        "LogicalManifest",
    )?)?;
    let domains = decode_domain_requirements(required_array(
        object,
        "domains",
        "decodeLogicalManifest",
        "LogicalManifest",
    )?)?;
    let coverage = decode_coverage_declaration(required_object(
        object,
        "coverage",
        "decodeLogicalManifest",
        "LogicalManifest",
    )?)?;
    let lanes = required_array(object, "lanes", "decodeLogicalManifest", "LogicalManifest")?
        .iter()
        .map(|lane| {
            lane.as_str().map(str::to_owned).ok_or_else(|| {
                ProcessFailure::new(
                    "ManifestValidationFailure",
                    "decodeLogicalManifest",
                    "lane names must be strings",
                )
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    if lanes.as_slice()
        != SEVEN_LANES
            .iter()
            .map(|lane| (*lane).to_owned())
            .collect::<Vec<_>>()
    {
        return Err(ProcessFailure::new(
            "ManifestValidationFailure",
            "decodeLogicalManifest",
            "lanes must be the seven verification lanes in declared order",
        ));
    }
    let timeout_policy = decode_timeout_policy(required_object(
        object,
        "timeoutPolicy",
        "decodeLogicalManifest",
        "LogicalManifest",
    )?)?;
    validate_tier_allocation(&generator, &domains)?;
    Ok(LogicalManifest {
        protocol_revision,
        generator,
        domains,
        coverage,
        lanes,
        timeout_policy,
        digest: sha256(bytes),
        bytes: bytes.to_vec(),
    })
}

fn decode_generator(object: &Map<String, Value>) -> Result<GeneratorDeclaration, ProcessFailure> {
    let explicit = object.contains_key("normalCaseCount") || object.contains_key("allocation");
    let mut keys = vec![
        "algorithm",
        "drawScheduleRevision",
        "revision",
        "seed",
        "shardCaseCount",
        "tier",
    ];
    if explicit {
        keys.extend(["allocation", "normalCaseCount"]);
    }
    exact_keys(object, &keys, "decodeLogicalManifest", "generator")?;
    let algorithm = required_str(object, "algorithm", "decodeLogicalManifest", "generator")?;
    if algorithm != GENERATOR_ALGORITHM {
        return Err(ProcessFailure::new(
            "ManifestValidationFailure",
            "decodeLogicalManifest",
            "generator.algorithm must be splitmix64-v1",
        ));
    }
    let draw_schedule_revision = required_str(
        object,
        "drawScheduleRevision",
        "decodeLogicalManifest",
        "generator",
    )?;
    if draw_schedule_revision != DRAW_SCHEDULE_REVISION {
        return Err(ProcessFailure::new(
            "ManifestValidationFailure",
            "decodeLogicalManifest",
            "generator.drawScheduleRevision must be draw-schedule-v1",
        ));
    }
    let seed = required_str(object, "seed", "decodeLogicalManifest", "generator")?;
    let seed_value = decode_u64_hex(&seed, "decodeLogicalManifest", "generator.seed")?;
    let explicit_allocation = if explicit {
        let allocation_object =
            required_object(object, "allocation", "decodeLogicalManifest", "generator")?;
        let mut allocation = BTreeMap::new();
        for (domain, count) in allocation_object {
            let count = count.as_u64().ok_or_else(|| {
                ProcessFailure::new(
                    "ManifestValidationFailure",
                    "decodeLogicalManifest",
                    "allocation counts must be u64",
                )
            })?;
            allocation.insert(domain.clone(), count);
        }
        Some(ExplicitAllocation {
            normal_case_count: required_u64(
                object,
                "normalCaseCount",
                "decodeLogicalManifest",
                "generator",
            )?,
            allocation,
        })
    } else {
        None
    };
    Ok(GeneratorDeclaration {
        revision: required_str(object, "revision", "decodeLogicalManifest", "generator")?,
        algorithm,
        draw_schedule_revision,
        seed,
        seed_value,
        tier: required_str(object, "tier", "decodeLogicalManifest", "generator")?,
        shard_case_count: required_u64(
            object,
            "shardCaseCount",
            "decodeLogicalManifest",
            "generator",
        )?,
        explicit_allocation,
    })
}

fn decode_domain_requirements(values: &[Value]) -> Result<Vec<DomainRequirement>, ProcessFailure> {
    let mut domains = Vec::new();
    let mut seen = BTreeSet::new();
    for value in values {
        let object = as_object(value, "decodeLogicalManifest", "domain")?;
        exact_keys(
            object,
            &[
                "adapterRevision",
                "checkerApiRevision",
                "coverageMinimum",
                "domain",
                "families",
                "observationSchema",
                "producerApiRevision",
                "projectionRevision",
                "requestSchema",
                "semanticsRef",
            ],
            "decodeLogicalManifest",
            "domain",
        )?;
        let domain = required_str(object, "domain", "decodeLogicalManifest", "domain")?;
        if !matches!(
            domain.as_str(),
            "neco.algebra" | "neco.calculus" | "neco.geometry"
        ) {
            return Err(ProcessFailure::new(
                "ManifestValidationFailure",
                "decodeLogicalManifest",
                "domain must be neco.algebra, neco.calculus, or neco.geometry",
            ));
        }
        if !seen.insert(domain.clone()) {
            return Err(ProcessFailure::new(
                "ManifestValidationFailure",
                "decodeLogicalManifest",
                "domain entries must be unique",
            ));
        }
        let request_schema =
            required_str(object, "requestSchema", "decodeLogicalManifest", "domain")?;
        if request_schema != REQUEST_SCHEMA {
            return Err(ProcessFailure::new(
                "ManifestValidationFailure",
                "decodeLogicalManifest",
                "requestSchema must be neco.verification.generated-request-batch.v1",
            ));
        }
        let families = decode_family_requirements(required_array(
            object,
            "families",
            "decodeLogicalManifest",
            "domain",
        )?)?;
        let coverage_minimum = decode_coverage_minima(required_array(
            object,
            "coverageMinimum",
            "decodeLogicalManifest",
            "domain",
        )?)?;
        domains.push(DomainRequirement {
            domain,
            request_schema,
            observation_schema: required_str(
                object,
                "observationSchema",
                "decodeLogicalManifest",
                "domain",
            )?,
            producer_api_revision: required_str(
                object,
                "producerApiRevision",
                "decodeLogicalManifest",
                "domain",
            )?,
            checker_api_revision: required_str(
                object,
                "checkerApiRevision",
                "decodeLogicalManifest",
                "domain",
            )?,
            adapter_revision: required_str(
                object,
                "adapterRevision",
                "decodeLogicalManifest",
                "domain",
            )?,
            projection_revision: required_str(
                object,
                "projectionRevision",
                "decodeLogicalManifest",
                "domain",
            )?,
            semantics_ref: decode_semantics_ref(
                object.get("semanticsRef").ok_or_else(|| {
                    ProcessFailure::new(
                        "UnboundBuildMetadata",
                        "decodeLogicalManifest",
                        "domain.semanticsRef is required",
                    )
                })?,
                "decodeLogicalManifest",
            )?,
            families,
            coverage_minimum,
        });
    }
    if domains.len() != 3 {
        return Err(ProcessFailure::new(
            "ManifestValidationFailure",
            "decodeLogicalManifest",
            "LogicalManifest must declare three domains",
        ));
    }
    let names: Vec<_> = domains
        .iter()
        .map(|domain| domain.domain.as_str())
        .collect();
    if names != ["neco.algebra", "neco.calculus", "neco.geometry"] {
        return Err(ProcessFailure::new(
            "ManifestValidationFailure",
            "decodeLogicalManifest",
            "domains must appear in UTF-8 byte order",
        ));
    }
    Ok(domains)
}

fn decode_family_requirements(values: &[Value]) -> Result<Vec<FamilyRequirement>, ProcessFailure> {
    let mut families = Vec::new();
    let mut seen = BTreeSet::new();
    for value in values {
        let object = as_object(value, "decodeLogicalManifest", "family")?;
        exact_keys(
            object,
            &["family", "minimum", "requestedObservationKinds", "weight"],
            "decodeLogicalManifest",
            "family",
        )?;
        let family = required_str(object, "family", "decodeLogicalManifest", "family")?;
        if !seen.insert(family.clone()) {
            return Err(ProcessFailure::new(
                "ManifestValidationFailure",
                "decodeLogicalManifest",
                "family names must be unique in a domain",
            ));
        }
        let kinds = required_array(
            object,
            "requestedObservationKinds",
            "decodeLogicalManifest",
            "family",
        )?
        .iter()
        .map(|kind| {
            kind.as_str().map(str::to_owned).ok_or_else(|| {
                ProcessFailure::new(
                    "ManifestValidationFailure",
                    "decodeLogicalManifest",
                    "requestedObservationKinds entries must be strings",
                )
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
        if kinds.is_empty() {
            return Err(ProcessFailure::new(
                "ManifestValidationFailure",
                "decodeLogicalManifest",
                "requestedObservationKinds must be non-empty",
            ));
        }
        families.push(FamilyRequirement {
            family,
            weight: required_u64(object, "weight", "decodeLogicalManifest", "family")?,
            minimum: required_u64(object, "minimum", "decodeLogicalManifest", "family")?,
            requested_observation_kinds: kinds,
        });
    }
    Ok(families)
}

fn decode_coverage_minima(values: &[Value]) -> Result<Vec<CoverageMinimum>, ProcessFailure> {
    let mut minima = Vec::new();
    for value in values {
        let object = as_object(value, "decodeLogicalManifest", "coverageMinimum")?;
        exact_keys(
            object,
            &["axis", "bucket", "family", "minimum"],
            "decodeLogicalManifest",
            "coverageMinimum",
        )?;
        minima.push(CoverageMinimum {
            family: required_str(object, "family", "decodeLogicalManifest", "coverageMinimum")?,
            axis: required_str(object, "axis", "decodeLogicalManifest", "coverageMinimum")?,
            bucket: required_str(object, "bucket", "decodeLogicalManifest", "coverageMinimum")?,
            minimum: required_u64(
                object,
                "minimum",
                "decodeLogicalManifest",
                "coverageMinimum",
            )?,
        });
    }
    Ok(minima)
}

fn decode_coverage_declaration(
    object: &Map<String, Value>,
) -> Result<CoverageDeclaration, ProcessFailure> {
    exact_keys(
        object,
        &["invalidProtocolExpectedVerdict", "mutationExpectedVerdict"],
        "decodeLogicalManifest",
        "coverage",
    )?;
    let mutation = required_str(
        object,
        "mutationExpectedVerdict",
        "decodeLogicalManifest",
        "coverage",
    )?;
    let invalid = required_str(
        object,
        "invalidProtocolExpectedVerdict",
        "decodeLogicalManifest",
        "coverage",
    )?;
    if mutation != "refuted" {
        return Err(ProcessFailure::new(
            "ManifestValidationFailure",
            "decodeLogicalManifest",
            "mutationExpectedVerdict must be refuted",
        ));
    }
    if invalid != "invalid" {
        return Err(ProcessFailure::new(
            "ManifestValidationFailure",
            "decodeLogicalManifest",
            "invalidProtocolExpectedVerdict must be invalid",
        ));
    }
    Ok(CoverageDeclaration {
        mutation_expected_verdict: mutation,
        invalid_protocol_expected_verdict: invalid,
    })
}

fn decode_timeout_policy(object: &Map<String, Value>) -> Result<TimeoutPolicy, ProcessFailure> {
    exact_keys(
        object,
        &["laneTimeoutSeconds", "shardTimeoutSeconds"],
        "decodeLogicalManifest",
        "timeoutPolicy",
    )?;
    Ok(TimeoutPolicy {
        shard_timeout_seconds: required_u64(
            object,
            "shardTimeoutSeconds",
            "decodeLogicalManifest",
            "timeoutPolicy",
        )?,
        lane_timeout_seconds: required_u64(
            object,
            "laneTimeoutSeconds",
            "decodeLogicalManifest",
            "timeoutPolicy",
        )?,
    })
}

fn validate_tier_allocation(
    generator: &GeneratorDeclaration,
    domains: &[DomainRequirement],
) -> Result<(), ProcessFailure> {
    if !matches!(generator.tier.as_str(), "smoke" | "standard" | "stress") {
        return Err(ProcessFailure::new(
            "ManifestValidationFailure",
            "decodeLogicalManifest",
            "tier must be smoke, standard, or stress",
        ));
    }
    let Some(explicit) = &generator.explicit_allocation else {
        return Ok(());
    };
    let weights: Vec<(String, u64)> = domains
        .iter()
        .map(|domain| (domain.domain.clone(), 1))
        .collect();
    let apportioned = hamilton_apportionment(explicit.normal_case_count, &weights)?;
    if explicit.allocation != apportioned {
        return Err(ProcessFailure::new(
            "ManifestValidationFailure",
            "decodeLogicalManifest",
            "allocation must be the equal-weight Hamilton apportionment",
        ));
    }
    Ok(())
}

pub fn hamilton_apportionment(
    total: u64,
    weights: &[(String, u64)],
) -> Result<BTreeMap<String, u64>, ProcessFailure> {
    if weights.is_empty() {
        return Err(ProcessFailure::new(
            "ManifestValidationFailure",
            "hamiltonApportionment",
            "weights must be non-empty",
        ));
    }
    let weight_sum: u64 = weights.iter().map(|(_, weight)| *weight).sum();
    if weight_sum == 0 {
        return Err(ProcessFailure::new(
            "ManifestValidationFailure",
            "hamiltonApportionment",
            "weights must be positive",
        ));
    }
    let mut floors = BTreeMap::new();
    let mut remainders = Vec::new();
    for (key, weight) in weights {
        let product = (*weight as u128).saturating_mul(total as u128);
        let floor = (product / weight_sum as u128) as u64;
        let remainder = product % weight_sum as u128;
        floors.insert(key.clone(), floor);
        remainders.push((remainder, key.clone()));
    }
    let assigned: u64 = floors.values().sum();
    let seats_left = total.saturating_sub(assigned);
    remainders.sort_by(|left, right| {
        right
            .0
            .cmp(&left.0)
            .then_with(|| left.1.as_bytes().cmp(right.1.as_bytes()))
    });
    for (_, key) in remainders.into_iter().take(seats_left as usize) {
        if let Some(count) = floors.get_mut(&key) {
            *count = count.saturating_add(1);
        }
    }
    Ok(floors)
}

pub fn decode_host_execution_config(bytes: &[u8]) -> Result<HostExecutionConfig, ProcessFailure> {
    let value = decode_canonical_bytes(bytes, "decodeHostExecutionConfig")?;
    let object = as_object(&value, "decodeHostExecutionConfig", "HostExecutionConfig")?;
    let mut host_keys = vec![
        "domains",
        "reportDirectory",
        "temporaryDirectory",
        "workingDirectory",
    ];
    if object.contains_key("shardParallelism") {
        host_keys.push("shardParallelism");
    }
    exact_keys(
        object,
        &host_keys,
        "decodeHostExecutionConfig",
        "HostExecutionConfig",
    )?;
    let shard_parallelism = match object.get("shardParallelism") {
        None => None,
        Some(value) => Some(value.as_u64().filter(|n| *n > 0).ok_or_else(|| {
            ProcessFailure::new(
                "HostExecutionConfigInvalid",
                "decodeHostExecutionConfig",
                "shardParallelism must be a positive integer",
            )
        })? as usize),
    };
    let working = required_str(
        object,
        "workingDirectory",
        "decodeHostExecutionConfig",
        "HostExecutionConfig",
    )?;
    let temporary = required_str(
        object,
        "temporaryDirectory",
        "decodeHostExecutionConfig",
        "HostExecutionConfig",
    )?;
    let report = required_str(
        object,
        "reportDirectory",
        "decodeHostExecutionConfig",
        "HostExecutionConfig",
    )?;
    reject_empty_host_path(&working, "workingDirectory")?;
    reject_empty_host_path(&temporary, "temporaryDirectory")?;
    reject_empty_host_path(&report, "reportDirectory")?;
    let domains_object = required_object(
        object,
        "domains",
        "decodeHostExecutionConfig",
        "HostExecutionConfig",
    )?;
    let mut domains = BTreeMap::new();
    for (domain, entry) in domains_object {
        let entry = as_object(entry, "decodeHostExecutionConfig", "domainPaths")?;
        exact_keys(
            entry,
            &["checkerPath", "producerPath"],
            "decodeHostExecutionConfig",
            "domainPaths",
        )?;
        let producer_path = required_str(
            entry,
            "producerPath",
            "decodeHostExecutionConfig",
            "domainPaths",
        )?;
        let checker_path = required_str(
            entry,
            "checkerPath",
            "decodeHostExecutionConfig",
            "domainPaths",
        )?;
        reject_empty_host_path(&producer_path, "producerPath")?;
        reject_empty_host_path(&checker_path, "checkerPath")?;
        domains.insert(
            domain.clone(),
            DomainExecutionPaths {
                producer_path: PathBuf::from(producer_path),
                checker_path: PathBuf::from(checker_path),
            },
        );
    }
    Ok(HostExecutionConfig {
        domains,
        working_directory: PathBuf::from(working),
        temporary_directory: PathBuf::from(temporary),
        report_directory: PathBuf::from(report),
        shard_parallelism,
        bytes: bytes.to_vec(),
    })
}

fn reject_empty_host_path(path: &str, field: &str) -> Result<(), ProcessFailure> {
    if path.is_empty() {
        return Err(ProcessFailure::new(
            "HostExecutionConfigInvalid",
            "decodeHostExecutionConfig",
            format!("{field} must not be empty"),
        ));
    }
    Ok(())
}

pub fn resolve_host_path(base: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base.join(path)
    }
}

pub fn validate_host_execution_config(
    config: &HostExecutionConfig,
    domains: &[DomainRequirement],
) -> Result<HostExecutionConfig, ProcessFailure> {
    let working = if config.working_directory.is_absolute() {
        config.working_directory.clone()
    } else {
        return Err(ProcessFailure::new(
            "HostExecutionConfigInvalid",
            "validateHostExecutionConfig",
            "workingDirectory must be an absolute path at child start",
        ));
    };
    let mut resolved = HostExecutionConfig {
        domains: BTreeMap::new(),
        working_directory: working.clone(),
        temporary_directory: resolve_host_path(&working, &config.temporary_directory),
        report_directory: resolve_host_path(&working, &config.report_directory),
        shard_parallelism: config.shard_parallelism,
        bytes: config.bytes.clone(),
    };
    for domain in domains {
        let paths = config.domains.get(&domain.domain).ok_or_else(|| {
            ProcessFailure::new(
                "HostExecutionConfigInvalid",
                "validateHostExecutionConfig",
                format!("{} is missing from HostExecutionConfig", domain.domain),
            )
        })?;
        resolved.domains.insert(
            domain.domain.clone(),
            DomainExecutionPaths {
                producer_path: resolve_host_path(&working, &paths.producer_path),
                checker_path: resolve_host_path(&working, &paths.checker_path),
            },
        );
    }
    Ok(resolved)
}

pub fn decode_producer_descriptor(bytes: &[u8]) -> Result<ProducerDescriptor, ProcessFailure> {
    let value = decode_canonical_bytes(bytes, "decodeProducerDescriptor")?;
    let object = as_object(&value, "decodeProducerDescriptor", "ProducerDescriptor")?;
    for field in [
        "adapterRevision",
        "domain",
        "producerApiRevision",
        "projectionClaim",
        "resourceClaim",
        "rustcIdentity",
        "schema",
        "semanticsRef",
        "targetTriple",
    ] {
        if !object.contains_key(field) {
            return Err(ProcessFailure::new(
                "UnboundBuildMetadata",
                "decodeProducerDescriptor",
                format!("ProducerDescriptor.{field} is required"),
            ));
        }
    }
    if object.contains_key("sourceOID") || object.contains_key("sourceOid") {
        return Err(ProcessFailure::new(
            "UnboundBuildMetadata",
            "decodeProducerDescriptor",
            "ProducerDescriptor must not contain sourceOID",
        ));
    }
    let schema = required_str(
        object,
        "schema",
        "decodeProducerDescriptor",
        "ProducerDescriptor",
    )?;
    if schema != PRODUCER_DESCRIPTION_SCHEMA {
        return Err(ProcessFailure::new(
            "UnboundBuildMetadata",
            "decodeProducerDescriptor",
            "schema must be neco.verification.producer-description.v1",
        ));
    }
    let semantics = required_object(
        object,
        "semanticsRef",
        "decodeProducerDescriptor",
        "ProducerDescriptor",
    )?;
    let projection = required_object(
        object,
        "projectionClaim",
        "decodeProducerDescriptor",
        "ProducerDescriptor",
    )?;
    let resource = required_object(
        object,
        "resourceClaim",
        "decodeProducerDescriptor",
        "ProducerDescriptor",
    )?;
    if resource.contains_key("resourceMeasurementRevision")
        || resource.contains_key("measurementBuildInputRef")
    {
        return Err(ProcessFailure::new(
            "UnboundBuildMetadata",
            "decodeProducerDescriptor",
            "resourceClaim must be a declaration value",
        ));
    }
    let projection_declarations = required_array(
        projection,
        "projectionDeclarations",
        "decodeProducerDescriptor",
        "projectionClaim",
    )?
    .clone();
    let rustc_identity = required_str(
        object,
        "rustcIdentity",
        "decodeProducerDescriptor",
        "ProducerDescriptor",
    )?;
    if !rustc_identity.starts_with("rustc ") {
        return Err(ProcessFailure::new(
            "UnboundBuildMetadata",
            "decodeProducerDescriptor",
            "rustcIdentity must start with rustc ",
        ));
    }
    Ok(ProducerDescriptor {
        domain: required_str(
            object,
            "domain",
            "decodeProducerDescriptor",
            "ProducerDescriptor",
        )?,
        producer_api_revision: required_str(
            object,
            "producerApiRevision",
            "decodeProducerDescriptor",
            "ProducerDescriptor",
        )?,
        adapter_revision: required_str(
            object,
            "adapterRevision",
            "decodeProducerDescriptor",
            "ProducerDescriptor",
        )?,
        target_triple: required_str(
            object,
            "targetTriple",
            "decodeProducerDescriptor",
            "ProducerDescriptor",
        )?,
        rustc_identity,
        semantics_revision: required_str(
            semantics,
            "revision",
            "decodeProducerDescriptor",
            "semanticsRef",
        )?,
        semantics_declaration_digest: required_str(
            semantics,
            "declarationDigest",
            "decodeProducerDescriptor",
            "semanticsRef",
        )?,
        projection_revision: required_str(
            projection,
            "projectionRevision",
            "decodeProducerDescriptor",
            "projectionClaim",
        )?,
        projection_declarations,
        resource_revision: required_str(
            resource,
            "resourceRevision",
            "decodeProducerDescriptor",
            "resourceClaim",
        )?,
        resource_declaration_digest: required_str(
            resource,
            "resourceDeclarationDigest",
            "decodeProducerDescriptor",
            "resourceClaim",
        )?,
        max_batch_cases: required_u64(
            resource,
            "maxBatchCases",
            "decodeProducerDescriptor",
            "resourceClaim",
        )?,
        digest: sha256(bytes),
        bytes: bytes.to_vec(),
    })
}

pub fn decode_projection_declarations(
    declarations: &[Value],
) -> Result<Vec<DecodedProjection>, ProcessFailure> {
    let mut decoded = Vec::new();
    let mut seen = BTreeSet::new();
    for value in declarations {
        let object = as_object(
            value,
            "decodeProjectionDeclaration",
            "ProjectionDeclaration",
        )?;
        if object.contains_key("applicableAxes") {
            return Err(ProcessFailure::new(
                "UnboundDescriptorReference",
                "decodeProjectionDeclaration",
                "applicableAxes is not an accepted field",
            ));
        }
        let family = required_str(
            object,
            "family",
            "decodeProjectionDeclaration",
            "ProjectionDeclaration",
        )?;
        if !seen.insert(family.clone()) {
            return Err(ProcessFailure::new(
                "UnboundDescriptorReference",
                "decodeProjectionDeclaration",
                "family catalog names must be unique",
            ));
        }
        let kinds = required_array(
            object,
            "requestedObservationKinds",
            "decodeProjectionDeclaration",
            "ProjectionDeclaration",
        )?
        .iter()
        .map(|kind| {
            kind.as_str().map(str::to_owned).ok_or_else(|| {
                ProcessFailure::new(
                    "UnboundDescriptorReference",
                    "decodeProjectionDeclaration",
                    "requestedObservationKinds entries must be strings",
                )
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
        if kinds.is_empty() {
            return Err(ProcessFailure::new(
                "UnboundDescriptorReference",
                "decodeProjectionDeclaration",
                "requestedObservationKinds must be non-empty",
            ));
        }
        let axes = required_array(
            object,
            "applicableCoverageAxes",
            "decodeProjectionDeclaration",
            "ProjectionDeclaration",
        )?
        .iter()
        .map(|axis| {
            axis.as_str().map(str::to_owned).ok_or_else(|| {
                ProcessFailure::new(
                    "UnboundDescriptorReference",
                    "decodeProjectionDeclaration",
                    "applicableCoverageAxes entries must be strings",
                )
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
        let bindings = decode_selector_bindings(
            required_array(
                object,
                "selectorBindings",
                "decodeProjectionDeclaration",
                "ProjectionDeclaration",
            )?,
            &axes,
        )?;
        decoded.push(DecodedProjection {
            family,
            requested_observation_kinds: kinds,
            applicable_coverage_axes: axes,
            bindings,
        });
    }
    Ok(decoded)
}

fn decode_selector_bindings(
    values: &[Value],
    axes: &[String],
) -> Result<Vec<SelectorBinding>, ProcessFailure> {
    let mut bindings = Vec::new();
    let axis_set: BTreeSet<&str> = axes.iter().map(String::as_str).collect();
    for value in values {
        let object = as_object(value, "decodeProjectionDeclaration", "selectorBinding")?;
        if !object.contains_key("selectorRule") {
            return Err(ProcessFailure::new(
                "UnboundDescriptorReference",
                "decodeProjectionDeclaration",
                "selectorBinding.selectorRule is required",
            ));
        }
        let axis = required_str(
            object,
            "axis",
            "decodeProjectionDeclaration",
            "selectorBinding",
        )?;
        if !axis_set.contains(axis.as_str()) {
            return Err(ProcessFailure::new(
                "UnboundDescriptorReference",
                "decodeProjectionDeclaration",
                format!("axis {axis} is not an applicableCoverageAxes entry"),
            ));
        }
        let selector = required_str(
            object,
            "selector",
            "decodeProjectionDeclaration",
            "selectorBinding",
        )?;
        if !SELECTOR_NAMES.contains(&selector.as_str()) {
            return Err(ProcessFailure::new(
                "UnboundDescriptorReference",
                "decodeProjectionDeclaration",
                format!("selector {selector} is not a declared selector slot"),
            ));
        }
        let rule = required_object(
            object,
            "selectorRule",
            "decodeProjectionDeclaration",
            "selectorBinding",
        )?;
        exact_keys(
            rule,
            &["bucketsByRemainder", "modulus", "selector"],
            "decodeProjectionDeclaration",
            "selectorRule",
        )?;
        let rule_selector = required_str(
            rule,
            "selector",
            "decodeProjectionDeclaration",
            "selectorRule",
        )?;
        if rule_selector != selector {
            return Err(ProcessFailure::new(
                "UnboundDescriptorReference",
                "decodeProjectionDeclaration",
                "selectorRule.selector must match the binding selector",
            ));
        }
        let modulus = required_u64(
            rule,
            "modulus",
            "decodeProjectionDeclaration",
            "selectorRule",
        )?;
        if modulus == 0 {
            return Err(ProcessFailure::new(
                "UnboundDescriptorReference",
                "decodeProjectionDeclaration",
                "selectorRule.modulus must be positive",
            ));
        }
        let map = required_object(
            rule,
            "bucketsByRemainder",
            "decodeProjectionDeclaration",
            "selectorRule",
        )?;
        let mut buckets_by_remainder = BTreeMap::new();
        for (key, bucket) in map {
            let remainder = key.parse::<u64>().map_err(|_| {
                ProcessFailure::new(
                    "UnboundDescriptorReference",
                    "decodeProjectionDeclaration",
                    "bucketsByRemainder keys must be remainders",
                )
            })?;
            let bucket = bucket.as_str().map(str::to_owned).ok_or_else(|| {
                ProcessFailure::new(
                    "UnboundDescriptorReference",
                    "decodeProjectionDeclaration",
                    "bucketsByRemainder values must be bucket names",
                )
            })?;
            if remainder >= modulus {
                return Err(ProcessFailure::new(
                    "UnboundDescriptorReference",
                    "decodeProjectionDeclaration",
                    "remainder is outside modulus",
                ));
            }
            if buckets_by_remainder.insert(remainder, bucket).is_some() {
                return Err(ProcessFailure::new(
                    "UnboundDescriptorReference",
                    "decodeProjectionDeclaration",
                    "remainder maps to more than one bucket",
                ));
            }
        }
        if buckets_by_remainder.len() as u64 != modulus {
            return Err(ProcessFailure::new(
                "UnboundDescriptorReference",
                "decodeProjectionDeclaration",
                "selectorRule remainder map must be total",
            ));
        }
        for remainder in 0..modulus {
            if !buckets_by_remainder.contains_key(&remainder) {
                return Err(ProcessFailure::new(
                    "UnboundDescriptorReference",
                    "decodeProjectionDeclaration",
                    "each remainder below modulus must map to exactly one bucket",
                ));
            }
        }
        let mut bucket_names = Vec::new();
        if let Some(buckets) = object.get("buckets").and_then(Value::as_array) {
            for bucket in buckets {
                let bucket_object = as_object(bucket, "decodeProjectionDeclaration", "bucket")?;
                let name = required_str(
                    bucket_object,
                    "name",
                    "decodeProjectionDeclaration",
                    "bucket",
                )?;
                bucket_names.push(name);
            }
        } else {
            let mut unique = BTreeSet::new();
            for name in buckets_by_remainder.values() {
                unique.insert(name.clone());
            }
            bucket_names.extend(unique);
        }
        let reachable: BTreeSet<&str> = buckets_by_remainder.values().map(String::as_str).collect();
        for name in &bucket_names {
            if !reachable.contains(name.as_str()) {
                return Err(ProcessFailure::new(
                    "UnboundDescriptorReference",
                    "decodeProjectionDeclaration",
                    format!("bucket {name} is unreachable from selectorRule"),
                ));
            }
        }
        bindings.push(SelectorBinding {
            axis,
            selector,
            modulus,
            buckets_by_remainder,
            bucket_names,
        });
    }
    for axis in axes {
        if !bindings.iter().any(|binding| &binding.axis == axis) {
            return Err(ProcessFailure::new(
                "UnboundDescriptorReference",
                "decodeProjectionDeclaration",
                format!("applicableCoverageAxes entry {axis} has no selectorRule"),
            ));
        }
    }
    Ok(bindings)
}

pub fn validate_descriptor_against_domain(
    descriptor: &ProducerDescriptor,
    domain: &DomainRequirement,
) -> Result<Vec<DecodedProjection>, ProcessFailure> {
    if descriptor.domain != domain.domain {
        return Err(ProcessFailure::new(
            "BuildIdentityMismatch",
            "validateProducerDescriptor",
            "descriptor domain does not match LogicalManifest",
        ));
    }
    if descriptor.adapter_revision != domain.adapter_revision {
        return Err(ProcessFailure::new(
            "BuildIdentityMismatch",
            "validateProducerDescriptor",
            "adapterRevision does not match LogicalManifest",
        ));
    }
    if descriptor.producer_api_revision != domain.producer_api_revision {
        return Err(ProcessFailure::new(
            "BuildIdentityMismatch",
            "validateProducerDescriptor",
            "producerApiRevision does not match LogicalManifest",
        ));
    }
    if descriptor.semantics_declaration_digest != domain.semantics_ref.declaration_digest {
        return Err(ProcessFailure::new(
            "BuildIdentityMismatch",
            "validateProducerDescriptor",
            "semantics declarationDigest does not match LogicalManifest",
        ));
    }
    let declarations = decode_projection_declarations(&descriptor.projection_declarations)?;
    for family in &domain.families {
        let declaration = declarations
            .iter()
            .find(|item| item.family == family.family)
            .ok_or_else(|| {
                ProcessFailure::new(
                    "UnboundDescriptorReference",
                    "validateProducerDescriptor",
                    format!(
                        "family {} is not in the decoded ProjectionDeclaration catalog",
                        family.family
                    ),
                )
            })?;
        for kind in &family.requested_observation_kinds {
            if !declaration
                .requested_observation_kinds
                .iter()
                .any(|declared| declared == kind)
            {
                return Err(ProcessFailure::new(
                    "UnboundDescriptorReference",
                    "validateProducerDescriptor",
                    format!(
                        "requestedObservationKind {kind} is unbound for {}",
                        family.family
                    ),
                ));
            }
        }
    }
    for minimum in &domain.coverage_minimum {
        let declaration = declarations
            .iter()
            .find(|item| item.family == minimum.family)
            .ok_or_else(|| {
                ProcessFailure::new(
                    "UnboundDescriptorReference",
                    "validateProducerDescriptor",
                    format!("coverage minimum family {} is unbound", minimum.family),
                )
            })?;
        if !declaration
            .applicable_coverage_axes
            .iter()
            .any(|axis| axis == &minimum.axis)
        {
            return Err(ProcessFailure::new(
                "UnboundDescriptorReference",
                "validateProducerDescriptor",
                format!(
                    "coverage axis {} is unbound for {}",
                    minimum.axis, minimum.family
                ),
            ));
        }
        let binding = declaration
            .bindings
            .iter()
            .find(|binding| binding.axis == minimum.axis)
            .ok_or_else(|| {
                ProcessFailure::new(
                    "UnboundDescriptorReference",
                    "validateProducerDescriptor",
                    format!("selectorRule for axis {} is unbound", minimum.axis),
                )
            })?;
        if !binding
            .buckets_by_remainder
            .values()
            .any(|bucket| bucket == &minimum.bucket)
        {
            return Err(ProcessFailure::new(
                "UnboundDescriptorReference",
                "validateProducerDescriptor",
                format!(
                    "bucket {} is unbound for axis {}",
                    minimum.bucket, minimum.axis
                ),
            ));
        }
    }
    Ok(declarations)
}

pub fn runtime_host_matches_target_triple(target_triple: &str) -> bool {
    let arch = std::env::consts::ARCH;
    let os = std::env::consts::OS;
    let parts: Vec<&str> = target_triple.split('-').collect();
    let triple_arch = parts.first().copied();
    let os_ok = match os {
        "macos" => parts.contains(&"darwin"),
        "linux" => parts.contains(&"linux"),
        other => parts.last().copied() == Some(other),
    };
    triple_arch == Some(arch) && os_ok
}

pub fn validate_runtime_host_compatibility(target_triple: &str) -> Result<(), ProcessFailure> {
    if runtime_host_matches_target_triple(target_triple) {
        Ok(())
    } else {
        Err(ProcessFailure::new(
            "RuntimeHostCompatibilityFailure",
            "validateRuntimeHostCompatibility",
            "descriptor targetTriple does not match the current host ARCH/OS",
        )
        .with_witness(object([
            ("hostArch", Value::String(std::env::consts::ARCH.to_owned())),
            ("hostOs", Value::String(std::env::consts::OS.to_owned())),
            ("targetTriple", Value::String(target_triple.to_owned())),
        ])))
    }
}

pub fn generated_request_case_value(case: &GeneratedRequestCase) -> Value {
    let mut selectors = Map::new();
    for name in SELECTOR_NAMES {
        if let Some(value) = case.selectors.get(name) {
            selectors.insert(name.to_owned(), Value::String(encode_u64_hex(*value)));
        }
    }
    object([
        ("caseId", Value::String(case.case_id.clone())),
        (
            "domainCaseIndex",
            Value::String(encode_u64_hex(case.domain_case_index)),
        ),
        ("family", Value::String(case.family.clone())),
        (
            "globalCaseIndex",
            Value::String(encode_u64_hex(case.global_case_index)),
        ),
        (
            "requestedObservationKind",
            Value::String(case.requested_observation_kind.clone()),
        ),
        ("selectors", Value::Object(selectors)),
    ])
}

pub fn generated_request_batch_value(batch: &GeneratedRequestBatch) -> Value {
    object([
        (
            "adapterRevision",
            Value::String(batch.adapter_revision.clone()),
        ),
        (
            "cases",
            Value::Array(
                batch
                    .cases
                    .iter()
                    .map(generated_request_case_value)
                    .collect(),
            ),
        ),
        ("domain", Value::String(batch.domain.clone())),
        (
            "generator",
            object([
                ("algorithm", Value::String(batch.algorithm.clone())),
                (
                    "drawScheduleRevision",
                    Value::String(batch.draw_schedule_revision.clone()),
                ),
                ("seed", Value::String(batch.seed.clone())),
            ]),
        ),
        ("schema", Value::String(REQUEST_SCHEMA.to_owned())),
    ])
}

pub fn encode_generated_request_batch(
    batch: &GeneratedRequestBatch,
) -> Result<Vec<u8>, ProcessFailure> {
    canonical_json(&generated_request_batch_value(batch)).map_err(|error| {
        ProcessFailure::new(
            "CanonicalDecodeFailure",
            "encodeGeneratedRequestBatch",
            error,
        )
    })
}

pub fn decode_generated_request_batch(
    bytes: &[u8],
) -> Result<GeneratedRequestBatch, ProcessFailure> {
    let value = decode_canonical_bytes(bytes, "decodeGeneratedRequestBatch")?;
    let object = as_object(
        &value,
        "decodeGeneratedRequestBatch",
        "generated request batch",
    )?;
    exact_keys(
        object,
        &["adapterRevision", "cases", "domain", "generator", "schema"],
        "decodeGeneratedRequestBatch",
        "generated request batch",
    )?;
    let schema = required_str(
        object,
        "schema",
        "decodeGeneratedRequestBatch",
        "generated request batch",
    )?;
    if schema != REQUEST_SCHEMA {
        return Err(ProcessFailure::new(
            "CanonicalDecodeFailure",
            "decodeGeneratedRequestBatch",
            "schema must be neco.verification.generated-request-batch.v1",
        ));
    }
    let generator = required_object(
        object,
        "generator",
        "decodeGeneratedRequestBatch",
        "generated request batch",
    )?;
    exact_keys(
        generator,
        &["algorithm", "drawScheduleRevision", "seed"],
        "decodeGeneratedRequestBatch",
        "generator",
    )?;
    let cases = required_array(
        object,
        "cases",
        "decodeGeneratedRequestBatch",
        "generated request batch",
    )?;
    let mut decoded_cases = Vec::new();
    for case in cases {
        decoded_cases.push(decode_generated_request_case(case)?);
    }
    Ok(GeneratedRequestBatch {
        adapter_revision: required_str(
            object,
            "adapterRevision",
            "decodeGeneratedRequestBatch",
            "generated request batch",
        )?,
        domain: required_str(
            object,
            "domain",
            "decodeGeneratedRequestBatch",
            "generated request batch",
        )?,
        algorithm: required_str(
            generator,
            "algorithm",
            "decodeGeneratedRequestBatch",
            "generator",
        )?,
        draw_schedule_revision: required_str(
            generator,
            "drawScheduleRevision",
            "decodeGeneratedRequestBatch",
            "generator",
        )?,
        seed: required_str(
            generator,
            "seed",
            "decodeGeneratedRequestBatch",
            "generator",
        )?,
        cases: decoded_cases,
    })
}

fn decode_generated_request_case(value: &Value) -> Result<GeneratedRequestCase, ProcessFailure> {
    let object = as_object(value, "decodeGeneratedRequestBatch", "case")?;
    exact_keys(
        object,
        &[
            "caseId",
            "domainCaseIndex",
            "family",
            "globalCaseIndex",
            "requestedObservationKind",
            "selectors",
        ],
        "decodeGeneratedRequestBatch",
        "case",
    )?;
    let global_case_index = decode_u64_hex(
        &required_str(
            object,
            "globalCaseIndex",
            "decodeGeneratedRequestBatch",
            "case",
        )?,
        "decodeGeneratedRequestBatch",
        "globalCaseIndex",
    )?;
    let case_id = required_str(object, "caseId", "decodeGeneratedRequestBatch", "case")?;
    if case_id != format!("case-{}", encode_u64_hex(global_case_index)) {
        return Err(ProcessFailure::new(
            "CanonicalDecodeFailure",
            "decodeGeneratedRequestBatch",
            "caseId must be case- concatenated with the global case index",
        ));
    }
    let selectors_object =
        required_object(object, "selectors", "decodeGeneratedRequestBatch", "case")?;
    exact_keys(
        selectors_object,
        &SELECTOR_NAMES,
        "decodeGeneratedRequestBatch",
        "selectors",
    )?;
    let mut selectors = BTreeMap::new();
    for name in SELECTOR_NAMES {
        let text = required_str(
            selectors_object,
            name,
            "decodeGeneratedRequestBatch",
            "selectors",
        )?;
        selectors.insert(
            name.to_owned(),
            decode_u64_hex(&text, "decodeGeneratedRequestBatch", name)?,
        );
    }
    Ok(GeneratedRequestCase {
        case_id,
        global_case_index,
        domain_case_index: decode_u64_hex(
            &required_str(
                object,
                "domainCaseIndex",
                "decodeGeneratedRequestBatch",
                "case",
            )?,
            "decodeGeneratedRequestBatch",
            "domainCaseIndex",
        )?,
        family: required_str(object, "family", "decodeGeneratedRequestBatch", "case")?,
        requested_observation_kind: required_str(
            object,
            "requestedObservationKind",
            "decodeGeneratedRequestBatch",
            "case",
        )?,
        selectors,
    })
}

pub fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), ProcessFailure> {
    let parent = path.parent().ok_or_else(|| {
        ProcessFailure::new(
            "ShardEventWriteFailure",
            "atomicWrite",
            "report path has no parent directory",
        )
    })?;
    fs::create_dir_all(parent).map_err(|error| {
        ProcessFailure::new(
            "ShardEventWriteFailure",
            "atomicWrite",
            format!("{}: {error}", parent.display()),
        )
    })?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            ProcessFailure::new(
                "ShardEventWriteFailure",
                "atomicWrite",
                "report file name must be UTF-8",
            )
        })?;
    let temporary = parent.join(format!(".{file_name}.tmp"));
    fs::write(&temporary, bytes).map_err(|error| {
        ProcessFailure::new(
            "ShardEventWriteFailure",
            "atomicWrite",
            format!("{}: {error}", temporary.display()),
        )
    })?;
    fs::rename(&temporary, path).map_err(|error| {
        ProcessFailure::new(
            "ShardEventWriteFailure",
            "atomicWrite",
            format!("{}: {error}", path.display()),
        )
    })
}

pub fn sha256_file(path: &Path, operation: &str) -> Result<String, ProcessFailure> {
    let bytes = fs::read(path).map_err(|error| {
        ProcessFailure::new(operation, operation, format!("{}: {error}", path.display()))
    })?;
    Ok(sha256(&bytes))
}

pub fn current_host_target_triple() -> String {
    format!(
        "{}-{}-{}",
        std::env::consts::ARCH,
        match std::env::consts::OS {
            "macos" => "apple",
            "linux" => "unknown",
            other => other,
        },
        match std::env::consts::OS {
            "macos" => "darwin",
            other => other,
        }
    )
}
