mod checker;
mod config;
mod marker;
mod report;
mod source_file;

use std::{env, process::ExitCode};

use crate::{checker::Checker, config::Config};

fn main() -> ExitCode {
    let base = match env::current_dir() {
        Ok(base) => base,
        Err(error) => {
            eprintln!("lint-style: failed to resolve the current directory: {error}");
            return ExitCode::FAILURE;
        }
    };

    let config = match Config::load(&base) {
        Ok(config) => config,
        Err(error) => {
            eprintln!("lint-style: {error:#}");
            return ExitCode::FAILURE;
        }
    };

    let report = match Checker::new(&base, config).run() {
        Ok(report) => report,
        Err(error) => {
            eprintln!("lint-style: {error:#}");
            return ExitCode::FAILURE;
        }
    };

    for debt in &report.debts {
        println!("{debt}");
    }
    for error in &report.errors {
        println!("{error}");
    }

    if report.errors.is_empty() {
        println!("lint-style: ok ({} allow, {} debt)", report.allow_count, report.debts.len());
        ExitCode::SUCCESS
    } else {
        println!();
        println!(
            "lint-style: {} error(s), {} allow, {} debt. see docs/coding/rust.md",
            report.errors.len(),
            report.allow_count,
            report.debts.len()
        );
        ExitCode::FAILURE
    }
}
