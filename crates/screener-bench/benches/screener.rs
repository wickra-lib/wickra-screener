//! Criterion benchmarks for `scan_batch`: how the batch scan scales with the
//! universe size (100 / 1k / 10k symbols) and the indicator count (5 / 20). The
//! same benchmark, run with and without the `parallel` feature, measures the
//! rayon path against the sequential one.

use std::collections::BTreeMap;
use std::hint::black_box;

use criterion::{criterion_group, criterion_main, BatchSize, BenchmarkId, Criterion, Throughput};
use screener_core::{scan_batch, Candle, Condition, Expr, ScanSpec, SymbolInput};

const BARS: usize = 60;
const INDICATORS: [&str; 4] = ["Sma", "Ema", "Rsi", "Roc"];

/// A synthetic universe of `symbols` symbols, each a deterministic sine path.
fn universe(symbols: usize) -> BTreeMap<String, SymbolInput> {
    let mut data = BTreeMap::new();
    for s in 0..symbols {
        let phase = f64::from(u32::try_from(s % 360).unwrap());
        let candles = (0..BARS)
            .map(|i| {
                let t = f64::from(u32::try_from(i).unwrap());
                let close = 100.0 + 10.0 * ((t + phase) / 8.0).sin() + 0.1 * t;
                Candle {
                    time: i64::try_from(i).unwrap(),
                    open: close,
                    high: close,
                    low: close,
                    close,
                    volume: 1000.0,
                }
            })
            .collect::<Vec<Candle>>();
        data.insert(format!("s{s:05}"), candles.into());
    }
    data
}

/// A spec whose condition references `count` distinct indicators (so the fold
/// touches `count` indicators per bar). The threshold is permissive so the scan
/// exercises matching + serialization too.
fn spec(universe: &BTreeMap<String, SymbolInput>, count: usize) -> ScanSpec {
    let conditions = (0..count)
        .map(|i| {
            let name = INDICATORS[i % INDICATORS.len()];
            let period = f64::from(u32::try_from(5 + i).unwrap());
            Condition::Cmp {
                left: Expr::Indicator {
                    name: name.to_string(),
                    params: vec![period],
                    field: None,
                },
                op: screener_core::Comparator::Gt,
                right: Expr::Const { value: -1.0e9 },
            }
        })
        .collect();
    ScanSpec {
        universe: universe.keys().cloned().collect(),
        timeframe: None,
        condition: Condition::All { conditions },
        rank: None,
        limit: None,
    }
}

fn bench_scan(c: &mut Criterion) {
    let mut group = c.benchmark_group("scan_batch");
    group.sample_size(10);
    for &symbols in &[100usize, 1_000, 10_000] {
        let data = universe(symbols);
        for &indicators in &[5usize, 20] {
            let spec = spec(&data, indicators);
            group.throughput(Throughput::Elements(u64::try_from(symbols).unwrap()));
            group.bench_with_input(
                BenchmarkId::from_parameter(format!("{symbols}sym_{indicators}ind")),
                &(&data, &spec),
                |b, (data, spec)| {
                    // A scan owns the series it folds, so each iteration needs its
                    // own copy. `iter_batched` builds that copy outside the timed
                    // region, leaving the measurement on the scan itself.
                    b.iter_batched(
                        || (*data).clone(),
                        |owned| black_box(scan_batch(owned, black_box(spec)).unwrap()),
                        BatchSize::LargeInput,
                    );
                },
            );
        }
    }
    group.finish();
}

criterion_group!(benches, bench_scan);
criterion_main!(benches);
