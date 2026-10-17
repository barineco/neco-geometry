use std::{env, io::Write, process::ExitCode};

use geometry_observation_producer::render_adapter_failure;

mod cli;

fn main() -> ExitCode {
    let arguments: Vec<String> = env::args().skip(1).collect();
    match cli::dispatch(&arguments) {
        Ok(output) => emit(&output, false),
        Err(failure) => match render_adapter_failure(failure) {
            Ok(output) => emit(&output, true),
            Err(_) => ExitCode::from(1),
        },
    }
}

fn emit(output: &[u8], error: bool) -> ExitCode {
    let result = if error {
        std::io::stderr().lock().write_all(output)
    } else {
        std::io::stdout().lock().write_all(output)
    };
    match (result, error) {
        (Ok(()), false) => ExitCode::SUCCESS,
        _ => ExitCode::from(1),
    }
}
