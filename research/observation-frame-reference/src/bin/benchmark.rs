use std::time::Instant;

use serde::Serialize;
use symtropy_observation_frame_reference::{
    compile_observation_frame, compute_observable_set_sha256, validate_borrowed_observation_view,
    BorrowedObservationViewV1, FrameIdentityV1, ModelTimeV1, ObservationBufferIdentityV1,
    ObservableV1, ObservationEntityV1, ObservationEventV1, ObservationFrameInputV1,
    ObservationSourceClass, ObservationValueV1, RuntimeSubjectRefV1,
};

#[derive(Serialize)]
struct BenchmarkResult {
    profile: String,
    architecture: String,
    target_os: String,
    runner_image_os: String,
    runner_image_version: String,
    cpu_model: String,
    rustc_version: String,
    github_sha: String,
    entity_count: usize,
    observables_per_entity: usize,
    observable_count: usize,
    event_count: usize,
    warmup_runs: usize,
    samples: usize,
    p50_ns: u128,
    p95_ns: u128,
    p99_ns: u128,
    min_ns: u128,
    max_ns: u128,
    invalid_reject_p50_ns: u128,
    invalid_reject_p95_ns: u128,
    invalid_reject_p99_ns: u128,
    input_bytes: usize,
    input_bytes_per_entity_including_events: f64,
    compile_entities_per_second_at_p50: f64,
    event_records_per_second_at_p50: f64,
    borrowed_guard_valid_p50_ns: u128,
    borrowed_guard_valid_p95_ns: u128,
    borrowed_guard_valid_p99_ns: u128,
    borrowed_guard_rebuild_reject_p50_ns: u128,
    borrowed_guard_rebuild_reject_p95_ns: u128,
    borrowed_guard_rebuild_reject_p99_ns: u128,
    owned_frame_dto_clone_p50_ns: u128,
    owned_frame_dto_clone_p95_ns: u128,
    owned_frame_dto_clone_p99_ns: u128,
}

fn digest(character: char) -> String {
    std::iter::repeat_n(character, 64).collect()
}

fn synthetic_frame(
    entity_count: usize,
    observables_per_entity: usize,
    event_count: usize,
) -> Result<ObservationFrameInputV1, Box<dyn std::error::Error>> {
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
                observables: (0..observables_per_entity)
                    .map(|channel| ObservableV1 {
                        observable_id: format!("channel-{channel:02}"),
                        quantity_id: format!("diagnostic.quantity-{channel:02}"),
                        unit: "1".to_string(),
                        value: Some(ObservationValueV1::Binary64Bits(
                            "3ff0000000000000".to_string(),
                        )),
                        source_class: ObservationSourceClass::Synthetic,
                        source_ref_sha256: None,
                        source_time: None,
                        uncertainty: None,
                        unavailable_reason: None,
                    })
                    .collect(),
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

    let observable_set_sha256 = compute_observable_set_sha256(&entities)?;
    Ok(ObservationFrameInputV1 {
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
            observable_set_sha256,
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
    })
}

fn percentile(samples: &[u128], percentile: usize) -> u128 {
    let rank = (samples.len() * percentile).div_ceil(100).max(1);
    samples[rank.saturating_sub(1).min(samples.len() - 1)]
}

fn run_profile(
    entity_count: usize,
    observables_per_entity: usize,
    event_count: usize,
    samples: usize,
) -> Result<BenchmarkResult, Box<dyn std::error::Error>> {
    let input = synthetic_frame(entity_count, observables_per_entity, event_count)?;
    let input_bytes = serde_json::to_vec(&input)?.len();
    let initial_receipt = compile_observation_frame(input.clone())?;
    let buffer_identity = ObservationBufferIdentityV1 {
        world_instance_id: "benchmark-world-instance".to_string(),
        world_generation_id: "benchmark-generation-0001".to_string(),
        rebuild_epoch: 1,
        frame_sha256: initial_receipt.frame_sha256,
        observable_set_sha256: input.identity.observable_set_sha256.clone(),
    };
    let borrowed_view = BorrowedObservationViewV1 {
        view_id: "benchmark-engineering-view".to_string(),
        source: buffer_identity.clone(),
    };
    let mut rebuilt_buffer = buffer_identity.clone();
    rebuilt_buffer.rebuild_epoch += 1;
    let mut hostile = input.clone();
    hostile.entities[0].observables[0].unit.push_str("-mismatch");

    const WARMUP_RUNS: usize = 2;
    for _ in 0..WARMUP_RUNS {
        let receipt = compile_observation_frame(input.clone())?;
        validate_borrowed_observation_view(&borrowed_view, &buffer_identity)?;
        if validate_borrowed_observation_view(&borrowed_view, &rebuilt_buffer)
            != Err(symtropy_observation_frame_reference::BorrowedViewError::RebuildEpochChanged)
        {
            return Err("borrowed view unexpectedly survived a rebuild".into());
        }
        if receipt.entity_count != entity_count || receipt.event_count != event_count {
            return Err("benchmark warm-up receipt count mismatch".into());
        }
        if compile_observation_frame(hostile.clone()).is_ok() {
            return Err("observable-set mismatch unexpectedly accepted during warm-up".into());
        }
    }

    let mut timings = Vec::with_capacity(samples);
    let mut reject_timings = Vec::with_capacity(samples);
    let mut valid_guard_timings = Vec::with_capacity(samples);
    let mut rebuild_guard_timings = Vec::with_capacity(samples);
    let mut owned_clone_timings = Vec::with_capacity(samples);

    for _ in 0..samples {
        let candidate = input.clone();
        let start = Instant::now();
        let receipt = compile_observation_frame(candidate)?;
        let elapsed = start.elapsed();
        if receipt.entity_count != entity_count || receipt.event_count != event_count {
            return Err("benchmark receipt count mismatch".into());
        }
        timings.push(elapsed.as_nanos().max(1));

        let hostile_candidate = hostile.clone();
        let reject_start = Instant::now();
        match compile_observation_frame(hostile_candidate) {
            Err(symtropy_observation_frame_reference::FrameError::ObservableSetDigestMismatch) => {}
            _ => {
                return Err(
                    "deep hostile fixture was not rejected by observable-set commitment".into(),
                );
            }
        }
        reject_timings.push(reject_start.elapsed().as_nanos().max(1));

        let guard_start = Instant::now();
        let valid_guard = validate_borrowed_observation_view(&borrowed_view, &buffer_identity);
        std::hint::black_box(&valid_guard);
        valid_guard?;
        valid_guard_timings.push(guard_start.elapsed().as_nanos().max(1));

        let rebuild_start = Instant::now();
        let rebuild_guard =
            validate_borrowed_observation_view(&borrowed_view, &rebuilt_buffer);
        if rebuild_guard
            != Err(symtropy_observation_frame_reference::BorrowedViewError::RebuildEpochChanged)
        {
            return Err("rebuild guard failed to reject stale view".into());
        }
        std::hint::black_box(&rebuild_guard);
        rebuild_guard_timings.push(rebuild_start.elapsed().as_nanos().max(1));

        let clone_start = Instant::now();
        let owned_copy = std::hint::black_box(input.clone());
        std::hint::black_box(owned_copy.entities.len());
        owned_clone_timings.push(clone_start.elapsed().as_nanos().max(1));
    }

    timings.sort_unstable();
    reject_timings.sort_unstable();
    valid_guard_timings.sort_unstable();
    rebuild_guard_timings.sort_unstable();
    owned_clone_timings.sort_unstable();
    let p50 = percentile(&timings, 50);
    let entities_per_second = entity_count as f64 * 1_000_000_000.0 / p50 as f64;
    let events_per_second = event_count as f64 * 1_000_000_000.0 / p50 as f64;
    Ok(BenchmarkResult {
        profile: format!(
            "entities-{entity_count}-observables-{observables_per_entity}-events-{event_count}"
        ),
        architecture: std::env::consts::ARCH.to_string(),
        target_os: std::env::consts::OS.to_string(),
        runner_image_os: std::env::var("RUNNER_IMAGE_OS")
            .unwrap_or_else(|_| "unrecorded".to_string()),
        runner_image_version: std::env::var("RUNNER_IMAGE_VERSION")
            .unwrap_or_else(|_| "unrecorded".to_string()),
        cpu_model: std::env::var("CPU_MODEL").unwrap_or_else(|_| "unrecorded".to_string()),
        rustc_version: std::env::var("RUSTC_VERSION").unwrap_or_else(|_| "unrecorded".to_string()),
        github_sha: std::env::var("GITHUB_SHA")
            .unwrap_or_else(|_| "unrecorded-local-run".to_string()),
        entity_count,
        observables_per_entity,
        observable_count: entity_count * observables_per_entity,
        event_count,
        warmup_runs: WARMUP_RUNS,
        samples,
        p50_ns: p50,
        p95_ns: percentile(&timings, 95),
        p99_ns: percentile(&timings, 99),
        min_ns: timings[0],
        max_ns: timings[timings.len() - 1],
        invalid_reject_p50_ns: percentile(&reject_timings, 50),
        invalid_reject_p95_ns: percentile(&reject_timings, 95),
        invalid_reject_p99_ns: percentile(&reject_timings, 99),
        input_bytes,
        input_bytes_per_entity_including_events: input_bytes as f64 / entity_count as f64,
        compile_entities_per_second_at_p50: entities_per_second,
        event_records_per_second_at_p50: events_per_second,
        borrowed_guard_valid_p50_ns: percentile(&valid_guard_timings, 50),
        borrowed_guard_valid_p95_ns: percentile(&valid_guard_timings, 95),
        borrowed_guard_valid_p99_ns: percentile(&valid_guard_timings, 99),
        borrowed_guard_rebuild_reject_p50_ns: percentile(&rebuild_guard_timings, 50),
        borrowed_guard_rebuild_reject_p95_ns: percentile(&rebuild_guard_timings, 95),
        borrowed_guard_rebuild_reject_p99_ns: percentile(&rebuild_guard_timings, 99),
        owned_frame_dto_clone_p50_ns: percentile(&owned_clone_timings, 50),
        owned_frame_dto_clone_p95_ns: percentile(&owned_clone_timings, 95),
        owned_frame_dto_clone_p99_ns: percentile(&owned_clone_timings, 99),
    })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let samples = std::env::args()
        .nth(1)
        .map(|value| value.parse::<usize>())
        .transpose()?
        .unwrap_or(1_000);
    if !(1_000..=2_000).contains(&samples) {
        return Err("sample count must be between 1000 and 2000 for meaningful p99 reporting".into());
    }

    for (entities, observables_per_entity, events) in
        [(1_000, 1, 1_000), (10_000, 4, 5_000), (100_000, 1, 10_000)]
    {
        let result = run_profile(entities, observables_per_entity, events, samples)?;
        println!("{}", serde_json::to_string(&result)?);
    }
    Ok(())
}
