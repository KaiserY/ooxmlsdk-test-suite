use std::process::ExitCode;

fn main() -> ExitCode {
    match olecfsdk_test::audit_classic_office_file_roots()
        .and_then(|report| report.to_pretty_json().map_err(|error| error.to_string()))
    {
        Ok(json) => {
            print!("{json}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("olecfsdk coverage audit failed: {error}");
            ExitCode::FAILURE
        }
    }
}
