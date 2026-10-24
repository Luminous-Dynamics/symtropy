// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Workload runner for SEMANTIC-LOD-01.
//!
//! This benchmarks only the CPU allocation policy. It deliberately does not
//! label policy latency as frame time or GPU time.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{Duration, Instant};
use symtropy_render_bridge::{LodAllocation, LodCandidate, LodDemand, allocate_lod};

const DEFAULT_MANIFEST: &str = "benchmarks/render/SEMANTIC-LOD-01.json";
const DEFAULT_OUTPUT: &str = "target/semantic-lod-report.json";
const MIN_SAMPLES: usize = 30;

#[derive(Debug, Deserialize)]
struct Manifest {
    schema_version: String,
    fixture_id: String,
    screen_error_profiles: Vec<ScreenErrorProfile>,
    candidate_representations: Vec<CandidateSpec>,
    scaling_workloads: Vec<WorkloadSpec>,
}

#[derive(Debug, Deserialize)]
struct ScreenErrorProfile {
    profile_id: String,
    max_screen_error_px: f32,
}

#[derive(Debug, Deserialize)]
struct CandidateSpec {
    quality_rank: u16,
    representation_id_offset: u32,
    screen_error_px: f32,
    cost_units: u32,
}

#[derive(Debug, Deserialize)]
struct WorkloadSpec {
    workload_id: String,
    entity_count: usize,
    candidate_levels: usize,
    budget_fraction_of_finest_cost: f64,
    screen_error_profile_id: String,
    selected_entity_ids: Vec<u64>,
    critical_entity_ids: Vec<u64>,
    importance_rule: String,
}

#[derive(Debug, Serialize)]
struct BenchmarkReport {
    report_schema_version: String,
    status: String,
    fixture_id: String,
    fixture_sha256: String,
    cargo_lock_sha256: String,
    source_revision: String,
    rustc_version: String,
    os: String,
    architecture: String,
    samples_per_workload: usize,
    warmup_samples: usize,
    results: Vec<WorkloadResult>,
    claim_scope: Vec<String>,
}

type AllocationSignature = Vec<(u64, u32, u16, u32, bool)>;

#[derive(Debug, Serialize)]
struct WorkloadResult {
    workload_id: String,
    entity_count: usize,
    candidate_levels: usize,
    screen_error_profile_id: String,
    max_screen_error_px: f32,
    budget_fraction_of_finest_cost: f64,
    budget_units: u64,
    final_cost_units: u64,
    final_cost_fraction_of_finest: f64,
    quality_rank_histogram: BTreeMap<u16, usize>,
    degraded_entity_count: usize,
    protected_entity_count: usize,
    protected_demotion_count: usize,
    screen_error_violations: usize,
    weighted_screen_error_squared: f64,
    allocator_wall_time_p50_ns: u64,
    allocator_wall_time_p95_ns: u64,
    allocator_wall_time_p99_ns: u64,
    reference_scan_samples: usize,
    reference_scan_p50_ns: u64,
    reference_scan_p95_ns: u64,
    reference_scan_p99_ns: u64,
    heap_vs_reference_p50_speedup: f64,
    reference_matches_heap: bool,
    output_sha256: String,
    repeat_outputs_identical: bool,
}

#[derive(Debug)]
struct Options {
    manifest_path: PathBuf,
    output_path: PathBuf,
    samples: usize,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("symtropy-lod-benchmark: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let options = parse_args()?;
    if options.samples < MIN_SAMPLES {
        return Err(format!("--samples must be at least {MIN_SAMPLES}").into());
    }

    let manifest_bytes = fs::read(&options.manifest_path)?;
    let fixture_sha256 = sha256_hex(&manifest_bytes);
    let cargo_lock_bytes = fs::read("Cargo.lock")?;
    let cargo_lock_sha256 = sha256_hex(&cargo_lock_bytes);
    let manifest: Manifest = serde_json::from_slice(&manifest_bytes)?;
    validate_manifest(&manifest)?;

    let source_revision = command_text("git", &["rev-parse", "HEAD"])?;
    let rustc_version = command_text("rustc", &["--version"])?;

    let mut results = Vec::with_capacity(manifest.scaling_workloads.len());
    for workload in &manifest.scaling_workloads {
        let profile = manifest
            .screen_error_profiles
            .iter()
            .find(|profile| profile.profile_id == workload.screen_error_profile_id)
            .ok_or_else(|| {
                format!(
                    "workload {} references unknown screen-error profile {}",
                    workload.workload_id, workload.screen_error_profile_id
                )
            })?;
        results.push(run_workload(
            workload,
            profile,
            &manifest.candidate_representations,
            options.samples,
        )?);
    }

    let report = BenchmarkReport {
        report_schema_version: "1.0.0".to_owned(),
        status: "measured_policy_only_no_rendering".to_owned(),
        fixture_id: manifest.fixture_id,
        fixture_sha256,
        cargo_lock_sha256,
        source_revision,
        rustc_version,
        os: env::consts::OS.to_owned(),
        architecture: env::consts::ARCH.to_owned(),
        samples_per_workload: options.samples,
        warmup_samples: options.samples.clamp(5, 10),
        results,
        claim_scope: vec![
            "Measures CPU wall-clock time for the LOD allocation policy only.".to_owned(),
            "Does not measure Bevy frame time, GPU pass time, visual fidelity, or memory residency.".to_owned(),
            "Relative candidate cost units are policy inputs, not measured milliseconds.".to_owned(),
            "Screen-space error guarantees are conditional on the host-supplied estimates.".to_owned(),
            "No comparison with industry engines or SOTA is established by this report alone.".to_owned(),
        ],
    };

    let output = serde_json::to_vec_pretty(&report)?;
    if let Some(parent) = options
        .output_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }
    fs::write(&options.output_path, &output)?;
    println!("{}", String::from_utf8(output)?);
    Ok(())
}

fn parse_args() -> Result<Options, Box<dyn Error>> {
    let mut args = env::args().skip(1);
    let mut options = Options {
        manifest_path: PathBuf::from(DEFAULT_MANIFEST),
        output_path: PathBuf::from(DEFAULT_OUTPUT),
        samples: 30,
    };

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--manifest" => {
                options.manifest_path =
                    PathBuf::from(args.next().ok_or("--manifest requires a path")?);
            }
            "--output" => {
                options.output_path = PathBuf::from(args.next().ok_or("--output requires a path")?);
            }
            "--samples" => {
                options.samples = args
                    .next()
                    .ok_or("--samples requires a positive integer")?
                    .parse()?;
            }
            "--help" | "-h" => {
                println!(
                    "Usage: symtropy-lod-benchmark [--manifest PATH] [--output PATH] [--samples N]\n\
                     Runs a deterministic CPU-only LOD-policy benchmark; N must be at least 30."
                );
                std::process::exit(0);
            }
            other => return Err(format!("unknown argument {other:?}; use --help").into()),
        }
    }
    Ok(options)
}

fn validate_manifest(manifest: &Manifest) -> Result<(), Box<dyn Error>> {
    if manifest.schema_version != "1.1.0" {
        return Err(format!(
            "unsupported manifest schema version {:?}",
            manifest.schema_version
        )
        .into());
    }
    if manifest.fixture_id != "SEMANTIC-LOD-01" {
        return Err(format!("unexpected fixture ID {:?}", manifest.fixture_id).into());
    }
    if manifest.candidate_representations.len() < 2 {
        return Err("manifest needs at least two candidate representations".into());
    }
    if manifest.scaling_workloads.is_empty() {
        return Err("manifest contains no scaling workloads".into());
    }

    let mut previous: Option<&CandidateSpec> = None;
    let mut representation_offsets = std::collections::HashSet::new();
    for candidate in &manifest.candidate_representations {
        if candidate.cost_units == 0
            || !candidate.screen_error_px.is_finite()
            || candidate.screen_error_px < 0.0
            || candidate.representation_id_offset == 0
            || candidate.representation_id_offset >= 100
            || !representation_offsets.insert(candidate.representation_id_offset)
        {
            return Err("candidate costs/errors/representation offsets are invalid".into());
        }
        if previous.is_some_and(|prior| {
            candidate.quality_rank <= prior.quality_rank
                || candidate.cost_units <= prior.cost_units
                || candidate.screen_error_px > prior.screen_error_px
        }) {
            return Err("candidate levels must be ordered coarse-to-fine".into());
        }
        previous = Some(candidate);
    }

    for profile in &manifest.screen_error_profiles {
        if !profile.max_screen_error_px.is_finite() || profile.max_screen_error_px < 0.0 {
            return Err(format!("invalid error ceiling in profile {}", profile.profile_id).into());
        }
    }
    for workload in &manifest.scaling_workloads {
        if workload.entity_count == 0
            || workload.candidate_levels != manifest.candidate_representations.len()
            || !workload.budget_fraction_of_finest_cost.is_finite()
            || !(0.0..=1.0).contains(&workload.budget_fraction_of_finest_cost)
        {
            return Err(format!("invalid workload {}", workload.workload_id).into());
        }
        if workload.importance_rule != "1 + (entity_id modulo 10)" {
            return Err(format!("unsupported importance rule in {}", workload.workload_id).into());
        }
        let entity_count_u64 = u64::try_from(workload.entity_count).map_err(|_| {
            format!(
                "entity count cannot be represented for {}",
                workload.workload_id
            )
        })?;
        if workload
            .selected_entity_ids
            .iter()
            .any(|id| *id == 0 || *id > entity_count_u64)
            || workload
                .critical_entity_ids
                .iter()
                .any(|id| *id == 0 || *id > entity_count_u64)
        {
            return Err(format!("protected IDs outside workload {}", workload.workload_id).into());
        }
        let mut protected_ids = std::collections::HashSet::new();
        for id in &workload.selected_entity_ids {
            if !protected_ids.insert(*id) {
                return Err(format!(
                    "duplicate selected entity ID {id} in {}",
                    workload.workload_id
                )
                .into());
            }
        }
        for id in &workload.critical_entity_ids {
            if !protected_ids.insert(*id) {
                return Err(format!(
                    "duplicate/overlapping critical entity ID {id} in {}",
                    workload.workload_id
                )
                .into());
            }
        }
        let profile_exists = manifest
            .screen_error_profiles
            .iter()
            .any(|profile| profile.profile_id == workload.screen_error_profile_id);
        if !profile_exists {
            return Err(format!("unknown error profile in {}", workload.workload_id).into());
        }
    }
    Ok(())
}

fn run_workload(
    workload: &WorkloadSpec,
    profile: &ScreenErrorProfile,
    candidate_specs: &[CandidateSpec],
    samples: usize,
) -> Result<WorkloadResult, Box<dyn Error>> {
    let demands = build_demands(workload, candidate_specs)?;
    let finest_cost_per_entity = u64::from(
        candidate_specs
            .last()
            .ok_or("missing candidate levels")?
            .cost_units,
    );
    let finest_total = finest_cost_per_entity
        .checked_mul(workload.entity_count as u64)
        .ok_or("finest representation cost overflow")?;
    let budget_units =
        ((finest_total as f64) * workload.budget_fraction_of_finest_cost).floor() as u64;

    let warmup_samples = samples.clamp(5, 10);
    for _ in 0..warmup_samples {
        let allocation = allocate_lod(&demands, budget_units, profile.max_screen_error_px)
            .map_err(|error| format!("warm-up failed for {}: {error:?}", workload.workload_id))?;
        validate_allocation(
            &allocation,
            workload,
            candidate_specs,
            profile.max_screen_error_px,
            budget_units,
        )?;
    }

    let mut elapsed_ns = Vec::with_capacity(samples);
    let mut output_sha = None;
    let mut repeat_outputs_identical = true;
    let mut last_allocation: Option<LodAllocation> = None;

    for _ in 0..samples {
        let start = Instant::now();
        let allocation = allocate_lod(&demands, budget_units, profile.max_screen_error_px)
            .map_err(|error| {
                format!("allocation failed for {}: {error:?}", workload.workload_id)
            })?;
        let elapsed = start.elapsed();
        elapsed_ns.push(duration_nanos(elapsed)?);

        validate_allocation(
            &allocation,
            workload,
            candidate_specs,
            profile.max_screen_error_px,
            budget_units,
        )?;
        let digest = allocation_sha256(&allocation)?;
        if let Some(previous) = &output_sha {
            if previous != &digest {
                repeat_outputs_identical = false;
            }
        } else {
            output_sha = Some(digest);
        }
        if let Some(previous_allocation) = &last_allocation {
            if previous_allocation != &allocation {
                repeat_outputs_identical = false;
            }
        } else {
            last_allocation = Some(allocation);
        }
    }

    if !repeat_outputs_identical {
        return Err(format!(
            "allocation output was not deterministic for {}",
            workload.workload_id
        )
        .into());
    }

    let allocation = last_allocation.ok_or("benchmark generated no allocations")?;
    let mut quality_rank_histogram = BTreeMap::new();
    for choice in &allocation.choices {
        *quality_rank_histogram
            .entry(choice.quality_rank)
            .or_insert(0) += 1;
    }
    let protected_entity_count =
        workload.selected_entity_ids.len() + workload.critical_entity_ids.len();
    let protected_demotion_count = allocation
        .choices
        .iter()
        .filter(|choice| {
            (workload.selected_entity_ids.contains(&choice.entity_id)
                || workload.critical_entity_ids.contains(&choice.entity_id))
                && (!choice.protected
                    || choice.quality_rank
                        != candidate_specs
                            .last()
                            .map(|candidate| candidate.quality_rank)
                            .unwrap_or_default())
        })
        .count();
    let screen_error_violations = allocation
        .choices
        .iter()
        .filter(|choice| choice.screen_error_px > profile.max_screen_error_px)
        .count();

    let production_signature = allocation_signature(&allocation);
    let reference_scan_samples = if workload.entity_count <= 1_000 {
        samples
    } else {
        samples.min(3)
    };
    let mut reference_elapsed_ns = Vec::with_capacity(reference_scan_samples);
    let mut reference_signature: Option<AllocationSignature> = None;
    let mut reference_matches_heap = true;

    for _ in 0..reference_scan_samples {
        let start = Instant::now();
        let signature =
            reference_scan_signature(&demands, budget_units, profile.max_screen_error_px)?;
        reference_elapsed_ns.push(duration_nanos(start.elapsed())?);
        if let Some(previous) = &reference_signature {
            if previous != &signature {
                reference_matches_heap = false;
            }
        } else {
            reference_signature = Some(signature);
        }
    }

    if reference_signature.as_ref() != Some(&production_signature) {
        reference_matches_heap = false;
    }
    if !reference_matches_heap {
        return Err(format!(
            "heap allocator and independent scan reference differ for {}",
            workload.workload_id
        )
        .into());
    }

    let final_cost_fraction_of_finest = allocation.total_cost_units as f64 / finest_total as f64;
    elapsed_ns.sort_unstable();
    reference_elapsed_ns.sort_unstable();
    let allocator_p50_ns = percentile(&elapsed_ns, 0.50);
    let reference_scan_p50_ns = percentile(&reference_elapsed_ns, 0.50);
    let heap_vs_reference_p50_speedup = if allocator_p50_ns == 0 {
        0.0
    } else {
        reference_scan_p50_ns as f64 / allocator_p50_ns as f64
    };
    Ok(WorkloadResult {
        workload_id: workload.workload_id.clone(),
        entity_count: workload.entity_count,
        candidate_levels: workload.candidate_levels,
        screen_error_profile_id: profile.profile_id.clone(),
        max_screen_error_px: profile.max_screen_error_px,
        budget_fraction_of_finest_cost: workload.budget_fraction_of_finest_cost,
        budget_units,
        final_cost_units: allocation.total_cost_units,
        final_cost_fraction_of_finest,
        quality_rank_histogram,
        degraded_entity_count: allocation.degraded_entity_count,
        protected_entity_count,
        protected_demotion_count,
        screen_error_violations,
        weighted_screen_error_squared: allocation.total_weighted_error_squared,
        allocator_wall_time_p50_ns: allocator_p50_ns,
        allocator_wall_time_p95_ns: percentile(&elapsed_ns, 0.95),
        allocator_wall_time_p99_ns: percentile(&elapsed_ns, 0.99),
        reference_scan_samples,
        reference_scan_p50_ns,
        reference_scan_p95_ns: percentile(&reference_elapsed_ns, 0.95),
        reference_scan_p99_ns: percentile(&reference_elapsed_ns, 0.99),
        heap_vs_reference_p50_speedup,
        reference_matches_heap,
        output_sha256: output_sha.ok_or("no output digest recorded")?,
        repeat_outputs_identical,
    })
}

fn build_demands(
    workload: &WorkloadSpec,
    candidate_specs: &[CandidateSpec],
) -> Result<Vec<LodDemand>, Box<dyn Error>> {
    let mut demands = Vec::with_capacity(workload.entity_count);
    for entity_number in 1..=workload.entity_count {
        let entity_id = entity_number as u64;
        let mut candidates = Vec::with_capacity(candidate_specs.len());
        for spec in candidate_specs {
            let representation_id = entity_id
                .checked_mul(100)
                .and_then(|base| base.checked_add(u64::from(spec.representation_id_offset)))
                .and_then(|value| u32::try_from(value).ok())
                .ok_or("representation ID overflow")?;
            candidates.push(LodCandidate {
                quality_rank: spec.quality_rank,
                representation_id,
                screen_error_px: spec.screen_error_px,
                cost_units: spec.cost_units,
            });
        }

        demands.push(LodDemand {
            entity_id,
            importance: 1 + (entity_id % 10) as u32,
            selected: workload.selected_entity_ids.contains(&entity_id),
            critical: workload.critical_entity_ids.contains(&entity_id),
            candidates,
        });
    }
    Ok(demands)
}

fn validate_allocation(
    allocation: &LodAllocation,
    workload: &WorkloadSpec,
    candidates: &[CandidateSpec],
    max_screen_error_px: f32,
    budget_units: u64,
) -> Result<(), Box<dyn Error>> {
    if allocation.choices.len() != workload.entity_count {
        return Err(format!("choice count mismatch in {}", workload.workload_id).into());
    }
    if allocation.total_cost_units > budget_units {
        return Err(format!("budget exceeded in {}", workload.workload_id).into());
    }
    let computed_cost = allocation
        .choices
        .iter()
        .try_fold(0_u64, |total, choice| {
            total.checked_add(u64::from(choice.cost_units))
        })
        .ok_or("choice cost overflow")?;
    if computed_cost != allocation.total_cost_units {
        return Err(format!("cost ledger mismatch in {}", workload.workload_id).into());
    }
    let finest_rank = candidates
        .last()
        .ok_or("missing finest candidate")?
        .quality_rank;
    for choice in &allocation.choices {
        if choice.screen_error_px > max_screen_error_px {
            return Err(format!(
                "screen-error ceiling violated for entity {}",
                choice.entity_id
            )
            .into());
        }
        let protected = workload.selected_entity_ids.contains(&choice.entity_id)
            || workload.critical_entity_ids.contains(&choice.entity_id);
        if protected && (!choice.protected || choice.quality_rank != finest_rank) {
            return Err(
                format!("protected detail degraded for entity {}", choice.entity_id).into(),
            );
        }
    }
    if allocation
        .choices
        .windows(2)
        .any(|pair| pair[0].entity_id >= pair[1].entity_id)
    {
        return Err("allocation output is not strictly sorted by entity ID".into());
    }
    Ok(())
}

fn allocation_signature(allocation: &LodAllocation) -> AllocationSignature {
    allocation
        .choices
        .iter()
        .map(|choice| {
            (
                choice.entity_id,
                choice.representation_id,
                choice.quality_rank,
                choice.cost_units,
                choice.protected,
            )
        })
        .collect()
}

fn allocation_sha256(allocation: &LodAllocation) -> Result<String, Box<dyn Error>> {
    Ok(sha256_hex(&serde_json::to_vec(&allocation_signature(
        allocation,
    ))?))
}

fn reference_scan_signature(
    demands: &[LodDemand],
    budget_units: u64,
    max_screen_error_px: f32,
) -> Result<AllocationSignature, Box<dyn Error>> {
    let mut selected_indices: Vec<usize> = demands
        .iter()
        .map(|demand| demand.candidates.len().saturating_sub(1))
        .collect();
    let mut total_cost = demands
        .iter()
        .try_fold(0_u64, |total, demand| {
            let candidate = demand.candidates.last()?;
            total.checked_add(u64::from(candidate.cost_units))
        })
        .ok_or("reference cost overflow or empty candidate list")?;

    while total_cost > budget_units {
        let mut best: Option<(f64, u64, usize, u64)> = None;
        for (demand_index, demand) in demands.iter().enumerate() {
            let selected_index = selected_indices[demand_index];
            if demand.selected || demand.critical || selected_index == 0 {
                continue;
            }
            let current = demand.candidates[selected_index];
            let next = demand.candidates[selected_index - 1];
            if next.screen_error_px > max_screen_error_px {
                continue;
            }
            let saved_cost = u64::from(current.cost_units - next.cost_units);
            let current_error = f64::from(current.screen_error_px);
            let next_error = f64::from(next.screen_error_px);
            let added_error = f64::from(demand.importance)
                * (next_error * next_error - current_error * current_error);
            let ratio = added_error / saved_cost as f64;
            let replace = match best {
                None => true,
                Some((best_ratio, best_entity_id, _, _)) => {
                    ratio.total_cmp(&best_ratio).is_lt()
                        || (ratio.total_cmp(&best_ratio).is_eq()
                            && demand.entity_id < best_entity_id)
                }
            };
            if replace {
                best = Some((ratio, demand.entity_id, demand_index, saved_cost));
            }
        }

        let Some((_, _, demand_index, saved_cost)) = best else {
            return Err("reference scan found no feasible downgrade".into());
        };
        selected_indices[demand_index] -= 1;
        total_cost = total_cost
            .checked_sub(saved_cost)
            .ok_or("reference cost underflow")?;
    }

    let mut signature: AllocationSignature = demands
        .iter()
        .zip(selected_indices)
        .map(|(demand, selected_index)| {
            let candidate = demand.candidates[selected_index];
            (
                demand.entity_id,
                candidate.representation_id,
                candidate.quality_rank,
                candidate.cost_units,
                demand.selected || demand.critical,
            )
        })
        .collect();
    signature.sort_by_key(|entry| entry.0);
    Ok(signature)
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(digest.len() * 2);
    for byte in digest {
        output.push_str(&format!("{byte:02x}"));
    }
    output
}

fn command_text(command: &str, args: &[&str]) -> Result<String, Box<dyn Error>> {
    let output = Command::new(command).args(args).output()?;
    if !output.status.success() {
        return Err(format!("{command} {:?} exited with {}", args, output.status).into());
    }
    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}

fn duration_nanos(duration: Duration) -> Result<u64, Box<dyn Error>> {
    u64::try_from(duration.as_nanos())
        .map_err(|_| "duration does not fit in u64 nanoseconds".into())
}

fn percentile(sorted_values: &[u64], quantile: f64) -> u64 {
    if sorted_values.is_empty() {
        return 0;
    }
    let one_based_index = ((sorted_values.len() as f64) * quantile).ceil() as usize;
    sorted_values[one_based_index
        .saturating_sub(1)
        .min(sorted_values.len() - 1)]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percentile_uses_nearest_rank_convention() {
        let values = [10, 20, 30, 40, 50];
        assert_eq!(percentile(&values, 0.50), 30);
        assert_eq!(percentile(&values, 0.95), 50);
        assert_eq!(percentile(&values, 0.99), 50);
    }

    #[test]
    fn sha256_matches_known_answer() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn heap_allocator_matches_independent_scan_reference() {
        let candidate =
            |quality_rank, representation_id, screen_error_px, cost_units| LodCandidate {
                quality_rank,
                representation_id,
                screen_error_px,
                cost_units,
            };
        let demands = vec![
            LodDemand {
                entity_id: 1,
                importance: 1,
                selected: true,
                critical: false,
                candidates: vec![
                    candidate(0, 101, 4.0, 1),
                    candidate(1, 102, 2.0, 3),
                    candidate(2, 103, 0.5, 8),
                ],
            },
            LodDemand {
                entity_id: 2,
                importance: 2,
                selected: false,
                critical: false,
                candidates: vec![
                    candidate(0, 201, 4.0, 1),
                    candidate(1, 202, 2.0, 3),
                    candidate(2, 203, 0.5, 8),
                ],
            },
            LodDemand {
                entity_id: 3,
                importance: 8,
                selected: false,
                critical: true,
                candidates: vec![
                    candidate(0, 301, 4.0, 1),
                    candidate(1, 302, 2.0, 3),
                    candidate(2, 303, 0.5, 8),
                ],
            },
            LodDemand {
                entity_id: 4,
                importance: 3,
                selected: false,
                critical: false,
                candidates: vec![
                    candidate(0, 401, 4.0, 1),
                    candidate(1, 402, 2.0, 3),
                    candidate(2, 403, 0.5, 8),
                ],
            },
        ];
        let heap = allocate_lod(&demands, 22, 2.0).expect("policy allocation should be feasible");
        let reference = reference_scan_signature(&demands, 22, 2.0)
            .expect("reference allocation should be feasible");
        assert_eq!(allocation_signature(&heap), reference);
    }

    #[test]
    fn manifest_candidate_order_is_validated() {
        let invalid = Manifest {
            schema_version: "1.1.0".to_owned(),
            fixture_id: "SEMANTIC-LOD-01".to_owned(),
            screen_error_profiles: vec![],
            candidate_representations: vec![
                CandidateSpec {
                    quality_rank: 1,
                    representation_id_offset: 1,
                    screen_error_px: 2.0,
                    cost_units: 3,
                },
                CandidateSpec {
                    quality_rank: 0,
                    representation_id_offset: 2,
                    screen_error_px: 4.0,
                    cost_units: 1,
                },
            ],
            scaling_workloads: vec![],
        };
        assert!(validate_manifest(&invalid).is_err());
    }
}
