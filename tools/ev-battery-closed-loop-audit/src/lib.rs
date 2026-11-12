#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
        matches!(self, Self::PhysicalMeasurement | Self::IndependentLaboratory)
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
    pub source_digest: String,
    pub evidence_class: EvidenceClass,
    pub admission: EvidenceAdmission,
    pub claim_scope: String,
    pub validity_domain: String,
    pub metrology_ref: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DesignCandidate {
    pub design_id: String,
    pub revision: String,
    pub digest: String,
    pub recipe_id: String,
    pub recipe_revision: String,
    pub process_window_id: String,
    pub process_window_revision: String,
    pub evidence_id: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RunDisposition {
    InWindow,
    OutOfWindow,
    Incomplete,
    Invalid,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProcessRun {
    pub run_id: String,
    pub cell_id: String,
    pub lot_id: String,
    pub lot_lineage_digest: String,
    pub line_id: String,
    pub line_identity_digest: String,
    pub equipment_id: String,
    pub design_id: String,
    pub design_revision: String,
    pub recipe_id: String,
    pub recipe_revision: String,
    pub process_window_id: String,
    pub process_window_revision: String,
    pub disposition: RunDisposition,
    pub digest: String,
    pub evidence_id: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CellTestDisposition {
    SyntheticPass,
    SyntheticFail,
    PhysicalPass,
    PhysicalFail,
    Incomplete,
    Invalid,
}

impl CellTestDisposition {
    fn is_physical(self) -> bool {
        matches!(self, Self::PhysicalPass | Self::PhysicalFail)
    }

    fn is_synthetic(self) -> bool {
        matches!(self, Self::SyntheticPass | Self::SyntheticFail)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CellTest {
    pub test_id: String,
    pub cell_id: String,
    pub run_id: String,
    pub design_id: String,
    pub design_revision: String,
    pub boundary: String,
    pub disposition: CellTestDisposition,
    pub digest: String,
    pub evidence_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackMember {
    pub cell_id: String,
    pub design_id: String,
    pub design_revision: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pack {
    pub pack_id: String,
    pub revision: String,
    pub digest: String,
    pub design_id: String,
    pub design_revision: String,
    pub members: Vec<PackMember>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MissionOutcomeKind {
    PhysicalMeasured,
    Simulated,
    Predicted,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MissionOutcome {
    pub outcome_id: String,
    pub pack_id: String,
    pub pack_revision: String,
    pub scenario_id: String,
    pub boundary: String,
    pub kind: MissionOutcomeKind,
    pub digest: String,
    pub evidence_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecisionReceipt {
    pub decision_id: String,
    pub design_id: String,
    pub design_revision: String,
    pub basis_evidence_ids: Vec<String>,
    pub dependency_digests: Vec<String>,
    pub next_experiment_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Manifest {
    pub design: DesignCandidate,
    pub process_runs: Vec<ProcessRun>,
    pub cell_tests: Vec<CellTest>,
    pub pack: Pack,
    pub mission_outcomes: Vec<MissionOutcome>,
    pub decision: DecisionReceipt,
    pub evidence: Vec<EvidenceRecord>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuditReport {
    pub design_id: String,
    pub design_revision: String,
    pub process_run_count: usize,
    pub in_window_run_count: usize,
    pub out_of_window_run_count: usize,
    pub incomplete_or_invalid_run_count: usize,
    pub unique_lot_count: usize,
    pub unique_line_count: usize,
    pub physical_cell_pass_count: usize,
    pub physical_cell_fail_count: usize,
    pub synthetic_cell_outcome_count: usize,
    pub incomplete_or_invalid_cell_test_count: usize,
    pub simulated_mission_count: usize,
    pub physical_mission_count: usize,
    pub physical_chain_closed: bool,
    pub claim_ceiling: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuditError(pub String);

impl std::fmt::Display for AuditError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for AuditError {}

fn require_nonempty(value: &str, label: &str) -> Result<(), AuditError> {
    if value.trim().is_empty() {
        return Err(AuditError(format!("{label} must not be empty")));
    }
    Ok(())
}

fn validate_unique<'a>(
    values: impl IntoIterator<Item = &'a str>,
    label: &str,
) -> Result<(), AuditError> {
    let mut seen = BTreeSet::new();
    for value in values {
        require_nonempty(value, label)?;
        if !seen.insert(value) {
            return Err(AuditError(format!("duplicate {label}: {value}")));
        }
    }
    Ok(())
}

fn validate_digest(value: &str, label: &str) -> Result<(), AuditError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(AuditError(format!(
            "{label} must be a lowercase 64-character SHA-256 digest"
        )));
    }
    Ok(())
}

fn evidence_registry(
    evidence: &[EvidenceRecord],
) -> Result<BTreeMap<&str, &EvidenceRecord>, AuditError> {
    if evidence.is_empty() {
        return Err(AuditError("evidence registry must not be empty".into()));
    }
    validate_unique(
        evidence.iter().map(|record| record.evidence_id.as_str()),
        "evidence ID",
    )?;

    let mut registry = BTreeMap::new();
    for record in evidence {
        validate_digest(&record.source_digest, "evidence source digest")?;
        require_nonempty(&record.claim_scope, "evidence claim scope")?;
        require_nonempty(&record.validity_domain, "evidence validity domain")?;
        if record.evidence_class.is_physical()
            && record
                .metrology_ref
                .as_deref()
                .map_or(true, |value| value.trim().is_empty())
        {
            return Err(AuditError(format!(
                "physical evidence {} requires a metrology reference",
                record.evidence_id
            )));
        }
        registry.insert(record.evidence_id.as_str(), record);
    }
    Ok(registry)
}

fn require_admitted<'a>(
    registry: &BTreeMap<&'a str, &'a EvidenceRecord>,
    evidence_id: &str,
    owner: &str,
) -> Result<&'a EvidenceRecord, AuditError> {
    let record = registry.get(evidence_id).copied().ok_or_else(|| {
        AuditError(format!(
            "{owner} references unknown evidence ID {evidence_id}"
        ))
    })?;
    if record.admission != EvidenceAdmission::AdmittedForEvaluation {
        return Err(AuditError(format!(
            "{owner} references evidence {} that is not admitted for evaluation",
            evidence_id
        )));
    }
    Ok(record)
}

fn validate_lot_and_line_identity(runs: &[ProcessRun]) -> Result<(usize, usize), AuditError> {
    let mut lot_to_digest = BTreeMap::<&str, &str>::new();
    let mut digest_to_lot = BTreeMap::<&str, &str>::new();
    let mut line_to_digest = BTreeMap::<&str, &str>::new();
    let mut digest_to_line = BTreeMap::<&str, &str>::new();

    for run in runs {
        let lot_id = run.lot_id.as_str();
        let lot_digest = run.lot_lineage_digest.as_str();
        let line_id = run.line_id.as_str();
        let line_digest = run.line_identity_digest.as_str();
        match lot_to_digest.insert(lot_id, lot_digest) {
            Some(previous) if previous != lot_digest => {
                return Err(AuditError(format!(
                    "lot {} has conflicting lineage digests",
                    run.lot_id
                )));
            }
            _ => {}
        }
        match digest_to_lot.insert(lot_digest, lot_id) {
            Some(previous) if previous != lot_id => {
                return Err(AuditError(format!(
                    "lot lineage digest is aliased by {} and {}",
                    previous, run.lot_id
                )));
            }
            _ => {}
        }
        match line_to_digest.insert(line_id, line_digest) {
            Some(previous) if previous != line_digest => {
                return Err(AuditError(format!(
                    "line {} has conflicting identity digests",
                    run.line_id
                )));
            }
            _ => {}
        }
        match digest_to_line.insert(line_digest, line_id) {
            Some(previous) if previous != line_id => {
                return Err(AuditError(format!(
                    "line identity digest is aliased by {} and {}",
                    previous, run.line_id
                )));
            }
            _ => {}
        }
    }

    Ok((lot_to_digest.len(), line_to_digest.len()))
}

fn expected_dependency_digests(manifest: &Manifest) -> Vec<String> {
    let mut values = BTreeSet::new();
    values.insert(manifest.design.digest.clone());
    values.insert(manifest.pack.digest.clone());
    for run in &manifest.process_runs {
        values.insert(run.digest.clone());
        values.insert(run.lot_lineage_digest.clone());
        values.insert(run.line_identity_digest.clone());
    }
    for test in &manifest.cell_tests {
        values.insert(test.digest.clone());
    }
    for outcome in &manifest.mission_outcomes {
        values.insert(outcome.digest.clone());
    }
    let evidence_ids: BTreeSet<&str> = std::iter::once(manifest.design.evidence_id.as_str())
        .chain(manifest.process_runs.iter().map(|run| run.evidence_id.as_str()))
        .chain(manifest.cell_tests.iter().map(|test| test.evidence_id.as_str()))
        .chain(
            manifest
                .mission_outcomes
                .iter()
                .map(|outcome| outcome.evidence_id.as_str()),
        )
        .collect();
    for evidence in &manifest.evidence {
        if evidence_ids.contains(evidence.evidence_id.as_str()) {
            values.insert(evidence.source_digest.clone());
        }
    }
    values.into_iter().collect()
}

fn expected_basis_evidence_ids(manifest: &Manifest) -> Vec<String> {
    let mut values = BTreeSet::new();
    values.insert(manifest.design.evidence_id.clone());
    for run in &manifest.process_runs {
        values.insert(run.evidence_id.clone());
    }
    for test in &manifest.cell_tests {
        values.insert(test.evidence_id.clone());
    }
    for outcome in &manifest.mission_outcomes {
        values.insert(outcome.evidence_id.clone());
    }
    values.into_iter().collect()
}

/// Validate exact reference closure and evidence semantics for one design/pack thread.
///
/// This function does not calculate physical performance or statistical process capability.
/// It preserves a strict distinction between synthetic, simulated, predicted and measured claims.
pub fn audit_closed_loop(manifest: &Manifest) -> Result<AuditReport, AuditError> {
    let design = &manifest.design;
    for (value, label) in [
        (design.design_id.as_str(), "design ID"),
        (design.revision.as_str(), "design revision"),
        (design.recipe_id.as_str(), "recipe ID"),
        (design.recipe_revision.as_str(), "recipe revision"),
        (design.process_window_id.as_str(), "process-window ID"),
        (
            design.process_window_revision.as_str(),
            "process-window revision",
        ),
    ] {
        require_nonempty(value, label)?;
    }
    validate_digest(&design.digest, "design digest")?;

    if manifest.process_runs.is_empty() || manifest.cell_tests.is_empty() {
        return Err(AuditError(
            "at least one process run and cell test are required".into(),
        ));
    }
    if manifest.mission_outcomes.is_empty() {
        return Err(AuditError("at least one mission outcome is required".into()));
    }

    let registry = evidence_registry(&manifest.evidence)?;
    let mut used_evidence = BTreeSet::new();
    let design_evidence = require_admitted(&registry, &design.evidence_id, "design")?;
    used_evidence.insert(design_evidence.evidence_id.as_str());

    validate_unique(
        manifest.process_runs.iter().map(|run| run.run_id.as_str()),
        "process-run ID",
    )?;
    validate_unique(
        manifest.process_runs.iter().map(|run| run.cell_id.as_str()),
        "process-run cell ID",
    )?;

    for run in &manifest.process_runs {
        for (value, label) in [
            (run.lot_id.as_str(), "lot ID"),
            (run.line_id.as_str(), "line ID"),
            (run.equipment_id.as_str(), "equipment ID"),
            (run.design_id.as_str(), "run design ID"),
            (run.design_revision.as_str(), "run design revision"),
            (run.recipe_id.as_str(), "run recipe ID"),
            (run.recipe_revision.as_str(), "run recipe revision"),
            (run.process_window_id.as_str(), "run process-window ID"),
            (
                run.process_window_revision.as_str(),
                "run process-window revision",
            ),
        ] {
            require_nonempty(value, label)?;
        }
        validate_digest(&run.lot_lineage_digest, "lot lineage digest")?;
        validate_digest(&run.line_identity_digest, "line identity digest")?;
        validate_digest(&run.digest, "process-run digest")?;

        if run.design_id != design.design_id || run.design_revision != design.revision {
            return Err(AuditError(format!(
                "run {} references a different design revision",
                run.run_id
            )));
        }
        if run.recipe_id != design.recipe_id || run.recipe_revision != design.recipe_revision {
            return Err(AuditError(format!(
                "run {} references a different recipe revision",
                run.run_id
            )));
        }
        if run.process_window_id != design.process_window_id
            || run.process_window_revision != design.process_window_revision
        {
            return Err(AuditError(format!(
                "run {} references a different process-window revision",
                run.run_id
            )));
        }

        let evidence = require_admitted(&registry, &run.evidence_id, &format!("run {}", run.run_id))?;
        used_evidence.insert(evidence.evidence_id.as_str());
    }

    let (unique_lot_count, unique_line_count) =
        validate_lot_and_line_identity(&manifest.process_runs)?;

    validate_unique(
        manifest.cell_tests.iter().map(|test| test.test_id.as_str()),
        "cell-test ID",
    )?;
    validate_unique(
        manifest.cell_tests.iter().map(|test| test.cell_id.as_str()),
        "cell-test cell ID",
    )?;

    let runs_by_cell: BTreeMap<&str, &ProcessRun> = manifest
        .process_runs
        .iter()
        .map(|run| (run.cell_id.as_str(), run))
        .collect();

    for test in &manifest.cell_tests {
        for (value, label) in [
            (test.run_id.as_str(), "cell-test run ID"),
            (test.design_id.as_str(), "cell-test design ID"),
            (test.design_revision.as_str(), "cell-test design revision"),
            (test.boundary.as_str(), "cell-test boundary"),
        ] {
            require_nonempty(value, label)?;
        }
        validate_digest(&test.digest, "cell-test digest")?;
        let run = runs_by_cell.get(test.cell_id.as_str()).copied().ok_or_else(|| {
            AuditError(format!("cell test {} has no matching process run", test.test_id))
        })?;
        if test.run_id != run.run_id {
            return Err(AuditError(format!(
                "cell test {} references a different production run",
                test.test_id
            )));
        }
        if test.design_id != design.design_id || test.design_revision != design.revision {
            return Err(AuditError(format!(
                "cell test {} references a different design revision",
                test.test_id
            )));
        }
        let evidence = require_admitted(&registry, &test.evidence_id, &format!("cell test {}", test.test_id))?;
        if test.disposition.is_physical() && !evidence.evidence_class.is_physical() {
            return Err(AuditError(format!(
                "physical cell test {} requires physical measurement evidence",
                test.test_id
            )));
        }
        if test.disposition.is_synthetic()
            && evidence.evidence_class != EvidenceClass::SyntheticFixture
        {
            return Err(AuditError(format!(
                "synthetic cell test {} requires synthetic-fixture evidence",
                test.test_id
            )));
        }
        used_evidence.insert(evidence.evidence_id.as_str());
    }

    let test_cells: BTreeSet<&str> = manifest
        .cell_tests
        .iter()
        .map(|test| test.cell_id.as_str())
        .collect();
    let run_cells: BTreeSet<&str> = manifest
        .process_runs
        .iter()
        .map(|run| run.cell_id.as_str())
        .collect();
    if test_cells != run_cells {
        return Err(AuditError(
            "process-run cells and tested cells do not close exactly".into(),
        ));
    }

    let pack = &manifest.pack;
    for (value, label) in [
        (pack.pack_id.as_str(), "pack ID"),
        (pack.revision.as_str(), "pack revision"),
        (pack.design_id.as_str(), "pack design ID"),
        (pack.design_revision.as_str(), "pack design revision"),
    ] {
        require_nonempty(value, label)?;
    }
    validate_digest(&pack.digest, "pack digest")?;
    if pack.design_id != design.design_id || pack.design_revision != design.revision {
        return Err(AuditError("pack references a different design revision".into()));
    }
    if pack.members.is_empty() {
        return Err(AuditError("pack membership must not be empty".into()));
    }
    validate_unique(
        pack.members.iter().map(|member| member.cell_id.as_str()),
        "pack member cell ID",
    )?;
    for member in &pack.members {
        if member.design_id != design.design_id || member.design_revision != design.revision {
            return Err(AuditError(format!(
                "pack member {} references a different design revision",
                member.cell_id
            )));
        }
        if !test_cells.contains(member.cell_id.as_str()) {
            return Err(AuditError(format!(
                "pack member {} has no admitted cell test",
                member.cell_id
            )));
        }
    }
    let pack_cells: BTreeSet<&str> = pack
        .members
        .iter()
        .map(|member| member.cell_id.as_str())
        .collect();
    if pack_cells != run_cells {
        return Err(AuditError(
            "pack membership and produced cells do not close exactly".into(),
        ));
    }

    validate_unique(
        manifest
            .mission_outcomes
            .iter()
            .map(|outcome| outcome.outcome_id.as_str()),
        "mission-outcome ID",
    )?;
    for outcome in &manifest.mission_outcomes {
        for (value, label) in [
            (outcome.pack_id.as_str(), "mission pack ID"),
            (outcome.pack_revision.as_str(), "mission pack revision"),
            (outcome.scenario_id.as_str(), "mission scenario ID"),
            (outcome.boundary.as_str(), "mission boundary"),
        ] {
            require_nonempty(value, label)?;
        }
        validate_digest(&outcome.digest, "mission-outcome digest")?;
        if outcome.pack_id != pack.pack_id || outcome.pack_revision != pack.revision {
            return Err(AuditError(format!(
                "mission outcome {} references a different pack revision",
                outcome.outcome_id
            )));
        }
        let evidence = require_admitted(
            &registry,
            &outcome.evidence_id,
            &format!("mission outcome {}", outcome.outcome_id),
        )?;
        let class_ok = match outcome.kind {
            MissionOutcomeKind::PhysicalMeasured => evidence.evidence_class.is_physical(),
            MissionOutcomeKind::Simulated => matches!(
                evidence.evidence_class,
                EvidenceClass::Simulation | EvidenceClass::Calculation
            ),
            MissionOutcomeKind::Predicted => matches!(
                evidence.evidence_class,
                EvidenceClass::Surrogate
                    | EvidenceClass::LiteratureReported
                    | EvidenceClass::VendorClaim
            ),
            MissionOutcomeKind::Unknown => true,
        };
        if !class_ok {
            return Err(AuditError(format!(
                "mission outcome {} evidence class does not match its declared kind",
                outcome.outcome_id
            )));
        }
        used_evidence.insert(evidence.evidence_id.as_str());
    }

    let decision = &manifest.decision;
    require_nonempty(&decision.decision_id, "decision ID")?;
    if decision.design_id != design.design_id || decision.design_revision != design.revision {
        return Err(AuditError(
            "decision receipt references a different design revision".into(),
        ));
    }
    validate_unique(
        decision.basis_evidence_ids.iter().map(String::as_str),
        "decision basis evidence ID",
    )?;
    validate_unique(
        decision.dependency_digests.iter().map(String::as_str),
        "decision dependency digest",
    )?;
    for digest in &decision.dependency_digests {
        validate_digest(digest, "decision dependency digest")?;
    }

    let mut expected_basis = expected_basis_evidence_ids(manifest);
    let mut actual_basis = decision.basis_evidence_ids.clone();
    expected_basis.sort();
    actual_basis.sort();
    if expected_basis != actual_basis {
        return Err(AuditError(
            "decision evidence basis is stale or incomplete".into(),
        ));
    }

    let mut expected_digests = expected_dependency_digests(manifest);
    let mut actual_digests = decision.dependency_digests.clone();
    expected_digests.sort();
    actual_digests.sort();
    if expected_digests != actual_digests {
        return Err(AuditError(
            "decision dependency digests are stale or incomplete".into(),
        ));
    }

    let in_window_run_count = manifest
        .process_runs
        .iter()
        .filter(|run| run.disposition == RunDisposition::InWindow)
        .count();
    let out_of_window_run_count = manifest
        .process_runs
        .iter()
        .filter(|run| run.disposition == RunDisposition::OutOfWindow)
        .count();
    let incomplete_or_invalid_run_count = manifest
        .process_runs
        .iter()
        .filter(|run| matches!(run.disposition, RunDisposition::Incomplete | RunDisposition::Invalid))
        .count();
    let physical_cell_pass_count = manifest
        .cell_tests
        .iter()
        .filter(|test| test.disposition == CellTestDisposition::PhysicalPass)
        .count();
    let physical_cell_fail_count = manifest
        .cell_tests
        .iter()
        .filter(|test| test.disposition == CellTestDisposition::PhysicalFail)
        .count();
    let synthetic_cell_outcome_count = manifest
        .cell_tests
        .iter()
        .filter(|test| test.disposition.is_synthetic())
        .count();
    let incomplete_or_invalid_cell_test_count = manifest
        .cell_tests
        .iter()
        .filter(|test| {
            matches!(
                test.disposition,
                CellTestDisposition::Incomplete | CellTestDisposition::Invalid
            )
        })
        .count();
    let simulated_mission_count = manifest
        .mission_outcomes
        .iter()
        .filter(|outcome| outcome.kind == MissionOutcomeKind::Simulated)
        .count();
    let physical_mission_count = manifest
        .mission_outcomes
        .iter()
        .filter(|outcome| outcome.kind == MissionOutcomeKind::PhysicalMeasured)
        .count();

    let all_runs_physical = manifest.process_runs.iter().all(|run| {
        registry
            .get(run.evidence_id.as_str())
            .is_some_and(|record| record.evidence_class.is_physical())
    });
    let all_tests_physical = manifest
        .cell_tests
        .iter()
        .all(|test| test.disposition.is_physical());
    let all_missions_physical = manifest
        .mission_outcomes
        .iter()
        .all(|outcome| outcome.kind == MissionOutcomeKind::PhysicalMeasured);
    let physical_chain_closed = all_runs_physical && all_tests_physical && all_missions_physical;

    let claim_ceiling = if physical_chain_closed {
        "Physical evidence references close for this exact design, pack and declared missions; this is not an independent reproduction, safety certification or product release.".into()
    } else {
        "Synthetic, simulated, predicted or incomplete evidence remains in the chain; no complete physical design-to-mission claim is established.".into()
    };

    let _ = used_evidence;
    Ok(AuditReport {
        design_id: design.design_id.clone(),
        design_revision: design.revision.clone(),
        process_run_count: manifest.process_runs.len(),
        in_window_run_count,
        out_of_window_run_count,
        incomplete_or_invalid_run_count,
        unique_lot_count,
        unique_line_count,
        physical_cell_pass_count,
        physical_cell_fail_count,
        synthetic_cell_outcome_count,
        incomplete_or_invalid_cell_test_count,
        simulated_mission_count,
        physical_mission_count,
        physical_chain_closed,
        claim_ceiling,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(character: char) -> String {
        std::iter::repeat_n(character, 64).collect()
    }

    fn evidence(
        evidence_id: &str,
        character: char,
        evidence_class: EvidenceClass,
        metrology_ref: Option<&str>,
    ) -> EvidenceRecord {
        EvidenceRecord {
            evidence_id: evidence_id.into(),
            source_digest: digest(character),
            evidence_class,
            admission: EvidenceAdmission::AdmittedForEvaluation,
            claim_scope: format!("synthetic test scope for {evidence_id}"),
            validity_domain: "unit test only".into(),
            metrology_ref: metrology_ref.map(str::to_string),
        }
    }

    fn fixture() -> Manifest {
        let design = DesignCandidate {
            design_id: "LFP-4AH-CELL".into(),
            revision: "r1".into(),
            digest: digest('a'),
            recipe_id: "LFP-COATING-CALENDER-R1".into(),
            recipe_revision: "r1".into(),
            process_window_id: "LFP-PROCESS-WINDOW-01".into(),
            process_window_revision: "r1".into(),
            evidence_id: "E-DESIGN".into(),
        };
        let lot_a = digest('1');
        let lot_b = digest('5');
        let lot_c = digest('8');
        let line_1 = digest('2');
        let line_2 = digest('6');
        let mut process_runs = Vec::new();
        for (index, cell, lot, lot_digest, line, line_digest, disposition, run_char) in [
            ("1", "CELL-001", "LOT-A", lot_a.as_str(), "LINE-1", line_1.as_str(), RunDisposition::InWindow, '3'),
            ("2", "CELL-002", "LOT-A", lot_a.as_str(), "LINE-1", line_1.as_str(), RunDisposition::InWindow, '4'),
            ("3", "CELL-003", "LOT-B", lot_b.as_str(), "LINE-2", line_2.as_str(), RunDisposition::OutOfWindow, '7'),
            ("4", "CELL-004", "LOT-C", lot_c.as_str(), "LINE-2", line_2.as_str(), RunDisposition::InWindow, '9'),
        ] {
            process_runs.push(ProcessRun {
                run_id: format!("RUN-{index}"),
                cell_id: cell.into(),
                lot_id: lot.into(),
                lot_lineage_digest: lot_digest.to_string(),
                line_id: line.into(),
                line_identity_digest: line_digest.to_string(),
                equipment_id: format!("CALENDER-{}", if line == "LINE-1" { "1" } else { "2" }),
                design_id: design.design_id.clone(),
                design_revision: design.revision.clone(),
                recipe_id: design.recipe_id.clone(),
                recipe_revision: design.recipe_revision.clone(),
                process_window_id: design.process_window_id.clone(),
                process_window_revision: design.process_window_revision.clone(),
                disposition,
                digest: digest(run_char),
                evidence_id: "E-PROCESS".into(),
            });
        }
        let cell_tests = [
            ("1", "CELL-001", CellTestDisposition::SyntheticPass, 'a'),
            ("2", "CELL-002", CellTestDisposition::SyntheticPass, 'b'),
            ("3", "CELL-003", CellTestDisposition::SyntheticFail, 'c'),
            ("4", "CELL-004", CellTestDisposition::Incomplete, 'd'),
        ]
        .into_iter()
        .map(|(index, cell_id, disposition, digest_char)| CellTest {
            test_id: format!("TEST-{index}"),
            cell_id: cell_id.into(),
            run_id: format!("RUN-{index}"),
            design_id: design.design_id.clone(),
            design_revision: design.revision.clone(),
            boundary: "complete_cell".into(),
            disposition,
            digest: digest(digest_char),
            evidence_id: "E-CELL".into(),
        })
        .collect::<Vec<_>>();
        let pack = Pack {
            pack_id: "PACK-001".into(),
            revision: "r1".into(),
            digest: digest('e'),
            design_id: design.design_id.clone(),
            design_revision: design.revision.clone(),
            members: ["CELL-001", "CELL-002", "CELL-003", "CELL-004"]
                .into_iter()
                .map(|cell_id| PackMember {
                    cell_id: cell_id.into(),
                    design_id: design.design_id.clone(),
                    design_revision: design.revision.clone(),
                })
                .collect(),
        };
        let mission_outcomes = vec![MissionOutcome {
            outcome_id: "MISSION-001".into(),
            pack_id: pack.pack_id.clone(),
            pack_revision: pack.revision.clone(),
            scenario_id: "EV-DELIVERY-DUTY-01".into(),
            boundary: "installed_pack".into(),
            kind: MissionOutcomeKind::Simulated,
            digest: digest('f'),
            evidence_id: "E-MISSION".into(),
        }];
        let evidence = vec![
            evidence("E-DESIGN", 'c', EvidenceClass::SyntheticFixture, None),
            evidence("E-PROCESS", 'd', EvidenceClass::SyntheticFixture, None),
            evidence("E-CELL", 'e', EvidenceClass::SyntheticFixture, None),
            evidence("E-MISSION", 'f', EvidenceClass::Simulation, None),
        ];
        let mut manifest = Manifest {
            design,
            process_runs,
            cell_tests,
            pack,
            mission_outcomes,
            decision: DecisionReceipt {
                decision_id: "DECISION-001".into(),
                design_id: "LFP-4AH-CELL".into(),
                design_revision: "r1".into(),
                basis_evidence_ids: Vec::new(),
                dependency_digests: Vec::new(),
                next_experiment_id: Some("EXP-AGING-002".into()),
            },
            evidence,
        };
        manifest.decision.basis_evidence_ids = expected_basis_evidence_ids(&manifest);
        manifest.decision.dependency_digests = expected_dependency_digests(&manifest);
        manifest
    }

    #[test]
    fn synthetic_fixture_closes_references_but_not_physical_claims() {
        let report = audit_closed_loop(&fixture()).expect("synthetic graph should close");
        assert_eq!(report.process_run_count, 4);
        assert_eq!(report.in_window_run_count, 3);
        assert_eq!(report.out_of_window_run_count, 1);
        assert_eq!(report.incomplete_or_invalid_run_count, 0);
        assert_eq!(report.unique_lot_count, 3);
        assert_eq!(report.unique_line_count, 2);
        assert_eq!(report.synthetic_cell_outcome_count, 3);
        assert_eq!(report.incomplete_or_invalid_cell_test_count, 1);
        assert_eq!(report.simulated_mission_count, 1);
        assert!(!report.physical_chain_closed);
    }

    #[test]
    fn wrong_design_revision_in_run_is_rejected() {
        let mut value = fixture();
        value.process_runs[0].design_revision = "r2".into();
        assert!(audit_closed_loop(&value).is_err());
    }

    #[test]
    fn wrong_recipe_revision_in_run_is_rejected() {
        let mut value = fixture();
        value.process_runs[0].recipe_revision = "r2".into();
        assert!(audit_closed_loop(&value).is_err());
    }

    #[test]
    fn wrong_process_window_revision_is_rejected() {
        let mut value = fixture();
        value.process_runs[0].process_window_revision = "r2".into();
        assert!(audit_closed_loop(&value).is_err());
    }

    #[test]
    fn a_cell_without_a_matching_test_is_rejected() {
        let mut value = fixture();
        value.cell_tests.pop();
        value.decision.basis_evidence_ids = expected_basis_evidence_ids(&value);
        value.decision.dependency_digests = expected_dependency_digests(&value);
        assert!(audit_closed_loop(&value).is_err());
    }

    #[test]
    fn pack_member_without_cell_test_is_rejected() {
        let mut value = fixture();
        value.pack.members.pop();
        value.decision.dependency_digests = expected_dependency_digests(&value);
        assert!(audit_closed_loop(&value).is_err());
    }

    #[test]
    fn mission_must_reference_exact_pack_revision() {
        let mut value = fixture();
        value.mission_outcomes[0].pack_revision = "r2".into();
        value.decision.dependency_digests = expected_dependency_digests(&value);
        assert!(audit_closed_loop(&value).is_err());
    }

    #[test]
    fn synthetic_evidence_cannot_be_declared_as_physical_cell_test() {
        let mut value = fixture();
        value.cell_tests[0].disposition = CellTestDisposition::PhysicalPass;
        assert!(audit_closed_loop(&value).is_err());
    }

    #[test]
    fn physical_evidence_requires_a_metrology_reference() {
        let mut value = fixture();
        value.evidence[1].evidence_class = EvidenceClass::PhysicalMeasurement;
        assert!(audit_closed_loop(&value).is_err());
    }

    #[test]
    fn a_second_lot_id_cannot_alias_the_same_lot_lineage() {
        let mut value = fixture();
        value.process_runs[2].lot_lineage_digest = value.process_runs[0].lot_lineage_digest.clone();
        assert!(audit_closed_loop(&value).is_err());
    }

    #[test]
    fn a_second_line_id_cannot_alias_the_same_line_identity() {
        let mut value = fixture();
        value.process_runs[2].line_identity_digest = value.process_runs[0].line_identity_digest.clone();
        assert!(audit_closed_loop(&value).is_err());
    }

    #[test]
    fn stale_decision_dependency_digest_is_rejected() {
        let mut value = fixture();
        value.process_runs[0].digest = digest('0');
        assert!(audit_closed_loop(&value).is_err());
    }

    #[test]
    fn changed_lot_lineage_invalidates_decision_receipt() {
        let mut value = fixture();
        value.process_runs[0].lot_lineage_digest = digest('0');
        value.process_runs[1].lot_lineage_digest = digest('0');
        assert!(audit_closed_loop(&value).is_err());
    }

    #[test]
    fn incomplete_and_invalid_process_runs_are_counted() {
        let mut value = fixture();
        value.process_runs[0].disposition = RunDisposition::Incomplete;
        value.process_runs[1].disposition = RunDisposition::Invalid;
        value.decision.dependency_digests = expected_dependency_digests(&value);
        let report = audit_closed_loop(&value).expect("run status remains auditable");
        assert_eq!(report.incomplete_or_invalid_run_count, 2);
        assert_eq!(report.process_run_count, 4);
    }

    #[test]
    fn incomplete_decision_evidence_basis_is_rejected() {
        let mut value = fixture();
        value.decision.basis_evidence_ids.pop();
        assert!(audit_closed_loop(&value).is_err());
    }

    #[test]
    fn evidence_not_admitted_for_evaluation_is_rejected() {
        let mut value = fixture();
        value.evidence[1].admission = EvidenceAdmission::AdmittedForResearch;
        assert!(audit_closed_loop(&value).is_err());
    }

    #[test]
    fn malformed_digest_is_rejected() {
        let mut value = fixture();
        value.process_runs[0].digest = "not-a-digest".into();
        assert!(audit_closed_loop(&value).is_err());
    }

    #[test]
    fn physical_chain_closes_only_with_physical_run_test_and_mission_evidence() {
        let mut value = fixture();
        for record in &mut value.evidence {
            if ["E-PROCESS", "E-CELL", "E-MISSION"].contains(&record.evidence_id.as_str()) {
                record.evidence_class = EvidenceClass::PhysicalMeasurement;
                record.metrology_ref = Some(format!("METROLOGY-{}", record.evidence_id));
            }
        }
        for test in &mut value.cell_tests {
            test.disposition = CellTestDisposition::PhysicalPass;
        }
        value.mission_outcomes[0].kind = MissionOutcomeKind::PhysicalMeasured;
        value.decision.basis_evidence_ids = expected_basis_evidence_ids(&value);
        value.decision.dependency_digests = expected_dependency_digests(&value);
        let report = audit_closed_loop(&value).expect("physical evidence chain should close");
        assert!(report.physical_chain_closed);
        assert_eq!(report.physical_cell_pass_count, 4);
        assert_eq!(report.physical_mission_count, 1);
    }

    #[test]
    fn input_permutation_does_not_change_the_report() {
        let value = fixture();
        let expected = audit_closed_loop(&value).expect("fixture should close");
        let mut permuted = value;
        permuted.process_runs.reverse();
        permuted.cell_tests.reverse();
        permuted.mission_outcomes.reverse();
        permuted.evidence.reverse();
        permuted.pack.members.reverse();
        permuted.decision.basis_evidence_ids.reverse();
        permuted.decision.dependency_digests.reverse();
        let actual = audit_closed_loop(&permuted).expect("permuted fixture should close");
        assert_eq!(expected, actual);
    }
}
