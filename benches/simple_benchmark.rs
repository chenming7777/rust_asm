use cpu_time::ProcessTime;
use criterion::{criterion_group, criterion_main, Criterion};
use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};
use tokio::sync::Mutex;
use std::thread;
use std::time::{Duration, Instant};
use systemstat::{Platform, System};
use tokio::runtime::Runtime;

// Define your stock-related structs and enums
#[derive(Clone, Debug)]
enum Phase {
    Bull,
    Bear,
}

#[derive(Debug)]
struct MemoryUsage {
    total_mb: u64,
    free_mb: u64,
    used_mb: u64,
    cpu_usage: f32,
}

#[derive(Debug)]
struct IOMetrics {
    total_bytes_sent: usize,
    total_bytes_received: usize,
    latencies: Vec<Duration>,
}

impl Default for IOMetrics {
    fn default() -> Self {
        Self {
            total_bytes_sent: 0,
            total_bytes_received: 0,
            latencies: Vec::new(),
        }
    }
}

#[derive(Clone, Debug)]
struct Stock {
    price: f64,
    last_price: f64,
    trend: f64,
    last_trend: f64,
    volatility: f64,
    demand: u32,
    supply: u32,
    quantity: u32,
    phase: Phase,
}

// Function to monitor system resources
fn start_resource_monitoring(
    stop_flag: Arc<AtomicBool>,
    memory_samples: Arc<Mutex<Vec<MemoryUsage>>>,
) {
    thread::spawn(move || {
        let sys = System::new();
        let mut prev_cpu = sys.cpu_load_aggregate().unwrap();

        while !stop_flag.load(Ordering::Relaxed) {
            thread::sleep(Duration::from_secs(1)); // Sleep before taking measurement

            // Memory Usage
            let memory = sys.memory().ok();
            let total_mb = memory.as_ref().map(|m| m.total.as_u64() / 1_048_576).unwrap_or(0);
            let free_mb = memory.as_ref().map(|m| m.free.as_u64() / 1_048_576).unwrap_or(0);
            let used_mb = total_mb - free_mb;

            // CPU Usage
            let cpu = prev_cpu.done().ok();
            let cpu_usage = cpu
                .map(|cpu| 100.0 - (cpu.idle * 100.0)) // Calculate usage as non-idle percentage
                .unwrap_or(0.0);

            // Store the sample
            let sample = MemoryUsage {
                total_mb,
                free_mb,
                used_mb,
                cpu_usage,
            };

            let mut samples = memory_samples.blocking_lock(); // Use blocking lock for the sync thread
            samples.push(sample);

            // Update CPU measurement for next iteration
            prev_cpu = sys.cpu_load_aggregate().unwrap();
        }
    });
}

async fn demand_supply_update_task(
    stocks: Arc<Mutex<HashMap<String, Stock>>>,
    max_iterations: usize,
    io_metrics: Arc<Mutex<IOMetrics>>,
) {
    let mut interval = tokio::time::interval(Duration::from_millis(10)); // Faster interval for benchmarking
    for _ in 0..max_iterations {
        let start_time = Instant::now(); // Start latency measurement

        interval.tick().await;

        let mut stocks = stocks.lock().await;
        for stock in stocks.values_mut() {
            let min_base_demand = (stock.quantity as f64 * 0.05).round() as u32;
            let max_demand = (stock.quantity as f64 * 0.5).round() as u32;

            if stock.demand < min_base_demand {
                stock.demand = min_base_demand;
            }

            if stock.demand > max_demand {
                stock.demand = max_demand;
            }

            match stock.phase {
                Phase::Bull => {
                    let boost = fastrand::u32(1..10);
                    stock.demand = (stock.demand as f64 * 1.05).round() as u32 + boost;
                }
                Phase::Bear => {
                    let reduction = fastrand::u32(1..10);
                    stock.demand = stock.demand.saturating_sub(reduction);
                }
            }

            stock.demand = stock.demand
                .max(min_base_demand)
                .min(max_demand);

            let min_base_supply = (stock.quantity as f64 * 0.03).round() as u32;
            let max_supply = (stock.quantity as f64 * 0.4).round() as u32;

            stock.supply = stock
                .supply
                .max(min_base_supply)
                .min(max_supply);
        }

        // Simulate I/O (bytes sent/received)
        let mut metrics = io_metrics.lock().await;
        metrics.total_bytes_sent += 1024; // Example: 1 KB sent
        metrics.total_bytes_received += 512; // Example: 512 bytes received

        // Measure and store latency
        let elapsed_time = start_time.elapsed();
        metrics.latencies.push(elapsed_time);
    }
}

fn benchmark_demand_supply_update_shared(c: &mut Criterion) {
    let mut initial_stocks: HashMap<String, Stock> = HashMap::new();
    for i in 0..1000 {
        let symbol = format!("STOCK{}", i);
        initial_stocks.insert(
            symbol,
            Stock {
                price: 100.0 + fastrand::f64() * 50.0,
                last_price: 100.0,
                trend: 0.0,
                last_trend: 0.0,
                volatility: 0.5,
                demand: 500,
                supply: 500,
                quantity: 1000,
                phase: if fastrand::bool() { Phase::Bull } else { Phase::Bear },
            },
        );
    }

    let shared_stocks = Arc::new(Mutex::new(initial_stocks));
    let max_iterations = 100;

    let stop_flag = Arc::new(AtomicBool::new(false));
    let memory_samples = Arc::new(Mutex::new(Vec::new()));
    start_resource_monitoring(stop_flag.clone(), Arc::clone(&memory_samples));

    let cpu_times = Arc::new(Mutex::new(Vec::new()));
    let io_metrics = Arc::new(Mutex::new(IOMetrics::default()));

    let mut group = c.benchmark_group("demand_supply_update_shared");
    group.warm_up_time(Duration::from_secs(1)); // Adjust warm-up time

    group.bench_function("update_task_shared", |b| {
        let rt = Runtime::new().unwrap();

        b.to_async(&rt).iter({
            let shared_stocks = Arc::clone(&shared_stocks);
            let cpu_times = Arc::clone(&cpu_times);
            let io_metrics = Arc::clone(&io_metrics);

            move || {
                let shared_stocks_inner = Arc::clone(&shared_stocks);
                let cpu_times_inner = Arc::clone(&cpu_times);
                let io_metrics_inner = Arc::clone(&io_metrics);

                async move {
                    let start_cpu = ProcessTime::now();

                    demand_supply_update_task(
                        Arc::clone(&shared_stocks_inner),
                        max_iterations,
                        Arc::clone(&io_metrics_inner),
                    )
                    .await;

                    let cpu_time = start_cpu.elapsed();
                    {
                        let mut times = cpu_times_inner.lock().await;
                        times.push(cpu_time);
                    }
                }
            }
        });
    });

    group.finish();

    stop_flag.store(true, Ordering::Relaxed);
    thread::sleep(Duration::from_secs(2));

    if let Ok(samples) = memory_samples.try_lock() {
        let total_samples = samples.len() as f64;
        if total_samples > 0.0 {
            let sum_used_mb: u64 = samples.iter().map(|s| s.used_mb).sum();
            let sum_cpu_usage: f32 = samples.iter().map(|s| s.cpu_usage).sum();

            println!("\n=== System Resource Metrics ===");
            println!(
                "Average Used Memory: {:.2} MB",
                sum_used_mb as f64 / total_samples
            );
            println!("Average CPU Usage: {:.2}%", (sum_cpu_usage as f64) / total_samples);
        }
    }

    if let Ok(times) = cpu_times.try_lock() {
        let total_samples = times.len() as f64;
        let sum_cpu_nanos: u128 = times.iter().map(|&t| t.as_nanos()).sum();

        println!("\n=== CPU Time ===");
        println!(
            "Average CPU Time: {:.3} ms",
            sum_cpu_nanos as f64 / total_samples / 1_000_000.0
        );
    }

    if let Ok(metrics) = io_metrics.try_lock() {
        let total_latency: Duration = metrics.latencies.iter().sum();
        let avg_latency = if !metrics.latencies.is_empty() {
            total_latency / metrics.latencies.len() as u32
        } else {
            Duration::ZERO
        };

        println!("\n=== I/O and Latency Metrics ===");
        println!("Total Bytes Sent: {}", metrics.total_bytes_sent);
        println!("Total Bytes Received: {}", metrics.total_bytes_received);
        println!(
            "Average Latency: {:.3} ms",
            avg_latency.as_secs_f64() * 1000.0
        );
    };
}

criterion_group!(benches, benchmark_demand_supply_update_shared);
criterion_main!(benches);