use neco_geometry_observer::canonical_json;
use neco_geometry_observer::protocol::{
    current_host_target_triple, decode_canonical_bytes, decode_host_execution_config,
    decode_logical_manifest, decode_producer_descriptor, decode_projection_declarations, object,
    validate_runtime_host_compatibility,
};
use neco_geometry_observer::stack_runner::stub_main;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;

fn observer_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn smoke_manifest_path() -> PathBuf {
    observer_dir().join("manifests/smoke.json")
}

fn committed_host_path() -> PathBuf {
    observer_dir().join("host-execution/host.json")
}

#[test]
fn logical_manifest_is_canonical_portable_requirement() {
    for name in ["smoke.json", "standard.json", "stress.json"] {
        let path = observer_dir().join("manifests").join(name);
        let bytes = fs::read(&path).unwrap();
        let manifest = decode_logical_manifest(&bytes).expect("logical manifest");
        assert_eq!(
            canonical_json(&serde_json::from_slice::<Value>(&bytes).unwrap()).unwrap(),
            bytes
        );
        let text = String::from_utf8(bytes.clone()).unwrap();
        assert!(!text.contains("gitObjectId"));
        assert!(!text.contains("descriptorDigest"));
        assert!(!text.contains("artifactDigest"));
        assert!(!text.contains("selfDigest"));
        assert!(!text.contains("/Users/"));
        assert_eq!(manifest.domains.len(), 3);
        assert!(!manifest.domains[0]
            .semantics_ref
            .protocol_revision
            .is_empty());
    }
}

#[test]
fn validates_host_execution_config_and_runtime_host_compatibility() {
    let bytes = fs::read(committed_host_path()).unwrap();
    decode_host_execution_config(&bytes).expect("committed host execution config");
    let mut broken = serde_json::from_slice::<Value>(&bytes).unwrap();
    broken.as_object_mut().unwrap().remove("workingDirectory");
    let encoded = canonical_json(&broken).unwrap();
    let missing = decode_host_execution_config(&encoded).expect_err("missing field");
    assert_eq!(missing.kind(), "ManifestValidationFailure");
    let noncanonical = b"{ \"domains\":{},\"reportDirectory\":\"a\",\"temporaryDirectory\":\"b\",\"workingDirectory\":\"c\"}";
    let failed = decode_canonical_bytes(noncanonical, "decodeHostExecutionConfig")
        .expect_err("noncanonical");
    assert_eq!(failed.kind(), "CanonicalDecodeFailure");
    let mut manifest: Value =
        serde_json::from_slice(&fs::read(smoke_manifest_path()).unwrap()).unwrap();
    manifest
        .as_object_mut()
        .unwrap()
        .insert("note".to_owned(), Value::String("/tmp/absolute".to_owned()));
    let with_path = canonical_json(&manifest).unwrap();
    let absolute = decode_logical_manifest(&with_path).expect_err("absolute path");
    assert_eq!(absolute.kind(), "ManifestValidationFailure");
    let mismatch =
        validate_runtime_host_compatibility("mips-unknown-linux-gnu").expect_err("triple");
    assert_eq!(mismatch.kind(), "RuntimeHostCompatibilityFailure");
    validate_runtime_host_compatibility(&current_host_target_triple()).expect("current host");
    let empty = decode_host_execution_config(
        &canonical_json(&object([
            ("domains", object([])),
            ("reportDirectory", Value::String("reports".to_owned())),
            ("temporaryDirectory", Value::String("temporary".to_owned())),
            ("workingDirectory", Value::String(String::new())),
        ]))
        .unwrap(),
    )
    .expect_err("empty working directory");
    assert_eq!(empty.kind(), "HostExecutionConfigInvalid");
}

#[test]
fn rejects_unbound_descriptor_references_and_preserves_structured_process_failures() {
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
                ("selector", Value::String("value".to_owned())),
                (
                    "selectorRule",
                    object([
                        (
                            "bucketsByRemainder",
                            object([("0", Value::String("even".to_owned()))]),
                        ),
                        ("modulus", Value::from(2)),
                        ("selector", Value::String("value".to_owned())),
                    ]),
                ),
            ])]),
        ),
    ]);
    let incomplete =
        decode_projection_declarations(&[declaration]).expect_err("partial remainder map");
    assert_eq!(incomplete.kind(), "UnboundDescriptorReference");
    let alias = object([
        (
            "applicableAxes",
            Value::Array(vec![Value::String("value".to_owned())]),
        ),
        (
            "applicableCoverageAxes",
            Value::Array(vec![Value::String("value".to_owned())]),
        ),
        ("family", Value::String("rational-reduction".to_owned())),
        (
            "requestedObservationKinds",
            Value::Array(vec![Value::String("success".to_owned())]),
        ),
        ("selectorBindings", Value::Array(vec![])),
    ]);
    let rejected = decode_projection_declarations(&[alias]).expect_err("alias");
    assert_eq!(rejected.kind(), "UnboundDescriptorReference");
    let unreachable = object([
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
                    Value::Array(vec![object([
                        ("name", Value::String("missing".to_owned())),
                        ("projectedCondition", Value::String("none".to_owned())),
                    ])]),
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
    let unreachable_error =
        decode_projection_declarations(&[unreachable]).expect_err("unreachable");
    assert_eq!(unreachable_error.kind(), "UnboundDescriptorReference");
    let value = rejected.to_value();
    let object = value.as_object().unwrap();
    for field in [
        "kind",
        "failingOperation",
        "operandProvenance",
        "violatedCondition",
        "witness",
        "contributingFailures",
    ] {
        assert!(object.contains_key(field), "{field}");
    }
    assert_ne!(rejected.kind(), "verified");
    assert_ne!(rejected.kind(), "refuted");
    assert_ne!(rejected.kind(), "invalid");
}

#[test]
fn stub_describe_is_canonical() {
    let bytes = stub_main(vec!["describe".to_owned()], "neco.geometry-producer").unwrap();
    decode_producer_descriptor(&bytes).unwrap();
}
