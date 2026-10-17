use neco_geometry_observer::canonical_json;
use neco_geometry_observer::corpus_generator::{
    declared_buckets, domain_plans, enforce_shard_at_most_producer_max, generate_all_cases,
    request_digest, shard_batches, splitmix64_draw,
};
use neco_geometry_observer::coverage::{
    collect_corpus_coverage, ignore_nonapplicable_selectors, validate_selector_rules,
};
use neco_geometry_observer::mutation::{
    classification_name, expected_classification, invalid_protocol_vectors, mutate_canonical_observation,
    ExpectedClassification,
};
use neco_geometry_observer::protocol::{
    decode_logical_manifest, decode_projection_declarations, encode_u64_hex, object,
    validate_host_execution_config, ProducerDescriptor, SELECTOR_NAMES,
};
use neco_geometry_observer::stack_runner::{
    command_compare_repeatability, command_generate, command_test_infeasible_minimum,
    command_validate_resource_reports, describe_all, load_host_execution_config,
    load_logical_manifest,
};
use neco_geometry_observer::sha256;
use serde_json::{Map, Value};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

fn observer_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn smoke_path() -> PathBuf {
    observer_dir().join("manifests/smoke.json")
}

fn stub_executable() -> PathBuf {
    static STUB: OnceLock<PathBuf> = OnceLock::new();
    STUB.get_or_init(build_stub).clone()
}

fn build_stub() -> PathBuf {
    let crate_dir = observer_dir().join("target/verification-stub-crate");
    fs::create_dir_all(crate_dir.join("src")).expect("stub crate directory");
    fs::write(
        crate_dir.join("Cargo.toml"),
        format!(
            "[package]\nname = \"child-stub\"\nversion = \"0.1.0\"\nedition = \"2021\"\n[workspace]\n[dependencies]\nneco-geometry-observer = {{ path = \"{}\" }}\n",
            observer_dir().display()
        ),
    )
    .expect("stub Cargo.toml");
    fs::write(
        crate_dir.join("src/main.rs"),
        r#"fn main() {
    let mut arguments: Vec<String> = std::env::args().collect();
    let executable = arguments.remove(0);
    match neco_geometry_observer::stack_runner::stub_main(arguments, &executable) {
        Ok(bytes) => {
            use std::io::Write;
            let _ = std::io::stdout().lock().write_all(&bytes);
        }
        Err(failure) => {
            use std::io::Write;
            let rendered = failure
                .render()
                .unwrap_or_else(|_| failure.violated_condition().to_owned());
            let _ = std::io::stderr().lock().write_all(rendered.as_bytes());
            std::process::exit(1);
        }
    }
}
"#,
    )
    .expect("stub main");
    let status = Command::new(env!("CARGO"))
        .args([
            "build",
            "--manifest-path",
            crate_dir.join("Cargo.toml").to_str().expect("utf8"),
            "--offline",
        ])
        .env(
            "CARGO_TARGET_DIR",
            observer_dir().join("target/verification-stub-target"),
        )
        .status()
        .expect("stub cargo build");
    assert!(status.success(), "stub cargo build");
    observer_dir()
        .join("target/verification-stub-target/debug")
        .join(format!("child-stub{}", std::env::consts::EXE_SUFFIX))
}

fn fixture_manifest_bytes() -> Vec<u8> {
    let bytes = fs::read(smoke_path()).expect("smoke");
    let mut value: Value = serde_json::from_slice(&bytes).unwrap();
    let domains = value["domains"].as_array_mut().unwrap();
    for domain in domains.iter_mut() {
        let domain_object = domain.as_object_mut().unwrap();
        let name = domain_object["domain"].as_str().unwrap().to_owned();
        let family = match name.as_str() {
            "neco.algebra" => "rational-reduction",
            "neco.calculus" => "polynomial-exact",
            _ => "region-polynomial-normalization",
        };
        let kinds = match name.as_str() {
            "neco.algebra" => vec!["success", "zero-denominator"],
            "neco.calculus" => vec!["success", "empty-coefficients"],
            _ => vec!["success", "failure"],
        };
        domain_object.insert(
            "families".to_owned(),
            Value::Array(vec![object([
                ("family", Value::String(family.to_owned())),
                ("minimum", Value::from(1)),
                (
                    "requestedObservationKinds",
                    Value::Array(kinds.into_iter().map(|kind| Value::String(kind.to_owned())).collect()),
                ),
                ("weight", Value::from(1)),
            ])]),
        );
        domain_object
            .get_mut("semanticsRef")
            .unwrap()
            .as_object_mut()
            .unwrap()
            .insert(
                "declarationDigest".to_owned(),
                Value::String("2".repeat(64)),
            );
    }
    canonical_json(&value).unwrap()
}

fn stage_host(root: &Path) -> PathBuf {
    let stub = stub_executable();
    let mut domains = Map::new();
    for domain in ["neco.algebra", "neco.calculus", "neco.geometry"] {
        let producer = root.join(format!("{domain}-producer"));
        let checker = root.join(format!("{domain}-checker"));
        fs::copy(&stub, &producer).unwrap();
        fs::copy(&stub, &checker).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut permissions = fs::metadata(&producer).unwrap().permissions();
            permissions.set_mode(0o755);
            fs::set_permissions(&producer, permissions.clone()).unwrap();
            fs::set_permissions(&checker, permissions).unwrap();
        }
        domains.insert(
            domain.to_owned(),
            object([
                ("checkerPath", Value::String(checker.display().to_string())),
                ("producerPath", Value::String(producer.display().to_string())),
            ]),
        );
    }
    fs::create_dir_all(root.join("reports")).unwrap();
    fs::create_dir_all(root.join("temporary")).unwrap();
    let config = object([
        ("domains", Value::Object(domains)),
        (
            "reportDirectory",
            Value::String(root.join("reports").display().to_string()),
        ),
        (
            "temporaryDirectory",
            Value::String(root.join("temporary").display().to_string()),
        ),
        (
            "workingDirectory",
            Value::String(root.display().to_string()),
        ),
    ]);
    let path = root.join("host-execution.json");
    fs::write(&path, canonical_json(&config).unwrap()).unwrap();
    path
}

#[test]
fn descriptor_selector_buckets_ignore_nonapplicable_selectors_and_enforce_max_batch() {
    let declaration = object([
        (
            "applicableCoverageAxes",
            Value::Array(vec![Value::String("value".to_owned())]),
        ),
        ("family", Value::String("rational-reduction".to_owned())),
        (
            "requestedObservationKinds",
            Value::Array(vec![Value::String("success".to_owned())]),
        ),
        (
            "selectorBindings",
            Value::Array(vec![object([
                ("axis", Value::String("value".to_owned())),
                (
                    "buckets",
                    Value::Array(vec![
                        object([
                            ("name", Value::String("even".to_owned())),
                            ("projectedCondition", Value::String("remainder 0".to_owned())),
                        ]),
                        object([
                            ("name", Value::String("odd".to_owned())),
                            ("projectedCondition", Value::String("remainder 1".to_owned())),
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
    ]);
    let decoded = decode_projection_declarations(&[declaration]).unwrap();
    validate_selector_rules(&decoded).unwrap();
    let ignored = ignore_nonapplicable_selectors(&decoded[0]);
    assert!(ignored.contains(&"sign".to_owned()));
    assert!(ignored.contains(&"formulaVariant".to_owned()));
    let manifest = decode_logical_manifest(&fs::read(smoke_path()).unwrap()).unwrap();
    let mut declarations = BTreeMap::new();
    let cases = generate_all_cases(&manifest, &declarations).unwrap();
    let algebra: Vec<_> = cases
        .iter()
        .filter(|(domain, case)| domain == "neco.algebra" && case.family == "rational-reduction")
        .map(|(_, case)| case.clone())
        .collect();
    declarations.insert("neco.algebra".to_owned(), decoded.clone());
    let coverage = collect_corpus_coverage(&decoded, &algebra).unwrap();
    for ((family, axis, _), _) in coverage.buckets {
        assert_eq!(family, "rational-reduction");
        assert_eq!(axis, "value");
    }
    let mut descriptor = ProducerDescriptor {
        domain: "neco.algebra".to_owned(),
        producer_api_revision: "1".to_owned(),
        adapter_revision: "algebra-generated-request-v1".to_owned(),
        target_triple: "aarch64-apple-darwin".to_owned(),
        rustc_identity: "rustc stub".to_owned(),
        semantics_revision: "fixture-algebra-semantics".to_owned(),
        semantics_declaration_digest: "2".repeat(64),
        projection_revision: "0".repeat(64),
        projection_declarations: vec![],
        resource_revision: "algebra-resource-v1".to_owned(),
        resource_declaration_digest: "1".repeat(64),
        max_batch_cases: 8,
        bytes: Vec::new(),
        digest: String::new(),
    };
    let mut descriptors = BTreeMap::new();
    descriptors.insert("neco.algebra".to_owned(), descriptor.clone());
    let exceeded = enforce_shard_at_most_producer_max(9, &descriptors).expect_err("max batch");
    assert_eq!(exceeded.kind(), "ShardExceedsProducerMaxBatchCases");
    descriptor.max_batch_cases = 2000;
    descriptors.insert("neco.algebra".to_owned(), descriptor);
    enforce_shard_at_most_producer_max(2000, &descriptors).unwrap();
    let _ = SELECTOR_NAMES;
}

#[test]
fn mutation_and_invalid_protocol_have_distinct_expected_classifications() {
    let observation = object([
        (
            "cases",
            Value::Array(vec![object([
                ("caseId", Value::String("case-0000000000000000".to_owned())),
                ("input", object([])),
                (
                    "observation",
                    object([
                        ("kind", Value::String("success".to_owned())),
                        (
                            "value",
                            object([("gcd", Value::String("2".to_owned()))]),
                        ),
                    ]),
                ),
                ("operation", Value::String("rational-reduction".to_owned())),
            ])]),
        ),
        ("schema", Value::String("neco.verification.observation-batch.v1".to_owned())),
    ]);
    let bytes = canonical_json(&observation).unwrap();
    let mutated = mutate_canonical_observation(&bytes).unwrap();
    assert_ne!(sha256(&bytes), sha256(&mutated));
    assert_eq!(
        expected_classification("semantic-mutation").unwrap(),
        ExpectedClassification::Refuted
    );
    assert_eq!(classification_name(ExpectedClassification::Refuted), "refuted");
    for (kind, _) in invalid_protocol_vectors() {
        assert_eq!(
            expected_classification(kind).unwrap(),
            ExpectedClassification::Invalid
        );
    }
    assert_eq!(
        expected_classification("logical-manifest").unwrap(),
        ExpectedClassification::ProcessError
    );
    assert_eq!(
        expected_classification("descriptor").unwrap(),
        ExpectedClassification::ProcessError
    );
    assert_eq!(
        expected_classification("host-execution-config").unwrap(),
        ExpectedClassification::ProcessError
    );
    assert_eq!(
        expected_classification("executable-digest").unwrap(),
        ExpectedClassification::ProcessError
    );
}

#[test]
fn generate_request_digest_is_independent_of_shard_size() {
    let root = observer_dir().join("target/verification-generate");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    let manifest = root.join("logical-manifest.json");
    fs::write(&manifest, fixture_manifest_bytes()).unwrap();
    let host = stage_host(&root);
    let one = root.join("one");
    let two = root.join("two");
    command_generate(&manifest, &host, 2000, &one).unwrap();
    command_generate(&manifest, &host, 317, &two).unwrap();
    let left = neco_geometry_observer::stack_runner::command_compare_request_digests(&one, &two).unwrap();
    assert!(left.get("requestDigest").and_then(Value::as_str).is_some());
    let decoded = decode_logical_manifest(&fs::read(&manifest).unwrap()).unwrap();
    let host_config = validate_host_execution_config(
        &load_host_execution_config(&host).unwrap(),
        &decoded.domains,
    )
    .unwrap();
    let mut declarations = BTreeMap::new();
    for (domain, descriptor) in describe_all(&decoded, &host_config).unwrap() {
        declarations.insert(
            domain,
            decode_projection_declarations(&descriptor.projection_declarations).unwrap(),
        );
    }
    let digest = request_digest(&decoded, &declarations).unwrap();
    assert_eq!(left["requestDigest"], digest);
    let seed = decoded.generator.seed_value;
    assert_eq!(
        splitmix64_draw(seed, 0, 0),
        splitmix64_draw(seed, 0, 0)
    );
    assert_ne!(
        splitmix64_draw(seed, 0, 0),
        splitmix64_draw(seed ^ 1, 0, 0)
    );
}

#[test]
fn test_infeasible_minimum_reports_process_failure() {
    let value = command_test_infeasible_minimum(&smoke_path()).unwrap();
    assert_eq!(value["kind"], "InfeasibleMinimum");
}

#[test]
fn compare_repeatability_runs_smoke_fixture_twice() {
    let root = observer_dir().join("target/verification-repeatability");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    let manifest_path = root.join("logical-manifest.json");
    fs::write(&manifest_path, fixture_manifest_bytes()).unwrap();
    let host = stage_host(&root);
    let report = root.join("reports/repeatability.json");
    let value = command_compare_repeatability(&manifest_path, &host, 2, true, 64, &report).unwrap();
    assert_eq!(value["realPasses"], 2);
    assert_eq!(value["sequentialStaging"], true);
    command_validate_resource_reports(true, true, true, &[report]).unwrap();
}

#[test]
fn case_id_uses_global_index_hex() {
    let manifest = decode_logical_manifest(&fs::read(smoke_path()).unwrap()).unwrap();
    let batches = shard_batches(&manifest, &BTreeMap::new(), 2000).unwrap();
    let first = &batches[0].cases[0];
    assert_eq!(first.case_id, format!("case-{}", encode_u64_hex(0)));
    assert_eq!(first.selectors.len(), 9);
}

#[test]
fn derived_corpus_is_the_smallest_selector_sequence_covering_every_declared_bucket() {
    let root = observer_dir().join("target/verification-derived-corpus");
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();
    let manifest_path = root.join("logical-manifest.json");
    fs::write(&manifest_path, fixture_manifest_bytes()).unwrap();
    let host_path = stage_host(&root);
    let manifest = load_logical_manifest(&manifest_path).unwrap();
    assert!(manifest.generator.explicit_allocation.is_none());
    let host = validate_host_execution_config(
        &load_host_execution_config(&host_path).unwrap(),
        &manifest.domains,
    )
    .unwrap();
    let descriptors = describe_all(&manifest, &host).unwrap();
    let mut declarations = BTreeMap::new();
    for (domain, descriptor) in &descriptors {
        declarations.insert(
            domain.clone(),
            decode_projection_declarations(&descriptor.projection_declarations).unwrap(),
        );
    }
    let plans = domain_plans(&manifest, &declarations).unwrap();
    let cases = generate_all_cases(&manifest, &declarations).unwrap();
    assert_eq!(
        cases.len() as u64,
        plans.iter().map(|plan| plan.case_count).sum::<u64>()
    );
    for plan in &plans {
        let items = &declarations[&plan.domain];
        for entry in &plan.families {
            let family_cases: Vec<_> = cases
                .iter()
                .filter(|(domain, case)| *domain == plan.domain && case.family == entry.family)
                .map(|(_, case)| case.clone())
                .collect();
            assert_eq!(family_cases.len() as u64, plan.family_counts[&entry.family]);
            let declaration = items.iter().find(|item| item.family == entry.family).unwrap();
            let covered = |selected: &[neco_geometry_observer::protocol::GeneratedRequestCase]| {
                let counts = collect_corpus_coverage(items, selected).unwrap();
                let buckets_ok = declared_buckets(declaration).into_iter().all(|((family, axis), buckets)| {
                    buckets.into_iter().all(|bucket| {
                        counts.buckets.get(&(family.clone(), axis.clone(), bucket)).copied().unwrap_or(0) > 0
                    })
                });
                let kinds_ok = entry.requested_observation_kinds.iter().all(|kind| {
                    selected.iter().any(|case| &case.requested_observation_kind == kind)
                });
                buckets_ok && kinds_ok
            };
            assert!(covered(&family_cases), "{} is not covered", entry.family);
            if family_cases.len() as u64 > entry.minimum {
                assert!(
                    !covered(&family_cases[..family_cases.len() - 1]),
                    "{} is not minimal",
                    entry.family
                );
            }
        }
    }
}

#[test]
fn explicit_allocation_manifests_keep_their_declared_counts() {
    for (name, total) in [("standard.json", 50000u64), ("stress.json", 100000u64)] {
        let manifest = decode_logical_manifest(
            &fs::read(observer_dir().join("manifests").join(name)).unwrap(),
        )
        .unwrap();
        let explicit = manifest.generator.explicit_allocation.unwrap();
        assert_eq!(explicit.normal_case_count, total);
        assert_eq!(explicit.allocation.values().sum::<u64>(), total);
        let plans = domain_plans(
            &decode_logical_manifest(&fs::read(observer_dir().join("manifests").join(name)).unwrap())
                .unwrap(),
            &BTreeMap::new(),
        )
        .unwrap();
        assert_eq!(plans.iter().map(|plan| plan.case_count).sum::<u64>(), total);
    }
}
