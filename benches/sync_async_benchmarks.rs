mod stock_send;
mod stock_listener;

use criterion::{criterion_group, criterion_main, Criterion};
use tokio::runtime::Runtime;
use stock_send::run_stock_send;
use stock_listener::{run_stock_listener, StockStore};
use tokio::sync::broadcast;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;



fn benchmark_stock_send(c: &mut Criterion) {
    // Benchmarking the stock sending process
    c.bench_function("stock_send", |b| {
        let rt = Runtime::new().unwrap();
        b.iter(|| {
            rt.block_on(async {
                run_stock_send().await.unwrap();
            });
        });
    });
}

fn benchmark_stock_listener(c: &mut Criterion) {
    // Benchmarking the stock listener process
    c.bench_function("stock_listener", |b| {
        let rt = Runtime::new().unwrap();
        b.iter(|| {
            let stock_store: StockStore = Arc::new(RwLock::new(HashMap::new()));
            let (tx, _rx) = broadcast::channel(100);
            rt.block_on(async {
                run_stock_listener(tx.clone(), stock_store.clone()).await.unwrap();
            });
        });
    });
}

fn criterion_benchmark(c: &mut Criterion) {
    benchmark_stock_send(c);
    benchmark_stock_listener(c);
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
