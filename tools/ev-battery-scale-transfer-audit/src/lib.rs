#![forbid(unsafe_code)]

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
    pub source_digest: String,
    pub evidence_class: EvidenceClass,
    pub admission: EvidenceAdmission,
    pub claim_scope: String,
    pub validity_domain: String,
    pub metrology_ref: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ScaleStageKind {
    Material,
    Electrode,
    CoinCell,
    SmallPouch,
    LargeFormatCell,
    Module,
    Pack,
}

impl ScaleStageKind {
    fn rank(self) -> u8 {
        match self {
            Self::Material => 0,
            Self::Electrode => 1,
            Self::CoinCell => 2,
            Self::SmallPouch => 3,
            Self::LargeFormatCell => 4,
            Self::Module => 5,
            Self::Pack => 6,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ScaleStage {
    pub stage_id: String,
    pub stage_kind: ScaleStageKind,
    /// Stable candidate lineage; stage revisions may legitimately change with scale.
    pub design_id: String,
    pub design_revision: String,
    pub artifact_config_ref: String,
    pub parent_stage_ids: Vec<String>,
    /// A positive quantity with a unit local to this stage. It is not compared across dimensions.
    pub scale_value: f64,
    pub scale_unit: String,
    pub stage_digest: String,
    pub evidence_id: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum MappingKind {
    Invariant,
    Rescaled,
    Reoptimized,
    Unknown,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScaleVariableMapping {
    pub variable_id: String,
    pub mapping_kind: MappingKind,
    pub rationale: String,
    pub evidence_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScaleBridge {
    pub bridge_id: String,
    pub source_stage_id: String,
    pub target_stage_id: String,
    pub mapping_profile_ref: String,
    pub variables: Vec<ScaleVariableMapping>,
    pub evidence_id: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ComparisonDisposition {
    PredictedOnly,
    PhysicalMeasurement,
    IndependentLaboratory,
    Unknown,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TransferComparison {
    pub comparison_id: String,
    pub bridge_id: String,
    pub quantity_id: String,
    pub unit: String,
    pub boundary: String,
    pub predicted_value: f64,
    pub prediction_lower_bound: f64,
    pub prediction_upper_bound: f64,
    pub observed_value: Option<f64>,
    pub prediction_evidence_id: String,
    pub observation_evidence_id: Option<String>,
    pub disposition: ComparisonDisposition,
    pub comparison_digest: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScaleTransferManifest {
    pub manifest_id: String,
    pub stages: Vec<ScaleStage>,
    pub bridges: Vec<ScaleBridge>,
    pub comparisons: Vec<TransferComparison>,
    pub evidence: Vec<EvidenceRecord>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScaleTransferAuditReport {
    pub manifest_id: String,
    pub stage_count: usize,
    pub bridge_count: usize,
    pub comparison_count: usize,
    pub invariant_mapping_count: usize,
    pub rescaled_mapping_count: usize,
    pub reoptimized_mapping_count: usize,
    pub unknown_mapping_count: usize,
    pub predicted_only_count: usize,
    pub physical_measurement_comparison_count: usize,
    pub independent_laboratory_comparison_count: usize,
    pub unresolved_comparison_count: usize,
    pub claim_ceiling: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScaleTransferAuditError(pub String);

impl std::fmt::Display for ScaleTransferAuditError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for ScaleTransferAuditError {}

fn require_nonempty(value: &str, label: &str) -> Result<(), ScaleTransferAuditError> {
    if value.trim().is_empty() {
        return Err(ScaleTransferAuditError(format!(
            "{label} must not be empty"
        )));
    }
    Ok(())
}

fn validate_unique<'a>(
    values: impl IntoIterator<Item = &'a str>,
    label: &str,
) -> Result<(), ScaleTransferAuditError> {
    let mut seen = BTreeSet::new();
    for value in values {
        require_nonempty(value, label)?;
        if !seen.insert(value) {
            return Err(ScaleTransferAuditError(format!(
                "duplicate {label}: {value}"
            )));
        }
    }
    Ok(())
}

fn validate_digest(value: &str, label: &str) -> Result<(), ScaleTransferAuditError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err(ScaleTransferAuditError(format!(
            "{label} must be a lowercase 64-character SHA-256 digest"
        )));
    }
    Ok(())
}

fn evidence_registry(
    evidence: &[EvidenceRecord],
) -> Result<BTreeMap<&str, &EvidenceRecord>, ScaleTransferAuditError> {
    if evidence.is_empty() {
        return Err(ScaleTransferAuditError(
            "evidence registry must not be empty".into(),
        ));
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
            return Err(ScaleTransferAuditError(format!(
                "physical evidence {} requires a metrology reference",
                record.evidence_id
            )));
        }
        registry.insert(record.evidence_id.as_str(), record);
    }
    Ok(registry)
}

fn require_evidence<'a>(
    registry: &BTreeMap<&'a str, &'a EvidenceRecord>,
    evidence_id: &str,
    owner: &str,
) -> Result<&'a EvidenceRecord, ScaleTransferAuditError> {
    registry.get(evidence_id).copied().ok_or_else(|| {
        ScaleTransferAuditError(format!(
            "{owner} references unknown evidence ID {evidence_id}"
        ))
    })
}

fn require_evaluation_evidence<'a>(
    registry: &BTreeMap<&'a str, &'a EvidenceRecord>,
    evidence_id: &str,
    owner: &str,
) -> Result<&'a EvidenceRecord, ScaleTransferAuditError> {
    let record = require_evidence(registry, evidence_id, owner)?;
    if record.admission != EvidenceAdmission::AdmittedForEvaluation {
        return Err(ScaleTransferAuditError(format!(
            "{owner} references evidence {evidence_id} not admitted for evaluation"
        )));
    }
    Ok(record)
}

fn validate_stages<'a>(
    stages: &'a [ScaleStage],
    registry: &BTreeMap<&str, &EvidenceRecord>,
) -> Result<BTreeMap<&'a str, &'a ScaleStage>, ScaleTransferAuditError> {
    if stages.is_empty() {
        return Err(ScaleTransferAuditError(
            "at least one scale stage is required".into(),
        ));
    }
    validate_unique(
        stages.iter().map(|stage| stage.stage_id.as_str()),
        "scale-stage ID",
    )?;

    let by_id: BTreeMap<&str, &ScaleStage> = stages
        .iter()
        .map(|stage| (stage.stage_id.as_str(), stage))
        .collect();

    for stage in stages {
        for (value, label) in [
            (stage.design_id.as_str(), "stage design ID"),
            (stage.design_revision.as_str(), "stage design revision"),
            (stage.artifact_config_ref.as_str(), "stage artifact configuration"),
            (stage.scale_unit.as_str(), "stage scale unit"),
        ] {
            require_nonempty(value, label)?;
        }
        if !stage.scale_value.is_finite() || stage.scale_value <= 0.0 {
            return Err(ScaleTransferAuditError(format!(
                "stage {} has an invalid scale value",
                stage.stage_id
            )));
        }
        validate_digest(&stage.stage_digest, "scale-stage digest")?;
        require_evidence(registry, &stage.evidence_id, &format!("stage {}", stage.stage_id))?;

        if stage.stage_kind.rank() == 0 && !stage.parent_stage_ids.is_empty() {
            return Err(ScaleTransferAuditError(format!(
                "root stage {} must not declare parent stages",
                stage.stage_id
            )));
        }
        if stage.stage_kind.rank() > 0 && stage.parent_stage_ids.is_empty() {
            return Err(ScaleTransferAuditError(format!(
                "non-root stage {} requires at least one parent stage",
                stage.stage_id
            )));
        }
        validate_unique(
            stage.parent_stage_ids.iter().map(String::as_str),
            "parent stage ID",
        )?;
        for parent_id in &stage.parent_stage_ids {
            let parent = by_id.get(parent_id.as_str()).ok_or_else(|| {
                ScaleTransferAuditError(format!(
                    "stage {} references unknown parent stage {}",
                    stage.stage_id, parent_id
                ))
            })?;
            if parent.stage_kind.rank() >= stage.stage_kind.rank() {
                return Err(ScaleTransferAuditError(format!(
                    "stage {} does not advance beyond parent stage {}",
                    stage.stage_id, parent_id
                )));
            }
            if parent.design_id != stage.design_id {
                return Err(ScaleTransferAuditError(format!(
                    "stage {} has a parent from a different candidate lineage",
                    stage.stage_id
                )));
            }
        }
    }

    Ok(by_id)
}

fn validate_bridges<'a>(
    bridges: &'a [ScaleBridge],
    stages: &BTreeMap<&str, &ScaleStage>,
    registry: &BTreeMap<&str, &EvidenceRecord>,
) -> Result<BTreeMap<&'a str, &'a ScaleBridge>, ScaleTransferAuditError> {
    validate_unique(
        bridges.iter().map(|bridge| bridge.bridge_id.as_str()),
        "scale-bridge ID",
    )?;

    let mut by_id = BTreeMap::new();
    for bridge in bridges {
        require_nonempty(&bridge.mapping_profile_ref, "scale mapping profile reference")?;
        let source = stages.get(bridge.source_stage_id.as_str()).ok_or_else(|| {
            ScaleTransferAuditError(format!(
                "bridge {} references unknown source stage {}",
                bridge.bridge_id, bridge.source_stage_id
            ))
        })?;
        let target = stages.get(bridge.target_stage_id.as_str()).ok_or_else(|| {
            ScaleTransferAuditError(format!(
                "bridge {} references unknown target stage {}",
                bridge.bridge_id, bridge.target_stage_id
            ))
        })?;
        if source.stage_id == target.stage_id
            || source.stage_kind.rank() >= target.stage_kind.rank()
        {
            return Err(ScaleTransferAuditError(format!(
                "bridge {} does not advance to a higher scale stage",
                bridge.bridge_id
            )));
        }
        if !target
            .parent_stage_ids
            .iter()
            .any(|parent_id| parent_id == &source.stage_id)
        {
            return Err(ScaleTransferAuditError(format!(
                "bridge {} must join a declared parent stage directly to its target",
                bridge.bridge_id
            )));
        }
        if bridge.variables.is_empty() {
            return Err(ScaleTransferAuditError(format!(
                "bridge {} requires explicit scale-variable mappings",
                bridge.bridge_id
            )));
        }
        validate_unique(
            bridge
                .variables
                .iter()
                .map(|variable| variable.variable_id.as_str()),
            "scale-variable ID",
        )?;
        require_evidence(
            registry,
            &bridge.evidence_id,
            &format!("bridge {}", bridge.bridge_id),
        )?;
        for variable in &bridge.variables {
            require_nonempty(&variable.rationale, "scale-variable mapping rationale")?;
            require_evidence(
                registry,
                &variable.evidence_id,
                &format!("bridge {} variable {}", bridge.bridge_id, variable.variable_id),
            )?;
        }
        by_id.insert(bridge.bridge_id.as_str(), bridge);
    }
    Ok(by_id)
}

fn validate_comparisons(
    comparisons: &[TransferComparison],
    bridges: &BTreeMap<&str, &ScaleBridge>,
    registry: &BTreeMap<&str, &EvidenceRecord>,
) -> Result<(), ScaleTransferAuditError> {
    validate_unique(
        comparisons
            .iter()
            .map(|comparison| comparison.comparison_id.as_str()),
        "transfer-comparison ID",
    )?;

    for comparison in comparisons {
        for (value, label) in [
            (comparison.quantity_id.as_str(), "comparison quantity ID"),
            (comparison.unit.as_str(), "comparison unit"),
            (comparison.boundary.as_str(), "comparison boundary"),
        ] {
            require_nonempty(value, label)?;
        }
        validate_digest(&comparison.comparison_digest, "comparison digest")?;
        if !comparison.predicted_value.is_finite()
            || !comparison.prediction_lower_bound.is_finite()
            || !comparison.prediction_upper_bound.is_finite()
            || comparison.prediction_lower_bound > comparison.predicted_value
            || comparison.predicted_value > comparison.prediction_upper_bound
        {
            return Err(ScaleTransferAuditError(format!(
                "comparison {} has invalid prediction bounds",
                comparison.comparison_id
            )));
        }
        if comparison
            .observed_value
            .is_some_and(|value| !value.is_finite())
        {
            return Err(ScaleTransferAuditError(format!(
                "comparison {} has a non-finite observed value",
                comparison.comparison_id
            )));
        }
        if !bridges.contains_key(comparison.bridge_id.as_str()) {
            return Err(ScaleTransferAuditError(format!(
                "comparison {} references unknown bridge {}",
                comparison.comparison_id, comparison.bridge_id
            )));
        }

        let prediction_evidence = require_evaluation_evidence(
            registry,
            &comparison.prediction_evidence_id,
            &format!("comparison {} prediction", comparison.comparison_id),
        )?;
        if comparison.disposition == ComparisonDisposition::PhysicalMeasurement
            || comparison.disposition == ComparisonDisposition::IndependentLaboratory
        {
            let observation_id = comparison.observation_evidence_id.as_deref().ok_or_else(|| {
                ScaleTransferAuditError(format!(
                    "physical comparison {} requires observation evidence",
                    comparison.comparison_id
                ))
            })?;
            let observed = comparison.observed_value.ok_or_else(|| {
                ScaleTransferAuditError(format!(
                    "physical comparison {} requires an observed value",
                    comparison.comparison_id
                ))
            })?;
            let _ = observed;
            let observation = require_evaluation_evidence(
                registry,
                observation_id,
                &format!("comparison {} observation", comparison.comparison_id),
            )?;
            if comparison.disposition == ComparisonDisposition::PhysicalMeasurement
                && observation.evidence_class != EvidenceClass::PhysicalMeasurement
            {
                return Err(ScaleTransferAuditError(format!(
                    "comparison {} requires physical-measurement evidence",
                    comparison.comparison_id
                )));
            }
            if comparison.disposition == ComparisonDisposition::IndependentLaboratory
                && observation.evidence_class != EvidenceClass::IndependentLaboratory
            {
                return Err(ScaleTransferAuditError(format!(
                    "comparison {} requires independent-laboratory evidence",
                    comparison.comparison_id
                )));
            }
            if prediction_evidence.evidence_class.is_physical() {
                return Err(ScaleTransferAuditError(format!(
                    "comparison {} prediction must not reuse physical observation evidence as its prediction source",
                    comparison.comparison_id
                )));
            }
        } else if comparison.disposition == ComparisonDisposition::PredictedOnly {
            if comparison.observed_value.is_some() || comparison.observation_evidence_id.is_some() {
                return Err(ScaleTransferAuditError(format!(
                    "predicted-only comparison {} must not declare an observed value",
                    comparison.comparison_id
                )));
            }
        } else if comparison.disposition == ComparisonDisposition::Unknown {
            if comparison.observed_value.is_some() || comparison.observation_evidence_id.is_some() {
                return Err(ScaleTransferAuditError(format!(
                    "unknown comparison {} must not declare observed evidence",
                    comparison.comparison_id
                )));
            }
        }
    }
    Ok(())
}

/// Validate scale-stage lineage, explicit variable mappings and transfer evidence classes.
///
/// Stage scale quantities retain their own units; the audit never compares grams, area,
/// amp-hours and kilowatt-hours as though they were a common dimension.
pub fn audit_scale_transfer(
    manifest: &ScaleTransferManifest,
) -> Result<ScaleTransferAuditReport, ScaleTransferAuditError> {
    require_nonempty(&manifest.manifest_id, "manifest ID")?;
    if manifest.bridges.is_empty() {
        return Err(ScaleTransferAuditError(
            "at least one scale bridge is required".into(),
        ));
    }

    let registry = evidence_registry(&manifest.evidence)?;
    let stages = validate_stages(&manifest.stages, &registry)?;
    let bridges = validate_bridges(&manifest.bridges, &stages, &registry)?;
    validate_comparisons(&manifest.comparisons, &bridges, &registry)?;

    let mut invariant_mapping_count = 0;
    let mut rescaled_mapping_count = 0;
    let mut reoptimized_mapping_count = 0;
    let mut unknown_mapping_count = 0;
    for bridge in &manifest.bridges {
        for variable in &bridge.variables {
            match variable.mapping_kind {
                MappingKind::Invariant => invariant_mapping_count += 1,
                MappingKind::Rescaled => rescaled_mapping_count += 1,
                MappingKind::Reoptimized => reoptimized_mapping_count += 1,
                MappingKind::Unknown => unknown_mapping_count += 1,
            }
        }
    }

    let predicted_only_count = manifest
        .comparisons
        .iter()
        .filter(|item| item.disposition == ComparisonDisposition::PredictedOnly)
        .count();
    let physical_measurement_comparison_count = manifest
        .comparisons
        .iter()
        .filter(|item| item.disposition == ComparisonDisposition::PhysicalMeasurement)
        .count();
    let independent_laboratory_comparison_count = manifest
        .comparisons
        .iter()
        .filter(|item| item.disposition == ComparisonDisposition::IndependentLaboratory)
        .count();
    let unresolved_comparison_count = manifest
        .comparisons
        .iter()
        .filter(|item| item.disposition == ComparisonDisposition::Unknown)
        .count();

    let claim_ceiling = if physical_measurement_comparison_count
        + independent_laboratory_comparison_count
        > 0
    {
        "At least one admitted physical scale-transfer comparison is referenced for this exact bridge; this does not establish population-level scale-up capability, independent replication or product qualification.".into()
    } else {
        "Scale lineage and declared mapping references are internally consistent; no physical scale-transfer claim is established.".into()
    };

    Ok(ScaleTransferAuditReport {
        manifest_id: manifest.manifest_id.clone(),
        stage_count: manifest.stages.len(),
        bridge_count: manifest.bridges.len(),
        comparison_count: manifest.comparisons.len(),
        invariant_mapping_count,
        rescaled_mapping_count,
        reoptimized_mapping_count,
        unknown_mapping_count,
        predicted_only_count,
        physical_measurement_comparison_count,
        independent_laboratory_comparison_count,
        unresolved_comparison_count,
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
        admission: EvidenceAdmission,
        metrology_ref: Option<&str>,
    ) -> EvidenceRecord {
        EvidenceRecord {
            evidence_id: evidence_id.into(),
            source_digest: digest(character),
            evidence_class,
            admission,
            claim_scope: format!("test-only scale-transfer scope for {evidence_id}"),
            validity_domain: "synthetic unit test only".into(),
            metrology_ref: metrology_ref.map(str::to_owned),
        }
    }

    fn stage(
        stage_id: &str,
        stage_kind: ScaleStageKind,
        parents: &[&str],
        scale_value: f64,
        scale_unit: &str,
        digest_char: char,
    ) -> ScaleStage {
        ScaleStage {
            stage_id: stage_id.into(),
            stage_kind,
            design_id: "LFP-CANDIDATE-01".into(),
            design_revision: format!("rev-{stage_id}"),
            artifact_config_ref: format!("CONFIG-{stage_id}"),
            parent_stage_ids: parents.iter().map(|value| (*value).into()).collect(),
            scale_value,
            scale_unit: scale_unit.into(),
            stage_digest: digest(digest_char),
            evidence_id: "STAGE-SYNTH".into(),
        }
    }

    fn fixture() -> ScaleTransferManifest {
        let stages = vec![
            stage("MATERIAL", ScaleStageKind::Material, &[], 10.0, "g_active_material", '1'),
            stage("ELECTRODE", ScaleStageKind::Electrode, &["MATERIAL"], 100.0, "cm2_coated_area", '2'),
            stage("COIN", ScaleStageKind::CoinCell, &["ELECTRODE"], 5.0, "mAh_nominal", '3'),
            stage("SMALL-POUCH", ScaleStageKind::SmallPouch, &["COIN"], 1.0, "Ah_nominal", '4'),
        ];
        let bridges = vec![
            ScaleBridge {
                bridge_id: "MATERIAL-TO-ELECTRODE".into(),
                source_stage_id: "MATERIAL".into(),
                target_stage_id: "ELECTRODE".into(),
                mapping_profile_ref: "MAP-PRECUSOR-COATING-R1".into(),
                variables: vec![
                    ScaleVariableMapping {
                        variable_id: "chemistry_family".into(),
                        mapping_kind: MappingKind::Invariant,
                        rationale: "candidate chemistry is unchanged in this synthetic bridge".into(),
                        evidence_id: "BRIDGE-SYNTH".into(),
                    },
                    ScaleVariableMapping {
                        variable_id: "areal_loading".into(),
                        mapping_kind: MappingKind::Reoptimized,
                        rationale: "coating loading is re-optimized at electrode stage".into(),
                        evidence_id: "BRIDGE-SYNTH".into(),
                    },
                ],
                evidence_id: "BRIDGE-SYNTH".into(),
            },
            ScaleBridge {
                bridge_id: "ELECTRODE-TO-COIN".into(),
                source_stage_id: "ELECTRODE".into(),
                target_stage_id: "COIN".into(),
                mapping_profile_ref: "MAP-ELECTRODE-COIN-R1".into(),
                variables: vec![
                    ScaleVariableMapping {
                        variable_id: "electrode_thickness".into(),
                        mapping_kind: MappingKind::Invariant,
                        rationale: "nominal electrode thickness is held constant in the synthetic case".into(),
                        evidence_id: "BRIDGE-SYNTH".into(),
                    },
                    ScaleVariableMapping {
                        variable_id: "stack_pressure".into(),
                        mapping_kind: MappingKind::Unknown,
                        rationale: "fixture leaves pressure mapping unresolved".into(),
                        evidence_id: "BRIDGE-SYNTH".into(),
                    },
                ],
                evidence_id: "BRIDGE-SYNTH".into(),
            },
            ScaleBridge {
                bridge_id: "COIN-TO-POUCH".into(),
                source_stage_id: "COIN".into(),
                target_stage_id: "SMALL-POUCH".into(),
                mapping_profile_ref: "MAP-COIN-POUCH-R1".into(),
                variables: vec![
                    ScaleVariableMapping {
                        variable_id: "electrode_area".into(),
                        mapping_kind: MappingKind::Rescaled,
                        rationale: "format area changes".into(),
                        evidence_id: "BRIDGE-SYNTH".into(),
                    },
                ],
                evidence_id: "BRIDGE-SYNTH".into(),
            },
        ];
        let comparisons = vec![
            TransferComparison {
                comparison_id: "COIN-TO-POUCH-PREDICTION".into(),
                bridge_id: "COIN-TO-POUCH".into(),
                quantity_id: "capacity_retention".into(),
                unit: "percent".into(),
                boundary: "complete_cell".into(),
                predicted_value: 96.0,
                prediction_lower_bound: 90.0,
                prediction_upper_bound: 99.0,
                observed_value: None,
                prediction_evidence_id: "PREDICTION-SYNTH".into(),
                observation_evidence_id: None,
                disposition: ComparisonDisposition::PredictedOnly,
                comparison_digest: digest('5'),
            },
            TransferComparison {
                comparison_id: "ELECTRODE-TO-COIN-UNKNOWN".into(),
                bridge_id: "ELECTRODE-TO-COIN".into(),
                quantity_id: "rate_capability".into(),
                unit: "C_rate".into(),
                boundary: "complete_cell".into(),
                predicted_value: 1.0,
                prediction_lower_bound: 0.5,
                prediction_upper_bound: 2.0,
                observed_value: None,
                prediction_evidence_id: "PREDICTION-SYNTH".into(),
                observation_evidence_id: None,
                disposition: ComparisonDisposition::Unknown,
                comparison_digest: digest('6'),
            },
        ];
        ScaleTransferManifest {
            manifest_id: "SYNTH-SCALE-TRANSFER-001".into(),
            stages,
            bridges,
            comparisons,
            evidence: vec![
                evidence(
                    "STAGE-SYNTH",
                    'a',
                    EvidenceClass::SyntheticFixture,
                    EvidenceAdmission::AdmittedForEvaluation,
                    None,
                ),
                evidence(
                    "BRIDGE-SYNTH",
                    'b',
                    EvidenceClass::SyntheticFixture,
                    EvidenceAdmission::AdmittedForEvaluation,
                    None,
                ),
                evidence(
                    "PREDICTION-SYNTH",
                    'c',
                    EvidenceClass::Calculation,
                    EvidenceAdmission::AdmittedForEvaluation,
                    None,
                ),
            ],
        }
    }

    #[test]
    fn synthetic_scale_chain_closes_without_claiming_physical_transfer() {
        let report = audit_scale_transfer(&fixture()).expect("synthetic lineage should close");
        assert_eq!(report.stage_count, 4);
        assert_eq!(report.bridge_count, 3);
        assert_eq!(report.comparison_count, 2);
        assert_eq!(report.invariant_mapping_count, 2);
        assert_eq!(report.rescaled_mapping_count, 1);
        assert_eq!(report.reoptimized_mapping_count, 1);
        assert_eq!(report.unknown_mapping_count, 1);
        assert_eq!(report.predicted_only_count, 1);
        assert_eq!(report.unresolved_comparison_count, 1);
        assert_eq!(report.physical_measurement_comparison_count, 0);
    }

    #[test]
    fn non_root_stage_requires_parent_lineage() {
        let mut value = fixture();
        value.stages[1].parent_stage_ids.clear();
        assert!(audit_scale_transfer(&value).is_err());
    }

    #[test]
    fn parent_stage_must_be_lower_scale_rank() {
        let mut value = fixture();
        value.stages[0].parent_stage_ids = vec!["ELECTRODE".into()];
        assert!(audit_scale_transfer(&value).is_err());
    }

    #[test]
    fn parent_candidate_lineage_must_match() {
        let mut value = fixture();
        value.stages[1].design_id = "DIFFERENT-CANDIDATE".into();
        assert!(audit_scale_transfer(&value).is_err());
    }

    #[test]
    fn bridge_must_match_a_direct_parent_edge() {
        let mut value = fixture();
        value.bridges[2].source_stage_id = "MATERIAL".into();
        assert!(audit_scale_transfer(&value).is_err());
    }

    #[test]
    fn unknown_mapping_is_preserved_and_reported() {
        let report = audit_scale_transfer(&fixture()).expect("unknown mapping is representable");
        assert_eq!(report.unknown_mapping_count, 1);
    }

    #[test]
    fn predicted_only_comparison_cannot_carry_an_observed_value() {
        let mut value = fixture();
        value.comparisons[0].observed_value = Some(95.0);
        assert!(audit_scale_transfer(&value).is_err());
    }

    #[test]
    fn physical_comparison_requires_physical_evidence_and_metrology() {
        let mut value = fixture();
        value.evidence.push(evidence(
            "TEST-PHYSICAL",
            'd',
            EvidenceClass::PhysicalMeasurement,
            EvidenceAdmission::AdmittedForEvaluation,
            None,
        ));
        value.comparisons[0].disposition = ComparisonDisposition::PhysicalMeasurement;
        value.comparisons[0].observed_value = Some(94.0);
        value.comparisons[0].observation_evidence_id = Some("TEST-PHYSICAL".into());
        assert!(audit_scale_transfer(&value).is_err());
    }

    #[test]
    fn physical_comparison_requires_an_independent_measurement_class() {
        let mut value = fixture();
        value.evidence.push(evidence(
            "TEST-SIMULATION",
            'd',
            EvidenceClass::Simulation,
            EvidenceAdmission::AdmittedForEvaluation,
            None,
        ));
        value.comparisons[0].disposition = ComparisonDisposition::PhysicalMeasurement;
        value.comparisons[0].observed_value = Some(94.0);
        value.comparisons[0].observation_evidence_id = Some("TEST-SIMULATION".into());
        assert!(audit_scale_transfer(&value).is_err());
    }

    #[test]
    fn admitted_physical_comparison_is_counted_without_overclaiming_population_transfer() {
        let mut value = fixture();
        value.evidence.push(evidence(
            "TEST-PHYSICAL",
            'd',
            EvidenceClass::PhysicalMeasurement,
            EvidenceAdmission::AdmittedForEvaluation,
            Some("TEST-METROLOGY-REF"),
        ));
        value.comparisons[0].disposition = ComparisonDisposition::PhysicalMeasurement;
        value.comparisons[0].observed_value = Some(94.0);
        value.comparisons[0].observation_evidence_id = Some("TEST-PHYSICAL".into());
        let report = audit_scale_transfer(&value).expect("physical evidence structure is valid");
        assert_eq!(report.physical_measurement_comparison_count, 1);
        assert!(report.claim_ceiling.contains("does not establish population-level"));
    }

    #[test]
    fn malformed_prediction_interval_is_rejected() {
        let mut value = fixture();
        value.comparisons[0].prediction_lower_bound = 97.0;
        assert!(audit_scale_transfer(&value).is_err());
    }

    #[test]
    fn report_is_invariant_under_input_permutation() {
        let first = audit_scale_transfer(&fixture()).expect("first report");
        let mut value = fixture();
        value.stages.reverse();
        value.bridges.reverse();
        value.comparisons.reverse();
        value.evidence.reverse();
        let second = audit_scale_transfer(&value).expect("permuted report");
        assert_eq!(first, second);
    }
}
