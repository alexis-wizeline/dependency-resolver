use criterion::{BenchmarkId, Criterion, criterion_group, criterion_main};
use dependency_resolver::DependencyGraph;
use std::hint::black_box;

fn linear_chain(len: usize) -> (DependencyGraph, String) {
    assert!(len > 0);
    let mut graph = DependencyGraph::new();
    for i in 0..len {
        graph
            .add_task(&format!("n{i}"))
            .expect("for a new node to be added");
    }
    for i in 0..len.saturating_sub(1) {
        graph
            .add_dependency(&format!("n{i}"), &format!("n{}", i + 1))
            .expect("for a new dependency to be formed");
    }
    (graph, "n0".to_string())
}

fn layered_dag(layers: usize, width: usize) -> (DependencyGraph, String) {
    assert!(layers > 0);
    assert!(width > 0);
    let mut graph = DependencyGraph::new();
    graph
        .add_task("root")
        .expect("for the root node to be added");
    for l in 0..layers {
        for w in 0..width {
            graph
                .add_task(&format!("l{l}_w{w}"))
                .expect("for a new node to be added");
        }
    }

    for w in 0..width {
        graph
            .add_dependency("root", &format!("l0_w{w}"))
            .expect("for a dependency in root to be added");
    }

    for l in 0..layers.saturating_sub(1) {
        for w in 0..width {
            for nw in 0..width {
                graph
                    .add_dependency(&format!("l{l}_w{w}"), &format!("l{}_w{nw}", l + 1))
                    .expect("for a new dependency layered to be added");
            }
        }
    }

    (graph, "root".to_string())
}

fn bench_resolve_linear(c: &mut Criterion) {
    let mut group = c.benchmark_group("resolution_linear");
    for depth in [1_000usize, 10_000] {
        let (graph, root) = linear_chain(depth);
        group.bench_with_input(BenchmarkId::new("recursive", depth), &depth, |b, _| {
            b.iter(|| black_box(graph.resolve(&root).unwrap()));
        });
        group.bench_with_input(BenchmarkId::new("iterative", depth), &depth, |b, _| {
            b.iter(|| black_box(graph.resolve_iterative(&root).unwrap()));
        });
    }
}

fn bench_resolve_dag(c: &mut Criterion) {
    let mut group = c.benchmark_group("resolution_dag");
    for input in [(50, 20), (500, 20)] {
        let (graph, root) = layered_dag(input.0, input.1);
        group.bench_with_input(
            BenchmarkId::new("recursive", format!("l: {}, w: {}", input.0, input.1)),
            &input,
            |b, _| {
                b.iter(|| black_box(graph.resolve(&root).unwrap()));
            },
        );
        group.bench_with_input(
            BenchmarkId::new("iterative", format!("l: {}, w: {}", input.0, input.1)),
            &input,
            |b, _| {
                b.iter(|| black_box(graph.resolve_iterative(&root).unwrap()));
            },
        );
    }
}

criterion_group!(benches, bench_resolve_linear, bench_resolve_dag);
criterion_main!(benches);
