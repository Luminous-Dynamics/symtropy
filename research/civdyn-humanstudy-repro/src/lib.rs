// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

#![forbid(unsafe_code)]

//! Source-pinned HumanStudy-Bench receipt validation.
//!
//! This crate validates normalized, typed receipts. It deliberately does not
//! reimplement HumanStudy-Bench's Python study runner, study-specific tests,
//! PAS, or ECS. Those remain the upstream reproduction authority.

use std::collections::BTreeSet;

pub const PROTOCOL_ID: &str = "HSB-COMMUNITY-16-2026-11-07";
pub const SOURCE_REPOSITORY: &str = "HumanStudy-Hub/HumanStudy-Bench";
pub const SOURCE_COMMIT: &str = "b34d214b980be13ec5944a4d570a38bc4047b0f9";
pub const SOURCE_ROOT_TREE: &str = "26851c1d6564c78b25600afc7de8ec435e4aa3b8";

pub const PAPER_12_PROTOCOL_ID: &str = "HSB-PAPER-12-CF05D330";
pub const PAPER_12_SOURCE_REPOSITORY: &str = "AISmithLab/HumanStudy-Bench";
pub const PAPER_12_SOURCE_COMMIT: &str = "cf05d330fc315168c8c8e23b4acd882f91bb7408";
pub const PAPER_12_SOURCE_ROOT_TREE: &str = "9b56c62406fb6b77d87d5032c3c6906583aca24e";

/// Exact data/studies/study_NNN Git tree identities at the original paper repository commit.
pub const PAPER_12_STUDY_PINS: [StudyPin; 12] = [
    StudyPin {
        study_id: "study_001",
        tree_sha: "ada9bcfe467e82acd230f08b4970e5eff3fff7f9",
    },
    StudyPin {
        study_id: "study_002",
        tree_sha: "f20390e427b48e081c07273c70fda897e04a16bf",
    },
    StudyPin {
        study_id: "study_003",
        tree_sha: "bbdba6bd7f4a71270a13d889bfb71c51d633398b",
    },
    StudyPin {
        study_id: "study_004",
        tree_sha: "1af52b05645a9e9f9936e2c458ad8460a521c934",
    },
    StudyPin {
        study_id: "study_005",
        tree_sha: "ed29e5fe8711402977e7bb7385a6f939f1ed4bfd",
    },
    StudyPin {
        study_id: "study_006",
        tree_sha: "739255d6b6746ba9e93360d07f12cf631ad7d8d2",
    },
    StudyPin {
        study_id: "study_007",
        tree_sha: "480c0468a8e3f588dfb67a827f6320581847132a",
    },
    StudyPin {
        study_id: "study_008",
        tree_sha: "15d397ca8e7781ed6a3efcfdbb0c6eacf825964f",
    },
    StudyPin {
        study_id: "study_009",
        tree_sha: "ba78aafc24a997eeb6957a2607a9ee8475b2c45f",
    },
    StudyPin {
        study_id: "study_010",
        tree_sha: "1e49238a66b4da4fc4f6bfb44cb628c39aa1b7b8",
    },
    StudyPin {
        study_id: "study_011",
        tree_sha: "aa5858deda592a67e391d271c1fc3c64d8d9362e",
    },
    StudyPin {
        study_id: "study_012",
        tree_sha: "1a4979759d78205621080f7fb7a261c132051886",
    },
];

#[derive(Clone, Copy)]
struct PinnedProtocol {
    source_repository: &'static str,
    source_commit: &'static str,
    source_root_tree: &'static str,
    study_pins: &'static [StudyPin],
}

fn pinned_protocol(protocol_id: &str) -> Option<PinnedProtocol> {
    match protocol_id {
        PROTOCOL_ID => Some(PinnedProtocol {
            source_repository: SOURCE_REPOSITORY,
            source_commit: SOURCE_COMMIT,
            source_root_tree: SOURCE_ROOT_TREE,
            study_pins: &STUDY_PINS,
        }),
        PAPER_12_PROTOCOL_ID => Some(PinnedProtocol {
            source_repository: PAPER_12_SOURCE_REPOSITORY,
            source_commit: PAPER_12_SOURCE_COMMIT,
            source_root_tree: PAPER_12_SOURCE_ROOT_TREE,
            study_pins: &PAPER_12_STUDY_PINS,
        }),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StudyPin {
    pub study_id: &'static str,
    pub tree_sha: &'static str,
}

/// Exact study-directory Git tree identities at SOURCE_COMMIT.
/// A change to any tree requires a new protocol profile and a new run receipt.
pub const STUDY_PINS: [StudyPin; 16] = [
    StudyPin {
        study_id: "study_001",
        tree_sha: "41965042b7fbf13a98e35756348e54c83cf9af9d",
    },
    StudyPin {
        study_id: "study_002",
        tree_sha: "cac39c3746fcf03a861a9b6f115e807d68f0e434",
    },
    StudyPin {
        study_id: "study_003",
        tree_sha: "fd8fa2a6dcc41e8aaf598f8b4924c1d32f76cbe6",
    },
    StudyPin {
        study_id: "study_004",
        tree_sha: "42e8f65c7117f6ee6682266a5952a8d81412ea42",
    },
    StudyPin {
        study_id: "study_005",
        tree_sha: "8558bae1cb9408af82db5b5538cf984a202e6ab1",
    },
    StudyPin {
        study_id: "study_006",
        tree_sha: "ec96290930f8b4855e39139cfe97ed60b474b463",
    },
    StudyPin {
        study_id: "study_007",
        tree_sha: "a09ef7a25fa17bd5cf49a0ec026e55630167db8f",
    },
    StudyPin {
        study_id: "study_008",
        tree_sha: "db042e02734d4726ee86446298f53fa967422963",
    },
    StudyPin {
        study_id: "study_009",
        tree_sha: "ef9d0c2b41887ec65ef8316187a4df7aceada579",
    },
    StudyPin {
        study_id: "study_010",
        tree_sha: "e5014dc6039572a0bda6e00f4047456f4f83da1f",
    },
    StudyPin {
        study_id: "study_011",
        tree_sha: "6b0946acb826a0660f158a157a79824bf531b3b2",
    },
    StudyPin {
        study_id: "study_012",
        tree_sha: "a50ddb86d2a5727168b2b55636415ecc3fa1e21c",
    },
    StudyPin {
        study_id: "study_013",
        tree_sha: "693b00924e2f67ad034c9a66136b21871148465d",
    },
    StudyPin {
        study_id: "study_014",
        tree_sha: "b3abae7b4df6b4631f4280b773394c72c6254bb4",
    },
    StudyPin {
        study_id: "study_015",
        tree_sha: "65056a93e9f7fd6fd5789097b289468ac781047d",
    },
    StudyPin {
        study_id: "study_016",
        tree_sha: "823c0ae46633a04919fae3cd74809ad45c6cb9c2",
    },
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IndependentUnit {
    Participant,
    Group,
    Session,
    Cluster,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EffectEstimate {
    pub finding_id: String,
    pub test_id: String,
    pub measure: String,
    pub unit: String,
    pub human_estimate: f64,
    pub simulated_estimate: f64,
    pub human_ci95: Option<[f64; 2]>,
    pub simulated_ci95: Option<[f64; 2]>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArtifactReceipt {
    pub role: String,
    /// SHA-256 of exact artifact bytes, not a digest of a reconstructed object.
    pub sha256: String,
    pub record_count: u64,
    pub contains_record_level_human_data: bool,
    pub published_in_repository: bool,
    pub redistribution_authorized: bool,
    /// Human-readable policy, licence, consent, or access basis.
    pub rights_basis: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StudyReceipt {
    pub study_id: String,
    pub study_tree_sha: String,
    pub participant_count: u64,
    pub trial_count: u64,
    pub independent_unit: IndependentUnit,
    pub analysis_pipeline_sha256: String,
    pub agent_config_sha256: String,
    pub prompt_template_sha256: String,
    pub analysis_protocol_preserved: bool,
    /// Official upstream PAS. This crate validates its scale; it does not recalculate it.
    pub official_pas: f64,
    /// Official upstream ECS. This crate validates its scale; it does not recalculate it.
    pub official_ecs: f64,
    pub effects: Vec<EffectEstimate>,
    pub artifacts: Vec<ArtifactReceipt>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BenchmarkRunReceipt {
    pub protocol_id: String,
    pub source_repository: String,
    pub source_commit: String,
    pub source_root_tree: String,
    pub run_id: String,
    pub model_id: String,
    pub studies: Vec<StudyReceipt>,
}

fn is_lower_hex(value: &str, expected_len: usize) -> bool {
    value.len() == expected_len
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn check_ci(label: &str, ci: Option<[f64; 2]>, errors: &mut Vec<String>) {
    if let Some([lower, upper]) = ci
        && (!lower.is_finite() || !upper.is_finite() || lower > upper)
    {
        errors.push(format!("{label}: invalid confidence interval"));
    }
}

fn validate_study(study: &StudyReceipt, errors: &mut Vec<String>) {
    if study.participant_count == 0 || study.trial_count == 0 {
        errors.push(format!(
            "{}: participant and trial counts must be positive",
            study.study_id
        ));
    }
    if !is_lower_hex(&study.study_tree_sha, 40) {
        errors.push(format!(
            "{}: study_tree_sha must be a 40-character lowercase Git object id",
            study.study_id
        ));
    }
    for (name, digest) in [
        ("analysis_pipeline_sha256", &study.analysis_pipeline_sha256),
        ("agent_config_sha256", &study.agent_config_sha256),
        ("prompt_template_sha256", &study.prompt_template_sha256),
    ] {
        if !is_lower_hex(digest, 64) {
            errors.push(format!(
                "{}: {name} must be a 64-character lowercase SHA-256",
                study.study_id
            ));
        }
    }
    if !study.analysis_protocol_preserved {
        errors.push(format!(
            "{}: modified or substituted analysis is not an exact reproduction",
            study.study_id
        ));
    }
    if !study.official_pas.is_finite() || !(0.0..=1.0).contains(&study.official_pas) {
        errors.push(format!(
            "{}: PAS must be finite and in [0, 1]",
            study.study_id
        ));
    }
    if !study.official_ecs.is_finite() || !(-1.0..=1.0).contains(&study.official_ecs) {
        errors.push(format!(
            "{}: ECS must be finite and in [-1, 1]",
            study.study_id
        ));
    }

    let required_roles = [
        "human_reference_aggregate",
        "simulated_responses",
        "official_analysis_output",
        "official_score_output",
    ];
    let mut roles = BTreeSet::new();
    for artifact in &study.artifacts {
        if artifact.role.trim().is_empty() || !roles.insert(artifact.role.as_str()) {
            errors.push(format!(
                "{}: artifact roles must be non-empty and unique",
                study.study_id
            ));
        }
        if !is_lower_hex(&artifact.sha256, 64) {
            errors.push(format!(
                "{}: artifact {} has an invalid SHA-256",
                study.study_id, artifact.role
            ));
        }
        if artifact.rights_basis.trim().is_empty() {
            errors.push(format!(
                "{}: artifact {} is missing its rights basis",
                study.study_id, artifact.role
            ));
        }
        if artifact.contains_record_level_human_data
            && artifact.published_in_repository
            && !artifact.redistribution_authorized
        {
            errors.push(format!(
                "{}: restricted record-level human data cannot be published",
                study.study_id
            ));
        }
        if artifact.role == "human_reference_aggregate" && artifact.contains_record_level_human_data
        {
            errors.push(format!(
                "{}: human_reference_aggregate must not contain record-level human data",
                study.study_id
            ));
        }
    }
    for role in required_roles {
        if !roles.contains(role) {
            errors.push(format!(
                "{}: missing required artifact role {role}",
                study.study_id
            ));
        }
    }

    let mut finding_tests = BTreeSet::new();
    for effect in &study.effects {
        if effect.finding_id.trim().is_empty()
            || effect.test_id.trim().is_empty()
            || effect.measure.trim().is_empty()
            || effect.unit.trim().is_empty()
        {
            errors.push(format!(
                "{}: effect records require finding, test, measure and unit identities",
                study.study_id
            ));
        }
        if !finding_tests.insert((&effect.finding_id, &effect.test_id)) {
            errors.push(format!(
                "{}: duplicate effect identity {}/{}",
                study.study_id, effect.finding_id, effect.test_id
            ));
        }
        if !effect.human_estimate.is_finite() || !effect.simulated_estimate.is_finite() {
            errors.push(format!(
                "{}: effect estimates must be finite",
                study.study_id
            ));
        }
        check_ci("human_ci95", effect.human_ci95, errors);
        check_ci("simulated_ci95", effect.simulated_ci95, errors);
    }
}

/// Validate one complete, exact-protocol benchmark run against the pinned
/// community-edition snapshot. This is a conformance gate, not a scorer.
pub fn validate_run(receipt: &BenchmarkRunReceipt) -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    let Some(profile) = pinned_protocol(&receipt.protocol_id) else {
        return Err(vec![
            "protocol_id does not match a pinned protocol profile".to_string(),
        ]);
    };

    if receipt.source_repository != profile.source_repository {
        errors.push("source_repository does not match the selected pinned protocol".to_string());
    }
    if receipt.source_commit != profile.source_commit {
        errors.push("source_commit does not match the selected pinned protocol".to_string());
    }
    if receipt.source_root_tree != profile.source_root_tree {
        errors.push("source_root_tree does not match the selected pinned protocol".to_string());
    }
    if receipt.run_id.trim().is_empty() || receipt.model_id.trim().is_empty() {
        errors.push("run_id and model_id must be non-empty".to_string());
    }
    if receipt.studies.len() != profile.study_pins.len() {
        errors.push(format!(
            "expected exactly {} study receipts, got {}",
            profile.study_pins.len(),
            receipt.studies.len()
        ));
    }

    let mut seen = BTreeSet::new();
    for study in &receipt.studies {
        if !seen.insert(study.study_id.as_str()) {
            errors.push(format!("duplicate study receipt: {}", study.study_id));
        }
        match profile
            .study_pins
            .iter()
            .find(|pin| pin.study_id == study.study_id)
        {
            Some(pin) if pin.tree_sha == study.study_tree_sha => {}
            Some(_) => errors.push(format!(
                "{}: study tree does not match the selected pinned source",
                study.study_id
            )),
            None => errors.push(format!(
                "{}: study is not part of the selected {}-study profile",
                study.study_id,
                profile.study_pins.len()
            )),
        }
        validate_study(study, &mut errors);
    }
    for pin in profile.study_pins {
        if !seen.contains(pin.study_id) {
            errors.push(format!("missing study receipt: {}", pin.study_id));
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    fn artifact(
        role: &str,
        record_level: bool,
        published: bool,
        authorized: bool,
    ) -> ArtifactReceipt {
        ArtifactReceipt {
            role: role.to_string(),
            sha256: "a".repeat(64),
            record_count: 120,
            contains_record_level_human_data: record_level,
            published_in_repository: published,
            redistribution_authorized: authorized,
            rights_basis: "synthetic conformance fixture".to_string(),
        }
    }

    fn study(pin: StudyPin) -> StudyReceipt {
        StudyReceipt {
            study_id: pin.study_id.to_string(),
            study_tree_sha: pin.tree_sha.to_string(),
            participant_count: 100,
            trial_count: 120,
            independent_unit: IndependentUnit::Participant,
            analysis_pipeline_sha256: "b".repeat(64),
            agent_config_sha256: "c".repeat(64),
            prompt_template_sha256: "d".repeat(64),
            analysis_protocol_preserved: true,
            official_pas: 0.5,
            official_ecs: 0.0,
            effects: vec![EffectEstimate {
                finding_id: "finding_1".to_string(),
                test_id: "test_1".to_string(),
                measure: "standardized_mean_difference".to_string(),
                unit: "standard_deviations".to_string(),
                human_estimate: 0.4,
                simulated_estimate: 0.38,
                human_ci95: Some([0.1, 0.7]),
                simulated_ci95: Some([0.08, 0.68]),
            }],
            artifacts: vec![
                artifact("human_reference_aggregate", false, true, true),
                artifact("simulated_responses", false, true, true),
                artifact("official_analysis_output", false, true, true),
                artifact("official_score_output", false, true, true),
            ],
        }
    }

    fn valid_run() -> BenchmarkRunReceipt {
        BenchmarkRunReceipt {
            protocol_id: PROTOCOL_ID.to_string(),
            source_repository: SOURCE_REPOSITORY.to_string(),
            source_commit: SOURCE_COMMIT.to_string(),
            source_root_tree: SOURCE_ROOT_TREE.to_string(),
            run_id: "synthetic-conformance-only".to_string(),
            model_id: "synthetic-reference-not-a-model".to_string(),
            studies: STUDY_PINS.iter().copied().map(study).collect(),
        }
    }

    #[test]
    fn pinned_inventory_contains_sixteen_unique_studies() {
        let ids: BTreeSet<_> = STUDY_PINS.iter().map(|pin| pin.study_id).collect();
        assert_eq!(STUDY_PINS.len(), 16);
        assert_eq!(ids.len(), 16);
    }

    #[test]
    fn paper_inventory_contains_twelve_unique_studies() {
        let ids: BTreeSet<_> = PAPER_12_STUDY_PINS.iter().map(|pin| pin.study_id).collect();
        assert_eq!(PAPER_12_STUDY_PINS.len(), 12);
        assert_eq!(ids.len(), 12);
    }

    #[test]
    fn paper_twelve_profile_passes_its_exact_receipt() {
        let receipt = BenchmarkRunReceipt {
            protocol_id: PAPER_12_PROTOCOL_ID.to_string(),
            source_repository: PAPER_12_SOURCE_REPOSITORY.to_string(),
            source_commit: PAPER_12_SOURCE_COMMIT.to_string(),
            source_root_tree: PAPER_12_SOURCE_ROOT_TREE.to_string(),
            run_id: "synthetic-paper-12-conformance".to_string(),
            model_id: "synthetic-reference-not-a-model".to_string(),
            studies: PAPER_12_STUDY_PINS.iter().copied().map(study).collect(),
        };
        assert!(validate_run(&receipt).is_ok());
    }

    #[test]
    fn community_sixteen_profile_rejects_paper_only_receipt() {
        let mut receipt = valid_run();
        receipt.studies = PAPER_12_STUDY_PINS.iter().copied().map(study).collect();
        assert!(validate_run(&receipt).is_err());
    }

    #[test]
    fn synthetic_complete_receipt_passes_structure() {
        assert!(validate_run(&valid_run()).is_ok());
    }

    #[test]
    fn missing_study_fails_closed() {
        let mut receipt = valid_run();
        receipt.studies.pop();
        assert!(validate_run(&receipt).is_err());
    }

    #[test]
    fn changed_study_tree_fails_closed() {
        let mut receipt = valid_run();
        receipt.studies[0].study_tree_sha = "0".repeat(40);
        assert!(validate_run(&receipt).is_err());
    }

    #[test]
    fn score_outside_upstream_scale_fails_closed() {
        let mut receipt = valid_run();
        receipt.studies[0].official_pas = 1.01;
        assert!(validate_run(&receipt).is_err());
    }

    #[test]
    fn non_authorized_record_level_export_fails_closed() {
        let mut receipt = valid_run();
        receipt.studies[0].artifacts.push(artifact(
            "restricted_raw_data_reference",
            true,
            true,
            false,
        ));
        assert!(validate_run(&receipt).is_err());
    }

    #[test]
    fn changed_analysis_pipeline_fails_exact_reproduction() {
        let mut receipt = valid_run();
        receipt.studies[0].analysis_protocol_preserved = false;
        assert!(validate_run(&receipt).is_err());
    }
}
