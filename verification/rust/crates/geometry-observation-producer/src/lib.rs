#![forbid(unsafe_code)]

mod descriptor;
mod encode;
mod generated_request;
mod projection;
mod region;
mod surface;

use geometry_observation_types::{
    canonical_bytes, canonical_value_bytes, validate_canonical, AdapterFailure, PUBLIC_API_PARITY_SCHEMA,
    object, Value,
};

pub const PRODUCER_REVISION: &str = "1";

pub fn describe() -> Result<Vec<u8>, AdapterFailure> {
    descriptor::describe_bytes()
}

pub fn generate_request(bytes: &[u8]) -> Result<Vec<u8>, AdapterFailure> {
    let cases = generated_request::generate(bytes)?;
    canonical_bytes(&cases).map_err(|failure| AdapterFailure::invalid_request("generate", failure.to_string()))
}

pub fn self_check_request(request: &[u8], supplied: &[u8]) -> Result<Vec<u8>, AdapterFailure> {
    let cases = generated_request::generate(request)?;
    let replayed = canonical_bytes(&cases)
        .map_err(|failure| AdapterFailure::invalid_request("selfCheck", failure.to_string()))?;
    validate_canonical(supplied)
        .map_err(|error| AdapterFailure::invalid_request("selfCheck", error))?;
    if replayed == supplied {
        canonical_value_bytes(&object([
            ("schema", Value::String(PUBLIC_API_PARITY_SCHEMA.to_owned())),
            ("verified", Value::Bool(true)),
        ]))
        .map_err(|failure| AdapterFailure::invalid_request("selfCheck", failure.to_string()))
    } else {
        Err(AdapterFailure::invalid_request(
            "selfCheck",
            "supplied observation bytes differ from public API replay",
        ))
    }
}

pub fn render_adapter_failure(failure: AdapterFailure) -> Result<Vec<u8>, AdapterFailure> {
    canonical_value_bytes(&failure.canonical_value())
        .map_err(|encoding| AdapterFailure::invalid_request("renderFailure", encoding.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use geometry_observation_types::{
        canonical_request_bytes, GeneratedRequestBatch, GeneratedRequestCase, SELECTOR_NAMES,
        ADAPTER_REVISION, DRAW_SCHEDULE_REVISION, GENERATOR_ALGORITHM, GEOMETRY_DOMAIN,
    };
    use std::collections::BTreeMap;

    fn selectors() -> BTreeMap<String, u64> {
        SELECTOR_NAMES
            .into_iter()
            .map(|name| (name.to_owned(), 0))
            .collect()
    }

    fn request_with_kind(family: &str, kind: &str) -> Vec<u8> {
        canonical_request_bytes(&GeneratedRequestBatch {
            adapter_revision: ADAPTER_REVISION.to_owned(),
            domain: GEOMETRY_DOMAIN.to_owned(),
            algorithm: GENERATOR_ALGORITHM.to_owned(),
            draw_schedule_revision: DRAW_SCHEDULE_REVISION.to_owned(),
            seed: "0000000000000000".to_owned(),
            cases: vec![GeneratedRequestCase {
                case_id: "case-0000000000000000".to_owned(),
                global_case_index: 0,
                domain_case_index: 0,
                family: family.to_owned(),
                requested_observation_kind: kind.to_owned(),
                selectors: selectors(),
            }],
        })
        .unwrap()
    }

    #[test]
    fn six_family_success_observations_round_trip() {
        for family in [
            "region-polynomial-normalization",
            "region-affine-substitution",
            "region-formula-construction",
            "region-membership",
            "surface-witness-exact",
            "surface-witness-checked",
        ] {
            let request = request_with_kind(family, "success");
            let generated = generate_request(&request).expect(family);
            let cases = validate_canonical(&generated).expect("canonical");
            assert_eq!(cases.len(), 1);
            assert_eq!(cases[0]["operation"], family);
            assert_eq!(cases[0]["observation"]["kind"], "success");
            self_check_request(&request, &generated).expect("self-check");
        }
    }
}
