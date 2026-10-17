use neco_geometry_observer::protocol::{
    decode_host_execution_config, decode_logical_manifest, decode_producer_descriptor, object,
    validate_descriptor_against_domain, validate_host_execution_config,
};
use neco_geometry_observer::stack_runner::{
    command_compare_repeatability, command_run_lanes, describe_producer,
    start_children_from_validated_inputs,
};
use neco_geometry_observer::{canonical_batch, canonical_json, sha256, verdict_counts};
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::ffi::{OsStr, OsString};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::OnceLock;

fn worktree_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("observer manifest is inside verification/observer")
        .to_path_buf()
}

fn cargo_program() -> OsString {
    env::var_os("CARGO")
        .filter(|program| !program.is_empty())
        .unwrap_or_else(|| OsString::from(format!("{}/.cargo/bin/cargo", env::var("HOME").unwrap())))
}

fn run(command: &mut Command, subject: &str) -> Output {
    if env::var_os("CARGO_TARGET_DIR").is_some_and(|directory| directory.is_empty()) {
        command.env_remove("CARGO_TARGET_DIR");
    }
    let output = command
        .output()
        .unwrap_or_else(|error| panic!("{subject} を起動できません: {error}"));
    assert!(
        output.status.success(),
        "{subject} が {} で終了しました\nstdout:\n{}\nstderr:\n{}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    output
}

fn canonical_value(bytes: &[u8], subject: &str) -> Value {
    let text = std::str::from_utf8(bytes).unwrap_or_else(|error| panic!("{subject} は UTF-8: {error}"));
    let text = text.strip_suffix('\n').unwrap_or(text);
    let value: Value =
        serde_json::from_str(text).unwrap_or_else(|error| panic!("{subject} は JSON: {error}"));
    assert_eq!(
        canonical_json(&value).unwrap_or_else(|error| panic!("{subject} を canonicalize: {error}")),
        text.as_bytes(),
        "{subject} は canonical JSON"
    );
    value
}

fn staged_checker(root: &Path) -> PathBuf {
    let lean = root.join("verification/lean");
    let executable = lean.join(".lake/build/bin").join(format!(
        "neco-geometry-checker{}",
        env::consts::EXE_SUFFIX
    ));
    if !executable.is_file() {
        let mut build = Command::new("lake");
        build.current_dir(&lean).arg("build");
        run(&mut build, "lake build");
    }
    assert!(
        executable.is_file(),
        "Lean checker executable がありません: {}",
        executable.display()
    );
    let mut tests = Command::new(&executable);
    tests.arg("all");
    run(&mut tests, "Lean checker individual test");
    executable
}

fn build_producer(root: &Path) -> PathBuf {
    let manifest = root.join("verification/rust/Cargo.toml");
    let mut build = Command::new(cargo_program());
    build.current_dir(root).args([
        "build",
        "--manifest-path",
        manifest.to_str().expect("Rust manifest path is UTF-8"),
        "--locked",
        "-p",
        "geometry-observation-producer",
        "--bin",
        "geometry-observation-producer",
        "--message-format=json-render-diagnostics",
    ]);
    let output = run(&mut build, "Rust producer build");
    output
        .stdout
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .filter_map(|line| serde_json::from_slice::<Value>(line).ok())
        .find_map(|message| {
            (message["reason"] == "compiler-artifact"
                && message["target"]["name"] == "geometry-observation-producer"
                && message["target"]["kind"]
                    .as_array()
                    .is_some_and(|kinds| kinds.iter().any(|kind| kind == "bin")))
            .then(|| message["executable"].as_str().map(PathBuf::from))
            .flatten()
        })
        .expect("Rust producer build は executable 成果物を返します")
}

fn observer() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_neco-geometry-observer"))
}

fn checker_result(checker: &Path, batch: &Path) -> Value {
    let mut command = Command::new(checker);
    command.args([OsStr::new("check"), batch.as_os_str()]);
    canonical_value(&run(&mut command, "Lean checker").stdout, "Lean checker stdout")
}

#[test]
fn stack_runner_connects_producer_checker_and_observer() {
    let root = worktree_root();
    let checker = staged_checker(&root);
    let producer = build_producer(&root);
    let mut describe = Command::new(&producer);
    describe.arg("describe");
    let description = canonical_value(&run(&mut describe, "producer describe").stdout, "describe");
    assert_eq!(description["schema"], "neco.verification.producer-description.v1");
    assert_eq!(description["domain"], "neco.geometry");
    assert!(description.get("sourceOID").is_none());
    assert!(description["rustcIdentity"].as_str().unwrap().starts_with("rustc "));
    assert!(!description["targetTriple"].as_str().unwrap().is_empty());
    assert_eq!(description["resourceClaim"]["maxBatchCases"], 2000);
    assert_eq!(
        description["projectionClaim"]["projectionDeclarations"]
            .as_array()
            .unwrap()
            .len(),
        6
    );

    let mut unsupported = Command::new(&producer);
    unsupported.arg("unsupported-command");
    let failure = unsupported.output().expect("producer failure");
    assert!(!failure.status.success());
    let rendered = canonical_value(&failure.stderr, "adapter failure");
    assert_eq!(rendered["schema"], "neco.verification.adapter-failure.v1");
    assert_eq!(rendered["kind"], "invalidProcessInvocation");
    assert!(rendered["failingOperation"].as_str().is_some());
    assert!(rendered["operandProvenance"].is_object());
    assert!(rendered["violatedCondition"].as_str().is_some());
    assert!(rendered["contributingFailures"].is_array());

    let batch = root.join("verification/vectors/known/families.json");
    canonical_batch(&batch).expect("known batch is canonical");
    let result = checker_result(&checker, &batch);
    let digest = sha256(&fs::read(&batch).unwrap());
    assert_eq!(result["digest"], digest);
    let counts = verdict_counts(&result["cases"]).unwrap();
    assert_eq!(counts.get("verified"), Some(&12));

    let result_path = root.join("verification/observer/target/known-checker-result.json");
    fs::create_dir_all(result_path.parent().unwrap()).unwrap();
    fs::write(&result_path, canonical_json(&result).unwrap()).unwrap();
    let mut observer_cmd = Command::new(observer());
    observer_cmd.args([
        OsStr::new("--batch"),
        batch.as_os_str(),
        OsStr::new("--checker-result"),
        result_path.as_os_str(),
    ]);
    let observed = canonical_value(&run(&mut observer_cmd, "observer").stdout, "observer");
    assert_eq!(observed["digest"], digest);
}

#[test]
fn manifests_separate_inputs_outputs_and_component_responsibilities() {
    let root = worktree_root();
    let manifest_directory = root.join("verification/manifest");
    let expected_keys = BTreeSet::from([
        "repositoryInputs",
        "toolchainInputs",
        "runtimeInputs",
        "repositoryOutputs",
        "forbidden",
    ]);
    for file_name in ["lean-inputs.json", "rust-inputs.json", "observer-inputs.json"] {
        let bytes = fs::read(manifest_directory.join(file_name)).unwrap();
        let manifest: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(canonical_json(&manifest).unwrap(), bytes);
        let object = manifest.as_object().unwrap();
        assert_eq!(
            object.keys().map(String::as_str).collect::<BTreeSet<_>>(),
            expected_keys
        );
        let output_text = serde_json::to_string(&manifest["repositoryOutputs"]).unwrap();
        assert!(output_text.contains("digest"));
    }

    let observer_manifest = root.join("verification/observer/Cargo.toml");
    let mut metadata = Command::new(cargo_program());
    metadata.current_dir(&root).args([
        "metadata",
        "--manifest-path",
        observer_manifest.to_str().unwrap(),
        "--locked",
        "--no-deps",
        "--format-version",
        "1",
    ]);
    let output = run(&mut metadata, "observer Cargo metadata");
    let metadata: Value = serde_json::from_slice(&output.stdout).unwrap();
    let package = metadata["packages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|package| package["name"] == "neco-geometry-observer")
        .unwrap();
    let direct_dependencies = package["dependencies"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|dependency| dependency["name"].as_str())
        .collect::<BTreeSet<_>>();
    assert!(
        direct_dependencies.is_disjoint(&BTreeSet::from(["neco-geometry", "neco-calculus"])),
        "observer は製品 API crate に直接依存しません: {direct_dependencies:?}"
    );
}

fn observer_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
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
    let status = Command::new(cargo_program())
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
    let bytes = fs::read(observer_dir().join("manifests/smoke.json")).unwrap();
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

#[test]
fn validated_descriptor_and_host_execution_config_start_children() {
    let root = observer_dir().join("target/verification-start-children");
    fs::create_dir_all(&root).unwrap();
    let stub = stub_executable();
    let mut staged = BTreeMap::new();
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
        staged.insert(domain.to_owned(), (producer.clone(), checker.clone()));
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
    let manifest_path = root.join("logical-manifest.json");
    fs::write(&manifest_path, fixture_manifest_bytes()).unwrap();
    let host_path = root.join("host-execution.json");
    fs::write(
        &host_path,
        canonical_json(&object([
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
        ]))
        .unwrap(),
    )
    .unwrap();
    let manifest = decode_logical_manifest(&fs::read(&manifest_path).unwrap()).unwrap();
    let host = validate_host_execution_config(
        &decode_host_execution_config(&fs::read(&host_path).unwrap()).unwrap(),
        &manifest.domains,
    )
    .unwrap();
    let mut descriptors = BTreeMap::new();
    for domain in &manifest.domains {
        let (producer, _) = staged.get(&domain.domain).unwrap();
        let descriptor = describe_producer(producer, &host.working_directory).unwrap();
        decode_producer_descriptor(&descriptor.bytes).unwrap();
        validate_descriptor_against_domain(&descriptor, domain).unwrap();
        descriptors.insert(domain.domain.clone(), descriptor);
    }
    let started = start_children_from_validated_inputs(&manifest, &host, &descriptors).unwrap();
    assert_eq!(started["started"], true);
}

const DOMAINS: [&str; 3] = ["neco.algebra", "neco.calculus", "neco.geometry"];
const SEVEN_LANES: &str = "observationVerification,soundnessTheorem,publicApiParity,mutationRejection,invalidProtocol,corpusExecution,repeatability";

struct CheckerLie {
    domain: &'static str,
    input_glob: &'static str,
    sed: String,
}

fn lane_fixture(name: &str, lies: &[CheckerLie]) -> (PathBuf, PathBuf) {
    let root = observer_dir().join("target").join(name);
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(root.join("reports")).unwrap();
    fs::create_dir_all(root.join("temporary")).unwrap();
    let stub = stub_executable();
    let mut domains = Map::new();
    for domain in DOMAINS {
        let producer = root.join(format!("{domain}-producer"));
        let checker = root.join(format!("{domain}-checker"));
        fs::copy(&stub, &producer).unwrap();
        match lies.iter().find(|lie| lie.domain == domain) {
            None => {
                fs::copy(&stub, &checker).unwrap();
            }
            Some(lie) => {
                let real = root.join(format!("{domain}-checker.real"));
                fs::copy(&stub, &real).unwrap();
                fs::write(
                    &checker,
                    format!(
                        "#!/bin/sh\ncase \"$(basename \"$2\")\" in\n{glob}) \"$0.real\" \"$@\" | sed '{sed}' ;;\n*) exec \"$0.real\" \"$@\" ;;\nesac\n",
                        glob = lie.input_glob,
                        sed = lie.sed,
                    ),
                )
                .unwrap();
                set_executable(&real);
            }
        }
        set_executable(&producer);
        set_executable(&checker);
        domains.insert(
            domain.to_owned(),
            object([
                ("checkerPath", Value::String(checker.display().to_string())),
                ("producerPath", Value::String(producer.display().to_string())),
            ]),
        );
    }
    let manifest_path = root.join("logical-manifest.json");
    fs::write(&manifest_path, fixture_manifest_bytes()).unwrap();
    let host_path = root.join("host-execution.json");
    fs::write(
        &host_path,
        canonical_json(&object([
            ("domains", Value::Object(domains)),
            ("reportDirectory", Value::String(root.join("reports").display().to_string())),
            ("temporaryDirectory", Value::String(root.join("temporary").display().to_string())),
            ("workingDirectory", Value::String(root.display().to_string())),
        ]))
        .unwrap(),
    )
    .unwrap();
    (manifest_path, host_path)
}

fn set_executable(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let mut permissions = fs::metadata(path).unwrap().permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(path, permissions).unwrap();
}

fn lanes_failure(name: &str, lies: &[CheckerLie], lanes: &str) -> neco_geometry_observer::protocol::ProcessFailure {
    let (manifest, host) = lane_fixture(name, lies);
    command_run_lanes(&manifest, &host, lanes, true, true).expect_err("run-lanes は拒否します")
}

#[test]
fn run_lanes_reports_mutation_and_invalid_protocol_for_every_domain() {
    let (manifest, host) = lane_fixture("verification-lanes-honest", &[]);
    let report = command_run_lanes(&manifest, &host, SEVEN_LANES, true, true).unwrap();
    for lane in ["mutationRejection", "invalidProtocol"] {
        let by_domain = report_lane(&report, lane);
        let keys: Vec<&String> = by_domain.keys().collect();
        assert_eq!(keys, DOMAINS.iter().collect::<Vec<_>>(), "{lane}");
    }
}

fn report_lane<'a>(report: &'a Value, lane: &str) -> &'a Map<String, Value> {
    let path = report["hostExecutionProvenance"]["reportDirectory"].as_str().unwrap();
    let written = canonical_value(&fs::read(Path::new(path).join("lanes.json")).unwrap(), "lanes report");
    assert_eq!(&written, report);
    report["laneResults"][lane].as_object().unwrap_or_else(|| panic!("{lane} は分野別 object です: {report}"))
}

#[test]
fn run_lanes_rejects_mutation_misjudged_by_a_domain_other_than_the_first() {
    for domain in ["neco.calculus", "neco.geometry"] {
        let lies = [CheckerLie {
            domain,
            input_glob: "observation-mutation-*",
            sed: r#"s/"verdict":"refuted"/"verdict":"verified"/g"#.to_owned(),
        }];
        let failure = lanes_failure("verification-lanes-mutation-lie", &lies, SEVEN_LANES);
        assert_eq!(failure.failing_operation(), "mutationRejection", "{domain}");
        assert_eq!(failure.witness()["observedClassification"], "verified", "{domain}");
    }
}

#[test]
fn run_lanes_rejects_invalid_protocol_misjudged_by_a_domain_other_than_the_first() {
    for domain in ["neco.calculus", "neco.geometry"] {
        let lies = [CheckerLie {
            domain,
            input_glob: "invalid-*",
            sed: r#"s/"batchVerdict":"invalid"/"batchVerdict":"verified"/"#.to_owned(),
        }];
        let failure = lanes_failure("verification-lanes-invalid-lie", &lies, SEVEN_LANES);
        assert_eq!(failure.failing_operation(), "invalidProtocol", "{domain}");
    }
}

fn foreign_digest_lie() -> Vec<CheckerLie> {
    vec![CheckerLie {
        domain: "neco.calculus",
        input_glob: "observation-neco.calculus-*",
        sed: format!(r#"s/"digest":"[0-9a-f]\{{64\}}"/"digest":"{}"/"#, "0".repeat(64)),
    }]
}

#[test]
fn run_lanes_rejects_checker_result_bound_to_a_different_batch() {
    let failure = lanes_failure("verification-lanes-digest-lie", &foreign_digest_lie(), "observationVerification");
    assert_eq!(failure.kind(), "CaseCorrespondenceMismatch");
    assert_eq!(failure.failing_operation(), "checkerCheck");
}

#[test]
fn compare_repeatability_rejects_checker_result_bound_to_a_different_batch() {
    let (manifest, host) = lane_fixture("verification-compare-digest-lie", &foreign_digest_lie());
    let report = observer_dir().join("target/verification-compare-digest-lie/compare.json");
    let failure = command_compare_repeatability(&manifest, &host, 2, true, 1, &report)
        .expect_err("compare-repeatability は拒否します");
    assert_eq!(failure.kind(), "CaseCorrespondenceMismatch");
    assert_eq!(failure.failing_operation(), "checkerCheck");
}
