// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! SEL-10E2C qualification of unavailable current model outcomes.
//!
//! A missing E2B row is retained evidence, but absence of a current outcome is not
//! self-proving. This layer binds every missing row to explicit observation and
//! qualification authorities while leaving the E2B taxonomic/robustness conclusion
//! unchanged.

use crate::{
    CrossModelCurrentSpeciesReport, CrossModelCurrentSpeciesReportDigest,
    CrossModelOutcomeDisposition, CurrentCrossModelRobustnessAuthority,
    CurrentCrossModelRobustnessDesignDigest, ModelSpecificEvidenceSurfaceDigest,
    ValidatedCrossModelCurrentSpeciesReport,
};
use serde::{Deserialize, Deserializer, Serialize};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, error::Error, fmt};
use symtropy_evolution_core::{AnalysisAuthorityRef, AnalysisContentDigest, AnalysisMethodId};
use symtropy_species_concept::{
    OpenSpeciesConceptIdentity, SpeciesConceptFamilyDescriptorDigest,
};

pub const OUTCOME_AVAILABILITY_POLICY_VERSION: u32 = 1;
pub const OUTCOME_AVAILABILITY_LEDGER_VERSION: u32 = 1;
const POLICY_DOMAIN: &[u8] = b"symtropy:species-concept:outcome-availability-policy:v1\0";
const LEDGER_DOMAIN: &[u8] = b"symtropy:species-concept:outcome-availability-ledger:v1\0";
const RULE_DOMAIN: &[u8] = b"symtropy:species-concept:outcome-availability-rule:v1\0";
const RULE_SPEC: &[u8] = b"current outcome availability v1: bind policy to exact current E2A design before relying on E2B outcomes; retain exactly one availability record per exact report row; observed current outcomes require no absence assertion; every missing-current-capability row requires exact observation plus independent qualification authorities; unavailability evidence binds the exact report row and cannot infer the unavailable model's latent support, contradiction, or non-support conclusion; restored bytes are representation only; current authority requires fresh E2A policy replay plus fresh validated E2B report and exact unavailability inputs; no majority vote, historical robustness, nomenclature, or universal taxonomy claim";

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize)]
pub struct OutcomeAvailabilityPolicyId(String);

impl OutcomeAvailabilityPolicyId {
    pub fn new(value: impl Into<String>) -> Result<Self, OutcomeAvailabilityError> {
        let value = value.into();
        validate_id("OutcomeAvailabilityPolicyId", &value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for OutcomeAvailabilityPolicyId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(<D::Error as serde::de::Error>::custom)
    }
}

macro_rules! digest_type {
    ($name:ident) => {
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        pub struct $name([u8; 32]);
        impl $name {
            pub fn new(bytes: [u8; 32]) -> Self {
                Self(bytes)
            }
            pub fn as_bytes(&self) -> &[u8; 32] {
                &self.0
            }
        }
        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, concat!(stringify!($name), "("))?;
                fmt_hex(&self.0, f)?;
                write!(f, ")")
            }
        }
        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                fmt_hex(&self.0, f)
            }
        }
    };
}

digest_type!(OutcomeAvailabilityPolicyDigest);
digest_type!(CrossModelOutcomeAvailabilityLedgerDigest);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutcomeAvailabilityPolicy {
    policy_version: u32,
    pub policy_id: OutcomeAvailabilityPolicyId,
    pub e2a_design_digest: CurrentCrossModelRobustnessDesignDigest,
    pub unavailable_observation_protocol_authority: AnalysisAuthorityRef,
    pub rule_authority: AnalysisAuthorityRef,
}

impl OutcomeAvailabilityPolicy {
    pub fn declare(
        policy_id: OutcomeAvailabilityPolicyId,
        e2a: &CurrentCrossModelRobustnessAuthority<'_>,
        unavailable_observation_protocol_authority: AnalysisAuthorityRef,
    ) -> Result<Self, OutcomeAvailabilityError> {
        validate_authority(
            &unavailable_observation_protocol_authority,
            "unavailable_observation_protocol_revision",
        )?;
        let policy = Self {
            policy_version: OUTCOME_AVAILABILITY_POLICY_VERSION,
            policy_id,
            e2a_design_digest: e2a.design_digest(),
            unavailable_observation_protocol_authority,
            rule_authority: outcome_availability_rule_v1(),
        };
        policy.validate_local()?;
        Ok(policy)
    }

    pub fn canonical_digest(
        &self,
    ) -> Result<OutcomeAvailabilityPolicyDigest, OutcomeAvailabilityError> {
        self.validate_local()?;
        let mut digest = Sha256::new();
        digest.update(POLICY_DOMAIN);
        put_u32(&mut digest, self.policy_version);
        put_text(&mut digest, self.policy_id.as_str());
        digest.update(self.e2a_design_digest.as_bytes());
        put_authority(
            &mut digest,
            &self.unavailable_observation_protocol_authority,
        );
        put_authority(&mut digest, &self.rule_authority);
        Ok(OutcomeAvailabilityPolicyDigest::new(digest.finalize().into()))
    }

    fn validate_local(&self) -> Result<(), OutcomeAvailabilityError> {
        if self.policy_version != OUTCOME_AVAILABILITY_POLICY_VERSION {
            return Err(OutcomeAvailabilityError::UnsupportedPolicyVersion(
                self.policy_version,
            ));
        }
        validate_id("OutcomeAvailabilityPolicyId", self.policy_id.as_str())?;
        validate_authority(
            &self.unavailable_observation_protocol_authority,
            "unavailable_observation_protocol_revision",
        )?;
        if self.rule_authority != outcome_availability_rule_v1() {
            return Err(OutcomeAvailabilityError::RuleAuthorityMismatch);
        }
        Ok(())
    }
}

#[derive(Debug)]
#[must_use = "persisted availability policy is not current without fresh E2A replay"]
pub struct ValidatedOutcomeAvailabilityPolicy<'a> {
    policy: &'a OutcomeAvailabilityPolicy,
    policy_digest: OutcomeAvailabilityPolicyDigest,
}

impl<'a> ValidatedOutcomeAvailabilityPolicy<'a> {
    pub fn validate_current(
        policy: &'a OutcomeAvailabilityPolicy,
        e2a: &CurrentCrossModelRobustnessAuthority<'_>,
        unavailable_observation_protocol_authority: AnalysisAuthorityRef,
    ) -> Result<Self, OutcomeAvailabilityError> {
        policy.validate_local()?;
        let recomputed = OutcomeAvailabilityPolicy::declare(
            policy.policy_id.clone(),
            e2a,
            unavailable_observation_protocol_authority,
        )?;
        if recomputed != *policy {
            return Err(OutcomeAvailabilityError::PolicyReplayMismatch);
        }
        Ok(Self {
            policy,
            policy_digest: policy.canonical_digest()?,
        })
    }

    pub fn policy(&self) -> &'a OutcomeAvailabilityPolicy {
        self.policy
    }

    pub fn policy_digest(&self) -> OutcomeAvailabilityPolicyDigest {
        self.policy_digest
    }
}

#[derive(Debug, Clone)]
pub struct OutcomeUnavailabilityInput {
    pub conceptual_identity: OpenSpeciesConceptIdentity,
    pub observation_authority: AnalysisAuthorityRef,
    pub qualification_authority: AnalysisAuthorityRef,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModelOutcomeAvailabilityDisposition {
    ObservedCurrentOutcome,
    QualifiedUnavailable,
}

impl ModelOutcomeAvailabilityDisposition {
    fn tag(self) -> u8 {
        match self {
            Self::ObservedCurrentOutcome => 0,
            Self::QualifiedUnavailable => 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelOutcomeAvailabilityRecord {
    pub conceptual_identity: OpenSpeciesConceptIdentity,
    pub descriptor_digest: SpeciesConceptFamilyDescriptorDigest,
    pub evidence_surface_digest: ModelSpecificEvidenceSurfaceDigest,
    pub row_disposition: CrossModelOutcomeDisposition,
    pub availability: ModelOutcomeAvailabilityDisposition,
    pub observation_protocol_authority: AnalysisAuthorityRef,
    pub unavailability_observation_authority: Option<AnalysisAuthorityRef>,
    pub unavailability_qualification_authority: Option<AnalysisAuthorityRef>,
}

impl ModelOutcomeAvailabilityRecord {
    fn put(&self, digest: &mut Sha256) {
        put_identity(digest, &self.conceptual_identity);
        digest.update(self.descriptor_digest.as_bytes());
        digest.update(self.evidence_surface_digest.as_bytes());
        digest.update([outcome_disposition_tag(self.row_disposition)]);
        digest.update([self.availability.tag()]);
        put_authority(digest, &self.observation_protocol_authority);
        put_optional_authority(digest, self.unavailability_observation_authority.as_ref());
        put_optional_authority(
            digest,
            self.unavailability_qualification_authority.as_ref(),
        );
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrossModelOutcomeAvailabilityLedger {
    ledger_version: u32,
    pub policy: OutcomeAvailabilityPolicy,
    pub policy_digest: OutcomeAvailabilityPolicyDigest,
    pub report: CrossModelCurrentSpeciesReport,
    pub report_digest: CrossModelCurrentSpeciesReportDigest,
    pub records: Vec<ModelOutcomeAvailabilityRecord>,
    pub rule_authority: AnalysisAuthorityRef,
}

impl CrossModelOutcomeAvailabilityLedger {
    pub fn capture(
        policy: &ValidatedOutcomeAvailabilityPolicy<'_>,
        report: &ValidatedCrossModelCurrentSpeciesReport<'_>,
        unavailable_inputs: impl IntoIterator<Item = OutcomeUnavailabilityInput>,
    ) -> Result<Self, OutcomeAvailabilityError> {
        let raw_policy = policy.policy();
        let raw_report = report.report();
        if raw_report.design_digest != raw_policy.e2a_design_digest {
            return Err(OutcomeAvailabilityError::PolicyReportDesignMismatch);
        }

        let mut inputs = BTreeMap::new();
        for input in unavailable_inputs {
            validate_authority(
                &input.observation_authority,
                "unavailability_observation_revision",
            )?;
            validate_authority(
                &input.qualification_authority,
                "unavailability_qualification_revision",
            )?;
            if inputs
                .insert(input.conceptual_identity.clone(), input)
                .is_some()
            {
                return Err(OutcomeAvailabilityError::DuplicateUnavailabilityInput);
            }
        }

        let mut records = Vec::with_capacity(raw_report.rows.len());
        for row in &raw_report.rows {
            let missing =
                row.disposition() == CrossModelOutcomeDisposition::MissingCurrentCapability;
            let input = inputs.remove(&row.conceptual_identity);
            let (availability, observation, qualification) = match (missing, input) {
                (false, None) => (
                    ModelOutcomeAvailabilityDisposition::ObservedCurrentOutcome,
                    None,
                    None,
                ),
                (false, Some(_)) => {
                    return Err(OutcomeAvailabilityError::UnavailabilityForObservedRow)
                }
                (true, None) => {
                    return Err(OutcomeAvailabilityError::MissingQualifiedUnavailability)
                }
                (true, Some(input)) => (
                    ModelOutcomeAvailabilityDisposition::QualifiedUnavailable,
                    Some(input.observation_authority),
                    Some(input.qualification_authority),
                ),
            };
            records.push(ModelOutcomeAvailabilityRecord {
                conceptual_identity: row.conceptual_identity.clone(),
                descriptor_digest: row.descriptor_digest,
                evidence_surface_digest: row.evidence_surface_digest,
                row_disposition: row.disposition(),
                availability,
                observation_protocol_authority: raw_policy
                    .unavailable_observation_protocol_authority
                    .clone(),
                unavailability_observation_authority: observation,
                unavailability_qualification_authority: qualification,
            });
        }
        if !inputs.is_empty() {
            return Err(OutcomeAvailabilityError::UnexpectedUnavailabilityInput);
        }

        let ledger = Self {
            ledger_version: OUTCOME_AVAILABILITY_LEDGER_VERSION,
            policy: raw_policy.clone(),
            policy_digest: policy.policy_digest(),
            report: raw_report.clone(),
            report_digest: report.report_digest(),
            records,
            rule_authority: outcome_availability_rule_v1(),
        };
        ledger.validate_local()?;
        Ok(ledger)
    }

    pub fn canonical_digest(
        &self,
    ) -> Result<CrossModelOutcomeAvailabilityLedgerDigest, OutcomeAvailabilityError> {
        self.validate_local()?;
        let mut digest = Sha256::new();
        digest.update(LEDGER_DOMAIN);
        put_u32(&mut digest, self.ledger_version);
        digest.update(self.policy_digest.as_bytes());
        digest.update(self.report_digest.as_bytes());
        put_u64(&mut digest, self.records.len() as u64);
        for record in &self.records {
            record.put(&mut digest);
        }
        put_authority(&mut digest, &self.rule_authority);
        Ok(CrossModelOutcomeAvailabilityLedgerDigest::new(
            digest.finalize().into(),
        ))
    }

    fn validate_local(&self) -> Result<(), OutcomeAvailabilityError> {
        if self.ledger_version != OUTCOME_AVAILABILITY_LEDGER_VERSION {
            return Err(OutcomeAvailabilityError::UnsupportedLedgerVersion(
                self.ledger_version,
            ));
        }
        if self.policy.canonical_digest()? != self.policy_digest {
            return Err(OutcomeAvailabilityError::PolicyDigestMismatch);
        }
        if self.report.canonical_digest()? != self.report_digest {
            return Err(OutcomeAvailabilityError::ReportDigestMismatch);
        }
        if self.report.design_digest != self.policy.e2a_design_digest {
            return Err(OutcomeAvailabilityError::PolicyReportDesignMismatch);
        }
        if self.records.len() != self.report.rows.len() {
            return Err(OutcomeAvailabilityError::IncompleteAvailabilityCoverage);
        }
        for (row, record) in self.report.rows.iter().zip(&self.records) {
            if record.conceptual_identity != row.conceptual_identity
                || record.descriptor_digest != row.descriptor_digest
                || record.evidence_surface_digest != row.evidence_surface_digest
                || record.row_disposition != row.disposition()
            {
                return Err(OutcomeAvailabilityError::AvailabilityRowBindingMismatch);
            }
            if record.observation_protocol_authority
                != self.policy.unavailable_observation_protocol_authority
            {
                return Err(OutcomeAvailabilityError::ObservationProtocolMismatch);
            }
            let missing =
                row.disposition() == CrossModelOutcomeDisposition::MissingCurrentCapability;
            match (
                missing,
                record.availability,
                record.unavailability_observation_authority.as_ref(),
                record.unavailability_qualification_authority.as_ref(),
            ) {
                (
                    false,
                    ModelOutcomeAvailabilityDisposition::ObservedCurrentOutcome,
                    None,
                    None,
                ) => {}
                (
                    true,
                    ModelOutcomeAvailabilityDisposition::QualifiedUnavailable,
                    Some(observation),
                    Some(qualification),
                ) => {
                    validate_authority(observation, "unavailability_observation_revision")?;
                    validate_authority(qualification, "unavailability_qualification_revision")?;
                }
                (false, ModelOutcomeAvailabilityDisposition::QualifiedUnavailable, ..) => {
                    return Err(OutcomeAvailabilityError::UnavailabilityForObservedRow)
                }
                (true, ModelOutcomeAvailabilityDisposition::ObservedCurrentOutcome, ..) => {
                    return Err(OutcomeAvailabilityError::MissingQualifiedUnavailability)
                }
                _ => return Err(OutcomeAvailabilityError::AvailabilityEvidenceShapeMismatch),
            }
        }
        if self.rule_authority != outcome_availability_rule_v1() {
            return Err(OutcomeAvailabilityError::RuleAuthorityMismatch);
        }
        Ok(())
    }
}

#[derive(Debug)]
#[must_use = "availability-qualified report authority should gate downstream reliance on missing rows"]
pub struct ValidatedCrossModelOutcomeAvailability<'a> {
    ledger: &'a CrossModelOutcomeAvailabilityLedger,
    ledger_digest: CrossModelOutcomeAvailabilityLedgerDigest,
    report_digest: CrossModelCurrentSpeciesReportDigest,
}

impl<'a> ValidatedCrossModelOutcomeAvailability<'a> {
    pub fn validate_current(
        ledger: &'a CrossModelOutcomeAvailabilityLedger,
        policy: &ValidatedOutcomeAvailabilityPolicy<'_>,
        report: &ValidatedCrossModelCurrentSpeciesReport<'_>,
        unavailable_inputs: impl IntoIterator<Item = OutcomeUnavailabilityInput>,
    ) -> Result<Self, OutcomeAvailabilityError> {
        ledger.validate_local()?;
        let recomputed =
            CrossModelOutcomeAvailabilityLedger::capture(policy, report, unavailable_inputs)?;
        if recomputed != *ledger {
            return Err(OutcomeAvailabilityError::LedgerReplayMismatch);
        }
        Ok(Self {
            ledger,
            ledger_digest: ledger.canonical_digest()?,
            report_digest: report.report_digest(),
        })
    }

    pub fn ledger(&self) -> &'a CrossModelOutcomeAvailabilityLedger {
        self.ledger
    }

    pub fn ledger_digest(&self) -> CrossModelOutcomeAvailabilityLedgerDigest {
        self.ledger_digest
    }

    pub fn report_digest(&self) -> CrossModelCurrentSpeciesReportDigest {
        self.report_digest
    }
}

pub fn outcome_availability_rule_v1() -> AnalysisAuthorityRef {
    let mut digest = Sha256::new();
    digest.update(RULE_DOMAIN);
    digest.update(RULE_SPEC);
    AnalysisAuthorityRef::new(
        AnalysisMethodId::new("current-model-outcome-availability-v1")
            .expect("static availability rule ID must be valid"),
        1,
        AnalysisContentDigest::new(digest.finalize().into()),
    )
}

fn outcome_disposition_tag(value: CrossModelOutcomeDisposition) -> u8 {
    match value {
        CrossModelOutcomeDisposition::Supports => 0,
        CrossModelOutcomeDisposition::DoesNotSupport => 1,
        CrossModelOutcomeDisposition::Contradicts => 2,
        CrossModelOutcomeDisposition::InsufficientEvidence => 3,
        CrossModelOutcomeDisposition::OutsideValidityDomain => 4,
        CrossModelOutcomeDisposition::MissingCurrentCapability => 5,
    }
}

fn put_optional_authority(digest: &mut Sha256, value: Option<&AnalysisAuthorityRef>) {
    match value {
        None => digest.update([0]),
        Some(value) => {
            digest.update([1]);
            put_authority(digest, value);
        }
    }
}

fn put_identity(digest: &mut Sha256, identity: &OpenSpeciesConceptIdentity) {
    put_text(digest, identity.family_id.as_str());
    put_u32(digest, identity.family_version);
    digest.update(identity.content_digest.as_bytes());
}

fn put_authority(digest: &mut Sha256, authority: &AnalysisAuthorityRef) {
    put_text(digest, authority.method_id.as_str());
    put_u64(digest, authority.revision);
    digest.update(authority.content_digest.as_bytes());
}

fn validate_authority(
    authority: &AnalysisAuthorityRef,
    field: &'static str,
) -> Result<(), OutcomeAvailabilityError> {
    if authority.revision == 0 {
        return Err(OutcomeAvailabilityError::ZeroAuthorityRevision(field));
    }
    Ok(())
}

fn validate_id(field: &'static str, value: &str) -> Result<(), OutcomeAvailabilityError> {
    if value.is_empty() || value.len() > 96 {
        return Err(OutcomeAvailabilityError::InvalidId(field));
    }
    if !value
        .bytes()
        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || matches!(byte, b'-' | b'_' | b'.'))
    {
        return Err(OutcomeAvailabilityError::InvalidId(field));
    }
    Ok(())
}

fn put_text(digest: &mut Sha256, value: &str) {
    put_u64(digest, value.len() as u64);
    digest.update(value.as_bytes());
}

fn put_u32(digest: &mut Sha256, value: u32) {
    digest.update(value.to_le_bytes());
}

fn put_u64(digest: &mut Sha256, value: u64) {
    digest.update(value.to_le_bytes());
}

fn fmt_hex(bytes: &[u8; 32], f: &mut fmt::Formatter<'_>) -> fmt::Result {
    for byte in bytes {
        write!(f, "{byte:02x}")?;
    }
    Ok(())
}

#[derive(Debug)]
pub enum OutcomeAvailabilityError {
    UnsupportedPolicyVersion(u32),
    UnsupportedLedgerVersion(u32),
    InvalidId(&'static str),
    ZeroAuthorityRevision(&'static str),
    RuleAuthorityMismatch,
    PolicyReplayMismatch,
    PolicyDigestMismatch,
    ReportDigestMismatch,
    PolicyReportDesignMismatch,
    DuplicateUnavailabilityInput,
    UnexpectedUnavailabilityInput,
    UnavailabilityForObservedRow,
    MissingQualifiedUnavailability,
    IncompleteAvailabilityCoverage,
    AvailabilityRowBindingMismatch,
    ObservationProtocolMismatch,
    AvailabilityEvidenceShapeMismatch,
    LedgerReplayMismatch,
    Report(crate::CrossModelRobustnessReportError),
}

impl From<crate::CrossModelRobustnessReportError> for OutcomeAvailabilityError {
    fn from(value: crate::CrossModelRobustnessReportError) -> Self {
        Self::Report(value)
    }
}

impl fmt::Display for OutcomeAvailabilityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedPolicyVersion(version) => {
                write!(f, "unsupported outcome-availability policy version {version}")
            }
            Self::UnsupportedLedgerVersion(version) => {
                write!(f, "unsupported outcome-availability ledger version {version}")
            }
            Self::InvalidId(field) => write!(f, "invalid {field}"),
            Self::ZeroAuthorityRevision(field) => write!(f, "{field} must be nonzero"),
            Self::RuleAuthorityMismatch => {
                write!(f, "outcome-availability rule authority does not match V1")
            }
            Self::PolicyReplayMismatch => {
                write!(f, "persisted availability policy does not replay from current E2A authority")
            }
            Self::PolicyDigestMismatch => {
                write!(f, "persisted availability policy does not match its digest")
            }
            Self::ReportDigestMismatch => {
                write!(f, "persisted E2B report does not match its digest")
            }
            Self::PolicyReportDesignMismatch => {
                write!(f, "availability policy and E2B report bind different E2A designs")
            }
            Self::DuplicateUnavailabilityInput => {
                write!(f, "duplicate unavailability input for one conceptual family")
            }
            Self::UnexpectedUnavailabilityInput => {
                write!(f, "unavailability input does not correspond to a report row")
            }
            Self::UnavailabilityForObservedRow => {
                write!(f, "unavailability evidence cannot replace an observed current outcome")
            }
            Self::MissingQualifiedUnavailability => {
                write!(f, "every missing current-outcome row requires qualified unavailability evidence")
            }
            Self::IncompleteAvailabilityCoverage => {
                write!(f, "availability ledger must retain exactly one record per E2B report row")
            }
            Self::AvailabilityRowBindingMismatch => {
                write!(f, "availability record does not bind the exact E2B report row")
            }
            Self::ObservationProtocolMismatch => {
                write!(f, "availability record does not bind the frozen observation protocol")
            }
            Self::AvailabilityEvidenceShapeMismatch => {
                write!(f, "availability record evidence shape is inconsistent with its disposition")
            }
            Self::LedgerReplayMismatch => {
                write!(f, "persisted availability ledger does not replay from current policy/report/evidence")
            }
            Self::Report(error) => write!(f, "E2B report error: {error}"),
        }
    }
}

impl Error for OutcomeAvailabilityError {}
