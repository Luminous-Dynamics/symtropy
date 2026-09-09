// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Canonical spatial/temporal sufficiency for ecological information.
//!
//! Information identity and evidence strength are necessary but not sufficient
//! for authoritative process execution. Exact occupancy at kilometre support is
//! not exact contact state at centimetre support, and stale state does not become
//! current because the renderer produced a new frame. This successor layer keeps
//! the #263 information algebra frozen while adding canonical integer spatial
//! support, canonical-tick freshness/cadence, and explicit integration semantics.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

use crate::information::{EcologicalInformation, ProcessKey, RepresentationKey};
use crate::information_policy_manifest::{
    InformationPolicyAuthorityStamp, InformationPolicyIdentityError,
    ManifestBoundInformationPolicyRegistry,
};
use crate::information_registry::InformationRegistryError;

/// Canonical simulation tick. Wall-clock/render-frame time is deliberately absent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CanonicalTick(pub u64);

/// Integer spatial resolution/support in one versioned simulation world unit.
/// Smaller values are finer resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SpatialResolutionUnits(u64);

impl SpatialResolutionUnits {
    pub const fn new(units: u64) -> Result<Self, SpatiotemporalPolicyError> {
        if units == 0 {
            return Err(SpatiotemporalPolicyError::ZeroSpatialResolution);
        }
        Ok(Self(units))
    }

    pub const fn get(self) -> u64 {
        self.0
    }

    pub const fn is_fine_enough_for(self, required: Self) -> bool {
        self.0 <= required.0
    }
}

/// Maximum canonical age of state accepted by a process. Zero means current tick only.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MaximumStateAgeTicks(pub u64);

/// Maximum update/cadence interval accepted by a process.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MaximumUpdateIntervalTicks(u64);

impl MaximumUpdateIntervalTicks {
    pub const fn new(ticks: u64) -> Result<Self, SpatiotemporalPolicyError> {
        if ticks == 0 {
            return Err(SpatiotemporalPolicyError::ZeroUpdateInterval);
        }
        Ok(Self(ticks))
    }

    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Exact aggregation/integration window when the semantics require one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct TemporalAggregationWindowTicks(u64);

impl TemporalAggregationWindowTicks {
    pub const fn new(ticks: u64) -> Result<Self, SpatiotemporalPolicyError> {
        if ticks == 0 {
            return Err(SpatiotemporalPolicyError::ZeroAggregationWindow);
        }
        Ok(Self(ticks))
    }

    pub const fn get(self) -> u64 {
        self.0
    }
}

/// Optional calibrated spatial/temporal applicability-domain identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ResolutionDomainKey {
    id: u128,
    version: u32,
}

impl ResolutionDomainKey {
    pub const fn new(id: u128, version: u32) -> Self {
        Self { id, version }
    }

    pub const fn id(self) -> u128 {
        self.id
    }

    pub const fn version(self) -> u32 {
        self.version
    }
}

/// Integration semantics are exact authority vocabulary, not labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum IntegrationSemantics {
    Instantaneous,
    MeanOverWindow,
    AccumulatedExposure,
    MaximumOverWindow,
    MinimumOverWindow,
    ThresholdCrossing,
}

fn validate_window(
    semantics: IntegrationSemantics,
    window: Option<TemporalAggregationWindowTicks>,
) -> Result<(), SpatiotemporalPolicyError> {
    match (semantics, window) {
        (IntegrationSemantics::Instantaneous, None) => Ok(()),
        (IntegrationSemantics::Instantaneous, Some(_)) => {
            Err(SpatiotemporalPolicyError::InstantaneousHasAggregationWindow)
        }
        (_, Some(_)) => Ok(()),
        (_, None) => Err(SpatiotemporalPolicyError::MissingAggregationWindow {
            semantics,
        }),
    }
}

/// Spatial/temporal contract declared by one registered process for one information kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ProcessSpatiotemporalRequirement {
    process: ProcessKey,
    information: EcologicalInformation,
    maximum_spatial_resolution: SpatialResolutionUnits,
    maximum_state_age: MaximumStateAgeTicks,
    maximum_update_interval: MaximumUpdateIntervalTicks,
    integration: IntegrationSemantics,
    aggregation_window: Option<TemporalAggregationWindowTicks>,
    required_domain: Option<ResolutionDomainKey>,
}

impl ProcessSpatiotemporalRequirement {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        process: ProcessKey,
        information: EcologicalInformation,
        maximum_spatial_resolution: SpatialResolutionUnits,
        maximum_state_age: MaximumStateAgeTicks,
        maximum_update_interval: MaximumUpdateIntervalTicks,
        integration: IntegrationSemantics,
        aggregation_window: Option<TemporalAggregationWindowTicks>,
        required_domain: Option<ResolutionDomainKey>,
    ) -> Result<Self, SpatiotemporalPolicyError> {
        validate_window(integration, aggregation_window)?;
        Ok(Self {
            process,
            information,
            maximum_spatial_resolution,
            maximum_state_age,
            maximum_update_interval,
            integration,
            aggregation_window,
            required_domain,
        })
    }

    pub const fn process(self) -> ProcessKey {
        self.process
    }

    pub const fn information(self) -> EcologicalInformation {
        self.information
    }

    pub const fn maximum_spatial_resolution(self) -> SpatialResolutionUnits {
        self.maximum_spatial_resolution
    }

    pub const fn maximum_state_age(self) -> MaximumStateAgeTicks {
        self.maximum_state_age
    }

    pub const fn maximum_update_interval(self) -> MaximumUpdateIntervalTicks {
        self.maximum_update_interval
    }

    pub const fn integration(self) -> IntegrationSemantics {
        self.integration
    }

    pub const fn aggregation_window(self) -> Option<TemporalAggregationWindowTicks> {
        self.aggregation_window
    }

    pub const fn required_domain(self) -> Option<ResolutionDomainKey> {
        self.required_domain
    }
}

/// Static spatial/temporal support declared for one representation capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RepresentationSpatiotemporalCapability {
    representation: RepresentationKey,
    information: EcologicalInformation,
    spatial_resolution: SpatialResolutionUnits,
    update_interval: MaximumUpdateIntervalTicks,
    integration: IntegrationSemantics,
    aggregation_window: Option<TemporalAggregationWindowTicks>,
    domain: Option<ResolutionDomainKey>,
}

impl RepresentationSpatiotemporalCapability {
    pub fn new(
        representation: RepresentationKey,
        information: EcologicalInformation,
        spatial_resolution: SpatialResolutionUnits,
        update_interval: MaximumUpdateIntervalTicks,
        integration: IntegrationSemantics,
        aggregation_window: Option<TemporalAggregationWindowTicks>,
        domain: Option<ResolutionDomainKey>,
    ) -> Result<Self, SpatiotemporalPolicyError> {
        validate_window(integration, aggregation_window)?;
        Ok(Self {
            representation,
            information,
            spatial_resolution,
            update_interval,
            integration,
            aggregation_window,
            domain,
        })
    }

    pub const fn representation(self) -> RepresentationKey {
        self.representation
    }

    pub const fn information(self) -> EcologicalInformation {
        self.information
    }

    pub const fn spatial_resolution(self) -> SpatialResolutionUnits {
        self.spatial_resolution
    }

    pub const fn update_interval(self) -> MaximumUpdateIntervalTicks {
        self.update_interval
    }

    pub const fn integration(self) -> IntegrationSemantics {
        self.integration
    }

    pub const fn aggregation_window(self) -> Option<TemporalAggregationWindowTicks> {
        self.aggregation_window
    }

    pub const fn domain(self) -> Option<ResolutionDomainKey> {
        self.domain
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SpatiotemporalPolicyRegistryKey {
    id: u128,
    version: u32,
}

impl SpatiotemporalPolicyRegistryKey {
    pub const fn new(id: u128, version: u32) -> Self {
        Self { id, version }
    }

    pub const fn id(self) -> u128 {
        self.id
    }

    pub const fn version(self) -> u32 {
        self.version
    }
}

/// Duplicate-safe current-state timing evidence for one representation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpatiotemporalStateContext {
    representation: RepresentationKey,
    state_ticks: BTreeMap<EcologicalInformation, CanonicalTick>,
}

impl SpatiotemporalStateContext {
    pub fn from_records(
        representation: RepresentationKey,
        records: impl IntoIterator<Item = (EcologicalInformation, CanonicalTick)>,
    ) -> Result<Self, SpatiotemporalPolicyError> {
        use std::collections::btree_map::Entry;

        let mut state_ticks = BTreeMap::new();
        for (information, tick) in records {
            match state_ticks.entry(information) {
                Entry::Vacant(entry) => {
                    entry.insert(tick);
                }
                Entry::Occupied(_) => {
                    return Err(SpatiotemporalPolicyError::DuplicateStateTick { information });
                }
            }
        }
        Ok(Self {
            representation,
            state_ticks,
        })
    }

    pub const fn representation(&self) -> RepresentationKey {
        self.representation
    }

    pub fn state_tick(&self, information: EcologicalInformation) -> Option<CanonicalTick> {
        self.state_ticks.get(&information).copied()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpatiotemporalPolicyRegistryBuilder {
    key: SpatiotemporalPolicyRegistryKey,
    process_requirements: BTreeMap<(ProcessKey, EcologicalInformation), ProcessSpatiotemporalRequirement>,
    representation_capabilities:
        BTreeMap<(RepresentationKey, EcologicalInformation), RepresentationSpatiotemporalCapability>,
}

impl SpatiotemporalPolicyRegistryBuilder {
    pub const fn new(key: SpatiotemporalPolicyRegistryKey) -> Self {
        Self {
            key,
            process_requirements: BTreeMap::new(),
            representation_capabilities: BTreeMap::new(),
        }
    }

    pub fn register_process_requirement(
        &mut self,
        requirement: ProcessSpatiotemporalRequirement,
    ) -> Result<(), SpatiotemporalPolicyError> {
        use std::collections::btree_map::Entry;

        let key = (requirement.process(), requirement.information());
        match self.process_requirements.entry(key) {
            Entry::Vacant(entry) => {
                entry.insert(requirement);
                Ok(())
            }
            Entry::Occupied(entry) if entry.get() == &requirement => Ok(()),
            Entry::Occupied(_) => Err(SpatiotemporalPolicyError::ConflictingProcessRequirement {
                process: key.0,
                information: key.1,
            }),
        }
    }

    pub fn register_representation_capability(
        &mut self,
        capability: RepresentationSpatiotemporalCapability,
    ) -> Result<(), SpatiotemporalPolicyError> {
        use std::collections::btree_map::Entry;

        let key = (capability.representation(), capability.information());
        match self.representation_capabilities.entry(key) {
            Entry::Vacant(entry) => {
                entry.insert(capability);
                Ok(())
            }
            Entry::Occupied(entry) if entry.get() == &capability => Ok(()),
            Entry::Occupied(_) => {
                Err(SpatiotemporalPolicyError::ConflictingRepresentationCapability {
                    representation: key.0,
                    information: key.1,
                })
            }
        }
    }

    pub fn seal(
        self,
        policy: &ManifestBoundInformationPolicyRegistry<'_>,
    ) -> Result<SpatiotemporalPolicyRegistry, SpatiotemporalPolicyError> {
        policy
            .authority_stamp()
            .validate_registry(policy.registry())
            .map_err(SpatiotemporalPolicyError::PolicyIdentity)?;

        for requirement in self.process_requirements.values() {
            let process = policy
                .registry()
                .resolve_process(requirement.process())
                .map_err(SpatiotemporalPolicyError::InformationRegistry)?;
            if !process
                .profile()
                .requirements()
                .iter()
                .any(|candidate| candidate.information() == requirement.information())
            {
                return Err(SpatiotemporalPolicyError::ProcessDoesNotRequireInformation {
                    process: requirement.process(),
                    information: requirement.information(),
                });
            }
        }

        for capability in self.representation_capabilities.values() {
            let representation = policy
                .registry()
                .resolve_representation(capability.representation())
                .map_err(SpatiotemporalPolicyError::InformationRegistry)?;
            let carries = representation
                .capabilities()
                .claims()
                .keys()
                .copied()
                .any(|available| available.covers(capability.information()));
            if !carries {
                return Err(SpatiotemporalPolicyError::RepresentationDoesNotCarryInformation {
                    representation: capability.representation(),
                    information: capability.information(),
                });
            }
        }

        let authority = SpatiotemporalPolicyAuthorityStamp {
            key: self.key,
            information_policy_authority: policy.authority_stamp().clone(),
            process_requirements: self.process_requirements,
            representation_capabilities: self.representation_capabilities,
        };
        Ok(SpatiotemporalPolicyRegistry { authority })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpatiotemporalPolicyAuthorityStamp {
    key: SpatiotemporalPolicyRegistryKey,
    information_policy_authority: InformationPolicyAuthorityStamp,
    process_requirements: BTreeMap<(ProcessKey, EcologicalInformation), ProcessSpatiotemporalRequirement>,
    representation_capabilities:
        BTreeMap<(RepresentationKey, EcologicalInformation), RepresentationSpatiotemporalCapability>,
}

impl SpatiotemporalPolicyAuthorityStamp {
    pub const fn key(&self) -> SpatiotemporalPolicyRegistryKey {
        self.key
    }

    pub const fn information_policy_authority(&self) -> &InformationPolicyAuthorityStamp {
        &self.information_policy_authority
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpatiotemporalPolicyRegistry {
    authority: SpatiotemporalPolicyAuthorityStamp,
}

impl SpatiotemporalPolicyRegistry {
    pub const fn authority_stamp(&self) -> &SpatiotemporalPolicyAuthorityStamp {
        &self.authority
    }

    pub fn validate_current(
        &self,
        policy: &ManifestBoundInformationPolicyRegistry<'_>,
    ) -> Result<(), SpatiotemporalPolicyError> {
        self.authority
            .information_policy_authority
            .validate_registry(policy.registry())
            .map_err(SpatiotemporalPolicyError::PolicyIdentity)?;
        if policy.authority_stamp() != &self.authority.information_policy_authority {
            return Err(SpatiotemporalPolicyError::PolicyAuthorityMismatch);
        }
        Ok(())
    }

    pub fn evaluate(
        &self,
        policy: &ManifestBoundInformationPolicyRegistry<'_>,
        processes: impl IntoIterator<Item = ProcessKey>,
        representation: RepresentationKey,
        current_tick: CanonicalTick,
        state: &SpatiotemporalStateContext,
    ) -> Result<SpatiotemporalSufficiencyReport, SpatiotemporalPolicyError> {
        self.validate_current(policy)?;
        policy
            .registry()
            .resolve_representation(representation)
            .map_err(SpatiotemporalPolicyError::InformationRegistry)?;
        if state.representation() != representation {
            return Err(SpatiotemporalPolicyError::StateRepresentationMismatch {
                expected: representation,
                actual: state.representation(),
            });
        }

        let process_keys = processes.into_iter().collect::<BTreeSet<_>>();
        let mut failures = Vec::new();

        for process_key in &process_keys {
            let process = policy
                .registry()
                .resolve_process(*process_key)
                .map_err(SpatiotemporalPolicyError::InformationRegistry)?;

            for information_requirement in process.profile().requirements() {
                let information = information_requirement.information();
                let requirement = match self
                    .authority
                    .process_requirements
                    .get(&(*process_key, information))
                    .copied()
                {
                    Some(requirement) => requirement,
                    None => {
                        failures.push(SpatiotemporalFailure::MissingProcessRequirement {
                            process: *process_key,
                            information,
                        });
                        continue;
                    }
                };

                let candidates = self
                    .authority
                    .representation_capabilities
                    .values()
                    .copied()
                    .filter(|capability| {
                        capability.representation() == representation
                            && capability.information().covers(information)
                    })
                    .collect::<Vec<_>>();

                if candidates.is_empty() {
                    failures.push(SpatiotemporalFailure::MissingRepresentationCapability {
                        process: *process_key,
                        representation,
                        information,
                    });
                    continue;
                }

                let mut candidate_failures = Vec::new();
                let mut satisfied = false;
                for capability in candidates {
                    let tick = match state.state_tick(capability.information()) {
                        Some(tick) => tick,
                        None => {
                            candidate_failures.push(
                                SpatiotemporalFailure::MissingStateTick {
                                    process: *process_key,
                                    representation,
                                    information: capability.information(),
                                },
                            );
                            continue;
                        }
                    };
                    match evaluate_one(
                        *process_key,
                        requirement,
                        capability,
                        current_tick,
                        tick,
                    ) {
                        Ok(()) => {
                            satisfied = true;
                            break;
                        }
                        Err(failure) => candidate_failures.push(failure),
                    }
                }

                if !satisfied {
                    failures.extend(candidate_failures);
                }
            }
        }

        Ok(SpatiotemporalSufficiencyReport {
            authority: self.authority.clone(),
            process_keys,
            representation,
            current_tick,
            state: state.clone(),
            failures,
        })
    }
}

fn evaluate_one(
    process: ProcessKey,
    requirement: ProcessSpatiotemporalRequirement,
    capability: RepresentationSpatiotemporalCapability,
    current_tick: CanonicalTick,
    state_tick: CanonicalTick,
) -> Result<(), SpatiotemporalFailure> {
    if !capability
        .spatial_resolution()
        .is_fine_enough_for(requirement.maximum_spatial_resolution())
    {
        return Err(SpatiotemporalFailure::SpatialResolutionTooCoarse {
            process,
            information: requirement.information(),
            required: requirement.maximum_spatial_resolution(),
            available: capability.spatial_resolution(),
        });
    }

    if capability.update_interval().get() > requirement.maximum_update_interval().get() {
        return Err(SpatiotemporalFailure::UpdateIntervalTooSlow {
            process,
            information: requirement.information(),
            required: requirement.maximum_update_interval(),
            available: capability.update_interval(),
        });
    }

    if capability.integration() != requirement.integration()
        || capability.aggregation_window() != requirement.aggregation_window()
    {
        return Err(SpatiotemporalFailure::IntegrationSemanticsMismatch {
            process,
            information: requirement.information(),
            required: requirement.integration(),
            available: capability.integration(),
            required_window: requirement.aggregation_window(),
            available_window: capability.aggregation_window(),
        });
    }

    if capability.domain() != requirement.required_domain() {
        return Err(SpatiotemporalFailure::ResolutionDomainMismatch {
            process,
            information: requirement.information(),
            required: requirement.required_domain(),
            available: capability.domain(),
        });
    }

    if state_tick.0 > current_tick.0 {
        return Err(SpatiotemporalFailure::StateFromFuture {
            process,
            information: requirement.information(),
            current_tick,
            state_tick,
        });
    }
    let age = current_tick.0 - state_tick.0;
    if age > requirement.maximum_state_age().0 {
        return Err(SpatiotemporalFailure::StateTooStale {
            process,
            information: requirement.information(),
            maximum_age: requirement.maximum_state_age(),
            actual_age: age,
        });
    }

    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpatiotemporalSufficiencyReport {
    authority: SpatiotemporalPolicyAuthorityStamp,
    process_keys: BTreeSet<ProcessKey>,
    representation: RepresentationKey,
    current_tick: CanonicalTick,
    state: SpatiotemporalStateContext,
    failures: Vec<SpatiotemporalFailure>,
}

impl SpatiotemporalSufficiencyReport {
    pub const fn authority(&self) -> &SpatiotemporalPolicyAuthorityStamp {
        &self.authority
    }

    pub fn process_keys(&self) -> &BTreeSet<ProcessKey> {
        &self.process_keys
    }

    pub const fn representation(&self) -> RepresentationKey {
        self.representation
    }

    pub const fn current_tick(&self) -> CanonicalTick {
        self.current_tick
    }

    pub const fn state(&self) -> &SpatiotemporalStateContext {
        &self.state
    }

    pub fn failures(&self) -> &[SpatiotemporalFailure] {
        &self.failures
    }

    pub fn is_sufficient(&self) -> bool {
        self.failures.is_empty()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SpatiotemporalFailure {
    MissingProcessRequirement {
        process: ProcessKey,
        information: EcologicalInformation,
    },
    MissingRepresentationCapability {
        process: ProcessKey,
        representation: RepresentationKey,
        information: EcologicalInformation,
    },
    MissingStateTick {
        process: ProcessKey,
        representation: RepresentationKey,
        information: EcologicalInformation,
    },
    SpatialResolutionTooCoarse {
        process: ProcessKey,
        information: EcologicalInformation,
        required: SpatialResolutionUnits,
        available: SpatialResolutionUnits,
    },
    UpdateIntervalTooSlow {
        process: ProcessKey,
        information: EcologicalInformation,
        required: MaximumUpdateIntervalTicks,
        available: MaximumUpdateIntervalTicks,
    },
    IntegrationSemanticsMismatch {
        process: ProcessKey,
        information: EcologicalInformation,
        required: IntegrationSemantics,
        available: IntegrationSemantics,
        required_window: Option<TemporalAggregationWindowTicks>,
        available_window: Option<TemporalAggregationWindowTicks>,
    },
    ResolutionDomainMismatch {
        process: ProcessKey,
        information: EcologicalInformation,
        required: Option<ResolutionDomainKey>,
        available: Option<ResolutionDomainKey>,
    },
    StateFromFuture {
        process: ProcessKey,
        information: EcologicalInformation,
        current_tick: CanonicalTick,
        state_tick: CanonicalTick,
    },
    StateTooStale {
        process: ProcessKey,
        information: EcologicalInformation,
        maximum_age: MaximumStateAgeTicks,
        actual_age: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpatiotemporalPolicyError {
    ZeroSpatialResolution,
    ZeroUpdateInterval,
    ZeroAggregationWindow,
    InstantaneousHasAggregationWindow,
    MissingAggregationWindow {
        semantics: IntegrationSemantics,
    },
    DuplicateStateTick {
        information: EcologicalInformation,
    },
    ConflictingProcessRequirement {
        process: ProcessKey,
        information: EcologicalInformation,
    },
    ConflictingRepresentationCapability {
        representation: RepresentationKey,
        information: EcologicalInformation,
    },
    ProcessDoesNotRequireInformation {
        process: ProcessKey,
        information: EcologicalInformation,
    },
    RepresentationDoesNotCarryInformation {
        representation: RepresentationKey,
        information: EcologicalInformation,
    },
    PolicyIdentity(InformationPolicyIdentityError),
    PolicyAuthorityMismatch,
    InformationRegistry(InformationRegistryError),
    StateRepresentationMismatch {
        expected: RepresentationKey,
        actual: RepresentationKey,
    },
}

impl fmt::Display for SpatiotemporalPolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroSpatialResolution => write!(formatter, "spatial resolution must be nonzero"),
            Self::ZeroUpdateInterval => write!(formatter, "update interval must be nonzero"),
            Self::ZeroAggregationWindow => write!(formatter, "aggregation window must be nonzero"),
            Self::InstantaneousHasAggregationWindow => write!(
                formatter,
                "instantaneous integration cannot carry an aggregation window"
            ),
            Self::MissingAggregationWindow { semantics } => write!(
                formatter,
                "integration semantics {semantics:?} requires an explicit aggregation window"
            ),
            Self::DuplicateStateTick { information } => {
                write!(formatter, "duplicate state-tick record for {information:?}")
            }
            Self::ConflictingProcessRequirement {
                process,
                information,
            } => write!(
                formatter,
                "conflicting spatiotemporal requirement for process {process:?} / {information:?}"
            ),
            Self::ConflictingRepresentationCapability {
                representation,
                information,
            } => write!(
                formatter,
                "conflicting spatiotemporal capability for representation {representation:?} / {information:?}"
            ),
            Self::ProcessDoesNotRequireInformation {
                process,
                information,
            } => write!(
                formatter,
                "process {process:?} does not declare information requirement {information:?}"
            ),
            Self::RepresentationDoesNotCarryInformation {
                representation,
                information,
            } => write!(
                formatter,
                "representation {representation:?} does not carry {information:?} in information policy"
            ),
            Self::PolicyIdentity(error) => {
                write!(formatter, "information-policy identity error: {error}")
            }
            Self::PolicyAuthorityMismatch => write!(
                formatter,
                "spatiotemporal policy was sealed under a different exact information-policy corpus"
            ),
            Self::InformationRegistry(error) => {
                write!(formatter, "information registry error: {error}")
            }
            Self::StateRepresentationMismatch { expected, actual } => write!(
                formatter,
                "state timing context belongs to {actual:?}, expected {expected:?}"
            ),
        }
    }
}

impl Error for SpatiotemporalPolicyError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::PolicyIdentity(error) => Some(error),
            Self::InformationRegistry(error) => Some(error),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::information::{
        CapabilityEvidence, EcologicalAuthorityLevel, ProcessInformationProfile,
        ProcessInformationRequirement, RepresentationCapabilities,
    };
    use crate::information_registry::{InformationPolicyRegistryBuilder, InformationPolicyRegistryKey};

    const POLICY: InformationPolicyRegistryKey = InformationPolicyRegistryKey::new(5_000, 1);
    const PROCESS: ProcessKey = ProcessKey::new(5_010, 1);
    const REPRESENTATION: RepresentationKey = RepresentationKey::new(5_020, 1);
    const ST_POLICY: SpatiotemporalPolicyRegistryKey = SpatiotemporalPolicyRegistryKey::new(5_030, 1);

    fn exact_policy() -> crate::information_registry::InformationPolicyRegistry {
        let mut builder = InformationPolicyRegistryBuilder::new(POLICY);
        builder
            .register_process(ProcessInformationProfile::new(
                PROCESS,
                EcologicalAuthorityLevel::Coarse,
                [ProcessInformationRequirement::exact(
                    EcologicalInformation::OccupancyDistribution,
                )],
            ))
            .unwrap();
        builder
            .register_representation(RepresentationCapabilities::new(
                REPRESENTATION,
                EcologicalAuthorityLevel::Coarse,
                [(
                    EcologicalInformation::OccupancyDistribution,
                    CapabilityEvidence::Exact,
                )],
            ))
            .unwrap();
        builder.seal()
    }

    fn registry(
        process_resolution: u64,
        representation_resolution: u64,
        maximum_age: u64,
        process_interval: u64,
        representation_interval: u64,
        process_integration: IntegrationSemantics,
        capability_integration: IntegrationSemantics,
        window: Option<u64>,
    ) -> SpatiotemporalPolicyRegistry {
        let policy = exact_policy();
        let exact = ManifestBoundInformationPolicyRegistry::new(&policy);
        let mut builder = SpatiotemporalPolicyRegistryBuilder::new(ST_POLICY);
        let process_window = window
            .map(|ticks| TemporalAggregationWindowTicks::new(ticks).unwrap());
        let capability_window = window
            .map(|ticks| TemporalAggregationWindowTicks::new(ticks).unwrap());
        builder
            .register_process_requirement(
                ProcessSpatiotemporalRequirement::new(
                    PROCESS,
                    EcologicalInformation::OccupancyDistribution,
                    SpatialResolutionUnits::new(process_resolution).unwrap(),
                    MaximumStateAgeTicks(maximum_age),
                    MaximumUpdateIntervalTicks::new(process_interval).unwrap(),
                    process_integration,
                    process_window,
                    None,
                )
                .unwrap(),
            )
            .unwrap();
        builder
            .register_representation_capability(
                RepresentationSpatiotemporalCapability::new(
                    REPRESENTATION,
                    EcologicalInformation::OccupancyDistribution,
                    SpatialResolutionUnits::new(representation_resolution).unwrap(),
                    MaximumUpdateIntervalTicks::new(representation_interval).unwrap(),
                    capability_integration,
                    capability_window,
                    None,
                )
                .unwrap(),
            )
            .unwrap();
        builder.seal(&exact).unwrap()
    }

    fn report(
        registry: &SpatiotemporalPolicyRegistry,
        current: u64,
        state_tick: u64,
    ) -> SpatiotemporalSufficiencyReport {
        let policy = exact_policy();
        let exact = ManifestBoundInformationPolicyRegistry::new(&policy);
        let state = SpatiotemporalStateContext::from_records(
            REPRESENTATION,
            [(
                EcologicalInformation::OccupancyDistribution,
                CanonicalTick(state_tick),
            )],
        )
        .unwrap();
        registry
            .evaluate(&exact, [PROCESS], REPRESENTATION, CanonicalTick(current), &state)
            .unwrap()
    }

    #[test]
    fn finer_one_unit_resolution_satisfies_five_unit_requirement() {
        let registry = registry(
            5,
            1,
            0,
            1,
            1,
            IntegrationSemantics::Instantaneous,
            IntegrationSemantics::Instantaneous,
            None,
        );
        assert!(report(&registry, 10, 10).is_sufficient());
    }

    #[test]
    fn coarse_five_unit_resolution_rejects_one_unit_requirement() {
        let registry = registry(
            1,
            5,
            0,
            1,
            1,
            IntegrationSemantics::Instantaneous,
            IntegrationSemantics::Instantaneous,
            None,
        );
        let report = report(&registry, 10, 10);
        assert!(report.failures().iter().any(|failure| matches!(
            failure,
            SpatiotemporalFailure::SpatialResolutionTooCoarse { .. }
        )));
    }

    #[test]
    fn stale_state_fails_but_current_state_passes() {
        let registry = registry(
            5,
            5,
            2,
            1,
            1,
            IntegrationSemantics::Instantaneous,
            IntegrationSemantics::Instantaneous,
            None,
        );
        assert!(report(&registry, 10, 10).is_sufficient());
        assert!(report(&registry, 10, 7).failures().iter().any(|failure| matches!(
            failure,
            SpatiotemporalFailure::StateTooStale { .. }
        )));
    }

    #[test]
    fn slow_update_interval_cannot_satisfy_fast_process() {
        let registry = registry(
            5,
            5,
            0,
            1,
            10,
            IntegrationSemantics::Instantaneous,
            IntegrationSemantics::Instantaneous,
            None,
        );
        assert!(report(&registry, 10, 10).failures().iter().any(|failure| matches!(
            failure,
            SpatiotemporalFailure::UpdateIntervalTooSlow { .. }
        )));
    }

    #[test]
    fn mean_window_does_not_substitute_for_peak_semantics() {
        let registry = registry(
            5,
            5,
            0,
            1,
            1,
            IntegrationSemantics::MaximumOverWindow,
            IntegrationSemantics::MeanOverWindow,
            Some(10),
        );
        assert!(report(&registry, 10, 10).failures().iter().any(|failure| matches!(
            failure,
            SpatiotemporalFailure::IntegrationSemanticsMismatch { .. }
        )));
    }

    #[test]
    fn future_state_is_rejected() {
        let registry = registry(
            5,
            5,
            2,
            1,
            1,
            IntegrationSemantics::Instantaneous,
            IntegrationSemantics::Instantaneous,
            None,
        );
        assert!(report(&registry, 10, 11).failures().iter().any(|failure| matches!(
            failure,
            SpatiotemporalFailure::StateFromFuture { .. }
        )));
    }

    #[test]
    fn duplicate_state_tick_records_fail_before_evaluation() {
        assert!(matches!(
            SpatiotemporalStateContext::from_records(
                REPRESENTATION,
                [
                    (
                        EcologicalInformation::OccupancyDistribution,
                        CanonicalTick(10),
                    ),
                    (
                        EcologicalInformation::OccupancyDistribution,
                        CanonicalTick(10),
                    ),
                ],
            ),
            Err(SpatiotemporalPolicyError::DuplicateStateTick { .. })
        ));
    }
}
