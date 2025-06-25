use criterion::{criterion_group, criterion_main, Criterion};
use pcode::runtime::Runtime;
use std::time::Duration;

fn benchmark_runtime(c: &mut Criterion) {
    let mut group = c.benchmark_group("runtime");
    group.measurement_time(Duration::from_secs(10));

    group.bench_function("runtime_creation", |b| {
        b.iter(|| {
            let _ = Runtime::new();
        })
    });

    group.bench_function("spawn_task", |b| {
        let runtime = Runtime::new().unwrap();
        b.iter(|| {
            runtime.spawn(async { 42 });
        })
    });

    group.bench_function("spawn_blocking", |b| {
        let runtime = Runtime::new().unwrap();
        b.iter(|| {
            runtime.spawn_blocking(|| 42);
        })
    });

    group.finish();
}

criterion_group!(benches, benchmark_runtime);
criterion_main!(benches);
