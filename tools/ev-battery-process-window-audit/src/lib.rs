use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum EvidenceClass {
    SyntheticFixture,
    Assumption,
    VendorClaim,
    LiteratureReported,
    Simulation,
    Surrogate,
    Calculation,
    PhysicalMeasurement,
    IndependentLaboratory,
}

impl EvidenceClass {
    fn is_physical(self) -> bool {
        matches!(
            self,
            Self::PhysicalMeasurement | Self::IndependentLaboratory
        )
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceAdmission {
    Candidate,
    AdmittedForResearch,
    AdmittedForEvaluation,
    Blocked,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceRecord {
    pub evidence_id: String,
    pub evidence_class: EvidenceClass,
    pub source_digest: String,
    pub claim_scope: String,
    pub validity_domain: String,
    pub admission: EvidenceAdmission,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProcessParameterSpec {
    pub parameter_id: String,
    pub unit: String,
    pub lower_bound: f64,
    pub upper_bound: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct QualityMetricSpec {
    pub metric_id: String,
    pub unit: String,
    pub boundary: String,
    pub lower_limit: Option<f64>,
    pub upper_limit: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProcessParameterObservation {
    pub parameter_id: String,
    pub unit: String,
    pub value: f64,
    pub evidence_id: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeasurementStatus {
    Observed,
    NotMeasured,
    InvalidMeasurement,
}

#[derive(Clone, Debug, PartialEq)]
pub struct QualityObservation {
    pub metric_id: String,
    pub unit: String,
    pub boundary: String,
    pub status: MeasurementStatus,
    pub value: Option<f64>,
    pub evidence_id: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProductionCellRun {
    pub run_id: String,
    pub cell_id: String,
    pub lot_id: String,
    pub production_line_id: String,
    pub equipment_id: String,
    pub recipe_ref: String,
    pub recipe_revision: String,
    pub cell_configuration_ref: String,
    pub process_parameters: Vec<ProcessParameterObservation>,
    pub quality_observations: Vec<QualityObservation>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProcessWindowManifest {
    pub manifest_id: String,
    pub recipe_ref: String,
    pub recipe_revision: String,
    pub cell_configuration_ref: String,
    pub process_parameters: Vec<ProcessParameterSpec>,
    pub quality_specs: Vec<QualityMetricSpec>,
    pub evidence: Vec<EvidenceRecord>,
    pub production_runs: Vec<ProductionCellRun>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RunEvidenceTier {
    PhysicalAdmitted,
    NonPhysicalAdmitted,
    NotAdmittedForEvaluation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CellQualityDisposition {
    Conforming,
    Nonconforming,
    Incomplete,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CellRunAudit {
    pub run_id: String,
    pub cell_id: String,
    pub lot_id: String,
    pub production_line_id: String,
    pub evidence_tier: RunEvidenceTier,
    pub process_in_window: bool,
    pub disposition: CellQualityDisposition,
    pub measured_metric_count: usize,
    pub unmeasured_metric_count: usize,
    pub invalid_metric_count: usize,
    pub out_of_spec_metric_ids: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LotSummary {
    pub production_line_id: String,
    pub lot_id: String,
    pub evidence_tier: RunEvidenceTier,
    pub run_count: usize,
    pub in_window_run_count: usize,
    pub out_of_window_run_count: usize,
    pub conforming_in_window_count: usize,
    pub nonconforming_in_window_count: usize,
    pub incomplete_in_window_count: usize,
    pub first_pass_yield: Option<f64>,
    pub process_window_adherence_rate: Option<f64>,
    pub in_window_complete_conformance_rate: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LineSummary {
    pub production_line_id: String,
    pub evidence_tier: RunEvidenceTier,
    pub lot_count: usize,
    pub run_count: usize,
    pub in_window_run_count: usize,
    pub out_of_window_run_count: usize,
    pub conforming_in_window_count: usize,
    pub nonconforming_in_window_count: usize,
    pub incomplete_in_window_count: usize,
    pub first_pass_yield: Option<f64>,
    pub process_window_adherence_rate: Option<f64>,
    pub in_window_complete_conformance_rate: Option<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ProcessWindowAuditReport {
    pub manifest_id: String,
    pub run_audits: Vec<CellRunAudit>,
    pub lot_summaries: Vec<LotSummary>,
    pub line_summaries: Vec<LineSummary>,
    pub physical_evidence_run_count: usize,
    pub non_physical_evidence_run_count: usize,
    pub unadmitted_evidence_run_count: usize,
    pub out_of_window_run_count: usize,
    pub incomplete_in_window_run_count: usize,
    pub claim_ceiling: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProcessWindowAuditError(pub String);

impl std::fmt::Display for ProcessWindowAuditError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for ProcessWindowAuditError {}

fn require_nonempty(value: &str, label: &str) -> Result<(), ProcessWindowAuditError> {
    if value.trim().is_empty() {
        return Err(ProcessWindowAuditError(format!(
            "{label} must not be empty"
        )));
    }
    Ok(())
}

fn validate_unique_ids<'a>(
    values: impl IntoIterator<Item = &'a str>,
    label: &str,
) -> Result<(), ProcessWindowAuditError> {
    let mut seen = BTreeSet::new();

    for value in values {
        require_nonempty(value, label)?;
        if !seen.insert(value) {
            return Err(ProcessWindowAuditError(format!(
                "duplicate {label}: {value}"
            )));
        }
    }

    Ok(())
}

fn validate_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn evidence_registry<'a>(
    evidence: &'a [EvidenceRecord],
) -> Result<BTreeMap<&'a str, &'a EvidenceRecord>, ProcessWindowAuditError> {
    if evidence.is_empty() {
        return Err(ProcessWindowAuditError(
            "at least one evidence record is required".into(),
        ));
    }

    validate_unique_ids(
        evidence.iter().map(|record| record.evidence_id.as_str()),
        "evidence ID",
    )?;

    let mut registry = BTreeMap::new();

    for record in evidence {
        if !validate_digest(&record.source_digest) {
            return Err(ProcessWindowAuditError(format!(
                "evidence {} has an invalid lowercase SHA-256 digest",
                record.evidence_id
            )));
        }
        require_nonempty(&record.claim_scope, "evidence claim scope")?;
        require_nonempty(&record.validity_domain, "evidence validity domain")?;
        registry.insert(record.evidence_id.as_str(), record);
    }

    Ok(registry)
}

fn resolve_evidence<'a>(
    registry: &BTreeMap<&'a str, &'a EvidenceRecord>,
    evidence_id: &str,
    owner: &str,
) -> Result<&'a EvidenceRecord, ProcessWindowAuditError> {
    registry.get(evidence_id).copied().ok_or_else(|| {
        ProcessWindowAuditError(format!(
            "{owner} references unknown evidence ID {evidence_id}"
        ))
    })
}

fn evidence_tier(
    registry: &BTreeMap<&str, &EvidenceRecord>,
    parameter_observations: &[ProcessParameterObservation],
    quality_observations: &[QualityObservation],
) -> Result<RunEvidenceTier, ProcessWindowAuditError> {
    let mut all_physical = true;
    let mut all_admitted = true;

    for id in parameter_observations
        .iter()
        .map(|item| item.evidence_id.as_str())
        .chain(
            quality_observations
                .iter()
                .map(|item| item.evidence_id.as_str()),
        )
    {
        let record = resolve_evidence(registry, id, "production observation")?;

        if record.admission != EvidenceAdmission::AdmittedForEvaluation {
            all_admitted = false;
        }
        if !record.evidence_class.is_physical() {
            all_physical = false;
        }
    }

    if !all_admitted {
        Ok(RunEvidenceTier::NotAdmittedForEvaluation)
    } else if all_physical {
        Ok(RunEvidenceTier::PhysicalAdmitted)
    } else {
        Ok(RunEvidenceTier::NonPhysicalAdmitted)
    }
}

fn validate_specs(manifest: &ProcessWindowManifest) -> Result<(), ProcessWindowAuditError> {
    if manifest.process_parameters.is_empty() || manifest.quality_specs.is_empty() {
        return Err(ProcessWindowAuditError(
            "at least one process parameter and quality specification are required".into(),
        ));
    }

    validate_unique_ids(
        manifest
            .process_parameters
            .iter()
            .map(|item| item.parameter_id.as_str()),
        "process parameter ID",
    )?;
    validate_unique_ids(
        manifest.quality_specs.iter().map(|item| item.metric_id.as_str()),
        "quality metric ID",
    )?;

    for spec in &manifest.process_parameters {
        require_nonempty(&spec.unit, "process parameter unit")?;
        if !spec.lower_bound.is_finite()
            || !spec.upper_bound.is_finite()
            || spec.lower_bound > spec.upper_bound
        {
            return Err(ProcessWindowAuditError(format!(
                "process parameter {} has invalid window bounds",
                spec.parameter_id
            )));
        }
    }

    for spec in &manifest.quality_specs {
        require_nonempty(&spec.unit, "quality metric unit")?;
        require_nonempty(&spec.boundary, "quality metric boundary")?;

        match (spec.lower_limit, spec.upper_limit) {
            (None, None) => {
                return Err(ProcessWindowAuditError(format!(
                    "quality metric {} has no specification limits",
                    spec.metric_id
                )));
            }
            (Some(lower), Some(upper))
                if !lower.is_finite() || !upper.is_finite() || lower > upper =>
            {
                return Err(ProcessWindowAuditError(format!(
                    "quality metric {} has invalid specification limits",
                    spec.metric_id
                )));
            }
            (Some(lower), None) if !lower.is_finite() => {
                return Err(ProcessWindowAuditError(format!(
                    "quality metric {} has invalid lower limit",
                    spec.metric_id
                )));
            }
            (None, Some(upper)) if !upper.is_finite() => {
                return Err(ProcessWindowAuditError(format!(
                    "quality metric {} has invalid upper limit",
                    spec.metric_id
                )));
            }
            _ => {}
        }
    }

    Ok(())
}

fn validate_run_identity(
    manifest: &ProcessWindowManifest,
    run: &ProductionCellRun,
) -> Result<(), ProcessWindowAuditError> {
    for (value, label) in [
        (run.run_id.as_str(), "run ID"),
        (run.cell_id.as_str(), "cell ID"),
        (run.lot_id.as_str(), "lot ID"),
        (run.production_line_id.as_str(), "production line ID"),
        (run.equipment_id.as_str(), "equipment ID"),
        (run.recipe_ref.as_str(), "run recipe reference"),
        (run.recipe_revision.as_str(), "run recipe revision"),
        (
            run.cell_configuration_ref.as_str(),
            "run cell configuration reference",
        ),
    ] {
        require_nonempty(value, label)?;
    }

    if run.recipe_ref != manifest.recipe_ref
        || run.recipe_revision != manifest.recipe_revision
        || run.cell_configuration_ref != manifest.cell_configuration_ref
    {
        return Err(ProcessWindowAuditError(format!(
            "run {} has a recipe or cell-configuration identity mismatch",
            run.run_id
        )));
    }

    Ok(())
}

fn process_window_disposition(
    specs: &[ProcessParameterSpec],
    observations: &[ProcessParameterObservation],
) -> Result<bool, ProcessWindowAuditError> {
    validate_unique_ids(
        observations.iter().map(|item| item.parameter_id.as_str()),
        "observed process parameter ID",
    )?;

    if observations.len() != specs.len() {
        return Err(ProcessWindowAuditError(format!(
            "process parameter observation count {} does not match required count {}",
            observations.len(),
            specs.len()
        )));
    }

    let mut in_window = true;

    for spec in specs {
        let observation = observations
            .iter()
            .find(|item| item.parameter_id == spec.parameter_id)
            .ok_or_else(|| {
                ProcessWindowAuditError(format!(
                    "missing process parameter observation {}",
                    spec.parameter_id
                ))
            })?;

        if observation.unit != spec.unit || !observation.value.is_finite() {
            return Err(ProcessWindowAuditError(format!(
                "process parameter {} has mismatched unit or invalid value",
                spec.parameter_id
            )));
        }

        if observation.value < spec.lower_bound || observation.value > spec.upper_bound {
            in_window = false;
        }
    }

    Ok(in_window)
}

fn quality_disposition(
    specs: &[QualityMetricSpec],
    observations: &[QualityObservation],
) -> Result<
    (
        CellQualityDisposition,
        usize,
        usize,
        usize,
        Vec<String>,
    ),
    ProcessWindowAuditError,
> {
    validate_unique_ids(
        observations.iter().map(|item| item.metric_id.as_str()),
        "quality observation metric ID",
    )?;

    if observations.len() != specs.len() {
        return Err(ProcessWindowAuditError(format!(
            "quality observation count {} does not match required count {}",
            observations.len(),
            specs.len()
        )));
    }

    let mut measured = 0usize;
    let mut unmeasured = 0usize;
    let mut invalid = 0usize;
    let mut out_of_spec = Vec::new();

    for spec in specs {
        let observation = observations
            .iter()
            .find(|item| item.metric_id == spec.metric_id)
            .ok_or_else(|| {
                ProcessWindowAuditError(format!(
                    "missing required quality observation {}",
                    spec.metric_id
                ))
            })?;

        if observation.unit != spec.unit || observation.boundary != spec.boundary {
            return Err(ProcessWindowAuditError(format!(
                "quality metric {} has mismatched unit or physical boundary",
                spec.metric_id
            )));
        }

        match observation.status {
            MeasurementStatus::Observed => {
                let value = observation.value.ok_or_else(|| {
                    ProcessWindowAuditError(format!(
                        "observed quality metric {} has no value",
                        spec.metric_id
                    ))
                })?;

                if !value.is_finite() {
                    return Err(ProcessWindowAuditError(format!(
                        "observed quality metric {} is not finite",
                        spec.metric_id
                    )));
                }

                measured += 1;

                if spec.lower_limit.is_some_and(|limit| value < limit)
                    || spec.upper_limit.is_some_and(|limit| value > limit)
                {
                    out_of_spec.push(spec.metric_id.clone());
                }
            }
            MeasurementStatus::NotMeasured => {
                if observation.value.is_some() {
                    return Err(ProcessWindowAuditError(format!(
                        "not-measured quality metric {} must not carry a value",
                        spec.metric_id
                    )));
                }
                unmeasured += 1;
            }
            MeasurementStatus::InvalidMeasurement => {
                if observation.value.is_some() {
                    return Err(ProcessWindowAuditError(format!(
                        "invalid quality metric {} must not carry a qualified value",
                        spec.metric_id
                    )));
                }
                invalid += 1;
            }
        }
    }

    let disposition = if !out_of_spec.is_empty() {
        CellQualityDisposition::Nonconforming
    } else if unmeasured > 0 || invalid > 0 {
        CellQualityDisposition::Incomplete
    } else {
        CellQualityDisposition::Conforming
    };

    out_of_spec.sort();

    Ok((
        disposition,
        measured,
        unmeasured,
        invalid,
        out_of_spec,
    ))
}

#[derive(Clone, Debug, Default)]
struct GroupCounts {
    run_count: usize,
    in_window_run_count: usize,
    out_of_window_run_count: usize,
    conforming_in_window_count: usize,
    nonconforming_in_window_count: usize,
    incomplete_in_window_count: usize,
}

impl GroupCounts {
    fn add(&mut self, audit: &CellRunAudit) {
        self.run_count += 1;

        if !audit.process_in_window {
            self.out_of_window_run_count += 1;
            return;
        }

        self.in_window_run_count += 1;

        match audit.disposition {
            CellQualityDisposition::Conforming => self.conforming_in_window_count += 1,
            CellQualityDisposition::Nonconforming => self.nonconforming_in_window_count += 1,
            CellQualityDisposition::Incomplete => self.incomplete_in_window_count += 1,
        }
    }

    fn first_pass_yield(&self) -> Option<f64> {
        if self.run_count == 0 {
            None
        } else {
            // A strict process-conforming first pass must be both in-window
            // and quality-conforming. Out-of-window and incomplete runs stay
            // in the denominator and cannot be counted as passes.
            Some(self.conforming_in_window_count as f64 / self.run_count as f64)
        }
    }

    fn process_window_adherence_rate(&self) -> Option<f64> {
        if self.run_count == 0 {
            None
        } else {
            Some(self.in_window_run_count as f64 / self.run_count as f64)
        }
    }

    fn in_window_complete_conformance_rate(&self) -> Option<f64> {
        let denominator =
            self.conforming_in_window_count + self.nonconforming_in_window_count;
        if denominator == 0 {
            None
        } else {
            Some(self.conforming_in_window_count as f64 / denominator as f64)
        }
    }
}

/// Audit one declared battery recipe/process window against cell-level run records.
///
/// Strict process-conforming first-pass yield uses all declared runs as its
/// denominator: out-of-window and incomplete runs cannot count as passes.
/// A separate conditional conformance rate is reported only among complete,
/// measured, in-window outcomes. Reports are stratified by lot, line and evidence tier.
/// This is descriptive finite-sample accounting, not a process-capability index,
/// confidence interval, causal model or production qualification.
pub fn audit_process_window(
    manifest: &ProcessWindowManifest,
) -> Result<ProcessWindowAuditReport, ProcessWindowAuditError> {
    require_nonempty(&manifest.manifest_id, "manifest ID")?;
    require_nonempty(&manifest.recipe_ref, "manifest recipe reference")?;
    require_nonempty(&manifest.recipe_revision, "manifest recipe revision")?;
    require_nonempty(
        &manifest.cell_configuration_ref,
        "manifest cell configuration reference",
    )?;
    validate_specs(manifest)?;

    if manifest.production_runs.is_empty() {
        return Err(ProcessWindowAuditError(
            "at least one production cell run is required".into(),
        ));
    }

    validate_unique_ids(
        manifest.production_runs.iter().map(|item| item.run_id.as_str()),
        "production run ID",
    )?;
    validate_unique_ids(
        manifest.production_runs.iter().map(|item| item.cell_id.as_str()),
        "physical cell ID",
    )?;

    let registry = evidence_registry(&manifest.evidence)?;
    let mut run_audits = Vec::with_capacity(manifest.production_runs.len());

    for run in &manifest.production_runs {
        validate_run_identity(manifest, run)?;

        let process_in_window =
            process_window_disposition(&manifest.process_parameters, &run.process_parameters)?;
        let (
            disposition,
            measured_metric_count,
            unmeasured_metric_count,
            invalid_metric_count,
            out_of_spec_metric_ids,
        ) = quality_disposition(&manifest.quality_specs, &run.quality_observations)?;

        let tier = evidence_tier(
            &registry,
            &run.process_parameters,
            &run.quality_observations,
        )?;

        run_audits.push(CellRunAudit {
            run_id: run.run_id.clone(),
            cell_id: run.cell_id.clone(),
            lot_id: run.lot_id.clone(),
            production_line_id: run.production_line_id.clone(),
            evidence_tier: tier,
            process_in_window,
            disposition,
            measured_metric_count,
            unmeasured_metric_count,
            invalid_metric_count,
            out_of_spec_metric_ids,
        });
    }

    run_audits.sort_by(|left, right| left.run_id.cmp(&right.run_id));

    let mut lot_counts: BTreeMap<(String, String, RunEvidenceTier), GroupCounts> =
        BTreeMap::new();
    let mut line_counts: BTreeMap<(String, RunEvidenceTier), GroupCounts> = BTreeMap<>();
    let mut line_lots: BTreeMap<(String, RunEvidenceTier), BTreeSet<String>> =
        BTreeMap::new();

    let mut physical_evidence_run_count = 0usize;
    let mut non_physical_evidence_run_count = 0usize;
    let mut unadmitted_evidence_run_count = 0usize;
    let mut out_of_window_run_count = 0usize;
    let mut incomplete_in_window_run_count = 0usize;

    for audit in &run_audits {
        match audit.evidence_tier {
            RunEvidenceTier::PhysicalAdmitted => physical_evidence_run_count += 1,
            RunEvidenceTier::NonPhysicalAdmitted => non_physical_evidence_run_count += 1,
            RunEvidenceTier::NotAdmittedForEvaluation => unadmitted_evidence_run_count += 1,
        }

        if !audit.process_in_window {
            out_of_window_run_count += 1;
        } else if audit.disposition == CellQualityDisposition::Incomplete {
            incomplete_in_window_run_count += 1;
        }

        lot_counts
            .entry((
                audit.production_line_id.clone(),
                audit.lot_id.clone(),
                audit.evidence_tier,
            ))
            .or_default()
            .add(audit);

        line_counts
            .entry((
                audit.production_line_id.clone(),
                audit.evidence_tier,
            ))
            .or_default()
            .add(audit);

        line_lots
            .entry((
                audit.production_line_id.clone(),
                audit.evidence_tier,
            ))
            .or_default()
            .insert(audit.lot_id.clone());
    }

    let lot_summaries = lot_counts
        .into_iter()
        .map(|((production_line_id, lot_id, evidence_tier), counts)| LotSummary {
            production_line_id,
            lot_id,
            evidence_tier,
            run_count: counts.run_count,
            in_window_run_count: counts.in_window_run_count,
            out_of_window_run_count: counts.out_of_window_run_count,
            conforming_in_window_count: counts.conforming_in_window_count,
            nonconforming_in_window_count: counts.nonconforming_in_window_count,
            incomplete_in_window_count: counts.incomplete_in_window_count,
            first_pass_yield: counts.first_pass_yield(),
            process_window_adherence_rate: counts.process_window_adherence_rate(),
            in_window_complete_conformance_rate: counts.in_window_complete_conformance_rate(),
        })
        .collect();

    let line_summaries = line_counts
        .into_iter()
        .map(|((production_line_id, evidence_tier), counts)| LineSummary {
            lot_count: line_lots
                .get(&(production_line_id.clone(), evidence_tier))
                .map_or(0, BTreeSet::len),
            production_line_id,
            evidence_tier,
            run_count: counts.run_count,
            in_window_run_count: counts.in_window_run_count,
            out_of_window_run_count: counts.out_of_window_run_count,
            conforming_in_window_count: counts.conforming_in_window_count,
            nonconforming_in_window_count: counts.nonconforming_in_window_count,
            incomplete_in_window_count: counts.incomplete_in_window_count,
            first_pass_yield: counts.first_pass_yield(),
            process_window_adherence_rate: counts.process_window_adherence_rate(),
            in_window_complete_conformance_rate: counts.in_window_complete_conformance_rate(),
        })
        .collect();

    Ok(ProcessWindowAuditReport {
        manifest_id: manifest.manifest_id.clone(),
        run_audits,
        lot_summaries,
        line_summaries,
        physical_evidence_run_count,
        non_physical_evidence_run_count,
        unadmitted_evidence_run_count,
        out_of_window_run_count,
        incomplete_in_window_run_count,
        claim_ceiling: "Descriptive first-pass conformance on the exact declared runs, recipe, lots, lines and evidence tiers. Not a statistical process-capability qualification, population-yield guarantee, process-causality claim, safety claim or product release.".into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn evidence(id: &str, class: EvidenceClass) -> EvidenceRecord {
        EvidenceRecord {
            evidence_id: id.into(),
            evidence_class: class,
            source_digest: "a".repeat(64),
            claim_scope: "synthetic test fixture only".into(),
            validity_domain: "unit tests only".into(),
            admission: EvidenceAdmission::AdmittedForEvaluation,
        }
    }

    fn run(id: &str, lot: &str, line: &str, quality: Vec<QualityObservation>) -> ProductionCellRun {
        ProductionCellRun {
            run_id: format!("RUN-{id}"),
            cell_id: format!("CELL-{id}"),
            lot_id: lot.into(),
            production_line_id: line.into(),
            equipment_id: format!("EQUIPMENT-{line}"),
            recipe_ref: "RECIPE-R1".into(),
            recipe_revision: "r1".into(),
            cell_configuration_ref: "CELL-CONFIG-R1".into(),
            process_parameters: vec![
                ProcessParameterObservation {
                    parameter_id: "coating_loading".into(),
                    unit: "mg/cm2".into(),
                    value: 20.0,
                    evidence_id: "PROC-SYNTH".into(),
                },
                ProcessParameterObservation {
                    parameter_id: "calender_density".into(),
                    unit: "g/cm3".into(),
                    value: 2.4,
                    evidence_id: "PROC-SYNTH".into(),
                },
            ],
            quality_observations: quality,
        }
    }

    fn observed_quality(
        capacity: f64,
        resistance: f64,
        evidence_id: &str,
    ) -> Vec<QualityObservation> {
        vec![
            QualityObservation {
                metric_id: "capacity".into(),
                unit: "Ah".into(),
                boundary: "complete_cell".into(),
                status: MeasurementStatus::Observed,
                value: Some(capacity),
                evidence_id: evidence_id.into(),
            },
            QualityObservation {
                metric_id: "dcir".into(),
                unit: "mOhm".into(),
                boundary: "complete_cell_at_50_soc".into(),
                status: MeasurementStatus::Observed,
                value: Some(resistance),
                evidence_id: evidence_id.into(),
            },
        ]
    }

    fn valid_manifest() -> ProcessWindowManifest {
        ProcessWindowManifest {
            manifest_id: "SYNTH-MFG-WINDOW".into(),
            recipe_ref: "RECIPE-R1".into(),
            recipe_revision: "r1".into(),
            cell_configuration_ref: "CELL-CONFIG-R1".into(),
            process_parameters: vec![
                ProcessParameterSpec {
                    parameter_id: "coating_loading".into(),
                    unit: "mg/cm2".into(),
                    lower_bound: 18.0,
                    upper_bound: 22.0,
                },
                ProcessParameterSpec {
                    parameter_id: "calender_density".into(),
                    unit: "g/cm3".into(),
                    lower_bound: 2.2,
                    upper_bound: 2.6,
                },
            ],
            quality_specs: vec![
                QualityMetricSpec {
                    metric_id: "capacity".into(),
                    unit: "Ah".into(),
                    boundary: "complete_cell".into(),
                    lower_limit: Some(4.8),
                    upper_limit: None,
                },
                QualityMetricSpec {
                    metric_id: "dcir".into(),
                    unit: "mOhm".into(),
                    boundary: "complete_cell_at_50_soc".into(),
                    lower_limit: None,
                    upper_limit: Some(30.0),
                },
            ],
            evidence: vec![
                evidence("PROC-SYNTH", EvidenceClass::SyntheticFixture),
                evidence("QUALITY-SYNTH", EvidenceClass::SyntheticFixture),
                evidence("QUALITY-ADMITTED-PHYSICAL", EvidenceClass::PhysicalMeasurement),
            ],
            production_runs: vec![
                run("1", "LOT-A", "LINE-1", observed_quality(4.9, 28.0, "QUALITY-SYNTH")),
                run("2", "LOT-A", "LINE-1", observed_quality(4.7, 28.0, "QUALITY-SYNTH")),
                run("3", "LOT-B", "LINE-1", observed_quality(5.0, 25.0, "QUALITY-SYNTH")),
                run("4", "LOT-C", "LINE-2", observed_quality(4.9, 27.0, "QUALITY-SYNTH")),
            ],
        }
    }

    #[test]
    fn reports_separate_lot_and_line_summaries() {
        let report = audit_process_window(&valid_manifest()).expect("valid synthetic campaign");
        assert_eq!(report.run_audits.len(), 4);
        assert_eq!(report.lot_summaries.len(), 3);
        assert_eq!(report.line_summaries.len(), 2);
        assert_eq!(report.non_physical_evidence_run_count, 4);
        assert_eq!(report.physical_evidence_run_count, 0);
        assert_eq!(report.lot_summaries[0].first_pass_yield, Some(0.5));
        assert_eq!(report.line_summaries[0].lot_count, 2);
    }

    #[test]
    fn incomplete_and_out_of_window_runs_cannot_inflate_strict_first_pass_yield() {
        let mut manifest = valid_manifest();
        let extra = run(
            "6",
            "LOT-C",
            "LINE-2",
            observed_quality(4.9, 27.0, "QUALITY-SYNTH"),
        );
        manifest.production_runs.push(extra);

        let report = audit_process_window(&manifest).expect("mixed run statuses");
        let lot_c = report
            .lot_summaries
            .iter()
            .find(|item| item.lot_id == "LOT-C")
            .expect("LOT-C summary");

        assert_eq!(lot_c.run_count, 3);
        assert_eq!(lot_c.first_pass_yield, Some(1.0 / 3.0));
        assert_eq!(lot_c.process_window_adherence_rate, Some(2.0 / 3.0));
        assert_eq!(lot_c.in_window_complete_conformance_rate, Some(1.0));
    }

    #[test]
    fn out_of_window_run_is_not_counted_as_in_window_yield() {
        let mut manifest = valid_manifest();
        manifest.production_runs[0].process_parameters[0].value = 30.0;

        let report = audit_process_window(&manifest).expect("run remains auditable");
        assert_eq!(report.out_of_window_run_count, 1);
        assert_eq!(report.lot_summaries[0].out_of_window_run_count, 1);
    }

    #[test]
    fn not_measured_quality_does_not_become_a_pass() {
        let mut manifest = valid_manifest();
        manifest.production_runs[0].quality_observations[1].status =
            MeasurementStatus::NotMeasured;
        manifest.production_runs[0].quality_observations[1].value = None;

        let report = audit_process_window(&manifest).expect("missing observation remains visible");
        assert_eq!(report.incomplete_in_window_run_count, 1);
    }

    #[test]
    fn invalid_measurement_does_not_become_a_pass() {
        let mut manifest = valid_manifest();
        manifest.production_runs[0].quality_observations[1].status =
            MeasurementStatus::InvalidMeasurement;
        manifest.production_runs[0].quality_observations[1].value = None;

        let report = audit_process_window(&manifest).expect("invalid result remains visible");
        assert_eq!(report.incomplete_in_window_run_count, 1);
        assert_eq!(report.lot_summaries[0].incomplete_in_window_count, 1);
    }

    #[test]
    fn candidate_or_research_only_evidence_cannot_be_reported_as_physical_yield() {
        let mut manifest = valid_manifest();
        for record in &mut manifest.evidence {
            if record.evidence_id == "QUALITY-SYNTH" {
                record.admission = EvidenceAdmission::AdmittedForResearch;
            }
        }

        let report = audit_process_window(&manifest).expect("evidence tier remains explicit");
        assert_eq!(report.physical_evidence_run_count, 0);
        assert_eq!(report.unadmitted_evidence_run_count, 4);
        assert!(report
            .lot_summaries
            .iter()
            .all(|summary| summary.evidence_tier == RunEvidenceTier::NotAdmittedForEvaluation));
    }

    #[test]
    fn admitted_physical_evidence_is_required_for_physical_tier() {
        let mut manifest = valid_manifest();
        for run in &mut manifest.production_runs {
            for observation in &mut run.process_parameters {
                observation.evidence_id = "QUALITY-ADMITTED-PHYSICAL".into();
            }
            for observation in &mut run.quality_observations {
                observation.evidence_id = "QUALITY-ADMITTED-PHYSICAL".into();
            }
        }

        let report = audit_process_window(&manifest).expect("declared physical tier");
        assert_eq!(report.physical_evidence_run_count, 4);
        assert_eq!(report.non_physical_evidence_run_count, 0);
    }

    #[test]
    fn rejects_recipe_or_cell_configuration_mismatch() {
        let mut manifest = valid_manifest();
        manifest.production_runs[0].cell_configuration_ref = "CELL-CONFIG-R2".into();

        let error = audit_process_window(&manifest).expect_err("configuration mismatch fails");
        assert!(error.0.contains("recipe or cell-configuration identity mismatch"));
    }

    #[test]
    fn rejects_process_parameter_unit_mismatch() {
        let mut manifest = valid_manifest();
        manifest.production_runs[0].process_parameters[0].unit = "kg/m2".into();

        let error = audit_process_window(&manifest).expect_err("unit mismatch fails");
        assert!(error.0.contains("mismatched unit or invalid value"));
    }

    #[test]
    fn rejects_quality_metric_boundary_mismatch() {
        let mut manifest = valid_manifest();
        manifest.production_runs[0].quality_observations[0].boundary = "active_material_only".into();

        let error = audit_process_window(&manifest).expect_err("boundary mismatch fails");
        assert!(error.0.contains("mismatched unit or physical boundary"));
    }

    #[test]
    fn rejects_missing_evidence_reference() {
        let mut manifest = valid_manifest();
        manifest.production_runs[0].quality_observations[0].evidence_id =
            "UNREGISTERED".into();

        let error = audit_process_window(&manifest).expect_err("unknown evidence fails");
        assert!(error.0.contains("references unknown evidence ID"));
    }

    #[test]
    fn rejects_invalid_process_window_and_quality_specifications() {
        let mut manifest = valid_manifest();
        manifest.process_parameters[0].lower_bound = 25.0;

        let error = audit_process_window(&manifest).expect_err("invalid window fails");
        assert!(error.0.contains("invalid window bounds"));
    }

    #[test]
    fn report_is_deterministic_under_input_permutation() {
        let first = valid_manifest();
        let mut second = valid_manifest();
        second.production_runs.reverse();
        second.evidence.reverse();
        second.process_parameters.reverse();
        second.quality_specs.reverse();

        let first_report = audit_process_window(&first).expect("first report");
        let second_report = audit_process_window(&second).expect("permuted report");
        assert_eq!(first_report, second_report);
    }
}
