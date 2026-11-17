//! Paired, whole-workflow construction productivity scoring.
//!
//! This module compares independently accepted output from matched manual and
//! robotic workflows. The statistical unit is a matched work-package block;
//! uncertainty intervals resample independent project/lab clusters rather
//! than treating every printed entity as an independent trial.
//!
//! This is a benchmark analysis core, not a causal-inference engine. The
//! evaluator must freeze randomization, matching, exclusions and practical
//! significance thresholds before field data collection.

use std::collections::BTreeMap;

use crate::field_layout_acceptance::{CrewHours, FieldAcceptanceScore};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WorkflowArm {
    Manual,
    Robotic,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FieldStudyDesign {
    RandomizedWithinProject,
    SteppedWedge,
    MatchedObservational,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkflowRunIdentity {
    pub run_id: String,
    /// Independent project/lab cluster. Bootstrap resampling is performed at
    /// this level, never at individual line/point entity level.
    pub cluster_id: String,
    pub block_id: String,
    pub site_id: String,
    pub task_family: String,
    pub pairing_sha256: String,
    pub environment_profile_sha256: String,
    pub randomization_receipt_sha256: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WorkflowResourceObservation {
    /// Complete elapsed wall-clock duration from the preregistered start event
    /// to the preregistered completion/acceptance cutoff, in hours.
    pub elapsed_wall_hours: f64,
    /// Metered energy over the complete workflow, in kWh.
    pub energy_kwh: f64,
    /// Directly observed high-strain worker-minutes; do not infer from robot
    /// utilization or time away from the workface.
    pub high_strain_worker_minutes: f64,
    pub intervention_count: u64,
    pub safety_event_count: u64,
    pub severe_incident_count: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FieldWorkflowArmRun {
    pub identity: WorkflowRunIdentity,
    pub arm: WorkflowArm,
    pub acceptance: FieldAcceptanceScore,
    pub resources: WorkflowResourceObservation,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MatchedWorkflowBlock {
    pub study_design: FieldStudyDesign,
    pub manual: FieldWorkflowArmRun,
    pub robotic: FieldWorkflowArmRun,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FieldWorkflowStudyProfile {
    /// Allowed reduction in accepted-output rate for the robot arm relative
    /// to manual, expressed as an absolute fraction (e.g. 0.005 = 0.5 pp).
    pub quality_noninferiority_margin: f64,
    /// Number of project/lab clusters needed before a transferability
    /// estimate can be labelled ready for cross-cluster inference.
    pub minimum_independent_clusters: usize,
    /// Deterministic cluster bootstrap replicate count.
    pub bootstrap_resamples: usize,
    pub bootstrap_seed: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FieldWorkflowScoreError {
    EmptyRunIdentity,
    InvalidPairingDigest,
    InvalidEnvironmentDigest,
    MissingRandomizationReceipt,
    UnexpectedRandomizationReceipt,
    MixedStudyDesigns,
    DuplicateBlock(String),
    ArmOrderInvalid(String),
    PairIdentityMismatch(String),
    WorkPackageMismatch(String),
    InvalidAcceptanceAccounting(String),
    InvalidCrewHourAccounting(String),
    InvalidResourceObservation(String),
    InvalidStudyProfile,
    NoMatchedBlocks,
    NoProductiveManualBaseline(String),
    NoProductiveRoboticRun(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct MatchedBlockScore {
    pub cluster_id: String,
    pub block_id: String,
    pub task_family: String,
    pub design_entity_count: u64,
    pub manual_accepted_count: u64,
    pub robotic_accepted_count: u64,
    pub manual_acceptance_rate: f64,
    pub robotic_acceptance_rate: f64,
    pub accepted_output_rate_difference: f64,
    /// Ratio >1 means more independently accepted output per complete
    /// workflow crew-hour in the robotic arm.
    pub accepted_output_per_crew_hour_ratio: f64,
    /// Ratio <1 means fewer total crew-hours per accepted entity in robotics.
    pub crew_hours_per_accepted_entity_ratio: f64,
    /// Ratio <1 means shorter complete elapsed duration.
    pub elapsed_wall_hours_ratio: f64,
    /// Ratio <1 means less measured energy per run.
    pub energy_kwh_ratio: Option<f64>,
    /// Ratio <1 means lower high-strain worker-minutes.
    pub high_strain_worker_minutes_ratio: Option<f64>,
    pub manual_total_crew_hours: f64,
    pub robotic_total_crew_hours: f64,
    pub manual_interventions: u64,
    pub robotic_interventions: u64,
    pub manual_safety_events: u64,
    pub robotic_safety_events: u64,
    pub manual_severe_incidents: u64,
    pub robotic_severe_incidents: u64,
    pub quality_noninferiority_passed: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BootstrapInterval {
    pub lower_95: f64,
    pub upper_95: f64,
    pub resamples: usize,
    pub independent_clusters: usize,
    pub seed: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FieldWorkflowStudyScore {
    pub study_design: FieldStudyDesign,
    pub matched_block_count: u64,
    pub independent_cluster_count: u64,
    pub minimum_independent_clusters: u64,
    pub cross_cluster_inference_ready: bool,
    pub quality_noninferiority_margin: f64,
    pub quality_noninferiority_passed_block_count: u64,
    pub quality_noninferiority_failed_block_count: u64,
    pub mean_cluster_productivity_ratio: f64,
    pub median_cluster_productivity_ratio: f64,
    pub productivity_ratio_bootstrap_95: Option<BootstrapInterval>,
    pub overall_manual_accepted_count: u64,
    pub overall_robotic_accepted_count: u64,
    pub overall_manual_entity_count: u64,
    pub overall_robotic_entity_count: u64,
    pub overall_manual_acceptance_rate: f64,
    pub overall_robotic_acceptance_rate: f64,
    pub block_scores: Vec<MatchedBlockScore>,
}

impl Default for FieldWorkflowStudyProfile {
    fn default() -> Self {
        Self {
            quality_noninferiority_margin: 0.005,
            minimum_independent_clusters: 3,
            bootstrap_resamples: 10_000,
            bootstrap_seed: 0xF1E1_DA7A_2026_001,
        }
    }
}

/// Score one matched manual/robotic work-package pair.
///
/// Both arms must refer to exactly the same frozen package, revision, frame,
/// entity manifest, task family, environment and block. A result with waived,
/// held or rejected output never silently inflates accepted throughput.
pub fn score_matched_workflow_block(
    block: &MatchedWorkflowBlock,
    profile: FieldWorkflowStudyProfile,
) -> Result<MatchedBlockScore, FieldWorkflowScoreError> {
    validate_profile(profile)?;
    validate_run(&block.manual, block.study_design)?;
    validate_run(&block.robotic, block.study_design)?;

    if block.manual.arm != WorkflowArm::Manual || block.robotic.arm != WorkflowArm::Robotic {
        return Err(FieldWorkflowScoreError::ArmOrderInvalid(
            block.manual.identity.block_id.clone(),
        ));
    }
    if !same_pairing_identity(&block.manual.identity, &block.robotic.identity) {
        return Err(FieldWorkflowScoreError::PairIdentityMismatch(
            block.manual.identity.block_id.clone(),
        ));
    }
    validate_acceptance_score(&block.manual.acceptance, &block.manual.identity.block_id)?;
    validate_acceptance_score(&block.robotic.acceptance, &block.robotic.identity.block_id)?;

    if !same_work_package(&block.manual.acceptance, &block.robotic.acceptance) {
        return Err(FieldWorkflowScoreError::WorkPackageMismatch(
            block.manual.identity.block_id.clone(),
        ));
    }
    if block.manual.acceptance.accepted_count == 0 {
        return Err(FieldWorkflowScoreError::NoProductiveManualBaseline(
            block.manual.identity.block_id.clone(),
        ));
    }
    if block.robotic.acceptance.accepted_count == 0 {
        return Err(FieldWorkflowScoreError::NoProductiveRoboticRun(
            block.robotic.identity.block_id.clone(),
        ));
    }

    let manual = &block.manual.acceptance;
    let robot = &block.robotic.acceptance;
    let manual_rate = manual.accepted_count as f64 / manual.design_entity_count as f64;
    let robotic_rate = robot.accepted_count as f64 / robot.design_entity_count as f64;
    let manual_per_hour = manual.accepted_count as f64 / manual.total_crew_hours;
    let robotic_per_hour = robot.accepted_count as f64 / robot.total_crew_hours;
    let manual_hours_per_entity = manual.total_crew_hours / manual.accepted_count as f64;
    let robotic_hours_per_entity = robot.total_crew_hours / robot.accepted_count as f64;

    let resource = &block.manual.resources;
    let robot_resource = &block.robotic.resources;
    let energy_kwh_ratio = (resource.energy_kwh > 0.0)
        .then_some(robot_resource.energy_kwh / resource.energy_kwh);
    let high_strain_worker_minutes_ratio = (resource.high_strain_worker_minutes > 0.0)
        .then_some(
            robot_resource.high_strain_worker_minutes
                / resource.high_strain_worker_minutes,
        );

    Ok(MatchedBlockScore {
        cluster_id: block.manual.identity.cluster_id.clone(),
        block_id: block.manual.identity.block_id.clone(),
        task_family: block.manual.identity.task_family.clone(),
        design_entity_count: manual.design_entity_count,
        manual_accepted_count: manual.accepted_count,
        robotic_accepted_count: robot.accepted_count,
        manual_acceptance_rate: manual_rate,
        robotic_acceptance_rate: robotic_rate,
        accepted_output_rate_difference: robotic_rate - manual_rate,
        accepted_output_per_crew_hour_ratio: robotic_per_hour / manual_per_hour,
        crew_hours_per_accepted_entity_ratio: robotic_hours_per_entity
            / manual_hours_per_entity,
        elapsed_wall_hours_ratio: robot_resource.elapsed_wall_hours
            / resource.elapsed_wall_hours,
        energy_kwh_ratio,
        high_strain_worker_minutes_ratio,
        manual_total_crew_hours: manual.total_crew_hours,
        robotic_total_crew_hours: robot.total_crew_hours,
        manual_interventions: resource.intervention_count,
        robotic_interventions: robot_resource.intervention_count,
        manual_safety_events: resource.safety_event_count,
        robotic_safety_events: robot_resource.safety_event_count,
        manual_severe_incidents: resource.severe_incident_count,
        robotic_severe_incidents: robot_resource.severe_incident_count,
        quality_noninferiority_passed: robotic_rate + profile.quality_noninferiority_margin
            >= manual_rate,
    })
}

/// Aggregate matched blocks. Bootstrap resampling is cluster-level so repeated
/// zones from one project/lab do not masquerade as independent replications.
pub fn score_field_workflow_study(
    blocks: &[MatchedWorkflowBlock],
    profile: FieldWorkflowStudyProfile,
) -> Result<FieldWorkflowStudyScore, FieldWorkflowScoreError> {
    validate_profile(profile)?;
    if blocks.is_empty() {
        return Err(FieldWorkflowScoreError::NoMatchedBlocks);
    }

    let design = blocks[0].study_design;
    if blocks.iter().any(|block| block.study_design != design) {
        return Err(FieldWorkflowScoreError::MixedStudyDesigns);
    }

    let mut block_ids = std::collections::BTreeSet::new();
    let mut scored = Vec::with_capacity(blocks.len());
    for block in blocks {
        if !block_ids.insert(block.manual.identity.block_id.as_str()) {
            return Err(FieldWorkflowScoreError::DuplicateBlock(
                block.manual.identity.block_id.clone(),
            ));
        }
        scored.push(score_matched_workflow_block(block, profile)?);
    }

    let mut clusters = BTreeMap::<String, Vec<f64>>::new();
    for score in &scored {
        clusters
            .entry(score.cluster_id.clone())
            .or_default()
            .push(score.accepted_output_per_crew_hour_ratio);
    }
    let mut cluster_means: Vec<f64> = clusters
        .values()
        .map(|ratios| ratios.iter().sum::<f64>() / ratios.len() as f64)
        .collect();
    cluster_means.sort_by(f64::total_cmp);

    let mean_cluster_productivity_ratio =
        cluster_means.iter().sum::<f64>() / cluster_means.len() as f64;
    let median_cluster_productivity_ratio = percentile(&cluster_means, 0.5);
    let productivity_ratio_bootstrap_95 = if cluster_means.len() >= 2
        && profile.bootstrap_resamples >= 100
    {
        Some(cluster_bootstrap_interval(
            &cluster_means,
            profile.bootstrap_resamples,
            profile.bootstrap_seed,
        ))
    } else {
        None
    };

    let overall_manual_accepted_count: u64 =
        scored.iter().map(|score| score.manual_accepted_count).sum();
    let overall_robotic_accepted_count: u64 =
        scored.iter().map(|score| score.robotic_accepted_count).sum();
    let overall_manual_entity_count: u64 =
        scored.iter().map(|score| score.design_entity_count).sum();
    let overall_robotic_entity_count = overall_manual_entity_count;
    let quality_noninferiority_passed_block_count = scored
        .iter()
        .filter(|score| score.quality_noninferiority_passed)
        .count() as u64;

    Ok(FieldWorkflowStudyScore {
        study_design: design,
        matched_block_count: scored.len() as u64,
        independent_cluster_count: clusters.len() as u64,
        minimum_independent_clusters: profile.minimum_independent_clusters as u64,
        cross_cluster_inference_ready: clusters.len() >= profile.minimum_independent_clusters,
        quality_noninferiority_margin: profile.quality_noninferiority_margin,
        quality_noninferiority_passed_block_count,
        quality_noninferiority_failed_block_count: scored.len() as u64
            - quality_noninferiority_passed_block_count,
        mean_cluster_productivity_ratio,
        median_cluster_productivity_ratio,
        productivity_ratio_bootstrap_95,
        overall_manual_accepted_count,
        overall_robotic_accepted_count,
        overall_manual_entity_count,
        overall_robotic_entity_count,
        overall_manual_acceptance_rate: overall_manual_accepted_count as f64
            / overall_manual_entity_count as f64,
        overall_robotic_acceptance_rate: overall_robotic_accepted_count as f64
            / overall_robotic_entity_count as f64,
        block_scores: scored,
    })
}

fn validate_profile(profile: FieldWorkflowStudyProfile) -> Result<(), FieldWorkflowScoreError> {
    if !profile.quality_noninferiority_margin.is_finite()
        || !(0.0..1.0).contains(&profile.quality_noninferiority_margin)
        || profile.minimum_independent_clusters == 0
        || (profile.bootstrap_resamples > 0 && profile.bootstrap_resamples < 100)
    {
        return Err(FieldWorkflowScoreError::InvalidStudyProfile);
    }
    Ok(())
}

fn validate_run(
    run: &FieldWorkflowArmRun,
    study_design: FieldStudyDesign,
) -> Result<(), FieldWorkflowScoreError> {
    let id = &run.identity;
    if [
        id.run_id.as_str(),
        id.cluster_id.as_str(),
        id.block_id.as_str(),
        id.site_id.as_str(),
        id.task_family.as_str(),
    ]
    .into_iter()
    .any(|value| value.trim().is_empty())
    {
        return Err(FieldWorkflowScoreError::EmptyRunIdentity);
    }
    if !is_sha256(&id.pairing_sha256) {
        return Err(FieldWorkflowScoreError::InvalidPairingDigest);
    }
    if !is_sha256(&id.environment_profile_sha256) {
        return Err(FieldWorkflowScoreError::InvalidEnvironmentDigest);
    }
    match (study_design, id.randomization_receipt_sha256.as_deref()) {
        (FieldStudyDesign::RandomizedWithinProject | FieldStudyDesign::SteppedWedge, Some(value))
            if is_sha256(value) => {}
        (FieldStudyDesign::RandomizedWithinProject | FieldStudyDesign::SteppedWedge, _) => {
            return Err(FieldWorkflowScoreError::MissingRandomizationReceipt);
        }
        (FieldStudyDesign::MatchedObservational, None) => {}
        (FieldStudyDesign::MatchedObservational, Some(_)) => {
            return Err(FieldWorkflowScoreError::UnexpectedRandomizationReceipt);
        }
    }
    let resources = &run.resources;
    if !resources.elapsed_wall_hours.is_finite()
        || resources.elapsed_wall_hours <= 0.0
        || !resources.energy_kwh.is_finite()
        || resources.energy_kwh < 0.0
        || !resources.high_strain_worker_minutes.is_finite()
        || resources.high_strain_worker_minutes < 0.0
    {
        return Err(FieldWorkflowScoreError::InvalidResourceObservation(
            id.block_id.clone(),
        ));
    }
    Ok(())
}

fn validate_acceptance_score(
    score: &FieldAcceptanceScore,
    block_id: &str,
) -> Result<(), FieldWorkflowScoreError> {
    if score.design_entity_count == 0
        || score.accepted_count
            + score.rejected_count
            + score.held_count
            + score.waived_count
            + score.not_attempted_count
            != score.design_entity_count
        || score.accepted_count > score.attempted_count
        || score.first_pass_accepted_count > score.accepted_count
    {
        return Err(FieldWorkflowScoreError::InvalidAcceptanceAccounting(
            block_id.to_owned(),
        ));
    }
    let recomputed_hours = score.crew_hours.total();
    if !recomputed_hours.is_finite()
        || recomputed_hours <= 0.0
        || !score.total_crew_hours.is_finite()
        || (recomputed_hours - score.total_crew_hours).abs() > 1.0e-9
    {
        return Err(FieldWorkflowScoreError::InvalidCrewHourAccounting(
            block_id.to_owned(),
        ));
    }
    if score.accepted_entities_per_crew_hour.is_none_or(|value| {
        !value.is_finite()
            || (value - (score.accepted_count as f64 / score.total_crew_hours)).abs() > 1.0e-9
    }) {
        return Err(FieldWorkflowScoreError::InvalidCrewHourAccounting(
            block_id.to_owned(),
        ));
    }
    Ok(())
}

fn same_pairing_identity(
    manual: &WorkflowRunIdentity,
    robotic: &WorkflowRunIdentity,
) -> bool {
    manual.block_id == robotic.block_id
        && manual.cluster_id == robotic.cluster_id
        && manual.site_id == robotic.site_id
        && manual.task_family == robotic.task_family
        && manual.pairing_sha256 == robotic.pairing_sha256
        && manual.environment_profile_sha256 == robotic.environment_profile_sha256
}

fn same_work_package(
    manual: &FieldAcceptanceScore,
    robotic: &FieldAcceptanceScore,
) -> bool {
    manual.package_sha256 == robotic.package_sha256
        && manual.design_revision_sha256 == robotic.design_revision_sha256
        && manual.frame_id == robotic.frame_id
        && manual.entity_manifest_sha256 == robotic.entity_manifest_sha256
        && manual.design_entity_count == robotic.design_entity_count
}

fn cluster_bootstrap_interval(
    cluster_means: &[f64],
    resamples: usize,
    initial_seed: u64,
) -> BootstrapInterval {
    let mut seed = initial_seed;
    let mut means = Vec::with_capacity(resamples);
    for _ in 0..resamples {
        let mut total = 0.0;
        for _ in 0..cluster_means.len() {
            seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
            let index = (seed as usize) % cluster_means.len();
            total += cluster_means[index];
        }
        means.push(total / cluster_means.len() as f64);
    }
    means.sort_by(f64::total_cmp);
    BootstrapInterval {
        lower_95: percentile(&means, 0.025),
        upper_95: percentile(&means, 0.975),
        resamples,
        independent_clusters: cluster_means.len(),
        seed: initial_seed,
    }
}

fn percentile(sorted: &[f64], percentile: f64) -> f64 {
    debug_assert!(!sorted.is_empty());
    let position = percentile.clamp(0.0, 1.0) * (sorted.len() - 1) as f64;
    let lower = position.floor() as usize;
    let upper = position.ceil() as usize;
    let fraction = position - lower as f64;
    sorted[lower] * (1.0 - fraction) + sorted[upper] * fraction
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(byte: char) -> String {
        std::iter::repeat_n(byte, 64).collect()
    }

    fn crew_hours(total: f64) -> CrewHours {
        CrewHours {
            bim_vdc: total * 0.10,
            coordination: total * 0.10,
            survey_control: total * 0.10,
            setup_calibration: total * 0.10,
            stationing: total * 0.10,
            execution_supervision: total * 0.10,
            inspection: total * 0.10,
            recovery: total * 0.05,
            rework: total * 0.15,
            other: total * 0.10,
        }
    }

    fn acceptance(accepted: u64, total: u64, crew: f64) -> FieldAcceptanceScore {
        FieldAcceptanceScore {
            package_sha256: digest('a'),
            design_revision_sha256: digest('b'),
            frame_id: "site-level-02".into(),
            entity_manifest_sha256: digest('c'),
            design_entity_count: total,
            accepted_count: accepted,
            rejected_count: total - accepted,
            held_count: 0,
            waived_count: 0,
            not_attempted_count: 0,
            attempted_count: total,
            first_pass_accepted_count: accepted,
            rework_entity_count: 0,
            crew_hours: crew_hours(crew),
            total_crew_hours: crew,
            accepted_entities_per_crew_hour: Some(accepted as f64 / crew),
            first_pass_yield: Some(accepted as f64 / total as f64),
            rework_crew_hour_fraction: Some(crew_hours(crew).rework / crew),
            entity_results: Vec::new(),
        }
    }

    fn arm(
        id: &str,
        cluster: &str,
        block: &str,
        arm: WorkflowArm,
        accepted: u64,
        total: u64,
        crew: f64,
    ) -> FieldWorkflowArmRun {
        FieldWorkflowArmRun {
            identity: WorkflowRunIdentity {
                run_id: id.into(),
                cluster_id: cluster.into(),
                block_id: block.into(),
                site_id: "site-01".into(),
                task_family: "interior-layout-simple".into(),
                pairing_sha256: digest('d'),
                environment_profile_sha256: digest('e'),
                randomization_receipt_sha256: Some(digest('f')),
            },
            arm,
            acceptance: acceptance(accepted, total, crew),
            resources: WorkflowResourceObservation {
                elapsed_wall_hours: if arm == WorkflowArm::Manual { 8.0 } else { 5.0 },
                energy_kwh: if arm == WorkflowArm::Manual { 1.0 } else { 2.0 },
                high_strain_worker_minutes: if arm == WorkflowArm::Manual { 120.0 } else { 20.0 },
                intervention_count: if arm == WorkflowArm::Manual { 4 } else { 2 },
                safety_event_count: 0,
                severe_incident_count: 0,
            },
        }
    }

    fn block(cluster: &str, block_id: &str) -> MatchedWorkflowBlock {
        MatchedWorkflowBlock {
            study_design: FieldStudyDesign::RandomizedWithinProject,
            manual: arm("manual", cluster, block_id, WorkflowArm::Manual, 95, 100, 40.0),
            robotic: arm("robot", cluster, block_id, WorkflowArm::Robotic, 94, 100, 20.0),
        }
    }

    #[test]
    fn whole_workflow_productivity_uses_all_crew_hours_and_accepted_output() {
        let score = score_matched_workflow_block(&block("lab-a", "block-1"), Default::default())
            .unwrap();
        assert_eq!(score.manual_total_crew_hours, 40.0);
        assert_eq!(score.robotic_total_crew_hours, 20.0);
        assert!((score.accepted_output_per_crew_hour_ratio - (94.0 / 20.0) / (95.0 / 40.0)).abs() < 1.0e-9);
        assert!((score.crew_hours_per_accepted_entity_ratio - (20.0 / 94.0) / (40.0 / 95.0)).abs() < 1.0e-9);
        assert!((score.elapsed_wall_hours_ratio - 5.0 / 8.0).abs() < 1.0e-9);
        assert!(score.quality_noninferiority_passed);
    }

    #[test]
    fn waived_and_held_entities_cannot_be_counted_as_accepted_output() {
        let mut candidate = block("lab-a", "block-1");
        candidate.robotic.acceptance.accepted_count = 80;
        candidate.robotic.acceptance.rejected_count = 5;
        candidate.robotic.acceptance.waived_count = 10;
        candidate.robotic.acceptance.held_count = 5;
        candidate.robotic.acceptance.accepted_entities_per_crew_hour = Some(4.0);
        let score = score_matched_workflow_block(&candidate, Default::default()).unwrap();
        assert_eq!(score.robotic_accepted_count, 80);
        assert!(!score.quality_noninferiority_passed);
        assert_eq!(score.accepted_output_per_crew_hour_ratio, (80.0 / 20.0) / (95.0 / 40.0));
    }

    #[test]
    fn mismatched_work_package_or_environment_is_not_comparable() {
        let mut candidate = block("lab-a", "block-1");
        candidate.robotic.acceptance.frame_id = "stale-frame".into();
        assert_eq!(
            score_matched_workflow_block(&candidate, Default::default()),
            Err(FieldWorkflowScoreError::WorkPackageMismatch("block-1".into()))
        );

        let mut candidate = block("lab-a", "block-1");
        candidate.robotic.identity.environment_profile_sha256 = digest('0');
        assert_eq!(
            score_matched_workflow_block(&candidate, Default::default()),
            Err(FieldWorkflowScoreError::PairIdentityMismatch("block-1".into()))
        );
    }

    #[test]
    fn randomized_study_requires_a_randomization_receipt() {
        let mut candidate = block("lab-a", "block-1");
        candidate.manual.identity.randomization_receipt_sha256 = None;
        assert_eq!(
            score_matched_workflow_block(&candidate, Default::default()),
            Err(FieldWorkflowScoreError::MissingRandomizationReceipt)
        );
    }

    #[test]
    fn cluster_bootstrap_uses_independent_projects_not_entity_count() {
        let blocks = vec![
            block("lab-a", "block-1"),
            block("lab-a", "block-2"),
            block("lab-b", "block-3"),
            block("lab-c", "block-4"),
        ];
        let score = score_field_workflow_study(&blocks, Default::default()).unwrap();
        assert_eq!(score.matched_block_count, 4);
        assert_eq!(score.independent_cluster_count, 3);
        assert!(score.cross_cluster_inference_ready);
        assert_eq!(score.productivity_ratio_bootstrap_95.as_ref().unwrap().independent_clusters, 3);
        assert_eq!(score.productivity_ratio_bootstrap_95, score_field_workflow_study(&blocks, Default::default()).unwrap().productivity_ratio_bootstrap_95);
    }

    #[test]
    fn one_cluster_is_not_misrepresented_as_cross_cluster_evidence() {
        let blocks = vec![
            block("lab-a", "block-1"),
            block("lab-a", "block-2"),
        ];
        let score = score_field_workflow_study(&blocks, Default::default()).unwrap();
        assert!(!score.cross_cluster_inference_ready);
        assert_eq!(score.independent_cluster_count, 1);
        assert_eq!(score.productivity_ratio_bootstrap_95, None);
    }

    #[test]
    fn duplicate_blocks_and_mixed_study_designs_are_rejected() {
        let duplicate = vec![
            block("lab-a", "same"),
            block("lab-b", "same"),
        ];
        assert_eq!(
            score_field_workflow_study(&duplicate, Default::default()),
            Err(FieldWorkflowScoreError::DuplicateBlock("same".into()))
        );

        let mut matched = block("lab-b", "block-2");
        matched.study_design = FieldStudyDesign::MatchedObservational;
        matched.manual.identity.randomization_receipt_sha256 = None;
        matched.robotic.identity.randomization_receipt_sha256 = None;
        assert_eq!(
            score_field_workflow_study(
                &[block("lab-a", "block-1"), matched],
                Default::default()
            ),
            Err(FieldWorkflowScoreError::MixedStudyDesigns)
        );
    }
}