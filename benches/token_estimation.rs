use criterion::{black_box, criterion_group, criterion_main, Criterion};
use pcode::token_estimation::Tokenizer;

fn benchmark_token_estimation(c: &mut Criterion) {
    let tokenizer = Tokenizer::new();

    let short_text = "The quick brown fox jumps over the lazy dog";
    let medium_text = short_text.repeat(10);
    let long_text = short_text.repeat(100);

    c.bench_function("token_estimate_short", |b| {
        b.iter(|| tokenizer.estimate_tokens(black_box(short_text)))
    });

    c.bench_function("token_estimate_medium", |b| {
        b.iter(|| tokenizer.estimate_tokens(black_box(&medium_text)))
    });

    c.bench_function("token_estimate_long", |b| {
        b.iter(|| tokenizer.estimate_tokens(black_box(&long_text)))
    });

    c.bench_function("token_estimate_fast_short", |b| {
        b.iter(|| tokenizer.estimate_tokens_fast(black_box(short_text)))
    });

    c.bench_function("token_estimate_fast_long", |b| {
        b.iter(|| tokenizer.estimate_tokens_fast(black_box(&long_text)))
    });
}

criterion_group!(benches, benchmark_token_estimation);
criterion_main!(benches);
