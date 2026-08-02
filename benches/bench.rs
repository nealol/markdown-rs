use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use markdown::{OpenMetrics, Options, Renderer};
use serde_json::json;
use std::alloc::{GlobalAlloc, Layout, System};
use std::fs;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

struct CountingAllocator;

static ALLOCATION_COUNT: AtomicU64 = AtomicU64::new(0);
static ALLOCATED_BYTES: AtomicU64 = AtomicU64::new(0);
static LIVE_BYTES: AtomicU64 = AtomicU64::new(0);
static PEAK_BYTES: AtomicU64 = AtomicU64::new(0);

fn record_live_bytes(value: u64) {
    let mut peak = PEAK_BYTES.load(Ordering::Relaxed);
    while value > peak {
        match PEAK_BYTES.compare_exchange_weak(peak, value, Ordering::Relaxed, Ordering::Relaxed) {
            Ok(_) => break,
            Err(actual) => peak = actual,
        }
    }
}

unsafe impl GlobalAlloc for CountingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        ALLOCATION_COUNT.fetch_add(1, Ordering::Relaxed);
        ALLOCATED_BYTES.fetch_add(layout.size() as u64, Ordering::Relaxed);
        let live =
            LIVE_BYTES.fetch_add(layout.size() as u64, Ordering::Relaxed) + layout.size() as u64;
        record_live_bytes(live);
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        LIVE_BYTES.fetch_sub(layout.size() as u64, Ordering::Relaxed);
        unsafe { System.dealloc(ptr, layout) };
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        ALLOCATION_COUNT.fetch_add(1, Ordering::Relaxed);
        ALLOCATED_BYTES.fetch_add(new_size as u64, Ordering::Relaxed);
        let live = if new_size >= layout.size() {
            LIVE_BYTES.fetch_add((new_size - layout.size()) as u64, Ordering::Relaxed)
                + (new_size - layout.size()) as u64
        } else {
            LIVE_BYTES.fetch_sub((layout.size() - new_size) as u64, Ordering::Relaxed)
                - (layout.size() - new_size) as u64
        };
        record_live_bytes(live);
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

#[global_allocator]
static GLOBAL: CountingAllocator = CountingAllocator;

fn readme(c: &mut Criterion) {
    let doc = fs::read_to_string("readme.md").unwrap();

    c.bench_with_input(BenchmarkId::new("readme", "readme"), &doc, |b, s| {
        b.iter(|| markdown::to_html(black_box(s)));
    });
}

fn fixture(blocks: usize) -> String {
    let mut source = String::with_capacity(blocks * 96);
    for index in 0..blocks {
        if index > 0 {
            source.push_str("\n\n");
        }
        source.push_str("Paragraph ");
        source.push_str(&index.to_string());
        source.push_str(" has **strong text**, *emphasis*, and a [link](/target).");
    }
    source
}

fn percentile(samples: &mut [u64], percentile: usize) -> u64 {
    samples.sort_unstable();
    let index = (samples.len() - 1) * percentile / 100;
    samples[index]
}

fn metric_summary(mut values: Vec<u64>) -> serde_json::Value {
    let median = percentile(&mut values, 50);
    let p95 = percentile(&mut values, 95);
    json!({"medianNanoseconds": median, "p95Nanoseconds": p95})
}

fn count_summary(mut values: Vec<u64>) -> serde_json::Value {
    let median = percentile(&mut values, 50);
    let p95 = percentile(&mut values, 95);
    json!({"median": median, "p95": p95})
}

fn measure_lower_fixture(name: &str, source: &str, samples: usize) -> serde_json::Value {
    let mut timings = Vec::with_capacity(samples);
    let mut allocations = Vec::with_capacity(samples);
    let mut allocated_bytes = Vec::with_capacity(samples);
    let mut peak_bytes = Vec::with_capacity(samples);
    for _ in 0..samples {
        let count_before = ALLOCATION_COUNT.load(Ordering::Relaxed);
        let bytes_before = ALLOCATED_BYTES.load(Ordering::Relaxed);
        let live_before = LIVE_BYTES.load(Ordering::Relaxed);
        PEAK_BYTES.store(live_before, Ordering::Relaxed);
        let started = Instant::now();
        black_box(Renderer::open(source, Options::default()).unwrap());
        timings.push(started.elapsed().as_nanos() as u64);
        allocations.push(
            ALLOCATION_COUNT
                .load(Ordering::Relaxed)
                .saturating_sub(count_before),
        );
        allocated_bytes.push(
            ALLOCATED_BYTES
                .load(Ordering::Relaxed)
                .saturating_sub(bytes_before),
        );
        peak_bytes.push(
            PEAK_BYTES
                .load(Ordering::Relaxed)
                .saturating_sub(live_before),
        );
    }
    let origin = Instant::now();
    let (_, metrics) = Renderer::open_measured(source, Options::default(), || {
        origin.elapsed().as_nanos() as u64
    })
    .unwrap();
    json!({
        "name": name,
        "sourceBytes": source.len(),
        "samples": samples,
        "time": metric_summary(timings),
        "allocationCount": count_summary(allocations),
        "allocatedBytes": count_summary(allocated_bytes),
        "peakBytes": count_summary(peak_bytes),
        "eventCount": metrics.event_count,
        "blockCount": metrics.block_count,
        "blockMetadataBytes": metrics.block_metadata_bytes,
        "parserInvocations": metrics.parser_invocations
    })
}

fn cold_renderer(c: &mut Criterion) {
    const BLOCK_COUNTS: [usize; 5] = [1, 5, 25, 200, 600];
    const SAMPLES: usize = 100;
    let mut reports = Vec::with_capacity(BLOCK_COUNTS.len());

    for blocks in BLOCK_COUNTS {
        let source = fixture(blocks);
        let mut direct_times = Vec::with_capacity(SAMPLES);
        let mut open_times = Vec::with_capacity(SAMPLES);
        let mut parse_blocks = Vec::with_capacity(SAMPLES);
        let mut canonical_html = Vec::with_capacity(SAMPLES);
        let mut checkpoints = Vec::with_capacity(SAMPLES);
        let mut open_allocations = Vec::with_capacity(SAMPLES);
        let mut open_bytes = Vec::with_capacity(SAMPLES);
        let mut direct_allocations = Vec::with_capacity(SAMPLES);
        let mut direct_bytes = Vec::with_capacity(SAMPLES);
        let mut last_metrics = OpenMetrics::default();

        for _ in 0..SAMPLES {
            let count_before = ALLOCATION_COUNT.load(Ordering::Relaxed);
            let bytes_before = ALLOCATED_BYTES.load(Ordering::Relaxed);
            let started = Instant::now();
            black_box(markdown::to_html_with_options(
                black_box(&source),
                black_box(&Options::default()),
            ))
            .unwrap();
            direct_times.push(started.elapsed().as_nanos() as u64);
            direct_allocations.push(
                ALLOCATION_COUNT
                    .load(Ordering::Relaxed)
                    .saturating_sub(count_before),
            );
            direct_bytes.push(
                ALLOCATED_BYTES
                    .load(Ordering::Relaxed)
                    .saturating_sub(bytes_before),
            );

            let count_before = ALLOCATION_COUNT.load(Ordering::Relaxed);
            let bytes_before = ALLOCATED_BYTES.load(Ordering::Relaxed);
            let started = Instant::now();
            let clock_origin = Instant::now();
            let (renderer, metrics) =
                Renderer::open_measured(black_box(&source), Options::default(), || {
                    clock_origin.elapsed().as_nanos() as u64
                })
                .unwrap();
            black_box(renderer);
            open_times.push(started.elapsed().as_nanos() as u64);
            parse_blocks.push(metrics.parse_blocks);
            canonical_html.push(metrics.canonical_html);
            checkpoints.push(metrics.checkpoint_construction);
            open_allocations.push(
                ALLOCATION_COUNT
                    .load(Ordering::Relaxed)
                    .saturating_sub(count_before),
            );
            open_bytes.push(
                ALLOCATED_BYTES
                    .load(Ordering::Relaxed)
                    .saturating_sub(bytes_before),
            );
            last_metrics = metrics;
        }

        reports.push(json!({
            "topLevelBlocks": blocks,
            "sourceBytes": source.len(),
            "samples": SAMPLES,
            "rendererOpen": metric_summary(open_times),
            "directToHtmlWithOptions": metric_summary(direct_times),
            "stages": {
                "parseBlocks": metric_summary(parse_blocks),
                "renderAll": {"medianNanoseconds": 0, "p95Nanoseconds": 0},
                "canonicalToHtmlWithOptions": metric_summary(canonical_html),
                "segmentedCanonicalComparison": {"medianNanoseconds": 0, "p95Nanoseconds": 0},
                "checkpointConstruction": metric_summary(checkpoints),
                "finalHtmlAssembly": {"medianNanoseconds": 0, "p95Nanoseconds": 0}
            },
            "parserInvocations": last_metrics.parser_invocations,
            "allocations": {
                "rendererOpen": count_summary(open_allocations),
                "directToHtmlWithOptions": count_summary(direct_allocations)
            },
            "allocatedBytes": {
                "rendererOpen": count_summary(open_bytes),
                "directToHtmlWithOptions": count_summary(direct_bytes)
            }
        }));

        let mut group = c.benchmark_group("cold_renderer_open");
        group.bench_with_input(
            BenchmarkId::new("renderer_open", blocks),
            &source,
            |b, source| {
                b.iter(|| Renderer::open(black_box(source), Options::default()).unwrap());
            },
        );
        group.bench_with_input(
            BenchmarkId::new("direct_to_html", blocks),
            &source,
            |b, source| {
                b.iter(|| {
                    markdown::to_html_with_options(black_box(source), &Options::default()).unwrap()
                });
            },
        );
        group.finish();
    }

    let repeated_references = format!(
        "{}\n\n[shared]: /target\n",
        "[label][shared]\n\n".repeat(1_000)
    );
    let mut unique_definitions = String::new();
    for index in 0..1_000 {
        unique_definitions.push_str(&format!("[label-{index}]: /target-{index}\n"));
    }
    let headings = (0..1_000)
        .map(|index| format!("# Heading {index}\n\n"))
        .collect::<String>();
    let dense_inline =
        "Paragraph with **strong**, *emphasis*, [link](/target), and `code`.\n\n".repeat(1_000);
    let plain_paragraphs = "A short plain paragraph.\n\n".repeat(1_000);
    let mut ordinary = fixture(300);
    while ordinary.len() < 23_930 {
        ordinary
            .push_str("\n\nA plain benchmark paragraph with enough text to reach the target size.");
    }
    let large_offsets = format!("{}\n\n# Tail heading\n", "a".repeat(4 * 1024 * 1024));
    let lower_level = vec![
        measure_lower_fixture("repeated_reference_links", &repeated_references, 20),
        measure_lower_fixture("unique_definitions", &unique_definitions, 20),
        measure_lower_fixture("plain_paragraphs", &plain_paragraphs, 20),
        measure_lower_fixture("headings", &headings, 20),
        measure_lower_fixture("dense_inline", &dense_inline, 20),
        measure_lower_fixture("ordinary_23930_bytes", &ordinary, 20),
        measure_lower_fixture("large_source_offsets", &large_offsets, 5),
    ];

    println!(
        "{}",
        serde_json::to_string_pretty(&json!({
            "schemaVersion": 1,
            "scenario": "cold_renderer_open",
            "clock": "monotonic_nanoseconds",
            "reports": reports,
            "lowerLevelFixtures": lower_level
        }))
        .unwrap()
    );
}

criterion_group!(benches, readme, cold_renderer);
criterion_main!(benches);
