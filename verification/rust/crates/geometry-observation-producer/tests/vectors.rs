use geometry_observation_producer::generate_request;
use geometry_observation_types::{
    canonical_request_bytes, canonical_value_bytes, digest_hex, validate_canonical,
    GeneratedRequestBatch, GeneratedRequestCase, SELECTOR_NAMES, ADAPTER_REVISION,
    DRAW_SCHEDULE_REVISION, GENERATOR_ALGORITHM, GEOMETRY_DOMAIN,
};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

fn vectors() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../vectors")
}

fn selectors() -> BTreeMap<String, u64> {
    SELECTOR_NAMES
        .into_iter()
        .map(|name| (name.to_owned(), 0))
        .collect()
}

fn case_at(index: u64, family: &str, kind: &str, formula: u64, family_parameter: u64) -> GeneratedRequestCase {
    let mut selectors = selectors();
    selectors.insert("formulaVariant".to_owned(), formula);
    selectors.insert("familyParameter".to_owned(), family_parameter);
    GeneratedRequestCase {
        case_id: format!("case-{index:016x}"),
        global_case_index: index,
        domain_case_index: index,
        family: family.to_owned(),
        requested_observation_kind: kind.to_owned(),
        selectors,
    }
}

fn known_request() -> Vec<u8> {
    let families = [
        ("region-polynomial-normalization", 0, 0, 3, 0),
        ("region-affine-substitution", 0, 0, 0, 3),
        ("region-formula-construction", 0, 0, 3, 0),
        ("region-membership", 0, 0, 3, 0),
        ("surface-witness-exact", 0, 0, 3, 0),
        ("surface-witness-checked", 0, 0, 3, 0),
    ];
    let mut cases = Vec::new();
    let mut index = 0u64;
    for (family, success_formula, success_param, failure_formula, failure_param) in families {
        cases.push(case_at(index, family, "success", success_formula, success_param));
        index += 1;
        cases.push(case_at(index, family, "failure", failure_formula, failure_param));
        index += 1;
    }
    canonical_request_bytes(&GeneratedRequestBatch {
        adapter_revision: ADAPTER_REVISION.to_owned(),
        domain: GEOMETRY_DOMAIN.to_owned(),
        algorithm: GENERATOR_ALGORITHM.to_owned(),
        draw_schedule_revision: DRAW_SCHEDULE_REVISION.to_owned(),
        seed: "known".to_owned(),
        cases,
    })
    .unwrap()
}

#[test]
fn known_vectors_cover_success_and_typed_failure() {
    let generated = generate_request(&known_request()).expect("known generate");
    let cases = validate_canonical(&generated).expect("known canonical");
    assert_eq!(cases.len(), 12);
    let mut operations = BTreeMap::<String, (bool, bool)>::new();
    for case in &cases {
        let operation = case["operation"].as_str().unwrap().to_owned();
        let kind = case["observation"]["kind"].as_str().unwrap();
        let entry = operations.entry(operation).or_insert((false, false));
        if kind == "success" {
            entry.0 = true;
        }
        if kind == "failure" {
            entry.1 = true;
        }
    }
    assert_eq!(operations.len(), 6);
    for (operation, (success, failure)) in operations {
        assert!(success, "{operation} lacks success");
        assert!(failure, "{operation} lacks typed failure");
    }
    fs::create_dir_all(vectors().join("known")).unwrap();
    fs::write(vectors().join("known/families.json"), &generated).unwrap();
    fs::create_dir_all(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../protocol/v1"),
    )
    .unwrap();
    fs::write(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../protocol/v1/geometry-observation-batch.json"),
        &generated,
    )
    .unwrap();
}

#[test]
fn tampered_vectors_change_observed_payloads() {
    let generated = generate_request(&known_request()).expect("known generate");
    let batch: Value = serde_json::from_slice(&generated).unwrap();
    let tampered_dir = vectors().join("tampered");
    fs::create_dir_all(&tampered_dir).unwrap();
    write_tampered(
        &tampered_dir,
        "point.json",
        tampered_case(&batch, 8, |value| {
            if let Some(point) = value.pointer_mut("/cases/8/observation/value/point/x/numerator") {
                *point = Value::String("99".to_owned());
            }
        }),
    );
    write_tampered(
        &tampered_dir,
        "enclosure.json",
        tampered_case(&batch, 10, |value| {
            if let Some(endpoint) = value.pointer_mut("/cases/10/observation/value/x/lower/integer")
            {
                *endpoint = Value::String("99".to_owned());
            }
        }),
    );
    write_tampered(
        &tampered_dir,
        "identity.json",
        tampered_case(&batch, 8, |value| {
            if let Some(version) = value.pointer_mut("/cases/8/observation/value/identity/version") {
                *version = Value::String("9".to_owned());
            }
        }),
    );
    write_tampered(
        &tampered_dir,
        "provenance.json",
        tampered_case(&batch, 10, |value| {
            if let Some(expression) =
                value.pointer_mut("/cases/10/observation/value/provenance/expressions/0")
            {
                *expression = Value::String("9".to_owned());
            }
        }),
    );
    write_tampered(
        &tampered_dir,
        "failure.json",
        tampered_case(&batch, 1, |value| {
            if let Some(variant) = value.pointer_mut("/cases/1/observation/failure/variant") {
                *variant = Value::String("parameter-outside-domain".to_owned());
            }
        }),
    );
}

#[test]
fn invalid_vectors_are_noncanonical_or_untyped() {
    let invalid = vectors().join("invalid");
    fs::create_dir_all(&invalid).unwrap();
    fs::write(invalid.join("whitespace.json"), b"{ \"cases\":[],\"schema\":\"neco.geometry.observation-batch.v1\"}").unwrap();
    fs::write(
        invalid.join("duplicate-key.json"),
        br#"{"cases":[{"caseId":"a","input":{},"observation":{"kind":"success","value":{}},"operation":"region-polynomial-normalization"}],"schema":"neco.geometry.observation-batch.v1","schema":"neco.geometry.observation-batch.v1"}"#,
    )
    .unwrap();
    fs::write(
        invalid.join("duplicate-case-id.json"),
        canonical_value_bytes(&serde_json::json!({
            "cases": [
                {"caseId":"same","input":{},"observation":{"kind":"success","value":{}},"operation":"region-polynomial-normalization"},
                {"caseId":"same","input":{},"observation":{"kind":"success","value":{}},"operation":"region-formula-construction"}
            ],
            "schema": "neco.geometry.observation-batch.v1"
        }))
        .unwrap(),
    )
    .unwrap();
    fs::write(
        invalid.join("key-order.json"),
        br#"{"schema":"neco.geometry.observation-batch.v1","cases":[]}"#,
    )
    .unwrap();
    fs::write(
        invalid.join("unknown-operation.json"),
        canonical_value_bytes(&serde_json::json!({
            "cases": [{
                "caseId": "unknown",
                "input": {},
                "observation": {"kind": "success", "value": {}},
                "operation": "unknown-family"
            }],
            "schema": "neco.geometry.observation-batch.v1"
        }))
        .unwrap(),
    )
    .unwrap();
    assert!(validate_canonical(&fs::read(invalid.join("whitespace.json")).unwrap()).is_err());
    assert!(validate_canonical(&fs::read(invalid.join("duplicate-key.json")).unwrap()).is_err());
    assert!(validate_canonical(&fs::read(invalid.join("duplicate-case-id.json")).unwrap()).is_err());
    assert!(validate_canonical(&fs::read(invalid.join("key-order.json")).unwrap()).is_err());
    assert!(validate_canonical(&fs::read(invalid.join("unknown-operation.json")).unwrap()).is_ok());
}

#[test]
fn declaration_files_have_no_self_digest() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../manifest");
    for name in [
        "geometry-semantics.json",
        "geometry-projection-declaration.json",
        "geometry-resource-declaration.json",
    ] {
        let bytes = fs::read(manifest.join(name)).unwrap();
        let value: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(canonical_value_bytes(&value).unwrap(), bytes, "{name}");
        assert!(value.get("digest").is_none(), "{name} carries a self digest");
        assert_eq!(digest_hex(&bytes).len(), 64);
    }
}

fn tampered_case(batch: &Value, index: usize, edit: impl FnOnce(&mut Value)) -> Value {
    let mut value = batch.clone();
    edit(&mut value);
    serde_json::json!({
        "cases": [value["cases"][index].clone()],
        "schema": "neco.geometry.observation-batch.v1"
    })
}

fn write_tampered(directory: &Path, name: &str, value: Value) {
    fs::write(
        directory.join(name),
        canonical_value_bytes(&value).unwrap(),
    )
    .unwrap();
}
