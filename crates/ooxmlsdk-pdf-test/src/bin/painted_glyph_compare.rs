//! Inspect painted glyph identity without changing golden acceptance.

use std::path::Path;
use std::process::ExitCode;

use ooxmlsdk_pdf_test::pdf_painted_glyphs::{compare_pdf_files, painted_glyph_pages};

fn main() -> ExitCode {
    let mut arguments = std::env::args().skip(1);
    if arguments.next().as_deref() == Some("--dump-pages") {
        let Some(pdf) = arguments.next() else {
            eprintln!("usage: painted_glyph_compare --dump-pages PDF.pdf");
            return ExitCode::FAILURE;
        };
        let result = std::fs::read(&pdf)
            .map_err(|error| error.to_string())
            .and_then(|pdf| painted_glyph_pages(&pdf))
            .and_then(|pages| serde_json::to_string(&pages).map_err(|error| error.to_string()));
        return match result {
            Ok(pages) => {
                println!("{pages}");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("painted glyph dump failed: {error}");
                ExitCode::FAILURE
            }
        };
    }
    let mut arguments = std::env::args().skip(1);
    let (Some(candidate), Some(golden), None) =
        (arguments.next(), arguments.next(), arguments.next())
    else {
        eprintln!("usage: painted_glyph_compare CANDIDATE.pdf GOLDEN.pdf");
        return ExitCode::FAILURE;
    };
    match compare_pdf_files(Path::new(&candidate), Path::new(&golden))
        .and_then(|report| serde_json::to_string_pretty(&report).map_err(|error| error.to_string()))
    {
        Ok(report) => {
            println!("{report}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("painted glyph comparison failed: {error}");
            ExitCode::FAILURE
        }
    }
}
