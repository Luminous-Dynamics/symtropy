// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Machine-readable corpus registration and claim/use qualification for CivBench.
//!
//! This module is a provenance and rights firewall, not a legal opinion,
//! dataset downloader, or substitute for source-owner permission. It preserves
//! separate dataset, article and code licensing evidence. A repository licence
//! never silently becomes a dataset licence; missing Rights metadata never
//! means public permission.

use std::collections::BTreeSet;
use std::fmt::Write as _;

use crate::benchmark_split::{LeakageDimension, SplitProtocolPreset};
use crate::decision_fixture::{FixtureError, JsonParser, JsonValue};

const MAX_REGISTRY_BYTES: usize = 2 * 1024 * 1024;

/// Evidence lane. Metrics from different lanes are not implicitly poolable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CorpusLane {
    /// Measurements from a real operating water utility.
    RealUtility,
    /// EPANET/WNTR or other simulated hydraulic measurements.
    SimulatedHydraulic,
    /// Physical instrumented cyber-physical water testbed.
    CyberPhysicalTestbed,
    /// Acoustic/pressure leak laboratory or pipe-loop testbed.
    AcousticLaboratory,
    /// Derived, transformed, or feature-only data rather than raw sensor data.
    DerivedFeatures,
}

impl CorpusLane {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::RealUtility => "real_utility",
            Self::SimulatedHydraulic => "simulated_hydraulic",
            Self::CyberPhysicalTestbed => "cyber_physical_testbed",
            Self::AcousticLaboratory => "acoustic_laboratory",
            Self::DerivedFeatures => "derived_features",
        }
    }
}

/// Verification state for the *dataset's* own rights statement.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RightsVerification {
    /// Dataset licence and the source asserting it have been checked.
    Verified,
    /// A licence is claimed or inferred by a secondary source, but unresolved.
    Unverified,
    /// The reviewed source does not state a dataset licence.
    NotProvided,
    /// Access/use is restricted by the owner or a separate agreement.
    Restricted,
}

impl RightsVerification {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Verified => "verified",
            Self::Unverified => "unverified",
            Self::NotProvided => "not_provided",
            Self::Restricted => "restricted",
        }
    }
}

/// Access terms are distinct from copyright/database rights.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CorpusAccess {
    /// Direct public retrieval with no separate account/contact gate identified.
    PublicDirect,
    /// Registration, request, contact form, or competition access process.
    RegistrationOrContact,
    /// Source says access is restricted or controlled.
    Restricted,
    /// Access terms have not been established.
    Unknown,
}

impl CorpusAccess {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::PublicDirect => "public_direct",
            Self::RegistrationOrContact => "registration_or_contact",
            Self::Restricted => "restricted",
            Self::Unknown => "unknown",
        }
    }
}

/// Proposed use being evaluated against the registered rights/access evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum IntendedCorpusUse {
    /// Reading citation and metadata only; no dataset bytes used.
    MetadataOnly,
    /// Research benchmarking/evaluation on acquired dataset bytes.
    ResearchEvaluation,
    /// Non-commercial model fitting.
    NonCommercialTraining,
    /// Commercial model fitting or use in a commercial service.
    CommercialTraining,
    /// Redistribution or republication of dataset bytes.
    Redistribution,
}

/// A claim a benchmark result might try to support.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum CorpusClaim {
    /// Event/burst/anomaly detection in the registered lane.
    EventDetection,
    /// Cyberattack/leak/sensor-failure diagnosis in a testbed lane.
    CyberPhysicalDiagnosis,
    /// Acoustic or hydrophone/accelerometer leak detection.
    AcousticLeakDetection,
    /// Pipe/asset-level localization.
    PipeLocalization,
    /// Zone/district-level localization.
    ZoneLocalization,
    /// Transfer to unseen hydraulic network IDs.
    CrossNetworkTransfer,
    /// Transfer to unseen topology families.
    CrossTopologyTransfer,
    /// Temporal early warning against a documented fault/repair time.
    TemporalEarlyWarning,
    /// Validation using data from an operating utility.
    UtilityFieldValidation,
    /// Evaluation on raw instrument measurements rather than derived features.
    RawSensorEvaluation,
    /// Evaluation of a pressure/storage control policy.
    ControlPolicyEvaluation,
}

impl CorpusClaim {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::EventDetection => "event_detection",
            Self::CyberPhysicalDiagnosis => "cyber_physical_diagnosis",
            Self::AcousticLeakDetection => "acoustic_leak_detection",
            Self::PipeLocalization => "pipe_localization",
            Self::ZoneLocalization => "zone_localization",
            Self::CrossNetworkTransfer => "cross_network_transfer",
            Self::CrossTopologyTransfer => "cross_topology_transfer",
            Self::TemporalEarlyWarning => "temporal_early_warning",
            Self::UtilityFieldValidation => "utility_field_validation",
            Self::RawSensorEvaluation => "raw_sensor_evaluation",
            Self::ControlPolicyEvaluation => "control_policy_evaluation",
        }
    }
}

/// Separately recorded dataset, article and code licensing evidence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorpusRights {
    /// Rights status for the dataset bytes/database themselves.
    pub dataset_status: RightsVerification,
    /// Dataset-level licence identifier, if a source claims one.
    pub dataset_license_id: Option<String>,
    /// URL or source note for the dataset-level licence claim.
    pub dataset_license_source: Option<String>,
    /// Article/paper licence, never inherited by the dataset.
    pub article_license_id: Option<String>,
    /// Repository/code licence, never inherited by the dataset.
    pub code_license_id: Option<String>,
    /// Acquisition/access conditions.
    pub access: CorpusAccess,
}

/// Basic scale/granularity and sensor/feature information.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorpusScope {
    /// Number of independent utilities, testbeds, or sites, if known.
    pub site_count: Option<u32>,
    /// Number of distinct hydraulic networks, if known.
    pub network_count: Option<u32>,
    /// Number of rows/events/scenarios, if known.
    pub sample_count: Option<u64>,
    /// Measurement cadence or sample duration, if applicable.
    pub cadence: Option<String>,
    /// Data time span or acquisition window, if applicable.
    pub time_span: Option<String>,
    /// Unit of one record (e.g. hourly row, 30-second capture, hydraulic scenario).
    pub granularity: String,
    /// True only when raw instrument measurements are present.
    pub raw_sensor_data: bool,
    /// Modalities/features such as pressure, flow, acoustic, energy, anomaly_score.
    pub modalities: BTreeSet<String>,
}

/// Label provenance and resolution limits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorpusTruth {
    /// Truth source type, e.g. repair_record, controlled_injection, simulator_oracle.
    pub source_kind: String,
    /// How labels were assigned or adjudicated.
    pub label_method: String,
    /// Window between label timestamp and affected records, if applicable.
    pub label_window: Option<String>,
    /// Whether each target event was independently adjudicated.
    pub independently_adjudicated: bool,
    /// Whether a pipe-level ground-truth identity is available.
    pub pipe_truth_available: bool,
    /// Whether a zone-level ground-truth identity is available.
    pub zone_truth_available: bool,
    /// Known label or observability caveats.
    pub caveats: Vec<String>,
}

/// One versioned corpus registration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorpusRecord {
    /// Stable machine identifier.
    pub corpus_id: String,
    /// Human-readable corpus title.
    pub title: String,
    /// Dataset owner/provider.
    pub provider: String,
    /// Canonical DOI or source accession URL.
    pub accession_uri: String,
    /// Version/release recorded by the registry.
    pub version: String,
    /// Date metadata was reviewed (YYYY-MM-DD).
    pub retrieved_on: String,
    /// Evidence lane.
    pub lane: CorpusLane,
    /// Dataset rights separate from paper and code.
    pub rights: CorpusRights,
    /// Scale, granularity and modality.
    pub scope: CorpusScope,
    /// Truth/label provenance.
    pub truth: CorpusTruth,
    /// Provenance dimensions that are documented for this source/version.
    pub provenance_dimensions: BTreeSet<LeakageDimension>,
    /// Registered split protocols for which this version has grouping metadata.
    pub eligible_split_presets: BTreeSet<SplitProtocolPreset>,
    /// Claims this registration permits the benchmark to make, subject to
    /// each specific experiment satisfying its own protocol and evidence rules.
    pub allowed_claims: BTreeSet<CorpusClaim>,
    /// Explicitly prohibited or unsupported claims.
    pub prohibited_claims: BTreeSet<CorpusClaim>,
    /// Human-readable constraints and limitations.
    pub limitations: Vec<String>,
}

/// Parsed, validated corpus registry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorpusRegistry {
    /// Registry identity.
    pub registry_id: String,
    /// Schema/registry version.
    pub registry_version: String,
    /// Corpus records sorted by stable corpus ID.
    pub corpora: Vec<CorpusRecord>,
}

impl CorpusRegistry {
    /// Stable line-oriented receipt. It records claims and rights status, not
    /// raw dataset identifiers or observations.
    pub fn canonical_text(&self) -> String {
        let mut output = String::new();
        let _ = writeln!(&mut output, "CIVBENCH_CORPUS_REGISTRY_V0.1");
        let _ = writeln!(&mut output, "registry_id={}", self.registry_id);
        let _ = writeln!(&mut output, "registry_version={}", self.registry_version);
        for corpus in &self.corpora {
            let _ = writeln!(
                &mut output,
                "corpus={},{},{},{},{},{}",
                corpus.corpus_id,
                corpus.lane.as_str(),
                corpus.version,
                corpus.rights.dataset_status.as_str(),
                corpus.rights.access.as_str(),
                corpus.scope.raw_sensor_data
            );
            let _ = writeln!(
                &mut output,
                "dataset_license={},{}",
                corpus.rights.dataset_license_id.as_deref().unwrap_or("none"),
                corpus
                    .rights
                    .dataset_license_source
                    .as_deref()
                    .unwrap_or("none")
            );
            for dimension in &corpus.provenance_dimensions {
                let _ = writeln!(
                    &mut output,
                    "provenance={},{}",
                    corpus.corpus_id,
                    dimension.as_str()
                );
            }
            for preset in &corpus.eligible_split_presets {
                let _ = writeln!(
                    &mut output,
                    "split_preset={},{}",
                    corpus.corpus_id,
                    preset.as_str()
                );
            }
            for claim in &corpus.allowed_claims {
                let _ = writeln!(
                    &mut output,
                    "allowed_claim={},{}",
                    corpus.corpus_id,
                    claim.as_str()
                );
            }
            for claim in &corpus.prohibited_claims {
                let _ = writeln!(
                    &mut output,
                    "prohibited_claim={},{}",
                    corpus.corpus_id,
                    claim.as_str()
                );
            }
        }
        output
    }

    /// Find one corpus by its stable registry identity.
    pub fn corpus(&self, corpus_id: &str) -> Option<&CorpusRecord> {
        self.corpora
            .binary_search_by(|record| record.corpus_id.as_str().cmp(corpus_id))
            .ok()
            .map(|index| &self.corpora[index])
    }
}

/// A stable machine-readable corpus registry error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CorpusRegistryError {
    /// Stable error category.
    pub code: &'static str,
    /// Corpus, field, or failing claim/use.
    pub subject: String,
    /// Human-readable explanation.
    pub detail: String,
}

impl CorpusRegistryError {
    fn new(
        code: &'static str,
        subject: impl Into<String>,
        detail: impl Into<String>,
    ) -> Self {
        Self {
            code,
            subject: subject.into(),
            detail: detail.into(),
        }
    }

    fn from_json(error: FixtureError) -> Self {
        Self::new(
            "invalid_registry_json",
            error.path,
            format!("{} at byte {}: {}", error.code, error.offset, error.detail),
        )
    }
}

impl std::fmt::Display for CorpusRegistryError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{}: {} ({})",
            self.code, self.subject, self.detail
        )
    }
}

impl std::error::Error for CorpusRegistryError {}

/// Parse and validate a strict, duplicate-key-rejecting JSON corpus registry.
pub fn parse_corpus_registry(input: &str) -> Result<CorpusRegistry, CorpusRegistryError> {
    if input.len() > MAX_REGISTRY_BYTES {
        return Err(CorpusRegistryError::new(
            "registry_too_large",
            "$",
            "registry exceeds the 2 MiB safety limit",
        ));
    }
    let root = JsonParser::new(input)
        .parse()
        .map_err(CorpusRegistryError::from_json)?;
    let root = as_object(&root, "$")?;
    reject_unknown_keys(
        root,
        &["registry_id", "registry_version", "corpora"],
        "$",
    )?;
    let registry_id = string_field(root, "registry_id", "$")?;
    if !valid_identifier(registry_id) {
        return Err(CorpusRegistryError::new(
            "invalid_registry_id",
            registry_id,
            "registry identifier must be a stable reference",
        ));
    }
    let registry_version = string_field(root, "registry_version", "$")?;
    if registry_version != "0.1.0" {
        return Err(CorpusRegistryError::new(
            "unsupported_registry_version",
            registry_version,
            "only registry version 0.1.0 is supported",
        ));
    }
    let corpus_values = array_field(root, "corpora", "$")?;
    if corpus_values.is_empty() {
        return Err(CorpusRegistryError::new(
            "empty_corpus_registry",
            registry_id,
            "registry must contain at least one corpus",
        ));
    }
    let mut corpora = Vec::with_capacity(corpus_values.len());
    for (index, value) in corpus_values.iter().enumerate() {
        let path = format!("$.corpora[{index}]");
        corpora.push(parse_record(value, &path)?);
    }
    corpora.sort_by(|left, right| left.corpus_id.cmp(&right.corpus_id));
    validate_registry_records(&corpora)?;
    Ok(CorpusRegistry {
        registry_id: registry_id.to_owned(),
        registry_version: registry_version.to_owned(),
        corpora,
    })
}

/// Validate whether the supplied registry evidence permits an intended use.
///
/// Metadata-only access is always permitted. Any use of dataset bytes requires
/// a verified dataset-level licence and a public-direct or explicitly granted
/// access path. A code or article licence is not accepted as a dataset licence.
/// Commercial use and redistribution are restricted to a deliberately small
/// permissive allow-list; this is a conservative screening rule, not legal advice.
pub fn validate_intended_use(
    corpus: &CorpusRecord,
    intended_use: IntendedCorpusUse,
) -> Result<(), CorpusRegistryError> {
    if intended_use == IntendedCorpusUse::MetadataOnly {
        return Ok(());
    }
    if corpus.rights.dataset_status != RightsVerification::Verified
        || corpus.rights.dataset_license_id.is_none()
        || corpus.rights.dataset_license_source.is_none()
    {
        return Err(CorpusRegistryError::new(
            "dataset_rights_unverified",
            corpus.corpus_id.as_str(),
            "dataset bytes require a verified dataset-level licence; paper/code licences do not substitute",
        ));
    }
    match corpus.rights.access {
        CorpusAccess::PublicDirect | CorpusAccess::PermissionGranted => {}
        CorpusAccess::RegistrationOrContact => {
            return Err(CorpusRegistryError::new(
                "access_requires_permission",
                corpus.corpus_id.as_str(),
                "registration/contact access requires a separately recorded permission grant",
            ));
        }
        CorpusAccess::Restricted | CorpusAccess::Unknown => {
            return Err(CorpusRegistryError::new(
                "access_not_authorized",
                corpus.corpus_id.as_str(),
                "dataset access is restricted or unknown",
            ));
        }
    }
    if matches!(
        intended_use,
        IntendedCorpusUse::CommercialTraining | IntendedCorpusUse::Redistribution
    ) {
        let license = corpus
            .rights
            .dataset_license_id
            .as_deref()
            .unwrap_or_default();
        if !is_permissive_commercial_license(license) {
            return Err(CorpusRegistryError::new(
                "license_not_cleared_for_commercial_use",
                corpus.corpus_id.as_str(),
                "commercial training/redistribution requires a verified licence on the registry allow-list",
            ));
        }
    }
    Ok(())
}

/// Verify that a dataset record supports a requested benchmark claim.
pub fn validate_claim(
    corpus: &CorpusRecord,
    claim: CorpusClaim,
) -> Result<(), CorpusRegistryError> {
    if corpus.prohibited_claims.contains(&claim) {
        return Err(CorpusRegistryError::new(
            "claim_explicitly_prohibited",
            format!("{}:{}", corpus.corpus_id, claim.as_str()),
            "registry marks this claim as prohibited",
        ));
    }
    if !corpus.allowed_claims.contains(&claim) {
        return Err(CorpusRegistryError::new(
            "claim_not_registered",
            format!("{}:{}", corpus.corpus_id, claim.as_str()),
            "claim is not in the corpus allowed-claims register",
        ));
    }
    match claim {
        CorpusClaim::CyberPhysicalDiagnosis
            if corpus.lane != CorpusLane::CyberPhysicalTestbed =>
        {
            return claim_lane_error(corpus, claim, "requires a cyber-physical testbed lane");
        }
        CorpusClaim::AcousticLeakDetection
            if !corpus.scope.modalities.contains("acoustic") =>
        {
            return claim_lane_error(corpus, claim, "requires an acoustic modality");
        }
        CorpusClaim::PipeLocalization if !corpus.truth.pipe_truth_available => {
            return claim_lane_error(corpus, claim, "requires pipe-level independent truth");
        }
        CorpusClaim::ZoneLocalization if !corpus.truth.zone_truth_available => {
            return claim_lane_error(corpus, claim, "requires zone-level independent truth");
        }
        CorpusClaim::CrossNetworkTransfer
            if !corpus
                .eligible_split_presets
                .contains(&SplitProtocolPreset::CrossNetwork) =>
        {
            return claim_lane_error(corpus, claim, "requires a registered cross-network split");
        }
        CorpusClaim::CrossTopologyTransfer
            if !corpus
                .eligible_split_presets
                .contains(&SplitProtocolPreset::CrossTopology) =>
        {
            return claim_lane_error(corpus, claim, "requires a registered cross-topology split");
        }
        CorpusClaim::TemporalEarlyWarning
            if corpus.truth.label_window.is_none() || corpus.scope.cadence.is_none() =>
        {
            return claim_lane_error(corpus, claim, "requires a cadence and explicit label window");
        }
        CorpusClaim::UtilityFieldValidation
            if corpus.lane != CorpusLane::RealUtility =>
        {
            return claim_lane_error(corpus, claim, "requires a real-utility evidence lane");
        }
        CorpusClaim::RawSensorEvaluation if !corpus.scope.raw_sensor_data => {
            return claim_lane_error(corpus, claim, "derived-feature corpora are not raw sensor data");
        }
        CorpusClaim::ControlPolicyEvaluation if corpus.lane != CorpusLane::RealUtility => {
            return claim_lane_error(corpus, claim, "control claim requires a registered operating-system lane");
        }
        _ => {}
    }
    Ok(())
}

fn claim_lane_error(
    corpus: &CorpusRecord,
    claim: CorpusClaim,
    detail: &str,
) -> Result<(), CorpusRegistryError> {
    Err(CorpusRegistryError::new(
        "claim_evidence_mismatch",
        format!("{}:{}", corpus.corpus_id, claim.as_str()),
        detail,
    ))
}

fn is_permissive_commercial_license(license: &str) -> bool {
    matches!(
        license,
        "CC0-1.0" | "CC-BY-4.0" | "CC-BY-3.0" | "MIT" | "Apache-2.0" | "BSD-3-Clause"
    )
}

fn validate_registry_records(corpora: &[CorpusRecord]) -> Result<(), CorpusRegistryError> {
    let mut corpus_ids = BTreeSet::new();
    for corpus in corpora {
        if !corpus_ids.insert(corpus.corpus_id.as_str()) {
            return Err(CorpusRegistryError::new(
                "duplicate_corpus_id",
                corpus.corpus_id.as_str(),
                "corpus IDs must be unique",
            ));
        }
        if !valid_identifier(&corpus.corpus_id)
            || corpus.title.trim().is_empty()
            || corpus.provider.trim().is_empty()
            || !corpus.accession_uri.starts_with("https://")
            || !valid_date(&corpus.retrieved_on)
            || corpus.version.trim().is_empty()
            || corpus.scope.granularity.trim().is_empty()
            || corpus.truth.source_kind.trim().is_empty()
            || corpus.truth.label_method.trim().is_empty()
        {
            return Err(CorpusRegistryError::new(
                "invalid_corpus_record",
                corpus.corpus_id.as_str(),
                "title, provider, https accession, version, retrieval date, granularity and label provenance are required",
            ));
        }
        if corpus.scope.site_count == Some(0)
            || corpus.scope.network_count == Some(0)
            || corpus.scope.sample_count == Some(0)
        {
            return Err(CorpusRegistryError::new(
                "invalid_corpus_scope",
                corpus.corpus_id.as_str(),
                "known site/network/sample counts must be positive",
            ));
        }
        if corpus.scope.modalities.is_empty() {
            return Err(CorpusRegistryError::new(
                "missing_measurement_modalities",
                corpus.corpus_id.as_str(),
                "at least one measurement or feature modality is required",
            ));
        }
        if corpus.lane == CorpusLane::DerivedFeatures && corpus.scope.raw_sensor_data {
            return Err(CorpusRegistryError::new(
                "derived_lane_claims_raw_sensor_data",
                corpus.corpus_id.as_str(),
                "derived feature lane cannot assert raw sensor data",
            ));
        }
        match corpus.rights.dataset_status {
            RightsVerification::Verified => {
                if corpus.rights.dataset_license_id.as_deref().is_none_or(str::is_empty)
                    || corpus
                        .rights
                        .dataset_license_source
                        .as_deref()
                        .is_none_or(str::is_empty)
                {
                    return Err(CorpusRegistryError::new(
                        "verified_rights_missing_source",
                        corpus.corpus_id.as_str(),
                        "verified dataset rights require a licence ID and source",
                    ));
                }
            }
            RightsVerification::NotProvided => {
                if corpus.rights.dataset_license_id.is_some()
                    || corpus.rights.dataset_license_source.is_some()
                {
                    return Err(CorpusRegistryError::new(
                        "not_provided_rights_has_claim",
                        corpus.corpus_id.as_str(),
                        "not_provided status cannot carry a claimed dataset licence",
                    ));
                }
            }
            RightsVerification::Unverified | RightsVerification::Restricted => {}
        }
        if !corpus.allowed_claims.is_disjoint(&corpus.prohibited_claims) {
            return Err(CorpusRegistryError::new(
                "claim_allow_deny_overlap",
                corpus.corpus_id.as_str(),
                "a claim cannot be both allowed and prohibited",
            ));
        }
        for preset in &corpus.eligible_split_presets {
            let required: BTreeSet<_> = preset
                .exclusive_dimensions()
                .union(&preset.shared_dimensions())
                .copied()
                .collect();
            if !required.is_subset(&corpus.provenance_dimensions) {
                let missing: Vec<_> = required
                    .difference(&corpus.provenance_dimensions)
                    .map(|dimension| dimension.as_str())
                    .collect();
                return Err(CorpusRegistryError::new(
                    "split_preset_missing_provenance",
                    format!("{}:{}", corpus.corpus_id, preset.as_str()),
                    format!("missing documented grouping dimensions: {}", missing.join(",")),
                ));
            }
        }
        for claim in &corpus.allowed_claims {
            // Re-run evidence compatibility without requiring membership lookup.
            validate_claim(corpus, *claim)?;
        }
    }
    Ok(())
}

fn valid_date(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| matches!(index, 4 | 7) || byte.is_ascii_digit())
}

fn valid_identifier(value: &str) -> bool {
    if value.is_empty() || value.len() > 256 || value.trim() != value {
        return false;
    }
    let mut bytes = value.bytes();
    let Some(first) = bytes.next() else {
        return false;
    };
    first.is_ascii_alphanumeric()
        && bytes.all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':' | b'/')
        })
}

fn as_object<'a>(
    value: &'a JsonValue,
    subject: &str,
) -> Result<&'a std::collections::BTreeMap<String, JsonValue>, CorpusRegistryError> {
    match value {
        JsonValue::Object(object) => Ok(object),
        _ => Err(CorpusRegistryError::new(
            "expected_object",
            subject,
            "expected a JSON object",
        )),
    }
}

fn reject_unknown(
    object: &std::collections::BTreeMap<String, JsonValue>,
    allowed: &[&str],
    subject: &str,
) -> Result<(), CorpusRegistryError> {
    if let Some(key) = object.keys().find(|key| !allowed.contains(&key.as_str())) {
        return Err(CorpusRegistryError::new(
            "unknown_registry_field",
            subject,
            format!("unknown field '{key}' is not permitted"),
        ));
    }
    Ok(())
}

fn required<'a>(
    object: &'a std::collections::BTreeMap<String, JsonValue>,
    key: &str,
    subject: &str,
) -> Result<&'a JsonValue, CorpusRegistryError> {
    object.get(key).ok_or_else(|| {
        CorpusRegistryError::new(
            "missing_required_field",
            format!("{subject}.{key}"),
            "required field is absent",
        )
    })
}

fn string_field<'a>(
    object: &'a std::collections::BTreeMap<String, JsonValue>,
    key: &str,
    subject: &str,
) -> Result<&'a str, CorpusRegistryError> {
    match required(object, key, subject)? {
        JsonValue::String(value) => Ok(value),
        _ => Err(CorpusRegistryError::new(
            "expected_string",
            format!("{subject}.{key}"),
            "field must be a string",
        )),
    }
}

fn optional_string_field(
    object: &std::collections::BTreeMap<String, JsonValue>,
    key: &str,
    subject: &str,
) -> Result<Option<String>, CorpusRegistryError> {
    match required(object, key, subject)? {
        JsonValue::Null => Ok(None),
        JsonValue::String(value) if !value.is_empty() => Ok(Some(value.clone())),
        _ => Err(CorpusRegistryError::new(
            "expected_optional_string",
            format!("{subject}.{key}"),
            "field must be a non-empty string or null",
        )),
    }
}

fn bool_field(
    object: &std::collections::BTreeMap<String, JsonValue>,
    key: &str,
    subject: &str,
) -> Result<bool, CorpusRegistryError> {
    match required(object, key, subject)? {
        JsonValue::Bool(value) => Ok(*value),
        _ => Err(CorpusRegistryError::new(
            "expected_boolean",
            format!("{subject}.{key}"),
            "field must be a boolean",
        )),
    }
}

fn u64_field(
    object: &std::collections::BTreeMap<String, JsonValue>,
    key: &str,
    subject: &str,
) -> Result<Option<u64>, CorpusRegistryError> {
    match required(object, key, subject)? {
        JsonValue::Null => Ok(None),
        JsonValue::Number(value) => value.parse::<u64>().map(Some).map_err(|_| {
            CorpusRegistryError::new(
                "invalid_integer",
                format!("{subject}.{key}"),
                "expected a non-negative integer",
            )
        }),
        _ => Err(CorpusRegistryError::new(
            "expected_optional_integer",
            format!("{subject}.{key}"),
            "field must be an integer or null",
        )),
    }
}

fn u32_field(
    object: &std::collections::BTreeMap<String, JsonValue>,
    key: &str,
    subject: &str,
) -> Result<Option<u32>, CorpusRegistryError> {
    u64_field(object, key, subject)?
        .map(|value| {
            u32::try_from(value).map_err(|_| {
                CorpusRegistryError::new(
                    "integer_out_of_range",
                    format!("{subject}.{key}"),
                    "value exceeds u32",
                )
            })
        })
        .transpose()
}

fn string_set(
    object: &std::collections::BTreeMap<String, JsonValue>,
    key: &str,
    subject: &str,
) -> Result<BTreeSet<String>, CorpusRegistryError> {
    let value = required(object, key, subject)?;
    let JsonValue::Array(values) = value else {
        return Err(CorpusRegistryError::new(
            "expected_string_array",
            format!("{subject}.{key}"),
            "field must be an array of strings",
        ));
    };
    let mut result = BTreeSet::new();
    for (index, value) in values.iter().enumerate() {
        let JsonValue::String(value) = value else {
            return Err(CorpusRegistryError::new(
                "expected_string",
                format!("{subject}.{key}[{index}]"),
                "array item must be a string",
            ));
        };
        if value.trim().is_empty() || value.trim() != value {
            return Err(CorpusRegistryError::new(
                "invalid_array_string",
                format!("{subject}.{key}[{index}]"),
                "array strings must be non-empty and not whitespace-padded",
            ));
        }
        if !result.insert(value.clone()) {
            return Err(CorpusRegistryError::new(
                "duplicate_array_value",
                format!("{subject}.{key}[{index}]"),
                "duplicate array values are not allowed",
            ));
        }
    }
    Ok(result)
}

fn array_field<'a>(
    object: &'a std::collections::BTreeMap<String, JsonValue>,
    key: &str,
    subject: &str,
) -> Result<&'a [JsonValue], CorpusRegistryError> {
    match required(object, key, subject)? {
        JsonValue::Array(values) => Ok(values),
        _ => Err(CorpusRegistryError::new(
            "expected_array",
            format!("{subject}.{key}"),
            "field must be an array",
        )),
    }
}

fn parse_lane(value: &str, subject: &str) -> Result<CorpusLane, CorpusRegistryError> {
    match value {
        "real_utility" => Ok(CorpusLane::RealUtility),
        "simulated_hydraulic" => Ok(CorpusLane::SimulatedHydraulic),
        "cyber_physical_testbed" => Ok(CorpusLane::CyberPhysicalTestbed),
        "acoustic_laboratory" => Ok(CorpusLane::AcousticLaboratory),
        "derived_features" => Ok(CorpusLane::DerivedFeatures),
        _ => Err(CorpusRegistryError::new("invalid_corpus_lane", subject, value)),
    }
}

fn parse_rights_verification(
    value: &str,
    subject: &str,
) -> Result<RightsVerification, CorpusRegistryError> {
    match value {
        "verified" => Ok(RightsVerification::Verified),
        "unverified" => Ok(RightsVerification::Unverified),
        "not_provided" => Ok(RightsVerification::NotProvided),
        "restricted" => Ok(RightsVerification::Restricted),
        _ => Err(CorpusRegistryError::new(
            "invalid_rights_status",
            subject,
            value,
        )),
    }
}

fn parse_access(value: &str, subject: &str) -> Result<CorpusAccess, CorpusRegistryError> {
    match value {
        "public_direct" => Ok(CorpusAccess::PublicDirect),
        "registration_or_contact" => Ok(CorpusAccess::RegistrationOrContact),
        "permission_granted" => Ok(CorpusAccess::PermissionGranted),
        "restricted" => Ok(CorpusAccess::Restricted),
        "unknown" => Ok(CorpusAccess::Unknown),
        _ => Err(CorpusRegistryError::new(
            "invalid_access_mode",
            subject,
            value,
        )),
    }
}

fn parse_dimension(
    value: &str,
    subject: &str,
) -> Result<LeakageDimension, CorpusRegistryError> {
    match value {
        "physical_event" => Ok(LeakageDimension::PhysicalEvent),
        "capture_session" => Ok(LeakageDimension::CaptureSession),
        "pipe_asset" => Ok(LeakageDimension::PipeAsset),
        "network" => Ok(LeakageDimension::Network),
        "topology_family" => Ok(LeakageDimension::TopologyFamily),
        "scenario_template" => Ok(LeakageDimension::ScenarioTemplate),
        "sensor_layout" => Ok(LeakageDimension::SensorLayout),
        "hydraulic_model" => Ok(LeakageDimension::HydraulicModel),
        "demand_profile" => Ok(LeakageDimension::DemandProfile),
        "time_block" => Ok(LeakageDimension::TimeBlock),
        _ => Err(CorpusRegistryError::new(
            "invalid_provenance_dimension",
            subject,
            value,
        )),
    }
}

fn parse_split_preset(
    value: &str,
    subject: &str,
) -> Result<SplitProtocolPreset, CorpusRegistryError> {
    match value {
        "within_network_unseen_event_v1" => Ok(SplitProtocolPreset::WithinNetworkUnseenEvent),
        "new_operating_regime_v1" => Ok(SplitProtocolPreset::NewOperatingRegime),
        "cross_network_v1" => Ok(SplitProtocolPreset::CrossNetwork),
        "cross_topology_v1" => Ok(SplitProtocolPreset::CrossTopology),
        "sensor_layout_transfer_v1" => Ok(SplitProtocolPreset::SensorLayoutTransfer),
        "asset_transfer_v1" => Ok(SplitProtocolPreset::AssetTransfer),
        _ => Err(CorpusRegistryError::new(
            "invalid_split_preset",
            subject,
            value,
        )),
    }
}

fn parse_claim(value: &str, subject: &str) -> Result<CorpusClaim, CorpusRegistryError> {
    match value {
        "event_detection" => Ok(CorpusClaim::EventDetection),
        "cyber_physical_diagnosis" => Ok(CorpusClaim::CyberPhysicalDiagnosis),
        "acoustic_leak_detection" => Ok(CorpusClaim::AcousticLeakDetection),
        "pipe_localization" => Ok(CorpusClaim::PipeLocalization),
        "zone_localization" => Ok(CorpusClaim::ZoneLocalization),
        "cross_network_transfer" => Ok(CorpusClaim::CrossNetworkTransfer),
        "cross_topology_transfer" => Ok(CorpusClaim::CrossTopologyTransfer),
        "temporal_early_warning" => Ok(CorpusClaim::TemporalEarlyWarning),
        "utility_field_validation" => Ok(CorpusClaim::UtilityFieldValidation),
        "raw_sensor_evaluation" => Ok(CorpusClaim::RawSensorEvaluation),
        "control_policy_evaluation" => Ok(CorpusClaim::ControlPolicyEvaluation),
        _ => Err(CorpusRegistryError::new(
            "invalid_corpus_claim",
            subject,
            value,
        )),
    }
}

fn parse_enum_set<T: Ord>(
    object: &std::collections::BTreeMap<String, JsonValue>,
    key: &str,
    subject: &str,
    parser: fn(&str, &str) -> Result<T, CorpusRegistryError>,
) -> Result<BTreeSet<T>, CorpusRegistryError> {
    let values = string_set(object, key, subject)?;
    values
        .iter()
        .map(|value| parser(value, &format!("{subject}.{key}")))
        .collect()
}

fn parse_record(value: &JsonValue, subject: &str) -> Result<CorpusRecord, CorpusRegistryError> {
    let object = as_object(value, subject)?;
    reject_unknown(
        object,
        &[
            "corpus_id",
            "title",
            "provider",
            "accession_uri",
            "version",
            "retrieved_on",
            "lane",
            "rights",
            "scope",
            "truth",
            "provenance_dimensions",
            "eligible_split_presets",
            "allowed_claims",
            "prohibited_claims",
            "limitations",
        ],
        subject,
    )?;
    let corpus_id = string_field(object, "corpus_id", subject)?.to_owned();
    let lane = parse_lane(string_field(object, "lane", subject)?, subject)?;

    let rights_object = as_object(
        required(object, "rights", subject)?,
        &format!("{subject}.rights"),
    )?;
    reject_unknown(
        rights_object,
        &[
            "dataset_status",
            "dataset_license_id",
            "dataset_license_source",
            "article_license_id",
            "code_license_id",
            "access",
        ],
        &format!("{subject}.rights"),
    )?;
    let rights = CorpusRights {
        dataset_status: parse_rights_verification(
            string_field(rights_object, "dataset_status", &format!("{subject}.rights"))?,
            &format!("{subject}.rights.dataset_status"),
        )?,
        dataset_license_id: optional_string_field(
            rights_object,
            "dataset_license_id",
            &format!("{subject}.rights"),
        )?,
        dataset_license_source: optional_string_field(
            rights_object,
            "dataset_license_source",
            &format!("{subject}.rights"),
        )?,
        article_license_id: optional_string_field(
            rights_object,
            "article_license_id",
            &format!("{subject}.rights"),
        )?,
        code_license_id: optional_string_field(
            rights_object,
            "code_license_id",
            &format!("{subject}.rights"),
        )?,
        access: parse_access(
            string_field(rights_object, "access", &format!("{subject}.rights"))?,
            &format!("{subject}.rights.access"),
        )?,
    };

    let scope_object = as_object(
        required(object, "scope", subject)?,
        &format!("{subject}.scope"),
    )?;
    reject_unknown(
        scope_object,
        &[
            "site_count",
            "network_count",
            "sample_count",
            "cadence",
            "time_span",
            "granularity",
            "raw_sensor_data",
            "modalities",
        ],
        &format!("{subject}.scope"),
    )?;
    let scope = CorpusScope {
        site_count: u32_field(scope_object, "site_count", &format!("{subject}.scope"))?,
        network_count: u32_field(scope_object, "network_count", &format!("{subject}.scope"))?,
        sample_count: u64_field(scope_object, "sample_count", &format!("{subject}.scope"))?,
        cadence: optional_string_field(scope_object, "cadence", &format!("{subject}.scope"))?,
        time_span: optional_string_field(scope_object, "time_span", &format!("{subject}.scope"))?,
        granularity: string_field(scope_object, "granularity", &format!("{subject}.scope"))?
            .to_owned(),
        raw_sensor_data: bool_field(
            scope_object,
            "raw_sensor_data",
            &format!("{subject}.scope"),
        )?,
        modalities: string_set(scope_object, "modalities", &format!("{subject}.scope"))?,
    };

    let truth_object = as_object(
        required(object, "truth", subject)?,
        &format!("{subject}.truth"),
    )?;
    reject_unknown(
        truth_object,
        &[
            "source_kind",
            "label_method",
            "label_window",
            "independently_adjudicated",
            "pipe_truth_available",
            "zone_truth_available",
            "caveats",
        ],
        &format!("{subject}.truth"),
    )?;
    let truth = CorpusTruth {
        source_kind: string_field(truth_object, "source_kind", &format!("{subject}.truth"))?
            .to_owned(),
        label_method: string_field(truth_object, "label_method", &format!("{subject}.truth"))?
            .to_owned(),
        label_window: optional_string_field(
            truth_object,
            "label_window",
            &format!("{subject}.truth"),
        )?,
        independently_adjudicated: bool_field(
            truth_object,
            "independently_adjudicated",
            &format!("{subject}.truth"),
        )?,
        pipe_truth_available: bool_field(
            truth_object,
            "pipe_truth_available",
            &format!("{subject}.truth"),
        )?,
        zone_truth_available: bool_field(
            truth_object,
            "zone_truth_available",
            &format!("{subject}.truth"),
        )?,
        caveats: string_set(truth_object, "caveats", &format!("{subject}.truth"))?
            .into_iter()
            .collect(),
    };
    let provenance_dimensions = parse_enum_set(
        object,
        "provenance_dimensions",
        subject,
        |value, path| parse_dimension(value, path),
    )?;
    let eligible_split_presets = parse_enum_set(
        object,
        "eligible_split_presets",
        subject,
        |value, path| parse_split_preset(value, path),
    )?;
    let allowed_claims = parse_enum_set(object, "allowed_claims", subject, parse_claim)?;
    let prohibited_claims = parse_enum_set(object, "prohibited_claims", subject, parse_claim)?;
    let limitations = string_set(object, "limitations", subject)?
        .into_iter()
        .collect();

    Ok(CorpusRecord {
        corpus_id,
        title: string_field(object, "title", subject)?.to_owned(),
        provider: string_field(object, "provider", subject)?.to_owned(),
        accession_uri: string_field(object, "accession_uri", subject)?.to_owned(),
        version: string_field(object, "version", subject)?.to_owned(),
        retrieved_on: string_field(object, "retrieved_on", subject)?.to_owned(),
        lane,
        rights,
        scope,
        truth,
        provenance_dimensions,
        eligible_split_presets,
        allowed_claims,
        prohibited_claims,
        limitations,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commercial_allowlist_is_small_and_explicit() {
        for license in [
            "CC0-1.0",
            "CC-BY-4.0",
            "CC-BY-3.0",
            "MIT",
            "Apache-2.0",
            "BSD-3-Clause",
        ] {
            assert!(is_permissive_commercial_license(license));
        }
        for license in ["CC-BY-NC-4.0", "CC-BY-NC-ND-4.0", "unknown", ""] {
            assert!(!is_permissive_commercial_license(license));
        }
    }
}
