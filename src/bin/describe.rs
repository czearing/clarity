//! Write about an input, whatever the input is.
//!
//! Give it a directory and it reads the source under it. Give it a file and it reads the prose in
//! it. Either way the same two searches decide what is worth saying and how to say it, and every
//! word printed was read out of what it was given.

use std::path::PathBuf;
use std::process::ExitCode;
use std::{env, fs};

use clarity::read::{read_prose, read_tree, Reading};
use clarity_say::{compose, Clause, Said, MOST_CLAIMS};
use fitkit::Answer;

fn main() -> ExitCode {
    let Some(target) = env::args().nth(1).map(PathBuf::from) else {
        eprintln!("give it a directory of source or a file of prose");
        return ExitCode::FAILURE;
    };
    let reading = if target.is_dir() {
        read_tree(&target)
    } else {
        match fs::read_to_string(&target) {
            Ok(text) => read_prose(&text),
            Err(problem) => {
                eprintln!("{}: {problem}", target.display());
                return ExitCode::FAILURE;
            }
        }
    };
    let reading = match reading {
        Ok(reading) => reading,
        Err(refusal) => {
            eprintln!("nothing written: {refusal}");
            return ExitCode::FAILURE;
        }
    };
    match write(&reading) {
        Ok(said) => {
            report(&reading, &said);
            ExitCode::SUCCESS
        }
        Err(refusal) => {
            eprintln!("nothing written: {refusal}");
            ExitCode::FAILURE
        }
    }
}

/// Compose, letting the search decide how much there is to say.
fn write(reading: &Reading) -> Answer<Said> {
    compose(reading.corpus(), reading.claims(), MOST_CLAIMS)
}

/// Print what was said, and what stands behind it.
fn report(reading: &Reading, said: &Said) {
    let passages = match said.passages() {
        Ok(passages) => passages,
        Err(refusal) => {
            eprintln!("nothing written: {refusal}");
            return;
        }
    };
    for (position, passage) in passages.iter().enumerate() {
        if position > 0 {
            println!();
        }
        let written: Vec<String> = passage.iter().map(|clause| clause.text()).collect();
        println!("{}", written.join(" "));
    }
    let clauses: Vec<&Clause> = passages.into_iter().flatten().collect();
    let trace = said.trace();
    eprintln!();
    eprintln!(
        "read {} words over {} tokens, weighed {} parts, stated {}",
        reading.corpus().vocabulary(),
        reading.corpus().tokens(),
        reading.claims().len(),
        said.stated(),
    );
    eprintln!(
        "selection considered {} and turned down {}, margin {:.3}",
        trace.considered(),
        trace.rejected(),
        trace.margin(),
    );
    for clause in &clauses {
        let step = clause.trace();
        eprintln!(
            "  clause cost {:.3} over {} positions and {} places, margin {:.3}",
            clause.cost(),
            step.steps(),
            step.states(),
            step.margin(),
        );
    }
}
