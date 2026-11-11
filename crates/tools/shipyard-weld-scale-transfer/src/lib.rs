//! Fail-closed scale-transfer evidence gate for welding distortion models.
//!
//! V0 is a synthetic contract evaluator. It does not solve thermomechanics,
//! validate a physical model, qualify a welding procedure or authorize release.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ScaleTier {
    BeadOrCoupon,
    JointSpecimen,
    Subassembly,
    BlockAssembly,
}

impl ScaleTier {
    fn rank(self) -> u8 {
        match self {
            Self::BeadOrCoupon => 0,
            Self::JointSpecimen => 1,
            Self::Subassembly => 2,
            Self::BlockAssembly => 3,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DomainSignature {
    pub material_family: String,
    pub material_grade: String,
    pub joint_family: String,
    pub penetration_class: String,
    pub process_family: String,
    pub process_variant: String,
    pub pass_topology: String,
    pub restraint_class: String,
    pub scale_tier: ScaleTier,
    pub thickness_min_mm: u16,
    pub thickness_max_mm: u16,
    pub boundary_class: String,
    pub output_scope: String,
    pub coordinate_frame: String,
}

impl DomainSignature {
    fn validate(&self) -> Result<(), TransferReason> {
        let required = [
            self.material_family.as_str(),
            self.material_grade.as_str(),
            self.joint_family.as_str(),
            self.penetration_class.as_str(),
            self.process_family.as_str(),
            self.process_variant.as_str(),
            self.pass_topology.as_str(),
            self.restraint_class.as_str(),
            self.boundary_class.as_str(),
            self.output_scope.as_str(),
            self.coordinate_frame.as_str(),
        ];
        if required.iter().any(|value| value.trim().is_empty())
            || self.thickness_min_mm == 0
            || self.thickness_min_mm > self.thickness_max_mm
        {
            return Err(TransferReason::InvalidDomainSignature);
        }
        Ok(())
    }

    fn changed_factors(&self, other: &Self) -> Vec<String> {
        let mut changed = Vec::new();
        if self.material_family != other.material_family {
            changed.push("material_family".to_owned());
        }
        if self.material_grade != other.material_grade {
            changed.push("material_grade".to_owned());
        }
        if self.joint_family != other.joint_family {
            changed.push("joint_family".to_owned());
        }
        if self.penetration_class != other.penetration_class {
            changed.push("penetration_class".to_owned());
        }
        if self.process_family != other.process_family {
            changed.push("process_family".to_owned());
        }
        if self.process_variant != other.process_variant {
            changed.push("process_variant".to_owned());
        }
        if self.pass_topology != other.pass_topology {
            changed.push("pass_topology".to_owned());
        }
        if self.restraint_class != other.restraint_class {
            changed.push("restraint_class".to_owned());
        }
        if self.scale_tier != other.scale_tier {
            changed.push("scale_tier".to_owned());
        }
        if self.thickness_min_mm != other.thickness_min_mm
            || self.thickness_max_mm != other.thickness_max_mm
        {
            changed.push("thickness_range".to_owned());
        }
        if self.boundary_class != other.boundary_class {
            changed.push("boundary_class".to_owned());
        }
        if self.output_scope != other.output_scope {
            changed.push("output_scope".to_owned());
        }
        if self.coordinate_frame != other.coordinate_frame {
            changed.push("coordinate_frame".to_owned());
        }
        changed.sort();
        changed
    }

    fn contains_thickness_range(&self, other: &Self) -> bool {
        other.thickness_min_mm >= self.thickness_min_mm
            && other.thickness_max_mm <= self.thickness_max_mm
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DomainNode {
    pub domain_id: String,
    pub signature: DomainSignature,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ModelRecord {
    pub model_id: String,
    pub revision: String,
    pub family: String,
    pub calibration_domain_id: String,
    pub output_scope: String,
    pub validation_status: String,
    pub model_digest_sha256: String,
    pub parameter_set_digest_sha256: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BridgeEdge {
    pub edge_id: String,
    pub from_domain_id: String,
    pub to_domain_id: String,
    pub model_id: String,
    pub model_revision: String,
    pub output_scope: String,
    pub changed_factors: Vec<String>,
    pub calibration_specimen_ids: Vec<String>,
    pub validation_specimen_ids: Vec<String>,
    pub validation_status: String,
    pub maximum_error_milli: u64,
    pub allowed_error_milli: u64,
    pub empirical_coverage_permille: u16,
    pub minimum_coverage_permille: u16,
    pub measurement_uncertainty_units: u64,
    pub parameter_uncertainty_units: u64,
    pub boundary_uncertainty_units: u64,
    pub model_form_uncertainty_units: u64,
    pub transfer_uncertainty_units: u64,
    pub evidence_digest_sha256: String,
    pub validated_at_tick: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TransferRequest {
    pub target_domain_id: String,
    pub requested_output_scope: String,
    pub decision_cutoff_tick: u64,
    pub maximum_total_uncertainty_units: u64,
    pub required_minimum_coverage_permille: u16,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TransferManifest {
    pub schema_version: String,
    pub model: ModelRecord,
    pub domains: Vec<DomainNode>,
    pub bridge_edges: Vec<BridgeEdge>,
    pub request: TransferRequest,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TransferDisposition {
    AdmittedInDomain,
    AdmittedWithBridgeChain,
    AbstainNoBridge,
    AbstainOutOfDomain,
    AbstainOutputScope,
    AbstainEvidenceInvalid,
    AbstainUncertainty,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TransferReason {
    EmptyIdentity,
    InvalidDomainSignature,
    DuplicateDomainId,
    DuplicateEdgeId,
    ModelNotValidated,
    InvalidModelDigest,
    SourceDomainMissing,
    TargetDomainMissing,
    OutputScopeMismatch,
    NoBridgePath,
    BridgeEvidenceInvalid,
    BridgeEdgeNotAdjacentScale,
    BridgeChangedFactorsMismatch,
    BridgeSpecimenLeakage,
    BridgeCoverageInsufficient,
    BridgeErrorExceedsTolerance,
    BridgeEvidenceFromFuture,
    UncertaintyLimitExceeded,
    NumericOverflow,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BridgePathReport {
    pub edge_ids: Vec<String>,
    pub domain_ids: Vec<String>,
    pub changed_factors: Vec<String>,
    pub cumulative_uncertainty_units: u64,
    pub minimum_empirical_coverage_permille: u16,
    pub maximum_error_milli: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TransferDecision {
    pub disposition: TransferDisposition,
    pub reason: Option<TransferReason>,
    pub model_id: String,
    pub model_revision: String,
    pub source_domain_id: String,
    pub target_domain_id: String,
    pub output_scope: String,
    pub selected_path: Option<BridgePathReport>,
    pub candidate_path_count: usize,
    pub claim_ceiling: String,
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
        && value.bytes().any(|byte| byte != b'0')
}

fn has_duplicates(values: &[String]) -> bool {
    let mut seen = HashSet::new();
    values.iter().any(|value| !seen.insert(value.as_str()))
}

fn domain_map(manifest: &TransferManifest) -> Result<HashMap<&str, &DomainNode>, TransferReason> {
    let mut map = HashMap::new();
    for domain in &manifest.domains {
        if domain.domain_id.trim().is_empty() {
            return Err(TransferReason::EmptyIdentity);
        }
        domain.signature.validate()?;
        if map.insert(domain.domain_id.as_str(), domain).is_some() {
            return Err(TransferReason::DuplicateDomainId);
        }
    }
    Ok(map)
}

fn is_in_domain(source: &DomainSignature, target: &DomainSignature) -> bool {
    let mut source_without_thickness = source.clone();
    let mut target_without_thickness = target.clone();
    source_without_thickness.thickness_min_mm = 1;
    source_without_thickness.thickness_max_mm = u16::MAX;
    target_without_thickness.thickness_min_mm = 1;
    target_without_thickness.thickness_max_mm = u16::MAX;
    source_without_thickness == target_without_thickness
        && source.contains_thickness_range(target)
}

fn edge_uncertainty(edge: &BridgeEdge) -> Result<u64, TransferReason> {
    [
        edge.measurement_uncertainty_units,
        edge.parameter_uncertainty_units,
        edge.boundary_uncertainty_units,
        edge.model_form_uncertainty_units,
        edge.transfer_uncertainty_units,
    ]
    .into_iter()
    .try_fold(0_u64, |total, value| {
        total.checked_add(value).ok_or(TransferReason::NumericOverflow)
    })
}

fn validate_edge(
    edge: &BridgeEdge,
    from: &DomainNode,
    to: &DomainNode,
    manifest: &TransferManifest,
) -> Result<u64, TransferReason> {
    if edge.edge_id.trim().is_empty()
        || edge.model_id != manifest.model.model_id
        || edge.model_revision != manifest.model.revision
        || edge.output_scope != manifest.request.requested_output_scope
        || edge.validation_status != "independently_validated"
        || !valid_sha256(&edge.evidence_digest_sha256)
        || edge.validation_specimen_ids.is_empty()
        || edge.calibration_specimen_ids.is_empty()
        || has_duplicates(&edge.validation_specimen_ids)
        || has_duplicates(&edge.calibration_specimen_ids)
    {
        return Err(TransferReason::BridgeEvidenceInvalid);
    }
    if edge
        .calibration_specimen_ids
        .iter()
        .any(|specimen| edge.validation_specimen_ids.contains(specimen))
    {
        return Err(TransferReason::BridgeSpecimenLeakage);
    }
    if edge.validated_at_tick > manifest.request.decision_cutoff_tick {
        return Err(TransferReason::BridgeEvidenceFromFuture);
    }
    if edge.empirical_coverage_permille < edge.minimum_coverage_permille
        || edge.empirical_coverage_permille < manifest.request.required_minimum_coverage_permille
        || edge.empirical_coverage_permille > 1_000
        || edge.minimum_coverage_permille > 1_000
    {
        return Err(TransferReason::BridgeCoverageInsufficient);
    }
    if edge.maximum_error_milli > edge.allowed_error_milli {
        return Err(TransferReason::BridgeErrorExceedsTolerance);
    }
    if from.signature.scale_tier.rank().abs_diff(to.signature.scale_tier.rank()) > 1 {
        return Err(TransferReason::BridgeEdgeNotAdjacentScale);
    }
    let expected = from.signature.changed_factors(&to.signature);
    let mut actual = edge.changed_factors.clone();
    actual.sort();
    if expected != actual || has_duplicates(&actual) {
        return Err(TransferReason::BridgeChangedFactorsMismatch);
    }
    edge_uncertainty(edge)
}

fn enumerate_paths(
    current: &str,
    target: &str,
    edges: &[BridgeEdge],
    visited: &mut HashSet<String>,
    path: &mut Vec<BridgeEdge>,
    output: &mut Vec<Vec<BridgeEdge>>,
    limit: usize,
) {
    if output.len() >= limit {
        return;
    }
    if current == target {
        output.push(path.clone());
        return;
    }
    for edge in edges.iter().filter(|edge| edge.from_domain_id == current) {
        if visited.contains(&edge.to_domain_id) {
            continue;
        }
        visited.insert(edge.to_domain_id.clone());
        path.push(edge.clone());
        enumerate_paths(
            &edge.to_domain_id,
            target,
            edges,
            visited,
            path,
            output,
            limit,
        );
        path.pop();
        visited.remove(&edge.to_domain_id);
    }
}

fn empty_decision(
    manifest: &TransferManifest,
    disposition: TransferDisposition,
    reason: TransferReason,
    source_domain_id: String,
    target_domain_id: String,
    candidate_path_count: usize,
) -> TransferDecision {
    TransferDecision {
        disposition,
        reason: Some(reason),
        model_id: manifest.model.model_id.clone(),
        model_revision: manifest.model.revision.clone(),
        source_domain_id,
        target_domain_id,
        output_scope: manifest.request.requested_output_scope.clone(),
        selected_path: None,
        candidate_path_count,
        claim_ceiling: "SYNTHETIC_SCALE_TRANSFER_CONTRACT_ONLY".to_owned(),
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CorpusSummary {
    pub benchmark_id: String,
    pub cases: usize,
    pub positive: usize,
    pub negative: usize,
    pub corpus_fingerprint_fnv1a64: String,
}

pub fn validate_corpus(bytes: &[u8]) -> Result<CorpusSummary, String> {
    let value: serde_json::Value =
        serde_json::from_slice(bytes).map_err(|error| error.to_string())?;
    let benchmark_id = value
        .get("benchmark_id")
        .and_then(serde_json::Value::as_str)
        .ok_or_else(|| "benchmark_id is required".to_owned())?;
    let cases = value
        .get("cases")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "cases array is required".to_owned())?;
    if value.get("schema_version").and_then(serde_json::Value::as_str) != Some("0.1.0")
        || benchmark_id != "SHIPYARD-WELD-SCALE-TRANSFER-001"
        || cases.len() != 30
    {
        return Err("corpus version, identity or fixed case count changed".to_owned());
    }
    let mut ids = HashSet::new();
    let mut positive = 0usize;
    let mut negative = 0usize;
    for (index, case) in cases.iter().enumerate() {
        let id = case
            .get("case_id")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| format!("cases[{index}].case_id is required"))?;
        let class_name = case
            .get("class")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| format!("cases[{index}].class is required"))?;
        if !ids.insert(id) {
            return Err(format!("duplicate case id: {id}"));
        }
        match class_name {
            "positive" => positive += 1,
            "negative" => negative += 1,
            _ => return Err(format!("invalid case class: {class_name}")),
        }
        if case.get("mutation").and_then(serde_json::Value::as_str).is_none()
            || case
                .get("expected_disposition")
                .and_then(serde_json::Value::as_str)
                .is_none()
        {
            return Err(format!("case {id} lacks mutation or expected disposition"));
        }
    }
    if positive != 2 || negative != 28 {
        return Err(format!("expected 2 positive / 28 negative cases, got {positive}/{negative}"));
    }
    Ok(CorpusSummary {
        benchmark_id: benchmark_id.to_owned(),
        cases: cases.len(),
        positive,
        negative,
        corpus_fingerprint_fnv1a64: format!("{:x}", fnv1a64(bytes)),
    })
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325_u64, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
    })
}

pub fn evaluate_transfer(manifest: &TransferManifest) -> TransferDecision {
    if manifest.schema_version != "0.1.0"
        || manifest.request.required_minimum_coverage_permille == 0
        || manifest.request.required_minimum_coverage_permille > 1_000
    {
        return empty_decision(
            manifest,
            TransferDisposition::AbstainEvidenceInvalid,
            TransferReason::BridgeEvidenceInvalid,
            manifest.model.calibration_domain_id.clone(),
            manifest.request.target_domain_id.clone(),
            0,
        );
    }

    let source_id = manifest.model.calibration_domain_id.clone();
    let target_id = manifest.request.target_domain_id.clone();
    let domains = match domain_map(manifest) {
        Ok(value) => value,
        Err(reason) => {
            return empty_decision(
                manifest,
                TransferDisposition::AbstainEvidenceInvalid,
                reason,
                source_id,
                target_id,
                0,
            );
        }
    };

    if manifest.model.model_id.trim().is_empty()
        || manifest.model.revision.trim().is_empty()
        || manifest.model.family.trim().is_empty()
        || manifest.request.requested_output_scope.trim().is_empty()
    {
        return empty_decision(
            manifest,
            TransferDisposition::AbstainEvidenceInvalid,
            TransferReason::EmptyIdentity,
            source_id,
            target_id,
            0,
        );
    }
    if manifest.model.validation_status != "validated"
        || !valid_sha256(&manifest.model.model_digest_sha256)
        || !valid_sha256(&manifest.model.parameter_set_digest_sha256)
    {
        return empty_decision(
            manifest,
            TransferDisposition::AbstainEvidenceInvalid,
            if manifest.model.validation_status != "validated" {
                TransferReason::ModelNotValidated
            } else {
                TransferReason::InvalidModelDigest
            },
            source_id,
            target_id,
            0,
        );
    }

    let Some(source) = domains.get(source_id.as_str()).copied() else {
        return empty_decision(
            manifest,
            TransferDisposition::AbstainEvidenceInvalid,
            TransferReason::SourceDomainMissing,
            source_id,
            target_id,
            0,
        );
    };
    let Some(target) = domains.get(target_id.as_str()).copied() else {
        return empty_decision(
            manifest,
            TransferDisposition::AbstainEvidenceInvalid,
            TransferReason::TargetDomainMissing,
            source_id,
            target_id,
            0,
        );
    };

    if source.signature.output_scope != manifest.model.output_scope
        || target.signature.output_scope != manifest.request.requested_output_scope
        || manifest.model.output_scope != manifest.request.requested_output_scope
    {
        return empty_decision(
            manifest,
            TransferDisposition::AbstainOutputScope,
            TransferReason::OutputScopeMismatch,
            source_id,
            target_id,
            0,
        );
    }

    if source_id == target_id || is_in_domain(&source.signature, &target.signature) {
        return TransferDecision {
            disposition: TransferDisposition::AdmittedInDomain,
            reason: None,
            model_id: manifest.model.model_id.clone(),
            model_revision: manifest.model.revision.clone(),
            source_domain_id: source_id,
            target_domain_id: target_id,
            output_scope: manifest.request.requested_output_scope.clone(),
            selected_path: None,
            candidate_path_count: 0,
            claim_ceiling: "SYNTHETIC_SCALE_TRANSFER_CONTRACT_ONLY".to_owned(),
        };
    }

    let mut edge_ids = HashSet::new();
    for edge in &manifest.bridge_edges {
        if edge.edge_id.trim().is_empty() || !edge_ids.insert(edge.edge_id.as_str()) {
            return empty_decision(
                manifest,
                TransferDisposition::AbstainEvidenceInvalid,
                TransferReason::DuplicateEdgeId,
                source_id,
                target_id,
                0,
            );
        }
        if !domains.contains_key(edge.from_domain_id.as_str())
            || !domains.contains_key(edge.to_domain_id.as_str())
        {
            return empty_decision(
                manifest,
                TransferDisposition::AbstainEvidenceInvalid,
                TransferReason::BridgeEvidenceInvalid,
                source_id,
                target_id,
                0,
            );
        }
    }

    let mut paths = Vec::new();
    let mut visited = HashSet::new();
    visited.insert(source_id.clone());
    enumerate_paths(
        &source_id,
        &target_id,
        &manifest.bridge_edges,
        &mut visited,
        &mut Vec::new(),
        &mut paths,
        256,
    );
    if paths.is_empty() {
        let thickness_only_or_outside = source.signature.changed_factors(&target.signature);
        return empty_decision(
            manifest,
            if thickness_only_or_outside.contains(&"thickness_range".to_owned())
                || source.signature.scale_tier != target.signature.scale_tier
            {
                TransferDisposition::AbstainOutOfDomain
            } else {
                TransferDisposition::AbstainNoBridge
            },
            TransferReason::NoBridgePath,
            source_id,
            target_id,
            0,
        );
    }

    let mut accepted = Vec::new();
    let mut first_failure = None;
    for path in &paths {
        let mut total_uncertainty = 0_u64;
        let mut minimum_coverage = 1_000_u16;
        let mut maximum_error = 0_u64;
        let mut valid = true;
        let mut path_calibration_specimens = HashSet::new();
        let mut path_validation_specimens = HashSet::new();
        for edge in path {
            if edge.validation_specimen_ids.iter().any(|specimen| {
                path_validation_specimens.contains(specimen)
                    || path_calibration_specimens.contains(specimen)
            }) || edge
                .calibration_specimen_ids
                .iter()
                .any(|specimen| path_validation_specimens.contains(specimen))
            {
                first_failure = Some(TransferReason::BridgeSpecimenLeakage);
                valid = false;
                break;
            }
            path_calibration_specimens.extend(edge.calibration_specimen_ids.iter().cloned());
            path_validation_specimens.extend(edge.validation_specimen_ids.iter().cloned());
            let Some(from) = domains.get(edge.from_domain_id.as_str()).copied() else {
                first_failure = Some(TransferReason::BridgeEvidenceInvalid);
                valid = false;
                break;
            };
            let Some(to) = domains.get(edge.to_domain_id.as_str()).copied() else {
                first_failure = Some(TransferReason::BridgeEvidenceInvalid);
                valid = false;
                break;
            };
            match validate_edge(edge, from, to, manifest) {
                Ok(uncertainty) => {
                    let Some(sum) = total_uncertainty.checked_add(uncertainty) else {
                        first_failure = Some(TransferReason::NumericOverflow);
                        valid = false;
                        break;
                    };
                    total_uncertainty = sum;
                    minimum_coverage = minimum_coverage.min(edge.empirical_coverage_permille);
                    maximum_error = maximum_error.max(edge.maximum_error_milli);
                }
                Err(reason) => {
                    first_failure = Some(reason);
                    valid = false;
                    break;
                }
            }
        }
        if valid {
            let mut domain_ids = Vec::with_capacity(path.len() + 1);
            domain_ids.push(source_id.clone());
            domain_ids.extend(path.iter().map(|edge| edge.to_domain_id.clone()));
            let changed_factors = path
                .iter()
                .flat_map(|edge| edge.changed_factors.clone())
                .collect::<HashSet<_>>()
                .into_iter()
                .collect::<Vec<_>>();
            let mut changed_factors = changed_factors;
            changed_factors.sort();
            accepted.push(BridgePathReport {
                edge_ids: path.iter().map(|edge| edge.edge_id.clone()).collect(),
                domain_ids,
                changed_factors,
                cumulative_uncertainty_units: total_uncertainty,
                minimum_empirical_coverage_permille: minimum_coverage,
                maximum_error_milli: maximum_error,
            });
        }
    }

    accepted.sort_by(|left, right| {
        left.cumulative_uncertainty_units
            .cmp(&right.cumulative_uncertainty_units)
            .then_with(|| left.edge_ids.len().cmp(&right.edge_ids.len()))
            .then_with(|| left.edge_ids.cmp(&right.edge_ids))
    });
    let Some(best) = accepted.into_iter().next() else {
        return empty_decision(
            manifest,
            TransferDisposition::AbstainEvidenceInvalid,
            first_failure.unwrap_or(TransferReason::BridgeEvidenceInvalid),
            source_id,
            target_id,
            paths.len(),
        );
    };
    if best.cumulative_uncertainty_units > manifest.request.maximum_total_uncertainty_units {
        return empty_decision(
            manifest,
            TransferDisposition::AbstainUncertainty,
            TransferReason::UncertaintyLimitExceeded,
            source_id,
            target_id,
            paths.len(),
        );
    }

    TransferDecision {
        disposition: TransferDisposition::AdmittedWithBridgeChain,
        reason: None,
        model_id: manifest.model.model_id.clone(),
        model_revision: manifest.model.revision.clone(),
        source_domain_id: source_id,
        target_domain_id: target_id,
        output_scope: manifest.request.requested_output_scope.clone(),
        selected_path: Some(best),
        candidate_path_count: paths.len(),
        claim_ceiling: "SYNTHETIC_SCALE_TRANSFER_CONTRACT_ONLY".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn signature(scale: ScaleTier, thickness_min: u16, thickness_max: u16) -> DomainSignature {
        DomainSignature {
            material_family: "ferritic_steel".to_owned(),
            material_grade: "S355J2+N".to_owned(),
            joint_family: "single_v_groove".to_owned(),
            penetration_class: "full_penetration".to_owned(),
            process_family: "GMAW".to_owned(),
            process_variant: "tandem_pmc_cmt".to_owned(),
            pass_topology: "six_layer".to_owned(),
            restraint_class: match scale {
                ScaleTier::BeadOrCoupon => "coupon_rigid_fixture",
                ScaleTier::JointSpecimen => "joint_test_fixture",
                ScaleTier::Subassembly => "stiffened_panel_fixture",
                ScaleTier::BlockAssembly => "block_gravity_fixture",
            }
            .to_owned(),
            scale_tier: scale,
            thickness_min_mm: thickness_min,
            thickness_max_mm: thickness_max,
            boundary_class: "ambient_air_cooling".to_owned(),
            output_scope: "released_angular_distortion".to_owned(),
            coordinate_frame: "welding_reference_frame".to_owned(),
        }
    }

    fn node(id: &str, sig: DomainSignature) -> DomainNode {
        DomainNode {
            domain_id: id.to_owned(),
            signature: sig,
        }
    }

    fn edge(
        id: &str,
        from: &str,
        to: &str,
        from_signature: &DomainSignature,
        to_signature: &DomainSignature,
    ) -> BridgeEdge {
        BridgeEdge {
            edge_id: id.to_owned(),
            from_domain_id: from.to_owned(),
            to_domain_id: to.to_owned(),
            model_id: "MODEL-TEP-01".to_owned(),
            model_revision: "R3".to_owned(),
            output_scope: "released_angular_distortion".to_owned(),
            changed_factors: from_signature.changed_factors(to_signature),
            calibration_specimen_ids: vec![format!("CAL-{id}")],
            validation_specimen_ids: vec![format!("VAL-{id}-A"), format!("VAL-{id}-B")],
            validation_status: "independently_validated".to_owned(),
            maximum_error_milli: 80,
            allowed_error_milli: 100,
            empirical_coverage_permille: 960,
            minimum_coverage_permille: 950,
            measurement_uncertainty_units: 1,
            parameter_uncertainty_units: 2,
            boundary_uncertainty_units: 3,
            model_form_uncertainty_units: 4,
            transfer_uncertainty_units: 5,
            evidence_digest_sha256: "a".repeat(64),
            validated_at_tick: 90,
        }
    }

    fn base_manifest() -> TransferManifest {
        let coupon = signature(ScaleTier::BeadOrCoupon, 28, 32);
        let joint = signature(ScaleTier::JointSpecimen, 28, 32);
        let panel = signature(ScaleTier::Subassembly, 28, 32);
        let block = signature(ScaleTier::BlockAssembly, 28, 32);
        TransferManifest {
            schema_version: "0.1.0".to_owned(),
            model: ModelRecord {
                model_id: "MODEL-TEP-01".to_owned(),
                revision: "R3".to_owned(),
                family: "transient_thermo_elasto_plastic_fem".to_owned(),
                calibration_domain_id: "D0".to_owned(),
                output_scope: "released_angular_distortion".to_owned(),
                validation_status: "validated".to_owned(),
                model_digest_sha256: "b".repeat(64),
                parameter_set_digest_sha256: "c".repeat(64),
            },
            domains: vec![
                node("D0", coupon.clone()),
                node("D1", joint.clone()),
                node("D2", panel.clone()),
                node("D3", block.clone()),
            ],
            bridge_edges: vec![
                edge("E01", "D0", "D1", &coupon, &joint),
                edge("E12", "D1", "D2", &joint, &panel),
                edge("E23", "D2", "D3", &panel, &block),
            ],
            request: TransferRequest {
                target_domain_id: "D3".to_owned(),
                requested_output_scope: "released_angular_distortion".to_owned(),
                decision_cutoff_tick: 100,
                maximum_total_uncertainty_units: 60,
                required_minimum_coverage_permille: 950,
            },
        }
    }

    #[test]
    fn valid_three_edge_scale_chain_is_admitted() {
        let result = evaluate_transfer(&base_manifest());
        assert_eq!(result.disposition, TransferDisposition::AdmittedWithBridgeChain);
        let path = result.selected_path.expect("validated bridge path");
        assert_eq!(path.edge_ids, vec!["E01", "E12", "E23"]);
        assert_eq!(path.domain_ids, vec!["D0", "D1", "D2", "D3"]);
        assert_eq!(path.cumulative_uncertainty_units, 45);
    }

    #[test]
    fn exact_calibration_domain_is_admitted_without_bridge() {
        let mut manifest = base_manifest();
        manifest.request.target_domain_id = "D0".to_owned();
        let result = evaluate_transfer(&manifest);
        assert_eq!(result.disposition, TransferDisposition::AdmittedInDomain);
        assert!(result.selected_path.is_none());
    }

    #[test]
    fn target_thickness_inside_calibration_range_is_in_domain() {
        let mut manifest = base_manifest();
        manifest.domains[3].signature = signature(ScaleTier::BeadOrCoupon, 29, 31);
        manifest.request.target_domain_id = "D3".to_owned();
        let result = evaluate_transfer(&manifest);
        assert_eq!(result.disposition, TransferDisposition::AdmittedInDomain);
    }

    #[test]
    fn direct_coupon_to_block_edge_is_rejected_for_skipping_scale_tiers() {
        let mut manifest = base_manifest();
        let coupon = manifest.domains[0].signature.clone();
        let block = manifest.domains[3].signature.clone();
        manifest.bridge_edges = vec![edge("E03", "D0", "D3", &coupon, &block)];
        let result = evaluate_transfer(&manifest);
        assert_eq!(result.disposition, TransferDisposition::AbstainEvidenceInvalid);
        assert_eq!(result.reason, Some(TransferReason::BridgeEdgeNotAdjacentScale));
    }

    #[test]
    fn missing_bridge_path_abstains_out_of_domain() {
        let mut manifest = base_manifest();
        manifest.bridge_edges.clear();
        let result = evaluate_transfer(&manifest);
        assert_eq!(result.disposition, TransferDisposition::AbstainOutOfDomain);
        assert_eq!(result.reason, Some(TransferReason::NoBridgePath));
    }

    #[test]
    fn material_change_requires_bridge_path() {
        let mut manifest = base_manifest();
        manifest.domains[3].signature.material_grade = "S460G2+M".to_owned();
        manifest.bridge_edges[2].changed_factors = manifest.domains[2]
            .signature
            .changed_factors(&manifest.domains[3].signature);
        let result = evaluate_transfer(&manifest);
        assert_eq!(result.disposition, TransferDisposition::AdmittedWithBridgeChain);
    }

    #[test]
    fn output_scope_promotion_is_rejected() {
        let mut manifest = base_manifest();
        manifest.request.requested_output_scope = "residual_stress_profile".to_owned();
        let result = evaluate_transfer(&manifest);
        assert_eq!(result.disposition, TransferDisposition::AbstainOutputScope);
        assert_eq!(result.reason, Some(TransferReason::OutputScopeMismatch));
    }

    #[test]
    fn model_without_validated_status_is_rejected() {
        let mut manifest = base_manifest();
        manifest.model.validation_status = "calibrated_only".to_owned();
        let result = evaluate_transfer(&manifest);
        assert_eq!(result.disposition, TransferDisposition::AbstainEvidenceInvalid);
        assert_eq!(result.reason, Some(TransferReason::ModelNotValidated));
    }

    #[test]
    fn edge_for_wrong_model_revision_is_rejected() {
        let mut manifest = base_manifest();
        manifest.bridge_edges[1].model_revision = "R2".to_owned();
        let result = evaluate_transfer(&manifest);
        assert_eq!(result.disposition, TransferDisposition::AbstainEvidenceInvalid);
        assert_eq!(result.reason, Some(TransferReason::BridgeEvidenceInvalid));
    }

    #[test]
    fn bridge_edge_with_future_evidence_is_rejected() {
        let mut manifest = base_manifest();
        manifest.bridge_edges[0].validated_at_tick = 101;
        let result = evaluate_transfer(&manifest);
        assert_eq!(result.disposition, TransferDisposition::AbstainEvidenceInvalid);
        assert_eq!(result.reason, Some(TransferReason::BridgeEvidenceFromFuture));
    }

    #[test]
    fn calibration_validation_specimen_reuse_is_rejected() {
        let mut manifest = base_manifest();
        manifest.bridge_edges[0].validation_specimen_ids[0] = "CAL-E01".to_owned();
        let result = evaluate_transfer(&manifest);
        assert_eq!(result.disposition, TransferDisposition::AbstainEvidenceInvalid);
        assert_eq!(result.reason, Some(TransferReason::BridgeSpecimenLeakage));
    }

    #[test]
    fn insufficient_bridge_coverage_is_rejected() {
        let mut manifest = base_manifest();
        manifest.bridge_edges[1].empirical_coverage_permille = 900;
        let result = evaluate_transfer(&manifest);
        assert_eq!(result.disposition, TransferDisposition::AbstainEvidenceInvalid);
        assert_eq!(result.reason, Some(TransferReason::BridgeCoverageInsufficient));
    }

    #[test]
    fn bridge_error_above_declared_tolerance_is_rejected() {
        let mut manifest = base_manifest();
        manifest.bridge_edges[2].maximum_error_milli = 101;
        let result = evaluate_transfer(&manifest);
        assert_eq!(result.reason, Some(TransferReason::BridgeErrorExceedsTolerance));
    }

    #[test]
    fn uncertainty_is_conservatively_accumulated_across_edges() {
        let mut manifest = base_manifest();
        manifest.request.maximum_total_uncertainty_units = 44;
        let result = evaluate_transfer(&manifest);
        assert_eq!(result.disposition, TransferDisposition::AbstainUncertainty);
        assert_eq!(result.reason, Some(TransferReason::UncertaintyLimitExceeded));
    }

    #[test]
    fn edge_changed_factor_list_must_match_signature_delta() {
        let mut manifest = base_manifest();
        manifest.bridge_edges[0].changed_factors.clear();
        let result = evaluate_transfer(&manifest);
        assert_eq!(result.reason, Some(TransferReason::BridgeChangedFactorsMismatch));
    }

    #[test]
    fn edge_evidence_digest_must_be_valid() {
        let mut manifest = base_manifest();
        manifest.bridge_edges[0].evidence_digest_sha256 = "invalid".to_owned();
        let result = evaluate_transfer(&manifest);
        assert_eq!(result.reason, Some(TransferReason::BridgeEvidenceInvalid));
    }

    #[test]
    fn duplicate_edge_identity_is_rejected() {
        let mut manifest = base_manifest();
        manifest.bridge_edges[1].edge_id = "E01".to_owned();
        let result = evaluate_transfer(&manifest);
        assert_eq!(result.reason, Some(TransferReason::DuplicateEdgeId));
    }

    #[test]
    fn duplicate_domain_identity_is_rejected() {
        let mut manifest = base_manifest();
        manifest.domains[1].domain_id = "D0".to_owned();
        let result = evaluate_transfer(&manifest);
        assert_eq!(result.reason, Some(TransferReason::DuplicateDomainId));
    }

    #[test]
    fn invalid_thickness_range_is_rejected() {
        let mut manifest = base_manifest();
        manifest.domains[0].signature.thickness_min_mm = 40;
        manifest.domains[0].signature.thickness_max_mm = 20;
        let result = evaluate_transfer(&manifest);
        assert_eq!(result.reason, Some(TransferReason::InvalidDomainSignature));
    }

    #[test]
    fn multiple_bridge_paths_choose_lowest_conservative_uncertainty() {
        let mut manifest = base_manifest();
        let joint = manifest.domains[1].signature.clone();
        let panel = manifest.domains[2].signature.clone();
        let alt = DomainNode {
            domain_id: "D1B".to_owned(),
            signature: joint.clone(),
        };
        manifest.domains.push(alt);
        let mut alt_edge = edge("E01B", "D0", "D1B", &manifest.domains[0].signature, &joint);
        alt_edge.transfer_uncertainty_units = 1;
        let mut alt_edge2 = edge("E1B2", "D1B", "D2", &joint, &panel);
        alt_edge2.transfer_uncertainty_units = 1;
        manifest.bridge_edges.push(alt_edge);
        manifest.bridge_edges.push(alt_edge2);
        let result = evaluate_transfer(&manifest);
        let path = result.selected_path.expect("one of the validated paths");
        assert!(path.edge_ids.contains(&"E01B".to_owned()));
        assert_eq!(result.disposition, TransferDisposition::AdmittedWithBridgeChain);
        assert_eq!(result.candidate_path_count, 2);
    }

    #[test]
    fn cycle_does_not_create_a_false_transfer_path() {
        let mut manifest = base_manifest();
        manifest.bridge_edges.push(edge(
            "E10",
            "D1",
            "D0",
            &manifest.domains[1].signature,
            &manifest.domains[0].signature,
        ));
        manifest.bridge_edges.retain(|edge| edge.edge_id != "E12" && edge.edge_id != "E23");
        let result = evaluate_transfer(&manifest);
        assert_eq!(result.disposition, TransferDisposition::AbstainOutOfDomain);
    }

    #[test]
    fn bridge_validation_specimen_cannot_be_reused_on_another_edge() {
        let mut manifest = base_manifest();
        manifest.bridge_edges[1].validation_specimen_ids[0] =
            manifest.bridge_edges[0].validation_specimen_ids[0].clone();
        let result = evaluate_transfer(&manifest);
        assert_eq!(result.reason, Some(TransferReason::BridgeSpecimenLeakage));
    }

    #[test]
    fn target_thickness_outside_source_domain_requires_a_bridge() {
        let mut manifest = base_manifest();
        manifest.domains[3].signature = signature(ScaleTier::BlockAssembly, 35, 38);
        manifest.bridge_edges[2].changed_factors = manifest.domains[2]
            .signature
            .changed_factors(&manifest.domains[3].signature);
        let result = evaluate_transfer(&manifest);
        assert_eq!(result.disposition, TransferDisposition::AdmittedWithBridgeChain);
    }

    #[test]
    fn transfer_uncertainty_overflow_is_rejected() {
        let mut manifest = base_manifest();
        manifest.bridge_edges[0].measurement_uncertainty_units = u64::MAX;
        manifest.bridge_edges[0].parameter_uncertainty_units = 1;
        let result = evaluate_transfer(&manifest);
        assert_eq!(result.reason, Some(TransferReason::NumericOverflow));
    }

    #[test]
    fn changed_output_operator_cannot_be_silently_inherited() {
        let mut manifest = base_manifest();
        manifest.domains[3].signature.coordinate_frame = "ship_block_frame".to_owned();
        manifest.bridge_edges[2].changed_factors = manifest.domains[2]
            .signature
            .changed_factors(&manifest.domains[3].signature);
        let result = evaluate_transfer(&manifest);
        assert_eq!(result.disposition, TransferDisposition::AdmittedWithBridgeChain);
    }

    #[test]
    fn each_edge_must_have_independent_validation_specimens() {
        let mut manifest = base_manifest();
        manifest.bridge_edges[0].validation_specimen_ids.clear();
        let result = evaluate_transfer(&manifest);
        assert_eq!(result.reason, Some(TransferReason::BridgeEvidenceInvalid));
    }

    #[test]
    fn non_adjacent_edge_fails_even_when_every_other_field_is_valid() {
        let mut manifest = base_manifest();
        manifest.bridge_edges[1].to_domain_id = "D3".to_owned();
        manifest.bridge_edges[1].changed_factors = manifest.domains[1]
            .signature
            .changed_factors(&manifest.domains[3].signature);
        let result = evaluate_transfer(&manifest);
        assert_eq!(result.reason, Some(TransferReason::BridgeEdgeNotAdjacentScale));
    }
}
