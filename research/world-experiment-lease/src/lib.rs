//! Read-only, evidence-bound leases over one immutable experiment observation.
//!
//! This crate composes already-selected physics and render profiles. It does not
//! execute either engine, authenticate upstream receipts, or authorize actuation.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const SCHEMA_VERSION: u32 = 1;
const CLAIM_CEILING: &str =
    "finite-read-only-lease-conformance-only;upstream-receipts-referenced-not-authenticated;no-physical-truth-or-actuation-authority";
const HASH_DOMAIN: &[u8] = b"symtropy-read-only-experiment-lease-v1\0";
const USAGE_HASH_DOMAIN: &[u8] = b"symtropy-read-only-usage-snapshot-v1\0";

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LeaseInputV1 {
    pub schema_version: u32,
    pub experiment: ExperimentIdentity,
    pub physics: PhysicsAdmissionRef,
    pub views: Vec<ViewAdmissionRef>,
    pub budget: LeaseBudget,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ExperimentIdentity {
    pub experiment_id: String,
    pub run_id: String,
    pub scenario_manifest_sha256: String,
    /// Full lowercase Git commit SHA for the exact simulation source.
    pub source_revision: String,
    pub initial_state_sha256: String,
    pub simulation_state_sha256: String,
    pub event_order_sha256: String,
    pub observation_frame_sha256: String,
    pub simulation_tick: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PhysicsAdmissionRef {
    pub policy_input_sha256: String,
    pub policy_receipt_sha256: String,
    pub selected_model_id: String,
    pub model_passport_sha256: String,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskMode {
    Cinematic,
    EngineeringInspection,
    SafetyInspection,
    Accessibility,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ViewAdmissionRef {
    pub view_id: String,
    pub task_mode: TaskMode,
    pub observation_frame_sha256: String,
    pub policy_input_sha256: String,
    pub policy_receipt_sha256: String,
    pub selected_profile_id: String,
    pub measurement_receipt_sha256: String,
    /// Copied from the referenced render-policy input; authenticity is upstream.
    pub max_p95_frame_time_us: u64,
    pub max_p99_frame_time_us: u64,
    pub max_peak_vram_bytes: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LeaseBudget {
    /// V0.1 is deliberately a single-snapshot lease, not an advancing run lease.
    pub valid_from_tick: u64,
    pub valid_through_tick: u64,
    pub max_distinct_views: u32,
    pub max_view_events: u64,
    pub max_rendered_frames: u64,
    pub max_peak_system_memory_bytes: u64,
    pub max_peak_vram_bytes: u64,
    pub max_gpu_energy_uj_total: Option<u64>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LeaseReceiptV1 {
    pub schema_version: u32,
    pub input_sha256: String,
    pub lease_sha256: String,
    pub experiment: ExperimentIdentity,
    pub physics: PhysicsAdmissionRef,
    pub views: Vec<ViewAdmissionRef>,
    pub budget: LeaseBudget,
    pub claim_ceiling: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct UsageSnapshotV1 {
    pub lease_sha256: String,
    /// Digest of the immediately preceding accepted usage snapshot; absent only at sequence 1.
    pub previous_usage_sha256: Option<String>,
    pub run_id: String,
    pub simulation_state_sha256: String,
    pub event_order_sha256: String,
    pub observation_frame_sha256: String,
    pub simulation_tick: u64,
    /// Starts at 1 and advances by exactly one for each cumulative snapshot.
    pub sequence: u64,
    pub view_event_count: u64,
    pub rendered_frames_total: u64,
    pub peak_system_memory_bytes: u64,
    /// Lease-wide integrated GPU-board energy; do not sum potentially overlapping view windows.
    pub gpu_energy_uj_total: Option<u64>,
    /// Receipt binding the lease-wide energy measurement, when present.
    pub gpu_energy_receipt_sha256: Option<String>,
    pub views: Vec<ViewUsageV1>,
    /// Any physical input/intervention invalidates this read-only lease.
    pub physical_mutation_attempted: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ViewUsageV1 {
    pub view_id: String,
    /// Cumulative totals for this view in this lease.
    pub view_events: u64,
    pub rendered_frames: u64,
    pub p95_frame_time_us: u64,
    pub p99_frame_time_us: u64,
    pub peak_vram_bytes: u64,
    /// Receipt binding the runtime frame-time and VRAM observations for this view.
    pub measurement_receipt_sha256: String,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    PhysicalMutationAttempted,
    ViewEventBudgetExceeded,
    RenderedFrameBudgetExceeded,
    SystemMemoryBudgetExceeded,
    GpuMemoryBudgetExceeded,
    FrameTimeBudgetExceeded,
    GpuEnergyBudgetExceeded,
    RequiredEnergyMissing,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "outcome", rename_all = "snake_case")]
pub enum UsageDecision {
    Continue {
        lease_sha256: String,
        sequence: u64,
        usage_sha256: String,
    },
    Stop {
        lease_sha256: String,
        sequence: u64,
        usage_sha256: String,
        reason: StopReason,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LeaseError {
    UnsupportedSchema(u32),
    EmptyField(&'static str),
    InvalidSha256(&'static str),
    InvalidSourceRevision,
    InvalidTickWindow,
    ZeroBudget(&'static str),
    EmptyViews,
    TooManyViews,
    DuplicateViewId(String),
    ObservationFrameMismatch(String),
    RenderBudgetInvalid(String),
    CounterOverflow,
    Serialization(String),
    ReceiptMismatch,
    UsageLeaseMismatch,
    UsageScopeMismatch,
    UsageTickMismatch,
    UnknownView(String),
    DuplicateUsageView(String),
    InvalidUsageCounters,
    NonMonotonicUsage,
    UsageChainMismatch,
    InconsistentUsageTotals,
    PreviousSnapshotStopped,
}

impl std::fmt::Display for LeaseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for LeaseError {}

fn is_lower_hex(value: &str, expected_len: usize) -> bool {
    value.len() == expected_len
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn require_text(value: &str, field: &'static str) -> Result<(), LeaseError> {
    if value.trim().is_empty() {
        return Err(LeaseError::EmptyField(field));
    }
    Ok(())
}

fn require_sha(value: &str, field: &'static str) -> Result<(), LeaseError> {
    if !is_lower_hex(value, 64) {
        return Err(LeaseError::InvalidSha256(field));
    }
    Ok(())
}

fn validate_input(input: &LeaseInputV1) -> Result<(), LeaseError> {
    if input.schema_version != SCHEMA_VERSION {
        return Err(LeaseError::UnsupportedSchema(input.schema_version));
    }

    for (value, field) in [
        (input.experiment.experiment_id.as_str(), "experiment_id"),
        (input.experiment.run_id.as_str(), "run_id"),
        (input.physics.selected_model_id.as_str(), "selected_model_id"),
    ] {
        require_text(value, field)?;
    }

    if !is_lower_hex(&input.experiment.source_revision, 40) {
        return Err(LeaseError::InvalidSourceRevision);
    }

    for (value, field) in [
        (&input.experiment.scenario_manifest_sha256, "scenario_manifest_sha256"),
        (&input.experiment.initial_state_sha256, "initial_state_sha256"),
        (&input.experiment.simulation_state_sha256, "simulation_state_sha256"),
        (&input.experiment.event_order_sha256, "event_order_sha256"),
        (&input.experiment.observation_frame_sha256, "observation_frame_sha256"),
        (&input.physics.policy_input_sha256, "physics.policy_input_sha256"),
        (&input.physics.policy_receipt_sha256, "physics.policy_receipt_sha256"),
        (&input.physics.model_passport_sha256, "physics.model_passport_sha256"),
    ] {
        require_sha(value, field)?;
    }

    if input.views.is_empty() {
        return Err(LeaseError::EmptyViews);
    }
    if input.budget.valid_from_tick != input.experiment.simulation_tick
        || input.budget.valid_through_tick != input.experiment.simulation_tick
    {
        return Err(LeaseError::InvalidTickWindow);
    }

    for (value, field) in [
        (input.budget.max_distinct_views as u64, "max_distinct_views"),
        (input.budget.max_view_events, "max_view_events"),
        (input.budget.max_rendered_frames, "max_rendered_frames"),
        (input.budget.max_peak_system_memory_bytes, "max_peak_system_memory_bytes"),
        (input.budget.max_peak_vram_bytes, "max_peak_vram_bytes"),
    ] {
        if value == 0 {
            return Err(LeaseError::ZeroBudget(field));
        }
    }
    if input.views.len() > input.budget.max_distinct_views as usize {
        return Err(LeaseError::TooManyViews);
    }
    if input.budget.max_gpu_energy_uj_total == Some(0) {
        return Err(LeaseError::ZeroBudget("max_gpu_energy_uj_total"));
    }

    let mut seen = BTreeSet::new();
    for view in &input.views {
        require_text(&view.view_id, "view_id")?;
        require_text(&view.selected_profile_id, "selected_profile_id")?;
        for (value, field) in [
            (&view.observation_frame_sha256, "view.observation_frame_sha256"),
            (&view.policy_input_sha256, "view.policy_input_sha256"),
            (&view.policy_receipt_sha256, "view.policy_receipt_sha256"),
            (&view.measurement_receipt_sha256, "view.measurement_receipt_sha256"),
        ] {
            require_sha(value, field)?;
        }
        if view.observation_frame_sha256 != input.experiment.observation_frame_sha256 {
            return Err(LeaseError::ObservationFrameMismatch(view.view_id.clone()));
        }
        if view.max_p95_frame_time_us == 0
            || view.max_p99_frame_time_us == 0
            || view.max_peak_vram_bytes == 0
            || view.max_p95_frame_time_us > view.max_p99_frame_time_us
        {
            return Err(LeaseError::RenderBudgetInvalid(view.view_id.clone()));
        }
        if !seen.insert(view.view_id.clone()) {
            return Err(LeaseError::DuplicateViewId(view.view_id.clone()));
        }
        if input.budget.max_peak_vram_bytes > view.max_peak_vram_bytes {
            return Err(LeaseError::RenderBudgetInvalid(view.view_id.clone()));
        }
    }

    Ok(())
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        write!(&mut output, "{byte:02x}").expect("writing to a String cannot fail");
    }
    output
}

/// Compile a deterministic lease. View ordering is canonicalized by view ID.
pub fn compile_lease(mut input: LeaseInputV1) -> Result<LeaseReceiptV1, LeaseError> {
    input.views.sort_by(|left, right| left.view_id.cmp(&right.view_id));
    validate_input(&input)?;
    let canonical = serde_json::to_vec(&input)
        .map_err(|error| LeaseError::Serialization(error.to_string()))?;
    let input_sha256 = sha256_hex(&canonical);
    let mut lease_material = Vec::with_capacity(HASH_DOMAIN.len() + canonical.len());
    lease_material.extend_from_slice(HASH_DOMAIN);
    lease_material.extend_from_slice(&canonical);
    let lease_sha256 = sha256_hex(&lease_material);

    Ok(LeaseReceiptV1 {
        schema_version: SCHEMA_VERSION,
        input_sha256,
        lease_sha256,
        experiment: input.experiment,
        physics: input.physics,
        views: input.views,
        budget: input.budget,
        claim_ceiling: CLAIM_CEILING.to_string(),
    })
}

/// Rebuild the canonical input and reject any receipt whose contents were altered.
pub fn verify_lease_receipt(receipt: &LeaseReceiptV1) -> Result<(), LeaseError> {
    if receipt.schema_version != SCHEMA_VERSION || receipt.claim_ceiling != CLAIM_CEILING {
        return Err(LeaseError::ReceiptMismatch);
    }
    let expected = compile_lease(LeaseInputV1 {
        schema_version: receipt.schema_version,
        experiment: receipt.experiment.clone(),
        physics: receipt.physics.clone(),
        views: receipt.views.clone(),
        budget: receipt.budget.clone(),
    })?;
    if &expected != receipt {
        return Err(LeaseError::ReceiptMismatch);
    }
    Ok(())
}

fn usage_scope_matches(
    lease: &LeaseReceiptV1,
    usage: &UsageSnapshotV1,
) -> Result<(), LeaseError> {
    if usage.lease_sha256 != lease.lease_sha256 {
        return Err(LeaseError::UsageLeaseMismatch);
    }
    if usage.run_id != lease.experiment.run_id
        || usage.simulation_state_sha256 != lease.experiment.simulation_state_sha256
        || usage.event_order_sha256 != lease.experiment.event_order_sha256
        || usage.observation_frame_sha256 != lease.experiment.observation_frame_sha256
    {
        return Err(LeaseError::UsageScopeMismatch);
    }
    if usage.simulation_tick != lease.experiment.simulation_tick
        || usage.simulation_tick != lease.budget.valid_from_tick
        || usage.simulation_tick != lease.budget.valid_through_tick
    {
        return Err(LeaseError::UsageTickMismatch);
    }
    Ok(())
}

fn validate_usage_shape(
    lease: &LeaseReceiptV1,
    usage: &UsageSnapshotV1,
) -> Result<(), LeaseError> {
    usage_scope_matches(lease, usage)?;
    if usage.sequence == 0 {
        return Err(LeaseError::InvalidUsageCounters);
    }
    if let Some(previous) = usage.previous_usage_sha256.as_deref() {
        require_sha(previous, "previous_usage_sha256")?;
    }

    let allowed: BTreeMap<&str, &ViewAdmissionRef> = lease
        .views
        .iter()
        .map(|view| (view.view_id.as_str(), view))
        .collect();
    let mut seen = BTreeSet::new();
    let mut view_events = 0_u64;
    let mut rendered_frames = 0_u64;

    for view in &usage.views {
        if !seen.insert(view.view_id.as_str()) {
            return Err(LeaseError::DuplicateUsageView(view.view_id.clone()));
        }
        if !allowed.contains_key(view.view_id.as_str()) {
            return Err(LeaseError::UnknownView(view.view_id.clone()));
        }
        require_sha(
            &view.measurement_receipt_sha256,
            "view.measurement_receipt_sha256",
        )?;
        if view.view_events == 0
            || view.rendered_frames == 0
            || view.p95_frame_time_us == 0
            || view.p99_frame_time_us == 0
            || view.p95_frame_time_us > view.p99_frame_time_us
            || view.peak_vram_bytes == 0
        {
            return Err(LeaseError::InvalidUsageCounters);
        }
        view_events = view_events
            .checked_add(view.view_events)
            .ok_or(LeaseError::CounterOverflow)?;
        rendered_frames = rendered_frames
            .checked_add(view.rendered_frames)
            .ok_or(LeaseError::CounterOverflow)?;

    }

    if view_events != usage.view_event_count || rendered_frames != usage.rendered_frames_total {
        return Err(LeaseError::InconsistentUsageTotals);
    }

    match (
        usage.gpu_energy_uj_total,
        usage.gpu_energy_receipt_sha256.as_deref(),
    ) {
        (Some(_), Some(receipt)) => require_sha(receipt, "gpu_energy_receipt_sha256")?,
        (None, None) => {}
        _ => return Err(LeaseError::InconsistentUsageTotals),
    }

    Ok(())
}

/// Return a canonical, domain-separated digest for one cumulative usage snapshot.
/// View records are treated as a set and sorted by `view_id` before hashing.
/// This digest links adjacent records; it is not a signature or authenticated storage.
pub fn usage_snapshot_sha256(usage: &UsageSnapshotV1) -> Result<String, LeaseError> {
    let mut canonical = usage.clone();
    canonical.views.sort_by(|left, right| left.view_id.cmp(&right.view_id));
    let bytes = serde_json::to_vec(&canonical)
        .map_err(|error| LeaseError::Serialization(error.to_string()))?;
    let mut material = Vec::with_capacity(USAGE_HASH_DOMAIN.len() + bytes.len());
    material.extend_from_slice(USAGE_HASH_DOMAIN);
    material.extend_from_slice(&bytes);
    Ok(sha256_hex(&material))
}

fn usage_stop_reason(
    lease: &LeaseReceiptV1,
    usage: &UsageSnapshotV1,
) -> Result<Option<StopReason>, LeaseError> {
    let admissions: BTreeMap<&str, &ViewAdmissionRef> = lease
        .views
        .iter()
        .map(|view| (view.view_id.as_str(), view))
        .collect();

    for view in &usage.views {
        let admission = admissions
            .get(view.view_id.as_str())
            .ok_or_else(|| LeaseError::UnknownView(view.view_id.clone()))?;
        if view.p95_frame_time_us > admission.max_p95_frame_time_us
            || view.p99_frame_time_us > admission.max_p99_frame_time_us
        {
            return Ok(Some(StopReason::FrameTimeBudgetExceeded));
        }
        if view.peak_vram_bytes > admission.max_peak_vram_bytes
            || view.peak_vram_bytes > lease.budget.max_peak_vram_bytes
        {
            return Ok(Some(StopReason::GpuMemoryBudgetExceeded));
        }
    }

    if usage.peak_system_memory_bytes == 0 {
        return Err(LeaseError::InvalidUsageCounters);
    }
    if usage.peak_system_memory_bytes > lease.budget.max_peak_system_memory_bytes {
        return Ok(Some(StopReason::SystemMemoryBudgetExceeded));
    }
    if usage.views.len() > lease.budget.max_distinct_views as usize {
        return Ok(Some(StopReason::ViewEventBudgetExceeded));
    }
    if usage.view_event_count > lease.budget.max_view_events {
        return Ok(Some(StopReason::ViewEventBudgetExceeded));
    }
    if usage.rendered_frames_total > lease.budget.max_rendered_frames {
        return Ok(Some(StopReason::RenderedFrameBudgetExceeded));
    }
    if usage
        .views
        .iter()
        .map(|view| view.peak_vram_bytes)
        .max()
        .unwrap_or(0)
        > lease.budget.max_peak_vram_bytes
    {
        return Ok(Some(StopReason::GpuMemoryBudgetExceeded));
    }

    if let Some(cap) = lease.budget.max_gpu_energy_uj_total {
        match usage.gpu_energy_uj_total {
            Some(energy) if energy <= cap => {}
            Some(_) => return Ok(Some(StopReason::GpuEnergyBudgetExceeded)),
            None => return Ok(Some(StopReason::RequiredEnergyMissing)),
        }
    }

    Ok(None)
}

/// Check a cumulative usage snapshot. Corrupt scope/counters are errors;
/// policy-budget overruns and physical mutation attempts produce a Stop result.
pub fn evaluate_usage(
    lease: &LeaseReceiptV1,
    current: &UsageSnapshotV1,
    previous: Option<&UsageSnapshotV1>,
) -> Result<UsageDecision, LeaseError> {
    verify_lease_receipt(lease)?;
    validate_usage_shape(lease, current)?;

    if let Some(previous) = previous {
        validate_usage_shape(lease, previous)?;
        if previous.physical_mutation_attempted || usage_stop_reason(lease, previous)?.is_some() {
            return Err(LeaseError::PreviousSnapshotStopped);
        }
        if current.sequence
            != previous
                .sequence
                .checked_add(1)
                .ok_or(LeaseError::CounterOverflow)?
            || current.view_event_count < previous.view_event_count
            || current.rendered_frames_total < previous.rendered_frames_total
            || current.peak_system_memory_bytes < previous.peak_system_memory_bytes
            || current.gpu_energy_uj_total.zip(previous.gpu_energy_uj_total)
                .is_some_and(|(now, before)| now < before)
            || (previous.gpu_energy_uj_total.is_some() && current.gpu_energy_uj_total.is_none())
        {
            return Err(LeaseError::NonMonotonicUsage);
        }
        for prior in &previous.views {
            let now = current
                .views
                .iter()
                .find(|view| view.view_id == prior.view_id)
                .ok_or(LeaseError::NonMonotonicUsage)?;
            if now.view_events < prior.view_events
                || now.rendered_frames < prior.rendered_frames
                || now.peak_vram_bytes < prior.peak_vram_bytes
            {
                return Err(LeaseError::NonMonotonicUsage);
            }
        }

        let expected_parent = usage_snapshot_sha256(previous)?;
        if current.previous_usage_sha256.as_deref() != Some(expected_parent.as_str()) {
            return Err(LeaseError::UsageChainMismatch);
        }
    } else if current.sequence != 1 || current.previous_usage_sha256.is_some() {
        return Err(LeaseError::UsageChainMismatch);
    }

    let usage_sha256 = usage_snapshot_sha256(current)?;
    if current.physical_mutation_attempted {
        return Ok(UsageDecision::Stop {
            lease_sha256: lease.lease_sha256.clone(),
            sequence: current.sequence,
            usage_sha256,
            reason: StopReason::PhysicalMutationAttempted,
        });
    }
    if let Some(reason) = usage_stop_reason(lease, current)? {
        return Ok(UsageDecision::Stop {
            lease_sha256: lease.lease_sha256.clone(),
            sequence: current.sequence,
            usage_sha256,
            reason,
        });
    }

    Ok(UsageDecision::Continue {
        lease_sha256: lease.lease_sha256.clone(),
        sequence: current.sequence,
        usage_sha256,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> LeaseInputV1 {
        serde_json::from_str(include_str!("../fixtures/machine-test-01.json"))
            .expect("synthetic lease fixture parses")
    }

    fn compile_fixture() -> LeaseReceiptV1 {
        compile_lease(fixture()).expect("synthetic lease fixture compiles")
    }

    fn valid_usage(lease: &LeaseReceiptV1) -> UsageSnapshotV1 {
        UsageSnapshotV1 {
            lease_sha256: lease.lease_sha256.clone(),
            run_id: lease.experiment.run_id.clone(),
            simulation_state_sha256: lease.experiment.simulation_state_sha256.clone(),
            event_order_sha256: lease.experiment.event_order_sha256.clone(),
            observation_frame_sha256: lease.experiment.observation_frame_sha256.clone(),
            simulation_tick: lease.experiment.simulation_tick,
            sequence: 1,
            previous_usage_sha256: None,
            view_event_count: 1,
            rendered_frames_total: 120,
            peak_system_memory_bytes: 1_200_000_000,
            gpu_energy_uj_total: Some(600_000),
            gpu_energy_receipt_sha256: Some("a".repeat(64)),
            views: vec![ViewUsageV1 {
                view_id: "engineering-inspection".to_string(),
                view_events: 1,
                rendered_frames: 120,
                p95_frame_time_us: 12_000,
                p99_frame_time_us: 18_000,
                peak_vram_bytes: 3_500_000_000,
                measurement_receipt_sha256: "b".repeat(64),
            }],
            physical_mutation_attempted: false,
        }
    }

    #[test]
    fn synthetic_known_answer_compiles_and_verifies() {
        let lease = compile_fixture();
        assert_eq!(lease.schema_version, 1);
        assert_eq!(lease.views.len(), 2);
        assert_eq!(lease.experiment.simulation_tick, 120);
        verify_lease_receipt(&lease).expect("receipt recomputes");
    }

    #[test]
    fn view_order_does_not_change_lease_identity() {
        let original = fixture();
        let mut permuted = original.clone();
        permuted.views.reverse();
        let left = compile_lease(original).expect("original order");
        let right = compile_lease(permuted).expect("permuted order");
        assert_eq!(left.input_sha256, right.input_sha256);
        assert_eq!(left.lease_sha256, right.lease_sha256);
        assert_eq!(left.views, right.views);
    }

    #[test]
    fn changed_observation_creates_a_different_lease() {
        let original = compile_fixture();
        let mut changed = fixture();
        changed.experiment.observation_frame_sha256 = "a".repeat(64);
        for view in &mut changed.views {
            view.observation_frame_sha256 = changed.experiment.observation_frame_sha256.clone();
        }
        let replacement = compile_lease(changed).expect("new observation lease");
        assert_ne!(original.lease_sha256, replacement.lease_sha256);
    }

    #[test]
    fn run_source_state_and_event_order_are_part_of_lease_identity() {
        let baseline = compile_fixture();
        let mut changed_inputs = Vec::new();

        let mut changed = fixture();
        changed.experiment.run_id.push_str("-fork");
        changed_inputs.push(changed);

        let mut changed = fixture();
        changed.experiment.source_revision = "f".repeat(40);
        changed_inputs.push(changed);

        let mut changed = fixture();
        changed.experiment.initial_state_sha256 = "f".repeat(64);
        changed_inputs.push(changed);

        let mut changed = fixture();
        changed.experiment.simulation_state_sha256 = "f".repeat(64);
        changed_inputs.push(changed);

        let mut changed = fixture();
        changed.experiment.event_order_sha256 = "f".repeat(64);
        changed_inputs.push(changed);

        for input in changed_inputs {
            let fork = compile_lease(input).expect("identity fork compiles");
            assert_ne!(baseline.lease_sha256, fork.lease_sha256);
        }
    }

    #[test]
    fn lease_wide_energy_overrun_stops() {
        let lease = compile_fixture();
        let mut usage = valid_usage(&lease);
        usage.gpu_energy_uj_total = Some(lease.budget.max_gpu_energy_uj_total.unwrap() + 1);
        usage.gpu_energy_receipt_sha256 = Some("c".repeat(64));
        assert!(matches!(
            evaluate_usage(&lease, &usage, None),
            Ok(UsageDecision::Stop { reason: StopReason::GpuEnergyBudgetExceeded, .. })
        ));
    }

    #[test]
    fn missing_view_measurement_receipt_is_rejected() {
        let lease = compile_fixture();
        let mut usage = valid_usage(&lease);
        usage.views[0].measurement_receipt_sha256 = "not-a-digest".to_string();
        assert_eq!(
            evaluate_usage(&lease, &usage, None).unwrap_err(),
            LeaseError::InvalidSha256("view.measurement_receipt_sha256")
        );
    }

    #[test]
    fn view_cannot_reference_a_different_observation() {
        let mut input = fixture();
        input.views[0].observation_frame_sha256 = "a".repeat(64);
        assert!(matches!(
            compile_lease(input),
            Err(LeaseError::ObservationFrameMismatch(_))
        ));
    }

    #[test]
    fn duplicate_view_ids_are_rejected() {
        let mut input = fixture();
        input.views[1].view_id = input.views[0].view_id.clone();
        assert!(matches!(
            compile_lease(input),
            Err(LeaseError::DuplicateViewId(_))
        ));
    }

    #[test]
    fn lease_cannot_widen_any_view_vram_budget() {
        let mut input = fixture();
        input.budget.max_peak_vram_bytes = 5_000_000_000;
        assert!(matches!(
            compile_lease(input),
            Err(LeaseError::RenderBudgetInvalid(_))
        ));
    }

    #[test]
    fn lease_must_bind_exactly_one_snapshot_tick() {
        let mut input = fixture();
        input.budget.valid_through_tick += 1;
        assert_eq!(compile_lease(input).unwrap_err(), LeaseError::InvalidTickWindow);
    }

    #[test]
    fn valid_read_only_usage_continues() {
        let lease = compile_fixture();
        let usage = valid_usage(&lease);
        assert!(matches!(
            evaluate_usage(&lease, &usage, None).expect("valid usage"),
            UsageDecision::Continue { sequence: 1, .. }
        ));
    }

    #[test]
    fn physical_mutation_attempt_stops_the_lease() {
        let lease = compile_fixture();
        let mut usage = valid_usage(&lease);
        usage.physical_mutation_attempted = true;
        assert!(matches!(
            evaluate_usage(&lease, &usage, None),
            Ok(UsageDecision::Stop { reason: StopReason::PhysicalMutationAttempted, .. })
        ));
    }

    #[test]
    fn missing_energy_never_becomes_zero() {
        let lease = compile_fixture();
        let mut usage = valid_usage(&lease);
        usage.gpu_energy_uj_total = None;
        usage.gpu_energy_receipt_sha256 = None;
        assert!(matches!(
            evaluate_usage(&lease, &usage, None),
            Ok(UsageDecision::Stop { reason: StopReason::RequiredEnergyMissing, .. })
        ));
    }

    #[test]
    fn frame_budget_overrun_stops_without_relaxing_the_lease() {
        let lease = compile_fixture();
        let mut usage = valid_usage(&lease);
        usage.rendered_frames_total = lease.budget.max_rendered_frames + 1;
        usage.views[0].rendered_frames = usage.rendered_frames_total;
        // The independent lease-wide energy sample remains below its cap.
        assert!(matches!(
            evaluate_usage(&lease, &usage, None),
            Ok(UsageDecision::Stop { reason: StopReason::RenderedFrameBudgetExceeded, .. })
        ));
    }

    #[test]
    fn usage_from_another_run_is_rejected() {
        let lease = compile_fixture();
        let mut usage = valid_usage(&lease);
        usage.run_id = "different-run".to_string();
        assert_eq!(
            evaluate_usage(&lease, &usage, None).unwrap_err(),
            LeaseError::UsageScopeMismatch
        );
    }

    #[test]
    fn unknown_view_is_rejected() {
        let lease = compile_fixture();
        let mut usage = valid_usage(&lease);
        usage.views[0].view_id = "unadmitted-view".to_string();
        assert_eq!(
            evaluate_usage(&lease, &usage, None).unwrap_err(),
            LeaseError::UnknownView("unadmitted-view".to_string())
        );
    }

    #[test]
    fn usage_chain_links_adjacent_cumulative_snapshots() {
        let lease = compile_fixture();
        let first = valid_usage(&lease);
        let first_receipt = evaluate_usage(&lease, &first, None).expect("first snapshot accepted");
        let first_digest = match first_receipt {
            UsageDecision::Continue { usage_sha256, .. } => usage_sha256,
            other => panic!("unexpected first-snapshot decision: {other:?}"),
        };

        let mut second = first.clone();
        second.sequence = 2;
        second.previous_usage_sha256 = Some(first_digest);
        second.view_event_count = 2;
        second.rendered_frames_total += 30;
        second.views[0].view_events = 2;
        second.views[0].rendered_frames += 30;
        second.gpu_energy_uj_total = Some(second.gpu_energy_uj_total.unwrap() + 10);
        second.gpu_energy_receipt_sha256 = Some("c".repeat(64));
        assert!(matches!(
            evaluate_usage(&lease, &second, Some(&first)),
            Ok(UsageDecision::Continue { sequence: 2, .. })
        ));
    }

    #[test]
    fn usage_chain_rejects_a_spliced_or_wrong_parent() {
        let lease = compile_fixture();
        let first = valid_usage(&lease);
        let mut second = first.clone();
        second.sequence = 2;
        second.previous_usage_sha256 = Some("f".repeat(64));
        second.view_event_count = 2;
        second.rendered_frames_total += 30;
        second.views[0].view_events = 2;
        second.views[0].rendered_frames += 30;
        second.gpu_energy_uj_total = Some(second.gpu_energy_uj_total.unwrap() + 10);
        second.gpu_energy_receipt_sha256 = Some("c".repeat(64));
        assert_eq!(
            evaluate_usage(&lease, &second, Some(&first)).unwrap_err(),
            LeaseError::UsageChainMismatch
        );
    }

    #[test]
    fn usage_digest_is_invariant_to_view_record_order() {
        let mut usage = valid_usage(&compile_fixture());
        usage.views.push(ViewUsageV1 {
            view_id: "accessibility-inspection".to_string(),
            view_events: 1,
            rendered_frames: 120,
            p95_frame_time_us: 15_000,
            p99_frame_time_us: 20_000,
            peak_vram_bytes: 3_000_000_000,
            measurement_receipt_sha256: "c".repeat(64),
        });
        usage.view_event_count = 2;
        usage.rendered_frames_total = 240;
        let digest = usage_snapshot_sha256(&usage).expect("canonical usage digest");
        usage.views.reverse();
        assert_eq!(
            digest,
            usage_snapshot_sha256(&usage).expect("permuted usage digest")
        );
    }

    #[test]
    fn usage_sequence_and_counters_must_be_monotonic() {
        let lease = compile_fixture();
        let first = valid_usage(&lease);
        let mut second = first.clone();
        second.sequence = 2;
        second.rendered_frames_total = 90;
        second.views[0].rendered_frames = 90;
        second.gpu_energy_uj_total = Some(50_000);
        second.gpu_energy_receipt_sha256 = Some("c".repeat(64));
        assert_eq!(
            evaluate_usage(&lease, &second, Some(&first)).unwrap_err(),
            LeaseError::NonMonotonicUsage
        );
    }

    #[test]
    fn modified_receipt_is_rejected() {
        let mut lease = compile_fixture();
        lease.budget.max_rendered_frames += 1;
        assert_eq!(verify_lease_receipt(&lease).unwrap_err(), LeaseError::ReceiptMismatch);
    }

}
