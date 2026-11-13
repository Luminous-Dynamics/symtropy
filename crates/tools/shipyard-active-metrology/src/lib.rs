//! Decision-aware metrology option selection for inter-pass welding.
//!
//! V0 accepts synthetic contract profiles only. It never treats a proposed
//! measurement as an observation and never authorizes a weld or robot action.

use std::collections::HashSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceClass {
    SyntheticContractOnly,
    Measured,
    Simulation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MeasurementOutput {
    ResidualFillGeometry,
    Other,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MeasurementProfile {
    pub profile_id: String,
    pub revision: String,
    pub evidence_class: EvidenceClass,
    pub output: MeasurementOutput,
    pub qualified: bool,
    pub operating_domain_supported: bool,
    pub guaranteed_post_uncertainty_bound_units: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CurrentObservation {
    pub evidence_class: EvidenceClass,
    pub workpiece_id: String,
    pub joint_revision: String,
    pub layer_id: String,
    pub predecessor_pass_id: String,
    pub scan_id: String,
    pub scan_sha256: [u8; 32],
    pub registration_receipt_sha256: [u8; 32],
    pub uncertainty_upper_bound_units: u64,
    pub decision_threshold_units: u64,
    pub observed_at_tick: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MeasurementOption {
    pub action_id: String,
    pub workpiece_id: String,
    pub joint_revision: String,
    pub layer_id: String,
    pub predecessor_pass_id: String,
    pub source_scan_id: String,
    pub source_scan_sha256: [u8; 32],
    pub source_registration_receipt_sha256: [u8; 32],
    pub profile_id: String,
    pub profile_revision: String,
    pub declared_post_uncertainty_bound_units: u64,
    pub acquisition_time_units: u64,
    pub energy_units: u64,
    pub interruption_risk_units: u64,
    pub resource_feasible: bool,
    pub access_feasible: bool,
    pub already_observed: bool,
    pub future_information_used: bool,
    pub live_action_requested: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectorInput {
    pub observation: CurrentObservation,
    pub profiles: Vec<MeasurementProfile>,
    pub options: Vec<MeasurementOption>,
    pub decision_at_tick: u64,
    pub maximum_observation_age_ticks: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AbstainReason {
    EmptyIdentity,
    MeasuredEvidenceNotAdmitted,
    MissingScanDigest,
    MissingRegistrationReceipt,
    ObservationFromFuture,
    ObservationTooOld,
    InvalidDecisionThreshold,
    DuplicateProfileIdentity,
    DuplicateActionIdentity,
    NoUsefulMeasurementOption,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExclusionReason {
    WrongWorkpieceOrPredecessor,
    WrongScanIdentity,
    WrongScanDigest,
    WrongRegistrationReceipt,
    ProfileMissingOrRevisionMismatch,
    ProfileNotQualified,
    WrongMeasurementOutput,
    ProfileEvidenceNotAdmitted,
    ProfileOutsideOperatingDomain,
    InvalidProfileBound,
    ClaimedUncertaintyBetterThanProfile,
    NoUncertaintyReduction,
    ResourceInfeasible,
    AccessInfeasible,
    ZeroAcquisitionTime,
    AlreadyObserved,
    FutureInformation,
    LiveActionForbidden,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MeasurementObjectiveVector {
    pub post_uncertainty_bound_units: u64,
    pub acquisition_time_units: u64,
    pub energy_units: u64,
    pub interruption_risk_units: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MeasurementRecommendation {
    pub action_id: String,
    pub objectives: MeasurementObjectiveVector,
    pub guaranteed_reduction_units: u64,
    pub threshold_ready: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExcludedOption {
    pub action_id: String,
    pub reason: ExclusionReason,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Decision {
    NoAdditionalMeasurementRequired {
        current_uncertainty_bound_units: u64,
        decision_threshold_units: u64,
        live_action_authorized: bool,
    },
    Abstain {
        reason: AbstainReason,
        excluded: Vec<ExcludedOption>,
    },
    MeasurementParetoSet {
        recommendations: Vec<MeasurementRecommendation>,
        eligible_option_count: usize,
        threshold_ready_option_count: usize,
        excluded: Vec<ExcludedOption>,
        automatic_selection: bool,
        live_action_authorized: bool,
        new_observation_required: bool,
        claim_ceiling: &'static str,
    },
}

fn is_empty(value: &str) -> bool {
    value.trim().is_empty()
}

fn digest_is_zero(digest: &[u8; 32]) -> bool {
    digest.iter().all(|byte| *byte == 0)
}

fn validate_input(input: &SelectorInput) -> Result<(), AbstainReason> {
    let observation = &input.observation;
    let identities = [
        observation.workpiece_id.as_str(),
        observation.joint_revision.as_str(),
        observation.layer_id.as_str(),
        observation.predecessor_pass_id.as_str(),
        observation.scan_id.as_str(),
    ];
    if identities.into_iter().any(is_empty) {
        return Err(AbstainReason::EmptyIdentity);
    }

    if observation.evidence_class != EvidenceClass::SyntheticContractOnly {
        return Err(AbstainReason::MeasuredEvidenceNotAdmitted);
    }
    if digest_is_zero(&observation.scan_sha256) {
        return Err(AbstainReason::MissingScanDigest);
    }
    if digest_is_zero(&observation.registration_receipt_sha256) {
        return Err(AbstainReason::MissingRegistrationReceipt);
    }
    if observation.decision_threshold_units == 0 {
        return Err(AbstainReason::InvalidDecisionThreshold);
    }
    if observation.observed_at_tick > input.decision_at_tick {
        return Err(AbstainReason::ObservationFromFuture);
    }
    if input
        .decision_at_tick
        .saturating_sub(observation.observed_at_tick)
        > input.maximum_observation_age_ticks
    {
        return Err(AbstainReason::ObservationTooOld);
    }

    let mut profile_ids = HashSet::new();
    for profile in &input.profiles {
        if is_empty(&profile.profile_id)
            || is_empty(&profile.revision)
            || !profile_ids.insert((profile.profile_id.as_str(), profile.revision.as_str()))
        {
            return Err(AbstainReason::DuplicateProfileIdentity);
        }
    }

    let mut action_ids = HashSet::new();
    for option in &input.options {
        if is_empty(&option.action_id) || !action_ids.insert(option.action_id.as_str()) {
            return Err(AbstainReason::DuplicateActionIdentity);
        }
    }
    Ok(())
}

fn profile_for<'a>(
    profiles: &'a [MeasurementProfile],
    option: &MeasurementOption,
) -> Option<&'a MeasurementProfile> {
    profiles.iter().find(|profile| {
        profile.profile_id == option.profile_id && profile.revision == option.profile_revision
    })
}

fn exclusion_for(
    observation: &CurrentObservation,
    profiles: &[MeasurementProfile],
    option: &MeasurementOption,
) -> Option<ExclusionReason> {
    if option.workpiece_id != observation.workpiece_id
        || option.joint_revision != observation.joint_revision
        || option.layer_id != observation.layer_id
        || option.predecessor_pass_id != observation.predecessor_pass_id
    {
        return Some(ExclusionReason::WrongWorkpieceOrPredecessor);
    }
    if option.source_scan_id != observation.scan_id {
        return Some(ExclusionReason::WrongScanIdentity);
    }
    if option.source_scan_sha256 != observation.scan_sha256 {
        return Some(ExclusionReason::WrongScanDigest);
    }
    if option.source_registration_receipt_sha256 != observation.registration_receipt_sha256 {
        return Some(ExclusionReason::WrongRegistrationReceipt);
    }

    let Some(profile) = profile_for(profiles, option) else {
        return Some(ExclusionReason::ProfileMissingOrRevisionMismatch);
    };
    if !profile.qualified {
        return Some(ExclusionReason::ProfileNotQualified);
    }
    if profile.output != MeasurementOutput::ResidualFillGeometry {
        return Some(ExclusionReason::WrongMeasurementOutput);
    }
    if profile.evidence_class != EvidenceClass::SyntheticContractOnly {
        return Some(ExclusionReason::ProfileEvidenceNotAdmitted);
    }
    if !profile.operating_domain_supported {
        return Some(ExclusionReason::ProfileOutsideOperatingDomain);
    }
    if profile.guaranteed_post_uncertainty_bound_units == 0 {
        return Some(ExclusionReason::InvalidProfileBound);
    }
    if option.declared_post_uncertainty_bound_units
        < profile.guaranteed_post_uncertainty_bound_units
    {
        return Some(ExclusionReason::ClaimedUncertaintyBetterThanProfile);
    }
    if option.declared_post_uncertainty_bound_units >= observation.uncertainty_upper_bound_units {
        return Some(ExclusionReason::NoUncertaintyReduction);
    }
    if !option.resource_feasible {
        return Some(ExclusionReason::ResourceInfeasible);
    }
    if !option.access_feasible {
        return Some(ExclusionReason::AccessInfeasible);
    }
    if option.acquisition_time_units == 0 {
        return Some(ExclusionReason::ZeroAcquisitionTime);
    }
    if option.already_observed {
        return Some(ExclusionReason::AlreadyObserved);
    }
    if option.future_information_used {
        return Some(ExclusionReason::FutureInformation);
    }
    if option.live_action_requested {
        return Some(ExclusionReason::LiveActionForbidden);
    }
    None
}

fn dominates(left: &MeasurementObjectiveVector, right: &MeasurementObjectiveVector) -> bool {
    let left_values = [
        left.post_uncertainty_bound_units,
        left.acquisition_time_units,
        left.energy_units,
        left.interruption_risk_units,
    ];
    let right_values = [
        right.post_uncertainty_bound_units,
        right.acquisition_time_units,
        right.energy_units,
        right.interruption_risk_units,
    ];

    let mut strictly_better = false;
    for (left_value, right_value) in left_values.into_iter().zip(right_values) {
        if left_value > right_value {
            return false;
        }
        if left_value < right_value {
            strictly_better = true;
        }
    }
    strictly_better
}

/// Select qualified synthetic measurement options for the latest scan.
///
/// Returned options are proposals only. A caller must acquire a new scan,
/// calculate fresh digests, and submit a new observation before replanning.
pub fn select_measurements(input: &SelectorInput) -> Decision {
    if let Err(reason) = validate_input(input) {
        return Decision::Abstain {
            reason,
            excluded: Vec::new(),
        };
    }

    let observation = &input.observation;
    if observation.uncertainty_upper_bound_units <= observation.decision_threshold_units {
        return Decision::NoAdditionalMeasurementRequired {
            current_uncertainty_bound_units: observation.uncertainty_upper_bound_units,
            decision_threshold_units: observation.decision_threshold_units,
            live_action_authorized: false,
        };
    }

    let mut eligible = Vec::new();
    let mut excluded = Vec::new();
    for option in &input.options {
        if let Some(reason) = exclusion_for(observation, &input.profiles, option) {
            excluded.push(ExcludedOption {
                action_id: option.action_id.clone(),
                reason,
            });
            continue;
        }

        let reduction = observation
            .uncertainty_upper_bound_units
            .checked_sub(option.declared_post_uncertainty_bound_units);
        let Some(reduction) = reduction else {
            return Decision::Abstain {
                reason: AbstainReason::InvalidDecisionThreshold,
                excluded,
            };
        };
        eligible.push(MeasurementRecommendation {
            action_id: option.action_id.clone(),
            objectives: MeasurementObjectiveVector {
                post_uncertainty_bound_units: option.declared_post_uncertainty_bound_units,
                acquisition_time_units: option.acquisition_time_units,
                energy_units: option.energy_units,
                interruption_risk_units: option.interruption_risk_units,
            },
            guaranteed_reduction_units: reduction,
            threshold_ready: option.declared_post_uncertainty_bound_units
                <= observation.decision_threshold_units,
        });
    }

    if eligible.is_empty() {
        return Decision::Abstain {
            reason: AbstainReason::NoUsefulMeasurementOption,
            excluded,
        };
    }

    let eligible_count = eligible.len();
    let threshold_ready_count = eligible
        .iter()
        .filter(|recommendation| recommendation.threshold_ready)
        .count();
    let mut frontier: Vec<MeasurementRecommendation> = eligible
        .iter()
        .filter(|candidate| {
            !eligible.iter().any(|other| {
                other.action_id != candidate.action_id
                    && dominates(&other.objectives, &candidate.objectives)
            })
        })
        .cloned()
        .collect();
    frontier.sort_by(|left, right| left.action_id.cmp(&right.action_id));

    Decision::MeasurementParetoSet {
        recommendations: frontier,
        eligible_option_count: eligible_count,
        threshold_ready_option_count: threshold_ready_count,
        excluded,
        automatic_selection: false,
        live_action_authorized: false,
        new_observation_required: true,
        claim_ceiling: "SYNTHETIC_MEASUREMENT_OPTION_CONTRACT_ONLY",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(id: &str, bound: u64, output: MeasurementOutput) -> MeasurementProfile {
        MeasurementProfile {
            profile_id: id.to_owned(),
            revision: "R1".to_owned(),
            evidence_class: EvidenceClass::SyntheticContractOnly,
            output,
            qualified: true,
            operating_domain_supported: true,
            guaranteed_post_uncertainty_bound_units: bound,
        }
    }

    fn option(
        id: &str,
        profile_id: &str,
        post_bound: u64,
        time: u64,
        energy: u64,
        risk: u64,
    ) -> MeasurementOption {
        MeasurementOption {
            action_id: id.to_owned(),
            workpiece_id: "WID-SYNTH-01".to_owned(),
            joint_revision: "JOINT-R1".to_owned(),
            layer_id: "LAYER-03".to_owned(),
            predecessor_pass_id: "PASS-02".to_owned(),
            source_scan_id: "SCAN-AFTER-PASS-02".to_owned(),
            source_scan_sha256: [0x11; 32],
            source_registration_receipt_sha256: [0x22; 32],
            profile_id: profile_id.to_owned(),
            profile_revision: "R1".to_owned(),
            declared_post_uncertainty_bound_units: post_bound,
            acquisition_time_units: time,
            energy_units: energy,
            interruption_risk_units: risk,
            resource_feasible: true,
            access_feasible: true,
            already_observed: false,
            future_information_used: false,
            live_action_requested: false,
        }
    }

    fn base_input() -> SelectorInput {
        SelectorInput {
            observation: CurrentObservation {
                evidence_class: EvidenceClass::SyntheticContractOnly,
                workpiece_id: "WID-SYNTH-01".to_owned(),
                joint_revision: "JOINT-R1".to_owned(),
                layer_id: "LAYER-03".to_owned(),
                predecessor_pass_id: "PASS-02".to_owned(),
                scan_id: "SCAN-AFTER-PASS-02".to_owned(),
                scan_sha256: [0x11; 32],
                registration_receipt_sha256: [0x22; 32],
                uncertainty_upper_bound_units: 20,
                decision_threshold_units: 6,
                observed_at_tick: 100,
            },
            profiles: vec![
                profile("HIGH-RES", 3, MeasurementOutput::ResidualFillGeometry),
                profile("ALTERNATE-VIEW", 5, MeasurementOutput::ResidualFillGeometry),
                profile("FAST-RESCAN", 8, MeasurementOutput::ResidualFillGeometry),
                profile("LOW-RES", 12, MeasurementOutput::ResidualFillGeometry),
                profile(
                    "SLOW-DOMINATED",
                    10,
                    MeasurementOutput::ResidualFillGeometry,
                ),
            ],
            options: vec![
                option("HIGH-RES", "HIGH-RES", 3, 10, 8, 1),
                option("ALTERNATE-VIEW", "ALTERNATE-VIEW", 5, 5, 3, 3),
                option("FAST-RESCAN", "FAST-RESCAN", 8, 2, 1, 1),
                option("LOW-RES", "LOW-RES", 12, 1, 1, 0),
                option("SLOW-DOMINATED", "SLOW-DOMINATED", 10, 8, 7, 5),
            ],
            decision_at_tick: 100,
            maximum_observation_age_ticks: 10,
        }
    }

    #[test]
    fn returns_pareto_measurement_options_without_automatic_selection() {
        let result = select_measurements(&base_input());
        let Decision::MeasurementParetoSet {
            recommendations,
            eligible_option_count,
            threshold_ready_option_count,
            automatic_selection,
            live_action_authorized,
            new_observation_required,
            ..
        } = result
        else {
            panic!("valid synthetic input should return measurement options");
        };

        assert_eq!(eligible_option_count, 5);
        assert_eq!(threshold_ready_option_count, 2);
        assert_eq!(
            recommendations
                .iter()
                .map(|item| item.action_id.as_str())
                .collect::<Vec<_>>(),
            vec!["ALTERNATE-VIEW", "FAST-RESCAN", "HIGH-RES", "LOW-RES"]
        );
        assert!(!automatic_selection);
        assert!(!live_action_authorized);
        assert!(new_observation_required);
    }

    #[test]
    fn observation_already_within_threshold_needs_no_additional_measurement() {
        let mut input = base_input();
        input.observation.uncertainty_upper_bound_units = 5;
        assert!(matches!(
            select_measurements(&input),
            Decision::NoAdditionalMeasurementRequired {
                current_uncertainty_bound_units: 5,
                decision_threshold_units: 6,
                live_action_authorized: false
            }
        ));
    }

    #[test]
    fn measured_observations_are_not_admitted_in_v0() {
        let mut input = base_input();
        input.observation.evidence_class = EvidenceClass::Measured;
        assert!(matches!(
            select_measurements(&input),
            Decision::Abstain {
                reason: AbstainReason::MeasuredEvidenceNotAdmitted,
                ..
            }
        ));
    }

    #[test]
    fn missing_scan_digest_causes_abstention() {
        let mut input = base_input();
        input.observation.scan_sha256 = [0; 32];
        assert!(matches!(
            select_measurements(&input),
            Decision::Abstain {
                reason: AbstainReason::MissingScanDigest,
                ..
            }
        ));
    }

    #[test]
    fn missing_registration_receipt_causes_abstention() {
        let mut input = base_input();
        input.observation.registration_receipt_sha256 = [0; 32];
        assert!(matches!(
            select_measurements(&input),
            Decision::Abstain {
                reason: AbstainReason::MissingRegistrationReceipt,
                ..
            }
        ));
    }

    #[test]
    fn future_observation_causes_abstention() {
        let mut input = base_input();
        input.observation.observed_at_tick = 101;
        assert!(matches!(
            select_measurements(&input),
            Decision::Abstain {
                reason: AbstainReason::ObservationFromFuture,
                ..
            }
        ));
    }

    #[test]
    fn stale_observation_causes_abstention() {
        let mut input = base_input();
        input.observation.observed_at_tick = 50;
        assert!(matches!(
            select_measurements(&input),
            Decision::Abstain {
                reason: AbstainReason::ObservationTooOld,
                ..
            }
        ));
    }

    #[test]
    fn wrong_scan_digest_is_excluded() {
        let mut input = base_input();
        input.options[0].source_scan_sha256 = [0x33; 32];
        let Decision::MeasurementParetoSet { excluded, .. } = select_measurements(&input) else {
            panic!("remaining options should be eligible");
        };
        assert_eq!(excluded[0].reason, ExclusionReason::WrongScanDigest);
    }

    #[test]
    fn wrong_registration_receipt_is_excluded() {
        let mut input = base_input();
        input.options[0].source_registration_receipt_sha256 = [0x44; 32];
        let Decision::MeasurementParetoSet { excluded, .. } = select_measurements(&input) else {
            panic!("remaining options should be eligible");
        };
        assert_eq!(
            excluded[0].reason,
            ExclusionReason::WrongRegistrationReceipt
        );
    }

    #[test]
    fn stale_predecessor_identity_is_excluded() {
        let mut input = base_input();
        input.options[0].predecessor_pass_id = "PASS-01".to_owned();
        let Decision::MeasurementParetoSet { excluded, .. } = select_measurements(&input) else {
            panic!("remaining options should be eligible");
        };
        assert_eq!(
            excluded[0].reason,
            ExclusionReason::WrongWorkpieceOrPredecessor
        );
    }

    #[test]
    fn profile_revision_mismatch_is_excluded() {
        let mut input = base_input();
        input.options[0].profile_revision = "R0".to_owned();
        let Decision::MeasurementParetoSet { excluded, .. } = select_measurements(&input) else {
            panic!("remaining options should be eligible");
        };
        assert_eq!(
            excluded[0].reason,
            ExclusionReason::ProfileMissingOrRevisionMismatch
        );
    }

    #[test]
    fn unqualified_profile_is_excluded() {
        let mut input = base_input();
        input.profiles[0].qualified = false;
        let Decision::MeasurementParetoSet { excluded, .. } = select_measurements(&input) else {
            panic!("remaining options should be eligible");
        };
        assert_eq!(excluded[0].reason, ExclusionReason::ProfileNotQualified);
    }

    #[test]
    fn wrong_measurement_output_is_excluded() {
        let mut input = base_input();
        input.profiles[0].output = MeasurementOutput::Other;
        let Decision::MeasurementParetoSet { excluded, .. } = select_measurements(&input) else {
            panic!("remaining options should be eligible");
        };
        assert_eq!(excluded[0].reason, ExclusionReason::WrongMeasurementOutput);
    }

    #[test]
    fn measured_profile_is_not_admitted() {
        let mut input = base_input();
        input.profiles[0].evidence_class = EvidenceClass::Measured;
        let Decision::MeasurementParetoSet { excluded, .. } = select_measurements(&input) else {
            panic!("remaining options should be eligible");
        };
        assert_eq!(
            excluded[0].reason,
            ExclusionReason::ProfileEvidenceNotAdmitted
        );
    }

    #[test]
    fn profile_outside_domain_is_excluded() {
        let mut input = base_input();
        input.profiles[0].operating_domain_supported = false;
        let Decision::MeasurementParetoSet { excluded, .. } = select_measurements(&input) else {
            panic!("remaining options should be eligible");
        };
        assert_eq!(
            excluded[0].reason,
            ExclusionReason::ProfileOutsideOperatingDomain
        );
    }

    #[test]
    fn option_cannot_claim_more_accuracy_than_profile_supports() {
        let mut input = base_input();
        input.options[0].declared_post_uncertainty_bound_units = 2;
        let Decision::MeasurementParetoSet { excluded, .. } = select_measurements(&input) else {
            panic!("remaining options should be eligible");
        };
        assert_eq!(
            excluded[0].reason,
            ExclusionReason::ClaimedUncertaintyBetterThanProfile
        );
    }

    #[test]
    fn non_reducing_option_is_excluded() {
        let mut input = base_input();
        input.options[0].declared_post_uncertainty_bound_units = 20;
        let Decision::MeasurementParetoSet { excluded, .. } = select_measurements(&input) else {
            panic!("remaining options should be eligible");
        };
        assert_eq!(excluded[0].reason, ExclusionReason::NoUncertaintyReduction);
    }

    #[test]
    fn resource_infeasible_option_is_excluded() {
        let mut input = base_input();
        input.options[0].resource_feasible = false;
        let Decision::MeasurementParetoSet { excluded, .. } = select_measurements(&input) else {
            panic!("remaining options should be eligible");
        };
        assert_eq!(excluded[0].reason, ExclusionReason::ResourceInfeasible);
    }

    #[test]
    fn access_infeasible_option_is_excluded() {
        let mut input = base_input();
        input.options[0].access_feasible = false;
        let Decision::MeasurementParetoSet { excluded, .. } = select_measurements(&input) else {
            panic!("remaining options should be eligible");
        };
        assert_eq!(excluded[0].reason, ExclusionReason::AccessInfeasible);
    }

    #[test]
    fn already_observed_option_is_excluded() {
        let mut input = base_input();
        input.options[0].already_observed = true;
        let Decision::MeasurementParetoSet { excluded, .. } = select_measurements(&input) else {
            panic!("remaining options should be eligible");
        };
        assert_eq!(excluded[0].reason, ExclusionReason::AlreadyObserved);
    }

    #[test]
    fn future_information_option_is_excluded() {
        let mut input = base_input();
        input.options[0].future_information_used = true;
        let Decision::MeasurementParetoSet { excluded, .. } = select_measurements(&input) else {
            panic!("remaining options should be eligible");
        };
        assert_eq!(excluded[0].reason, ExclusionReason::FutureInformation);
    }

    #[test]
    fn live_action_is_excluded() {
        let mut input = base_input();
        input.options[0].live_action_requested = true;
        let Decision::MeasurementParetoSet { excluded, .. } = select_measurements(&input) else {
            panic!("remaining options should be eligible");
        };
        assert_eq!(excluded[0].reason, ExclusionReason::LiveActionForbidden);
    }

    #[test]
    fn no_qualified_reducing_option_causes_abstention() {
        let mut input = base_input();
        input.options.iter_mut().for_each(|option| {
            option.resource_feasible = false;
        });
        assert!(matches!(
            select_measurements(&input),
            Decision::Abstain {
                reason: AbstainReason::NoUsefulMeasurementOption,
                ..
            }
        ));
    }

    #[test]
    fn duplicate_action_identity_causes_abstention() {
        let mut input = base_input();
        input.options[1].action_id = input.options[0].action_id.clone();
        assert!(matches!(
            select_measurements(&input),
            Decision::Abstain {
                reason: AbstainReason::DuplicateActionIdentity,
                ..
            }
        ));
    }

    #[test]
    fn duplicate_profile_identity_causes_abstention() {
        let mut input = base_input();
        input.profiles.push(input.profiles[0].clone());
        assert!(matches!(
            select_measurements(&input),
            Decision::Abstain {
                reason: AbstainReason::DuplicateProfileIdentity,
                ..
            }
        ));
    }

    #[test]
    fn declared_decision_threshold_is_never_lowered() {
        let mut input = base_input();
        input.observation.decision_threshold_units = 2;
        let Decision::MeasurementParetoSet {
            threshold_ready_option_count,
            ..
        } = select_measurements(&input)
        else {
            panic!("uncertainty-reducing options should remain visible");
        };
        assert_eq!(threshold_ready_option_count, 0);
    }
}
