// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Test-only executable reference for SYM-EVAL-001E-Q5 claim admission.
//!
//! This freezes typed validity-profile and receipt semantics without granting
//! product authority or inheriting PASS from any queued/incomplete predecessor.

#![allow(dead_code)]

use std::collections::BTreeMap;

const PROFILE_DOMAIN: &[u8] = b"sym-eval.benchmark-validity-profile.v1\0";
const RECEIPT_DOMAIN: &[u8] = b"sym-eval.benchmark-validity-receipt.v1\0";
const PROFILE_DOC: &str =
    include_str!("../../../../docs/research/SYM_EVAL_001E_Q5_VALIDITY_REFERENCE_V1.json");

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Id32([u8; 32]);

impl Id32 {
    const fn repeated(byte: u8) -> Self {
        Self([byte; 32])
    }

    fn write(self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.0);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ClaimClassV1 {
    SyntheticScorerReference,
    RawTrackerDiagnostic,
    RenderedTrackerBenchmark,
    IntegratedBeliefOnlineBenchmark,
    CausalWorldModelBenchmark,
}

impl ClaimClassV1 {
    const fn tag(self) -> u8 {
        match self {
            Self::SyntheticScorerReference => 1,
            Self::RawTrackerDiagnostic => 2,
            Self::RenderedTrackerBenchmark => 3,
            Self::IntegratedBeliefOnlineBenchmark => 4,
            Self::CausalWorldModelBenchmark => 5,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum GateIdV1 {
    A2,
    A3,
    A4,
    A5,
    RenderedSensor001B,
    BeliefAdapter001C,
    Core001,
    Core001S,
    Core003,
    Core004,
    Core006,
    Core007,
    Core008,
    Q0,
    Q1,
    Q1S,
    Q2,
    Q3,
    Q4,
}

impl GateIdV1 {
    const fn tag(self) -> u8 {
        match self {
            Self::A2 => 1,
            Self::A3 => 2,
            Self::A4 => 3,
            Self::A5 => 4,
            Self::RenderedSensor001B => 5,
            Self::BeliefAdapter001C => 6,
            Self::Core001 => 7,
            Self::Core001S => 8,
            Self::Core003 => 9,
            Self::Core004 => 10,
            Self::Core006 => 11,
            Self::Core007 => 12,
            Self::Core008 => 13,
            Self::Q0 => 14,
            Self::Q1 => 15,
            Self::Q1S => 16,
            Self::Q2 => 17,
            Self::Q3 => 18,
            Self::Q4 => 19,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ChallengeExposureV1 {
    FreshHidden,
    PreviouslyRevealed,
    PublicFixed,
    TrainingKnown,
    UnknownExposure,
}

impl ChallengeExposureV1 {
    const fn tag(self) -> u8 {
        match self {
            Self::FreshHidden => 1,
            Self::PreviouslyRevealed => 2,
            Self::PublicFixed => 3,
            Self::TrainingKnown => 4,
            Self::UnknownExposure => 5,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NotApplicableReasonV1 {
    NoRendererInSyntheticProfile,
    NoStatisticalProbeBecauseExactEqualityRequired,
    NoAdapterInRawTrackerBaseline,
}

impl NotApplicableReasonV1 {
    const fn tag(self) -> u8 {
        match self {
            Self::NoRendererInSyntheticProfile => 1,
            Self::NoStatisticalProbeBecauseExactEqualityRequired => 2,
            Self::NoAdapterInRawTrackerBaseline => 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum GateStatusV1 {
    Pass,
    Fail,
    LeakDetected,
    ManipulationDetected,
    RejectedInvalidInput,
    NotApplicable(NotApplicableReasonV1),
    NotExecuted,
    InfrastructureQueued,
    InfrastructureFailure,
    Superseded,
    ProfileMismatch,
    EvidenceStale,
    Underpowered,
    ProbeInvalid,
}

impl GateStatusV1 {
    const fn tag(self) -> u8 {
        match self {
            Self::Pass => 1,
            Self::Fail => 2,
            Self::LeakDetected => 3,
            Self::ManipulationDetected => 4,
            Self::RejectedInvalidInput => 5,
            Self::NotApplicable(_) => 6,
            Self::NotExecuted => 7,
            Self::InfrastructureQueued => 8,
            Self::InfrastructureFailure => 9,
            Self::Superseded => 10,
            Self::ProfileMismatch => 11,
            Self::EvidenceStale => 12,
            Self::Underpowered => 13,
            Self::ProbeInvalid => 14,
        }
    }

    fn write(self, out: &mut Vec<u8>) {
        out.push(self.tag());
        if let Self::NotApplicable(reason) = self {
            out.push(reason.tag());
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RequirementPolicyV1 {
    Pass,
    PassOrNotApplicable(NotApplicableReasonV1),
}

impl RequirementPolicyV1 {
    fn write(self, out: &mut Vec<u8>) {
        match self {
            Self::Pass => out.push(1),
            Self::PassOrNotApplicable(reason) => {
                out.push(2);
                out.push(reason.tag());
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GateRequirementV1 {
    gate: GateIdV1,
    policy: RequirementPolicyV1,
    expected_subject: Option<Id32>,
    expected_verifier: Option<Id32>,
}

impl GateRequirementV1 {
    fn write(&self, out: &mut Vec<u8>) {
        out.push(self.gate.tag());
        self.policy.write(out);
        write_optional_id(self.expected_subject, out);
        write_optional_id(self.expected_verifier, out);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct CompatibilityBindingsV1 {
    benchmark_profile: Id32,
    sensor_profile: Id32,
    producer_profile: Id32,
    scorer_profile: Id32,
    scenario_generator: Id32,
    environment_class: Id32,
    experiment_plan: Id32,
    challenge_commitment: Id32,
    population: Id32,
}

impl CompatibilityBindingsV1 {
    fn write(&self, out: &mut Vec<u8>) {
        self.benchmark_profile.write(out);
        self.sensor_profile.write(out);
        self.producer_profile.write(out);
        self.scorer_profile.write(out);
        self.scenario_generator.write(out);
        self.environment_class.write(out);
        self.experiment_plan.write(out);
        self.challenge_commitment.write(out);
        self.population.write(out);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BenchmarkValidityProfileV1 {
    profile_id: Id32,
    claim_class: ClaimClassV1,
    bindings: CompatibilityBindingsV1,
    required_challenge_exposure: Option<ChallengeExposureV1>,
    requirements: Vec<GateRequirementV1>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BuildErrorV1 {
    EmptyRequirements,
    DuplicateRequirement(GateIdV1),
    DuplicateEvidence(GateIdV1),
    ChallengeExposureWithoutCore008,
}

impl BenchmarkValidityProfileV1 {
    fn new(
        profile_id: Id32,
        claim_class: ClaimClassV1,
        bindings: CompatibilityBindingsV1,
        required_challenge_exposure: Option<ChallengeExposureV1>,
        mut requirements: Vec<GateRequirementV1>,
    ) -> Result<Self, BuildErrorV1> {
        if requirements.is_empty() {
            return Err(BuildErrorV1::EmptyRequirements);
        }

        requirements.sort_by_key(|requirement| requirement.gate.tag());
        for pair in requirements.windows(2) {
            if pair[0].gate == pair[1].gate {
                return Err(BuildErrorV1::DuplicateRequirement(pair[0].gate));
            }
        }

        if required_challenge_exposure.is_some()
            && !requirements
                .iter()
                .any(|requirement| requirement.gate == GateIdV1::Core008)
        {
            return Err(BuildErrorV1::ChallengeExposureWithoutCore008);
        }

        Ok(Self {
            profile_id,
            claim_class,
            bindings,
            required_challenge_exposure,
            requirements,
        })
    }

    fn canonical_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(PROFILE_DOMAIN);
        self.profile_id.write(&mut out);
        out.push(self.claim_class.tag());
        self.bindings.write(&mut out);
        write_optional_exposure(self.required_challenge_exposure, &mut out);
        out.extend_from_slice(
            &u16::try_from(self.requirements.len())
                .expect("gate requirement count is bounded")
                .to_be_bytes(),
        );
        for requirement in &self.requirements {
            requirement.write(&mut out);
        }
        out
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GateEvidenceV1 {
    gate: GateIdV1,
    status: GateStatusV1,
    evidence_id: Id32,
    subject_id: Id32,
    verifier_id: Id32,
    bindings: CompatibilityBindingsV1,
    challenge_exposure: Option<ChallengeExposureV1>,
}

impl GateEvidenceV1 {
    fn write(&self, out: &mut Vec<u8>) {
        out.push(self.gate.tag());
        self.status.write(out);
        self.evidence_id.write(out);
        self.subject_id.write(out);
        self.verifier_id.write(out);
        self.bindings.write(out);
        write_optional_exposure(self.challenge_exposure, out);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ViolationV1 {
    MissingGate(GateIdV1),
    UnexpectedGate(GateIdV1),
    BindingMismatch(GateIdV1),
    SubjectMismatch(GateIdV1),
    VerifierMismatch(GateIdV1),
    UnexpectedChallengeExposure(GateIdV1),
    ChallengeExposureMismatch,
    NotApplicableReasonMismatch(GateIdV1),
    GateBlocked(GateIdV1, GateStatusV1),
}

impl ViolationV1 {
    fn write(self, out: &mut Vec<u8>) {
        match self {
            Self::MissingGate(gate) => write_gate_violation(1, gate, out),
            Self::UnexpectedGate(gate) => write_gate_violation(2, gate, out),
            Self::BindingMismatch(gate) => write_gate_violation(3, gate, out),
            Self::SubjectMismatch(gate) => write_gate_violation(4, gate, out),
            Self::VerifierMismatch(gate) => write_gate_violation(5, gate, out),
            Self::UnexpectedChallengeExposure(gate) => write_gate_violation(6, gate, out),
            Self::ChallengeExposureMismatch => out.push(7),
            Self::NotApplicableReasonMismatch(gate) => write_gate_violation(8, gate, out),
            Self::GateBlocked(gate, status) => {
                out.push(9);
                out.push(gate.tag());
                status.write(out);
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AdmissionStateV1 {
    ConfirmatoryAdmitted,
    BlockedInvalidBenchmark,
    BlockedIncompleteValidity,
    BlockedProfileMismatch,
    BlockedInfrastructure,
    BlockedStaleEvidence,
}

impl AdmissionStateV1 {
    const fn tag(self) -> u8 {
        match self {
            Self::ConfirmatoryAdmitted => 1,
            Self::BlockedInvalidBenchmark => 2,
            Self::BlockedIncompleteValidity => 3,
            Self::BlockedProfileMismatch => 4,
            Self::BlockedInfrastructure => 5,
            Self::BlockedStaleEvidence => 6,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PerformanceAvailabilityV1 {
    Confirmatory,
    DiagnosticOnly,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BenchmarkValidityReceiptV1 {
    profile: BenchmarkValidityProfileV1,
    evidence: Vec<GateEvidenceV1>,
    violations: Vec<ViolationV1>,
    admission: AdmissionStateV1,
}

impl BenchmarkValidityReceiptV1 {
    fn evaluate(
        profile: BenchmarkValidityProfileV1,
        evidence: Vec<GateEvidenceV1>,
    ) -> Result<Self, BuildErrorV1> {
        let mut evidence_by_gate = BTreeMap::new();
        for item in evidence {
            let gate = item.gate;
            if evidence_by_gate.insert(gate, item).is_some() {
                return Err(BuildErrorV1::DuplicateEvidence(gate));
            }
        }

        let requirement_by_gate: BTreeMap<_, _> = profile
            .requirements
            .iter()
            .map(|requirement| (requirement.gate, requirement))
            .collect();
        let mut violations = Vec::new();

        for gate in evidence_by_gate.keys().copied() {
            if !requirement_by_gate.contains_key(&gate) {
                violations.push(ViolationV1::UnexpectedGate(gate));
            }
        }

        for requirement in &profile.requirements {
            let Some(item) = evidence_by_gate.get(&requirement.gate) else {
                violations.push(ViolationV1::MissingGate(requirement.gate));
                continue;
            };

            if item.bindings != profile.bindings {
                violations.push(ViolationV1::BindingMismatch(requirement.gate));
            }
            if requirement
                .expected_subject
                .is_some_and(|expected| expected != item.subject_id)
            {
                violations.push(ViolationV1::SubjectMismatch(requirement.gate));
            }
            if requirement
                .expected_verifier
                .is_some_and(|expected| expected != item.verifier_id)
            {
                violations.push(ViolationV1::VerifierMismatch(requirement.gate));
            }

            if requirement.gate == GateIdV1::Core008 {
                if item.challenge_exposure != profile.required_challenge_exposure {
                    violations.push(ViolationV1::ChallengeExposureMismatch);
                }
            } else if item.challenge_exposure.is_some() {
                violations.push(ViolationV1::UnexpectedChallengeExposure(requirement.gate));
            }

            match (requirement.policy, item.status) {
                (RequirementPolicyV1::Pass, GateStatusV1::Pass)
                | (RequirementPolicyV1::PassOrNotApplicable(_), GateStatusV1::Pass) => {}
                (
                    RequirementPolicyV1::PassOrNotApplicable(expected),
                    GateStatusV1::NotApplicable(actual),
                ) if expected == actual => {}
                (
                    RequirementPolicyV1::PassOrNotApplicable(_),
                    GateStatusV1::NotApplicable(_),
                ) => violations.push(ViolationV1::NotApplicableReasonMismatch(requirement.gate)),
                (_, status) => violations.push(ViolationV1::GateBlocked(requirement.gate, status)),
            }
        }

        let admission = classify_admission(&violations);
        Ok(Self {
            profile,
            evidence: evidence_by_gate.into_values().collect(),
            violations,
            admission,
        })
    }

    const fn performance_availability(&self) -> PerformanceAvailabilityV1 {
        match self.admission {
            AdmissionStateV1::ConfirmatoryAdmitted => PerformanceAvailabilityV1::Confirmatory,
            _ => PerformanceAvailabilityV1::DiagnosticOnly,
        }
    }

    fn canonical_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        out.extend_from_slice(RECEIPT_DOMAIN);
        let profile_bytes = self.profile.canonical_bytes();
        out.extend_from_slice(
            &u32::try_from(profile_bytes.len())
                .expect("reference profile is bounded")
                .to_be_bytes(),
        );
        out.extend_from_slice(&profile_bytes);
        out.push(self.admission.tag());
        out.extend_from_slice(
            &u16::try_from(self.evidence.len())
                .expect("gate evidence count is bounded")
                .to_be_bytes(),
        );
        for item in &self.evidence {
            item.write(&mut out);
        }
        out.extend_from_slice(
            &u16::try_from(self.violations.len())
                .expect("violation count is bounded")
                .to_be_bytes(),
        );
        for violation in &self.violations {
            violation.write(&mut out);
        }
        out
    }
}

fn classify_admission(violations: &[ViolationV1]) -> AdmissionStateV1 {
    if violations.is_empty() {
        return AdmissionStateV1::ConfirmatoryAdmitted;
    }

    let mut profile_mismatch = false;
    let mut stale = false;
    let mut invalid = false;
    let mut infrastructure = false;

    for violation in violations {
        match *violation {
            ViolationV1::UnexpectedGate(_)
            | ViolationV1::BindingMismatch(_)
            | ViolationV1::SubjectMismatch(_)
            | ViolationV1::VerifierMismatch(_)
            | ViolationV1::UnexpectedChallengeExposure(_)
            | ViolationV1::ChallengeExposureMismatch
            | ViolationV1::NotApplicableReasonMismatch(_)
            | ViolationV1::GateBlocked(_, GateStatusV1::ProfileMismatch)
            | ViolationV1::GateBlocked(_, GateStatusV1::NotApplicable(_)) => {
                profile_mismatch = true;
            }
            ViolationV1::GateBlocked(
                _,
                GateStatusV1::Superseded | GateStatusV1::EvidenceStale,
            ) => stale = true,
            ViolationV1::GateBlocked(
                _,
                GateStatusV1::Fail
                | GateStatusV1::LeakDetected
                | GateStatusV1::ManipulationDetected
                | GateStatusV1::RejectedInvalidInput
                | GateStatusV1::ProbeInvalid,
            ) => invalid = true,
            ViolationV1::GateBlocked(
                _,
                GateStatusV1::InfrastructureQueued | GateStatusV1::InfrastructureFailure,
            ) => infrastructure = true,
            ViolationV1::MissingGate(_)
            | ViolationV1::GateBlocked(
                _,
                GateStatusV1::NotExecuted | GateStatusV1::Underpowered,
            ) => {}
            ViolationV1::GateBlocked(_, GateStatusV1::Pass) => {
                unreachable!("pass evidence cannot produce a blocked gate")
            }
        }
    }

    if profile_mismatch {
        AdmissionStateV1::BlockedProfileMismatch
    } else if stale {
        AdmissionStateV1::BlockedStaleEvidence
    } else if invalid {
        AdmissionStateV1::BlockedInvalidBenchmark
    } else if infrastructure {
        AdmissionStateV1::BlockedInfrastructure
    } else {
        AdmissionStateV1::BlockedIncompleteValidity
    }
}

fn write_gate_violation(tag: u8, gate: GateIdV1, out: &mut Vec<u8>) {
    out.push(tag);
    out.push(gate.tag());
}

fn write_optional_id(value: Option<Id32>, out: &mut Vec<u8>) {
    match value {
        Some(value) => {
            out.push(1);
            value.write(out);
        }
        None => out.push(0),
    }
}

fn write_optional_exposure(value: Option<ChallengeExposureV1>, out: &mut Vec<u8>) {
    match value {
        Some(value) => {
            out.push(1);
            out.push(value.tag());
        }
        None => out.push(0),
    }
}

fn bindings(seed: u8) -> CompatibilityBindingsV1 {
    CompatibilityBindingsV1 {
        benchmark_profile: Id32::repeated(seed),
        sensor_profile: Id32::repeated(seed + 1),
        producer_profile: Id32::repeated(seed + 2),
        scorer_profile: Id32::repeated(seed + 3),
        scenario_generator: Id32::repeated(seed + 4),
        environment_class: Id32::repeated(seed + 5),
        experiment_plan: Id32::repeated(seed + 6),
        challenge_commitment: Id32::repeated(seed + 7),
        population: Id32::repeated(seed + 8),
    }
}

fn requirement(gate: GateIdV1) -> GateRequirementV1 {
    GateRequirementV1 {
        gate,
        policy: RequirementPolicyV1::Pass,
        expected_subject: Some(Id32::repeated(40 + gate.tag())),
        expected_verifier: Some(Id32::repeated(100 + gate.tag())),
    }
}

fn full_profile() -> BenchmarkValidityProfileV1 {
    BenchmarkValidityProfileV1::new(
        Id32::repeated(9),
        ClaimClassV1::IntegratedBeliefOnlineBenchmark,
        bindings(10),
        Some(ChallengeExposureV1::FreshHidden),
        vec![
            requirement(GateIdV1::A3),
            requirement(GateIdV1::A5),
            requirement(GateIdV1::Core003),
            requirement(GateIdV1::Core008),
            requirement(GateIdV1::Q1),
            requirement(GateIdV1::Q1S),
            requirement(GateIdV1::Q2),
            requirement(GateIdV1::Q3),
            requirement(GateIdV1::Q4),
        ],
    )
    .expect("reference profile is valid")
}

fn synthetic_profile() -> BenchmarkValidityProfileV1 {
    BenchmarkValidityProfileV1::new(
        Id32::repeated(8),
        ClaimClassV1::SyntheticScorerReference,
        bindings(20),
        None,
        vec![requirement(GateIdV1::Q2), requirement(GateIdV1::Q4)],
    )
    .expect("synthetic reference profile is valid")
}

fn passing_evidence(profile: &BenchmarkValidityProfileV1) -> Vec<GateEvidenceV1> {
    profile
        .requirements
        .iter()
        .enumerate()
        .map(|(index, requirement)| GateEvidenceV1 {
            gate: requirement.gate,
            status: GateStatusV1::Pass,
            evidence_id: Id32::repeated(180 + u8::try_from(index).expect("small fixture")),
            subject_id: requirement
                .expected_subject
                .expect("reference requirement binds subject"),
            verifier_id: requirement
                .expected_verifier
                .expect("reference requirement binds verifier"),
            bindings: profile.bindings.clone(),
            challenge_exposure: if requirement.gate == GateIdV1::Core008 {
                profile.required_challenge_exposure
            } else {
                None
            },
        })
        .collect()
}

fn evidence_mut(evidence: &mut [GateEvidenceV1], gate: GateIdV1) -> &mut GateEvidenceV1 {
    evidence
        .iter_mut()
        .find(|item| item.gate == gate)
        .expect("fixture gate must exist")
}

#[test]
fn q5_narrow_profile_admits_only_its_precommitted_claim_class() {
    let profile = synthetic_profile();
    let receipt = BenchmarkValidityReceiptV1::evaluate(
        profile.clone(),
        passing_evidence(&profile),
    )
    .unwrap();
    assert_eq!(receipt.admission, AdmissionStateV1::ConfirmatoryAdmitted);
    assert_eq!(
        receipt.performance_availability(),
        PerformanceAvailabilityV1::Confirmatory
    );
}

#[test]
fn q5_profile_bytes_are_requirement_order_independent() {
    let first = BenchmarkValidityProfileV1::new(
        Id32::repeated(30),
        ClaimClassV1::SyntheticScorerReference,
        bindings(31),
        None,
        vec![requirement(GateIdV1::Q2), requirement(GateIdV1::Q4)],
    )
    .unwrap();
    let second = BenchmarkValidityProfileV1::new(
        Id32::repeated(30),
        ClaimClassV1::SyntheticScorerReference,
        bindings(31),
        None,
        vec![requirement(GateIdV1::Q4), requirement(GateIdV1::Q2)],
    )
    .unwrap();
    assert_eq!(first.canonical_bytes(), second.canonical_bytes());
}

#[test]
fn q5_receipt_bytes_are_evidence_order_independent() {
    let profile = full_profile();
    let forward = passing_evidence(&profile);
    let mut reverse = forward.clone();
    reverse.reverse();
    let first = BenchmarkValidityReceiptV1::evaluate(profile.clone(), forward).unwrap();
    let second = BenchmarkValidityReceiptV1::evaluate(profile, reverse).unwrap();
    assert_eq!(first, second);
    assert_eq!(first.canonical_bytes(), second.canonical_bytes());
}

#[test]
fn q5_missing_not_executed_and_underpowered_never_become_pass() {
    let profile = full_profile();
    let mut missing = passing_evidence(&profile);
    missing.retain(|item| item.gate != GateIdV1::Q3);
    assert_eq!(
        BenchmarkValidityReceiptV1::evaluate(profile.clone(), missing)
            .unwrap()
            .admission,
        AdmissionStateV1::BlockedIncompleteValidity
    );

    for status in [GateStatusV1::NotExecuted, GateStatusV1::Underpowered] {
        let mut evidence = passing_evidence(&profile);
        evidence_mut(&mut evidence, GateIdV1::Q1S).status = status;
        assert_eq!(
            BenchmarkValidityReceiptV1::evaluate(profile.clone(), evidence)
                .unwrap()
                .admission,
            AdmissionStateV1::BlockedIncompleteValidity
        );
    }
}

#[test]
fn q5_queued_or_failed_infrastructure_is_not_pass() {
    for status in [
        GateStatusV1::InfrastructureQueued,
        GateStatusV1::InfrastructureFailure,
    ] {
        let profile = full_profile();
        let mut evidence = passing_evidence(&profile);
        evidence_mut(&mut evidence, GateIdV1::Q2).status = status;
        let receipt = BenchmarkValidityReceiptV1::evaluate(profile, evidence).unwrap();
        assert_eq!(receipt.admission, AdmissionStateV1::BlockedInfrastructure);
        assert_eq!(
            receipt.performance_availability(),
            PerformanceAvailabilityV1::DiagnosticOnly
        );
    }
}

#[test]
fn q5_stale_or_superseded_evidence_is_not_current_evidence() {
    for status in [GateStatusV1::Superseded, GateStatusV1::EvidenceStale] {
        let profile = full_profile();
        let mut evidence = passing_evidence(&profile);
        evidence_mut(&mut evidence, GateIdV1::Q3).status = status;
        assert_eq!(
            BenchmarkValidityReceiptV1::evaluate(profile, evidence)
                .unwrap()
                .admission,
            AdmissionStateV1::BlockedStaleEvidence
        );
    }
}

#[test]
fn q5_exact_leak_dominates_statistical_pass() {
    let profile = full_profile();
    let mut evidence = passing_evidence(&profile);
    evidence_mut(&mut evidence, GateIdV1::Q1).status = GateStatusV1::LeakDetected;
    evidence_mut(&mut evidence, GateIdV1::Q1S).status = GateStatusV1::Pass;
    assert_eq!(
        BenchmarkValidityReceiptV1::evaluate(profile, evidence)
            .unwrap()
            .admission,
        AdmissionStateV1::BlockedInvalidBenchmark
    );
}

#[test]
fn q5_probe_invalid_blocks_reassuring_clean_statistics() {
    let profile = full_profile();
    let mut evidence = passing_evidence(&profile);
    evidence_mut(&mut evidence, GateIdV1::Q1S).status = GateStatusV1::ProbeInvalid;
    assert_eq!(
        BenchmarkValidityReceiptV1::evaluate(profile, evidence)
            .unwrap()
            .admission,
        AdmissionStateV1::BlockedInvalidBenchmark
    );
}

#[test]
fn q5_binding_population_subject_and_verifier_mismatches_fail_closed() {
    for mutation in 0..4 {
        let profile = full_profile();
        let mut evidence = passing_evidence(&profile);
        let item = evidence_mut(&mut evidence, GateIdV1::Q2);
        match mutation {
            0 => item.bindings.sensor_profile = Id32::repeated(220),
            1 => item.bindings.population = Id32::repeated(221),
            2 => item.subject_id = Id32::repeated(222),
            3 => item.verifier_id = Id32::repeated(223),
            _ => unreachable!(),
        }
        assert_eq!(
            BenchmarkValidityReceiptV1::evaluate(profile, evidence)
                .unwrap()
                .admission,
            AdmissionStateV1::BlockedProfileMismatch
        );
    }
}

#[test]
fn q5_fresh_hidden_cannot_be_replaced_by_previously_revealed() {
    let profile = full_profile();
    let mut evidence = passing_evidence(&profile);
    evidence_mut(&mut evidence, GateIdV1::Core008).challenge_exposure =
        Some(ChallengeExposureV1::PreviouslyRevealed);
    assert_eq!(
        BenchmarkValidityReceiptV1::evaluate(profile, evidence)
            .unwrap()
            .admission,
        AdmissionStateV1::BlockedProfileMismatch
    );
}

#[test]
fn q5_not_applicable_requires_the_exact_precommitted_reason() {
    let requirement = GateRequirementV1 {
        gate: GateIdV1::Q1S,
        policy: RequirementPolicyV1::PassOrNotApplicable(
            NotApplicableReasonV1::NoStatisticalProbeBecauseExactEqualityRequired,
        ),
        expected_subject: Some(Id32::repeated(56)),
        expected_verifier: Some(Id32::repeated(116)),
    };
    let profile = BenchmarkValidityProfileV1::new(
        Id32::repeated(32),
        ClaimClassV1::SyntheticScorerReference,
        bindings(33),
        None,
        vec![requirement],
    )
    .unwrap();
    let base = GateEvidenceV1 {
        gate: GateIdV1::Q1S,
        status: GateStatusV1::NotApplicable(
            NotApplicableReasonV1::NoStatisticalProbeBecauseExactEqualityRequired,
        ),
        evidence_id: Id32::repeated(190),
        subject_id: Id32::repeated(56),
        verifier_id: Id32::repeated(116),
        bindings: profile.bindings.clone(),
        challenge_exposure: None,
    };
    assert_eq!(
        BenchmarkValidityReceiptV1::evaluate(profile.clone(), vec![base.clone()])
            .unwrap()
            .admission,
        AdmissionStateV1::ConfirmatoryAdmitted
    );

    let mut wrong = base;
    wrong.status = GateStatusV1::NotApplicable(
        NotApplicableReasonV1::NoRendererInSyntheticProfile,
    );
    assert_eq!(
        BenchmarkValidityReceiptV1::evaluate(profile, vec![wrong])
            .unwrap()
            .admission,
        AdmissionStateV1::BlockedProfileMismatch
    );
}

#[test]
fn q5_not_applicable_cannot_replace_required_pass() {
    let profile = synthetic_profile();
    let mut evidence = passing_evidence(&profile);
    evidence_mut(&mut evidence, GateIdV1::Q2).status =
        GateStatusV1::NotApplicable(NotApplicableReasonV1::NoRendererInSyntheticProfile);
    assert_eq!(
        BenchmarkValidityReceiptV1::evaluate(profile, evidence)
            .unwrap()
            .admission,
        AdmissionStateV1::BlockedProfileMismatch
    );
}

#[test]
fn q5_duplicate_and_unexpected_gate_evidence_are_rejected_or_blocked() {
    let profile = synthetic_profile();
    let mut duplicate = passing_evidence(&profile);
    duplicate.push(duplicate[0].clone());
    assert!(matches!(
        BenchmarkValidityReceiptV1::evaluate(profile.clone(), duplicate),
        Err(BuildErrorV1::DuplicateEvidence(_))
    ));

    let mut unexpected = passing_evidence(&profile);
    let mut extra = unexpected[0].clone();
    extra.gate = GateIdV1::A3;
    extra.evidence_id = Id32::repeated(191);
    unexpected.push(extra);
    assert_eq!(
        BenchmarkValidityReceiptV1::evaluate(profile, unexpected)
            .unwrap()
            .admission,
        AdmissionStateV1::BlockedProfileMismatch
    );
}

#[test]
fn q5_profile_rejects_ambiguous_requirement_contracts() {
    assert!(matches!(
        BenchmarkValidityProfileV1::new(
            Id32::repeated(34),
            ClaimClassV1::SyntheticScorerReference,
            bindings(35),
            None,
            Vec::new(),
        ),
        Err(BuildErrorV1::EmptyRequirements)
    ));
    assert!(matches!(
        BenchmarkValidityProfileV1::new(
            Id32::repeated(36),
            ClaimClassV1::SyntheticScorerReference,
            bindings(37),
            None,
            vec![requirement(GateIdV1::Q2), requirement(GateIdV1::Q2)],
        ),
        Err(BuildErrorV1::DuplicateRequirement(GateIdV1::Q2))
    ));
    assert!(matches!(
        BenchmarkValidityProfileV1::new(
            Id32::repeated(38),
            ClaimClassV1::SyntheticScorerReference,
            bindings(39),
            Some(ChallengeExposureV1::FreshHidden),
            vec![requirement(GateIdV1::Q2)],
        ),
        Err(BuildErrorV1::ChallengeExposureWithoutCore008)
    ));
}

#[test]
fn q5_reference_document_preserves_non_authority_boundary() {
    assert!(PROFILE_DOC.contains(r#""schema_id": "sym-eval.benchmark-validity-reference.v1""#));
    assert!(PROFILE_DOC.contains(r#""implementation_parent_sha": "1d639b2a0bf4856e2ae59ddf72c1799268814857""#));
    assert!(PROFILE_DOC.contains(r#""product_authority": "NotEstablished""#));
    assert!(PROFILE_DOC.contains(r#""lower_gate_execution": "NotInherited""#));
    assert!(PROFILE_DOC.contains(r#""performance_claim": "NotEvaluated""#));
}
