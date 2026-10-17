use crate::canonical_json;
use crate::protocol::{decode_canonical_bytes, object, ProcessFailure};
use serde_json::Value;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExpectedClassification {
    Refuted,
    Invalid,
    ProcessError,
}

pub fn classification_name(classification: ExpectedClassification) -> &'static str {
    match classification {
        ExpectedClassification::Refuted => "refuted",
        ExpectedClassification::Invalid => "invalid",
        ExpectedClassification::ProcessError => "processError",
    }
}

pub fn mutate_canonical_observation(bytes: &[u8]) -> Result<Vec<u8>, ProcessFailure> {
    let mut batch = decode_canonical_bytes(bytes, "mutateCanonicalObservation")?;
    {
        let object = batch.as_object_mut().ok_or_else(|| {
            ProcessFailure::new(
                "CanonicalDecodeFailure",
                "mutateCanonicalObservation",
                "observation batch must be an object",
            )
        })?;
        let cases = object.get_mut("cases").and_then(Value::as_array_mut).ok_or_else(|| {
            ProcessFailure::new(
                "CanonicalDecodeFailure",
                "mutateCanonicalObservation",
                "observation batch cases must be an array",
            )
        })?;
        let first = cases.iter_mut().find(|case| {
            case.get("observation")
                .and_then(|observation| observation.get("kind"))
                .and_then(Value::as_str)
                == Some("success")
                && case
                    .get("observation")
                    .and_then(|observation| observation.get("value"))
                    .is_some()
        }).ok_or_else(|| {
            ProcessFailure::new(
                "CanonicalDecodeFailure",
                "mutateCanonicalObservation",
                "observation batch must contain a success observation with value",
            )
        })?;
        let observation = first.get_mut("observation").ok_or_else(|| {
            ProcessFailure::new(
                "CanonicalDecodeFailure",
                "mutateCanonicalObservation",
                "observation field is required",
            )
        })?;
        let observation_value = observation.get_mut("value").ok_or_else(|| {
            ProcessFailure::new(
                "CanonicalDecodeFailure",
                "mutateCanonicalObservation",
                "canonical success observation must have value",
            )
        })?;
        if !mutate_numeric_component(observation_value) {
            return Err(ProcessFailure::new(
                "CanonicalDecodeFailure",
                "mutateCanonicalObservation",
                "observation value has no numeric component to mutate",
            ));
        }
    }
    canonical_json(&batch).map_err(|error| {
        ProcessFailure::new("CanonicalDecodeFailure", "mutateCanonicalObservation", error)
    })
}

fn mutate_numeric_component(value: &mut Value) -> bool {
    match value {
        Value::String(text) => {
            if let Ok(number) = text.parse::<i64>() {
                *text = (number.wrapping_add(1)).to_string();
                true
            } else {
                false
            }
        }
        Value::Number(number) => {
            if let Some(integer) = number.as_i64() {
                *value = Value::from(integer.wrapping_add(1));
                true
            } else {
                false
            }
        }
        Value::Object(fields) => fields.values_mut().any(mutate_numeric_component),
        Value::Array(items) => items.iter_mut().any(mutate_numeric_component),
        _ => false,
    }
}

pub fn invalid_protocol_vectors() -> Vec<(&'static str, Vec<u8>)> {
    vec![
        ("non-utf8", vec![0xff, 0xfe, 0x00]),
        (
            "duplicate-key",
            br#"{"cases":[],"schema":"neco.verification.observation-batch.v1","schema":"neco.verification.observation-batch.v1"}"#.to_vec(),
        ),
        (
            "key-order",
            br#"{"schema":"neco.verification.observation-batch.v1","cases":[]}"#.to_vec(),
        ),
        (
            "whitespace",
            br#"{ "cases":[],"schema":"neco.verification.observation-batch.v1"}"#.to_vec(),
        ),
        (
            "schema-invalid",
            br#"{"cases":[],"schema":"neco.verification.not-a-batch.v1"}"#.to_vec(),
        ),
        (
            "semantic-invalid",
            br#"{"cases":[{"caseId":"case-0000000000000000","input":{},"observation":{"kind":"success","value":{"ok":true}},"operation":"unbound-family"}],"schema":"neco.verification.observation-batch.v1"}"#.to_vec(),
        ),
    ]
}

pub fn expected_classification(kind: &str) -> Result<ExpectedClassification, ProcessFailure> {
    match kind {
        "semantic-mutation" => Ok(ExpectedClassification::Refuted),
        "non-utf8" | "duplicate-key" | "key-order" | "whitespace" | "schema-invalid"
        | "semantic-invalid" => Ok(ExpectedClassification::Invalid),
        "logical-manifest" | "descriptor" | "host-execution-config" | "executable-digest" => {
            Ok(ExpectedClassification::ProcessError)
        }
        other => Err(ProcessFailure::new(
            "ManifestValidationFailure",
            "expectedClassification",
            format!("unknown mutation kind {other}"),
        )),
    }
}

pub fn classification_report(
    kind: &str,
    before_digest: &str,
    after_digest: &str,
    observed: &str,
) -> Result<Value, ProcessFailure> {
    let expected = classification_name(expected_classification(kind)?);
    Ok(object([
        ("afterDigest", Value::String(after_digest.to_owned())),
        ("beforeDigest", Value::String(before_digest.to_owned())),
        ("expectedClassification", Value::String(expected.to_owned())),
        ("kind", Value::String(kind.to_owned())),
        ("observedClassification", Value::String(observed.to_owned())),
    ]))
}
