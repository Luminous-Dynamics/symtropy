//! Leakage-resistant manifests for six-axis tactile/localization transfer benchmarks.
//!
//! This module audits transfer claims and target-evidence burden before model
//! scores are compared. It separates target reference images, unlabeled target
//! contacts, target force labels and independent reference operations. A
//! marker-translation/unsupervised adaptation experiment is never relabeled
//! as zero-shot merely because it used no target force labels.
//!
//! Partition identities are supplied by the benchmark owner. This module
//! checks exact disjointness declarations but does not inspect raw media or
//! prove that the identifiers are truthful. Dataset curation and hash
//! attestation remain external responsibilities.

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
pub enum TransferRegime {
    WithinArticle,
    NewArticleSameFamily,
    NewSensorFamily,
    NewHostOrMount,
    NewSkinOrMaterialLot,
    NewReadoutChain,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
pub enum AdaptationMode {
    ZeroShot,
    UnsupervisedTargetUse,
    FewShotTargetCalibration,
    FullTargetCalibration,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
pub enum PartitionRole {
    SourceTraining,
    SourceValidation,
    TargetAdaptation,
    TargetValidation,
    FinalTest,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
pub enum LabelAccess {
    ForceLabelsVisible,
    UnlabeledOnly,
    EvaluatorOnly,
    NoTargetData,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
pub enum IdentityDimension {
    SensorFamily,
    SensorArticle,
    ManufacturingLot,
    SkinMaterialLot,
    HostContext,
    MountContext,
    ReadoutChain,
    ProbeGeometry,
    ContactEpisode,
    Session,
    SurfaceMaterial,
    OperatingCondition,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct PartitionIdentitySet {
    pub dimension: IdentityDimension,
    pub identities: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct BenchmarkPartition {
    pub role: PartitionRole,
    pub label_access: LabelAccess,
    pub sample_count: usize,
    pub identities: Vec<PartitionIdentitySet>,
}

impl BenchmarkPartition {
    fn identities_for(&self, dimension: IdentityDimension) -> Option<&[String]> {
        self.identities
            .iter()
            .find(|set| set.dimension == dimension)
            .map(|set| set.identities.as_slice())
    }

    fn validate(&self) -> Vec<String> {
        let mut errors = Vec::new();
        let mut dimensions = BTreeSet::new();
        for set in &self.identities {
            if !dimensions.insert(set.dimension) {
                errors.push(format!(
                    "partition {:?} duplicates identity dimension {:?}",
                    self.role, set.dimension
                ));
            }
            let mut ids = BTreeSet::new();
            for identity in &set.identities {
                if identity.trim().is_empty() || !ids.insert(identity) {
                    errors.push(format!(
                        "partition {:?} has an empty or duplicate {:?} identity",
                        self.role, set.dimension
                    ));
                }
            }
        }
        if self.sample_count == 0 && !self.identities.iter().all(|set| set.identities.is_empty()) {
            errors.push(format!(
                "empty partition {:?} contains sample identities",
                self.role
            ));
        }
        if self.sample_count > 0
            && self
                .identities_for(IdentityDimension::ContactEpisode)
                .is_none_or(|values| values.is_empty())
        {
            errors.push(format!(
                "nonempty partition {:?} is missing contact-episode identities",
                self.role
            ));
        }
        errors
    }
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct TargetEvidenceBudget {
    pub target_reference_image_count: usize,
    pub target_unlabeled_contact_episode_count: usize,
    pub target_force_labeled_contact_count: usize,
    pub target_external_reference_operation_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct TransferBenchmarkManifest {
    pub manifest_id: String,
    pub protocol_revision: String,
    pub transfer_regime: TransferRegime,
    pub adaptation_mode: AdaptationMode,
    pub target_evidence_budget: TargetEvidenceBudget,
    pub few_shot_max_force_labeled_contacts: usize,
    pub full_calibration_min_force_labeled_contacts: usize,
    /// Dimensions that must be disjoint between the source and target domains.
    pub required_holdout_dimensions: Vec<IdentityDimension>,
    pub partitions: Vec<BenchmarkPartition>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub enum TransferAuditDisposition {
    QualifiedManifest,
    InvalidManifest,
    LeakageDetected,
    AdaptationRegimeMismatch,
    MissingHoldoutDimension,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct IdentityLeakage {
    pub dimension: IdentityDimension,
    pub identity: String,
    pub source_roles: Vec<PartitionRole>,
    pub target_roles: Vec<PartitionRole>,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct TransferAuditReport {
    pub disposition: TransferAuditDisposition,
    pub manifest_id: String,
    pub transfer_regime: TransferRegime,
    pub adaptation_mode: AdaptationMode,
    pub source_sample_count: usize,
    pub target_adaptation_sample_count: usize,
    pub target_validation_sample_count: usize,
    pub final_test_sample_count: usize,
    pub target_evidence_budget: TargetEvidenceBudget,
    pub checked_holdout_dimensions: Vec<IdentityDimension>,
    pub leakage: Vec<IdentityLeakage>,
    pub reasons: Vec<String>,
    pub claim_class: &'static str,
}

const REQUIRED_PARTITION_ROLES: [PartitionRole; 5] = [
    PartitionRole::SourceTraining,
    PartitionRole::SourceValidation,
    PartitionRole::TargetAdaptation,
    PartitionRole::TargetValidation,
    PartitionRole::FinalTest,
];

fn required_regime_dimension(regime: TransferRegime) -> Option<IdentityDimension> {
    match regime {
        TransferRegime::WithinArticle => None,
        TransferRegime::NewArticleSameFamily => Some(IdentityDimension::SensorArticle),
        TransferRegime::NewSensorFamily => Some(IdentityDimension::SensorFamily),
        TransferRegime::NewHostOrMount => None,
        TransferRegime::NewSkinOrMaterialLot => Some(IdentityDimension::SkinMaterialLot),
        TransferRegime::NewReadoutChain => Some(IdentityDimension::ReadoutChain),
    }
}

fn dimensions_required_by_regime(regime: TransferRegime) -> Vec<IdentityDimension> {
    match regime {
        TransferRegime::WithinArticle => Vec::new(),
        TransferRegime::NewArticleSameFamily => vec![IdentityDimension::SensorArticle],
        TransferRegime::NewSensorFamily => vec![IdentityDimension::SensorFamily],
        TransferRegime::NewHostOrMount => vec![
            IdentityDimension::HostContext,
            IdentityDimension::MountContext,
        ],
        TransferRegime::NewSkinOrMaterialLot => vec![IdentityDimension::SkinMaterialLot],
        TransferRegime::NewReadoutChain => vec![IdentityDimension::ReadoutChain],
    }
}

fn partition<'a>(
    manifest: &'a TransferBenchmarkManifest,
    role: PartitionRole,
) -> Option<&'a BenchmarkPartition> {
    manifest.partitions.iter().find(|item| item.role == role)
}

fn validate_adaptation_budget(
    manifest: &TransferBenchmarkManifest,
    target_adaptation: &BenchmarkPartition,
) -> Vec<String> {
    let mut errors = Vec::new();
    let budget = &manifest.target_evidence_budget;
    let target_episode_count = target_adaptation
        .identities_for(IdentityDimension::ContactEpisode)
        .map_or(0, <[String]>::len);

    match manifest.adaptation_mode {
        AdaptationMode::ZeroShot => {
            if budget.target_reference_image_count != 0
                || budget.target_unlabeled_contact_episode_count != 0
                || budget.target_force_labeled_contact_count != 0
                || budget.target_external_reference_operation_count != 0
                || target_adaptation.sample_count != 0
                || target_adaptation.label_access != LabelAccess::NoTargetData
            {
                errors.push(
                    "zero-shot declaration used target images, contacts, labels, references or adaptation samples"
                        .to_owned(),
                );
            }
        }
        AdaptationMode::UnsupervisedTargetUse => {
            if budget.target_force_labeled_contact_count != 0
                || target_adaptation.label_access != LabelAccess::UnlabeledOnly
            {
                errors.push(
                    "unsupervised target use must not expose target force labels".to_owned(),
                );
            }
            if budget.target_reference_image_count == 0
                && budget.target_unlabeled_contact_episode_count == 0
                && budget.target_external_reference_operation_count == 0
            {
                errors.push(
                    "unsupervised target use declares no target evidence or reference operation"
                        .to_owned(),
                );
            }
        }
        AdaptationMode::FewShotTargetCalibration => {
            let labels = budget.target_force_labeled_contact_count;
            if labels == 0
                || labels > manifest.few_shot_max_force_labeled_contacts
                || target_adaptation.label_access != LabelAccess::ForceLabelsVisible
            {
                errors.push(
                    "few-shot adaptation must expose a positive target force-label count within the declared few-shot budget"
                        .to_owned(),
                );
            }
        }
        AdaptationMode::FullTargetCalibration => {
            let labels = budget.target_force_labeled_contact_count;
            if manifest.full_calibration_min_force_labeled_contacts
                <= manifest.few_shot_max_force_labeled_contacts
                || labels < manifest.full_calibration_min_force_labeled_contacts
                || target_adaptation.label_access != LabelAccess::ForceLabelsVisible
            {
                errors.push(
                    "full target calibration does not meet its declared force-label minimum"
                        .to_owned(),
                );
            }
        }
    }

    let declared_target_contacts = budget
        .target_unlabeled_contact_episode_count
        .saturating_add(budget.target_force_labeled_contact_count);
    if declared_target_contacts > target_episode_count {
        errors.push(
            "target evidence budget exceeds the target-adaptation contact-episode identities"
                .to_owned(),
        );
    }
    errors
}

fn source_roles() -> [PartitionRole; 2] {
    [PartitionRole::SourceTraining, PartitionRole::SourceValidation]
}

fn target_roles() -> [PartitionRole; 3] {
    [
        PartitionRole::TargetAdaptation,
        PartitionRole::TargetValidation,
        PartitionRole::FinalTest,
    ]
}

fn collect_role_identities(
    manifest: &TransferBenchmarkManifest,
    roles: &[PartitionRole],
    dimension: IdentityDimension,
) -> BTreeSet<String> {
    roles
        .iter()
        .filter_map(|role| partition(manifest, *role))
        .filter_map(|part| part.identities_for(dimension))
        .flatten()
        .cloned()
        .collect()
}

fn collect_leakage(
    manifest: &TransferBenchmarkManifest,
    dimension: IdentityDimension,
) -> Vec<IdentityLeakage> {
    let source_ids = collect_role_identities(manifest, &source_roles(), dimension);
    let target_ids = collect_role_identities(manifest, &target_roles(), dimension);
    let mut leakage = Vec::new();
    for identity in source_ids.intersection(&target_ids) {
        let source_roles_found: Vec<_> = source_roles()
            .into_iter()
            .filter(|role| {
                partition(manifest, *role)
                    .and_then(|part| part.identities_for(dimension))
                    .is_some_and(|ids| ids.contains(identity))
            })
            .collect();
        let target_roles_found: Vec<_> = target_roles()
            .into_iter()
            .filter(|role| {
                partition(manifest, *role)
                    .and_then(|part| part.identities_for(dimension))
                    .is_some_and(|ids| ids.contains(identity))
            })
            .collect();
        leakage.push(IdentityLeakage {
            dimension,
            identity: identity.clone(),
            source_roles: source_roles_found,
            target_roles: target_roles_found,
        });
    }
    leakage
}

fn collect_cross_partition_leakage(
    manifest: &TransferBenchmarkManifest,
    dimension: IdentityDimension,
    roles: &[PartitionRole],
) -> Vec<IdentityLeakage> {
    let mut seen_by_role: Vec<(PartitionRole, BTreeSet<String>)> = roles
        .iter()
        .filter_map(|role| {
            partition(manifest, *role).map(|part| {
                (
                    *role,
                    part.identities_for(dimension)
                        .unwrap_or(&[])
                        .iter()
                        .cloned()
                        .collect(),
                )
            })
        })
        .collect();
    seen_by_role.sort_by_key(|(role, _)| *role);

    let mut leakage = Vec::new();
    for left_index in 0..seen_by_role.len() {
        for right_index in (left_index + 1)..seen_by_role.len() {
            let (left_role, left_ids) = &seen_by_role[left_index];
            let (right_role, right_ids) = &seen_by_role[right_index];
            for identity in left_ids.intersection(right_ids) {
                leakage.push(IdentityLeakage {
                    dimension,
                    identity: identity.clone(),
                    source_roles: vec![*left_role],
                    target_roles: vec![*right_role],
                });
            }
        }
    }
    leakage
}

/// Audit exact partition identities and transfer evidence burden before any
/// cross-sensor performance numbers are compared.
pub fn audit_transfer_manifest(manifest: &TransferBenchmarkManifest) -> TransferAuditReport {
    let mut reasons = Vec::new();
    if manifest.manifest_id.trim().is_empty() || manifest.protocol_revision.trim().is_empty() {
        reasons.push("manifest ID and protocol revision must be nonempty".to_owned());
    }
    if manifest.full_calibration_min_force_labeled_contacts == 0 {
        reasons.push("full calibration label minimum must be positive".to_owned());
    }

    let mut roles = BTreeSet::new();
    for part in &manifest.partitions {
        if !roles.insert(part.role) {
            reasons.push(format!("duplicate partition role {:?}", part.role));
        }
        reasons.extend(part.validate());
    }
    for required in REQUIRED_PARTITION_ROLES {
        if !roles.contains(&required) {
            reasons.push(format!("required partition role {:?} is missing", required));
        }
    }
    if manifest.partitions.len() != REQUIRED_PARTITION_ROLES.len() {
        reasons.push("manifest must contain exactly five partition roles".to_owned());
    }

    let target_adaptation = partition(manifest, PartitionRole::TargetAdaptation);
    if let Some(target_adaptation) = target_adaptation {
        reasons.extend(validate_adaptation_budget(manifest, target_adaptation));
    }

    let mut holdout_dimensions = BTreeSet::new();
    for dimension in &manifest.required_holdout_dimensions {
        if !holdout_dimensions.insert(*dimension) {
            reasons.push(format!("holdout dimension {:?} is declared more than once", dimension));
        }
    }
    holdout_dimensions.insert(IdentityDimension::ContactEpisode);
    holdout_dimensions.insert(IdentityDimension::Session);

    for dimension in dimensions_required_by_regime(manifest.transfer_regime) {
        if !holdout_dimensions.contains(&dimension) {
            reasons.push(format!(
                "transfer regime {:?} requires holdout dimension {:?}",
                manifest.transfer_regime, dimension
            ));
        }
    }
    if manifest.transfer_regime == TransferRegime::WithinArticle
        && holdout_dimensions.contains(&IdentityDimension::SensorArticle)
    {
        reasons.push(
            "within-article transfer must not be scored as sensor-article-disjoint transfer"
                .to_owned(),
        );
    }

    for dimension in &holdout_dimensions {
        for role in REQUIRED_PARTITION_ROLES {
            if let Some(part) = partition(manifest, role) {
                if part.sample_count > 0
                    && part
                        .identities_for(*dimension)
                        .is_none_or(|identities| identities.is_empty())
                {
                    reasons.push(format!(
                        "nonempty partition {:?} is missing required holdout identities for {:?}",
                        role, dimension
                    ));
                }
            }
        }
    }

    let mut leakage = Vec::new();
    for dimension in &holdout_dimensions {
        leakage.extend(collect_leakage(manifest, *dimension));
    }
    leakage.extend(collect_cross_partition_leakage(
        manifest,
        IdentityDimension::ContactEpisode,
        &REQUIRED_PARTITION_ROLES,
    ));
    leakage.extend(collect_cross_partition_leakage(
        manifest,
        IdentityDimension::Session,
        &REQUIRED_PARTITION_ROLES,
    ));

    let source_sample_count = source_roles()
        .iter()
        .filter_map(|role| partition(manifest, *role))
        .map(|part| part.sample_count)
        .sum();
    let target_adaptation_sample_count = partition(manifest, PartitionRole::TargetAdaptation)
        .map_or(0, |part| part.sample_count);
    let target_validation_sample_count = partition(manifest, PartitionRole::TargetValidation)
        .map_or(0, |part| part.sample_count);
    let final_test_sample_count = partition(manifest, PartitionRole::FinalTest)
        .map_or(0, |part| part.sample_count);

    let disposition = if !leakage.is_empty() {
        TransferAuditDisposition::LeakageDetected
    } else if reasons.iter().any(|reason| {
        reason.contains("zero-shot")
            || reason.contains("unsupervised")
            || reason.contains("few-shot")
            || reason.contains("full target calibration")
    }) {
        TransferAuditDisposition::AdaptationRegimeMismatch
    } else if reasons.iter().any(|reason| reason.contains("holdout")) {
        TransferAuditDisposition::MissingHoldoutDimension
    } else if !reasons.is_empty() {
        TransferAuditDisposition::InvalidManifest
    } else {
        TransferAuditDisposition::QualifiedManifest
    };

    TransferAuditReport {
        disposition,
        manifest_id: manifest.manifest_id.clone(),
        transfer_regime: manifest.transfer_regime,
        adaptation_mode: manifest.adaptation_mode,
        source_sample_count,
        target_adaptation_sample_count,
        target_validation_sample_count,
        final_test_sample_count,
        target_evidence_budget: manifest.target_evidence_budget.clone(),
        checked_holdout_dimensions: holdout_dimensions.into_iter().collect(),
        leakage,
        reasons,
        claim_class: "benchmark_partition_and_transfer_regime_audit_only",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identities(
        dimension: IdentityDimension,
        values: &[&str],
    ) -> PartitionIdentitySet {
        PartitionIdentitySet {
            dimension,
            identities: values.iter().map(|value| (*value).to_owned()).collect(),
        }
    }

    fn zero_shot_manifest() -> TransferBenchmarkManifest {
        let source = vec![
            identities(IdentityDimension::SensorFamily, &["family-source"]),
            identities(IdentityDimension::SensorArticle, &["article-source"]),
            identities(IdentityDimension::ContactEpisode, &["episode-source-train"]),
            identities(IdentityDimension::Session, &["session-source-train"]),
            identities(IdentityDimension::ProbeGeometry, &["probe-source"]),
        ];
        let source_validation = vec![
            identities(IdentityDimension::SensorFamily, &["family-source"]),
            identities(IdentityDimension::SensorArticle, &["article-source"]),
            identities(IdentityDimension::ContactEpisode, &["episode-source-val"]),
            identities(IdentityDimension::Session, &["session-source-val"]),
            identities(IdentityDimension::ProbeGeometry, &["probe-source-val"]),
        ];
        let target = vec![
            identities(IdentityDimension::SensorFamily, &["family-target"]),
            identities(IdentityDimension::SensorArticle, &["article-target"]),
            identities(IdentityDimension::ContactEpisode, &["episode-target-adapt"]),
            identities(IdentityDimension::Session, &["session-target-adapt"]),
            identities(IdentityDimension::ProbeGeometry, &["probe-target"]),
        ];
        let target_validation = vec![
            identities(IdentityDimension::SensorFamily, &["family-target"]),
            identities(IdentityDimension::SensorArticle, &["article-target"]),
            identities(IdentityDimension::ContactEpisode, &["episode-target-val"]),
            identities(IdentityDimension::Session, &["session-target-val"]),
            identities(IdentityDimension::ProbeGeometry, &["probe-target"]),
        ];
        let final_test = vec![
            identities(IdentityDimension::SensorFamily, &["family-target"]),
            identities(IdentityDimension::SensorArticle, &["article-target"]),
            identities(IdentityDimension::ContactEpisode, &["episode-target-test"]),
            identities(IdentityDimension::Session, &["session-target-test"]),
            identities(IdentityDimension::ProbeGeometry, &["probe-target"]),
        ];

        TransferBenchmarkManifest {
            manifest_id: "manifest:hexa-touch-transfer-v0.1".to_owned(),
            protocol_revision: "phri-tac-multiaxis-transfer-v0.1".to_owned(),
            transfer_regime: TransferRegime::NewSensorFamily,
            adaptation_mode: AdaptationMode::ZeroShot,
            target_evidence_budget: TargetEvidenceBudget {
                target_reference_image_count: 0,
                target_unlabeled_contact_episode_count: 0,
                target_force_labeled_contact_count: 0,
                target_external_reference_operation_count: 0,
            },
            few_shot_max_force_labeled_contacts: 8,
            full_calibration_min_force_labeled_contacts: 32,
            required_holdout_dimensions: vec![
                IdentityDimension::SensorFamily,
                IdentityDimension::SensorArticle,
                IdentityDimension::ProbeGeometry,
            ],
            partitions: vec![
                BenchmarkPartition {
                    role: PartitionRole::SourceTraining,
                    label_access: LabelAccess::ForceLabelsVisible,
                    sample_count: 100,
                    identities: source,
                },
                BenchmarkPartition {
                    role: PartitionRole::SourceValidation,
                    label_access: LabelAccess::EvaluatorOnly,
                    sample_count: 20,
                    identities: source_validation,
                },
                BenchmarkPartition {
                    role: PartitionRole::TargetAdaptation,
                    label_access: LabelAccess::NoTargetData,
                    sample_count: 0,
                    identities: Vec::new(),
                },
                BenchmarkPartition {
                    role: PartitionRole::TargetValidation,
                    label_access: LabelAccess::EvaluatorOnly,
                    sample_count: 20,
                    identities: target_validation,
                },
                BenchmarkPartition {
                    role: PartitionRole::FinalTest,
                    label_access: LabelAccess::EvaluatorOnly,
                    sample_count: 40,
                    identities: final_test,
                },
            ],
        }
    }

    #[test]
    fn clean_zero_shot_cross_family_manifest_is_admitted() {
        let report = audit_transfer_manifest(&zero_shot_manifest());
        assert_eq!(report.disposition, TransferAuditDisposition::QualifiedManifest);
        assert_eq!(report.source_sample_count, 120);
        assert_eq!(report.final_test_sample_count, 40);
        assert!(report.leakage.is_empty());
    }

    #[test]
    fn zero_shot_cannot_hide_target_reference_images_or_unlabeled_contacts() {
        let mut manifest = zero_shot_manifest();
        manifest.target_evidence_budget.target_reference_image_count = 4;
        let report = audit_transfer_manifest(&manifest);
        assert_eq!(report.disposition, TransferAuditDisposition::AdaptationRegimeMismatch);
        assert!(report.reasons.iter().any(|reason| reason.contains("zero-shot")));
    }

    #[test]
    fn marker_translation_is_unlabelled_target_adaptation_not_zero_shot() {
        let mut manifest = zero_shot_manifest();
        manifest.adaptation_mode = AdaptationMode::UnsupervisedTargetUse;
        manifest.target_evidence_budget.target_reference_image_count = 4;
        manifest.target_evidence_budget.target_unlabeled_contact_episode_count = 2;
        manifest.partitions[2].sample_count = 2;
        manifest.partitions[2].label_access = LabelAccess::UnlabeledOnly;
        manifest.partitions[2].identities = vec![
            identities(IdentityDimension::ContactEpisode, &["episode-target-adapt"]),
            identities(IdentityDimension::Session, &["session-target-adapt"]),
        ];
        assert_eq!(
            audit_transfer_manifest(&manifest).disposition,
            TransferAuditDisposition::QualifiedManifest
        );
    }

    #[test]
    fn few_shot_and_full_calibration_are_distinct_budget_classes() {
        let mut few_shot = zero_shot_manifest();
        few_shot.adaptation_mode = AdaptationMode::FewShotTargetCalibration;
        few_shot.target_evidence_budget.target_force_labeled_contact_count = 4;
        few_shot.partitions[2].sample_count = 4;
        few_shot.partitions[2].label_access = LabelAccess::ForceLabelsVisible;
        few_shot.partitions[2].identities = vec![
            identities(IdentityDimension::ContactEpisode, &["episode-target-adapt"]),
            identities(IdentityDimension::Session, &["session-target-adapt"]),
        ];
        assert_eq!(
            audit_transfer_manifest(&few_shot).disposition,
            TransferAuditDisposition::QualifiedManifest
        );

        let mut over_budget = few_shot.clone();
        over_budget.target_evidence_budget.target_force_labeled_contact_count = 9;
        over_budget.partitions[2].sample_count = 9;
        assert_eq!(
            audit_transfer_manifest(&over_budget).disposition,
            TransferAuditDisposition::AdaptationRegimeMismatch
        );

        let mut full = zero_shot_manifest();
        full.adaptation_mode = AdaptationMode::FullTargetCalibration;
        full.target_evidence_budget.target_force_labeled_contact_count = 32;
        full.partitions[2].sample_count = 32;
        full.partitions[2].label_access = LabelAccess::ForceLabelsVisible;
        full.partitions[2].identities = vec![
            identities(IdentityDimension::ContactEpisode, &["episode-target-adapt"]),
            identities(IdentityDimension::Session, &["session-target-adapt"]),
        ];
        assert_eq!(
            audit_transfer_manifest(&full).disposition,
            TransferAuditDisposition::QualifiedManifest
        );
    }

    #[test]
    fn probe_and_contact_episode_overlap_are_detected_as_leakage() {
        let mut manifest = zero_shot_manifest();
        manifest.partitions[4].identities[2].identities =
            vec!["episode-source-train".to_owned()];
        manifest.partitions[4].identities[4].identities =
            vec!["probe-source".to_owned()];
        let report = audit_transfer_manifest(&manifest);
        assert_eq!(report.disposition, TransferAuditDisposition::LeakageDetected);
        assert!(report.leakage.iter().any(|item| {
            item.dimension == IdentityDimension::ContactEpisode
        }));
        assert!(report.leakage.iter().any(|item| {
            item.dimension == IdentityDimension::ProbeGeometry
        }));
    }

    #[test]
    fn held_out_host_and_skin_or_readout_regimes_require_the_right_dimensions() {
        let mut host = zero_shot_manifest();
        host.transfer_regime = TransferRegime::NewHostOrMount;
        host.required_holdout_dimensions = vec![IdentityDimension::HostContext];
        assert_eq!(
            audit_transfer_manifest(&host).disposition,
            TransferAuditDisposition::MissingHoldoutDimension
        );

        let mut skin = zero_shot_manifest();
        skin.transfer_regime = TransferRegime::NewSkinOrMaterialLot;
        skin.required_holdout_dimensions = vec![IdentityDimension::SkinMaterialLot];
        assert_eq!(
            audit_transfer_manifest(&skin).disposition,
            TransferAuditDisposition::QualifiedManifest
        );

        let mut readout = zero_shot_manifest();
        readout.transfer_regime = TransferRegime::NewReadoutChain;
        readout.required_holdout_dimensions = vec![IdentityDimension::ReadoutChain];
        assert_eq!(
            audit_transfer_manifest(&readout).disposition,
            TransferAuditDisposition::MissingHoldoutDimension
        );
    }

    #[test]
    fn final_test_labels_must_remain_evaluator_only() {
        let mut manifest = zero_shot_manifest();
        manifest.partitions[4].label_access = LabelAccess::ForceLabelsVisible;
        let report = audit_transfer_manifest(&manifest);
        assert_eq!(report.disposition, TransferAuditDisposition::InvalidManifest);
    }
}
