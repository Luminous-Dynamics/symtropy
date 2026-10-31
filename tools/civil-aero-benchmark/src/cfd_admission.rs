//! Admission gate for steady CFD evidence under explicit verification criteria.
//!
//! Passing means only that supplied run records satisfy the declared numerical checks and
//! the scalar grid-convergence study. It is not experimental validation, model validation,
//! flight certification, or proof that caller-supplied metadata is authentic.

use std::collections::HashSet;

use crate::grid_convergence::{
    GciReport, GciStudy, GridFamilyKind, GridObservation, evaluate_grid_convergence,
};

const MIN_HISTORY_SAMPLES: usize = 6;

#[derive(Debug, Clone, PartialEq)]
pub struct CfdGridRun {
    pub run_id: String,
    pub grid_id: String,
    pub grid_level: u32,
    pub characteristic_spacing: f64,
    pub mesh_sha256: String,
    pub solver_build_sha256: String,
    pub physics_config_sha256: String,
    pub geometry_sha256: String,
    /// Process status from the solver execution record; zero is required.
    pub solver_exit_code: i32,
    pub iterations_completed: u64,
    pub mesh_node_count: u64,
    pub cpu_core_hours: f64,
    /// None means not measured; it must not be reported as zero.
    pub gpu_hours: Option<f64>,
    pub wall_clock_seconds: f64,
    pub peak_memory_bytes: u64,
    /// None means not measured; it must not be reported as zero.
    pub energy_consumed_joules: Option<f64>,
    pub residual_initial: f64,
    pub residual_final: f64,
    /// Last steady-monitor samples for the audited coefficient.
    pub quantity_history: Vec<f64>,
    /// Absolute normalized global mass-balance defect.
    pub mass_imbalance_fraction: f64,
    /// Absolute normalized global energy-balance defect.
    pub energy_imbalance_fraction: f64,
    pub quantity_value: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AdmissionCriteria {
    pub residual_final_max: f64,
    /// Require residual_final / residual_initial <= this value.
    pub residual_reduction_ratio_max: f64,
    /// Maximum absolute difference between means of the last two three-sample windows.
    pub quantity_history_drift_absolute_max: f64,
    pub mass_imbalance_fraction_max: f64,
    pub energy_imbalance_fraction_max: f64,
    pub observed_order_min: f64,
    pub observed_order_max: f64,
    pub asymptotic_ratio_min: f64,
    pub asymptotic_ratio_max: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CfdEvidenceStudy {
    pub case_id: String,
    pub quantity: String,
    pub source_reference: String,
    pub spacing_definition: String,
    pub grid_family_kind: GridFamilyKind,
    pub safety_factor: f64,
    pub criteria: AdmissionCriteria,
    /// Ordered coarse-to-fine; at least three runs are required.
    pub runs: Vec<CfdGridRun>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CfdGridRunAudit {
    pub run_id: String,
    pub grid_id: String,
    pub grid_level: u32,
    pub characteristic_spacing: f64,
    pub mesh_sha256: String,
    pub mesh_node_count: u64,
    pub solver_exit_code: i32,
    pub iterations_completed: u64,
    pub quantity_value: f64,
    pub residual_final: f64,
    pub residual_reduction_ratio: f64,
    pub quantity_history_drift_absolute: f64,
    pub mass_imbalance_fraction: f64,
    pub energy_imbalance_fraction: f64,
    pub cpu_core_hours: f64,
    pub gpu_hours: Option<f64>,
    pub wall_clock_seconds: f64,
    pub peak_memory_bytes: u64,
    pub energy_consumed_joules: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CfdEvidenceAdmissionReport {
    pub case_id: String,
    pub quantity: String,
    pub source_reference: String,
    pub spacing_definition: String,
    pub solver_build_sha256: String,
    pub physics_config_sha256: String,
    pub geometry_sha256: String,
    pub grid_family_kind: GridFamilyKind,
    pub safety_factor: f64,
    pub criteria: AdmissionCriteria,
    pub status: &'static str,
    pub grid_run_audits: Vec<CfdGridRunAudit>,
    pub total_cpu_core_hours: f64,
    pub total_gpu_hours_measured: Option<f64>,
    pub gpu_measured_run_count: usize,
    pub total_wall_clock_seconds: f64,
    pub peak_memory_bytes: u64,
    pub total_energy_joules_measured: Option<f64>,
    pub energy_measured_run_count: usize,
    pub gci_report: GciReport,
}

impl AdmissionCriteria {
    fn validate(&self) -> Result<(), String> {
        for (name, value) in [
            ("residual_final_max", self.residual_final_max),
            (
                "residual_reduction_ratio_max",
                self.residual_reduction_ratio_max,
            ),
            (
                "quantity_history_drift_absolute_max",
                self.quantity_history_drift_absolute_max,
            ),
            (
                "mass_imbalance_fraction_max",
                self.mass_imbalance_fraction_max,
            ),
            (
                "energy_imbalance_fraction_max",
                self.energy_imbalance_fraction_max,
            ),
            ("observed_order_min", self.observed_order_min),
            ("observed_order_max", self.observed_order_max),
            ("asymptotic_ratio_min", self.asymptotic_ratio_min),
            ("asymptotic_ratio_max", self.asymptotic_ratio_max),
        ] {
            if !value.is_finite() || value < 0.0 {
                return Err(format!("criterion {name} must be finite and >= 0"));
            }
        }
        if self.residual_final_max == 0.0
            || self.residual_reduction_ratio_max == 0.0
            || self.residual_reduction_ratio_max >= 1.0
            || self.observed_order_min == 0.0
            || self.observed_order_min > self.observed_order_max
            || self.asymptotic_ratio_min == 0.0
            || self.asymptotic_ratio_min > self.asymptotic_ratio_max
        {
            return Err("admission criteria contain an invalid or reversed interval".to_owned());
        }
        Ok(())
    }
}

pub fn evaluate_cfd_evidence_admission(
    study: &CfdEvidenceStudy,
) -> Result<CfdEvidenceAdmissionReport, String> {
    if study.case_id.trim().is_empty()
        || study.quantity.trim().is_empty()
        || study.source_reference.trim().is_empty()
        || study.spacing_definition.trim().is_empty()
    {
        return Err(
            "case_id, quantity, source_reference, and spacing_definition must be non-empty"
                .to_owned(),
        );
    }
    study.criteria.validate()?;
    if study.runs.len() < 3 {
        return Err("at least three grid runs are required for numerical admission".to_owned());
    }

    let first = &study.runs[0];
    validate_sha256(&first.solver_build_sha256, "solver_build_sha256")?;
    validate_sha256(&first.physics_config_sha256, "physics_config_sha256")?;
    validate_sha256(&first.geometry_sha256, "geometry_sha256")?;

    let mut run_ids = HashSet::new();
    let mut grid_ids = HashSet::new();
    let mut grid_levels = HashSet::new();
    let mut mesh_digests = HashSet::new();
    let mut run_audits = Vec::with_capacity(study.runs.len());

    for (index, run) in study.runs.iter().enumerate() {
        if run.run_id.trim().is_empty() || !run_ids.insert(run.run_id.clone()) {
            return Err(format!("grid run {index}: run_id is empty or duplicated"));
        }
        if run.grid_id.trim().is_empty() || !grid_ids.insert(run.grid_id.clone()) {
            return Err(format!(
                "grid run {}: grid_id is empty or duplicated",
                run.run_id
            ));
        }
        if run.grid_level == 0 || !grid_levels.insert(run.grid_level) {
            return Err(format!(
                "grid run {}: grid_level must be positive and unique",
                run.run_id
            ));
        }
        validate_sha256(&run.mesh_sha256, "mesh_sha256")?;
        validate_sha256(&run.solver_build_sha256, "solver_build_sha256")?;
        validate_sha256(&run.physics_config_sha256, "physics_config_sha256")?;
        validate_sha256(&run.geometry_sha256, "geometry_sha256")?;

        if run.solver_build_sha256 != first.solver_build_sha256
            || run.physics_config_sha256 != first.physics_config_sha256
            || run.geometry_sha256 != first.geometry_sha256
        {
            return Err(format!(
                "grid run {} changes solver build, physics configuration, or geometry identity",
                run.run_id
            ));
        }
        if !mesh_digests.insert(run.mesh_sha256.clone()) {
            return Err(format!(
                "grid run {} reuses a mesh SHA-256; each grid must have distinct mesh bytes",
                run.run_id
            ));
        }
        if run.solver_exit_code != 0 {
            return Err(format!(
                "grid run {} has non-zero solver exit code {}",
                run.run_id, run.solver_exit_code
            ));
        }
        if run.iterations_completed == 0 || run.mesh_node_count == 0 || run.peak_memory_bytes == 0 {
            return Err(format!(
                "grid run {} is missing positive iteration, mesh-node, or peak-memory evidence",
                run.run_id
            ));
        }

        for (name, value) in [
            ("characteristic_spacing", run.characteristic_spacing),
            ("residual_initial", run.residual_initial),
            ("residual_final", run.residual_final),
            ("mass_imbalance_fraction", run.mass_imbalance_fraction),
            ("energy_imbalance_fraction", run.energy_imbalance_fraction),
            ("quantity_value", run.quantity_value),
            ("cpu_core_hours", run.cpu_core_hours),
            ("wall_clock_seconds", run.wall_clock_seconds),
        ] {
            if !value.is_finite() {
                return Err(format!("grid run {}: {name} is non-finite", run.run_id));
            }
        }
        if run.characteristic_spacing <= 0.0
            || run.cpu_core_hours <= 0.0
            || run.wall_clock_seconds <= 0.0
            || run.residual_initial <= 0.0
            || run.residual_final <= 0.0
            || run.mass_imbalance_fraction < 0.0
            || run.energy_imbalance_fraction < 0.0
        {
            return Err(format!(
                "grid run {}: spacing, resource times, and residuals must be positive and conservation defects non-negative",
                run.run_id
            ));
        }
        for (name, value) in [
            ("gpu_hours", run.gpu_hours),
            ("energy_consumed_joules", run.energy_consumed_joules),
        ] {
            if value.is_some_and(|number| !number.is_finite() || number < 0.0) {
                return Err(format!(
                    "grid run {}: {name} must be finite and non-negative when present",
                    run.run_id
                ));
            }
        }
        if index > 0 {
            let previous = &study.runs[index - 1];
            if previous.grid_level >= run.grid_level
                || previous.characteristic_spacing <= run.characteristic_spacing
            {
                return Err(format!(
                    "grid runs must be strictly ordered coarse-to-fine: {} before {}",
                    previous.grid_id, run.grid_id
                ));
            }
        }

        let residual_reduction_ratio = run.residual_final / run.residual_initial;
        if !residual_reduction_ratio.is_finite()
            || run.residual_final > study.criteria.residual_final_max
            || residual_reduction_ratio > study.criteria.residual_reduction_ratio_max
        {
            return Err(format!(
                "grid run {} fails residual convergence criteria: final={} ratio={}",
                run.run_id, run.residual_final, residual_reduction_ratio
            ));
        }

        let quantity_history_drift_absolute = history_drift(&run.quantity_history)
            .map_err(|error| format!("grid run {}: {error}", run.run_id))?;
        if quantity_history_drift_absolute > study.criteria.quantity_history_drift_absolute_max {
            return Err(format!(
                "grid run {} fails quantity-history stability: drift={quantity_history_drift_absolute}",
                run.run_id
            ));
        }
        if run.mass_imbalance_fraction > study.criteria.mass_imbalance_fraction_max {
            return Err(format!(
                "grid run {} fails mass conservation: defect={}",
                run.run_id, run.mass_imbalance_fraction
            ));
        }
        if run.energy_imbalance_fraction > study.criteria.energy_imbalance_fraction_max {
            return Err(format!(
                "grid run {} fails energy conservation: defect={}",
                run.run_id, run.energy_imbalance_fraction
            ));
        }

        run_audits.push(CfdGridRunAudit {
            run_id: run.run_id.clone(),
            grid_id: run.grid_id.clone(),
            grid_level: run.grid_level,
            characteristic_spacing: run.characteristic_spacing,
            mesh_sha256: run.mesh_sha256.clone(),
            quantity_value: run.quantity_value,
            residual_final: run.residual_final,
            residual_reduction_ratio,
            quantity_history_drift_absolute,
            mass_imbalance_fraction: run.mass_imbalance_fraction,
            energy_imbalance_fraction: run.energy_imbalance_fraction,
            cpu_core_hours: run.cpu_core_hours,
            gpu_hours: run.gpu_hours,
            wall_clock_seconds: run.wall_clock_seconds,
            peak_memory_bytes: run.peak_memory_bytes,
            energy_consumed_joules: run.energy_consumed_joules,
        });
    }

    let gci_study = GciStudy {
        quantity: study.quantity.clone(),
        source_reference: study.source_reference.clone(),
        spacing_definition: study.spacing_definition.clone(),
        grid_family_kind: study.grid_family_kind,
        safety_factor: study.safety_factor,
        observations: study
            .runs
            .iter()
            .map(|run| GridObservation {
                grid_id: run.grid_id.clone(),
                characteristic_spacing: run.characteristic_spacing,
                value: run.quantity_value,
            })
            .collect(),
    };
    let gci_report = evaluate_grid_convergence(&gci_study)?;

    for triplet in &gci_report.triplets {
        if triplet.observed_order < study.criteria.observed_order_min
            || triplet.observed_order > study.criteria.observed_order_max
        {
            return Err(format!(
                "GCI observed order {} is outside declared interval [{}, {}]",
                triplet.observed_order,
                study.criteria.observed_order_min,
                study.criteria.observed_order_max
            ));
        }
        if triplet.asymptotic_ratio < study.criteria.asymptotic_ratio_min
            || triplet.asymptotic_ratio > study.criteria.asymptotic_ratio_max
        {
            return Err(format!(
                "GCI asymptotic ratio {} is outside declared interval [{}, {}]",
                triplet.asymptotic_ratio,
                study.criteria.asymptotic_ratio_min,
                study.criteria.asymptotic_ratio_max
            ));
        }
    }

    let total_cpu_core_hours = study.runs.iter().map(|run| run.cpu_core_hours).sum::<f64>();
    let total_wall_clock_seconds = study
        .runs
        .iter()
        .map(|run| run.wall_clock_seconds)
        .sum::<f64>();
    let peak_memory_bytes = study
        .runs
        .iter()
        .map(|run| run.peak_memory_bytes)
        .max()
        .unwrap_or_default();
    let gpu_values = study
        .runs
        .iter()
        .filter_map(|run| run.gpu_hours)
        .collect::<Vec<_>>();
    let energy_values = study
        .runs
        .iter()
        .filter_map(|run| run.energy_consumed_joules)
        .collect::<Vec<_>>();
    let total_gpu_hours_measured = if gpu_values.is_empty() {
        None
    } else {
        Some(gpu_values.iter().sum::<f64>())
    };
    let total_energy_joules_measured = if energy_values.is_empty() {
        None
    } else {
        Some(energy_values.iter().sum::<f64>())
    };
    if !total_cpu_core_hours.is_finite()
        || !total_wall_clock_seconds.is_finite()
        || total_gpu_hours_measured.is_some_and(|value| !value.is_finite())
        || total_energy_joules_measured.is_some_and(|value| !value.is_finite())
    {
        return Err("study resource totals overflowed to non-finite values".to_owned());
    }

    Ok(CfdEvidenceAdmissionReport {
        case_id: study.case_id.clone(),
        quantity: study.quantity.clone(),
        source_reference: study.source_reference.clone(),
        spacing_definition: study.spacing_definition.clone(),
        solver_build_sha256: first.solver_build_sha256.clone(),
        physics_config_sha256: first.physics_config_sha256.clone(),
        geometry_sha256: first.geometry_sha256.clone(),
        grid_family_kind: study.grid_family_kind,
        safety_factor: study.safety_factor,
        criteria: study.criteria.clone(),
        status: "numerically_admitted_under_declared_criteria_caller_hashes_unverified_not_experimentally_validated",
        grid_run_audits: run_audits,
        total_cpu_core_hours,
        total_gpu_hours_measured,
        gpu_measured_run_count: gpu_values.len(),
        total_wall_clock_seconds,
        peak_memory_bytes,
        total_energy_joules_measured,
        energy_measured_run_count: energy_values.len(),
        gci_report,
    })
}

fn validate_sha256(value: &str, name: &str) -> Result<(), String> {
    if value.len() != 64 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(format!("{name} must be a 64-character hexadecimal digest"));
    }
    Ok(())
}

fn history_drift(history: &[f64]) -> Result<f64, String> {
    if history.len() < MIN_HISTORY_SAMPLES {
        return Err(format!(
            "quantity history requires at least {MIN_HISTORY_SAMPLES} finite samples"
        ));
    }
    if history.iter().any(|value| !value.is_finite()) {
        return Err("quantity history contains a non-finite sample".to_owned());
    }
    let tail = &history[history.len() - MIN_HISTORY_SAMPLES..];
    let first_mean = tail[..3].iter().sum::<f64>() / 3.0;
    let second_mean = tail[3..].iter().sum::<f64>() / 3.0;
    let drift = (second_mean - first_mean).abs();
    if !first_mean.is_finite() || !second_mean.is_finite() || !drift.is_finite() {
        return Err("quantity-history window mean overflowed".to_owned());
    }
    Ok(drift)
}

fn csv_escape(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

fn optional_decimal(value: Option<f64>) -> String {
    value
        .map(|number| format!("{number:.12}"))
        .unwrap_or_default()
}

pub const ADMISSION_CSV_HEADER: &str = "case_id,quantity,status,source_reference,spacing_definition,grid_family_kind,solver_build_sha256,physics_config_sha256,geometry_sha256,safety_factor,coarse_run_id,coarse_grid_id,coarse_grid_level,coarse_spacing,coarse_mesh_sha256,coarse_mesh_node_count,coarse_solver_exit_code,coarse_iterations_completed,coarse_cpu_core_hours,coarse_gpu_hours,coarse_wall_clock_seconds,coarse_peak_memory_bytes,coarse_energy_consumed_joules,coarse_quantity_value,coarse_residual_final,coarse_residual_reduction_ratio,coarse_quantity_history_drift_absolute,coarse_mass_imbalance_fraction,coarse_energy_imbalance_fraction,medium_run_id,medium_grid_id,medium_grid_level,medium_spacing,medium_mesh_sha256,medium_mesh_node_count,medium_solver_exit_code,medium_iterations_completed,medium_cpu_core_hours,medium_gpu_hours,medium_wall_clock_seconds,medium_peak_memory_bytes,medium_energy_consumed_joules,medium_quantity_value,medium_residual_final,medium_residual_reduction_ratio,medium_quantity_history_drift_absolute,medium_mass_imbalance_fraction,medium_energy_imbalance_fraction,fine_run_id,fine_grid_id,fine_grid_level,fine_spacing,fine_mesh_sha256,fine_mesh_node_count,fine_solver_exit_code,fine_iterations_completed,fine_cpu_core_hours,fine_gpu_hours,fine_wall_clock_seconds,fine_peak_memory_bytes,fine_energy_consumed_joules,fine_quantity_value,fine_residual_final,fine_residual_reduction_ratio,fine_quantity_history_drift_absolute,fine_mass_imbalance_fraction,fine_energy_imbalance_fraction,refinement_ratio_coarse_medium,refinement_ratio_medium_fine,observed_order,extrapolated_value,medium_gci_absolute,medium_gci_relative_percent,fine_gci_absolute,fine_gci_relative_percent,asymptotic_ratio,residual_final_max,residual_reduction_ratio_max,quantity_history_drift_absolute_max,mass_imbalance_fraction_max,energy_imbalance_fraction_max,observed_order_min,observed_order_max,asymptotic_ratio_min,asymptotic_ratio_max,study_total_cpu_core_hours,study_total_gpu_hours_measured,gpu_measured_run_count,study_total_wall_clock_seconds,study_peak_memory_bytes,study_total_energy_joules_measured,energy_measured_run_count";

pub fn report_to_csv(report: &CfdEvidenceAdmissionReport) -> Result<String, String> {
    let max_residual_final = report
        .grid_run_audits
        .iter()
        .map(|audit| audit.residual_final)
        .fold(0.0, f64::max);
    let max_residual_reduction_ratio = report
        .grid_run_audits
        .iter()
        .map(|audit| audit.residual_reduction_ratio)
        .fold(0.0, f64::max);
    let max_quantity_history_drift = report
        .grid_run_audits
        .iter()
        .map(|audit| audit.quantity_history_drift_absolute)
        .fold(0.0, f64::max);
    let max_mass_imbalance_fraction = report
        .grid_run_audits
        .iter()
        .map(|audit| audit.mass_imbalance_fraction)
        .fold(0.0, f64::max);
    let max_energy_imbalance_fraction = report
        .grid_run_audits
        .iter()
        .map(|audit| audit.energy_imbalance_fraction)
        .fold(0.0, f64::max);

    let mut output = format!("{ADMISSION_CSV_HEADER}\n");
    for triplet in &report.gci_report.triplets {
        let coarse = report
            .grid_run_audits
            .iter()
            .find(|audit| audit.grid_id == triplet.coarse_grid_id)
            .ok_or_else(|| format!("missing coarse run audit for {}", triplet.coarse_grid_id))?;
        let medium = report
            .grid_run_audits
            .iter()
            .find(|audit| audit.grid_id == triplet.medium_grid_id)
            .ok_or_else(|| format!("missing medium run audit for {}", triplet.medium_grid_id))?;
        let fine = report
            .grid_run_audits
            .iter()
            .find(|audit| audit.grid_id == triplet.fine_grid_id)
            .ok_or_else(|| format!("missing fine run audit for {}", triplet.fine_grid_id))?;

        let mut columns = vec![
            csv_escape(&report.case_id),
            csv_escape(&report.quantity),
            report.status.to_owned(),
            csv_escape(&report.source_reference),
            csv_escape(&report.spacing_definition),
            report.grid_family_kind.as_str().to_owned(),
            report.solver_build_sha256.clone(),
            report.physics_config_sha256.clone(),
            report.geometry_sha256.clone(),
            format!("{:.8}", report.safety_factor),
        ];
        for audit in [coarse, medium, fine] {
            columns.extend([
                csv_escape(&audit.run_id),
                csv_escape(&audit.grid_id),
                audit.grid_level.to_string(),
                format!("{:.12}", audit.characteristic_spacing),
                audit.mesh_sha256.clone(),
                audit.mesh_node_count.to_string(),
                audit.solver_exit_code.to_string(),
                audit.iterations_completed.to_string(),
                format!("{:.12}", audit.cpu_core_hours),
                optional_decimal(audit.gpu_hours),
                format!("{:.12}", audit.wall_clock_seconds),
                audit.peak_memory_bytes.to_string(),
                optional_decimal(audit.energy_consumed_joules),
                format!("{:.12}", audit.quantity_value),
                format!("{:.12}", audit.residual_final),
                format!("{:.12}", audit.residual_reduction_ratio),
                format!("{:.12}", audit.quantity_history_drift_absolute),
                format!("{:.12}", audit.mass_imbalance_fraction),
                format!("{:.12}", audit.energy_imbalance_fraction),
            ]);
        }
        columns.extend([
            format!("{:.12}", triplet.refinement_ratio_coarse_medium),
            format!("{:.12}", triplet.refinement_ratio_medium_fine),
            format!("{:.12}", triplet.observed_order),
            format!("{:.12}", triplet.extrapolated_value),
            format!("{:.12}", triplet.medium_gci_absolute),
            triplet
                .medium_gci_relative_percent
                .map(|value| format!("{value:.12}"))
                .unwrap_or_default(),
            format!("{:.12}", triplet.fine_gci_absolute),
            triplet
                .fine_gci_relative_percent
                .map(|value| format!("{value:.12}"))
                .unwrap_or_default(),
            format!("{:.12}", triplet.asymptotic_ratio),
            format!("{:.12}", max_residual_final),
            format!("{:.12}", max_residual_reduction_ratio),
            format!("{:.12}", max_quantity_history_drift),
            format!("{:.12}", max_mass_imbalance_fraction),
            format!("{:.12}", max_energy_imbalance_fraction),
            format!("{:.12}", report.criteria.observed_order_min),
            format!("{:.12}", report.criteria.observed_order_max),
            format!("{:.12}", report.criteria.asymptotic_ratio_min),
            format!("{:.12}", report.criteria.asymptotic_ratio_max),
            format!("{:.12}", report.total_cpu_core_hours),
            optional_decimal(report.total_gpu_hours_measured),
            report.gpu_measured_run_count.to_string(),
            format!("{:.12}", report.total_wall_clock_seconds),
            report.peak_memory_bytes.to_string(),
            optional_decimal(report.total_energy_joules_measured),
            report.energy_measured_run_count.to_string(),
        ]);
        output.push_str(&columns.join(","));
        output.push('\n');
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(byte: char) -> String {
        byte.to_string().repeat(64)
    }

    fn study() -> CfdEvidenceStudy {
        let spacings = [4.0_f64, 2.0, 1.0, 0.5, 0.25, 0.125];
        CfdEvidenceStudy {
            case_id: "synthetic-v1a".to_owned(),
            quantity: "CD".to_owned(),
            source_reference: "synthetic-known-answer-not-cfd".to_owned(),
            spacing_definition: "explicit synthetic characteristic spacing".to_owned(),
            grid_family_kind: GridFamilyKind::Structured,
            safety_factor: 1.25,
            criteria: AdmissionCriteria {
                residual_final_max: 1.0e-8,
                residual_reduction_ratio_max: 1.0e-7,
                quantity_history_drift_absolute_max: 1.0e-9,
                mass_imbalance_fraction_max: 1.0e-6,
                energy_imbalance_fraction_max: 1.0e-6,
                observed_order_min: 1.0,
                observed_order_max: 3.0,
                asymptotic_ratio_min: 0.9,
                asymptotic_ratio_max: 1.1,
            },
            runs: spacings
                .iter()
                .enumerate()
                .map(|(index, spacing)| {
                    let value = 2.0 + 0.8 * spacing.powi(2);
                    let run_number =
                        f64::from(u32::try_from(index + 1).expect("six fixture grids fit u32"));
                    CfdGridRun {
                        run_id: format!("run-{}", index + 1),
                        grid_id: format!("grid-{}", index + 1),
                        grid_level: u32::try_from(index + 1).expect("six fixture grids fit u32"),
                        characteristic_spacing: *spacing,
                        mesh_sha256: digest(
                            char::from_digit(
                                u32::try_from(index + 1).expect("six fixture grids fit u32"),
                                10,
                            )
                            .expect("grid IDs use base-10 digits"),
                        ),
                        solver_build_sha256: digest('a'),
                        physics_config_sha256: digest('b'),
                        geometry_sha256: digest('c'),
                        solver_exit_code: 0,
                        iterations_completed: 5000 + u64::try_from(index).expect("index fits u64"),
                        mesh_node_count: 10_000 * u64::try_from(index + 1).expect("index fits u64"),
                        cpu_core_hours: 0.25 * run_number,
                        gpu_hours: if index % 2 == 0 { Some(0.5) } else { None },
                        wall_clock_seconds: 120.0 * run_number,
                        peak_memory_bytes: 1024 * u64::try_from(index + 1).expect("index fits u64"),
                        energy_consumed_joules: if index < 4 {
                            Some(1000.0 * run_number)
                        } else {
                            None
                        },
                        residual_initial: 1.0e-1,
                        residual_final: 1.0e-10,
                        quantity_history: vec![value; 6],
                        mass_imbalance_fraction: 1.0e-8,
                        energy_imbalance_fraction: 2.0e-8,
                        quantity_value: value,
                    }
                })
                .collect(),
        }
    }

    #[test]
    fn admits_six_grid_known_answer_only_under_explicit_criteria() {
        let report = evaluate_cfd_evidence_admission(&study())
            .expect("synthetic numerical evidence meets declared criteria");
        assert_eq!(
            report.status,
            "numerically_admitted_under_declared_criteria_caller_hashes_unverified_not_experimentally_validated"
        );
        assert_eq!(report.grid_run_audits.len(), 6);
        assert_eq!(report.gci_report.triplets.len(), 4);
        assert!(
            report
                .gci_report
                .triplets
                .iter()
                .all(|triplet| (triplet.observed_order - 2.0).abs() < 1.0e-10)
        );
    }

    #[test]
    fn rejects_solver_config_geometry_or_mesh_identity_changes() {
        let mut changed = study();
        changed.runs[2].physics_config_sha256 = digest('d');
        assert!(
            evaluate_cfd_evidence_admission(&changed)
                .expect_err("mixed physics config rejected")
                .contains("changes solver build")
        );

        let mut changed = study();
        changed.runs[2].geometry_sha256 = digest('d');
        assert!(
            evaluate_cfd_evidence_admission(&changed)
                .expect_err("geometry change rejected")
                .contains("changes solver build")
        );

        let mut changed = study();
        changed.runs[2].mesh_sha256 = changed.runs[1].mesh_sha256.clone();
        assert!(
            evaluate_cfd_evidence_admission(&changed)
                .expect_err("mesh digest reused")
                .contains("reuses a mesh SHA-256")
        );
    }

    #[test]
    fn rejects_unconverged_residual_or_unstable_force_history() {
        let mut changed = study();
        changed.runs[1].solver_exit_code = 17;
        assert!(
            evaluate_cfd_evidence_admission(&changed)
                .expect_err("non-zero solver exit rejected")
                .contains("non-zero solver exit code")
        );

        let mut changed = study();
        changed.runs[1].residual_final = 1.0e-4;
        assert!(
            evaluate_cfd_evidence_admission(&changed)
                .expect_err("residual criteria fail")
                .contains("fails residual convergence")
        );

        let mut changed = study();
        changed.runs[1].quantity_history = vec![0.1, 0.1, 0.1, 0.2, 0.2, 0.2];
        assert!(
            evaluate_cfd_evidence_admission(&changed)
                .expect_err("unstable monitor rejected")
                .contains("quantity-history stability")
        );
    }

    #[test]
    fn rejects_conservation_violations_and_nonmonotonic_grid_order() {
        let mut changed = study();
        changed.runs[2].mass_imbalance_fraction = 1.0e-3;
        assert!(
            evaluate_cfd_evidence_admission(&changed)
                .expect_err("mass defect rejected")
                .contains("fails mass conservation")
        );

        let mut changed = study();
        changed.runs[2].energy_imbalance_fraction = 1.0e-3;
        assert!(
            evaluate_cfd_evidence_admission(&changed)
                .expect_err("energy defect rejected")
                .contains("fails energy conservation")
        );

        let mut changed = study();
        changed.runs[2].quantity_value = 10.0;
        assert!(
            evaluate_cfd_evidence_admission(&changed)
                .expect_err("oscillatory refinement rejected")
                .contains("non-monotonic")
        );
    }

    #[test]
    fn rejects_nonincreasing_mesh_node_counts_even_when_spacing_decreases() {
        let mut changed = study();
        changed.runs[3].mesh_node_count = changed.runs[2].mesh_node_count;
        assert!(
            evaluate_cfd_evidence_admission(&changed)
                .expect_err("a finer level must add nodes")
                .contains("increasing mesh node counts")
        );
    }

    #[test]
    fn rejects_bad_digest_and_invalid_criteria() {
        let mut changed = study();
        changed.runs[0].mesh_sha256 = "not-a-sha".to_owned();
        assert!(evaluate_cfd_evidence_admission(&changed).is_err());

        let mut changed = study();
        changed.criteria.asymptotic_ratio_min = 1.2;
        changed.criteria.asymptotic_ratio_max = 0.8;
        assert!(evaluate_cfd_evidence_admission(&changed).is_err());
    }

    #[test]
    fn rejects_missing_measurement_resources_and_nonzero_execution_state() {
        let mut changed = study();
        changed.runs[2].cpu_core_hours = 0.0;
        assert!(
            evaluate_cfd_evidence_admission(&changed)
                .expect_err("missing CPU cost rejected")
                .contains("resource times")
        );

        let mut changed = study();
        changed.runs[2].iterations_completed = 0;
        assert!(
            evaluate_cfd_evidence_admission(&changed)
                .expect_err("missing iteration count rejected")
                .contains("iteration, mesh-node, or peak-memory")
        );

        let mut changed = study();
        changed.runs[2].mesh_node_count = 0;
        assert!(evaluate_cfd_evidence_admission(&changed).is_err());

        let mut changed = study();
        changed.runs[2].gpu_hours = Some(f64::NAN);
        assert!(
            evaluate_cfd_evidence_admission(&changed)
                .expect_err("non-finite GPU measurement rejected")
                .contains("gpu_hours")
        );
    }

    #[test]
    fn unmeasured_gpu_and_energy_are_blank_and_counted_as_missing() {
        let report = evaluate_cfd_evidence_admission(&study()).expect("valid synthetic study");
        assert_eq!(report.gpu_measured_run_count, 3);
        assert_eq!(report.energy_measured_run_count, 4);
        let csv = report_to_csv(&report).expect("CSV export");
        let headers = ADMISSION_CSV_HEADER.split(',').collect::<Vec<_>>();
        let first_row = csv
            .lines()
            .nth(1)
            .expect("first triplet row")
            .split(',')
            .collect::<Vec<_>>();
        let medium_gpu_index = headers
            .iter()
            .position(|name| *name == "medium_gpu_hours")
            .expect("medium GPU column");
        assert_eq!(first_row[medium_gpu_index], "");
        let fourth_row = csv
            .lines()
            .nth(4)
            .expect("fourth triplet row")
            .split(',')
            .collect::<Vec<_>>();
        let fine_energy_index = headers
            .iter()
            .position(|name| *name == "fine_energy_consumed_joules")
            .expect("fine energy column");
        assert_eq!(fourth_row[fine_energy_index], "");
        assert!(csv.contains("study_total_gpu_hours_measured"));
        assert!(csv.contains("study_total_energy_joules_measured"));
    }

    #[test]
    fn csv_output_is_deterministic_and_keeps_nonvalidation_label() {
        let report = evaluate_cfd_evidence_admission(&study()).expect("valid synthetic study");
        let first = report_to_csv(&report).expect("complete provenance joins");
        assert_eq!(
            first,
            report_to_csv(&report).expect("complete provenance joins")
        );
        assert_eq!(first.lines().count(), 5);
        assert!(first.contains("not_experimentally_validated"));
        assert!(first.contains("coarse_mesh_sha256"));
        assert!(first.contains("coarse_cpu_core_hours"));
        assert!(first.contains("study_total_cpu_core_hours"));
        assert!(first.contains(
            ",5.250000000000,0.150000000000,3,2520.000000000000,6144,10000.000000000000,4"
        ));
        assert_eq!(report.total_cpu_core_hours, 5.25);
        assert_eq!(report.gpu_measured_run_count, 3);
        assert_eq!(report.total_gpu_hours_measured, Some(1.5));
        assert_eq!(report.energy_measured_run_count, 4);
        assert_eq!(report.total_energy_joules_measured, Some(10_000.0));
        let coarse_mesh_digest = digest('1');
        let fine_mesh_digest = digest('6');
        assert!(first.contains(coarse_mesh_digest.as_str()));
        assert!(first.contains(fine_mesh_digest.as_str()));
    }
}
