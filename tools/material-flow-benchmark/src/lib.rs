// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Strict, dependency-light contract validation for Material Flow Benchmark
//! scenarios. This crate validates declared feasibility and evidence contracts;
//! it does not prove physical safety, authorization authenticity, or simulation
//! correctness.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub mod campaign;
pub mod coavailability;
pub mod convergence;
pub mod cross_channel;
pub mod observation;
pub mod procedure;
pub mod simulation;
pub mod transaction;

pub const SCHEMA_VERSION: u16 = 1;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioManifest {
    pub schema_version: u16,
    pub scenario_id: String,
    pub seed: u64,
    pub evidence_level: EvidenceLevel,
    pub started_at_unix_ms: u64,
    pub clock_uncertainty_ms: u32,
    pub digests: InputDigests,
    pub actors: Vec<ActorBinding>,
    pub assets: Vec<AssetBinding>,
    pub locations: Vec<LocationBinding>,
    pub package: PackageSummary,
    pub container: ContainerSummary,
    pub authorization: AuthorizationEnvelope,
    pub flow: FlowPlan,
    pub required_events: Vec<RequiredEvent>,
    pub fault_injections: Vec<FaultInjection>,
    pub success_predicates: Vec<SuccessPredicate>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceLevel {
    L0Contract,
    L1Simulation,
    L2HardwareInTheLoop,
    L3GuardedRig,
    L4RestrictedField,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct InputDigests {
    /// Full 40-character lowercase Git commit identifier.
    pub source_commit: String,
    pub code_digest: String,
    pub model_digest: String,
    pub configuration_digest: String,
    pub site_profile_digest: String,
    pub environment_profile_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ActorBinding {
    /// Opaque test identifier; never a person's name, email, DID or address.
    pub actor_id: String,
    pub role: ActorRole,
    pub identity_digest: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActorRole {
    HouseholdMember,
    ServiceProvider,
    ReceiverDevice,
    Operator,
    Custodian,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AssetBinding {
    pub asset_id: String,
    pub kind: AssetKind,
    pub capability_digest: String,
    pub maximum_payload_mass_g: u64,
    pub maximum_payload_dimensions_mm: DimensionsMm,
    pub supported_modes: Vec<TransportMode>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetKind {
    RoadCarrier,
    Aircraft,
    GroundRobot,
    TransferMechanism,
    StorageSystem,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LocationBinding {
    pub node_id: String,
    pub kind: LocationKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LocationKind {
    CarrierBay,
    CommunityHub,
    ExternalReceiver,
    TransferStation,
    AmbientStorage,
    ChilledStorage,
    FrozenStorage,
    Quarantine,
    ReturnStation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransportMode {
    Road,
    Aerial,
    GroundRobot,
    FixedTransfer,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageSummary {
    pub package_id: String,
    pub mass_g: u64,
    pub dimensions_mm: DimensionsMm,
    pub handling_class: HandlingClass,
    pub seal_state: SealState,
    pub allowed_modes: Vec<TransportMode>,
    pub required_temperature_millic: Option<TemperatureBandMilliC>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HandlingClass {
    Ambient,
    Chilled,
    Frozen,
    Fragile,
    Restricted,
    HumanReview,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SealState {
    Intact,
    Damaged,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DimensionsMm {
    pub length_mm: u32,
    pub width_mm: u32,
    pub height_mm: u32,
}

impl DimensionsMm {
    pub fn is_valid(&self) -> bool {
        self.length_mm > 0 && self.width_mm > 0 && self.height_mm > 0
    }

    /// Conservative axis-aligned fit. It does not rotate or solve 3D packing.
    pub fn fits_within(&self, limit: &Self) -> bool {
        self.length_mm <= limit.length_mm
            && self.width_mm <= limit.width_mm
            && self.height_mm <= limit.height_mm
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TemperatureBandMilliC {
    pub minimum: i32,
    pub maximum: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ContainerSummary {
    pub container_id: String,
    pub internal_dimensions_mm: DimensionsMm,
    pub external_dimensions_mm: DimensionsMm,
    pub tare_mass_g: u64,
    pub maximum_gross_mass_g: u64,
    pub capability_digest: String,
    pub supported_modes: Vec<TransportMode>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthorizationEnvelope {
    pub grant_digest: String,
    pub household_scope_digest: String,
    pub policy_digest: String,
    pub principal_actor_id: String,
    pub audience_actor_id: String,
    pub receiver_device_actor_id: String,
    pub receiver_endpoint_node_id: String,
    pub not_before_unix_ms: u64,
    pub expires_at_unix_ms: u64,
    pub actions: Vec<AuthorizationAction>,
    pub use_limit: UseLimit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthorizationAction {
    ReceiveExternalPackage,
    TransferToTote,
    PutAwayAmbient,
    PutAwayChilled,
    PutAwayFrozen,
    Quarantine,
    ReturnContainer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UseLimit {
    OneShot,
    Bounded { maximum_uses: u16 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FlowPlan {
    pub source_node_id: String,
    pub destination_node_id: String,
    pub segments: Vec<TransferSegment>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TransferSegment {
    pub segment_id: String,
    pub from_node_id: String,
    pub to_node_id: String,
    pub asset_id: String,
    pub mode: TransportMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RequiredEvent {
    AuthorizationChecked,
    PackageReleased,
    PackageReceived,
    TransferVerified,
    PlacementVerified,
    CustodyAcknowledged,
    CustodyExceptionRecorded,
    SafetyDecisionRecorded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FaultInjection {
    pub fault_id: String,
    pub kind: FaultKind,
    pub at_segment_id: Option<String>,
}

/// Safe terminal oracle for an individual injected failure episode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExpectedTerminalState {
    Rejected,
    SafeHold,
    ReconciliationRequired,
}

/// A campaign is a fixed matrix of independent fault episodes over one
/// versioned base scenario. It is an executable test-plan contract, not a
/// simulator result or proof that the fault has been physically exercised.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FaultCampaignManifest {
    pub schema_version: u16,
    pub campaign_id: String,
    pub base_scenario_id: String,
    pub base_scenario_schema_version: u16,
    pub base_scenario_source_commit: String,
    pub episodes: Vec<FaultEpisode>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FaultEpisode {
    pub episode_id: String,
    pub injection: FaultInjection,
    pub expected_terminal_state: ExpectedTerminalState,
}

impl FaultCampaignManifest {
    /// Validate the exact v1 fault family set against its declared base scenario.
    pub fn validate_against(&self, scenario: &ScenarioManifest) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        if self.schema_version != SCHEMA_VERSION {
            errors.push(format!(
                "campaign.schema_version: expected {SCHEMA_VERSION}, got {}",
                self.schema_version
            ));
        }
        check_identifier(&mut errors, "campaign.campaign_id", &self.campaign_id);
        if self.base_scenario_id != scenario.scenario_id {
            errors.push("campaign.base_scenario_id: does not match the supplied scenario".into());
        }
        if self.base_scenario_schema_version != scenario.schema_version {
            errors.push(
                "campaign.base_scenario_schema_version: does not match the supplied scenario"
                    .into(),
            );
        }
        if self.base_scenario_source_commit != scenario.digests.source_commit {
            errors.push(
                "campaign.base_scenario_source_commit: does not match the supplied scenario".into(),
            );
        }
        check_hex(
            &mut errors,
            "campaign.base_scenario_source_commit",
            &self.base_scenario_source_commit,
            40,
            false,
        );

        const REQUIRED_FAULTS: [FaultKind; 10] = [
            FaultKind::WrongTote,
            FaultKind::OverMass,
            FaultKind::StaleTemperature,
            FaultKind::BlockedTransferGate,
            FaultKind::DamagedSeal,
            FaultKind::DuplicateHandoff,
            FaultKind::MissingReceiverAcknowledgement,
            FaultKind::ExpiredGrant,
            FaultKind::PowerInterruption,
            FaultKind::NetworkLoss,
        ];

        if self.episodes.len() != REQUIRED_FAULTS.len() {
            errors.push(format!(
                "campaign.episodes: expected exactly {} independent v1 fault episodes, got {}",
                REQUIRED_FAULTS.len(),
                self.episodes.len()
            ));
        }
        check_unique(
            &mut errors,
            "campaign.episodes.episode_id",
            self.episodes
                .iter()
                .map(|episode| episode.episode_id.as_str()),
        );
        check_unique(
            &mut errors,
            "campaign.episodes.fault_id",
            self.episodes
                .iter()
                .map(|episode| episode.injection.fault_id.as_str()),
        );

        let mut seen_kinds = HashSet::new();
        for (index, episode) in self.episodes.iter().enumerate() {
            let prefix = format!("campaign.episodes[{index}]");
            check_identifier(
                &mut errors,
                &format!("{prefix}.episode_id"),
                &episode.episode_id,
            );
            check_identifier(
                &mut errors,
                &format!("{prefix}.injection.fault_id"),
                &episode.injection.fault_id,
            );
            if !seen_kinds.insert(episode.injection.kind) {
                errors.push(format!("{prefix}.injection.kind: duplicate fault family"));
            }
            if let Some(segment_id) = &episode.injection.at_segment_id
                && !scenario
                    .flow
                    .segments
                    .iter()
                    .any(|segment| segment.segment_id == *segment_id)
            {
                errors.push(format!(
                    "{prefix}.injection.at_segment_id: undeclared segment"
                ));
            }

            let expected = expected_terminal_state(episode.injection.kind);
            if episode.expected_terminal_state != expected {
                errors.push(format!(
                    "{prefix}.expected_terminal_state: {:?} is required for {:?}",
                    expected, episode.injection.kind
                ));
            }
        }

        for required in REQUIRED_FAULTS {
            if !seen_kinds.contains(&required) {
                errors.push(format!(
                    "campaign.episodes: missing required fault family {:?}",
                    required
                ));
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }
}

fn expected_terminal_state(kind: FaultKind) -> ExpectedTerminalState {
    match kind {
        FaultKind::WrongTote | FaultKind::OverMass | FaultKind::ExpiredGrant => {
            ExpectedTerminalState::Rejected
        }
        FaultKind::StaleTemperature | FaultKind::BlockedTransferGate | FaultKind::DamagedSeal => {
            ExpectedTerminalState::SafeHold
        }
        FaultKind::DuplicateHandoff
        | FaultKind::MissingReceiverAcknowledgement
        | FaultKind::PowerInterruption
        | FaultKind::NetworkLoss => ExpectedTerminalState::ReconciliationRequired,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FaultKind {
    WrongTote,
    OverMass,
    StaleTemperature,
    BlockedTransferGate,
    DamagedSeal,
    DuplicateHandoff,
    MissingReceiverAcknowledgement,
    ExpiredGrant,
    PowerInterruption,
    NetworkLoss,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SuccessPredicate {
    PackageIdentityMatches,
    AuthorizedDestination,
    RequiredCustodyComplete,
    SafeHoldReached,
    NoCriticalSafetyViolation,
    NoUnauthorizedPhysicalMutation,
    NoHouseholdPrivateDataDisclosure,
}

impl ScenarioManifest {
    /// Validate the schema and its cross-field feasibility invariants.
    ///
    /// This only qualifies the manifest as internally consistent. It does not
    /// validate a real grant signature, inspect a physical asset, solve packing,
    /// or prove any safety property of a real installation.
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        if self.schema_version != SCHEMA_VERSION {
            errors.push(format!(
                "schema_version: expected {SCHEMA_VERSION}, got {}",
                self.schema_version
            ));
        }
        check_identifier(&mut errors, "scenario_id", &self.scenario_id);
        if self.clock_uncertainty_ms > 60_000 {
            errors.push("clock_uncertainty_ms: exceeds the 60,000 ms contract ceiling".into());
        }

        check_hex(
            &mut errors,
            "digests.source_commit",
            &self.digests.source_commit,
            40,
            false,
        );
        check_digest(
            &mut errors,
            "digests.code_digest",
            &self.digests.code_digest,
        );
        check_digest(
            &mut errors,
            "digests.model_digest",
            &self.digests.model_digest,
        );
        check_digest(
            &mut errors,
            "digests.configuration_digest",
            &self.digests.configuration_digest,
        );
        check_digest(
            &mut errors,
            "digests.site_profile_digest",
            &self.digests.site_profile_digest,
        );
        check_digest(
            &mut errors,
            "digests.environment_profile_digest",
            &self.digests.environment_profile_digest,
        );

        check_nonempty(&mut errors, "actors", self.actors.len());
        check_nonempty(&mut errors, "assets", self.assets.len());
        check_nonempty(&mut errors, "locations", self.locations.len());
        check_unique(
            &mut errors,
            "actors.actor_id",
            self.actors.iter().map(|item| item.actor_id.as_str()),
        );
        check_unique(
            &mut errors,
            "assets.asset_id",
            self.assets.iter().map(|item| item.asset_id.as_str()),
        );
        check_unique(
            &mut errors,
            "locations.node_id",
            self.locations.iter().map(|item| item.node_id.as_str()),
        );

        for (index, actor) in self.actors.iter().enumerate() {
            check_identifier(
                &mut errors,
                &format!("actors[{index}].actor_id"),
                &actor.actor_id,
            );
            check_digest(
                &mut errors,
                &format!("actors[{index}].identity_digest"),
                &actor.identity_digest,
            );
        }

        for (index, asset) in self.assets.iter().enumerate() {
            check_identifier(
                &mut errors,
                &format!("assets[{index}].asset_id"),
                &asset.asset_id,
            );
            check_digest(
                &mut errors,
                &format!("assets[{index}].capability_digest"),
                &asset.capability_digest,
            );
            if asset.maximum_payload_mass_g == 0 {
                errors.push(format!(
                    "assets[{index}].maximum_payload_mass_g: must be greater than zero"
                ));
            }
            if !asset.maximum_payload_dimensions_mm.is_valid() {
                errors.push(format!(
                    "assets[{index}].maximum_payload_dimensions_mm: all dimensions must be greater than zero"
                ));
            }
            check_nonempty(
                &mut errors,
                &format!("assets[{index}].supported_modes"),
                asset.supported_modes.len(),
            );
            check_unique(
                &mut errors,
                &format!("assets[{index}].supported_modes"),
                asset.supported_modes.iter().map(|mode| mode_name(*mode)),
            );
        }

        for (index, location) in self.locations.iter().enumerate() {
            check_identifier(
                &mut errors,
                &format!("locations[{index}].node_id"),
                &location.node_id,
            );
        }

        check_identifier(&mut errors, "package.package_id", &self.package.package_id);
        check_identifier(
            &mut errors,
            "container.container_id",
            &self.container.container_id,
        );
        check_digest(
            &mut errors,
            "container.capability_digest",
            &self.container.capability_digest,
        );
        if self.package.mass_g == 0 {
            errors.push("package.mass_g: must be greater than zero".into());
        }
        if !self.package.dimensions_mm.is_valid() {
            errors.push("package.dimensions_mm: all dimensions must be greater than zero".into());
        }
        if !self.container.internal_dimensions_mm.is_valid() {
            errors.push(
                "container.internal_dimensions_mm: all dimensions must be greater than zero".into(),
            );
        } else if self.package.dimensions_mm.is_valid()
            && !self
                .package
                .dimensions_mm
                .fits_within(&self.container.internal_dimensions_mm)
        {
            errors.push(
                "container.internal_dimensions_mm: package does not fit using the declared conservative axis-aligned check"
                    .into(),
            );
        }
        if !self.container.external_dimensions_mm.is_valid() {
            errors.push(
                "container.external_dimensions_mm: all dimensions must be greater than zero".into(),
            );
        } else if self.container.internal_dimensions_mm.is_valid()
            && !self
                .container
                .internal_dimensions_mm
                .fits_within(&self.container.external_dimensions_mm)
        {
            errors.push(
                "container.external_dimensions_mm: external dimensions must contain the declared internal envelope"
                    .into(),
            );
        }
        if self.container.maximum_gross_mass_g <= self.container.tare_mass_g {
            errors.push("container.maximum_gross_mass_g: must exceed tare_mass_g".into());
        }
        match self.container.tare_mass_g.checked_add(self.package.mass_g) {
            Some(gross) if gross <= self.container.maximum_gross_mass_g => {}
            Some(_) => errors.push(
                "container.maximum_gross_mass_g: package plus tare exceeds gross mass limit".into(),
            ),
            None => errors.push("container: gross mass calculation overflow".into()),
        }

        check_nonempty(
            &mut errors,
            "package.allowed_modes",
            self.package.allowed_modes.len(),
        );
        check_unique(
            &mut errors,
            "package.allowed_modes",
            self.package
                .allowed_modes
                .iter()
                .map(|mode| mode_name(*mode)),
        );
        check_nonempty(
            &mut errors,
            "container.supported_modes",
            self.container.supported_modes.len(),
        );
        check_unique(
            &mut errors,
            "container.supported_modes",
            self.container
                .supported_modes
                .iter()
                .map(|mode| mode_name(*mode)),
        );

        if let Some(band) = self.package.required_temperature_millic
            && band.minimum > band.maximum
        {
            errors.push(
                "package.required_temperature_millic: minimum must not exceed maximum".into(),
            );
        }
        match self.package.handling_class {
            HandlingClass::Chilled | HandlingClass::Frozen
                if self.package.required_temperature_millic.is_none() =>
            {
                errors.push(
                    "package.required_temperature_millic: chilled/frozen cargo requires a band"
                        .into(),
                );
            }
            _ => {}
        }

        if self.flow.segments.is_empty() {
            errors.push("flow.segments: at least one transfer segment is required".into());
        }
        check_identifier(
            &mut errors,
            "flow.source_node_id",
            &self.flow.source_node_id,
        );
        check_identifier(
            &mut errors,
            "flow.destination_node_id",
            &self.flow.destination_node_id,
        );

        let source_exists = self.location(&self.flow.source_node_id).is_some();
        let destination = self.location(&self.flow.destination_node_id);
        if !source_exists {
            errors.push("flow.source_node_id: does not reference a declared location".into());
        }
        if destination.is_none() {
            errors.push("flow.destination_node_id: does not reference a declared location".into());
        }

        check_unique(
            &mut errors,
            "flow.segments.segment_id",
            self.flow
                .segments
                .iter()
                .map(|segment| segment.segment_id.as_str()),
        );
        let mut expected_from = self.flow.source_node_id.as_str();
        let mut reached_endpoint = false;
        for (index, segment) in self.flow.segments.iter().enumerate() {
            let path = format!("flow.segments[{index}]");
            check_identifier(
                &mut errors,
                &format!("{path}.segment_id"),
                &segment.segment_id,
            );
            if segment.from_node_id != expected_from {
                errors.push(format!(
                    "{path}.from_node_id: transfer chain is discontinuous; expected {expected_from}"
                ));
            }
            if self.location(&segment.from_node_id).is_none() {
                errors.push(format!("{path}.from_node_id: undeclared location"));
            }
            if self.location(&segment.to_node_id).is_none() {
                errors.push(format!("{path}.to_node_id: undeclared location"));
            }
            if segment.to_node_id == self.authorization.receiver_endpoint_node_id
                || segment.from_node_id == self.authorization.receiver_endpoint_node_id
            {
                reached_endpoint = true;
            }
            match self.asset(&segment.asset_id) {
                None => errors.push(format!("{path}.asset_id: undeclared asset")),
                Some(asset) => {
                    if !asset.supported_modes.contains(&segment.mode) {
                        errors.push(format!("{path}.mode: asset does not declare this mode"));
                    }
                    if !asset_kind_supports_mode(asset.kind, segment.mode) {
                        errors.push(format!("{path}.mode: asset kind is incompatible with mode"));
                    }
                    if !self
                        .container
                        .external_dimensions_mm
                        .fits_within(&asset.maximum_payload_dimensions_mm)
                    {
                        errors.push(format!(
                            "{path}.asset_id: container external envelope exceeds asset payload dimensions"
                        ));
                    }
                    if let Some(gross_mass) =
                        self.container.tare_mass_g.checked_add(self.package.mass_g)
                        && gross_mass > asset.maximum_payload_mass_g
                    {
                        errors.push(format!(
                            "{path}.asset_id: package plus container tare exceeds asset payload mass"
                        ));
                    }
                }
            }
            if !self.package.allowed_modes.contains(&segment.mode) {
                errors.push(format!(
                    "{path}.mode: package manifest does not permit this mode"
                ));
            }
            if !self.container.supported_modes.contains(&segment.mode) {
                errors.push(format!("{path}.mode: container does not support this mode"));
            }
            expected_from = segment.to_node_id.as_str();
        }
        if expected_from != self.flow.destination_node_id {
            errors.push(
                "flow.segments: final transfer does not terminate at destination_node_id".into(),
            );
        }

        if let Some(kind) = destination.map(|location| location.kind) {
            let expected =
                expected_destination(self.package.handling_class, self.package.seal_state);
            if kind != expected {
                errors.push(format!(
                    "flow.destination_node_id: destination must be {expected:?} for this package handling/seal state"
                ));
            }
        }

        self.validate_authorization(&mut errors, reached_endpoint);
        self.validate_events_and_faults(&mut errors);

        if !self
            .success_predicates
            .contains(&SuccessPredicate::NoCriticalSafetyViolation)
        {
            errors.push(
                "success_predicates: NoCriticalSafetyViolation is a mandatory hard gate".into(),
            );
        }
        if !self
            .success_predicates
            .contains(&SuccessPredicate::NoUnauthorizedPhysicalMutation)
        {
            errors.push(
                "success_predicates: NoUnauthorizedPhysicalMutation is a mandatory hard gate"
                    .into(),
            );
        }
        if !self
            .success_predicates
            .contains(&SuccessPredicate::NoHouseholdPrivateDataDisclosure)
        {
            errors.push(
                "success_predicates: NoHouseholdPrivateDataDisclosure is a mandatory hard gate"
                    .into(),
            );
        }
        if !self
            .success_predicates
            .contains(&SuccessPredicate::PackageIdentityMatches)
        {
            errors.push("success_predicates: PackageIdentityMatches is mandatory".into());
        }
        if !self
            .success_predicates
            .contains(&SuccessPredicate::AuthorizedDestination)
        {
            errors.push("success_predicates: AuthorizedDestination is mandatory".into());
        }
        if !self
            .success_predicates
            .contains(&SuccessPredicate::RequiredCustodyComplete)
            && !self
                .success_predicates
                .contains(&SuccessPredicate::SafeHoldReached)
        {
            errors.push(
                "success_predicates: require RequiredCustodyComplete or SafeHoldReached".into(),
            );
        }
        check_unique(
            &mut errors,
            "success_predicates",
            self.success_predicates.iter().map(predicate_name),
        );
        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    fn validate_authorization(&self, errors: &mut Vec<String>, reached_endpoint: bool) {
        let auth = &self.authorization;
        check_digest(errors, "authorization.grant_digest", &auth.grant_digest);
        check_digest(
            errors,
            "authorization.household_scope_digest",
            &auth.household_scope_digest,
        );
        check_digest(errors, "authorization.policy_digest", &auth.policy_digest);

        match self.actor(&auth.principal_actor_id) {
            Some(actor) if actor.role == ActorRole::HouseholdMember => {}
            Some(_) => errors.push(
                "authorization.principal_actor_id: actor must have HouseholdMember role".into(),
            ),
            None => errors.push("authorization.principal_actor_id: undeclared actor".into()),
        }
        match self.actor(&auth.audience_actor_id) {
            Some(actor) if actor.role == ActorRole::ServiceProvider => {}
            Some(_) => errors.push(
                "authorization.audience_actor_id: actor must have ServiceProvider role".into(),
            ),
            None => errors.push("authorization.audience_actor_id: undeclared actor".into()),
        }
        match self.actor(&auth.receiver_device_actor_id) {
            Some(actor) if actor.role == ActorRole::ReceiverDevice => {}
            Some(_) => errors.push(
                "authorization.receiver_device_actor_id: actor must have ReceiverDevice role"
                    .into(),
            ),
            None => errors.push("authorization.receiver_device_actor_id: undeclared actor".into()),
        }
        match self.location(&auth.receiver_endpoint_node_id) {
            Some(location) if location.kind == LocationKind::ExternalReceiver => {}
            Some(_) => errors.push(
                "authorization.receiver_endpoint_node_id: endpoint must be ExternalReceiver".into(),
            ),
            None => {
                errors.push("authorization.receiver_endpoint_node_id: undeclared location".into())
            }
        }
        if !reached_endpoint {
            errors.push(
                "authorization.receiver_endpoint_node_id: endpoint is not part of the transfer chain"
                    .into(),
            );
        }
        if auth.not_before_unix_ms > self.started_at_unix_ms {
            errors.push(
                "authorization.not_before_unix_ms: grant is not yet valid at scenario start".into(),
            );
        }
        if auth.expires_at_unix_ms <= self.started_at_unix_ms
            || auth.expires_at_unix_ms <= auth.not_before_unix_ms
        {
            errors.push(
                "authorization.expires_at_unix_ms: grant is expired or has an invalid window"
                    .into(),
            );
        }
        check_nonempty(errors, "authorization.actions", auth.actions.len());
        check_unique(
            errors,
            "authorization.actions",
            auth.actions.iter().map(action_name),
        );
        if !auth
            .actions
            .contains(&AuthorizationAction::ReceiveExternalPackage)
        {
            errors.push("authorization.actions: ReceiveExternalPackage is required".into());
        }
        if !auth.actions.contains(&AuthorizationAction::TransferToTote) {
            errors.push("authorization.actions: TransferToTote is required".into());
        }
        let required_destination_action = match self.location(&self.flow.destination_node_id) {
            Some(location) => match location.kind {
                LocationKind::AmbientStorage => Some(AuthorizationAction::PutAwayAmbient),
                LocationKind::ChilledStorage => Some(AuthorizationAction::PutAwayChilled),
                LocationKind::FrozenStorage => Some(AuthorizationAction::PutAwayFrozen),
                LocationKind::Quarantine => Some(AuthorizationAction::Quarantine),
                _ => None,
            },
            None => None,
        };
        if let Some(action) = required_destination_action
            && !auth.actions.contains(&action)
        {
            errors.push(
                "authorization.actions: missing action required by destination storage class"
                    .into(),
            );
        }
        if let UseLimit::Bounded { maximum_uses } = auth.use_limit
            && maximum_uses == 0
        {
            errors.push("authorization.use_limit.maximum_uses: must be greater than zero".into());
        }
    }

    fn validate_events_and_faults(&self, errors: &mut Vec<String>) {
        check_nonempty(errors, "required_events", self.required_events.len());
        check_unique(
            errors,
            "required_events",
            self.required_events.iter().map(event_name),
        );
        if !self
            .required_events
            .contains(&RequiredEvent::AuthorizationChecked)
        {
            errors.push("required_events: AuthorizationChecked is mandatory".into());
        }
        if !self
            .required_events
            .contains(&RequiredEvent::SafetyDecisionRecorded)
        {
            errors.push("required_events: SafetyDecisionRecorded is mandatory".into());
        }

        check_unique(
            errors,
            "fault_injections.fault_id",
            self.fault_injections
                .iter()
                .map(|fault| fault.fault_id.as_str()),
        );
        for (index, fault) in self.fault_injections.iter().enumerate() {
            check_identifier(
                errors,
                &format!("fault_injections[{index}].fault_id"),
                &fault.fault_id,
            );
            if let Some(segment_id) = &fault.at_segment_id
                && !self
                    .flow
                    .segments
                    .iter()
                    .any(|segment| segment.segment_id == *segment_id)
            {
                errors.push(format!(
                    "fault_injections[{index}].at_segment_id: undeclared segment"
                ));
            }
        }
    }

    fn actor(&self, actor_id: &str) -> Option<&ActorBinding> {
        self.actors.iter().find(|actor| actor.actor_id == actor_id)
    }

    fn asset(&self, asset_id: &str) -> Option<&AssetBinding> {
        self.assets.iter().find(|asset| asset.asset_id == asset_id)
    }

    fn location(&self, node_id: &str) -> Option<&LocationBinding> {
        self.locations
            .iter()
            .find(|location| location.node_id == node_id)
    }
}

fn expected_destination(handling: HandlingClass, seal: SealState) -> LocationKind {
    if matches!(seal, SealState::Damaged | SealState::Unknown)
        || matches!(
            handling,
            HandlingClass::Restricted | HandlingClass::HumanReview
        )
    {
        LocationKind::Quarantine
    } else {
        match handling {
            HandlingClass::Chilled => LocationKind::ChilledStorage,
            HandlingClass::Frozen => LocationKind::FrozenStorage,
            HandlingClass::Ambient | HandlingClass::Fragile => LocationKind::AmbientStorage,
            HandlingClass::Restricted | HandlingClass::HumanReview => LocationKind::Quarantine,
        }
    }
}

fn asset_kind_supports_mode(kind: AssetKind, mode: TransportMode) -> bool {
    matches!(
        (kind, mode),
        (AssetKind::RoadCarrier, TransportMode::Road)
            | (AssetKind::Aircraft, TransportMode::Aerial)
            | (AssetKind::GroundRobot, TransportMode::GroundRobot)
            | (AssetKind::TransferMechanism, TransportMode::FixedTransfer)
            | (AssetKind::StorageSystem, TransportMode::FixedTransfer)
    )
}

fn check_identifier(errors: &mut Vec<String>, path: &str, value: &str) {
    let valid = !value.is_empty()
        && value.len() <= 96
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b':'));
    if !valid {
        errors.push(format!(
            "{path}: must be a non-empty opaque ASCII identifier (max 96 bytes)"
        ));
    }
}

fn check_digest(errors: &mut Vec<String>, path: &str, value: &str) {
    let Some(hex) = value.strip_prefix("sha256:") else {
        errors.push(format!(
            "{path}: expected sha256:<64 lowercase hex characters>"
        ));
        return;
    };
    if hex.len() != 64
        || !hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        errors.push(format!(
            "{path}: expected sha256:<64 lowercase hex characters>"
        ));
    }
}

fn check_hex(
    errors: &mut Vec<String>,
    path: &str,
    value: &str,
    expected_len: usize,
    require_prefix: bool,
) {
    let hex = if require_prefix {
        value.strip_prefix("0x").unwrap_or(value)
    } else {
        value
    };
    if hex.len() != expected_len
        || !hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        errors.push(format!(
            "{path}: expected {expected_len} lowercase hexadecimal characters"
        ));
    }
}

fn check_nonempty(errors: &mut Vec<String>, path: &str, count: usize) {
    if count == 0 {
        errors.push(format!("{path}: must not be empty"));
    }
}

fn check_unique<'a>(errors: &mut Vec<String>, path: &str, values: impl Iterator<Item = &'a str>) {
    let mut seen = HashSet::new();
    for value in values {
        if !seen.insert(value) {
            errors.push(format!("{path}: duplicate value"));
            return;
        }
    }
}

fn mode_name(mode: TransportMode) -> &'static str {
    match mode {
        TransportMode::Road => "road",
        TransportMode::Aerial => "aerial",
        TransportMode::GroundRobot => "ground_robot",
        TransportMode::FixedTransfer => "fixed_transfer",
    }
}

fn action_name(action: &AuthorizationAction) -> &'static str {
    match action {
        AuthorizationAction::ReceiveExternalPackage => "receive_external_package",
        AuthorizationAction::TransferToTote => "transfer_to_tote",
        AuthorizationAction::PutAwayAmbient => "put_away_ambient",
        AuthorizationAction::PutAwayChilled => "put_away_chilled",
        AuthorizationAction::PutAwayFrozen => "put_away_frozen",
        AuthorizationAction::Quarantine => "quarantine",
        AuthorizationAction::ReturnContainer => "return_container",
    }
}

fn event_name(event: &RequiredEvent) -> &'static str {
    match event {
        RequiredEvent::AuthorizationChecked => "authorization_checked",
        RequiredEvent::PackageReleased => "package_released",
        RequiredEvent::PackageReceived => "package_received",
        RequiredEvent::TransferVerified => "transfer_verified",
        RequiredEvent::PlacementVerified => "placement_verified",
        RequiredEvent::CustodyAcknowledged => "custody_acknowledged",
        RequiredEvent::CustodyExceptionRecorded => "custody_exception_recorded",
        RequiredEvent::SafetyDecisionRecorded => "safety_decision_recorded",
    }
}

fn predicate_name(predicate: &SuccessPredicate) -> &'static str {
    match predicate {
        SuccessPredicate::PackageIdentityMatches => "package_identity_matches",
        SuccessPredicate::AuthorizedDestination => "authorized_destination",
        SuccessPredicate::RequiredCustodyComplete => "required_custody_complete",
        SuccessPredicate::SafeHoldReached => "safe_hold_reached",
        SuccessPredicate::NoCriticalSafetyViolation => "no_critical_safety_violation",
        SuccessPredicate::NoUnauthorizedPhysicalMutation => "no_unauthorized_physical_mutation",
        SuccessPredicate::NoHouseholdPrivateDataDisclosure => {
            "no_household_private_data_disclosure"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../fixtures/nominal-stationary-carrier.json");

    fn valid_manifest() -> ScenarioManifest {
        serde_json::from_str(FIXTURE).expect("fixture must deserialize")
    }

    fn assert_has_error(result: Result<(), Vec<String>>, expected: &str) {
        let errors = result.expect_err("expected validation to fail");
        assert!(
            errors.iter().any(|error| error.contains(expected)),
            "expected error containing {expected:?}, got {errors:?}"
        );
    }

    const CAMPAIGN_FIXTURE: &str = include_str!("../fixtures/ten-fault-campaign.json");

    fn valid_campaign() -> FaultCampaignManifest {
        serde_json::from_str(CAMPAIGN_FIXTURE).expect("campaign fixture must deserialize")
    }

    fn assert_campaign_error(result: Result<(), Vec<String>>, expected: &str) {
        let errors = result.expect_err("expected campaign validation to fail");
        assert!(
            errors.iter().any(|error| error.contains(expected)),
            "expected error containing {expected:?}, got {errors:?}"
        );
    }

    #[test]
    fn ten_fault_campaign_fixture_is_valid() {
        let scenario = valid_manifest();
        valid_campaign()
            .validate_against(&scenario)
            .expect("ten-fault campaign should validate against nominal scenario");
    }

    #[test]
    fn campaign_rejects_missing_fault_episode() {
        let scenario = valid_manifest();
        let mut campaign = valid_campaign();
        campaign.episodes.pop();
        assert_campaign_error(
            campaign.validate_against(&scenario),
            "expected exactly 10 independent v1 fault episodes",
        );
    }

    #[test]
    fn campaign_rejects_duplicate_fault_family() {
        let scenario = valid_manifest();
        let mut campaign = valid_campaign();
        campaign.episodes[1].injection.kind = campaign.episodes[0].injection.kind;
        assert_campaign_error(
            campaign.validate_against(&scenario),
            "duplicate fault family",
        );
    }

    #[test]
    fn campaign_rejects_wrong_terminal_oracle() {
        let scenario = valid_manifest();
        let mut campaign = valid_campaign();
        campaign.episodes[0].expected_terminal_state =
            ExpectedTerminalState::ReconciliationRequired;
        assert_campaign_error(campaign.validate_against(&scenario), "is required for");
    }

    #[test]
    fn campaign_rejects_mismatched_base_scenario() {
        let scenario = valid_manifest();
        let mut campaign = valid_campaign();
        campaign.base_scenario_id = "different-scenario".into();
        assert_campaign_error(
            campaign.validate_against(&scenario),
            "does not match the supplied scenario",
        );
    }

    #[test]
    fn campaign_rejects_unknown_segment_reference() {
        let scenario = valid_manifest();
        let mut campaign = valid_campaign();
        campaign.episodes[0].injection.at_segment_id = Some("missing-segment".into());
        assert_campaign_error(campaign.validate_against(&scenario), "undeclared segment");
    }

    #[test]
    fn campaign_rejects_unknown_json_fields() {
        let mut value: serde_json::Value = serde_json::from_str(CAMPAIGN_FIXTURE).unwrap();
        value["unrecognized"] = serde_json::json!(true);
        assert!(serde_json::from_value::<FaultCampaignManifest>(value).is_err());
    }

    #[test]
    fn nominal_fixture_is_valid() {
        valid_manifest()
            .validate()
            .expect("nominal fixture should validate");
    }

    #[test]
    fn rejects_unknown_schema_version() {
        let mut manifest = valid_manifest();
        manifest.schema_version = 2;
        assert_has_error(manifest.validate(), "schema_version: expected 1");
    }

    #[test]
    fn rejects_malformed_digest() {
        let mut manifest = valid_manifest();
        manifest.digests.model_digest = "sha256:xyz".into();
        assert_has_error(manifest.validate(), "digests.model_digest");
    }

    #[test]
    fn rejects_non_hex_git_commit() {
        let mut manifest = valid_manifest();
        manifest.digests.source_commit = "g".repeat(40);
        assert_has_error(manifest.validate(), "digests.source_commit");
    }

    #[test]
    fn rejects_package_that_does_not_fit_tote() {
        let mut manifest = valid_manifest();
        manifest.package.dimensions_mm.length_mm = 301;
        assert_has_error(manifest.validate(), "package does not fit");
    }

    #[test]
    fn rejects_asset_payload_mass_limit() {
        let mut manifest = valid_manifest();
        manifest.assets[0].maximum_payload_mass_g = 1_800;
        assert_has_error(
            manifest.validate(),
            "package plus container tare exceeds asset payload mass",
        );
    }

    #[test]
    fn rejects_container_exceeding_asset_dimension_envelope() {
        let mut manifest = valid_manifest();
        manifest.assets[1].maximum_payload_dimensions_mm.length_mm = 300;
        assert_has_error(
            manifest.validate(),
            "container external envelope exceeds asset payload dimensions",
        );
    }

    #[test]
    fn rejects_external_container_dimensions_smaller_than_internal() {
        let mut manifest = valid_manifest();
        manifest.container.external_dimensions_mm.length_mm = 250;
        assert_has_error(
            manifest.validate(),
            "external dimensions must contain the declared internal envelope",
        );
    }

    #[test]
    fn rejects_package_plus_tare_over_gross_limit() {
        let mut manifest = valid_manifest();
        manifest.container.maximum_gross_mass_g = 1_800;
        assert_has_error(manifest.validate(), "package plus tare exceeds");
    }

    #[test]
    fn rejects_discontinuous_transfer_chain() {
        let mut manifest = valid_manifest();
        manifest.flow.segments[1].from_node_id = "carrier-bay-01".into();
        assert_has_error(manifest.validate(), "transfer chain is discontinuous");
    }

    #[test]
    fn rejects_unknown_asset_reference() {
        let mut manifest = valid_manifest();
        manifest.flow.segments[0].asset_id = "missing-asset".into();
        assert_has_error(manifest.validate(), "undeclared asset");
    }

    #[test]
    fn rejects_mode_not_supported_by_asset() {
        let mut manifest = valid_manifest();
        manifest.flow.segments[0].mode = TransportMode::Aerial;
        assert_has_error(manifest.validate(), "asset does not declare this mode");
    }

    #[test]
    fn rejects_wrong_asset_kind_for_mode() {
        let mut manifest = valid_manifest();
        manifest.assets[0]
            .supported_modes
            .push(TransportMode::Aerial);
        manifest.package.allowed_modes.push(TransportMode::Aerial);
        manifest
            .container
            .supported_modes
            .push(TransportMode::Aerial);
        manifest.flow.segments[0].mode = TransportMode::Aerial;
        assert_has_error(manifest.validate(), "asset kind is incompatible with mode");
    }

    #[test]
    fn rejects_expired_household_grant() {
        let mut manifest = valid_manifest();
        manifest.authorization.expires_at_unix_ms = manifest.started_at_unix_ms;
        assert_has_error(manifest.validate(), "grant is expired");
    }

    #[test]
    fn rejects_wrong_audience_role() {
        let mut manifest = valid_manifest();
        manifest.authorization.audience_actor_id = "household-01".into();
        assert_has_error(
            manifest.validate(),
            "audience_actor_id: actor must have ServiceProvider",
        );
    }

    #[test]
    fn rejects_grant_for_wrong_receiver_endpoint() {
        let mut manifest = valid_manifest();
        manifest.authorization.receiver_endpoint_node_id = "unlisted-endpoint".into();
        assert_has_error(
            manifest.validate(),
            "endpoint is not part of the transfer chain",
        );
    }

    #[test]
    fn rejects_damaged_package_to_ambient_storage() {
        let mut manifest = valid_manifest();
        manifest.package.seal_state = SealState::Damaged;
        assert_has_error(manifest.validate(), "destination must be Quarantine");
    }

    #[test]
    fn rejects_missing_required_safety_predicate() {
        let mut manifest = valid_manifest();
        manifest
            .success_predicates
            .retain(|item| *item != SuccessPredicate::NoCriticalSafetyViolation);
        assert_has_error(
            manifest.validate(),
            "NoCriticalSafetyViolation is a mandatory hard gate",
        );
    }

    #[test]
    fn rejects_missing_household_privacy_predicate() {
        let mut manifest = valid_manifest();
        manifest
            .success_predicates
            .retain(|item| *item != SuccessPredicate::NoHouseholdPrivateDataDisclosure);
        assert_has_error(
            manifest.validate(),
            "NoHouseholdPrivateDataDisclosure is a mandatory hard gate",
        );
    }

    #[test]
    fn rejects_missing_authorized_destination_predicate() {
        let mut manifest = valid_manifest();
        manifest
            .success_predicates
            .retain(|item| *item != SuccessPredicate::AuthorizedDestination);
        assert_has_error(manifest.validate(), "AuthorizedDestination is mandatory");
    }

    #[test]
    fn rejects_unknown_fault_segment() {
        let mut manifest = valid_manifest();
        manifest.fault_injections.push(FaultInjection {
            fault_id: "fault-01".into(),
            kind: FaultKind::NetworkLoss,
            at_segment_id: Some("missing-segment".into()),
        });
        assert_has_error(manifest.validate(), "at_segment_id: undeclared segment");
    }

    #[test]
    fn rejects_zero_bounded_grant_uses() {
        let mut manifest = valid_manifest();
        manifest.authorization.use_limit = UseLimit::Bounded { maximum_uses: 0 };
        assert_has_error(
            manifest.validate(),
            "maximum_uses: must be greater than zero",
        );
    }

    #[test]
    fn rejects_unknown_json_fields() {
        let mut value: serde_json::Value = serde_json::from_str(FIXTURE).unwrap();
        value["unrecognized"] = serde_json::json!(true);
        assert!(serde_json::from_value::<ScenarioManifest>(value).is_err());
    }

    #[test]
    fn accepts_safe_hold_as_terminal_predicate_for_fault_episode() {
        let mut manifest = valid_manifest();
        manifest
            .success_predicates
            .retain(|item| *item != SuccessPredicate::RequiredCustodyComplete);
        manifest
            .success_predicates
            .push(SuccessPredicate::SafeHoldReached);
        manifest.fault_injections.push(FaultInjection {
            fault_id: "fault-network-loss".into(),
            kind: FaultKind::NetworkLoss,
            at_segment_id: Some("segment-carrier-to-receiver".into()),
        });
        manifest
            .required_events
            .retain(|event| *event != RequiredEvent::CustodyAcknowledged);
        manifest
            .required_events
            .push(RequiredEvent::CustodyExceptionRecorded);
        manifest
            .validate()
            .expect("safe-hold fault scenario should validate");
    }
}
