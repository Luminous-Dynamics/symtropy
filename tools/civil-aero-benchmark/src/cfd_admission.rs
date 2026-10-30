//! Admission gate for steady CFD evidence under explicit verification criteria.
//!
//! Passing means only that supplied run records satisfy the declared numerical checks and
//! the scalar grid-convergence study. It is not experimental validation, model validation,
//! flight certification, or proof that caller-supplied metadata is authentic.

use std::collections::HashSet;

use crate::grid_convergence::{
    GciReport, GciStudy, GridObservation, evaluate_grid_convergence,
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
    pub iterative_converged: bool,
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
    pub safety_factor: f64,
    pub criteria: AdmissionCriteria,
    /// Ordered coarse-to-fine; at least three runs are required.
    pub runs: Vec<CfdGridRun>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CfdGridRunAudit {
    pub run_id: String,
    pub grid_id: String,
    pub residual_final: f64,
    pub residual_reduction_ratio: f64,
    pub quantity_history_drift_absolute: f64,
    pub mass_imbalance_fraction: f64,
    pub energy_imbalance_fraction: f64,
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
    pub safety_factor: f64,
    pub criteria: AdmissionCriteria,
    pub status: &'static str,
    pub grid_run_audits: Vec<CfdGridRunAudit>,
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
        return Err("case_id, quantity, source_reference, and spacing_definition must be non-empty".to_owned());
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
            return Err(format!("grid run {}: grid_id is empty or duplicated", run.run_id));
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
        if !run.iterative_converged {
            return Err(format!("grid run {} is not marked iteratively converged", run.run_id));
        }

        for (name, value) in [
            ("characteristic_spacing", run.characteristic_spacing),
            ("residual_initial", run.residual_initial),
            ("residual_final", run.residual_final),
            ("mass_imbalance_fraction", run.mass_imbalance_fraction),
            ("energy_imbalance_fraction", run.energy_imbalance_fraction),
            ("quantity_value", run.quantity_value),
        ] {
            if !value.is_finite() {
                return Err(format!("grid run {}: {name} is non-finite", run.run_id));
            }
        }
        if run.characteristic_spacing <= 0.0
            || run.residual_initial <= 0.0
            || run.residual_final <= 0.0
            || run.mass_imbalance_fraction < 0.0
            || run.energy_imbalance_fraction < 0.0
        {
            return Err(format!(
                "grid run {}: spacing/residuals must be positive and conservation defects non-negative",
                run.run_id
            ));
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
        if quantity_history_drift_absolute
            > study.criteria.quantity_history_drift_absolute_max
        {
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
            residual_final: run.residual_final,
            residual_reduction_ratio,
            quantity_history_drift_absolute,
            mass_imbalance_fraction: run.mass_imbalance_fraction,
            energy_imbalance_fraction: run.energy_imbalance_fraction,
        });
    }

    let gci_study = GciStudy {
        quantity: study.quantity.clone(),
        source_reference: study.source_reference.clone(),
        spacing_definition: study.spacing_definition.clone(),
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

    Ok(CfdEvidenceAdmissionReport {
        case_id: study.case_id.clone(),
        quantity: study.quantity.clone(),
        source_reference: study.source_reference.clone(),
        spacing_definition: study.spacing_definition.clone(),
        solver_build_sha256: first.solver_build_sha256.clone(),
        physics_config_sha256: first.physics_config_sha256.clone(),
        geometry_sha256: first.geometry_sha256.clone(),
        safety_factor: study.safety_factor,
        criteria: study.criteria.clone(),
        status: "numerically_admitted_under_declared_criteria_not_experimentally_validated",
        grid_run_audits: run_audits,
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

pub const ADMISSION_CSV_HEADER: &str = "case_id,quantity,status,source_reference,spacing_definition,solver_build_sha256,physics_config_sha256,geometry_sha256,safety_factor,coarse_grid_id,medium_grid_id,fine_grid_id,observed_order,extrapolated_value,medium_gci_absolute,fine_gci_absolute,asymptotic_ratio,max_residual_final,max_residual_reduction_ratio,max_quantity_history_drift,max_mass_imbalance_fraction,max_energy_imbalance_fraction";

pub fn report_to_csv(report: &CfdEvidenceAdmissionReport) -> String {
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
        output.push_str(&format!(
            "{},{},{},{},{},{},{},{},{:.8},{},{},{},{:.12},{:.12},{:.12},{:.12},{:.12},{:.12},{:.12},{:.12},{:.12},{:.12}\n",
            csv_escape(&report.case_id),
            csv_escape(&report.quantity),
            report.status,
            csv_escape(&report.source_reference),
            csv_escape(&report.spacing_definition),
            report.solver_build_sha256,
            report.physics_config_sha256,
            report.geometry_sha256,
            report.safety_factor,
            csv_escape(&triplet.coarse_grid_id),
            csv_escape(&triplet.medium_grid_id),
            csv_escape(&triplet.fine_grid_id),
            triplet.observed_order,
            triplet.extrapolated_value,
            triplet.medium_gci_absolute,
            triplet.fine_gci_absolute,
            triplet.asymptotic_ratio,
            max_residual_final,
            max_residual_reduction_ratio,
            max_quantity_history_drift,
            max_mass_imbalance_fraction,
            max_energy_imbalance_fraction,
        ));
    }
    output
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
                    CfdGridRun {
                        run_id: format!("run-{}", index + 1),
                        grid_id: format!("grid-{}", index + 1),
                        grid_level: u32::try_from(index + 1).expect("six fixture grids fit u32"),
                        characteristic_spacing: *spacing,
                        mesh_sha256: digest(
                            char::from_digit(u32::try_from(index + 1).expect("six fixture grids fit u32"), 10)
                                .expect("grid IDs use base-10 digits"),
                        ),
                        solver_build_sha256: digest('a'),
                        physics_config_sha256: digest('b'),
                        geometry_sha256: digest('c'),
                        iterative_converged: true,
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
        assert_eq!(report.status, "numerically_admitted_under_declared_criteria_not_experimentally_validated");
        assert_eq!(report.grid_run_audits.len(), 6);
        assert_eq!(report.gci_report.triplets.len(), 4);
        assert!(report
            .gci_report
            .triplets
            .iter()
            .all(|triplet| (triplet.observed_order - 2.0).abs() < 1.0e-10));
    }

    #[test]
    fn rejects_solver_config_geometry_or_mesh_identity_changes() {
        let mut changed = study();
        changed.runs[2].physics_config_sha256 = digest('d');
        assert!(evaluate_cfd_evidence_admission(&changed)
            .expect_err("mixed physics config rejected")
            .contains("changes solver build"));

        let mut changed = study();
        changed.runs[2].geometry_sha256 = digest('d');
        assert!(evaluate_cfd_evidence_admission(&changed)
            .expect_err("geometry change rejected")
            .contains("changes solver build"));

        let mut changed = study();
        changed.runs[2].mesh_sha256 = changed.runs[1].mesh_sha256.clone();
        assert!(evaluate_cfd_evidence_admission(&changed)
            .expect_err("mesh digest reused")
            .contains("reuses a mesh SHA-256"));
    }

    #[test]
    fn rejects_unconverged_residual_or_unstable_force_history() {
        let mut changed = study();
        changed.runs[1].iterative_converged = false;
        assert!(evaluate_cfd_evidence_admission(&changed)
            .expect_err("unconverged run rejected")
            .contains("not marked iteratively converged"));

        let mut changed = study();
        changed.runs[1].residual_final = 1.0e-4;
        assert!(evaluate_cfd_evidence_admission(&changed)
            .expect_err("residual criteria fail")
            .contains("fails residual convergence"));

        let mut changed = study();
        changed.runs[1].quantity_history = vec![0.1, 0.1, 0.1, 0.2, 0.2, 0.2];
        assert!(evaluate_cfd_evidence_admission(&changed)
            .expect_err("unstable monitor rejected")
            .contains("quantity-history stability"));
    }

    #[test]
    fn rejects_conservation_violations_and_nonmonotonic_grid_order() {
        let mut changed = study();
        changed.runs[2].mass_imbalance_fraction = 1.0e-3;
        assert!(evaluate_cfd_evidence_admission(&changed)
            .expect_err("mass defect rejected")
            .contains("fails mass conservation"));

        let mut changed = study();
        changed.runs[2].energy_imbalance_fraction = 1.0e-3;
        assert!(evaluate_cfd_evidence_admission(&changed)
            .expect_err("energy defect rejected")
            .contains("fails energy conservation"));

        let mut changed = study();
        changed.runs[2].quantity_value = 10.0;
        assert!(evaluate_cfd_evidence_admission(&changed)
            .expect_err("oscillatory refinement rejected")
            .contains("non-monotonic"));
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
    fn csv_output_is_deterministic_and_keeps_nonvalidation_label() {
        let report = evaluate_cfd_evidence_admission(&study()).expect("valid synthetic study");
        let first = report_to_csv(&report);
        assert_eq!(first, report_to_csv(&report));
        assert_eq!(first.lines().count(), 5);
        assert!(first.contains("not_experimentally_validated"));
    }
}
