//! Frozen specimen/sequence/evidence manifest validator for the
//! history-aware foot-terrain identification benchmark.
//!
//! This crate validates experiment identity, split independence, source-clock
//! evidence and Terrain-owned settlement receipts. It does not estimate soil
//! parameters, classify raw sensor streams, compute contact physics, or mutate
//! Terrain state.

use std::collections::{BTreeMap, BTreeSet};

pub const MAX_SPECIMENS: usize = 100_000;
pub const MAX_SEQUENCES: usize = 200_000;
pub const MAX_CONTACT_EVENTS: usize = 2_000_000;
pub const MAX_OBSERVATIONS: usize = 5_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DataSplit {
    Calibration,
    Validation,
    Test,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpecimenAssignment {
    pub specimen_id: String,
    pub preparation_batch_id: String,
    /// A physical collection/location cluster. Use a stable cluster identity,
    /// not a room or generic laboratory name.
    pub independence_cluster_id: String,
    pub material_profile_id: String,
    pub split: DataSplit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SensorChannel {
    D1Interface,
    D2NearFoot,
    D3Proprioception,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SensorSource {
    pub source_id: String,
    pub channel: SensorChannel,
    pub sensor_model_digest: String,
    pub calibration_digest: String,
    pub timebase_id: String,
    pub unit_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClockMappingReceipt {
    pub timebase_id: String,
    pub mapping_digest: String,
    pub valid_from_raw_ns: u64,
    pub valid_until_raw_ns: u64,
    pub maximum_uncertainty_ns: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChannelStatus {
    Present,
    Missing { reason_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelCoverage {
    pub channel: SensorChannel,
    pub status: ChannelStatus,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SensorObservation {
    pub observation_id: String,
    pub contact_event_id: String,
    pub source_id: String,
    pub calibration_digest: String,
    pub mapping_digest: String,
    pub unit_id: String,
    pub raw_timestamp_ns: u64,
    pub corrected_timestamp_ns: u64,
    pub arrival_timestamp_ns: u64,
    pub timestamp_uncertainty_ns: u64,
    /// Canonical fixed-point payload. Raw physical value and unit are retained;
    /// this validator does not interpret or rescale the value.
    pub value_fixed: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GroundDisposition {
    NoEffect,
    Settled {
        settlement_receipt_id: String,
        resulting_revision: u64,
    },
    Unknown {
        reason_id: String,
    },
    OutOfDomain {
        reason_id: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContactEvent {
    pub event_id: String,
    pub step_index: u64,
    pub contact_timestamp_ns: u64,
    pub contact_evidence_receipt_id: String,
    pub action_execution_digest: String,
    pub terrain_subject_id: String,
    pub ground_revision_before: u64,
    pub ground_disposition: GroundDisposition,
    pub channel_coverage: Vec<ChannelCoverage>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContactSequence {
    pub sequence_id: String,
    pub specimen_id: String,
    pub terrain_subject_id: String,
    pub initial_ground_revision: u64,
    pub contact_profile_digest: String,
    pub ground_history_profile_digest: String,
    pub events: Vec<ContactEvent>,
    pub observations: Vec<SensorObservation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExperimentManifest {
    pub manifest_id: String,
    pub manifest_digest: String,
    pub robot_configuration_digest: String,
    pub physics_engine_digest: String,
    pub solver_configuration_digest: String,
    pub model_family_digest: String,
    pub required_channels: Vec<SensorChannel>,
    pub specimen_assignments: Vec<SpecimenAssignment>,
    pub sensor_sources: Vec<SensorSource>,
    pub clock_mappings: Vec<ClockMappingReceipt>,
    pub sequences: Vec<ContactSequence>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SplitCounts {
    pub specimens: usize,
    pub sequences: usize,
    pub contacts: usize,
    pub observations: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestValidationReport {
    pub manifest_id: String,
    pub calibration: SplitCounts,
    pub validation: SplitCounts,
    pub test: SplitCounts,
    pub explicit_missing_channel_records: usize,
    pub no_effect_contacts: usize,
    pub settled_contacts: usize,
    pub unknown_or_out_of_domain_contacts: usize,
    pub settlement_receipt_ids: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManifestError {
    EmptyIdentifier,
    EmptyRequiredChannels,
    DuplicateRequiredChannel,
    NoSpecimens,
    NoSequences,
    TooManySpecimens,
    TooManySequences,
    TooManyContactEvents,
    TooManyObservations,
    DuplicateSpecimenId,
    DuplicatePreparationBatch,
    DuplicateIndependenceCluster,
    ObservationChannelNotRequired,
    UnexpectedChannelCoverage,
    SplitLeakage,
    UnknownSpecimen,
    DuplicateSequenceId,
    EmptyContactSequence,
    EmptySensorSources,
    DuplicateSensorSource,
    UnknownSensorSource,
    DuplicateClockMapping,
    MissingClockMapping,
    InvalidClockInterval,
    InvalidClockUncertainty,
    DuplicateObservationId,
    ObservationUsesUnknownEvent,
    ObservationChannelMissing,
    ChannelMarkedPresentWithoutObservation,
    ChannelMarkedMissingWithObservation,
    DuplicateChannelCoverage,
    MissingChannelCoverage,
    MissingReason,
    ObservationIdentityMismatch,
    ObservationUnitMismatch,
    ObservationOutsideClockMapping,
    ObservationUncertaintyExceeded,
    ArrivalPredatesCorrectedObservation,
    SourceTimestampNotIncreasing,
    CorrectedTimestampNotIncreasing,
    DuplicateContactEventId,
    ContactSequenceIdentityMismatch,
    ContactEventOrderNotIncreasing,
    DuplicateSettlementReceipt,
    MissingSettlementReceipt,
    NoEffectChangedGroundRevision,
    SettlementDidNotAdvanceRevision,
    UnknownEffectChangedGroundRevision,
    UnknownEffectMissingReason,
    GroundRevisionChainMismatch,
    ContactEvidenceReceiptMissing,
    ActionExecutionDigestMissing,
    ContactProfileDigestMissing,
    GroundHistoryProfileDigestMissing,
    ArithmeticOverflow,
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn checked_increment(value: usize) -> Result<usize, ManifestError> {
    value
        .checked_add(1)
        .ok_or(ManifestError::ArithmeticOverflow)
}

fn validate_manifest_identity(manifest: &ExperimentManifest) -> Result<(), ManifestError> {
    let identifiers = [
        manifest.manifest_id.as_str(),
        manifest.manifest_digest.as_str(),
        manifest.robot_configuration_digest.as_str(),
        manifest.physics_engine_digest.as_str(),
        manifest.solver_configuration_digest.as_str(),
        manifest.model_family_digest.as_str(),
    ];
    if identifiers.iter().any(|value| !nonempty(value)) {
        return Err(ManifestError::EmptyIdentifier);
    }
    if manifest.required_channels.is_empty() {
        return Err(ManifestError::EmptyRequiredChannels);
    }
    let mut required = BTreeSet::new();
    for channel in &manifest.required_channels {
        if !required.insert(*channel) {
            return Err(ManifestError::DuplicateRequiredChannel);
        }
    }
    Ok(())
}

fn validate_specimens(
    assignments: &[SpecimenAssignment],
) -> Result<BTreeMap<&str, DataSplit>, ManifestError> {
    if assignments.is_empty() {
        return Err(ManifestError::NoSpecimens);
    }
    if assignments.len() > MAX_SPECIMENS {
        return Err(ManifestError::TooManySpecimens);
    }
    let mut specimens = BTreeMap::new();
    let mut batches = BTreeMap::<&str, DataSplit>::new();
    let mut clusters = BTreeMap::<&str, DataSplit>::new();
    for assignment in assignments {
        if !nonempty(&assignment.specimen_id)
            || !nonempty(&assignment.preparation_batch_id)
            || !nonempty(&assignment.independence_cluster_id)
            || !nonempty(&assignment.material_profile_id)
        {
            return Err(ManifestError::EmptyIdentifier);
        }
        if specimens
            .insert(assignment.specimen_id.as_str(), assignment.split)
            .is_some()
        {
            return Err(ManifestError::DuplicateSpecimenId);
        }
        if let Some(previous) = batches.get(assignment.preparation_batch_id.as_str()) {
            if *previous != assignment.split {
                return Err(ManifestError::SplitLeakage);
            }
        } else {
            batches.insert(assignment.preparation_batch_id.as_str(), assignment.split);
        }
        if let Some(previous) = clusters.get(assignment.independence_cluster_id.as_str()) {
            if *previous != assignment.split {
                return Err(ManifestError::SplitLeakage);
            }
        } else {
            clusters.insert(
                assignment.independence_cluster_id.as_str(),
                assignment.split,
            );
        }
    }
    Ok(specimens)
}

fn validate_sources<'a, 'b>(
    sources: &'a [SensorSource],
    mappings: &'b [ClockMappingReceipt],
) -> Result<
    (
        BTreeMap<&'a str, &'a SensorSource>,
        BTreeMap<&'b str, &'b ClockMappingReceipt>,
    ),
    ManifestError,
> {
    if sources.is_empty() {
        return Err(ManifestError::EmptySensorSources);
    }
    let mut source_index = BTreeMap::new();
    for source in sources {
        if !nonempty(&source.source_id)
            || !nonempty(&source.sensor_model_digest)
            || !nonempty(&source.calibration_digest)
            || !nonempty(&source.timebase_id)
            || !nonempty(&source.unit_id)
        {
            return Err(ManifestError::EmptyIdentifier);
        }
        if source_index
            .insert(source.source_id.as_str(), source)
            .is_some()
        {
            return Err(ManifestError::DuplicateSensorSource);
        }
    }

    let mut mapping_index = BTreeMap::new();
    for mapping in mappings {
        if !nonempty(&mapping.timebase_id) || !nonempty(&mapping.mapping_digest) {
            return Err(ManifestError::EmptyIdentifier);
        }
        if mapping.valid_from_raw_ns > mapping.valid_until_raw_ns {
            return Err(ManifestError::InvalidClockInterval);
        }
        if mapping.maximum_uncertainty_ns == 0 {
            return Err(ManifestError::InvalidClockUncertainty);
        }
        if mapping_index
            .insert(mapping.timebase_id.as_str(), mapping)
            .is_some()
        {
            return Err(ManifestError::DuplicateClockMapping);
        }
    }

    for source in sources {
        if !mapping_index.contains_key(source.timebase_id.as_str()) {
            return Err(ManifestError::MissingClockMapping);
        }
    }
    Ok((source_index, mapping_index))
}

fn validate_coverage(
    event: &ContactEvent,
    required_channels: &[SensorChannel],
    observations_by_channel: &BTreeMap<SensorChannel, usize>,
) -> Result<usize, ManifestError> {
    let mut coverage = BTreeMap::new();
    let mut missing_count = 0_usize;
    for item in &event.channel_coverage {
        if coverage.insert(item.channel, &item.status).is_some() {
            return Err(ManifestError::DuplicateChannelCoverage);
        }
    }
    for channel in required_channels {
        let status = coverage
            .get(channel)
            .ok_or(ManifestError::MissingChannelCoverage)?;
        let observation_count = observations_by_channel.get(channel).copied().unwrap_or(0);
        match status {
            ChannelStatus::Present => {
                if observation_count == 0 {
                    return Err(ManifestError::ChannelMarkedPresentWithoutObservation);
                }
            }
            ChannelStatus::Missing { reason_id } => {
                if !nonempty(reason_id) {
                    return Err(ManifestError::MissingReason);
                }
                if observation_count > 0 {
                    return Err(ManifestError::ChannelMarkedMissingWithObservation);
                }
                missing_count = checked_increment(missing_count)?;
            }
        }
    }
    if coverage.len() != required_channels.len() {
        return Err(ManifestError::UnexpectedChannelCoverage);
    }
    Ok(missing_count)
}

fn validate_ground_chain(
    sequence: &ContactSequence,
    settlement_receipts: &mut BTreeSet<String>,
) -> Result<(usize, usize, usize), ManifestError> {
    if sequence.events.is_empty() {
        return Err(ManifestError::EmptyContactSequence);
    }
    if !nonempty(&sequence.sequence_id)
        || !nonempty(&sequence.specimen_id)
        || !nonempty(&sequence.terrain_subject_id)
        || !nonempty(&sequence.contact_profile_digest)
    {
        return Err(ManifestError::EmptyIdentifier);
    }
    if !nonempty(&sequence.ground_history_profile_digest) {
        return Err(ManifestError::GroundHistoryProfileDigestMissing);
    }

    let mut current_revision = sequence.initial_ground_revision;
    let mut previous_event_time = None;
    let mut previous_step_index = None;
    let mut no_effect_count = 0;
    let mut settled_count = 0;
    let mut unknown_count = 0;

    for event in &sequence.events {
        if !nonempty(&event.event_id) || !nonempty(&event.terrain_subject_id) {
            return Err(ManifestError::EmptyIdentifier);
        }
        if !nonempty(&event.contact_evidence_receipt_id) {
            return Err(ManifestError::ContactEvidenceReceiptMissing);
        }
        if !nonempty(&event.action_execution_digest) {
            return Err(ManifestError::ActionExecutionDigestMissing);
        }
        if event.terrain_subject_id != sequence.terrain_subject_id {
            return Err(ManifestError::ContactSequenceIdentityMismatch);
        }
        if let Some(previous) = previous_event_time {
            if event.contact_timestamp_ns <= previous {
                return Err(ManifestError::ContactEventOrderNotIncreasing);
            }
        }
        if let Some(previous) = previous_step_index {
            if event.step_index <= previous {
                return Err(ManifestError::ContactEventOrderNotIncreasing);
            }
        }
        previous_event_time = Some(event.contact_timestamp_ns);
        previous_step_index = Some(event.step_index);

        if event.ground_revision_before != current_revision {
            return Err(ManifestError::GroundRevisionChainMismatch);
        }
        match &event.ground_disposition {
            GroundDisposition::NoEffect => {
                no_effect_count = checked_increment(no_effect_count)?;
            }
            GroundDisposition::Settled {
                settlement_receipt_id,
                resulting_revision,
            } => {
                if !nonempty(settlement_receipt_id) {
                    return Err(ManifestError::MissingSettlementReceipt);
                }
                if *resulting_revision <= current_revision {
                    return Err(ManifestError::SettlementDidNotAdvanceRevision);
                }
                if !settlement_receipts.insert(settlement_receipt_id.clone()) {
                    return Err(ManifestError::DuplicateSettlementReceipt);
                }
                current_revision = *resulting_revision;
                settled_count = checked_increment(settled_count)?;
            }
            GroundDisposition::Unknown { reason_id }
            | GroundDisposition::OutOfDomain { reason_id } => {
                if !nonempty(reason_id) {
                    return Err(ManifestError::UnknownEffectMissingReason);
                }
                unknown_count = checked_increment(unknown_count)?;
            }
        }
    }
    Ok((no_effect_count, settled_count, unknown_count))
}

fn validate_observations(
    sequence: &ContactSequence,
    sources: &BTreeMap<&str, &SensorSource>,
    mappings: &BTreeMap<&str, &ClockMappingReceipt>,
    required_channels: &[SensorChannel],
    seen_observation_ids: &mut BTreeSet<String>,
) -> Result<(usize, usize), ManifestError> {
    let event_ids: BTreeSet<&str> = sequence
        .events
        .iter()
        .map(|event| event.event_id.as_str())
        .collect();
    let mut per_event_channel_counts = BTreeMap::<(&str, SensorChannel), usize>::new();
    let mut last_source_raw = BTreeMap::<&str, u64>::new();
    let mut last_source_corrected = BTreeMap::<&str, u64>::new();

    for observation in &sequence.observations {
        if !nonempty(&observation.observation_id)
            || !nonempty(&observation.contact_event_id)
            || !nonempty(&observation.source_id)
            || !nonempty(&observation.calibration_digest)
            || !nonempty(&observation.mapping_digest)
            || !nonempty(&observation.unit_id)
        {
            return Err(ManifestError::EmptyIdentifier);
        }
        if !seen_observation_ids.insert(observation.observation_id.clone()) {
            return Err(ManifestError::DuplicateObservationId);
        }
        if !event_ids.contains(observation.contact_event_id.as_str()) {
            return Err(ManifestError::ObservationUsesUnknownEvent);
        }
        let source = sources
            .get(observation.source_id.as_str())
            .ok_or(ManifestError::UnknownSensorSource)?;
        if !required_channels.contains(&source.channel) {
            return Err(ManifestError::ObservationChannelNotRequired);
        }
        let mapping = mappings
            .get(source.timebase_id.as_str())
            .ok_or(ManifestError::MissingClockMapping)?;

        if observation.calibration_digest != source.calibration_digest
            || observation.mapping_digest != mapping.mapping_digest
        {
            return Err(ManifestError::ObservationIdentityMismatch);
        }
        if observation.unit_id != source.unit_id {
            return Err(ManifestError::ObservationUnitMismatch);
        }
        if observation.raw_timestamp_ns < mapping.valid_from_raw_ns
            || observation.raw_timestamp_ns > mapping.valid_until_raw_ns
        {
            return Err(ManifestError::ObservationOutsideClockMapping);
        }
        let allowed_uncertainty = mapping.maximum_uncertainty_ns;
        if observation.timestamp_uncertainty_ns == 0
            || observation.timestamp_uncertainty_ns > allowed_uncertainty
        {
            return Err(ManifestError::ObservationUncertaintyExceeded);
        }
        if observation.arrival_timestamp_ns < observation.corrected_timestamp_ns
            && observation
                .corrected_timestamp_ns
                .saturating_sub(observation.arrival_timestamp_ns)
                > observation.timestamp_uncertainty_ns
        {
            return Err(ManifestError::ArrivalPredatesCorrectedObservation);
        }

        if let Some(previous) =
            last_source_raw.insert(observation.source_id.as_str(), observation.raw_timestamp_ns)
        {
            if observation.raw_timestamp_ns <= previous {
                return Err(ManifestError::SourceTimestampNotIncreasing);
            }
        }
        if let Some(previous) = last_source_corrected.insert(
            observation.source_id.as_str(),
            observation.corrected_timestamp_ns,
        ) {
            if observation.corrected_timestamp_ns <= previous {
                return Err(ManifestError::CorrectedTimestampNotIncreasing);
            }
        }

        let key = (observation.contact_event_id.as_str(), source.channel);
        let count = per_event_channel_counts.get(&key).copied().unwrap_or(0);
        per_event_channel_counts.insert(key, checked_increment(count)?);
    }

    let mut missing_count = 0;
    for event in &sequence.events {
        let mut channel_counts = BTreeMap::new();
        for channel in required_channels {
            let count = per_event_channel_counts
                .get(&(event.event_id.as_str(), *channel))
                .copied()
                .unwrap_or(0);
            channel_counts.insert(*channel, count);
        }
        missing_count = missing_count
            .checked_add(validate_coverage(
                event,
                required_channels,
                &channel_counts,
            )?)
            .ok_or(ManifestError::ArithmeticOverflow)?;
    }
    Ok((sequence.observations.len(), missing_count))
}

/// Validate whole-specimen split isolation, source/calibration/clock bindings,
/// per-source time ordering, event order and Terrain-owned ground revision
/// continuity. No sensor inference or state mutation is performed.
pub fn validate_experiment_manifest(
    manifest: &ExperimentManifest,
) -> Result<ManifestValidationReport, ManifestError> {
    validate_manifest_identity(manifest)?;
    if manifest.sequences.is_empty() {
        return Err(ManifestError::NoSequences);
    }
    if manifest.sequences.len() > MAX_SEQUENCES {
        return Err(ManifestError::TooManySequences);
    }

    let specimen_splits = validate_specimens(&manifest.specimen_assignments)?;
    let (sources, mappings) = validate_sources(&manifest.sensor_sources, &manifest.clock_mappings)?;

    let total_contacts = manifest
        .sequences
        .iter()
        .try_fold(0_usize, |total, sequence| {
            total
                .checked_add(sequence.events.len())
                .ok_or(ManifestError::ArithmeticOverflow)
        })?;
    if total_contacts > MAX_CONTACT_EVENTS {
        return Err(ManifestError::TooManyContactEvents);
    }
    let total_observations = manifest
        .sequences
        .iter()
        .try_fold(0_usize, |total, sequence| {
            total
                .checked_add(sequence.observations.len())
                .ok_or(ManifestError::ArithmeticOverflow)
        })?;
    if total_observations > MAX_OBSERVATIONS {
        return Err(ManifestError::TooManyObservations);
    }

    let mut sequence_ids = BTreeSet::new();
    let mut event_ids = BTreeSet::new();
    let mut observation_ids = BTreeSet::new();
    let mut settlement_receipts = BTreeSet::new();
    let mut split_counts = BTreeMap::from([
        (
            DataSplit::Calibration,
            SplitCounts {
                specimens: 0,
                sequences: 0,
                contacts: 0,
                observations: 0,
            },
        ),
        (
            DataSplit::Validation,
            SplitCounts {
                specimens: 0,
                sequences: 0,
                contacts: 0,
                observations: 0,
            },
        ),
        (
            DataSplit::Test,
            SplitCounts {
                specimens: 0,
                sequences: 0,
                contacts: 0,
                observations: 0,
            },
        ),
    ]);
    let mut missing_channels = 0_usize;
    let mut no_effect_contacts = 0_usize;
    let mut settled_contacts = 0_usize;
    let mut unknown_contacts = 0_usize;

    let mut sequence_count_by_specimen = BTreeMap::<&str, usize>::new();
    for sequence in &manifest.sequences {
        if !nonempty(&sequence.sequence_id) {
            return Err(ManifestError::EmptyIdentifier);
        }
        if !sequence_ids.insert(sequence.sequence_id.as_str()) {
            return Err(ManifestError::DuplicateSequenceId);
        }
        let split = specimen_splits
            .get(sequence.specimen_id.as_str())
            .copied()
            .ok_or(ManifestError::UnknownSpecimen)?;
        let count = sequence_count_by_specimen
            .get(sequence.specimen_id.as_str())
            .copied()
            .unwrap_or(0);
        sequence_count_by_specimen.insert(sequence.specimen_id.as_str(), checked_increment(count)?);

        for event in &sequence.events {
            if !event_ids.insert(event.event_id.as_str()) {
                return Err(ManifestError::DuplicateContactEventId);
            }
        }

        let (no_effect, settled, unknown) =
            validate_ground_chain(sequence, &mut settlement_receipts)?;
        let (observations, missing) = validate_observations(
            sequence,
            &sources,
            &mappings,
            &manifest.required_channels,
            &mut observation_ids,
        )?;
        missing_channels = missing_channels
            .checked_add(missing)
            .ok_or(ManifestError::ArithmeticOverflow)?;
        no_effect_contacts = no_effect_contacts
            .checked_add(no_effect)
            .ok_or(ManifestError::ArithmeticOverflow)?;
        settled_contacts = settled_contacts
            .checked_add(settled)
            .ok_or(ManifestError::ArithmeticOverflow)?;
        unknown_contacts = unknown_contacts
            .checked_add(unknown)
            .ok_or(ManifestError::ArithmeticOverflow)?;

        let counts = split_counts
            .get_mut(&split)
            .ok_or(ManifestError::ArithmeticOverflow)?;
        counts.sequences = checked_increment(counts.sequences)?;
        counts.contacts = counts
            .contacts
            .checked_add(sequence.events.len())
            .ok_or(ManifestError::ArithmeticOverflow)?;
        counts.observations = counts
            .observations
            .checked_add(observations)
            .ok_or(ManifestError::ArithmeticOverflow)?;
    }
    if sequence_count_by_specimen.len() != specimen_splits.len() {
        return Err(ManifestError::NoSequences);
    }
    for split in [
        DataSplit::Calibration,
        DataSplit::Validation,
        DataSplit::Test,
    ] {
        let distinct_specimens = specimen_splits
            .iter()
            .filter(|(_, assigned_split)| **assigned_split == split)
            .count();
        let counts = split_counts
            .get_mut(&split)
            .ok_or(ManifestError::ArithmeticOverflow)?;
        counts.specimens = distinct_specimens;
    }

    Ok(ManifestValidationReport {
        manifest_id: manifest.manifest_id.clone(),
        calibration: split_counts
            .get(&DataSplit::Calibration)
            .cloned()
            .ok_or(ManifestError::ArithmeticOverflow)?,
        validation: split_counts
            .get(&DataSplit::Validation)
            .cloned()
            .ok_or(ManifestError::ArithmeticOverflow)?,
        test: split_counts
            .get(&DataSplit::Test)
            .cloned()
            .ok_or(ManifestError::ArithmeticOverflow)?,
        explicit_missing_channel_records: missing_channels,
        no_effect_contacts,
        settled_contacts,
        unknown_or_out_of_domain_contacts: unknown_contacts,
        settlement_receipt_ids: settlement_receipts.into_iter().collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn specimen(id: &str, batch: &str, cluster: &str, split: DataSplit) -> SpecimenAssignment {
        SpecimenAssignment {
            specimen_id: id.to_owned(),
            preparation_batch_id: batch.to_owned(),
            independence_cluster_id: cluster.to_owned(),
            material_profile_id: "granular-profile-v1".to_owned(),
            split,
        }
    }

    fn event(
        id: &str,
        step: u64,
        time: u64,
        revision: u64,
        disposition: GroundDisposition,
    ) -> ContactEvent {
        ContactEvent {
            event_id: id.to_owned(),
            step_index: step,
            contact_timestamp_ns: time,
            contact_evidence_receipt_id: format!("contact-evidence-{id}"),
            action_execution_digest: format!("action-{id}"),
            terrain_subject_id: "terrain-patch-1".to_owned(),
            ground_revision_before: revision,
            ground_disposition: disposition,
            channel_coverage: vec![
                ChannelCoverage {
                    channel: SensorChannel::D1Interface,
                    status: ChannelStatus::Present,
                },
                ChannelCoverage {
                    channel: SensorChannel::D2NearFoot,
                    status: ChannelStatus::Present,
                },
                ChannelCoverage {
                    channel: SensorChannel::D3Proprioception,
                    status: ChannelStatus::Present,
                },
            ],
        }
    }

    fn observation(
        id: &str,
        event_id: &str,
        source_id: &str,
        calibration: &str,
        mapping: &str,
        unit: &str,
        raw: u64,
        corrected: u64,
        arrival: u64,
    ) -> SensorObservation {
        SensorObservation {
            observation_id: id.to_owned(),
            contact_event_id: event_id.to_owned(),
            source_id: source_id.to_owned(),
            calibration_digest: calibration.to_owned(),
            mapping_digest: mapping.to_owned(),
            unit_id: unit.to_owned(),
            raw_timestamp_ns: raw,
            corrected_timestamp_ns: corrected,
            arrival_timestamp_ns: arrival,
            timestamp_uncertainty_ns: 10,
            value_fixed: 42,
        }
    }

    fn sequence(specimen_id: &str) -> ContactSequence {
        let first_event_id = format!("event-{specimen_id}-1");
        let second_event_id = format!("event-{specimen_id}-2");
        let events = vec![
            event(&first_event_id, 1, 100, 7, GroundDisposition::NoEffect),
            event(
                &second_event_id,
                2,
                200,
                7,
                GroundDisposition::Settled {
                    settlement_receipt_id: format!("terrain-settlement-{specimen_id}"),
                    resulting_revision: 8,
                },
            ),
        ];
        let observations = vec![
            observation(
                &format!("d1-{specimen_id}-e1"),
                &first_event_id,
                "d1",
                "cal-d1",
                "map-d1",
                "kpa",
                90,
                100,
                120,
            ),
            observation(
                &format!("d2-{specimen_id}-e1"),
                &first_event_id,
                "d2",
                "cal-d2",
                "map-d2",
                "newton",
                92,
                101,
                130,
            ),
            observation(
                &format!("d3-{specimen_id}-e1"),
                &first_event_id,
                "d3",
                "cal-d3",
                "map-d3",
                "rad",
                93,
                102,
                125,
            ),
            observation(
                &format!("d1-{specimen_id}-e2"),
                &second_event_id,
                "d1",
                "cal-d1",
                "map-d1",
                "kpa",
                190,
                200,
                220,
            ),
            observation(
                &format!("d2-{specimen_id}-e2"),
                &second_event_id,
                "d2",
                "cal-d2",
                "map-d2",
                "newton",
                192,
                201,
                230,
            ),
            observation(
                &format!("d3-{specimen_id}-e2"),
                &second_event_id,
                "d3",
                "cal-d3",
                "map-d3",
                "rad",
                193,
                202,
                225,
            ),
        ];
        ContactSequence {
            sequence_id: format!("sequence-{specimen_id}"),
            specimen_id: specimen_id.to_owned(),
            terrain_subject_id: "terrain-patch-1".to_owned(),
            initial_ground_revision: 7,
            contact_profile_digest: "foot-contact-profile-v1".to_owned(),
            ground_history_profile_digest: "granular-history-profile-v1".to_owned(),
            events,
            observations,
        }
    }

    fn mapping(id: &str) -> ClockMappingReceipt {
        ClockMappingReceipt {
            timebase_id: format!("clock-{id}"),
            mapping_digest: format!("map-{id}"),
            valid_from_raw_ns: 0,
            valid_until_raw_ns: 1_000,
            maximum_uncertainty_ns: 100,
        }
    }

    fn source(id: &str, channel: SensorChannel, unit: &str) -> SensorSource {
        SensorSource {
            source_id: id.to_owned(),
            channel,
            sensor_model_digest: format!("sensor-model-{id}"),
            calibration_digest: format!("cal-{id}"),
            timebase_id: format!("clock-{id}"),
            unit_id: unit.to_owned(),
        }
    }

    fn valid_manifest() -> ExperimentManifest {
        ExperimentManifest {
            manifest_id: "terramech-experiment-v1".to_owned(),
            manifest_digest: "manifest-digest-v1".to_owned(),
            robot_configuration_digest: "go2-config-v1".to_owned(),
            physics_engine_digest: "engine-v1".to_owned(),
            solver_configuration_digest: "solver-v1".to_owned(),
            model_family_digest: "model-family-v1".to_owned(),
            required_channels: vec![
                SensorChannel::D1Interface,
                SensorChannel::D2NearFoot,
                SensorChannel::D3Proprioception,
            ],
            specimen_assignments: vec![
                specimen(
                    "specimen-cal",
                    "batch-cal",
                    "cluster-cal",
                    DataSplit::Calibration,
                ),
                specimen(
                    "specimen-val",
                    "batch-val",
                    "cluster-val",
                    DataSplit::Validation,
                ),
                specimen(
                    "specimen-test",
                    "batch-test",
                    "cluster-test",
                    DataSplit::Test,
                ),
            ],
            sensor_sources: vec![
                source("d1", SensorChannel::D1Interface, "kpa"),
                source("d2", SensorChannel::D2NearFoot, "newton"),
                source("d3", SensorChannel::D3Proprioception, "rad"),
            ],
            clock_mappings: vec![mapping("d1"), mapping("d2"), mapping("d3")],
            sequences: vec![
                sequence("specimen-cal"),
                sequence("specimen-val"),
                sequence("specimen-test"),
            ],
        }
    }

    #[test]
    fn valid_manifest_reports_specimen_disjoint_splits_and_ground_effects() {
        let report = validate_experiment_manifest(&valid_manifest()).expect("valid manifest");
        assert_eq!(report.calibration.specimens, 1);
        assert_eq!(report.validation.specimens, 1);
        assert_eq!(report.test.specimens, 1);
        assert_eq!(report.calibration.contacts, 2);
        assert_eq!(report.calibration.observations, 6);
        assert_eq!(report.no_effect_contacts, 3);
        assert_eq!(report.settled_contacts, 3);
        assert_eq!(
            report.settlement_receipt_ids,
            vec![
                "terrain-settlement-specimen-cal".to_owned(),
                "terrain-settlement-specimen-test".to_owned(),
                "terrain-settlement-specimen-val".to_owned()
            ]
        );
    }

    #[test]
    fn preparation_batch_cannot_cross_calibration_and_test_splits() {
        let mut manifest = valid_manifest();
        manifest.specimen_assignments[2].preparation_batch_id = "batch-cal".to_owned();
        assert_eq!(
            validate_experiment_manifest(&manifest),
            Err(ManifestError::SplitLeakage)
        );
    }

    #[test]
    fn multiple_specimens_in_one_batch_are_allowed_within_a_split() {
        let mut manifest = valid_manifest();
        manifest.specimen_assignments.push(specimen(
            "specimen-cal-2",
            "batch-cal",
            "cluster-cal",
            DataSplit::Calibration,
        ));
        manifest.sequences.push(sequence("specimen-cal-2"));
        assert!(validate_experiment_manifest(&manifest).is_ok());
    }

    #[test]
    fn independence_cluster_cannot_cross_splits() {
        let mut manifest = valid_manifest();
        manifest.specimen_assignments[2].independence_cluster_id = "cluster-val".to_owned();
        assert_eq!(
            validate_experiment_manifest(&manifest),
            Err(ManifestError::SplitLeakage)
        );
    }

    #[test]
    fn same_specimen_cannot_be_assigned_to_two_splits() {
        let mut manifest = valid_manifest();
        manifest.specimen_assignments.push(specimen(
            "specimen-cal",
            "batch-extra",
            "cluster-extra",
            DataSplit::Test,
        ));
        assert_eq!(
            validate_experiment_manifest(&manifest),
            Err(ManifestError::DuplicateSpecimenId)
        );
    }

    #[test]
    fn contact_sequence_must_be_strictly_ordered() {
        let mut manifest = valid_manifest();
        manifest.sequences[0].events.swap(0, 1);
        assert_eq!(
            validate_experiment_manifest(&manifest),
            Err(ManifestError::ContactEventOrderNotIncreasing)
        );
    }

    #[test]
    fn terrain_revision_cannot_advance_without_owner_settlement_receipt() {
        let mut manifest = valid_manifest();
        manifest.sequences[0].events[0].ground_disposition = GroundDisposition::NoEffect;
        manifest.sequences[0].events[1].ground_revision_before = 9;
        assert_eq!(
            validate_experiment_manifest(&manifest),
            Err(ManifestError::GroundRevisionChainMismatch)
        );
    }

    #[test]
    fn settlement_must_advance_revision_and_have_unique_receipt() {
        let mut manifest = valid_manifest();
        manifest.sequences[0].events[1].ground_disposition = GroundDisposition::Settled {
            settlement_receipt_id: String::new(),
            resulting_revision: 8,
        };
        assert_eq!(
            validate_experiment_manifest(&manifest),
            Err(ManifestError::MissingSettlementReceipt)
        );

        let mut duplicate = valid_manifest();
        duplicate.sequences[1].events[1].ground_disposition = GroundDisposition::Settled {
            settlement_receipt_id: "terrain-settlement-specimen-cal".to_owned(),
            resulting_revision: 8,
        };
        assert_eq!(
            validate_experiment_manifest(&duplicate),
            Err(ManifestError::DuplicateSettlementReceipt)
        );
    }

    #[test]
    fn unknown_and_out_of_domain_effects_cannot_mutate_revision() {
        let mut manifest = valid_manifest();
        manifest.sequences[0].events[0].ground_disposition = GroundDisposition::Unknown {
            reason_id: "insufficient-ground-evidence".to_owned(),
        };
        manifest.sequences[0].events[1].ground_revision_before = 8;
        assert_eq!(
            validate_experiment_manifest(&manifest),
            Err(ManifestError::GroundRevisionChainMismatch)
        );
    }

    #[test]
    fn missing_sensor_channel_requires_explicit_reason() {
        let mut manifest = valid_manifest();
        manifest.sequences[0].events[0].channel_coverage[1].status = ChannelStatus::Missing {
            reason_id: String::new(),
        };
        assert_eq!(
            validate_experiment_manifest(&manifest),
            Err(ManifestError::MissingReason)
        );
    }

    #[test]
    fn present_channel_requires_at_least_one_observation() {
        let mut manifest = valid_manifest();
        manifest.sequences[0].observations.retain(|item| {
            item.contact_event_id != "event-specimen-cal-1" || item.source_id != "d2"
        });
        assert_eq!(
            validate_experiment_manifest(&manifest),
            Err(ManifestError::ChannelMarkedPresentWithoutObservation)
        );
    }

    #[test]
    fn missing_channel_must_not_have_observation_rows() {
        let mut manifest = valid_manifest();
        manifest.sequences[0].events[0].channel_coverage[1].status = ChannelStatus::Missing {
            reason_id: "force-transducer-disconnected".to_owned(),
        };
        assert_eq!(
            validate_experiment_manifest(&manifest),
            Err(ManifestError::ChannelMarkedMissingWithObservation)
        );
    }

    #[test]
    fn observations_from_channels_outside_the_declared_set_are_rejected() {
        let mut manifest = valid_manifest();
        manifest.required_channels = vec![SensorChannel::D1Interface];
        for event in &mut manifest.sequences[0].events {
            event.channel_coverage = vec![ChannelCoverage {
                channel: SensorChannel::D1Interface,
                status: ChannelStatus::Present,
            }];
        }
        manifest.sequences[0]
            .observations
            .retain(|item| item.source_id == "d1");
        manifest.sequences[0].observations.push(observation(
            "extra-d2",
            "event-specimen-cal-1",
            "d2",
            "cal-d2",
            "map-d2",
            "newton",
            94,
            103,
            130,
        ));
        assert_eq!(
            validate_experiment_manifest(&manifest),
            Err(ManifestError::ObservationChannelNotRequired)
        );
    }

    #[test]
    fn clock_mapping_and_sensor_calibration_must_match() {
        let mut manifest = valid_manifest();
        manifest.sequences[0].observations[0].mapping_digest = "wrong-mapping".to_owned();
        assert_eq!(
            validate_experiment_manifest(&manifest),
            Err(ManifestError::ObservationIdentityMismatch)
        );
    }

    #[test]
    fn arrival_cannot_predate_corrected_sample_beyond_uncertainty() {
        let mut manifest = valid_manifest();
        manifest.sequences[0].observations[0].arrival_timestamp_ns = 50;
        assert_eq!(
            validate_experiment_manifest(&manifest),
            Err(ManifestError::ArrivalPredatesCorrectedObservation)
        );
    }

    #[test]
    fn source_samples_must_be_monotone_but_channels_need_not_be_synchronized() {
        let mut manifest = valid_manifest();
        manifest.sequences[0].observations[4].corrected_timestamp_ns = 99;
        assert_eq!(
            validate_experiment_manifest(&manifest),
            Err(ManifestError::CorrectedTimestampNotIncreasing)
        );
        let valid =
            validate_experiment_manifest(&valid_manifest()).expect("asynchronous sources valid");
        assert_eq!(valid.calibration.observations, 6);
    }

    #[test]
    fn out_of_domain_disposition_is_retained_as_unknown_not_success() {
        let mut manifest = valid_manifest();
        manifest.sequences[0].events[0].ground_disposition = GroundDisposition::OutOfDomain {
            reason_id: "outside-granular-profile".to_owned(),
        };
        let report = validate_experiment_manifest(&manifest).expect("out-of-domain observation");
        assert_eq!(report.unknown_or_out_of_domain_contacts, 1);
    }
}
