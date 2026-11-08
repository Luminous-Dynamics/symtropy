// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// This crate verifies reduced-order coupling mathematics and time-window
// transaction contracts. It is not a CFD/FEM solver or an aeroelastic model.

use std::collections::BTreeSet;
use std::fmt::Write as _;

const DIGEST_A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const DIGEST_B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const DIGEST_C: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
const DIGEST_D: &str = "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
const DIGEST_E: &str = "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee";
const DIGEST_F: &str = "ffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffff";
const DIGEST_1: &str = "1111111111111111111111111111111111111111111111111111111111111111";
const DIGEST_2: &str = "2222222222222222222222222222222222222222222222222222222222222222";
const DIGEST_3: &str = "3333333333333333333333333333333333333333333333333333333333333333";
const DIGEST_4: &str = "4444444444444444444444444444444444444444444444444444444444444444";
const DIGEST_5: &str = "5555555555555555555555555555555555555555555555555555555555555555";
const DIGEST_6: &str = "6666666666666666666666666666666666666666666666666666666666666666";
const DIGEST_7: &str = "7777777777777777777777777777777777777777777777777777777777777777";

#[derive(Clone, Debug, PartialEq, Eq)]
struct AuditError(String);

impl std::fmt::Display for AuditError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for AuditError {}

type AuditResult<T> = Result<T, AuditError>;

fn fail<T>(message: impl Into<String>) -> AuditResult<T> {
    Err(AuditError(message.into()))
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn require_digest(value: &str, label: &str) -> AuditResult<()> {
    if valid_digest(value) {
        Ok(())
    } else {
        fail(format!("{label} must be a lowercase 64-character SHA-256 digest"))
    }
}

fn require_finite(value: f64, label: &str) -> AuditResult<()> {
    if value.is_finite() {
        Ok(())
    } else {
        fail(format!("{label} must be finite"))
    }
}

fn require_positive(value: f64, label: &str) -> AuditResult<()> {
    if value.is_finite() && value > 0.0 {
        Ok(())
    } else {
        fail(format!("{label} must be finite and positive"))
    }
}

fn threshold(abs_tol: f64, rel_tol: f64, scale: f64) -> f64 {
    abs_tol + rel_tol * scale.abs().max(1.0)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FixedPointStatus {
    Converged,
    NonConverged,
}

#[derive(Clone, Copy, Debug)]
struct AddedMassInput {
    structural_mass_kg: f64,
    fluid_added_mass_kg: f64,
    applied_force_n: f64,
    initial_acceleration_m_per_s2: f64,
    relaxation: f64,
    absolute_residual_tolerance_n: f64,
    relative_residual_tolerance: f64,
    max_iterations: usize,
}

#[derive(Clone, Copy, Debug)]
struct AddedMassResult {
    status: FixedPointStatus,
    exact_acceleration_m_per_s2: f64,
    acceleration_m_per_s2: Option<f64>,
    last_iterate_m_per_s2: f64,
    residual_n: f64,
    amplification_factor: f64,
    iterations: usize,
}

fn solve_added_mass_fixed_point(input: AddedMassInput) -> AuditResult<AddedMassResult> {
    require_positive(input.structural_mass_kg, "structural_mass_kg")?;
    require_finite(input.fluid_added_mass_kg, "fluid_added_mass_kg")?;
    if input.fluid_added_mass_kg < 0.0 {
        return fail("fluid_added_mass_kg cannot be negative");
    }
    require_finite(input.applied_force_n, "applied_force_n")?;
    require_finite(
        input.initial_acceleration_m_per_s2,
        "initial_acceleration_m_per_s2",
    )?;
    if !input.relaxation.is_finite() || input.relaxation <= 0.0 || input.relaxation > 1.0 {
        return fail("relaxation must be in (0, 1]");
    }
    require_positive(
        input.absolute_residual_tolerance_n,
        "absolute_residual_tolerance_n",
    )?;
    if !input.relative_residual_tolerance.is_finite()
        || input.relative_residual_tolerance < 0.0
    {
        return fail("relative_residual_tolerance must be finite and non-negative");
    }
    if input.max_iterations == 0 {
        return fail("max_iterations must be positive");
    }

    let total_mass = input.structural_mass_kg + input.fluid_added_mass_kg;
    require_positive(total_mass, "total_mass_kg")?;
    let exact = input.applied_force_n / total_mass;
    let ratio = input.fluid_added_mass_kg / input.structural_mass_kg;
    let amplification = 1.0 - input.relaxation * (1.0 + ratio);

    if !exact.is_finite() || !amplification.is_finite() {
        return fail("added-mass closed form or amplification factor is non-finite");
    }

    let mut acceleration = input.initial_acceleration_m_per_s2;
    let scale = input.applied_force_n.abs().max(1.0);
    let residual_tolerance = threshold(
        input.absolute_residual_tolerance_n,
        input.relative_residual_tolerance,
        scale,
    );

    for iteration in 1..=input.max_iterations {
        let unrelaxed = (input.applied_force_n
            - input.fluid_added_mass_kg * acceleration)
            / input.structural_mass_kg;
        let next =
            acceleration + input.relaxation * (unrelaxed - acceleration);
        if !next.is_finite() {
            return Ok(AddedMassResult {
                status: FixedPointStatus::NonConverged,
                exact_acceleration_m_per_s2: exact,
                acceleration_m_per_s2: None,
                last_iterate_m_per_s2: acceleration,
                residual_n: f64::INFINITY,
                amplification_factor: amplification,
                iterations: iteration,
            });
        }
        acceleration = next;
        let residual = ((input.structural_mass_kg + input.fluid_added_mass_kg)
            * acceleration
            - input.applied_force_n)
            .abs();
        if residual.is_finite() && residual <= residual_tolerance {
            return Ok(AddedMassResult {
                status: FixedPointStatus::Converged,
                exact_acceleration_m_per_s2: exact,
                acceleration_m_per_s2: Some(acceleration),
                last_iterate_m_per_s2: acceleration,
                residual_n: residual,
                amplification_factor: amplification,
                iterations: iteration,
            });
        }
    }

    let residual = ((input.structural_mass_kg + input.fluid_added_mass_kg)
        * acceleration
        - input.applied_force_n)
        .abs();
    Ok(AddedMassResult {
        status: FixedPointStatus::NonConverged,
        exact_acceleration_m_per_s2: exact,
        acceleration_m_per_s2: None,
        last_iterate_m_per_s2: acceleration,
        residual_n: residual,
        amplification_factor: amplification,
        iterations: input.max_iterations,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TemporalInterpolation {
    EndpointHold,
    Linear,
}

#[derive(Clone, Copy, Debug)]
struct TimeGrid {
    start_tick: u64,
    end_tick: u64,
    step_ticks: u64,
}

impl TimeGrid {
    fn substeps(self, label: &str) -> AuditResult<u64> {
        if self.end_tick <= self.start_tick {
            return fail(format!("{label} time grid must have positive duration"));
        }
        if self.step_ticks == 0 {
            return fail(format!("{label} step_ticks must be positive"));
        }
        let duration = self.end_tick - self.start_tick;
        if duration % self.step_ticks != 0 {
            return fail(format!(
                "{label} time grid does not cover the synchronization window exactly"
            ));
        }
        Ok(duration / self.step_ticks)
    }
}

#[derive(Clone, Copy, Debug)]
struct Vec3([f64; 3]);

impl Vec3 {
    fn add(self, other: Self) -> Self {
        Self([
            self.0[0] + other.0[0],
            self.0[1] + other.0[1],
            self.0[2] + other.0[2],
        ])
    }

    fn max_abs(self) -> f64 {
        self.0.iter().map(|value| value.abs()).fold(0.0, f64::max)
    }

    fn is_finite(self) -> bool {
        self.0.iter().all(|value| value.is_finite())
    }
}

#[derive(Clone, Copy, Debug)]
struct ExchangeReceipt {
    exchange_id: &'static str,
    window_id: &'static str,
    timebase_sha256: &'static str,
    input_state_sha256: &'static str,
    interface_map_sha256: &'static str,
    start_tick: u64,
    end_tick: u64,
    fluid_impulse_ns: Vec3,
    structure_impulse_ns: Vec3,
    fluid_interface_work_j: f64,
    structure_interface_work_j: f64,
}

#[derive(Clone, Copy, Debug)]
struct EnergyLedger {
    initial_fluid_energy_j: f64,
    initial_structure_energy_j: f64,
    final_fluid_energy_j: f64,
    final_structure_energy_j: f64,
    external_work_j: f64,
    declared_dissipation_j: f64,
}

#[derive(Clone, Copy, Debug)]
struct AuditTolerance {
    impulse_abs_ns: f64,
    work_abs_j: f64,
    energy_abs_j: f64,
    relative: f64,
}

#[derive(Clone, Copy, Debug)]
struct WindowInput {
    window_id: &'static str,
    start_tick: u64,
    end_tick: u64,
    timebase_sha256: &'static str,
    configuration_sha256: &'static str,
    geometry_sha256: &'static str,
    loading_state_sha256: &'static str,
    mission_phase_sha256: &'static str,
    environment_sha256: &'static str,
    fluid_model_sha256: &'static str,
    structure_model_sha256: &'static str,
    interface_map_sha256: &'static str,
    temporal_operator_sha256: &'static str,
    input_state_sha256: &'static str,
    proposed_output_state_sha256: &'static str,
    fluid_grid: TimeGrid,
    structure_grid: TimeGrid,
    temporal_interpolation: TemporalInterpolation,
    extrapolation_allowed: bool,
    coupling_converged: bool,
    coupling_iterations: usize,
    max_coupling_iterations: usize,
    coupling_residual: f64,
    coupling_tolerance: f64,
    fluid_inner_residual: f64,
    structure_inner_residual: f64,
    inner_tolerance: f64,
    minimum_inner_tightening_factor: f64,
    energy: EnergyLedger,
    exchange: ExchangeReceipt,
    tolerance: AuditTolerance,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WindowStatus {
    Committed,
    Rejected,
}

#[derive(Clone, Debug)]
struct WindowAudit {
    window_id: String,
    exchange_id: String,
    start_tick: u64,
    end_tick: u64,
    configuration_sha256: String,
    timebase_sha256: String,
    input_state_sha256: String,
    committed_state_sha256: String,
    status: WindowStatus,
    fluid_substeps: u64,
    structure_substeps: u64,
    impulse_residual_ns: f64,
    interface_work_residual_j: f64,
    energy_residual_j: f64,
    errors: Vec<String>,
}

fn validate_window_identity(input: WindowInput) -> AuditResult<()> {
    for (digest, label) in [
        (input.timebase_sha256, "timebase_sha256"),
        (input.configuration_sha256, "configuration_sha256"),
        (input.geometry_sha256, "geometry_sha256"),
        (input.loading_state_sha256, "loading_state_sha256"),
        (input.mission_phase_sha256, "mission_phase_sha256"),
        (input.environment_sha256, "environment_sha256"),
        (input.fluid_model_sha256, "fluid_model_sha256"),
        (input.structure_model_sha256, "structure_model_sha256"),
        (input.interface_map_sha256, "interface_map_sha256"),
        (input.temporal_operator_sha256, "temporal_operator_sha256"),
        (input.input_state_sha256, "input_state_sha256"),
        (
            input.proposed_output_state_sha256,
            "proposed_output_state_sha256",
        ),
        (input.exchange.exchange_id, "exchange_id"),
        (input.exchange.window_id, "exchange.window_id"),
        (input.exchange.timebase_sha256, "exchange.timebase_sha256"),
        (
            input.exchange.input_state_sha256,
            "exchange.input_state_sha256",
        ),
        (
            input.exchange.interface_map_sha256,
            "exchange.interface_map_sha256",
        ),
    ] {
        require_digest(digest, label)?;
    }
    if input.window_id.trim().is_empty() {
        return fail("window_id must be non-empty");
    }
    if input.end_tick <= input.start_tick {
        return fail("synchronization window must have positive duration");
    }
    if input.exchange.window_id != input.window_id
        || input.exchange.timebase_sha256 != input.timebase_sha256
        || input.exchange.input_state_sha256 != input.input_state_sha256
        || input.exchange.interface_map_sha256 != input.interface_map_sha256
        || input.exchange.start_tick != input.start_tick
        || input.exchange.end_tick != input.end_tick
    {
        return fail("exchange receipt does not bind the exact time window and state identity");
    }
    if input.fluid_grid.start_tick != input.start_tick
        || input.fluid_grid.end_tick != input.end_tick
        || input.structure_grid.start_tick != input.start_tick
        || input.structure_grid.end_tick != input.end_tick
    {
        return fail("participant time grids must cover the exact canonical synchronization interval");
    }
    match input.temporal_interpolation {
        TemporalInterpolation::EndpointHold | TemporalInterpolation::Linear => {}
    }
    if input.extrapolation_allowed {
        return fail("temporal extrapolation is not admitted by this v0 audit profile");
    }
    Ok(())
}

fn audit_window(input: WindowInput) -> AuditResult<WindowAudit> {
    validate_window_identity(input)?;
    let fluid_substeps = input.fluid_grid.substeps("fluid")?;
    let structure_substeps = input.structure_grid.substeps("structure")?;

    if input.max_coupling_iterations == 0
        || input.coupling_iterations == 0
        || input.coupling_iterations > input.max_coupling_iterations
    {
        return fail("coupling iteration count is outside the declared positive budget");
    }
    require_positive(input.coupling_tolerance, "coupling_tolerance")?;
    require_positive(input.inner_tolerance, "inner_tolerance")?;
    require_positive(
        input.minimum_inner_tightening_factor,
        "minimum_inner_tightening_factor",
    )?;
    require_finite(input.coupling_residual, "coupling_residual")?;
    require_finite(input.fluid_inner_residual, "fluid_inner_residual")?;
    require_finite(input.structure_inner_residual, "structure_inner_residual")?;
    if input.coupling_residual < 0.0
        || input.fluid_inner_residual < 0.0
        || input.structure_inner_residual < 0.0
    {
        return fail("residual magnitudes cannot be negative");
    }

    for value in [
        input.tolerance.impulse_abs_ns,
        input.tolerance.work_abs_j,
        input.tolerance.energy_abs_j,
    ] {
        require_positive(value, "absolute audit tolerance")?;
    }
    if !input.tolerance.relative.is_finite() || input.tolerance.relative < 0.0 {
        return fail("relative audit tolerance must be finite and non-negative");
    }

    for value in [
        input.energy.initial_fluid_energy_j,
        input.energy.initial_structure_energy_j,
        input.energy.final_fluid_energy_j,
        input.energy.final_structure_energy_j,
        input.energy.external_work_j,
        input.energy.declared_dissipation_j,
        input.exchange.fluid_interface_work_j,
        input.exchange.structure_interface_work_j,
    ] {
        require_finite(value, "energy/work ledger value")?;
    }
    if input.energy.initial_fluid_energy_j < 0.0
        || input.energy.initial_structure_energy_j < 0.0
        || input.energy.final_fluid_energy_j < 0.0
        || input.energy.final_structure_energy_j < 0.0
        || input.energy.declared_dissipation_j < 0.0
    {
        return fail("stored energy and declared dissipation cannot be negative");
    }
    if !input.exchange.fluid_impulse_ns.is_finite()
        || !input.exchange.structure_impulse_ns.is_finite()
    {
        return fail("interface impulse contains a non-finite component");
    }

    let impulse_residual = input
        .exchange
        .fluid_impulse_ns
        .add(input.exchange.structure_impulse_ns)
        .max_abs();
    let work_residual = (input.exchange.fluid_interface_work_j
        + input.exchange.structure_interface_work_j)
        .abs();

    let energy_before =
        input.energy.initial_fluid_energy_j + input.energy.initial_structure_energy_j;
    let energy_after =
        input.energy.final_fluid_energy_j + input.energy.final_structure_energy_j;
    let observed_energy_change = energy_after - energy_before;
    let expected_energy_change =
        input.energy.external_work_j - input.energy.declared_dissipation_j;
    let energy_residual = (observed_energy_change - expected_energy_change).abs();

    let impulse_scale = input
        .exchange
        .fluid_impulse_ns
        .max_abs()
        .max(input.exchange.structure_impulse_ns.max_abs());
    let work_scale = input
        .exchange
        .fluid_interface_work_j
        .abs()
        .max(input.exchange.structure_interface_work_j.abs());
    let energy_scale = observed_energy_change
        .abs()
        .max(expected_energy_change.abs());

    let mut errors = Vec::new();
    if !input.coupling_converged {
        errors.push("coupling_not_converged".to_owned());
    }
    if input.coupling_residual
        > threshold(input.coupling_tolerance, input.tolerance.relative, 1.0)
    {
        errors.push("coupling_residual_exceeds_tolerance".to_owned());
    }
    if input.minimum_inner_tightening_factor * input.inner_tolerance
        > input.coupling_tolerance
    {
        errors.push("inner_tolerance_not_stricter_than_coupling_policy".to_owned());
    }
    if input.fluid_inner_residual
        > threshold(input.inner_tolerance, input.tolerance.relative, 1.0)
    {
        errors.push("fluid_inner_residual_exceeds_tolerance".to_owned());
    }
    if input.structure_inner_residual
        > threshold(input.inner_tolerance, input.tolerance.relative, 1.0)
    {
        errors.push("structure_inner_residual_exceeds_tolerance".to_owned());
    }
    if impulse_residual
        > threshold(input.tolerance.impulse_abs_ns, input.tolerance.relative, impulse_scale)
    {
        errors.push("interface_impulse_not_action_reaction_balanced".to_owned());
    }
    if work_residual
        > threshold(input.tolerance.work_abs_j, input.tolerance.relative, work_scale)
    {
        errors.push("interface_work_not_balanced".to_owned());
    }
    if energy_residual
        > threshold(input.tolerance.energy_abs_j, input.tolerance.relative, energy_scale)
    {
        errors.push("whole_window_energy_balance_failed".to_owned());
    }

    let status = if errors.is_empty() {
        WindowStatus::Committed
    } else {
        WindowStatus::Rejected
    };
    let committed_state_sha256 = match status {
        WindowStatus::Committed => input.proposed_output_state_sha256.to_owned(),
        WindowStatus::Rejected => input.input_state_sha256.to_owned(),
    };

    Ok(WindowAudit {
        window_id: input.window_id.to_owned(),
        exchange_id: input.exchange.exchange_id.to_owned(),
        start_tick: input.start_tick,
        end_tick: input.end_tick,
        configuration_sha256: input.configuration_sha256.to_owned(),
        timebase_sha256: input.timebase_sha256.to_owned(),
        input_state_sha256: input.input_state_sha256.to_owned(),
        committed_state_sha256,
        status,
        fluid_substeps,
        structure_substeps,
        impulse_residual_ns: impulse_residual,
        interface_work_residual_j: work_residual,
        energy_residual_j: energy_residual,
        errors,
    })
}

fn audit_committed_sequence(windows: &[WindowAudit]) -> AuditResult<()> {
    if windows.is_empty() {
        return fail("committed sequence cannot be empty");
    }
    let mut seen_windows = BTreeSet::new();
    let mut seen_exchanges = BTreeSet::new();
    for (index, window) in windows.iter().enumerate() {
        if !seen_windows.insert(window.window_id.as_str()) {
            return fail(format!("duplicate window ID {}", window.window_id));
        }
        if !seen_exchanges.insert(window.exchange_id.as_str()) {
            return fail(format!("duplicate exchange settlement {}", window.exchange_id));
        }
        if window.status != WindowStatus::Committed {
            return fail(format!(
                "window {} is rejected; sequence cannot commit past it",
                window.window_id
            ));
        }
        if index > 0 {
            let previous = &windows[index - 1];
            if window.start_tick != previous.end_tick {
                return fail("committed windows contain a time gap or overlap");
            }
            if window.input_state_sha256 != previous.committed_state_sha256 {
                return fail("committed state chain is discontinuous");
            }
            if window.configuration_sha256 != previous.configuration_sha256 {
                return fail("configuration identity changed inside a frozen sequence");
            }
            if window.timebase_sha256 != previous.timebase_sha256 {
                return fail("canonical timebase changed inside a frozen sequence");
            }
        }
    }
    Ok(())
}

fn added_mass_status(value: FixedPointStatus) -> &'static str {
    match value {
        FixedPointStatus::Converged => "CONVERGED",
        FixedPointStatus::NonConverged => "NON_CONVERGED",
    }
}

fn window_status(value: WindowStatus) -> &'static str {
    match value {
        WindowStatus::Committed => "COMMITTED",
        WindowStatus::Rejected => "REJECTED",
    }
}

fn result_json() -> AuditResult<String> {
    let low = solve_added_mass_fixed_point(AddedMassInput {
        structural_mass_kg: 2.0,
        fluid_added_mass_kg: 1.0,
        applied_force_n: 12.0,
        initial_acceleration_m_per_s2: 0.0,
        relaxation: 1.0,
        absolute_residual_tolerance_n: 1.0e-10,
        relative_residual_tolerance: 0.0,
        max_iterations: 100,
    })?;
    let high_unrelaxed = solve_added_mass_fixed_point(AddedMassInput {
        structural_mass_kg: 1.0,
        fluid_added_mass_kg: 4.0,
        applied_force_n: 10.0,
        initial_acceleration_m_per_s2: 0.0,
        relaxation: 1.0,
        absolute_residual_tolerance_n: 1.0e-10,
        relative_residual_tolerance: 0.0,
        max_iterations: 20,
    })?;
    let high_relaxed = solve_added_mass_fixed_point(AddedMassInput {
        structural_mass_kg: 1.0,
        fluid_added_mass_kg: 4.0,
        applied_force_n: 10.0,
        initial_acceleration_m_per_s2: 0.0,
        relaxation: 0.2,
        absolute_residual_tolerance_n: 1.0e-10,
        relative_residual_tolerance: 0.0,
        max_iterations: 20,
    })?;

    let window = good_window();
    let audit = audit_window(window)?;
    if audit.status != WindowStatus::Committed {
        return fail(format!("locked good window rejected: {:?}", audit.errors));
    }

    Ok(format!(
        concat!(
            "{{",
            "\"schema_version\":\"aero-dynamic-coupling-audit/0.1.0\",",
            "\"claim_ceiling\":\"reduced_order_coupling_and_transaction_contract_only\",",
            "\"added_mass\":{{",
            "\"low_ratio\":{{\"status\":\"{}\",\"exact_acceleration_m_per_s2\":{:.17e},",
            "\"acceleration_m_per_s2\":{:.17e},\"amplification_factor\":{:.17e},",
            "\"residual_n\":{:.17e},\"iterations\":{}}},",
            "\"high_unrelaxed\":{{\"status\":\"{}\",\"exact_acceleration_m_per_s2\":{:.17e},",
            "\"acceleration_m_per_s2\":null,\"last_iterate_m_per_s2\":{:.17e},",
            "\"amplification_factor\":{:.17e},\"residual_n\":{:.17e},\"iterations\":{}}},",
            "\"high_relaxed\":{{\"status\":\"{}\",\"exact_acceleration_m_per_s2\":{:.17e},",
            "\"acceleration_m_per_s2\":{:.17e},\"amplification_factor\":{:.17e},",
            "\"residual_n\":{:.17e},\"iterations\":{}}}},",
            "\"window\":{{",
            "\"status\":\"{}\",\"commit_allowed\":{},",
            "\"window_id\":\"{}\",\"exchange_id\":\"{}\",",
            "\"fluid_substeps\":{},\"structure_substeps\":{},",
            "\"impulse_residual_ns\":{:.17e},",
            "\"interface_work_residual_j\":{:.17e},",
            "\"energy_residual_j\":{:.17e},",
            "\"committed_state_sha256\":\"{}\",",
            "\"errors\":[]}},",
            "\"aircraft_feasibility\":\"NOT_ASSESSED\"",
            "}}"
        ),
        added_mass_status(low.status),
        low.exact_acceleration_m_per_s2,
        low.acceleration_m_per_s2.unwrap_or(f64::NAN),
        low.amplification_factor,
        low.residual_n,
        low.iterations,
        added_mass_status(high_unrelaxed.status),
        high_unrelaxed.exact_acceleration_m_per_s2,
        high_unrelaxed.last_iterate_m_per_s2,
        high_unrelaxed.amplification_factor,
        high_unrelaxed.residual_n,
        high_unrelaxed.iterations,
        added_mass_status(high_relaxed.status),
        high_relaxed.exact_acceleration_m_per_s2,
        high_relaxed.acceleration_m_per_s2.unwrap_or(f64::NAN),
        high_relaxed.amplification_factor,
        high_relaxed.residual_n,
        high_relaxed.iterations,
        window_status(audit.status),
        audit.status == WindowStatus::Committed,
        audit.window_id,
        audit.exchange_id,
        audit.fluid_substeps,
        audit.structure_substeps,
        audit.impulse_residual_ns,
        audit.interface_work_residual_j,
        audit.energy_residual_j,
        audit.committed_state_sha256,
    ))
}

fn good_window() -> WindowInput {
    WindowInput {
        window_id: "aero-window-0001",
        start_tick: 0,
        end_tick: 100,
        timebase_sha256: DIGEST_4,
        configuration_sha256: DIGEST_A,
        geometry_sha256: DIGEST_B,
        loading_state_sha256: DIGEST_C,
        mission_phase_sha256: DIGEST_D,
        environment_sha256: DIGEST_E,
        fluid_model_sha256: DIGEST_F,
        structure_model_sha256: DIGEST_1,
        interface_map_sha256: DIGEST_2,
        temporal_operator_sha256: DIGEST_3,
        input_state_sha256: DIGEST_5,
        proposed_output_state_sha256: DIGEST_6,
        fluid_grid: TimeGrid {
            start_tick: 0,
            end_tick: 100,
            step_ticks: 25,
        },
        structure_grid: TimeGrid {
            start_tick: 0,
            end_tick: 100,
            step_ticks: 100,
        },
        temporal_interpolation: TemporalInterpolation::Linear,
        extrapolation_allowed: false,
        coupling_converged: true,
        coupling_iterations: 4,
        max_coupling_iterations: 10,
        coupling_residual: 1.0e-8,
        coupling_tolerance: 1.0e-6,
        fluid_inner_residual: 1.0e-10,
        structure_inner_residual: 2.0e-10,
        inner_tolerance: 1.0e-8,
        minimum_inner_tightening_factor: 100.0,
        energy: EnergyLedger {
            initial_fluid_energy_j: 120.0,
            initial_structure_energy_j: 80.0,
            final_fluid_energy_j: 130.0,
            final_structure_energy_j: 75.0,
            external_work_j: 8.0,
            declared_dissipation_j: 3.0,
        },
        exchange: ExchangeReceipt {
            exchange_id: DIGEST_7,
            window_id: "aero-window-0001",
            timebase_sha256: DIGEST_4,
            input_state_sha256: DIGEST_5,
            interface_map_sha256: DIGEST_2,
            start_tick: 0,
            end_tick: 100,
            fluid_impulse_ns: Vec3([0.0, -10.0, 0.0]),
            structure_impulse_ns: Vec3([0.0, 10.0, 0.0]),
            fluid_interface_work_j: 5.0,
            structure_interface_work_j: -5.0,
        },
        tolerance: AuditTolerance {
            impulse_abs_ns: 1.0e-12,
            work_abs_j: 1.0e-12,
            energy_abs_j: 1.0e-12,
            relative: 1.0e-12,
        },
    }
}

fn main() {
    match result_json() {
        Ok(json) => println!("{json}"),
        Err(error) => {
            eprintln!("dynamic_coupling_audit_error={error}");
            std::process::exit(2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn low_case() -> AddedMassInput {
        AddedMassInput {
            structural_mass_kg: 2.0,
            fluid_added_mass_kg: 1.0,
            applied_force_n: 12.0,
            initial_acceleration_m_per_s2: 0.0,
            relaxation: 1.0,
            absolute_residual_tolerance_n: 1.0e-10,
            relative_residual_tolerance: 0.0,
            max_iterations: 100,
        }
    }

    fn high_case(relaxation: f64) -> AddedMassInput {
        AddedMassInput {
            structural_mass_kg: 1.0,
            fluid_added_mass_kg: 4.0,
            applied_force_n: 10.0,
            initial_acceleration_m_per_s2: 0.0,
            relaxation,
            absolute_residual_tolerance_n: 1.0e-10,
            relative_residual_tolerance: 0.0,
            max_iterations: 20,
        }
    }

    #[test]
    fn low_added_mass_ratio_converges_to_closed_form() {
        let result = solve_added_mass_fixed_point(low_case()).unwrap();
        assert_eq!(result.status, FixedPointStatus::Converged);
        assert!((result.exact_acceleration_m_per_s2 - 4.0).abs() < 1.0e-14);
        assert!((result.acceleration_m_per_s2.unwrap() - 4.0).abs() < 1.0e-9);
        assert!((result.amplification_factor + 0.5).abs() < 1.0e-14);
        assert!(result.residual_n <= 1.0e-10);
    }

    #[test]
    fn high_added_mass_unrelaxed_iteration_abstains() {
        let result = solve_added_mass_fixed_point(high_case(1.0)).unwrap();
        assert_eq!(result.status, FixedPointStatus::NonConverged);
        assert!(result.acceleration_m_per_s2.is_none());
        assert!((result.exact_acceleration_m_per_s2 - 2.0).abs() < 1.0e-14);
        assert!((result.amplification_factor + 4.0).abs() < 1.0e-14);
        assert_eq!(result.iterations, 20);
        assert!(result.residual_n > 1.0e-10);
    }

    #[test]
    fn high_added_mass_relaxed_iteration_converges_to_same_solution() {
        let result = solve_added_mass_fixed_point(high_case(0.2)).unwrap();
        assert_eq!(result.status, FixedPointStatus::Converged);
        assert!((result.acceleration_m_per_s2.unwrap() - 2.0).abs() < 1.0e-14);
        assert!((result.amplification_factor).abs() < 1.0e-14);
        assert_eq!(result.iterations, 1);
    }

    #[test]
    fn invalid_mass_and_relaxation_are_rejected() {
        let mut input = low_case();
        input.structural_mass_kg = 0.0;
        assert!(solve_added_mass_fixed_point(input).is_err());
        let mut input = low_case();
        input.relaxation = 0.0;
        assert!(solve_added_mass_fixed_point(input).is_err());
        let mut input = low_case();
        input.fluid_added_mass_kg = -1.0;
        assert!(solve_added_mass_fixed_point(input).is_err());
    }

    #[test]
    fn non_finite_force_and_zero_iteration_budget_are_rejected() {
        let mut input = low_case();
        input.applied_force_n = f64::NAN;
        assert!(solve_added_mass_fixed_point(input).is_err());
        let mut input = low_case();
        input.max_iterations = 0;
        assert!(solve_added_mass_fixed_point(input).is_err());
    }

    #[test]
    fn exact_integer_subcycling_is_counted() {
        let window = good_window();
        assert_eq!(window.fluid_grid.substeps("fluid").unwrap(), 4);
        assert_eq!(window.structure_grid.substeps("structure").unwrap(), 1);
    }

    #[test]
    fn missing_final_substep_is_rejected() {
        let mut window = good_window();
        window.fluid_grid.step_ticks = 30;
        assert!(audit_window(window).is_err());
    }

    #[test]
    fn exact_window_commits_only_after_all_audits_pass() {
        let audit = audit_window(good_window()).unwrap();
        assert_eq!(audit.status, WindowStatus::Committed);
        assert_eq!(audit.committed_state_sha256, DIGEST_6);
        assert_eq!(audit.fluid_substeps, 4);
        assert_eq!(audit.structure_substeps, 1);
        assert_eq!(audit.errors.len(), 0);
        assert!(audit.energy_residual_j <= 1.0e-12);
    }

    #[test]
    fn interface_work_imbalance_rejects_and_rolls_back() {
        let mut window = good_window();
        window.exchange.structure_interface_work_j = -4.0;
        let audit = audit_window(window).unwrap();
        assert_eq!(audit.status, WindowStatus::Rejected);
        assert!(audit.errors.iter().any(|e| e == "interface_work_not_balanced"));
        assert_eq!(audit.committed_state_sha256, DIGEST_5);
    }

    #[test]
    fn energy_creation_without_declared_source_rejects_and_rolls_back() {
        let mut window = good_window();
        window.energy.final_fluid_energy_j += 1.0;
        let audit = audit_window(window).unwrap();
        assert_eq!(audit.status, WindowStatus::Rejected);
        assert!(audit.errors.iter().any(|e| e == "whole_window_energy_balance_failed"));
        assert_eq!(audit.committed_state_sha256, DIGEST_5);
    }

    #[test]
    fn action_reaction_impulse_mismatch_rejects() {
        let mut window = good_window();
        window.exchange.structure_impulse_ns = Vec3([0.0, 9.0, 0.0]);
        let audit = audit_window(window).unwrap();
        assert_eq!(audit.status, WindowStatus::Rejected);
        assert!(audit.errors.iter().any(|e| e == "interface_impulse_not_action_reaction_balanced"));
    }

    #[test]
    fn participant_inner_tolerance_policy_must_be_stricter() {
        let mut window = good_window();
        window.inner_tolerance = 1.0e-6;
        let audit = audit_window(window).unwrap();
        assert_eq!(audit.status, WindowStatus::Rejected);
        assert!(audit.errors.iter().any(|e| e == "inner_tolerance_not_stricter_than_coupling_policy"));
    }

    #[test]
    fn non_converged_window_cannot_commit_partial_state() {
        let mut window = good_window();
        window.coupling_converged = false;
        window.proposed_output_state_sha256 = DIGEST_1;
        let audit = audit_window(window).unwrap();
        assert_eq!(audit.status, WindowStatus::Rejected);
        assert_eq!(audit.committed_state_sha256, DIGEST_5);
    }

    #[test]
    fn stale_exchange_state_and_timebase_are_rejected() {
        let mut window = good_window();
        window.exchange.input_state_sha256 = DIGEST_6;
        assert!(audit_window(window).is_err());

        let mut window = good_window();
        window.exchange.timebase_sha256 = DIGEST_3;
        assert!(audit_window(window).is_err());
    }

    #[test]
    fn committed_sequence_requires_contiguous_windows_unique_exchanges_and_state_chain() {
        let first = audit_window(good_window()).unwrap();
        let mut second_input = good_window();
        second_input.window_id = "aero-window-0002";
        second_input.start_tick = 100;
        second_input.end_tick = 200;
        second_input.input_state_sha256 = DIGEST_6;
        second_input.proposed_output_state_sha256 = DIGEST_1;
        second_input.fluid_grid = TimeGrid {
            start_tick: 100,
            end_tick: 200,
            step_ticks: 25,
        };
        second_input.structure_grid = TimeGrid {
            start_tick: 100,
            end_tick: 200,
            step_ticks: 100,
        };
        second_input.exchange = ExchangeReceipt {
            exchange_id: DIGEST_2,
            window_id: "aero-window-0002",
            timebase_sha256: DIGEST_4,
            input_state_sha256: DIGEST_6,
            interface_map_sha256: DIGEST_2,
            start_tick: 100,
            end_tick: 200,
            ..good_window().exchange
        };
        let second = audit_window(second_input).unwrap();
        assert!(audit_committed_sequence(&[first.clone(), second.clone()]).is_ok());

        let mut duplicate_exchange = second.clone();
        duplicate_exchange.exchange_id = first.exchange_id.clone();
        assert!(audit_committed_sequence(&[first.clone(), duplicate_exchange]).is_err());

        let mut gap = second.clone();
        gap.start_tick = 101;
        assert!(audit_committed_sequence(&[first.clone(), gap]).is_err());

        let mut stale_chain = second.clone();
        stale_chain.input_state_sha256 = DIGEST_5.to_owned();
        assert!(audit_committed_sequence(&[first, stale_chain]).is_err());
    }

    #[test]
    fn temporal_extrapolation_and_invalid_digest_are_rejected() {
        let mut window = good_window();
        window.extrapolation_allowed = true;
        assert!(audit_window(window).is_err());

        let mut window = good_window();
        window.temporal_operator_sha256 = "not-a-digest";
        assert!(audit_window(window).is_err());
    }
}
