use std::time::Instant;

use serde::Serialize;
use symtropy_observation_frame_reference::{
    FrameIdentityV1, ModelTimeV1, ObservationEntityV1, ObservationEventV1,
    ObservationFrameInputV1, ObservationSourceClass, ObservationValueV1,
    ObservableV1, RuntimeSubjectRefV1, compile_observation_frame,
};

#[derive(Serialize)]
struct BenchmarkResult {
    profile: String,
    architecture: String,
    target_os: String,
    github_sha: String,
    entity_count: usize,
    observable_count: usize,
    event_count: usize,
    samples: usize,
    p50_us: u128,
    p95_us: u128,
    p99_us: u128,
    min_us: u128,
    max_us: u128,
    input_bytes: usize,
    input_bytes_per_entity_including_events: f64,
    compile_entities_per_second_at_p50: f64,
    event_records_per_second_at_p50: f64,
}

fn digest(character: char) -> String {
    std::iter::repeat_n(character, 64).collect()
}

fn synthetic_frame(entity_count: usize, event_count: usize) -> ObservationFrameInputV1 {
    let entities = (0..entity_count)
        .map(|index| {
            let scenario_entity_id = format!("bench/entity-{index:06}");
            ObservationEntityV1 {
                scenario_entity_id,
                claim_bearing: true,
                coordinate_frame_id: "bench/world-frame".to_string(),
                binding_receipt_sha256: Some(digest('a')),
                runtime_subject: Some(RuntimeSubjectRefV1 {
                    physical_authority_id: "bench-authority".to_string(),
                    world_generation_id: "bench-generation".to_string(),
                    net_id: index as u64 + 1,
                }),
                observables: vec![ObservableV1 {
                    observable_id: "position-x".to_string(),
                    quantity_id: "position.x".to_string(),
                    unit: "m".to_string(),
                    value: Some(ObservationValueV1::Binary64Bits(
                        "3ff0000000000000".to_string(),
                    )),
                    source_class: ObservationSourceClass::Synthetic,
                    source_ref_sha256: None,
                    source_time: None,
                    uncertainty: None,
                    unavailable_reason: None,
                }],
            }
        })
        .collect();

    let events = (0..event_count)
        .map(|index| ObservationEventV1 {
            sequence: index as u64 + 1,
            event_id: format!("bench/event-{index:06}"),
            event_kind: "physics_step_observed".to_string(),
            scenario_entity_id: Some(format!("bench/entity-{index:06}")),
            payload_sha256: digest('b'),
        })
        .collect();

    ObservationFrameInputV1 {
        schema_version: 1,
        identity: FrameIdentityV1 {
            scenario_id: "observation-frame-benchmark".to_string(),
            scenario_revision_sha256: digest('c'),
            run_id: format!("benchmark-{entity_count}"),
            source_revision: "fab29d69346c6ce6b7e4c4ed6d2d3ac81b5705a3".to_string(),
            branch_id: "synthetic-benchmark".to_string(),
            replay_identity_sha256: digest('d'),
            initial_state_sha256: digest('e'),
            simulation_state_sha256: digest('f'),
            model_profile_sha256: digest('1'),
            observable_set_sha256: digest('2'),
            adapter_version: "reference-oracle-bench-v1".to_string(),
            simulation_tick: 1,
        },
        model_time: ModelTimeV1 {
            clock_id: "benchmark-clock".to_string(),
            timebase_id: "nanosecond-ticks-v1".to_string(),
            ticks: 1,
            nanoseconds_per_tick: 1,
        },
        entities,
        event_stream: Some(events),
    }
}

fn percentile(samples: &[u128], percentile: usize) -> u128 {
    let rank = (samples.len() * percentile).div_ceil(100).max(1);
    samples[rank.saturating_sub(1).min(samples.len() - 1)]
}

fn run_profile(
    entity_count: usize,
    event_count: usize,
    samples: usize,
) -> Result<BenchmarkResult, Box<dyn std::error::Error>> {
    let input = synthetic_frame(entity_count, event_count);
    let input_bytes = serde_json::to_vec(&input)?.len();
    let mut timings = Vec::with_capacity(samples);

    for _ in 0..samples {
        let candidate = input.clone();
        let start = Instant::now();
        let receipt = compile_observation_frame(candidate)?;
        let elapsed = start.elapsed();
        if receipt.entity_count != entity_count || receipt.event_count != event_count {
            return Err("benchmark receipt count mismatch".into());
        }
        timings.push(elapsed.as_micros().max(1));
    }

    timings.sort_unstable();
    let p50 = percentile(&timings, 50);
    let entities_per_second = entity_count as f64 * 1_000_000.0 / p50 as f64;
    let events_per_second = event_count as f64 * 1_000_000.0 / p50 as f64;
    Ok(BenchmarkResult {
        profile: format!("entities-{entity_count}-events-{event_count}"),
        architecture: std::env::consts::ARCH.to_string(),
        target_os: std::env::consts::OS.to_string(),
        github_sha: std::env::var("GITHUB_SHA")
            .unwrap_or_else(|_| "unrecorded-local-run".to_string()),
        entity_count,
        observable_count: entity_count,
        event_count,
        samples,
        p50_us: p50,
        p95_us: percentile(&timings, 95),
        p99_us: percentile(&timings, 99),
        min_us: timings[0],
        max_us: timings[timings.len() - 1],
        input_bytes,
        input_bytes_per_entity_including_events: input_bytes as f64 / entity_count as f64,
        compile_entities_per_second_at_p50: entities_per_second,
        event_records_per_second_at_p50: events_per_second,
    })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let samples = std::env::args()
        .nth(1)
        .map(|value| value.parse::<usize>())
        .transpose()?
        .unwrap_or(5);
    if !(3..=20).contains(&samples) {
        return Err("sample count must be between 3 and 20".into());
    }

    for (entities, events) in [(1_000, 1_000), (10_000, 5_000), (100_000, 10_000)] {
        let result = run_profile(entities, events, samples)?;
        println!("{}", serde_json::to_string(&result)?);
    }
    Ok(())
}
