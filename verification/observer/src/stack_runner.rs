use crate::canonical_json;
use crate::corpus_generator::{
    enforce_shard_at_most_producer_max, generate_all_cases, infeasible_minimum_failure,
    request_digest, shard_batches, write_shards,
};
use crate::coverage::{
    collect_corpus_coverage, coverage_value, enforce_declared_bucket_coverage,
    record_checker_verdict, record_producer_kind, validate_selector_rules, CoverageCounts,
};
use crate::mutation::{
    classification_report, expected_classification, invalid_protocol_vectors,
    mutate_canonical_observation, ExpectedClassification,
};
use crate::protocol::{
    atomic_write, decode_canonical_bytes, decode_canonical_file, decode_host_execution_config,
    decode_logical_manifest, decode_producer_descriptor, encode_generated_request_batch, object,
    validate_descriptor_against_domain, validate_host_execution_config,
    validate_runtime_host_compatibility, GeneratedRequestBatch, HostExecutionConfig,
    LogicalManifest, ProcessFailure, ProducerDescriptor, SEVEN_LANES,
};
use crate::sha256;
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

pub fn load_logical_manifest(path: &Path) -> Result<LogicalManifest, ProcessFailure> {
    let bytes = fs::read(path).map_err(|error| {
        ProcessFailure::new(
            "CanonicalDecodeFailure",
            "loadLogicalManifest",
            format!("{}: {error}", path.display()),
        )
    })?;
    decode_logical_manifest(&bytes)
}

pub fn load_host_execution_config(path: &Path) -> Result<HostExecutionConfig, ProcessFailure> {
    let bytes = fs::read(path).map_err(|error| {
        ProcessFailure::new(
            "HostExecutionConfigInvalid",
            "loadHostExecutionConfig",
            format!("{}: {error}", path.display()),
        )
    })?;
    decode_host_execution_config(&bytes)
}

fn spawn(
    program: &Path,
    args: &[&str],
    cwd: &Path,
    operation: &str,
    stdin: Option<&[u8]>,
) -> Result<Vec<u8>, ProcessFailure> {
    if !program.is_file() {
        return Err(ProcessFailure::new(
            "SpawnFailure",
            operation,
            format!("executable is missing: {}", program.display()),
        )
        .with_witness(object([(
            "command",
            Value::String(program.display().to_string()),
        )])));
    }
    let mut command = Command::new(program);
    command
        .args(args)
        .current_dir(cwd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if stdin.is_some() {
        command.stdin(Stdio::piped());
    }
    let mut child = command.spawn().map_err(|error| {
        ProcessFailure::new(
            "SpawnFailure",
            operation,
            format!("{}: {error}", program.display()),
        )
    })?;
    if let Some(bytes) = stdin {
        if let Some(mut handle) = child.stdin.take() {
            handle.write_all(bytes).map_err(|error| {
                ProcessFailure::new("SpawnFailure", operation, format!("stdin: {error}"))
            })?;
        }
    }
    let output = child.wait_with_output().map_err(|error| {
        ProcessFailure::new("SpawnFailure", operation, format!("wait: {error}"))
    })?;
    if !output.status.success() {
        return Err(ProcessFailure::new(
            "SpawnFailure",
            operation,
            format!("{operation} exited {}", output.status),
        )
        .with_witness(object([
            ("command", Value::String(program.display().to_string())),
            (
                "exitStatus",
                output.status.code().map(Value::from).unwrap_or(Value::Null),
            ),
            ("stderrDigest", Value::String(sha256(&output.stderr))),
            ("stdoutDigest", Value::String(sha256(&output.stdout))),
        ])));
    }
    Ok(output.stdout)
}

fn stdout_value(bytes: &[u8], operation: &str) -> Result<(Value, Vec<u8>), ProcessFailure> {
    let text = std::str::from_utf8(bytes).map_err(|error| {
        ProcessFailure::new(
            "CanonicalDecodeFailure",
            operation,
            format!("stdout is not UTF-8: {error}"),
        )
    })?;
    let trimmed = text.strip_suffix('\n').unwrap_or(text);
    let value = decode_canonical_bytes(trimmed.as_bytes(), operation)?;
    Ok((value, trimmed.as_bytes().to_vec()))
}

pub fn describe_producer(
    producer: &Path,
    cwd: &Path,
) -> Result<ProducerDescriptor, ProcessFailure> {
    let stdout = spawn(producer, &["describe"], cwd, "producerDescribe", None)?;
    let (_, bytes) = stdout_value(&stdout, "producerDescribe")?;
    decode_producer_descriptor(&bytes)
}

fn observation_kind(case: &Value) -> Option<String> {
    case.get("observation")
        .and_then(|observation| observation.get("kind"))
        .and_then(Value::as_str)
        .map(str::to_owned)
}

fn case_ids(value: &Value) -> Result<Vec<String>, ProcessFailure> {
    value
        .get("cases")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            ProcessFailure::new(
                "CaseCorrespondenceMismatch",
                "caseIds",
                "cases must be an array",
            )
        })?
        .iter()
        .map(|case| {
            case.get("caseId")
                .and_then(Value::as_str)
                .map(str::to_owned)
                .ok_or_else(|| {
                    ProcessFailure::new(
                        "CaseCorrespondenceMismatch",
                        "caseIds",
                        "caseId must be a string",
                    )
                })
        })
        .collect()
}

fn correspond(request: &GeneratedRequestBatch, observation: &Value) -> Result<(), ProcessFailure> {
    let ids = case_ids(observation)?;
    let expected: Vec<String> = request
        .cases
        .iter()
        .map(|case| case.case_id.clone())
        .collect();
    if ids != expected {
        return Err(ProcessFailure::new(
            "CaseCorrespondenceMismatch",
            "caseCorrespondence",
            "observation caseId values must match the request in number and order",
        ));
    }
    Ok(())
}

fn checker_case_ids(result: &Value) -> Result<Vec<String>, ProcessFailure> {
    if result.get("batchVerdict").is_some() {
        return Err(ProcessFailure::new(
            "CaseCorrespondenceMismatch",
            "checkerCaseIds",
            "checker returned a batchVerdict for a canonical observation batch",
        ));
    }
    case_ids(result)
}

fn generate_observation(
    producer: &Path,
    cwd: &Path,
    temporary: &Path,
    batch: &GeneratedRequestBatch,
    shard_index: usize,
) -> Result<(Value, Vec<u8>), ProcessFailure> {
    fs::create_dir_all(temporary).map_err(|error| {
        ProcessFailure::new(
            "ShardEventWriteFailure",
            "producerGenerate",
            format!("{}: {error}", temporary.display()),
        )
    })?;
    let request_bytes = encode_generated_request_batch(batch)?;
    let request_path = temporary.join(format!("request-{}-{shard_index:04}.json", batch.domain));
    atomic_write(&request_path, &request_bytes)?;
    let stdout = spawn(
        producer,
        &["generate", "--request", &request_path.to_string_lossy()],
        cwd,
        "producerGenerate",
        None,
    )?;
    let (value, bytes) = stdout_value(&stdout, "producerGenerate")?;
    correspond(batch, &value)?;
    Ok((value, bytes))
}

fn enforce_verified_normal_corpus(coverage: &CoverageCounts) -> Result<(), ProcessFailure> {
    let verified = coverage
        .checker_verdicts
        .get("verified")
        .copied()
        .unwrap_or(0);
    let unexpected: BTreeMap<String, u64> = coverage
        .checker_verdicts
        .iter()
        .filter(|(verdict, count)| *verdict != "verified" && **count > 0)
        .map(|(verdict, count)| (verdict.clone(), *count))
        .collect();
    if verified == 0 || !unexpected.is_empty() {
        return Err(ProcessFailure::new(
            "UnexpectedVerdict",
            "observationVerification",
            "normal corpus checker verdicts must all be verified",
        )
        .with_witness(coverage_value(coverage)));
    }
    Ok(())
}

fn bind_checker_result(
    result: &Value,
    batch_bytes: &[u8],
    operation: &str,
) -> Result<(), ProcessFailure> {
    crate::checker_result(result, &sha256(batch_bytes))
        .map(|_| ())
        .map_err(|message| ProcessFailure::new("CaseCorrespondenceMismatch", operation, message))
}

fn check_observation(
    checker: &Path,
    cwd: &Path,
    temporary: &Path,
    observation_bytes: &[u8],
    shard_index: usize,
    domain: &str,
) -> Result<(Value, Vec<u8>), ProcessFailure> {
    let observation_path = temporary.join(format!("observation-{domain}-{shard_index:04}.json"));
    atomic_write(&observation_path, observation_bytes)?;
    let stdout = spawn(
        checker,
        &["check", &observation_path.to_string_lossy()],
        cwd,
        "checkerCheck",
        None,
    )?;
    let (value, bytes) = stdout_value(&stdout, "checkerCheck")?;
    bind_checker_result(&value, observation_bytes, "checkerCheck")?;
    Ok((value, bytes))
}

fn self_check(
    producer: &Path,
    cwd: &Path,
    request_path: &Path,
    observation_path: &Path,
) -> Result<Value, ProcessFailure> {
    let stdout = spawn(
        producer,
        &[
            "self-check",
            "--request",
            &request_path.to_string_lossy(),
            "--batch",
            &observation_path.to_string_lossy(),
        ],
        cwd,
        "producerSelfCheck",
        None,
    )?;
    Ok(stdout_value(&stdout, "producerSelfCheck")?.0)
}

fn soundness(checker: &Path, cwd: &Path) -> Result<Value, ProcessFailure> {
    let stdout = spawn(checker, &["soundness"], cwd, "checkerSoundness", None)?;
    let text = std::str::from_utf8(&stdout).map_err(|error| {
        ProcessFailure::new(
            "CanonicalDecodeFailure",
            "checkerSoundness",
            format!("soundness stdout is not UTF-8: {error}"),
        )
    })?;
    Ok(object([
        ("outputDigest", Value::String(sha256(&stdout))),
        ("status", Value::String("passed".to_owned())),
        ("text", Value::String(text.trim().to_owned())),
    ]))
}

pub fn describe_all(
    manifest: &LogicalManifest,
    host: &HostExecutionConfig,
) -> Result<BTreeMap<String, ProducerDescriptor>, ProcessFailure> {
    let mut descriptors = BTreeMap::new();
    for domain in &manifest.domains {
        let paths = host.domains.get(&domain.domain).ok_or_else(|| {
            ProcessFailure::new(
                "HostExecutionConfigInvalid",
                "describeAll",
                format!("{} is missing from HostExecutionConfig", domain.domain),
            )
        })?;
        let descriptor = describe_producer(&paths.producer_path, &host.working_directory)?;
        validate_runtime_host_compatibility(&descriptor.target_triple)?;
        validate_descriptor_against_domain(&descriptor, domain)?;
        descriptors.insert(domain.domain.clone(), descriptor);
    }
    Ok(descriptors)
}

fn declarations_map(
    descriptors: &BTreeMap<String, ProducerDescriptor>,
) -> Result<BTreeMap<String, Vec<crate::protocol::DecodedProjection>>, ProcessFailure> {
    let mut map = BTreeMap::new();
    for (domain, descriptor) in descriptors {
        map.insert(
            domain.clone(),
            crate::protocol::decode_projection_declarations(&descriptor.projection_declarations)?,
        );
    }
    Ok(map)
}

fn write_shard_event(
    directory: &Path,
    index: usize,
    batch: &GeneratedRequestBatch,
    observation: &[u8],
    checker: &[u8],
) -> Result<String, ProcessFailure> {
    let request = encode_generated_request_batch(batch)?;
    let event = object([
        ("checkerDigest", Value::String(sha256(checker))),
        ("domain", Value::String(batch.domain.clone())),
        ("index", Value::from(index as u64)),
        ("observationDigest", Value::String(sha256(observation))),
        ("requestDigest", Value::String(sha256(&request))),
    ]);
    let encoded = canonical_json(&event)
        .map_err(|error| ProcessFailure::new("ShardEventWriteFailure", "writeShardEvent", error))?;
    let path = directory.join(format!("event-{index:04}.json"));
    atomic_write(&path, &encoded)?;
    Ok(sha256(&encoded))
}

struct CorpusRun {
    request_digest: String,
    observation_digest: String,
    shard_digests: Vec<String>,
    coverage: CoverageCounts,
    observation_bytes: Vec<(GeneratedRequestBatch, Vec<u8>, Value, Vec<u8>)>,
}

struct ShardOutput {
    event_digest: String,
    coverage: CoverageCounts,
    tuples: Vec<(String, u64, String)>,
    bytes: (GeneratedRequestBatch, Vec<u8>, Value, Vec<u8>),
}

fn run_shard(
    host: &HostExecutionConfig,
    descriptors: &BTreeMap<String, ProducerDescriptor>,
    declarations: &BTreeMap<String, Vec<crate::protocol::DecodedProjection>>,
    events_directory: &Path,
    index: usize,
    batch: &GeneratedRequestBatch,
) -> Result<ShardOutput, ProcessFailure> {
    let mut coverage = CoverageCounts::default();
    let producer_max = descriptors
        .get(&batch.domain)
        .map(|descriptor| descriptor.max_batch_cases)
        .ok_or_else(|| {
            ProcessFailure::new(
                "UnboundBuildMetadata",
                "runCorpus",
                format!("{} ProducerDescriptor is unbound", batch.domain),
            )
        })?;
    if batch.cases.len() as u64 > producer_max {
        return Err(ProcessFailure::new(
            "ShardExceedsProducerMaxBatchCases",
            "runCorpus",
            "shard exceeds producer maxBatchCases before child start",
        ));
    }
    let paths = host.domains.get(&batch.domain).ok_or_else(|| {
        ProcessFailure::new(
            "HostExecutionConfigInvalid",
            "runCorpus",
            format!("{} is missing from HostExecutionConfig", batch.domain),
        )
    })?;
    let (observation, observation_raw) = generate_observation(
        &paths.producer_path,
        &host.working_directory,
        &host.temporary_directory,
        batch,
        index,
    )?;
    let (checker, checker_raw) = check_observation(
        &paths.checker_path,
        &host.working_directory,
        &host.temporary_directory,
        &observation_raw,
        index,
        &batch.domain,
    )?;
    let checker_ids = checker_case_ids(&checker)?;
    let expected: Vec<String> = batch
        .cases
        .iter()
        .map(|case| case.case_id.clone())
        .collect();
    if checker_ids != expected {
        return Err(ProcessFailure::new(
            "CaseCorrespondenceMismatch",
            "runCorpus",
            "checker caseId values must match the request in number and order",
        ));
    }
    let event_digest = write_shard_event(
        events_directory,
        index,
        batch,
        &observation_raw,
        &checker_raw,
    )?;
    let observation_cases = observation
        .get("cases")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            ProcessFailure::new(
                "CanonicalDecodeFailure",
                "runCorpus",
                "observation cases must be an array",
            )
        })?;
    for case in observation_cases {
        let kind = observation_kind(case).ok_or_else(|| {
            ProcessFailure::new(
                "CanonicalDecodeFailure",
                "runCorpus",
                "observation kind is required",
            )
        })?;
        record_producer_kind(&mut coverage, &kind);
    }
    let checker_cases = checker
        .get("cases")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            ProcessFailure::new(
                "CanonicalDecodeFailure",
                "runCorpus",
                "checker cases must be an array",
            )
        })?;
    for case in checker_cases {
        let verdict = case.get("verdict").and_then(Value::as_str).ok_or_else(|| {
            ProcessFailure::new(
                "UnexpectedVerdict",
                "runCorpus",
                "checker case verdict is required",
            )
        })?;
        record_checker_verdict(&mut coverage, verdict);
    }
    let items = declarations.get(&batch.domain).ok_or_else(|| {
        ProcessFailure::new(
            "UnboundDescriptorReference",
            "runCorpus",
            format!("{} projectionDeclarations are unbound", batch.domain),
        )
    })?;
    let collected = collect_corpus_coverage(items, &batch.cases)?;
    for (key, count) in collected.buckets {
        *coverage.buckets.entry(key).or_default() += count;
    }
    let tuples = batch
        .cases
        .iter()
        .map(|case| {
            (
                batch.domain.clone(),
                case.global_case_index,
                sha256(&observation_raw),
            )
        })
        .collect();
    Ok(ShardOutput {
        event_digest,
        coverage,
        tuples,
        bytes: (batch.clone(), observation_raw, checker, checker_raw),
    })
}

fn run_corpus(
    manifest: &LogicalManifest,
    host: &HostExecutionConfig,
    descriptors: &BTreeMap<String, ProducerDescriptor>,
    shard_size: u64,
    events_directory: &Path,
) -> Result<CorpusRun, ProcessFailure> {
    enforce_shard_at_most_producer_max(shard_size, descriptors)?;
    let declarations = declarations_map(descriptors)?;
    let batches = shard_batches(manifest, &declarations, shard_size)?;
    fs::create_dir_all(events_directory).map_err(|error| {
        ProcessFailure::new(
            "ShardEventWriteFailure",
            "runCorpus",
            format!("{}: {error}", events_directory.display()),
        )
    })?;
    let next = AtomicUsize::new(0);
    let slots = Mutex::new(
        (0..batches.len())
            .map(|_| None)
            .collect::<Vec<Option<Result<ShardOutput, ProcessFailure>>>>(),
    );
    let workers = host.parallelism().min(batches.len()).max(1);
    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| loop {
                let index = next.fetch_add(1, Ordering::SeqCst);
                let Some(batch) = batches.get(index) else {
                    break;
                };
                let output = run_shard(
                    host,
                    descriptors,
                    &declarations,
                    events_directory,
                    index,
                    batch,
                );
                slots.lock().unwrap()[index] = Some(output);
            });
        }
    });
    let mut coverage = CoverageCounts::default();
    let mut observation_bytes = Vec::new();
    let mut shard_digests = Vec::new();
    let mut observation_tuple = Vec::new();
    for slot in slots.into_inner().unwrap() {
        let output = slot.unwrap()?;
        shard_digests.push(output.event_digest);
        for (key, count) in output.coverage.buckets {
            *coverage.buckets.entry(key).or_default() += count;
        }
        for (key, count) in output.coverage.producer_observation_kinds {
            *coverage.producer_observation_kinds.entry(key).or_default() += count;
        }
        for (key, count) in output.coverage.checker_verdicts {
            *coverage.checker_verdicts.entry(key).or_default() += count;
        }
        observation_tuple.extend(output.tuples);
        observation_bytes.push(output.bytes);
    }
    enforce_declared_bucket_coverage(manifest, &declarations, &coverage)?;
    enforce_verified_normal_corpus(&coverage)?;
    observation_tuple.sort_by(|left, right| left.0.cmp(&right.0).then(left.1.cmp(&right.1)));
    let observation_digest = sha256(
        &canonical_json(&Value::Array(
            observation_tuple
                .into_iter()
                .map(|(domain, index, digest)| {
                    object([
                        ("domain", Value::String(domain)),
                        (
                            "globalCaseIndex",
                            Value::String(crate::protocol::encode_u64_hex(index)),
                        ),
                        ("observationDigest", Value::String(digest)),
                    ])
                })
                .collect(),
        ))
        .map_err(|error| ProcessFailure::new("CanonicalDecodeFailure", "runCorpus", error))?,
    );
    Ok(CorpusRun {
        request_digest: request_digest(manifest, &declarations)?,
        observation_digest,
        shard_digests,
        coverage,
        observation_bytes,
    })
}

fn logical_report_value(
    manifest: &LogicalManifest,
    corpus: &CorpusRun,
    process_errors: &[ProcessFailure],
    lane_results: &Value,
    shared_normal_corpus: bool,
    sequential_staging: bool,
    real_passes: u64,
) -> Result<Value, ProcessFailure> {
    let mut verdicts = Map::new();
    for (verdict, count) in &corpus.coverage.checker_verdicts {
        verdicts.insert(verdict.clone(), Value::from(*count));
    }
    Ok(object([
        ("coverage", coverage_value(&corpus.coverage)),
        ("laneResults", lane_results.clone()),
        (
            "logicalManifestDigest",
            Value::String(manifest.digest.clone()),
        ),
        (
            "observationDigest",
            Value::String(corpus.observation_digest.clone()),
        ),
        (
            "processErrorCount",
            Value::from(process_errors.len() as u64),
        ),
        (
            "processErrors",
            Value::Array(
                process_errors
                    .iter()
                    .map(ProcessFailure::to_value)
                    .collect(),
            ),
        ),
        ("realPasses", Value::from(real_passes)),
        (
            "requestDigest",
            Value::String(corpus.request_digest.clone()),
        ),
        (
            "schema",
            Value::String("neco.verification.logical-report.v1".to_owned()),
        ),
        ("sequentialStaging", Value::Bool(sequential_staging)),
        ("sharedNormalCorpus", Value::Bool(shared_normal_corpus)),
        (
            "shardDigests",
            Value::Array(
                corpus
                    .shard_digests
                    .iter()
                    .map(|digest| Value::String(digest.clone()))
                    .collect(),
            ),
        ),
        ("verdictCounts", Value::Object(verdicts)),
    ]))
}

fn logical_report_digest(value: &Value) -> Result<String, ProcessFailure> {
    Ok(sha256(&canonical_json(value).map_err(|error| {
        ProcessFailure::new("CanonicalDecodeFailure", "logicalReportDigest", error)
    })?))
}

fn parse_lanes(csv: &str) -> Result<Vec<String>, ProcessFailure> {
    let lanes: Vec<String> = csv
        .split(',')
        .map(|lane| lane.trim().to_owned())
        .filter(|lane| !lane.is_empty())
        .collect();
    for lane in &lanes {
        if !SEVEN_LANES.contains(&lane.as_str()) {
            return Err(ProcessFailure::new(
                "ManifestValidationFailure",
                "parseLanes",
                format!("unknown lane {lane}"),
            ));
        }
    }
    Ok(lanes)
}

fn mutated_case_index(original: &[u8], mutated: &[u8]) -> Result<usize, ProcessFailure> {
    let cases_of = |bytes: &[u8]| -> Result<Vec<Value>, ProcessFailure> {
        decode_canonical_bytes(bytes, "mutationRejection")?
            .get("cases")
            .and_then(Value::as_array)
            .cloned()
            .ok_or_else(|| {
                ProcessFailure::new(
                    "CanonicalDecodeFailure",
                    "mutationRejection",
                    "observation cases must be an array",
                )
            })
    };
    let (before, after) = (cases_of(original)?, cases_of(mutated)?);
    before
        .iter()
        .zip(after.iter())
        .position(|(left, right)| left != right)
        .ok_or_else(|| {
            ProcessFailure::new(
                "ManifestValidationFailure",
                "mutationRejection",
                "mutation did not change any case",
            )
        })
}

fn checker_observed_classification(
    value: &Value,
    operation: &str,
    case_index: usize,
) -> Result<String, ProcessFailure> {
    if let Some(batch_verdict) = value.get("batchVerdict").and_then(Value::as_str) {
        if batch_verdict == "invalid" {
            return Ok("invalid".to_owned());
        }
        return Err(ProcessFailure::new(
            "UnexpectedVerdict",
            operation,
            format!("unsupported batchVerdict {batch_verdict}"),
        ));
    }
    let cases = value
        .get("cases")
        .and_then(Value::as_array)
        .ok_or_else(|| {
            ProcessFailure::new(
                "CanonicalDecodeFailure",
                operation,
                "checker result must have cases or batchVerdict",
            )
        })?;
    let verdict = cases
        .get(case_index)
        .and_then(|case| case.get("verdict"))
        .and_then(Value::as_str)
        .ok_or_else(|| {
            ProcessFailure::new(
                "UnexpectedVerdict",
                operation,
                "checker case verdict is required",
            )
        })?;
    Ok(verdict.to_owned())
}

fn run_mutation_lane(
    checker: &Path,
    cwd: &Path,
    temporary: &Path,
    domain: &str,
    observation: &[u8],
    expected: &str,
) -> Result<Value, ProcessFailure> {
    let mutated = mutate_canonical_observation(observation)?;
    let mutated_index = mutated_case_index(observation, &mutated)?;
    let before = sha256(observation);
    let after = sha256(&mutated);
    let checked = check_observation(
        checker,
        cwd,
        temporary,
        &mutated,
        9000,
        &format!("mutation-{domain}"),
    );
    let observed = match checked {
        Ok((value, _)) => {
            checker_observed_classification(&value, "mutationRejection", mutated_index)?
        }
        Err(failure) => {
            return Err(ProcessFailure::new(
                "SpawnFailure",
                "mutationRejection",
                "checker process error on mutated observation",
            )
            .with_contributing(vec![failure]));
        }
    };
    let report = classification_report("semantic-mutation", &before, &after, &observed)?;
    if observed != expected {
        return Err(ProcessFailure::new(
            "ManifestValidationFailure",
            "mutationRejection",
            "mutation lane did not observe the expected classification",
        )
        .with_witness(report));
    }
    Ok(report)
}

fn run_invalid_protocol_lane(
    checker: &Path,
    cwd: &Path,
    temporary: &Path,
    domain: &str,
    expected: &str,
) -> Result<Value, ProcessFailure> {
    let mut reports = Vec::new();
    for (kind, bytes) in invalid_protocol_vectors() {
        let path = temporary.join(format!("invalid-{domain}-{kind}.bin"));
        atomic_write(&path, &bytes)?;
        let spawned = spawn(
            checker,
            &["check", &path.to_string_lossy()],
            cwd,
            "invalidProtocol",
            None,
        );
        let observed = match spawned {
            Ok(stdout) => match stdout_value(&stdout, "invalidProtocol") {
                Ok((value, _)) => {
                    bind_checker_result(&value, &bytes, "invalidProtocol")?;
                    checker_observed_classification(&value, "invalidProtocol", 0)?
                }
                Err(failure) => {
                    return Err(ProcessFailure::new(
                        "CanonicalDecodeFailure",
                        "invalidProtocol",
                        "checker stdout is not a canonical result",
                    )
                    .with_contributing(vec![failure]));
                }
            },
            Err(failure) => {
                return Err(ProcessFailure::new(
                    "SpawnFailure",
                    "invalidProtocol",
                    "checker process error on invalid protocol input",
                )
                .with_contributing(vec![failure]));
            }
        };
        let report = classification_report(kind, &sha256(&bytes), &sha256(&bytes), &observed)?;
        if observed != expected && expected_classification(kind)? == ExpectedClassification::Invalid
        {
            return Err(ProcessFailure::new(
                "ManifestValidationFailure",
                "invalidProtocol",
                "invalid protocol lane did not observe invalid",
            )
            .with_witness(report.clone()));
        }
        reports.push(report);
    }
    Ok(Value::Array(reports))
}

pub fn command_generate(
    manifest_path: &Path,
    host_path: &Path,
    shard_size: u64,
    out: &Path,
) -> Result<Value, ProcessFailure> {
    let manifest = load_logical_manifest(manifest_path)?;
    let host =
        validate_host_execution_config(&load_host_execution_config(host_path)?, &manifest.domains)?;
    let declarations = declarations_map(&describe_all(&manifest, &host)?)?;
    let batches = shard_batches(&manifest, &declarations, shard_size)?;
    let digest = write_shards(&batches, out)?;
    Ok(object([
        ("out", Value::String(out.display().to_string())),
        ("requestDigest", Value::String(digest)),
        ("shardCount", Value::from(batches.len() as u64)),
    ]))
}

pub fn command_compare_request_digests(left: &Path, right: &Path) -> Result<Value, ProcessFailure> {
    let left_digest = digest_shard_directory(left)?;
    let right_digest = digest_shard_directory(right)?;
    if left_digest != right_digest {
        return Err(ProcessFailure::new(
            "ManifestValidationFailure",
            "compareRequestDigests",
            "request digests differ",
        )
        .with_witness(object([
            ("left", Value::String(left_digest.clone())),
            ("right", Value::String(right_digest.clone())),
        ])));
    }
    Ok(object([("requestDigest", Value::String(left_digest))]))
}

fn digest_shard_directory(directory: &Path) -> Result<String, ProcessFailure> {
    let mut files = Vec::new();
    let entries = fs::read_dir(directory).map_err(|error| {
        ProcessFailure::new(
            "CanonicalDecodeFailure",
            "compareRequestDigests",
            format!("{}: {error}", directory.display()),
        )
    })?;
    for entry in entries {
        let entry = entry.map_err(|error| {
            ProcessFailure::new(
                "CanonicalDecodeFailure",
                "compareRequestDigests",
                error.to_string(),
            )
        })?;
        let path = entry.path();
        if path.extension().and_then(|extension| extension.to_str()) == Some("json") {
            files.push(path);
        }
    }
    files.sort();
    let mut cases = Vec::new();
    for path in files {
        let bytes = fs::read(&path).map_err(|error| {
            ProcessFailure::new(
                "CanonicalDecodeFailure",
                "compareRequestDigests",
                format!("{}: {error}", path.display()),
            )
        })?;
        let batch = crate::protocol::decode_generated_request_batch(&bytes)?;
        cases.extend(batch.cases);
    }
    cases.sort_by_key(|case| case.global_case_index);
    let mut concatenated = Vec::new();
    for case in cases {
        concatenated.extend_from_slice(
            &canonical_json(&crate::protocol::generated_request_case_value(&case)).map_err(
                |error| {
                    ProcessFailure::new("CanonicalDecodeFailure", "compareRequestDigests", error)
                },
            )?,
        );
    }
    Ok(sha256(&concatenated))
}

pub fn command_verify_coverage(
    manifest_path: &Path,
    host_path: &Path,
    require_descriptor_buckets: bool,
    require_selector_rules: bool,
    require_shard_at_most_producer_max: bool,
) -> Result<Value, ProcessFailure> {
    let manifest = load_logical_manifest(manifest_path)?;
    let host =
        validate_host_execution_config(&load_host_execution_config(host_path)?, &manifest.domains)?;
    let descriptors = describe_all(&manifest, &host)?;
    if require_shard_at_most_producer_max {
        enforce_shard_at_most_producer_max(manifest.generator.shard_case_count, &descriptors)?;
    }
    let declarations = declarations_map(&descriptors)?;
    if require_selector_rules {
        for items in declarations.values() {
            validate_selector_rules(items)?;
        }
    }
    let cases = generate_all_cases(&manifest, &declarations)?;
    let mut coverage = CoverageCounts::default();
    for (domain, case) in &cases {
        if let Some(items) = declarations.get(domain) {
            let collected = collect_corpus_coverage(items, std::slice::from_ref(case))?;
            for (key, count) in collected.buckets {
                *coverage.buckets.entry(key).or_default() += count;
            }
        }
    }
    if require_descriptor_buckets {
        for domain in &manifest.domains {
            let items = declarations.get(&domain.domain).ok_or_else(|| {
                ProcessFailure::new(
                    "UnboundDescriptorReference",
                    "verifyCoverage",
                    format!("{} projectionDeclarations are unbound", domain.domain),
                )
            })?;
            if items.is_empty() {
                return Err(ProcessFailure::new(
                    "UnboundDescriptorReference",
                    "verifyCoverage",
                    format!("{} projectionDeclarations are empty", domain.domain),
                ));
            }
        }
    }
    Ok(coverage_value(&coverage))
}

pub fn command_test_infeasible_minimum(manifest_path: &Path) -> Result<Value, ProcessFailure> {
    let manifest = load_logical_manifest(manifest_path)?;
    let failure = infeasible_minimum_failure(&manifest)?;
    Ok(failure.to_value())
}

pub fn command_run_lanes(
    manifest_path: &Path,
    host_path: &Path,
    lanes_csv: &str,
    require_shared_normal_corpus: bool,
    require_structured_process_failures: bool,
) -> Result<Value, ProcessFailure> {
    let lanes = parse_lanes(lanes_csv)?;
    let manifest = load_logical_manifest(manifest_path)?;
    let host =
        validate_host_execution_config(&load_host_execution_config(host_path)?, &manifest.domains)?;
    fs::create_dir_all(&host.temporary_directory).map_err(|error| {
        ProcessFailure::new(
            "ShardEventWriteFailure",
            "runLanes",
            format!("{}: {error}", host.temporary_directory.display()),
        )
    })?;
    fs::create_dir_all(&host.report_directory).map_err(|error| {
        ProcessFailure::new(
            "ShardEventWriteFailure",
            "runLanes",
            format!("{}: {error}", host.report_directory.display()),
        )
    })?;
    let descriptors = describe_all(&manifest, &host)?;
    let events = host.temporary_directory.join("events");
    let shard_size = manifest.generator.shard_case_count;
    let corpus = run_corpus(&manifest, &host, &descriptors, shard_size, &events)?;
    let mut lane_map = Map::new();
    if require_shared_normal_corpus && !lanes.iter().any(|lane| lane == "observationVerification") {
        return Err(ProcessFailure::new(
            "ManifestValidationFailure",
            "runLanes",
            "observationVerification must share the normal corpus",
        ));
    }
    if lanes.iter().any(|lane| lane == "corpusExecution") {
        lane_map.insert(
            "corpusExecution".to_owned(),
            object([("status", Value::String("passed".to_owned()))]),
        );
    }
    if lanes.iter().any(|lane| lane == "observationVerification") {
        lane_map.insert(
            "observationVerification".to_owned(),
            object([
                ("sharedNormalCorpus", Value::Bool(true)),
                ("status", Value::String("passed".to_owned())),
            ]),
        );
    }
    if lanes.iter().any(|lane| lane == "soundnessTheorem") {
        let mut results = Map::new();
        for (domain, paths) in &host.domains {
            results.insert(
                domain.clone(),
                soundness(&paths.checker_path, &host.working_directory)?,
            );
        }
        lane_map.insert("soundnessTheorem".to_owned(), Value::Object(results));
    }
    if lanes.iter().any(|lane| lane == "publicApiParity") {
        let mut results = Map::new();
        for (index, (batch, observation, _, _)) in corpus.observation_bytes.iter().enumerate() {
            let paths = host.domains.get(&batch.domain).ok_or_else(|| {
                ProcessFailure::new(
                    "HostExecutionConfigInvalid",
                    "publicApiParity",
                    format!("{} is missing from HostExecutionConfig", batch.domain),
                )
            })?;
            let request_path = host
                .temporary_directory
                .join(format!("parity-request-{index:04}.json"));
            let observation_path = host
                .temporary_directory
                .join(format!("parity-observation-{index:04}.json"));
            atomic_write(&request_path, &encode_generated_request_batch(batch)?)?;
            atomic_write(&observation_path, observation)?;
            results.insert(
                format!("{}-{index}", batch.domain),
                self_check(
                    &paths.producer_path,
                    &host.working_directory,
                    &request_path,
                    &observation_path,
                )?,
            );
        }
        lane_map.insert("publicApiParity".to_owned(), Value::Object(results));
    }
    if lanes.iter().any(|lane| lane == "mutationRejection") {
        let mut results = Map::new();
        for domain in &manifest.domains {
            let (batch, observation, _, _) = corpus
                .observation_bytes
                .iter()
                .find(|(batch, _, _, _)| batch.domain == domain.domain)
                .ok_or_else(|| {
                    ProcessFailure::new(
                        "ManifestValidationFailure",
                        "mutationRejection",
                        format!(
                            "normal corpus produced no observation for {}",
                            domain.domain
                        ),
                    )
                })?;
            let paths = host.domains.get(&batch.domain).ok_or_else(|| {
                ProcessFailure::new(
                    "HostExecutionConfigInvalid",
                    "mutationRejection",
                    format!("{} is missing from HostExecutionConfig", batch.domain),
                )
            })?;
            results.insert(
                domain.domain.clone(),
                run_mutation_lane(
                    &paths.checker_path,
                    &host.working_directory,
                    &host.temporary_directory,
                    &domain.domain,
                    observation,
                    &manifest.coverage.mutation_expected_verdict,
                )?,
            );
        }
        lane_map.insert("mutationRejection".to_owned(), Value::Object(results));
    }
    if lanes.iter().any(|lane| lane == "invalidProtocol") {
        let mut results = Map::new();
        for domain in &manifest.domains {
            let paths = host.domains.get(&domain.domain).ok_or_else(|| {
                ProcessFailure::new(
                    "HostExecutionConfigInvalid",
                    "invalidProtocol",
                    format!("{} is missing from HostExecutionConfig", domain.domain),
                )
            })?;
            results.insert(
                domain.domain.clone(),
                run_invalid_protocol_lane(
                    &paths.checker_path,
                    &host.working_directory,
                    &host.temporary_directory,
                    &domain.domain,
                    &manifest.coverage.invalid_protocol_expected_verdict,
                )?,
            );
        }
        lane_map.insert("invalidProtocol".to_owned(), Value::Object(results));
    }
    if lanes.iter().any(|lane| lane == "repeatability") {
        let second = run_corpus(
            &manifest,
            &host,
            &descriptors,
            shard_size,
            &host.temporary_directory.join("events-repeat"),
        )?;
        if second.request_digest != corpus.request_digest
            || second.observation_digest != corpus.observation_digest
        {
            return Err(ProcessFailure::new(
                "ManifestValidationFailure",
                "repeatability",
                "repeatability digests differ",
            ));
        }
        lane_map.insert(
            "repeatability".to_owned(),
            object([
                (
                    "observationDigest",
                    Value::String(corpus.observation_digest.clone()),
                ),
                (
                    "requestDigest",
                    Value::String(corpus.request_digest.clone()),
                ),
                ("status", Value::String("passed".to_owned())),
            ]),
        );
    }
    let shared_normal_corpus = lanes.iter().any(|lane| lane == "observationVerification");
    let process_errors: Vec<ProcessFailure> = Vec::new();
    if require_structured_process_failures {
        for kind in corpus.coverage.process_errors.keys() {
            if matches!(
                kind.as_str(),
                "verified" | "refuted" | "undetermined" | "invalid"
            ) {
                return Err(ProcessFailure::new(
                    "ManifestValidationFailure",
                    "runLanes",
                    "process error kinds must not reuse checker verdict names",
                ));
            }
        }
        for failure in &process_errors {
            let value = failure.to_value();
            let object = value.as_object().ok_or_else(|| {
                ProcessFailure::new(
                    "ManifestValidationFailure",
                    "runLanes",
                    "process failure must be an object",
                )
            })?;
            for field in [
                "kind",
                "failingOperation",
                "operandProvenance",
                "violatedCondition",
                "witness",
                "contributingFailures",
            ] {
                if !object.contains_key(field) {
                    return Err(ProcessFailure::new(
                        "ManifestValidationFailure",
                        "runLanes",
                        format!("process failure missing {field}"),
                    ));
                }
            }
        }
    }
    let lane_results = Value::Object(lane_map);
    let logical = logical_report_value(
        &manifest,
        &corpus,
        &process_errors,
        &lane_results,
        shared_normal_corpus,
        true,
        1,
    )?;
    let digest = logical_report_digest(&logical)?;
    let mut report = logical;
    if let Some(fields) = report.as_object_mut() {
        fields.insert(
            "logicalReportDigest".to_owned(),
            Value::String(digest.clone()),
        );
        fields.insert(
            "hostExecutionProvenance".to_owned(),
            object([
                (
                    "reportDirectory",
                    Value::String(host.report_directory.display().to_string()),
                ),
                (
                    "workingDirectory",
                    Value::String(host.working_directory.display().to_string()),
                ),
            ]),
        );
    }
    let encoded = canonical_json(&report)
        .map_err(|error| ProcessFailure::new("CanonicalDecodeFailure", "runLanes", error))?;
    atomic_write(&host.report_directory.join("lanes.json"), &encoded)?;
    Ok(report)
}

pub fn command_compare_repeatability(
    manifest_path: &Path,
    host_path: &Path,
    runs: u64,
    sequential: bool,
    shard_size: u64,
    report_path: &Path,
) -> Result<Value, ProcessFailure> {
    if runs != 2 {
        return Err(ProcessFailure::new(
            "ManifestValidationFailure",
            "compareRepeatability",
            "compare-repeatability requires two runs",
        ));
    }
    let manifest = load_logical_manifest(manifest_path)?;
    let host =
        validate_host_execution_config(&load_host_execution_config(host_path)?, &manifest.domains)?;
    fs::create_dir_all(&host.temporary_directory).map_err(|error| {
        ProcessFailure::new(
            "ShardEventWriteFailure",
            "compareRepeatability",
            format!("{}: {error}", host.temporary_directory.display()),
        )
    })?;
    let descriptors = describe_all(&manifest, &host)?;
    enforce_shard_at_most_producer_max(shard_size, &descriptors)?;
    let first = run_corpus(
        &manifest,
        &host,
        &descriptors,
        shard_size,
        &host.temporary_directory.join("events-a"),
    )?;
    let second = run_corpus(
        &manifest,
        &host,
        &descriptors,
        shard_size,
        &host.temporary_directory.join("events-b"),
    )?;
    if first.request_digest != second.request_digest
        || first.observation_digest != second.observation_digest
    {
        return Err(ProcessFailure::new(
            "ManifestValidationFailure",
            "compareRepeatability",
            "repeatability digest mismatch",
        ));
    }
    let lane_results = object([(
        "repeatability",
        object([
            (
                "observationDigest",
                Value::String(first.observation_digest.clone()),
            ),
            ("requestDigest", Value::String(first.request_digest.clone())),
            ("status", Value::String("passed".to_owned())),
        ]),
    )]);
    let logical = logical_report_value(&manifest, &first, &[], &lane_results, true, sequential, 2)?;
    let digest = logical_report_digest(&logical)?;
    let second_logical =
        logical_report_value(&manifest, &second, &[], &lane_results, true, sequential, 2)?;
    let second_digest = logical_report_digest(&second_logical)?;
    if digest != second_digest {
        return Err(ProcessFailure::new(
            "ManifestValidationFailure",
            "compareRepeatability",
            "logical report digest mismatch",
        ));
    }
    let mut report = logical;
    if let Some(fields) = report.as_object_mut() {
        fields.insert(
            "logicalReportDigest".to_owned(),
            Value::String(digest.clone()),
        );
        fields.insert(
            "hostExecutionProvenance".to_owned(),
            object([
                (
                    "reportDirectory",
                    Value::String(host.report_directory.display().to_string()),
                ),
                ("sequential", Value::Bool(sequential)),
            ]),
        );
    }
    let encoded = canonical_json(&report).map_err(|error| {
        ProcessFailure::new("CanonicalDecodeFailure", "compareRepeatability", error)
    })?;
    atomic_write(report_path, &encoded)?;
    Ok(report)
}

pub fn command_validate_resource_reports(
    require_sequential_staging: bool,
    require_two_real_passes: bool,
    require_shard_at_most_producer_max: bool,
    reports: &[PathBuf],
) -> Result<Value, ProcessFailure> {
    let mut values = Vec::new();
    for path in reports {
        let (value, _) = decode_canonical_file(path, "validateResourceReports")?;
        if require_sequential_staging && value.get("sequentialStaging") != Some(&Value::Bool(true))
        {
            return Err(ProcessFailure::new(
                "ManifestValidationFailure",
                "validateResourceReports",
                "report must record sequential staging",
            ));
        }
        if require_two_real_passes && value.get("realPasses") != Some(&Value::from(2)) {
            return Err(ProcessFailure::new(
                "ManifestValidationFailure",
                "validateResourceReports",
                "report must record two real passes",
            ));
        }
        if require_shard_at_most_producer_max
            && value
                .get("shardDigests")
                .and_then(Value::as_array)
                .is_none()
        {
            return Err(ProcessFailure::new(
                "ManifestValidationFailure",
                "validateResourceReports",
                "report must record shard digests",
            ));
        }
        values.push(value);
    }
    Ok(object([("reports", Value::from(reports.len() as u64))]))
}

pub fn start_children_from_validated_inputs(
    manifest: &LogicalManifest,
    host: &HostExecutionConfig,
    descriptors: &BTreeMap<String, ProducerDescriptor>,
) -> Result<Value, ProcessFailure> {
    for descriptor in descriptors.values() {
        validate_runtime_host_compatibility(&descriptor.target_triple)?;
    }
    let declarations = declarations_map(descriptors)?;
    let batches = shard_batches(manifest, &declarations, 1)?;
    let batch = batches.first().ok_or_else(|| {
        ProcessFailure::new(
            "ManifestValidationFailure",
            "startChildren",
            "corpus has no shard",
        )
    })?;
    let paths = host.domains.get(&batch.domain).ok_or_else(|| {
        ProcessFailure::new(
            "HostExecutionConfigInvalid",
            "startChildren",
            format!("{} is missing from HostExecutionConfig", batch.domain),
        )
    })?;
    let (observation, observation_raw) = generate_observation(
        &paths.producer_path,
        &host.working_directory,
        &host.temporary_directory,
        batch,
        0,
    )?;
    let (checker, _) = check_observation(
        &paths.checker_path,
        &host.working_directory,
        &host.temporary_directory,
        &observation_raw,
        0,
        &batch.domain,
    )?;
    Ok(object([
        ("checker", checker),
        ("observation", observation),
        ("started", Value::Bool(true)),
    ]))
}

pub fn stub_main(args: Vec<String>, executable: &str) -> Result<Vec<u8>, ProcessFailure> {
    let domain = stub_domain(executable);
    match args.as_slice() {
        [command] if command == "describe" => stub_describe(&domain),
        [command, flag, request] if command == "generate" && flag == "--request" => {
            stub_generate(&domain, request)
        }
        [command, request_flag, request, batch_flag, batch]
            if command == "self-check"
                && request_flag == "--request"
                && batch_flag == "--batch" =>
        {
            stub_self_check(&domain, request, batch)
        }
        [command] if command == "all" || command == "soundness" => {
            Ok(b"{\"status\":\"passed\"}".to_vec())
        }
        [command, path] if command == "check" => stub_check(path),
        _ => Err(ProcessFailure::new(
            "SpawnFailure",
            "stubMain",
            "unsupported stub command",
        )),
    }
}

fn stub_domain(executable: &str) -> String {
    let executable = std::path::Path::new(executable)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(executable);
    if executable.contains("neco.algebra") || executable.contains("algebra") {
        "neco.algebra".to_owned()
    } else if executable.contains("neco.calculus") || executable.contains("calculus") {
        "neco.calculus".to_owned()
    } else {
        "neco.geometry".to_owned()
    }
}

fn stub_kinds(domain: &str) -> Vec<Value> {
    match domain {
        "neco.algebra" => vec![
            Value::String("success".to_owned()),
            Value::String("zero-denominator".to_owned()),
        ],
        "neco.calculus" => vec![
            Value::String("success".to_owned()),
            Value::String("empty-coefficients".to_owned()),
        ],
        _ => vec![
            Value::String("success".to_owned()),
            Value::String("failure".to_owned()),
        ],
    }
}

fn stub_describe(domain: &str) -> Result<Vec<u8>, ProcessFailure> {
    let kinds = stub_kinds(domain);
    let family = match domain {
        "neco.algebra" => "rational-reduction",
        "neco.calculus" => "polynomial-exact",
        _ => "region-polynomial-normalization",
    };
    let declarations = vec![object([
        (
            "applicableCoverageAxes",
            Value::Array(vec![Value::String("value".to_owned())]),
        ),
        ("family", Value::String(family.to_owned())),
        ("requestedObservationKinds", Value::Array(kinds)),
        (
            "selectorBindings",
            Value::Array(vec![object([
                ("axis", Value::String("value".to_owned())),
                (
                    "buckets",
                    Value::Array(vec![
                        object([
                            ("name", Value::String("even".to_owned())),
                            (
                                "projectedCondition",
                                Value::String("remainder 0".to_owned()),
                            ),
                        ]),
                        object([
                            ("name", Value::String("odd".to_owned())),
                            (
                                "projectedCondition",
                                Value::String("remainder 1".to_owned()),
                            ),
                        ]),
                    ]),
                ),
                ("selector", Value::String("value".to_owned())),
                (
                    "selectorRule",
                    object([
                        (
                            "bucketsByRemainder",
                            object([
                                ("0", Value::String("even".to_owned())),
                                ("1", Value::String("odd".to_owned())),
                            ]),
                        ),
                        ("modulus", Value::from(2)),
                        ("selector", Value::String("value".to_owned())),
                    ]),
                ),
            ])]),
        ),
    ])];
    let (adapter, producer_api, semantics_revision, observation_schema, resource_revision) =
        match domain {
            "neco.algebra" => (
                "algebra-generated-request-v1",
                "1",
                "fixture-algebra-semantics",
                "neco.algebra.observation-batch.v1",
                "algebra-resource-v1",
            ),
            "neco.calculus" => (
                "calculus-adapter-v3",
                "calculus-producer-api-v3",
                "fixture-calculus-semantics",
                "neco.calculus.observation-batch.v1",
                "calculus-resource-v1",
            ),
            _ => (
                "geometry-generated-request-v1",
                "1",
                "fixture-geometry-semantics",
                "neco.geometry.observation-batch.v1",
                "geometry-resource-v1",
            ),
        };
    let _ = observation_schema;
    let value = object([
        ("adapterRevision", Value::String(adapter.to_owned())),
        ("domain", Value::String(domain.to_owned())),
        (
            "producerApiRevision",
            Value::String(producer_api.to_owned()),
        ),
        (
            "projectionClaim",
            object([
                ("projectionDeclarationDigest", Value::String("0".repeat(64))),
                ("projectionDeclarations", Value::Array(declarations)),
                ("projectionRevision", Value::String("0".repeat(64))),
            ]),
        ),
        (
            "resourceClaim",
            object([
                ("maxBatchCases", Value::from(2000)),
                ("resourceDeclarationDigest", Value::String("1".repeat(64))),
                (
                    "resourceRevision",
                    Value::String(resource_revision.to_owned()),
                ),
            ]),
        ),
        ("rustcIdentity", Value::String("rustc stub".to_owned())),
        (
            "schema",
            Value::String("neco.verification.producer-description.v1".to_owned()),
        ),
        (
            "semanticsRef",
            object([
                ("declarationDigest", Value::String("2".repeat(64))),
                ("revision", Value::String(semantics_revision.to_owned())),
            ]),
        ),
        (
            "targetTriple",
            Value::String(crate::protocol::current_host_target_triple()),
        ),
    ]);
    canonical_json(&value)
        .map_err(|error| ProcessFailure::new("CanonicalDecodeFailure", "stubDescribe", error))
}

fn stub_generate(domain: &str, request_path: &str) -> Result<Vec<u8>, ProcessFailure> {
    if request_path == "-" {
        return Err(ProcessFailure::new(
            "SpawnFailure",
            "stubGenerate",
            "stdin request is unsupported in the stub file path",
        ));
    }
    let bytes = fs::read(request_path).map_err(|error| {
        ProcessFailure::new(
            "SpawnFailure",
            "stubGenerate",
            format!("{request_path}: {error}"),
        )
    })?;
    let batch = crate::protocol::decode_generated_request_batch(&bytes)?;
    let schema = match domain {
        "neco.algebra" => "neco.algebra.observation-batch.v1",
        "neco.calculus" => "neco.calculus.observation-batch.v1",
        _ => "neco.geometry.observation-batch.v1",
    };
    let cases: Vec<Value> = batch
        .cases
        .iter()
        .map(|case| {
            let ok = case.requested_observation_kind == "success";
            object([
                ("caseId", Value::String(case.case_id.clone())),
                ("input", object([])),
                (
                    "observation",
                    object([
                        (
                            "kind",
                            Value::String(if ok {
                                "success".to_owned()
                            } else {
                                case.requested_observation_kind.clone()
                            }),
                        ),
                        ("ok", Value::Bool(ok)),
                        ("value", object([("n", Value::String("0".to_owned()))])),
                    ]),
                ),
                ("operation", Value::String(case.family.clone())),
            ])
        })
        .collect();
    canonical_json(&object([
        ("cases", Value::Array(cases)),
        ("schema", Value::String(schema.to_owned())),
    ]))
    .map_err(|error| ProcessFailure::new("CanonicalDecodeFailure", "stubGenerate", error))
}

fn stub_self_check(domain: &str, request: &str, batch: &str) -> Result<Vec<u8>, ProcessFailure> {
    let generated = stub_generate(domain, request)?;
    let supplied = fs::read(batch).map_err(|error| {
        ProcessFailure::new("SpawnFailure", "stubSelfCheck", format!("{batch}: {error}"))
    })?;
    let supplied_text = std::str::from_utf8(&supplied).map_err(|error| {
        ProcessFailure::new("CanonicalDecodeFailure", "stubSelfCheck", error.to_string())
    })?;
    let supplied_bytes = supplied_text
        .strip_suffix('\n')
        .unwrap_or(supplied_text)
        .as_bytes();
    if generated != supplied_bytes {
        return Err(ProcessFailure::new(
            "CaseCorrespondenceMismatch",
            "stubSelfCheck",
            "supplied observation bytes differ from replay",
        ));
    }
    canonical_json(&object([
        (
            "schema",
            Value::String("neco.verification.public-api-parity-result.v1".to_owned()),
        ),
        ("verified", Value::Bool(true)),
    ]))
    .map_err(|error| ProcessFailure::new("CanonicalDecodeFailure", "stubSelfCheck", error))
}

fn stub_check(path: &str) -> Result<Vec<u8>, ProcessFailure> {
    let bytes = fs::read(path).map_err(|error| {
        ProcessFailure::new("SpawnFailure", "stubCheck", format!("{path}: {error}"))
    })?;
    let digest = sha256(&bytes);
    match decode_canonical_bytes(&bytes, "stubCheck") {
        Ok(value)
            if value
                .get("schema")
                .and_then(Value::as_str)
                .is_some_and(|schema| schema.ends_with(".observation-batch.v1")) =>
        {
            let cases = value
                .get("cases")
                .and_then(Value::as_array)
                .ok_or_else(|| {
                    ProcessFailure::new(
                        "CanonicalDecodeFailure",
                        "stubCheck",
                        "cases must be an array",
                    )
                })?;
            let mut results = Vec::new();
            for case in cases {
                let case_id = case.get("caseId").and_then(Value::as_str).ok_or_else(|| {
                    ProcessFailure::new("CanonicalDecodeFailure", "stubCheck", "caseId is required")
                })?;
                let operation = case
                    .get("operation")
                    .and_then(Value::as_str)
                    .ok_or_else(|| {
                        ProcessFailure::new(
                            "CanonicalDecodeFailure",
                            "stubCheck",
                            "operation is required",
                        )
                    })?;
                if operation == "unbound-family" {
                    results.push(object([
                        ("caseId", Value::String(case_id.to_owned())),
                        ("verdict", Value::String("invalid".to_owned())),
                    ]));
                    continue;
                }
                let observation = case
                    .get("observation")
                    .and_then(Value::as_object)
                    .ok_or_else(|| {
                        ProcessFailure::new(
                            "CanonicalDecodeFailure",
                            "stubCheck",
                            "observation is required",
                        )
                    })?;
                let ok = observation
                    .get("ok")
                    .and_then(Value::as_bool)
                    .ok_or_else(|| {
                        ProcessFailure::new(
                            "CanonicalDecodeFailure",
                            "stubCheck",
                            "observation.ok is required",
                        )
                    })?;
                let kind = observation
                    .get("kind")
                    .and_then(Value::as_str)
                    .ok_or_else(|| {
                        ProcessFailure::new(
                            "CanonicalDecodeFailure",
                            "stubCheck",
                            "observation.kind is required",
                        )
                    })?;
                let value_intact = observation
                    .get("value")
                    .is_none_or(|value| value == &object([("n", Value::String("0".to_owned()))]));
                let verdict = if kind == "success" && ok && !value_intact {
                    "refuted"
                } else if kind == "success" && ok {
                    "verified"
                } else if kind == "success" || ok {
                    "refuted"
                } else {
                    "verified"
                };
                results.push(object([
                    ("caseId", Value::String(case_id.to_owned())),
                    ("verdict", Value::String(verdict.to_owned())),
                ]));
            }
            canonical_json(&object([
                ("cases", Value::Array(results)),
                ("digest", Value::String(digest)),
            ]))
            .map_err(|error| ProcessFailure::new("CanonicalDecodeFailure", "stubCheck", error))
        }
        _ => canonical_json(&object([
            ("batchVerdict", Value::String("invalid".to_owned())),
            ("digest", Value::String(digest)),
        ]))
        .map_err(|error| ProcessFailure::new("CanonicalDecodeFailure", "stubCheck", error)),
    }
}
