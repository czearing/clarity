//! How long a sentence takes to read, judge, and repair.

#![allow(missing_docs)]

use criterion::{criterion_group, criterion_main, Criterion};

use clarity::check::check;
use clarity::grammar::Sentence;
use clarity::repair::repair;

const SHORT: &str = "the dog runs";
const LONG: &str = "the key to the cabinets in the office of the building on the hill is missing";
const FAULTY: &str =
    "the key to the cabinets in the office of the building on the hill are missing";

fn reading(bench: &mut Criterion) {
    bench.bench_function("check short", |run| {
        run.iter(|| check(&Sentence::read(SHORT)));
    });
    bench.bench_function("check long", |run| {
        run.iter(|| check(&Sentence::read(LONG)));
    });
    bench.bench_function("repair long", |run| {
        run.iter(|| repair(&Sentence::read(FAULTY)));
    });
}

criterion_group!(benches, reading);
criterion_main!(benches);
