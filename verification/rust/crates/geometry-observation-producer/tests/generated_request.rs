use geometry_observation_producer::{generate_request, self_check_request};
use geometry_observation_types::{
    canonical_request_bytes, decode_canonical_request, validate_canonical, GeneratedRequestBatch,
    GeneratedRequestCase, ADAPTER_REVISION, DRAW_SCHEDULE_REVISION, GENERATOR_ALGORITHM,
    GEOMETRY_DOMAIN, SELECTOR_NAMES,
};
use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

fn selectors_with(mut pairs: BTreeMap<String, u64>) -> BTreeMap<String, u64> {
    for name in SELECTOR_NAMES {
        pairs.entry(name.to_owned()).or_insert(0);
    }
    pairs
}

fn selectors() -> BTreeMap<String, u64> {
    selectors_with(BTreeMap::new())
}

fn request_batch(cases: Vec<GeneratedRequestCase>) -> Vec<u8> {
    canonical_request_bytes(&GeneratedRequestBatch {
        adapter_revision: ADAPTER_REVISION.to_owned(),
        domain: GEOMETRY_DOMAIN.to_owned(),
        algorithm: GENERATOR_ALGORITHM.to_owned(),
        draw_schedule_revision: DRAW_SCHEDULE_REVISION.to_owned(),
        seed: "a3c59ac2f1b0d4e7".to_owned(),
        cases,
    })
    .expect("request bytes")
}

fn case_at(
    index: u64,
    family: &str,
    kind: &str,
    selectors: BTreeMap<String, u64>,
) -> GeneratedRequestCase {
    GeneratedRequestCase {
        case_id: format!("case-{index:016x}"),
        global_case_index: index,
        domain_case_index: index,
        family: family.to_owned(),
        requested_observation_kind: kind.to_owned(),
        selectors,
    }
}

fn families() -> [&'static str; 6] {
    [
        "region-polynomial-normalization",
        "region-affine-substitution",
        "region-formula-construction",
        "region-membership",
        "surface-witness-exact",
        "surface-witness-checked",
    ]
}

#[test]
fn projection_declarations_are_unique_and_complete() {
    let bytes = geometry_observation_producer::describe().expect("describe");
    let value: serde_json::Value = serde_json::from_slice(&bytes).expect("json");
    assert_eq!(
        geometry_observation_types::canonical_value_bytes(&value).unwrap(),
        bytes
    );
    assert_eq!(value["schema"], "neco.verification.producer-description.v1");
    assert_eq!(value["domain"], "neco.geometry");
    let declarations = value["projectionClaim"]["projectionDeclarations"]
        .as_array()
        .expect("declarations");
    assert_eq!(declarations.len(), 6);
    let mut names = Vec::new();
    for declaration in declarations {
        names.push(declaration["family"].as_str().expect("family"));
        assert!(declaration["requestedObservationKinds"]
            .as_array()
            .is_some_and(|kinds| kinds.iter().any(|kind| kind == "success")));
        assert!(declaration["applicableCoverageAxes"]
            .as_array()
            .is_some_and(|axes| !axes.is_empty()));
        assert!(declaration["checkerPredicate"]
            .as_str()
            .is_some_and(|text| !text.is_empty()));
        assert!(declaration["publicApiProjection"]
            .as_str()
            .is_some_and(|text| !text.is_empty()));
        let bindings = declaration["selectorBindings"]
            .as_array()
            .expect("selectorBindings");
        assert!(!bindings.is_empty());
        for binding in bindings {
            let rule = &binding["selectorRule"];
            assert_eq!(rule["modulus"], 4);
            assert_eq!(rule["bucketsByRemainder"].as_object().unwrap().len(), 4);
        }
    }
    names.sort();
    names.dedup();
    assert_eq!(names.len(), 6);
    assert_eq!(value["resourceClaim"]["maxBatchCases"], 2000);
    assert_eq!(
        value["resourceClaim"]["resourceRevision"],
        "geometry-resource-v1"
    );
    assert!(value.get("sourceOID").is_none());
}

#[test]
fn applicable_buckets_are_reachable() {
    for (index, family) in families().into_iter().enumerate() {
        for remainder in 0..4 {
            let mut selectors = selectors();
            selectors.insert("formulaVariant".to_owned(), remainder);
            selectors.insert("value".to_owned(), remainder);
            selectors.insert("familyParameter".to_owned(), remainder);
            selectors.insert("sign".to_owned(), remainder);
            selectors.insert("intervalWidth".to_owned(), remainder);
            for kind in ["success", "failure"] {
                let request =
                    request_batch(vec![case_at(index as u64, family, kind, selectors.clone())]);
                generate_request(&request).unwrap_or_else(|error| {
                    panic!("{family} remainder {remainder} kind {kind}: {error:?}")
                });
            }
        }
    }
}

#[test]
fn selector_axes_are_orthogonal() {
    let family = "region-membership";
    let base = selectors();
    let base_obs = generate_request(&request_batch(vec![case_at(
        0,
        family,
        "success",
        base.clone(),
    )]))
    .expect("base");
    let mut changed_value = base.clone();
    changed_value.insert("value".to_owned(), 2);
    let changed = generate_request(&request_batch(vec![case_at(
        0,
        family,
        "success",
        changed_value,
    )]))
    .expect("changed value");
    let base_cases = validate_canonical(&base_obs).unwrap();
    let changed_cases = validate_canonical(&changed).unwrap();
    assert_ne!(base_cases[0]["input"]["x"], changed_cases[0]["input"]["x"]);
    assert_eq!(base_cases[0]["input"]["y"], changed_cases[0]["input"]["y"]);
}

#[test]
fn non_applicable_selectors_preserve_observations() {
    let family = "region-polynomial-normalization";
    let base = selectors();
    let mut changed = base.clone();
    changed.insert("observationClass".to_owned(), 9);
    changed.insert("mutation".to_owned(), 8);
    changed.insert("reserved".to_owned(), 7);
    changed.insert("sign".to_owned(), 6);
    changed.insert("multiplicity".to_owned(), 5);
    changed.insert("intervalWidth".to_owned(), 4);
    changed.insert("familyParameter".to_owned(), 3);
    let left = generate_request(&request_batch(vec![case_at(0, family, "success", base)])).unwrap();
    let right =
        generate_request(&request_batch(vec![case_at(0, family, "success", changed)])).unwrap();
    let left_cases = validate_canonical(&left).unwrap();
    let right_cases = validate_canonical(&right).unwrap();
    assert_eq!(left_cases[0]["input"], right_cases[0]["input"]);
    assert_eq!(left_cases[0]["observation"], right_cases[0]["observation"]);
    assert_eq!(left_cases[0]["operation"], right_cases[0]["operation"]);
}

#[test]
fn missing_projection_metadata_is_adapter_failure() {
    let request = request_batch(vec![case_at(0, "unknown-family", "success", selectors())]);
    let failure = generate_request(&request).expect_err("unknown family");
    assert_eq!(failure.kind(), "unsupportedProjection");
}

#[test]
fn batch_limit_is_product_resource_limit() {
    let expand = |count: u64| {
        request_batch(
            (0..count)
                .map(|index| {
                    case_at(
                        index,
                        "region-polynomial-normalization",
                        "success",
                        selectors(),
                    )
                })
                .collect(),
        )
    };
    assert!(generate_request(&expand(2000)).is_ok());
    let failure = generate_request(&expand(2001)).expect_err("2001 cases exceed the claim");
    assert_eq!(failure.kind(), "batchResourceLimit");
}

#[test]
fn resource_max_batch_cases_does_not_change_projection_revision() {
    let describe = geometry_observation_producer::describe().unwrap();
    let value: serde_json::Value = serde_json::from_slice(&describe).unwrap();
    let projection = value["projectionClaim"]["projectionRevision"].clone();
    let resource_bytes = fs::read(manifest("geometry-resource-declaration.json")).unwrap();
    let mut altered: serde_json::Value = serde_json::from_slice(&resource_bytes).unwrap();
    altered["maxBatchCases"] = serde_json::json!(1500);
    let altered_bytes = geometry_observation_types::canonical_value_bytes(&altered).unwrap();
    assert_ne!(altered_bytes, resource_bytes);
    assert_eq!(projection, value["projectionClaim"]["projectionRevision"]);
    assert_ne!(
        geometry_observation_types::digest_hex(&altered_bytes),
        geometry_observation_types::digest_hex(&resource_bytes)
    );
    assert!(!String::from_utf8_lossy(
        &geometry_observation_types::canonical_value_bytes(
            &value["projectionClaim"]["projectionDeclarations"]
        )
        .unwrap()
    )
    .contains("maxBatchCases"));
}

#[test]
fn generated_path_has_no_panic_expect_or_unwrap() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");
    let forbidden = ["panic!", "assert!", "unwrap(", "expect("];
    for entry in fs::read_dir(&root).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("rs") {
            continue;
        }
        let text = fs::read_to_string(&path).unwrap();
        let without_tests = text.split("#[cfg(test)]").next().unwrap();
        for token in forbidden {
            assert!(
                !without_tests.contains(token),
                "{} contains {token}",
                path.display()
            );
        }
    }
}

#[test]
fn file_and_self_check_preserve_case_correspondence() {
    let request = request_batch(vec![case_at(
        0,
        "region-polynomial-normalization",
        "success",
        selectors(),
    )]);
    let generated = generate_request(&request).unwrap();
    let decoded = decode_canonical_request(&request).unwrap();
    let observations = validate_canonical(&generated).unwrap();
    assert_eq!(decoded.cases.len(), observations.len());
    assert_eq!(observations[0]["caseId"], decoded.cases[0].case_id);
    self_check_request(&request, &generated).unwrap();
}

#[test]
fn smoke_generates_at_least_one_thousand_cases() {
    let mut cases = Vec::new();
    let families = families();
    for index in 0..1008u64 {
        let family = families[((index / 4) % 6) as usize];
        let remainder = index % 4;
        let kind = if index % 2 == 0 { "success" } else { "failure" };
        let mut selectors = selectors();
        selectors.insert("formulaVariant".to_owned(), remainder);
        selectors.insert("value".to_owned(), index);
        selectors.insert("familyParameter".to_owned(), remainder + 4 * (index / 4));
        selectors.insert("sign".to_owned(), index >> 2);
        selectors.insert("intervalWidth".to_owned(), index >> 3);
        cases.push(case_at(index, family, kind, selectors));
    }
    let request = request_batch(cases);
    let generated = generate_request(&request).expect("smoke generate");
    let observations = validate_canonical(&generated).expect("smoke canonical");
    assert!(observations.len() >= 1000);
    self_check_request(&request, &generated).expect("smoke self-check");
    let generated_dir =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../vectors/generated");
    fs::create_dir_all(&generated_dir).unwrap();
    fs::write(generated_dir.join("smoke-request.json"), &request).unwrap();
    fs::write(generated_dir.join("smoke-batch.json"), &generated).unwrap();
}

#[test]
fn file_and_stdin_generate_the_same_bytes() {
    let request = request_batch(vec![case_at(
        0,
        "region-polynomial-normalization",
        "success",
        selectors(),
    )]);
    let producer = env!("CARGO_BIN_EXE_geometry-observation-producer");
    let directory = std::env::temp_dir().join("geometry-generate");
    fs::create_dir_all(&directory).unwrap();
    let request_path = directory.join("request.json");
    fs::write(&request_path, &request).unwrap();
    let file_output = std::process::Command::new(producer)
        .args(["generate", "--request", request_path.to_str().unwrap()])
        .output()
        .expect("generate file");
    assert!(
        file_output.status.success(),
        "{}",
        String::from_utf8_lossy(&file_output.stderr)
    );
    let mut child = std::process::Command::new(producer)
        .args(["generate", "--request", "-"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("generate stdin");
    child.stdin.take().unwrap().write_all(&request).unwrap();
    let stdin_output = child.wait_with_output().unwrap();
    assert!(
        stdin_output.status.success(),
        "{}",
        String::from_utf8_lossy(&stdin_output.stderr)
    );
    assert_eq!(file_output.stdout, stdin_output.stdout);
    assert_eq!(file_output.stdout, generate_request(&request).unwrap());
}

#[test]
fn describe_rejects_altered_identity_fields() {
    let bytes = geometry_observation_producer::describe().unwrap();
    let mut value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    geometry_observation_types::validate_descriptor_value(&value).unwrap();
    value.as_object_mut().unwrap().remove("targetTriple");
    assert_eq!(
        geometry_observation_types::validate_descriptor_value(&value)
            .unwrap_err()
            .kind(),
        "unboundBuildIdentity"
    );
}

#[test]
fn projection_declaration_file_has_selector_rules() {
    let bytes = fs::read(manifest("geometry-projection-declaration.json")).unwrap();
    let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(
        geometry_observation_types::canonical_value_bytes(&value).unwrap(),
        bytes
    );
    assert!(value.get("digest").is_none());
    let families = value["families"].as_array().expect("families");
    assert_eq!(families.len(), 6);
    let describe = geometry_observation_producer::describe().unwrap();
    let descriptor: serde_json::Value = serde_json::from_slice(&describe).unwrap();
    assert_eq!(
        descriptor["projectionClaim"]["projectionDeclarations"],
        value["families"]
    );
    for family in families {
        assert!(!family["selectorBindings"].as_array().unwrap().is_empty());
        for binding in family["selectorBindings"].as_array().unwrap() {
            let rule = &binding["selectorRule"];
            assert_eq!(rule["modulus"], 4);
            assert_eq!(rule["bucketsByRemainder"].as_object().unwrap().len(), 4);
        }
    }
}

fn manifest(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../manifest")
        .join(name)
}
