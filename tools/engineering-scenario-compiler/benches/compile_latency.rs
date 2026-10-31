use engineering_scenario_compiler::{
    canonical_semantic_ir_bytes, compile_bytes, Disposition, COMPILER_VERSION,
    MAX_SOURCE_BYTES,
};
use std::hint::black_box;
use std::time::Instant;

const FIXTURE: &[u8] =
    include_bytes!("../../../benchmarks/aero/naca0012-inc-rans/case.toml");
const SAMPLE_COUNT: usize = 500;
const WARMUP_COUNT: usize = 50;
const INPUT_SIZES: [usize; 5] = [4 * 1024, 16 * 1024, 64 * 1024, 256 * 1024, 512 * 1024];

fn percentile(sorted: &[u128], percentile: usize) -> u128 {
    let rank = (sorted.len() * percentile).div_ceil(100).max(1);
    sorted[rank - 1]
}

fn padded_fixture(target_bytes: usize) -> Vec<u8> {
    assert!(target_bytes >= FIXTURE.len());
    assert!(target_bytes <= MAX_SOURCE_BYTES);
    let mut bytes = FIXTURE.to_vec();
    bytes.extend_from_slice(b"\n# deterministic benchmark padding ");
    bytes.resize(target_bytes, b'x');
    bytes
}

fn main() {
    eprintln!(
        "compiler={} target={}-{} logical_cpus={} samples={} warmup={} scope=compile_end_to_end_plus_separate_JCS_IR_canonicalization",
        COMPILER_VERSION,
        std::env::consts::ARCH,
        std::env::consts::OS,
        std::thread::available_parallelism().map_or(1, usize::from),
        SAMPLE_COUNT,
        WARMUP_COUNT
    );
    println!(
        "stage,input_bytes,output_bytes,identity_sha256,p50_ns,p95_ns,p99_ns,min_ns,max_ns,samples,disposition"
    );

    for target_bytes in INPUT_SIZES {
        let bytes = padded_fixture(target_bytes);
        let baseline = compile_bytes(black_box(&bytes));
        assert_eq!(baseline.disposition, Disposition::Blocked);
        let source_digest = baseline.source_sha256;

        for _ in 0..WARMUP_COUNT {
            let report = compile_bytes(black_box(&bytes));
            assert_eq!(report.disposition, Disposition::Blocked);
            black_box(report);
        }

        let mut samples = Vec::with_capacity(SAMPLE_COUNT);
        for _ in 0..SAMPLE_COUNT {
            let start = Instant::now();
            let report = compile_bytes(black_box(&bytes));
            let elapsed = start.elapsed().as_nanos();
            assert_eq!(report.disposition, Disposition::Blocked);
            samples.push(elapsed);
            black_box(report);
        }
        samples.sort_unstable();

        println!(
            "compile_end_to_end,{},{},{},{},{},{},{},{},{},blocked",
            bytes.len(),
            0,
            source_digest,
            percentile(&samples, 50),
            percentile(&samples, 95),
            percentile(&samples, 99),
            samples[0],
            samples[samples.len() - 1],
            SAMPLE_COUNT
        );
    }

    // Measure canonical serialization independently from parsing and typed
    // validation so JCS cost is visible rather than hidden in compile time.
    let baseline = compile_bytes(black_box(FIXTURE));
    assert_eq!(baseline.disposition, Disposition::Blocked);
    let scenario = baseline
        .scenario_ir
        .as_ref()
        .expect("frozen scenario compiles to typed IR");
    let canonical_example = canonical_semantic_ir_bytes(scenario)
        .expect("typed IR has valid JCS encoding");
    let canonical_output_bytes = canonical_example.len();
    let canonical_sha256 = baseline
        .semantic_ir_jcs_sha256
        .as_ref()
        .expect("compile report contains portable semantic digest")
        .clone();

    for _ in 0..WARMUP_COUNT {
        black_box(
            canonical_semantic_ir_bytes(black_box(scenario))
                .expect("JCS canonicalization succeeds"),
        );
    }

    let mut canonical_samples = Vec::with_capacity(SAMPLE_COUNT);
    for _ in 0..SAMPLE_COUNT {
        let start = Instant::now();
        let canonical = canonical_semantic_ir_bytes(black_box(scenario))
            .expect("JCS canonicalization succeeds");
        canonical_samples.push(start.elapsed().as_nanos());
        black_box(canonical);
    }
    canonical_samples.sort_unstable();
    println!(
        "canonical_ir_jcs,0,{},{},{},{},{},{},{},{},blocked",
        canonical_output_bytes,
        canonical_sha256,
        percentile(&canonical_samples, 50),
        percentile(&canonical_samples, 95),
        percentile(&canonical_samples, 99),
        canonical_samples[0],
        canonical_samples[canonical_samples.len() - 1],
        SAMPLE_COUNT
    );
}
