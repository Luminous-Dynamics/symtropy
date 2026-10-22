// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Reference-only canonical benchmark-validity admission mechanics for CORE-009A.
//!
//! This is intentionally an integration-test reference. It does not export
//! production authority and does not depend on the still-unqualified CORE-001
//! implementation module.

#![allow(dead_code)]

use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

const PROFILE_DOMAIN: &str = "sym-eval.validity-profile.v1";
const EVIDENCE_DOMAIN: &str = "sym-eval.validity-gate-evidence.v1";
const RECEIPT_DOMAIN: &str = "sym-eval.validity-receipt.v1";
const REFERENCE_DOC: &str =
    include_str!("../../../../docs/research/SYM_EVAL_CORE_009A_VALIDITY_REFERENCE.md");

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Digest32([u8; 32]);

impl Digest32 {
    const ZERO: Self = Self([0; 32]);

    const fn repeated(byte: u8) -> Self {
        Self([byte; 32])
    }

    fn write(self, out: &mut Vec<u8>) {
        out.extend_from_slice(&self.0);
    }

    fn to_hex(self) -> String {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let mut out = String::with_capacity(64);
        for byte in self.0 {
            out.push(char::from(HEX[usize::from(byte >> 4)]));
            out.push(char::from(HEX[usize::from(byte & 0x0f)]));
        }
        out
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct SchemaBindingV1 {
    schema_id: String,
    schema_version: u32,
}

impl SchemaBindingV1 {
    fn new(schema_id: &str, schema_version: u32) -> Self {
        assert!(!schema_id.is_empty());
        assert!(schema_version > 0);
        Self {
            schema_id: schema_id.to_owned(),
            schema_version,
        }
    }

    fn write(&self, out: &mut Vec<u8>) {
        write_string(&self.schema_id, out);
        out.extend_from_slice(&self.schema_version.to_be_bytes());
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum RequirementPolicyV1 {
    Pass,
    PassOrNotApplicable(String),
}

impl RequirementPolicyV1 {
    fn write(&self, out: &mut Vec<u8>) {
        match self {
            Self::Pass => out.push(1),
            Self::PassOrNotApplicable(reason) => {
                out.push(2);
                write_string(reason, out);
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct GateRequirementV1 {
    gate_key: String,
    policy: RequirementPolicyV1,
    expected_subject: Option<Digest32>,
    expected_verifier: Option<Digest32>,
}

impl GateRequirementV1 {
    fn pass(gate_key: &str, subject: u8, verifier: u8) -> Self {
        Self {
            gate_key: gate_key.to_owned(),
            policy: RequirementPolicyV1::Pass,
            expected_subject: Some(Digest32::repeated(subject)),
            expected_verifier: Some(Digest32::repeated(verifier)),
        }
    }

    fn pass_or_na(gate_key: &str, reason: &str, subject: u8, verifier: u8) -> Self {
        Self {
            gate_key: gate_key.to_owned(),
            policy: RequirementPolicyV1::PassOrNotApplicable(reason.to_owned()),
            expected_subject: Some(Digest32::repeated(subject)),
            expected_verifier: Some(Digest32::repeated(verifier)),
        }
    }

    fn write(&self, out: &mut Vec<u8>) {
        write_string(&self.gate_key, out);
        self.policy.write(out);
        write_optional_digest(self.expected_subject, out);
        write_optional_digest(self.expected_verifier, out);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct BindingRequirementV1 {
    key: String,
    expected: Digest32,
}

impl BindingRequirementV1 {
    fn new(key: &str, expected: Digest32) -> Self {
        Self {
            key: key.to_owned(),
            expected,
        }
    }

    fn write(&self, out: &mut Vec<u8>) {
        write_string(&self.key, out);
        self.expected.write(out);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ValidityProfileV1 {
    display_label: String,
    claim_schema: SchemaBindingV1,
    admission_policy_version: u32,
    bindings: Vec<BindingRequirementV1>,
    requirements: Vec<GateRequirementV1>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum BuildErrorV1 {
    EmptyRequirements,
    DuplicateRequirement(String),
    DuplicateBindingRequirement(String),
    DuplicateEvidence(String),
    DuplicateEvidenceBinding {
        gate_key: String,
        binding_key: String,
    },
}

impl ValidityProfileV1 {
    fn new(
        display_label: &str,
        claim_schema: SchemaBindingV1,
        admission_policy_version: u32,
        mut bindings: Vec<BindingRequirementV1>,
        mut requirements: Vec<GateRequirementV1>,
    ) -> Result<Self, BuildErrorV1> {
        if requirements.is_empty() {
            return Err(BuildErrorV1::EmptyRequirements);
        }
        assert!(admission_policy_version > 0);

        bindings.sort();
        for pair in bindings.windows(2) {
            if pair[0].key == pair[1].key {
                return Err(BuildErrorV1::DuplicateBindingRequirement(
                    pair[0].key.clone(),
                ));
            }
        }

        requirements.sort();
        for pair in requirements.windows(2) {
            if pair[0].gate_key == pair[1].gate_key {
                return Err(BuildErrorV1::DuplicateRequirement(pair[0].gate_key.clone()));
            }
        }

        Ok(Self {
            display_label: display_label.to_owned(),
            claim_schema,
            admission_policy_version,
            bindings,
            requirements,
        })
    }

    fn canonical_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        self.claim_schema.write(&mut out);
        out.extend_from_slice(&self.admission_policy_version.to_be_bytes());
        write_len(self.bindings.len(), &mut out);
        for binding in &self.bindings {
            binding.write(&mut out);
        }
        write_len(self.requirements.len(), &mut out);
        for requirement in &self.requirements {
            requirement.write(&mut out);
        }
        out
    }

    fn digest(&self) -> Digest32 {
        domain_hash(PROFILE_DOMAIN, &self.canonical_bytes())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum GateStatusV1 {
    Pass,
    Fail,
    Invalid,
    NotApplicable(String),
    NotExecuted,
    InfrastructureQueued,
    InfrastructureFailure,
    Superseded,
    EvidenceStale,
    Underpowered,
    ProfileMismatch,
}

impl GateStatusV1 {
    fn write(&self, out: &mut Vec<u8>) {
        match self {
            Self::Pass => out.push(1),
            Self::Fail => out.push(2),
            Self::Invalid => out.push(3),
            Self::NotApplicable(reason) => {
                out.push(4);
                write_string(reason, out);
            }
            Self::NotExecuted => out.push(5),
            Self::InfrastructureQueued => out.push(6),
            Self::InfrastructureFailure => out.push(7),
            Self::Superseded => out.push(8),
            Self::EvidenceStale => out.push(9),
            Self::Underpowered => out.push(10),
            Self::ProfileMismatch => out.push(11),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct EvidenceBindingV1 {
    key: String,
    value: Digest32,
}

impl EvidenceBindingV1 {
    fn new(key: &str, value: Digest32) -> Self {
        Self {
            key: key.to_owned(),
            value,
        }
    }

    fn write(&self, out: &mut Vec<u8>) {
        write_string(&self.key, out);
        self.value.write(out);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct GateEvidenceV1 {
    gate_key: String,
    status: GateStatusV1,
    evidence_ref: Digest32,
    subject_ref: Option<Digest32>,
    verifier_ref: Option<Digest32>,
    bindings: Vec<EvidenceBindingV1>,
}

impl GateEvidenceV1 {
    fn new(
        gate_key: &str,
        status: GateStatusV1,
        evidence_ref: Digest32,
        subject_ref: Option<Digest32>,
        verifier_ref: Option<Digest32>,
        mut bindings: Vec<EvidenceBindingV1>,
    ) -> Result<Self, BuildErrorV1> {
        bindings.sort();
        for pair in bindings.windows(2) {
            if pair[0].key == pair[1].key {
                return Err(BuildErrorV1::DuplicateEvidenceBinding {
                    gate_key: gate_key.to_owned(),
                    binding_key: pair[0].key.clone(),
                });
            }
        }

        Ok(Self {
            gate_key: gate_key.to_owned(),
            status,
            evidence_ref,
            subject_ref,
            verifier_ref,
            bindings,
        })
    }

    fn canonical_bytes(&self) -> Vec<u8> {
        let mut out = Vec::new();
        write_string(&self.gate_key, &mut out);
        self.status.write(&mut out);
        self.evidence_ref.write(&mut out);
        write_optional_digest(self.subject_ref, &mut out);
        write_optional_digest(self.verifier_ref, &mut out);
        write_len(self.bindings.len(), &mut out);
        for binding in &self.bindings {
            binding.write(&mut out);
        }
        out
    }

    fn digest(&self) -> Digest32 {
        domain_hash(EVIDENCE_DOMAIN, &self.canonical_bytes())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum ViolationV1 {
    MissingGate(String),
    UnexpectedGate(String),
    MissingBinding {
        gate_key: String,
        binding_key: String,
    },
    UnexpectedBinding {
        gate_key: String,
        binding_key: String,
    },
    BindingMismatch {
        gate_key: String,
        binding_key: String,
    },
    SubjectMismatch(String),
    VerifierMismatch(String),
    MissingEvidenceReference(String),
    NotApplicableReasonMismatch(String),
    GateBlocked {
        gate_key: String,
        status: GateStatusV1,
    },
}

impl ViolationV1 {
    fn write(&self, out: &mut Vec<u8>) {
        match self {
            Self::MissingGate(gate_key) => {
                out.push(1);
                write_string(gate_key, out);
            }
            Self::UnexpectedGate(gate_key) => {
                out.push(2);
                write_string(gate_key, out);
            }
            Self::MissingBinding {
                gate_key,
                binding_key,
            } => {
                out.push(3);
                write_string(gate_key, out);
                write_string(binding_key, out);
            }
            Self::UnexpectedBinding {
                gate_key,
                binding_key,
            } => {
                out.push(4);
                write_string(gate_key, out);
                write_string(binding_key, out);
            }
            Self::BindingMismatch {
                gate_key,
                binding_key,
            } => {
                out.push(5);
                write_string(gate_key, out);
                write_string(binding_key, out);
            }
            Self::SubjectMismatch(gate_key) => {
                out.push(6);
                write_string(gate_key, out);
            }
            Self::VerifierMismatch(gate_key) => {
                out.push(7);
                write_string(gate_key, out);
            }
            Self::MissingEvidenceReference(gate_key) => {
                out.push(8);
                write_string(gate_key, out);
            }
            Self::NotApplicableReasonMismatch(gate_key) => {
                out.push(9);
                write_string(gate_key, out);
            }
            Self::GateBlocked { gate_key, status } => {
                out.push(10);
                write_string(gate_key, out);
                status.write(out);
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AdmissionStateV1 {
    ConfirmatoryAdmitted,
    BlockedInvalidBenchmark,
    BlockedProfileMismatch,
    BlockedStaleEvidence,
    BlockedInfrastructure,
    BlockedIncompleteValidity,
}

impl AdmissionStateV1 {
    const fn tag(self) -> u8 {
        match self {
            Self::ConfirmatoryAdmitted => 1,
            Self::BlockedInvalidBenchmark => 2,
            Self::BlockedProfileMismatch => 3,
            Self::BlockedStaleEvidence => 4,
            Self::BlockedInfrastructure => 5,
            Self::BlockedIncompleteValidity => 6,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PerformanceAvailabilityV1 {
    Confirmatory,
    DiagnosticOnly,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ValidityReceiptV1 {
    profile_digest: Digest32,
    evidence: Vec<GateEvidenceV1>,
    violations: Vec<ViolationV1>,
    admission: AdmissionStateV1,
}

impl ValidityReceiptV1 {
    fn evaluate(
        profile: &ValidityProfileV1,
        evidence: Vec<GateEvidenceV1>,
    ) -> Result<Self, BuildErrorV1> {
        let mut evidence_by_gate = BTreeMap::new();
        for item in evidence {
            let gate_key = item.gate_key.clone();
            if evidence_by_gate.insert(gate_key.clone(), item).is_some() {
                return Err(BuildErrorV1::DuplicateEvidence(gate_key));
            }
        }

        let requirement_by_gate: BTreeMap<_, _> = profile
            .requirements
            .iter()
            .map(|requirement| (requirement.gate_key.as_str(), requirement))
            .collect();
        let expected_bindings: BTreeMap<_, _> = profile
            .bindings
            .iter()
            .map(|binding| (binding.key.as_str(), binding.expected))
            .collect();

        let mut violations = Vec::new();

        for gate_key in evidence_by_gate.keys() {
            if !requirement_by_gate.contains_key(gate_key.as_str()) {
                violations.push(ViolationV1::UnexpectedGate(gate_key.clone()));
            }
        }

        for requirement in &profile.requirements {
            let Some(item) = evidence_by_gate.get(&requirement.gate_key) else {
                violations.push(ViolationV1::MissingGate(requirement.gate_key.clone()));
                continue;
            };

            if item.evidence_ref == Digest32::ZERO {
                violations.push(ViolationV1::MissingEvidenceReference(
                    requirement.gate_key.clone(),
                ));
            }

            if let Some(expected_subject) = requirement.expected_subject
                && item.subject_ref != Some(expected_subject)
            {
                violations.push(ViolationV1::SubjectMismatch(requirement.gate_key.clone()));
            }

            if let Some(expected_verifier) = requirement.expected_verifier
                && item.verifier_ref != Some(expected_verifier)
            {
                violations.push(ViolationV1::VerifierMismatch(requirement.gate_key.clone()));
            }

            let item_bindings: BTreeMap<_, _> = item
                .bindings
                .iter()
                .map(|binding| (binding.key.as_str(), binding.value))
                .collect();

            for (key, expected) in &expected_bindings {
                match item_bindings.get(key) {
                    None => violations.push(ViolationV1::MissingBinding {
                        gate_key: requirement.gate_key.clone(),
                        binding_key: (*key).to_owned(),
                    }),
                    Some(actual) if actual != expected => {
                        violations.push(ViolationV1::BindingMismatch {
                            gate_key: requirement.gate_key.clone(),
                            binding_key: (*key).to_owned(),
                        });
                    }
                    Some(_) => {}
                }
            }

            for key in item_bindings.keys() {
                if !expected_bindings.contains_key(key) {
                    violations.push(ViolationV1::UnexpectedBinding {
                        gate_key: requirement.gate_key.clone(),
                        binding_key: (*key).to_owned(),
                    });
                }
            }

            match (&requirement.policy, &item.status) {
                (RequirementPolicyV1::Pass, GateStatusV1::Pass)
                | (RequirementPolicyV1::PassOrNotApplicable(_), GateStatusV1::Pass) => {}
                (
                    RequirementPolicyV1::PassOrNotApplicable(expected),
                    GateStatusV1::NotApplicable(actual),
                ) if expected == actual => {}
                (RequirementPolicyV1::PassOrNotApplicable(_), GateStatusV1::NotApplicable(_)) => {
                    violations.push(ViolationV1::NotApplicableReasonMismatch(
                        requirement.gate_key.clone(),
                    ))
                }
                (_, status) => violations.push(ViolationV1::GateBlocked {
                    gate_key: requirement.gate_key.clone(),
                    status: status.clone(),
                }),
            }
        }

        violations.sort();
        let admission = classify_admission(&violations);

        Ok(Self {
            profile_digest: profile.digest(),
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
        self.profile_digest.write(&mut out);
        out.push(self.admission.tag());

        write_len(self.evidence.len(), &mut out);
        for item in &self.evidence {
            write_string(&item.gate_key, &mut out);
            item.digest().write(&mut out);
        }

        write_len(self.violations.len(), &mut out);
        for violation in &self.violations {
            violation.write(&mut out);
        }

        out
    }

    fn digest(&self) -> Digest32 {
        domain_hash(RECEIPT_DOMAIN, &self.canonical_bytes())
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
        match violation {
            ViolationV1::UnexpectedGate(_)
            | ViolationV1::MissingBinding { .. }
            | ViolationV1::UnexpectedBinding { .. }
            | ViolationV1::BindingMismatch { .. }
            | ViolationV1::SubjectMismatch(_)
            | ViolationV1::VerifierMismatch(_)
            | ViolationV1::NotApplicableReasonMismatch(_)
            | ViolationV1::GateBlocked {
                status: GateStatusV1::ProfileMismatch | GateStatusV1::NotApplicable(_),
                ..
            } => profile_mismatch = true,
            ViolationV1::GateBlocked {
                status: GateStatusV1::Superseded | GateStatusV1::EvidenceStale,
                ..
            } => stale = true,
            ViolationV1::MissingEvidenceReference(_)
            | ViolationV1::GateBlocked {
                status: GateStatusV1::Fail | GateStatusV1::Invalid,
                ..
            } => invalid = true,
            ViolationV1::GateBlocked {
                status: GateStatusV1::InfrastructureQueued | GateStatusV1::InfrastructureFailure,
                ..
            } => infrastructure = true,
            ViolationV1::MissingGate(_)
            | ViolationV1::GateBlocked {
                status: GateStatusV1::NotExecuted | GateStatusV1::Underpowered,
                ..
            } => {}
            ViolationV1::GateBlocked {
                status: GateStatusV1::Pass,
                ..
            } => unreachable!("pass evidence cannot produce a blocked gate"),
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

fn domain_hash(domain: &str, payload: &[u8]) -> Digest32 {
    let mut hasher = Sha256::new();
    hasher.update(
        u32::try_from(domain.len())
            .expect("static domain length fits u32")
            .to_be_bytes(),
    );
    hasher.update(domain.as_bytes());
    hasher.update(
        u64::try_from(payload.len())
            .expect("reference payload length fits u64")
            .to_be_bytes(),
    );
    hasher.update(payload);
    Digest32(hasher.finalize().into())
}

fn write_len(len: usize, out: &mut Vec<u8>) {
    out.extend_from_slice(
        &u32::try_from(len)
            .expect("reference collection length fits u32")
            .to_be_bytes(),
    );
}

fn write_string(value: &str, out: &mut Vec<u8>) {
    write_len(value.len(), out);
    out.extend_from_slice(value.as_bytes());
}

fn write_optional_digest(value: Option<Digest32>, out: &mut Vec<u8>) {
    match value {
        Some(value) => {
            out.push(1);
            value.write(out);
        }
        None => out.push(0),
    }
}

fn reference_bindings(seed: u8) -> Vec<BindingRequirementV1> {
    vec![
        BindingRequirementV1::new("environment", Digest32::repeated(seed)),
        BindingRequirementV1::new("experiment-plan", Digest32::repeated(seed + 1)),
        BindingRequirementV1::new("population", Digest32::repeated(seed + 2)),
        BindingRequirementV1::new("producer-profile", Digest32::repeated(seed + 3)),
        BindingRequirementV1::new("scorer-profile", Digest32::repeated(seed + 4)),
        BindingRequirementV1::new("sensor-profile", Digest32::repeated(seed + 5)),
    ]
}

fn reference_profile(label: &str) -> ValidityProfileV1 {
    ValidityProfileV1::new(
        label,
        SchemaBindingV1::new("sym-eval.reference-claim.v1", 1),
        1,
        reference_bindings(10),
        vec![
            GateRequirementV1::pass("isolation", 40, 140),
            GateRequirementV1::pass_or_na("optional-renderer", "profile-has-no-renderer", 41, 141),
            GateRequirementV1::pass("preregistration", 42, 142),
            GateRequirementV1::pass("sensor-shielding", 43, 143),
        ],
    )
    .expect("reference profile is valid")
}

fn matching_evidence_bindings(profile: &ValidityProfileV1) -> Vec<EvidenceBindingV1> {
    profile
        .bindings
        .iter()
        .map(|binding| EvidenceBindingV1::new(&binding.key, binding.expected))
        .collect()
}

fn passing_evidence(profile: &ValidityProfileV1) -> Vec<GateEvidenceV1> {
    profile
        .requirements
        .iter()
        .enumerate()
        .map(|(index, requirement)| {
            GateEvidenceV1::new(
                &requirement.gate_key,
                GateStatusV1::Pass,
                Digest32::repeated(180 + u8::try_from(index).expect("small fixture")),
                requirement.expected_subject,
                requirement.expected_verifier,
                matching_evidence_bindings(profile),
            )
            .expect("reference evidence is valid")
        })
        .collect()
}

fn evidence_mut<'a>(evidence: &'a mut [GateEvidenceV1], gate_key: &str) -> &'a mut GateEvidenceV1 {
    evidence
        .iter_mut()
        .find(|item| item.gate_key == gate_key)
        .expect("fixture gate exists")
}

#[test]
fn core009a_profile_identity_is_content_derived_not_caller_label() {
    let first = reference_profile("display-a");
    let second = reference_profile("display-b");
    assert_eq!(first.digest(), second.digest());

    let mut changed_requirements = second.requirements.clone();
    changed_requirements.push(GateRequirementV1::pass("new-required-gate", 44, 144));
    let changed = ValidityProfileV1::new(
        "display-a",
        second.claim_schema.clone(),
        second.admission_policy_version,
        second.bindings.clone(),
        changed_requirements,
    )
    .unwrap();

    assert_ne!(first.digest(), changed.digest());
}

#[test]
fn core009a_profile_identity_is_order_independent() {
    let first = reference_profile("first");
    let mut reversed_bindings = first.bindings.clone();
    reversed_bindings.reverse();
    let mut reversed_requirements = first.requirements.clone();
    reversed_requirements.reverse();

    let second = ValidityProfileV1::new(
        "second",
        first.claim_schema.clone(),
        first.admission_policy_version,
        reversed_bindings,
        reversed_requirements,
    )
    .unwrap();

    assert_eq!(first.digest(), second.digest());
    assert_eq!(first.canonical_bytes(), second.canonical_bytes());
}

#[test]
fn core009a_all_required_compatible_evidence_admits_confirmatory_performance() {
    let profile = reference_profile("reference");
    let receipt = ValidityReceiptV1::evaluate(&profile, passing_evidence(&profile)).unwrap();

    assert_eq!(receipt.admission, AdmissionStateV1::ConfirmatoryAdmitted);
    assert!(receipt.violations.is_empty());
    assert_eq!(
        receipt.performance_availability(),
        PerformanceAvailabilityV1::Confirmatory
    );
}

#[test]
fn core009a_evidence_order_does_not_change_receipt_identity() {
    let profile = reference_profile("reference");
    let forward = passing_evidence(&profile);
    let mut reverse = forward.clone();
    reverse.reverse();

    let first = ValidityReceiptV1::evaluate(&profile, forward).unwrap();
    let second = ValidityReceiptV1::evaluate(&profile, reverse).unwrap();

    assert_eq!(first, second);
    assert_eq!(first.digest(), second.digest());
}

#[test]
fn core009a_missing_queued_and_underpowered_never_become_confirmatory() {
    let profile = reference_profile("reference");

    let mut evidence = passing_evidence(&profile);
    evidence.retain(|item| item.gate_key != "sensor-shielding");
    evidence_mut(&mut evidence, "isolation").status = GateStatusV1::InfrastructureQueued;
    evidence_mut(&mut evidence, "preregistration").status = GateStatusV1::Underpowered;

    let receipt = ValidityReceiptV1::evaluate(&profile, evidence).unwrap();

    assert_eq!(receipt.admission, AdmissionStateV1::BlockedInfrastructure);
    assert_eq!(
        receipt.performance_availability(),
        PerformanceAvailabilityV1::DiagnosticOnly
    );
    assert!(receipt.violations.iter().any(|violation| matches!(
        violation,
        ViolationV1::MissingGate(gate) if gate == "sensor-shielding"
    )));
    assert!(receipt.violations.iter().any(|violation| matches!(
        violation,
        ViolationV1::GateBlocked {
            gate_key,
            status: GateStatusV1::InfrastructureQueued
        } if gate_key == "isolation"
    )));
    assert!(receipt.violations.iter().any(|violation| matches!(
        violation,
        ViolationV1::GateBlocked {
            gate_key,
            status: GateStatusV1::Underpowered
        } if gate_key == "preregistration"
    )));
}

#[test]
fn core009a_profile_mismatch_and_stale_evidence_precedence_is_deterministic() {
    let profile = reference_profile("reference");
    let mut evidence = passing_evidence(&profile);

    evidence_mut(&mut evidence, "isolation").status = GateStatusV1::Invalid;
    evidence_mut(&mut evidence, "preregistration").status = GateStatusV1::EvidenceStale;
    evidence_mut(&mut evidence, "sensor-shielding").subject_ref = Some(Digest32::repeated(99));

    let receipt = ValidityReceiptV1::evaluate(&profile, evidence).unwrap();

    assert_eq!(receipt.admission, AdmissionStateV1::BlockedProfileMismatch);
    assert!(receipt.violations.iter().any(|violation| matches!(
        violation,
        ViolationV1::GateBlocked {
            gate_key,
            status: GateStatusV1::Invalid
        } if gate_key == "isolation"
    )));
    assert!(receipt.violations.iter().any(|violation| matches!(
        violation,
        ViolationV1::GateBlocked {
            gate_key,
            status: GateStatusV1::EvidenceStale
        } if gate_key == "preregistration"
    )));
    assert!(receipt.violations.iter().any(|violation| matches!(
        violation,
        ViolationV1::SubjectMismatch(gate) if gate == "sensor-shielding"
    )));
}

#[test]
fn core009a_invalid_gate_blocks_reassuring_other_gate() {
    let profile = reference_profile("reference");
    let mut evidence = passing_evidence(&profile);
    evidence_mut(&mut evidence, "sensor-shielding").status = GateStatusV1::Invalid;

    let receipt = ValidityReceiptV1::evaluate(&profile, evidence).unwrap();

    assert_eq!(receipt.admission, AdmissionStateV1::BlockedInvalidBenchmark);
    assert_eq!(
        receipt.performance_availability(),
        PerformanceAvailabilityV1::DiagnosticOnly
    );
}

#[test]
fn core009a_exact_not_applicable_reason_is_precommitted() {
    let profile = reference_profile("reference");
    let mut accepted = passing_evidence(&profile);
    evidence_mut(&mut accepted, "optional-renderer").status =
        GateStatusV1::NotApplicable("profile-has-no-renderer".to_owned());

    assert_eq!(
        ValidityReceiptV1::evaluate(&profile, accepted)
            .unwrap()
            .admission,
        AdmissionStateV1::ConfirmatoryAdmitted
    );

    let mut wrong = passing_evidence(&profile);
    evidence_mut(&mut wrong, "optional-renderer").status =
        GateStatusV1::NotApplicable("renderer-failed".to_owned());

    let receipt = ValidityReceiptV1::evaluate(&profile, wrong).unwrap();
    assert_eq!(receipt.admission, AdmissionStateV1::BlockedProfileMismatch);
    assert!(receipt.violations.iter().any(|violation| matches!(
        violation,
        ViolationV1::NotApplicableReasonMismatch(gate) if gate == "optional-renderer"
    )));
}

#[test]
fn core009a_binding_subject_verifier_and_unexpected_fields_fail_closed() {
    let profile = reference_profile("reference");
    let mut evidence = passing_evidence(&profile);
    let item = evidence_mut(&mut evidence, "isolation");
    item.subject_ref = Some(Digest32::repeated(1));
    item.verifier_ref = Some(Digest32::repeated(2));
    item.bindings
        .retain(|binding| binding.key != "experiment-plan");
    item.bindings.push(EvidenceBindingV1::new(
        "undeclared-axis",
        Digest32::repeated(3),
    ));
    item.bindings.sort();

    let receipt = ValidityReceiptV1::evaluate(&profile, evidence).unwrap();

    assert_eq!(receipt.admission, AdmissionStateV1::BlockedProfileMismatch);
    assert!(receipt.violations.iter().any(|violation| matches!(
        violation,
        ViolationV1::SubjectMismatch(gate) if gate == "isolation"
    )));
    assert!(receipt.violations.iter().any(|violation| matches!(
        violation,
        ViolationV1::VerifierMismatch(gate) if gate == "isolation"
    )));
    assert!(receipt.violations.iter().any(|violation| matches!(
        violation,
        ViolationV1::MissingBinding {
            gate_key,
            binding_key
        } if gate_key == "isolation" && binding_key == "experiment-plan"
    )));
    assert!(receipt.violations.iter().any(|violation| matches!(
        violation,
        ViolationV1::UnexpectedBinding {
            gate_key,
            binding_key
        } if gate_key == "isolation" && binding_key == "undeclared-axis"
    )));
}

#[test]
fn core009a_zero_evidence_reference_is_invalid_not_pass() {
    let profile = reference_profile("reference");
    let mut evidence = passing_evidence(&profile);
    evidence_mut(&mut evidence, "isolation").evidence_ref = Digest32::ZERO;

    let receipt = ValidityReceiptV1::evaluate(&profile, evidence).unwrap();

    assert_eq!(receipt.admission, AdmissionStateV1::BlockedInvalidBenchmark);
    assert!(receipt.violations.iter().any(|violation| matches!(
        violation,
        ViolationV1::MissingEvidenceReference(gate) if gate == "isolation"
    )));
}

#[test]
fn core009a_duplicate_requirements_bindings_and_evidence_are_rejected() {
    let profile = reference_profile("reference");

    let duplicate_requirement = ValidityProfileV1::new(
        "duplicate",
        profile.claim_schema.clone(),
        profile.admission_policy_version,
        profile.bindings.clone(),
        vec![
            GateRequirementV1::pass("same", 1, 2),
            GateRequirementV1::pass("same", 3, 4),
        ],
    );
    assert_eq!(
        duplicate_requirement,
        Err(BuildErrorV1::DuplicateRequirement("same".to_owned()))
    );

    let duplicate_binding = ValidityProfileV1::new(
        "duplicate",
        profile.claim_schema.clone(),
        profile.admission_policy_version,
        vec![
            BindingRequirementV1::new("same", Digest32::repeated(1)),
            BindingRequirementV1::new("same", Digest32::repeated(2)),
        ],
        profile.requirements.clone(),
    );
    assert_eq!(
        duplicate_binding,
        Err(BuildErrorV1::DuplicateBindingRequirement("same".to_owned()))
    );

    let mut duplicate_evidence = passing_evidence(&profile);
    duplicate_evidence.push(duplicate_evidence[0].clone());
    assert!(matches!(
        ValidityReceiptV1::evaluate(&profile, duplicate_evidence),
        Err(BuildErrorV1::DuplicateEvidence(_))
    ));

    assert_eq!(
        GateEvidenceV1::new(
            "gate",
            GateStatusV1::Pass,
            Digest32::repeated(1),
            None,
            None,
            vec![
                EvidenceBindingV1::new("same", Digest32::repeated(2)),
                EvidenceBindingV1::new("same", Digest32::repeated(3)),
            ],
        ),
        Err(BuildErrorV1::DuplicateEvidenceBinding {
            gate_key: "gate".to_owned(),
            binding_key: "same".to_owned(),
        })
    );
}

#[test]
fn core009a_any_evidence_byte_change_changes_receipt_identity() {
    let profile = reference_profile("reference");
    let first = ValidityReceiptV1::evaluate(&profile, passing_evidence(&profile)).unwrap();

    let mut changed_evidence = passing_evidence(&profile);
    evidence_mut(&mut changed_evidence, "isolation").evidence_ref = Digest32::repeated(250);
    let second = ValidityReceiptV1::evaluate(&profile, changed_evidence).unwrap();

    assert_ne!(first.digest(), second.digest());
}

#[test]
fn core009a_reference_vectors_are_frozen() {
    let profile = reference_profile("ignored-display-label");
    let receipt = ValidityReceiptV1::evaluate(&profile, passing_evidence(&profile)).unwrap();

    assert_eq!(
        profile.digest().to_hex(),
        "e9ed0534345c5ab153232a85dab89409f55d2cc343cdc28f74e33395443e4a4d"
    );
    assert_eq!(
        receipt.digest().to_hex(),
        "d853d852cfc2d9aa83f9682432adaedac671d9b8e286129e27a8ff80b11a2e9d"
    );
}

#[test]
fn core009a_document_keeps_reference_boundary_explicit() {
    assert!(REFERENCE_DOC.contains("Reference only"));
    assert!(REFERENCE_DOC.contains("production authority is not established"));
    assert!(REFERENCE_DOC.contains("opaque evidence reference"));
    assert!(REFERENCE_DOC.contains("CORE-001 r2 exact execution PASS"));
    assert!(REFERENCE_DOC.contains("terminal score"));
}
