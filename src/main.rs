//! `stringutils-cli`: a small command-line demo for the `stringutils` safe API.
//!
//! # Usage
//! ```text
//! stringutils-cli <string>
//! ```
//!
//! Given exactly one string argument, the demo runs all four safe-API string
//! operations on it and prints one labeled `key: value` line per operation to
//! stdout, then exits with a zero status:
//!
//! ```text
//! length: 5
//! vowels: 2
//! reversed: olleh
//! uppercase: HELLO
//! ```
//!
//! With no argument or more than one argument, it prints usage guidance to
//! stderr and exits non-zero. If the input contains an interior NUL byte the
//! safe API returns a conversion error; the demo prints that error to stderr,
//! exits non-zero, and prints no operation results (the values are all computed
//! before anything is printed, so a failure prints nothing).
//!
//! This binary uses only the safe API (`stringutils::str_length`,
//! `count_vowels`, `str_reverse`, `to_uppercase`); it contains no `unsafe`.

use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let input = match (args.next(), args.next()) {
        // Exactly one argument -> proceed (Req 6.1).
        (Some(s), None) => s,
        // Zero arguments or more than one -> usage to stderr, non-zero (Req 6.3).
        _ => {
            eprintln!("usage: stringutils-cli <string>");
            return ExitCode::FAILURE;
        }
    };

    match run(&input) {
        // All operations printed successfully; zero status (Req 6.2).
        Ok(()) => ExitCode::SUCCESS,
        // A Conversion error (or any Err) prints the error to stderr and exits
        // non-zero. Because `run` computes every value before printing, no
        // operation result line is emitted on the error path (Req 6.4).
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

/// Invoke all four safe-API operations on `input` and print one labeled
/// `key: value` line per operation to stdout.
///
/// All four results are computed first — each `?` propagates a
/// [`stringutils::Error`] (e.g. the `Conversion` error from an interior NUL) —
/// and only then are they printed. This ordering guarantees that on any error
/// nothing is written to stdout (Req 6.4).
fn run(input: &str) -> Result<(), stringutils::Error> {
    let length = stringutils::str_length(input)?;
    let vowels = stringutils::count_vowels(input)?;
    let reversed = stringutils::str_reverse(input)?;
    let uppercase = stringutils::to_uppercase(input)?;

    println!("length: {length}");
    println!("vowels: {vowels}");
    println!("reversed: {reversed}");
    println!("uppercase: {uppercase}");

    Ok(())
}
