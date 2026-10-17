use std::io::Read;

use geometry_observation_types::{AdapterFailure, OperandProvenance};

use geometry_observation_producer::{describe, generate_request, self_check_request};

pub(crate) fn dispatch(arguments: &[String]) -> Result<Vec<u8>, AdapterFailure> {
    match arguments {
        [command] if command == "describe" => describe(),
        [command, flag, request] if command == "generate" && flag == "--request" => {
            generate_request(&read_input(request)?)
        }
        [command, request_flag, request, batch_flag, batch]
            if command == "self-check"
                && request_flag == "--request"
                && batch_flag == "--batch" =>
        {
            let request = read_input(request)?;
            let batch = read_input(batch)?;
            self_check_request(&request, &batch)
        }
        _ => Err(process_failure(
            "decodeProcessArguments",
            "expected describe, generate --request <path|->, or self-check --request <path|-> --batch <path|->",
        )),
    }
}

fn read_input(path: &str) -> Result<Vec<u8>, AdapterFailure> {
    if path == "-" {
        let mut bytes = Vec::new();
        std::io::stdin()
            .read_to_end(&mut bytes)
            .map_err(|error| process_failure("readProcessInput", error.to_string()))?;
        Ok(bytes)
    } else {
        std::fs::read(path).map_err(|error| process_failure("readProcessInput", error.to_string()))
    }
}

fn process_failure(operation: &'static str, condition: impl Into<String>) -> AdapterFailure {
    AdapterFailure::new(
        "invalidProcessInvocation",
        operation,
        OperandProvenance {
            case_id: None,
            family: None,
            requested_observation_kind: None,
            selectors: Default::default(),
            projection_revision: None,
            resource_revision: None,
            build_identity: None,
        },
        condition,
    )
}
