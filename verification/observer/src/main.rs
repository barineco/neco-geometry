use neco_geometry_observer::protocol::ProcessFailure;
use neco_geometry_observer::stack_runner::{
    command_compare_repeatability, command_compare_request_digests, command_generate,
    command_run_lanes, command_test_infeasible_minimum, command_validate_resource_reports,
    command_verify_coverage,
};
use neco_geometry_observer::{
    canonical_batch, canonical_json, checker_result, sha256, verdict_counts, CheckerResult,
};
use serde_json::json;
use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn canonical_value(path: &Path) -> Result<serde_json::Value, String> {
    let bytes = fs::read(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let value: serde_json::Value = serde_json::from_slice(&bytes)
        .map_err(|error| format!("{}: {error}", path.display()))?;
    if canonical_json(&value)? != bytes {
        return Err(format!(
            "{}: canonical JSON bytes と一致しません",
            path.display()
        ));
    }
    Ok(value)
}

fn batch_case_ids(batch: &serde_json::Value) -> Result<Vec<String>, String> {
    batch["cases"]
        .as_array()
        .ok_or_else(|| "batch cases は array です".to_owned())?
        .iter()
        .map(|case| {
            case["caseId"]
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| "batch caseId は string です".to_owned())
        })
        .collect()
}

fn verify(batch_path: &Path, checker_result_path: &Path) -> Result<String, String> {
    let batch_bytes = canonical_batch(batch_path)?;
    let batch: serde_json::Value = serde_json::from_slice(&batch_bytes)
        .map_err(|error| format!("{}: {error}", batch_path.display()))?;
    let batch_digest = sha256(&batch_bytes);
    let result = canonical_value(checker_result_path)?;
    match checker_result(&result, &batch_digest)? {
        CheckerResult::BatchInvalid => Err(
            "checker result の batchVerdict は observer が batch の case を列挙できた結果と矛盾します"
                .to_owned(),
        ),
        CheckerResult::Cases(cases) => {
            let counts = verdict_counts(cases)?;
            let checker_case_ids = cases
                .as_array()
                .ok_or_else(|| "checker cases は array です".to_owned())?
                .iter()
                .map(|case| {
                    case["caseId"]
                        .as_str()
                        .map(str::to_owned)
                        .ok_or_else(|| "checker caseId は string です".to_owned())
                })
                .collect::<Result<Vec<_>, _>>()?;
            if checker_case_ids != batch_case_ids(&batch)? {
                return Err("checker caseId は batch の同数同順の caseId と一致します".to_owned());
            }
            String::from_utf8(
                canonical_json(&json!({"cases":counts,"digest":batch_digest}))
                    .map_err(|error| error.to_string())?,
            )
            .map_err(|error| error.to_string())
        }
    }
}

fn flag_value(arguments: &[String], name: &str) -> Result<String, String> {
    let mut index = 0;
    while index + 1 < arguments.len() {
        if arguments[index] == name {
            return Ok(arguments[index + 1].clone());
        }
        index += 1;
    }
    Err(format!("missing {name}"))
}

fn has_flag(arguments: &[String], name: &str) -> bool {
    arguments.iter().any(|argument| argument == name)
}

fn usage_text() -> String {
    "usage: --batch <canonical-observation-batch> --checker-result <canonical-checker-result> | generate --logical-manifest <path> --host-execution-config <path> --shard-size <n> --out <dir> | compare-request-digests <dir1> <dir2> | verify-coverage --logical-manifest <path> --host-execution-config <path> --require-descriptor-buckets --require-selector-rules --require-shard-at-most-producer-max | test-infeasible-minimum --logical-manifest <path> | run-lanes --logical-manifest <path> --host-execution-config <path> --lanes <csv> --require-shared-normal-corpus --require-structured-process-failures | compare-repeatability --logical-manifest <path> --host-execution-config <path> --runs 2 --sequential --shard-size <n> --write-report <path> | validate-resource-reports --require-sequential-staging --require-two-real-passes --require-shard-at-most-producer-max <reports...>".to_owned()
}

fn render_failure(failure: ProcessFailure) -> String {
    failure
        .render()
        .unwrap_or_else(|error| error)
}

fn run_command(arguments: &[String]) -> Result<String, String> {
    let command = arguments.first().map(String::as_str).ok_or_else(usage_text)?;
    let rest = &arguments[1..];
    let value = match command {
        "generate" => {
            let manifest = flag_value(rest, "--logical-manifest")?;
            let shard_size: u64 = flag_value(rest, "--shard-size")?
                .parse()
                .map_err(|_| "shard-size must be u64".to_owned())?;
            let out = flag_value(rest, "--out")?;
            let host = flag_value(rest, "--host-execution-config")?;
            command_generate(
                Path::new(&manifest),
                Path::new(&host),
                shard_size,
                Path::new(&out),
            )
        }
        "compare-request-digests" => {
            if rest.len() != 2 {
                return Err(usage_text());
            }
            command_compare_request_digests(Path::new(&rest[0]), Path::new(&rest[1]))
        }
        "verify-coverage" => {
            let manifest = flag_value(rest, "--logical-manifest")?;
            let host = flag_value(rest, "--host-execution-config")?;
            command_verify_coverage(
                Path::new(&manifest),
                Path::new(&host),
                has_flag(rest, "--require-descriptor-buckets"),
                has_flag(rest, "--require-selector-rules"),
                has_flag(rest, "--require-shard-at-most-producer-max"),
            )
        }
        "test-infeasible-minimum" => {
            let manifest = flag_value(rest, "--logical-manifest")?;
            command_test_infeasible_minimum(Path::new(&manifest))
        }
        "run-lanes" => {
            let manifest = flag_value(rest, "--logical-manifest")?;
            let host = flag_value(rest, "--host-execution-config")?;
            let lanes = flag_value(rest, "--lanes")?;
            command_run_lanes(
                Path::new(&manifest),
                Path::new(&host),
                &lanes,
                has_flag(rest, "--require-shared-normal-corpus"),
                has_flag(rest, "--require-structured-process-failures"),
            )
        }
        "compare-repeatability" => {
            let manifest = flag_value(rest, "--logical-manifest")?;
            let host = flag_value(rest, "--host-execution-config")?;
            let runs: u64 = flag_value(rest, "--runs")?
                .parse()
                .map_err(|_| "runs must be u64".to_owned())?;
            let shard_size: u64 = flag_value(rest, "--shard-size")?
                .parse()
                .map_err(|_| "shard-size must be u64".to_owned())?;
            let report = flag_value(rest, "--write-report")?;
            command_compare_repeatability(
                Path::new(&manifest),
                Path::new(&host),
                runs,
                has_flag(rest, "--sequential"),
                shard_size,
                Path::new(&report),
            )
        }
        "validate-resource-reports" => {
            let reports = rest
                .iter()
                .filter(|argument| !argument.starts_with("--"))
                .map(PathBuf::from)
                .collect::<Vec<_>>();
            command_validate_resource_reports(
                has_flag(rest, "--require-sequential-staging"),
                has_flag(rest, "--require-two-real-passes"),
                has_flag(rest, "--require-shard-at-most-producer-max"),
                &reports,
            )
        }
        _ => return Err(usage_text()),
    }
    .map_err(render_failure)?;
    String::from_utf8(canonical_json(&value).map_err(|error| error.to_string())?)
        .map_err(|error| error.to_string())
}

fn run() -> Result<String, String> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    if arguments.first().map(String::as_str) == Some("--batch") {
        let [batch_flag, batch, result_flag, result] = arguments.as_slice() else {
            return Err("usage: --batch <canonical-observation-batch> --checker-result <canonical-checker-result>".to_owned());
        };
        if batch_flag != "--batch" || result_flag != "--checker-result" {
            return Err("usage: --batch <canonical-observation-batch> --checker-result <canonical-checker-result>".to_owned());
        }
        return verify(Path::new(batch), Path::new(result));
    }
    run_command(&arguments)
}

fn main() -> ExitCode {
    match run() {
        Ok(output) => {
            let mut stdout = std::io::stdout().lock();
            if stdout.write_all(output.as_bytes()).is_ok() && stdout.write_all(b"\n").is_ok() {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            }
        }
        Err(error) => {
            let mut stderr = std::io::stderr().lock();
            let _ = stderr.write_all(format!("neco-geometry-observer: {error}\n").as_bytes());
            ExitCode::from(1)
        }
    }
}
