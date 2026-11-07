// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

#![forbid(unsafe_code)]

//! Evidence-aware knowledge transmission and cultural-memory resilience.
//!
//! Delivery, acquisition, comprehension, demonstrated performance, delayed
//! retention and independent teaching are deliberately separate states.
//! Archive copies are evaluated by failure domain, not raw copy count.

use core::fmt;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EpistemicStatus {
    Verified,
    Provisional,
    Disputed,
    Refuted,
}

impl EpistemicStatus {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Verified => "verified",
            Self::Provisional => "provisional",
            Self::Disputed => "disputed",
            Self::Refuted => "refuted",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcquisitionSource {
    Transmission,
    Archive,
    IndependentDiscovery,
}

impl AcquisitionSource {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Transmission => "transmission",
            Self::Archive => "archive",
            Self::IndependentDiscovery => "independent_discovery",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct KnowledgeItem {
    pub item_id: String,
    pub semantic_family_id: String,
    pub epistemic_status: EpistemicStatus,
    /// Independent source families, not raw URLs or duplicated copies.
    pub evidence_source_families: Vec<String>,
    /// Required knowledge items for this capability or procedure.
    pub prerequisites: Vec<String>,
    /// Minimum score on the item-specific demonstration task.
    pub demonstration_threshold: f64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Participant {
    pub participant_id: String,
    pub generation: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransmissionEvent {
    pub event_id: String,
    pub sender_id: String,
    pub receiver_id: String,
    pub item_id: String,
    pub method: String,
    pub delivered: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetentionAssessment {
    pub delay_steps: u32,
    pub retained: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CarrierState {
    pub participant_id: String,
    pub item_id: String,
    pub acquired: bool,
    pub acquisition_source: Option<AcquisitionSource>,
    pub comprehended: bool,
    /// Optional score on a declared item-specific task, in [0, 1].
    pub demonstration_score: Option<f64>,
    pub retention: Option<RetentionAssessment>,
    pub independently_taught: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchiveCopy {
    pub copy_id: String,
    pub item_id: String,
    /// Copies in one failure domain are not independent redundancy.
    pub failure_domain_id: String,
    pub checksum_verified: bool,
    pub accessible: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KnowledgeDataset {
    pub items: Vec<KnowledgeItem>,
    pub participants: Vec<Participant>,
    pub transmissions: Vec<TransmissionEvent>,
    /// One row per participant and knowledge item; include exposed non-acquirers.
    pub carriers: Vec<CarrierState>,
    pub archive_copies: Vec<ArchiveCopy>,
}

impl KnowledgeDataset {
    pub fn validate(&self) -> Result<(), KnowledgeError> {
        if self.items.is_empty() {
            return Err(KnowledgeError::EmptyItems);
        }
        if self.participants.is_empty() {
            return Err(KnowledgeError::EmptyParticipants);
        }

        let mut item_by_id = BTreeMap::new();
        for (index, item) in self.items.iter().enumerate() {
            if item.item_id.trim().is_empty() || item.semantic_family_id.trim().is_empty() {
                return Err(KnowledgeError::EmptyItemIdentity { index });
            }
            if !item.demonstration_threshold.is_finite()
                || !(0.0..=1.0).contains(&item.demonstration_threshold)
            {
                return Err(KnowledgeError::InvalidDemonstrationThreshold {
                    item_id: item.item_id.clone(),
                    value: item.demonstration_threshold,
                });
            }
            if item_by_id.insert(item.item_id.as_str(), item).is_some() {
                return Err(KnowledgeError::DuplicateItemId(item.item_id.clone()));
            }
            if item.epistemic_status == EpistemicStatus::Verified
                && item.evidence_source_families.is_empty()
            {
                return Err(KnowledgeError::VerifiedWithoutEvidence {
                    item_id: item.item_id.clone(),
                });
            }
            let mut source_families = BTreeSet::new();
            for family in &item.evidence_source_families {
                if family.trim().is_empty() {
                    return Err(KnowledgeError::EmptyEvidenceFamily {
                        item_id: item.item_id.clone(),
                    });
                }
                if !source_families.insert(family.as_str()) {
                    return Err(KnowledgeError::DuplicateEvidenceFamily {
                        item_id: item.item_id.clone(),
                        family_id: family.clone(),
                    });
                }
            }
            let mut prerequisites = BTreeSet::new();
            for prerequisite in &item.prerequisites {
                if prerequisite.trim().is_empty() {
                    return Err(KnowledgeError::EmptyPrerequisite {
                        item_id: item.item_id.clone(),
                    });
                }
                if !prerequisites.insert(prerequisite.as_str()) {
                    return Err(KnowledgeError::DuplicatePrerequisite {
                        item_id: item.item_id.clone(),
                        prerequisite_id: prerequisite.clone(),
                    });
                }
            }
        }

        for item in &self.items {
            for prerequisite in &item.prerequisites {
                if !item_by_id.contains_key(prerequisite.as_str()) {
                    return Err(KnowledgeError::UnknownPrerequisite {
                        item_id: item.item_id.clone(),
                        prerequisite_id: prerequisite.clone(),
                    });
                }
            }
        }
        validate_prerequisite_graph(&self.items)?;

        let mut participant_by_id = BTreeMap::new();
        for participant in &self.participants {
            if participant.participant_id.trim().is_empty() {
                return Err(KnowledgeError::EmptyParticipantId);
            }
            if participant_by_id
                .insert(participant.participant_id.as_str(), participant.generation)
                .is_some()
            {
                return Err(KnowledgeError::DuplicateParticipantId(
                    participant.participant_id.clone(),
                ));
            }
        }

        let mut event_ids = BTreeSet::new();
        let mut delivered_to = BTreeSet::new();
        for event in &self.transmissions {
            if event.event_id.trim().is_empty() || event.method.trim().is_empty() {
                return Err(KnowledgeError::EmptyTransmissionIdentity);
            }
            if !event_ids.insert(event.event_id.as_str()) {
                return Err(KnowledgeError::DuplicateTransmissionId(
                    event.event_id.clone(),
                ));
            }
            let sender_generation = participant_by_id
                .get(event.sender_id.as_str())
                .copied()
                .ok_or_else(|| KnowledgeError::UnknownParticipant(event.sender_id.clone()))?;
            let receiver_generation = participant_by_id
                .get(event.receiver_id.as_str())
                .copied()
                .ok_or_else(|| KnowledgeError::UnknownParticipant(event.receiver_id.clone()))?;
            if sender_generation > receiver_generation {
                return Err(KnowledgeError::FutureToPastTransmission {
                    sender_id: event.sender_id.clone(),
                    receiver_id: event.receiver_id.clone(),
                });
            }
            if !item_by_id.contains_key(event.item_id.as_str()) {
                return Err(KnowledgeError::UnknownItem(event.item_id.clone()));
            }
            if event.delivered {
                delivered_to.insert((event.receiver_id.as_str(), event.item_id.as_str()));
            }
        }

        let mut carrier_keys = BTreeSet::new();
        for carrier in &self.carriers {
            if !participant_by_id.contains_key(carrier.participant_id.as_str()) {
                return Err(KnowledgeError::UnknownParticipant(
                    carrier.participant_id.clone(),
                ));
            }
            if !item_by_id.contains_key(carrier.item_id.as_str()) {
                return Err(KnowledgeError::UnknownItem(carrier.item_id.clone()));
            }
            if !carrier_keys.insert((carrier.participant_id.as_str(), carrier.item_id.as_str())) {
                return Err(KnowledgeError::DuplicateCarrierState {
                    participant_id: carrier.participant_id.clone(),
                    item_id: carrier.item_id.clone(),
                });
            }

            if carrier.acquired != carrier.acquisition_source.is_some() {
                return Err(KnowledgeError::AcquisitionSourceMismatch {
                    participant_id: carrier.participant_id.clone(),
                    item_id: carrier.item_id.clone(),
                });
            }
            if carrier.acquired
                && carrier.acquisition_source == Some(AcquisitionSource::Transmission)
                && !delivered_to
                    .contains(&(carrier.participant_id.as_str(), carrier.item_id.as_str()))
            {
                return Err(KnowledgeError::AcquisitionWithoutDeliveredTransmission {
                    participant_id: carrier.participant_id.clone(),
                    item_id: carrier.item_id.clone(),
                });
            }
            if carrier.acquired
                && carrier.acquisition_source == Some(AcquisitionSource::Archive)
                && !self.archive_copies.iter().any(|copy| {
                    copy.item_id == carrier.item_id && copy.checksum_verified && copy.accessible
                })
            {
                return Err(KnowledgeError::ArchiveAcquisitionWithoutVerifiedCopy {
                    participant_id: carrier.participant_id.clone(),
                    item_id: carrier.item_id.clone(),
                });
            }
            if carrier.comprehended && !carrier.acquired {
                return Err(KnowledgeError::ComprehensionWithoutAcquisition {
                    participant_id: carrier.participant_id.clone(),
                    item_id: carrier.item_id.clone(),
                });
            }
            if let Some(score) = carrier.demonstration_score
                && (!carrier.acquired || !score.is_finite() || !(0.0..=1.0).contains(&score))
            {
                return Err(KnowledgeError::InvalidDemonstration {
                    participant_id: carrier.participant_id.clone(),
                    item_id: carrier.item_id.clone(),
                    score,
                });
            }
            if let Some(retention) = carrier.retention
                && (!carrier.acquired || retention.delay_steps == 0)
            {
                return Err(KnowledgeError::InvalidRetentionAssessment {
                    participant_id: carrier.participant_id.clone(),
                    item_id: carrier.item_id.clone(),
                });
            }
            if carrier.independently_taught && !carrier.acquired {
                return Err(KnowledgeError::TeachingWithoutAcquisition {
                    participant_id: carrier.participant_id.clone(),
                    item_id: carrier.item_id.clone(),
                });
            }
        }

        let mut archive_ids = BTreeSet::new();
        for copy in &self.archive_copies {
            if copy.copy_id.trim().is_empty() || copy.failure_domain_id.trim().is_empty() {
                return Err(KnowledgeError::EmptyArchiveIdentity);
            }
            if !archive_ids.insert(copy.copy_id.as_str()) {
                return Err(KnowledgeError::DuplicateArchiveId(copy.copy_id.clone()));
            }
            if !item_by_id.contains_key(copy.item_id.as_str()) {
                return Err(KnowledgeError::UnknownItem(copy.item_id.clone()));
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum KnowledgeError {
    EmptyItems,
    EmptyParticipants,
    EmptyItemIdentity {
        index: usize,
    },
    DuplicateItemId(String),
    InvalidDemonstrationThreshold {
        item_id: String,
        value: f64,
    },
    EmptyEvidenceFamily {
        item_id: String,
    },
    VerifiedWithoutEvidence {
        item_id: String,
    },
    DuplicateEvidenceFamily {
        item_id: String,
        family_id: String,
    },
    EmptyPrerequisite {
        item_id: String,
    },
    DuplicatePrerequisite {
        item_id: String,
        prerequisite_id: String,
    },
    UnknownPrerequisite {
        item_id: String,
        prerequisite_id: String,
    },
    PrerequisiteCycle,
    EmptyParticipantId,
    DuplicateParticipantId(String),
    EmptyTransmissionIdentity,
    DuplicateTransmissionId(String),
    UnknownParticipant(String),
    UnknownItem(String),
    FutureToPastTransmission {
        sender_id: String,
        receiver_id: String,
    },
    DuplicateCarrierState {
        participant_id: String,
        item_id: String,
    },
    AcquisitionSourceMismatch {
        participant_id: String,
        item_id: String,
    },
    AcquisitionWithoutDeliveredTransmission {
        participant_id: String,
        item_id: String,
    },
    ArchiveAcquisitionWithoutVerifiedCopy {
        participant_id: String,
        item_id: String,
    },
    ComprehensionWithoutAcquisition {
        participant_id: String,
        item_id: String,
    },
    InvalidDemonstration {
        participant_id: String,
        item_id: String,
        score: f64,
    },
    InvalidRetentionAssessment {
        participant_id: String,
        item_id: String,
    },
    TeachingWithoutAcquisition {
        participant_id: String,
        item_id: String,
    },
    EmptyArchiveIdentity,
    DuplicateArchiveId(String),
}

impl fmt::Display for KnowledgeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyItems => write!(f, "at least one knowledge item is required"),
            Self::EmptyParticipants => write!(f, "at least one participant is required"),
            Self::EmptyItemIdentity { index } => {
                write!(f, "knowledge item at index {index} has an empty identity")
            }
            Self::DuplicateItemId(id) => write!(f, "duplicate knowledge item ID: {id}"),
            Self::InvalidDemonstrationThreshold { item_id, value } => {
                write!(
                    f,
                    "item {item_id} has invalid demonstration threshold {value}"
                )
            }
            Self::EmptyEvidenceFamily { item_id } => {
                write!(f, "item {item_id} has an empty evidence source family")
            }
            Self::VerifiedWithoutEvidence { item_id } => {
                write!(
                    f,
                    "verified item {item_id} has no declared evidence source family"
                )
            }
            Self::DuplicateEvidenceFamily { item_id, family_id } => {
                write!(f, "item {item_id} repeats evidence family {family_id}")
            }
            Self::EmptyPrerequisite { item_id } => {
                write!(f, "item {item_id} has an empty prerequisite ID")
            }
            Self::DuplicatePrerequisite {
                item_id,
                prerequisite_id,
            } => {
                write!(f, "item {item_id} repeats prerequisite {prerequisite_id}")
            }
            Self::UnknownPrerequisite {
                item_id,
                prerequisite_id,
            } => {
                write!(
                    f,
                    "item {item_id} references unknown prerequisite {prerequisite_id}"
                )
            }
            Self::PrerequisiteCycle => write!(f, "knowledge prerequisite graph contains a cycle"),
            Self::EmptyParticipantId => write!(f, "participant ID must not be empty"),
            Self::DuplicateParticipantId(id) => write!(f, "duplicate participant ID: {id}"),
            Self::EmptyTransmissionIdentity => {
                write!(f, "transmission event ID and method must not be empty")
            }
            Self::DuplicateTransmissionId(id) => write!(f, "duplicate transmission ID: {id}"),
            Self::UnknownParticipant(id) => write!(f, "unknown participant: {id}"),
            Self::UnknownItem(id) => write!(f, "unknown knowledge item: {id}"),
            Self::FutureToPastTransmission {
                sender_id,
                receiver_id,
            } => {
                write!(
                    f,
                    "future participant {sender_id} cannot teach earlier participant {receiver_id}"
                )
            }
            Self::DuplicateCarrierState {
                participant_id,
                item_id,
            } => {
                write!(
                    f,
                    "duplicate carrier state for ({participant_id}, {item_id})"
                )
            }
            Self::AcquisitionSourceMismatch {
                participant_id,
                item_id,
            } => {
                write!(
                    f,
                    "acquisition/source mismatch for ({participant_id}, {item_id})"
                )
            }
            Self::AcquisitionWithoutDeliveredTransmission {
                participant_id,
                item_id,
            } => {
                write!(
                    f,
                    "transmission acquisition for ({participant_id}, {item_id}) lacks a delivered event"
                )
            }
            Self::ArchiveAcquisitionWithoutVerifiedCopy {
                participant_id,
                item_id,
            } => {
                write!(
                    f,
                    "archive acquisition for ({participant_id}, {item_id}) lacks a verified copy"
                )
            }
            Self::ComprehensionWithoutAcquisition {
                participant_id,
                item_id,
            } => {
                write!(
                    f,
                    "comprehension without acquisition for ({participant_id}, {item_id})"
                )
            }
            Self::InvalidDemonstration {
                participant_id,
                item_id,
                score,
            } => {
                write!(
                    f,
                    "invalid demonstration {score} for ({participant_id}, {item_id})"
                )
            }
            Self::InvalidRetentionAssessment {
                participant_id,
                item_id,
            } => {
                write!(
                    f,
                    "invalid delayed retention assessment for ({participant_id}, {item_id})"
                )
            }
            Self::TeachingWithoutAcquisition {
                participant_id,
                item_id,
            } => {
                write!(
                    f,
                    "independent teaching without acquisition for ({participant_id}, {item_id})"
                )
            }
            Self::EmptyArchiveIdentity => {
                write!(f, "archive copy ID and failure domain must not be empty")
            }
            Self::DuplicateArchiveId(id) => write!(f, "duplicate archive copy ID: {id}"),
        }
    }
}

impl std::error::Error for KnowledgeError {}

#[derive(Debug, Clone, PartialEq)]
pub struct ItemMetrics {
    pub item_id: String,
    pub semantic_family_id: String,
    pub epistemic_status: EpistemicStatus,
    pub source_evidence_family_count: usize,
    pub carrier_state_count: usize,
    pub transmission_events: usize,
    pub delivered_transmissions: usize,
    pub acquisition_count: usize,
    pub comprehension_count: usize,
    pub demonstration_assessed_count: usize,
    pub demonstration_success_count: usize,
    pub retention_assessed_count: usize,
    pub retained_count: usize,
    pub independent_teacher_count: usize,
    pub refuted_knowledge_acquisition_count: usize,
    pub distinct_demonstrated_generations: usize,
    pub longest_consecutive_demonstrated_generations: usize,
    pub verified_archive_copy_count: usize,
    pub independent_archive_failure_domain_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchiveRecovery {
    pub item_id: String,
    pub surviving_verified_copy_count: usize,
    pub recoverable: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct KnowledgeEvaluation {
    pub items: Vec<ItemMetrics>,
    pub archive_recovery: Vec<ArchiveRecovery>,
}

pub fn evaluate(dataset: &KnowledgeDataset) -> Result<KnowledgeEvaluation, KnowledgeError> {
    dataset.validate()?;
    let mut metrics = Vec::with_capacity(dataset.items.len());

    for item in &dataset.items {
        let item_transmissions = dataset
            .transmissions
            .iter()
            .filter(|event| event.item_id == item.item_id)
            .collect::<Vec<_>>();
        let item_carriers = dataset
            .carriers
            .iter()
            .filter(|carrier| carrier.item_id == item.item_id)
            .collect::<Vec<_>>();
        let demonstrated_generations = item_carriers
            .iter()
            .filter(|carrier| {
                carrier
                    .demonstration_score
                    .is_some_and(|score| score >= item.demonstration_threshold)
            })
            .map(|carrier| {
                dataset
                    .participants
                    .iter()
                    .find(|participant| participant.participant_id == carrier.participant_id)
                    .expect("validated participant must exist")
                    .generation
            })
            .collect::<BTreeSet<_>>();
        let archive_copies = dataset
            .archive_copies
            .iter()
            .filter(|copy| copy.item_id == item.item_id && copy.checksum_verified)
            .collect::<Vec<_>>();
        let archive_domains = archive_copies
            .iter()
            .map(|copy| copy.failure_domain_id.as_str())
            .collect::<BTreeSet<_>>();

        let acquisition_count = item_carriers
            .iter()
            .filter(|carrier| carrier.acquired)
            .count();
        let comprehension_count = item_carriers
            .iter()
            .filter(|carrier| carrier.comprehended)
            .count();
        let demonstration_assessed_count = item_carriers
            .iter()
            .filter(|carrier| carrier.demonstration_score.is_some())
            .count();
        let demonstration_success_count = item_carriers
            .iter()
            .filter(|carrier| {
                carrier
                    .demonstration_score
                    .is_some_and(|score| score >= item.demonstration_threshold)
            })
            .count();
        let retention_assessed_count = item_carriers
            .iter()
            .filter(|carrier| carrier.retention.is_some())
            .count();
        let retained_count = item_carriers
            .iter()
            .filter(|carrier| carrier.retention.is_some_and(|result| result.retained))
            .count();

        metrics.push(ItemMetrics {
            item_id: item.item_id.clone(),
            semantic_family_id: item.semantic_family_id.clone(),
            epistemic_status: item.epistemic_status,
            source_evidence_family_count: item.evidence_source_families.len(),
            carrier_state_count: item_carriers.len(),
            transmission_events: item_transmissions.len(),
            delivered_transmissions: item_transmissions
                .iter()
                .filter(|event| event.delivered)
                .count(),
            acquisition_count,
            comprehension_count,
            demonstration_assessed_count,
            demonstration_success_count,
            retention_assessed_count,
            retained_count,
            independent_teacher_count: item_carriers
                .iter()
                .filter(|carrier| carrier.independently_taught)
                .count(),
            refuted_knowledge_acquisition_count: if item.epistemic_status
                == EpistemicStatus::Refuted
            {
                acquisition_count
            } else {
                0
            },
            distinct_demonstrated_generations: demonstrated_generations.len(),
            longest_consecutive_demonstrated_generations: longest_consecutive_run(
                &demonstrated_generations,
            ),
            verified_archive_copy_count: archive_copies.len(),
            independent_archive_failure_domain_count: archive_domains.len(),
        });
    }

    Ok(KnowledgeEvaluation {
        items: metrics,
        archive_recovery: archive_recovery_after_failures(dataset, &BTreeSet::new()),
    })
}

pub fn archive_recovery_after_failures(
    dataset: &KnowledgeDataset,
    failed_domains: &BTreeSet<String>,
) -> Vec<ArchiveRecovery> {
    dataset
        .items
        .iter()
        .map(|item| {
            let surviving_verified_copy_count = dataset
                .archive_copies
                .iter()
                .filter(|copy| {
                    copy.item_id == item.item_id
                        && copy.checksum_verified
                        && copy.accessible
                        && !failed_domains.contains(&copy.failure_domain_id)
                })
                .count();
            ArchiveRecovery {
                item_id: item.item_id.clone(),
                surviving_verified_copy_count,
                recoverable: surviving_verified_copy_count > 0,
            }
        })
        .collect()
}

fn longest_consecutive_run(generations: &BTreeSet<u32>) -> usize {
    let mut longest = 0usize;
    let mut current = 0usize;
    let mut previous: Option<u32> = None;
    for generation in generations {
        if previous.is_some_and(|last| last.checked_add(1) == Some(*generation)) {
            current += 1;
        } else {
            current = 1;
        }
        longest = longest.max(current);
        previous = Some(*generation);
    }
    longest
}

fn validate_prerequisite_graph(items: &[KnowledgeItem]) -> Result<(), KnowledgeError> {
    let index_by_id = items
        .iter()
        .enumerate()
        .map(|(index, item)| (item.item_id.as_str(), index))
        .collect::<BTreeMap<_, _>>();
    let mut indegree = vec![0usize; items.len()];
    let mut outgoing = vec![Vec::new(); items.len()];

    for (child, item) in items.iter().enumerate() {
        for prerequisite in &item.prerequisites {
            let parent = index_by_id[prerequisite.as_str()];
            outgoing[parent].push(child);
            indegree[child] += 1;
        }
    }
    let mut ready = BTreeSet::new();
    for (index, value) in indegree.iter().enumerate() {
        if *value == 0 {
            ready.insert(index);
        }
    }
    let mut visited = 0usize;
    while let Some(index) = ready.pop_first() {
        visited += 1;
        for child in &outgoing[index] {
            indegree[*child] -= 1;
            if indegree[*child] == 0 {
                ready.insert(*child);
            }
        }
    }
    if visited == items.len() {
        Ok(())
    } else {
        Err(KnowledgeError::PrerequisiteCycle)
    }
}

pub fn reference_dataset() -> KnowledgeDataset {
    let items = vec![
        KnowledgeItem {
            item_id: "hydrology_skill".into(),
            semantic_family_id: "water_systems".into(),
            epistemic_status: EpistemicStatus::Verified,
            evidence_source_families: vec!["field_measurement".into(), "independent_lab".into()],
            prerequisites: Vec::new(),
            demonstration_threshold: 0.7,
        },
        KnowledgeItem {
            item_id: "silt_myth".into(),
            semantic_family_id: "water_systems".into(),
            epistemic_status: EpistemicStatus::Refuted,
            evidence_source_families: vec!["unverified_rumour".into()],
            prerequisites: Vec::new(),
            demonstration_threshold: 0.7,
        },
    ];
    let participants = vec![
        Participant {
            participant_id: "agent_a".into(),
            generation: 0,
        },
        Participant {
            participant_id: "agent_b".into(),
            generation: 0,
        },
        Participant {
            participant_id: "agent_c".into(),
            generation: 1,
        },
        Participant {
            participant_id: "agent_d".into(),
            generation: 1,
        },
        Participant {
            participant_id: "agent_e".into(),
            generation: 2,
        },
        Participant {
            participant_id: "agent_f".into(),
            generation: 2,
        },
    ];
    let transmissions = vec![
        transmission("tx_01", "agent_a", "agent_c", "hydrology_skill", true),
        transmission("tx_02", "agent_a", "agent_d", "hydrology_skill", true),
        transmission("tx_03", "agent_c", "agent_e", "hydrology_skill", true),
        transmission("tx_04", "agent_a", "agent_b", "silt_myth", true),
        transmission("tx_05", "agent_b", "agent_f", "silt_myth", true),
    ];
    let carriers = vec![
        carrier(
            "agent_a",
            "hydrology_skill",
            Some(AcquisitionSource::IndependentDiscovery),
            true,
            Some(0.95),
            Some(true),
            true,
        ),
        carrier(
            "agent_c",
            "hydrology_skill",
            Some(AcquisitionSource::Transmission),
            true,
            Some(0.90),
            Some(true),
            true,
        ),
        carrier("agent_d", "hydrology_skill", None, false, None, None, false),
        carrier(
            "agent_e",
            "hydrology_skill",
            Some(AcquisitionSource::Transmission),
            true,
            Some(0.80),
            Some(false),
            false,
        ),
        carrier(
            "agent_b",
            "silt_myth",
            Some(AcquisitionSource::Transmission),
            true,
            Some(0.85),
            Some(true),
            true,
        ),
        carrier("agent_f", "silt_myth", None, false, None, None, false),
    ];
    let archive_copies = vec![
        archive(
            "hydro_copy_a",
            "hydrology_skill",
            "archive_domain_a",
            true,
            true,
        ),
        archive(
            "hydro_copy_b",
            "hydrology_skill",
            "archive_domain_b",
            true,
            true,
        ),
        archive(
            "hydro_copy_c",
            "hydrology_skill",
            "archive_domain_b",
            true,
            true,
        ),
        archive("silt_copy_a", "silt_myth", "archive_domain_a", false, true),
    ];

    KnowledgeDataset {
        items,
        participants,
        transmissions,
        carriers,
        archive_copies,
    }
}

fn transmission(
    event_id: &str,
    sender_id: &str,
    receiver_id: &str,
    item_id: &str,
    delivered: bool,
) -> TransmissionEvent {
    TransmissionEvent {
        event_id: event_id.into(),
        sender_id: sender_id.into(),
        receiver_id: receiver_id.into(),
        item_id: item_id.into(),
        method: "demonstration".into(),
        delivered,
    }
}

fn carrier(
    participant_id: &str,
    item_id: &str,
    acquisition_source: Option<AcquisitionSource>,
    comprehended: bool,
    demonstration_score: Option<f64>,
    retained: Option<bool>,
    independently_taught: bool,
) -> CarrierState {
    let acquired = acquisition_source.is_some();
    CarrierState {
        participant_id: participant_id.into(),
        item_id: item_id.into(),
        acquired,
        acquisition_source,
        comprehended,
        demonstration_score,
        retention: retained.map(|retained| RetentionAssessment {
            delay_steps: 30,
            retained,
        }),
        independently_taught,
    }
}

fn archive(
    copy_id: &str,
    item_id: &str,
    failure_domain_id: &str,
    checksum_verified: bool,
    accessible: bool,
) -> ArchiveCopy {
    ArchiveCopy {
        copy_id: copy_id.into(),
        item_id: item_id.into(),
        failure_domain_id: failure_domain_id.into(),
        checksum_verified,
        accessible,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delivery_is_not_equivalent_to_acquisition_or_durable_capability() {
        let evaluation = evaluate(&reference_dataset()).unwrap();
        let hydro = evaluation
            .items
            .iter()
            .find(|item| item.item_id == "hydrology_skill")
            .unwrap();
        assert_eq!(hydro.delivered_transmissions, 3);
        assert_eq!(hydro.acquisition_count, 3);
        assert_eq!(hydro.comprehension_count, 3);
        assert_eq!(hydro.demonstration_success_count, 3);
        assert_eq!(hydro.retention_assessed_count, 3);
        assert_eq!(hydro.retained_count, 2);
        assert_eq!(hydro.independent_teacher_count, 2);
        assert_eq!(hydro.longest_consecutive_demonstrated_generations, 3);
    }

    #[test]
    fn refuted_knowledge_acquisition_is_reported_separately() {
        let evaluation = evaluate(&reference_dataset()).unwrap();
        let myth = evaluation
            .items
            .iter()
            .find(|item| item.item_id == "silt_myth")
            .unwrap();
        assert_eq!(myth.epistemic_status, EpistemicStatus::Refuted);
        assert_eq!(myth.acquisition_count, 1);
        assert_eq!(myth.refuted_knowledge_acquisition_count, 1);
        assert_eq!(myth.independent_teacher_count, 1);
    }

    #[test]
    fn independent_archive_failure_domains_matter_more_than_copy_count() {
        let dataset = reference_dataset();
        let evaluation = evaluate(&dataset).unwrap();
        let hydro = evaluation
            .items
            .iter()
            .find(|item| item.item_id == "hydrology_skill")
            .unwrap();
        assert_eq!(hydro.verified_archive_copy_count, 3);
        assert_eq!(hydro.independent_archive_failure_domain_count, 2);

        let failures = BTreeSet::from(["archive_domain_a".to_owned()]);
        let recovery = archive_recovery_after_failures(&dataset, &failures);
        let hydro_recovery = recovery
            .iter()
            .find(|item| item.item_id == "hydrology_skill")
            .unwrap();
        let myth_recovery = recovery
            .iter()
            .find(|item| item.item_id == "silt_myth")
            .unwrap();
        assert_eq!(hydro_recovery.surviving_verified_copy_count, 2);
        assert!(hydro_recovery.recoverable);
        assert_eq!(myth_recovery.surviving_verified_copy_count, 0);
        assert!(!myth_recovery.recoverable);
    }

    #[test]
    fn verified_status_requires_at_least_one_declared_evidence_family() {
        let mut dataset = reference_dataset();
        dataset.items[0].evidence_source_families.clear();
        assert!(matches!(
            dataset.validate(),
            Err(KnowledgeError::VerifiedWithoutEvidence { .. })
        ));
    }

    #[test]
    fn duplicate_evidence_families_do_not_inflate_corroboration() {
        let mut dataset = reference_dataset();
        dataset.items[0]
            .evidence_source_families
            .push("field_measurement".into());
        assert!(matches!(
            dataset.validate(),
            Err(KnowledgeError::DuplicateEvidenceFamily { .. })
        ));
    }

    #[test]
    fn knowledge_prerequisite_cycles_fail_closed() {
        let mut dataset = reference_dataset();
        dataset.items[0].prerequisites.push("silt_myth".into());
        dataset.items[1]
            .prerequisites
            .push("hydrology_skill".into());
        assert!(matches!(
            dataset.validate(),
            Err(KnowledgeError::PrerequisiteCycle)
        ));
    }

    #[test]
    fn transmission_from_a_future_generation_fails_closed() {
        let mut dataset = reference_dataset();
        dataset.transmissions[0].sender_id = "agent_e".into();
        assert!(matches!(
            dataset.validate(),
            Err(KnowledgeError::FutureToPastTransmission { .. })
        ));
    }

    #[test]
    fn comprehension_and_teaching_without_acquisition_fail_closed() {
        let mut dataset = reference_dataset();
        dataset.carriers[2].comprehended = true;
        assert!(matches!(
            dataset.validate(),
            Err(KnowledgeError::ComprehensionWithoutAcquisition { .. })
        ));
    }

    #[test]
    fn archive_acquisition_requires_a_verified_copy() {
        let mut dataset = reference_dataset();
        dataset.carriers[2].acquired = true;
        dataset.carriers[2].acquisition_source = Some(AcquisitionSource::Archive);
        dataset.carriers[2].comprehended = true;
        dataset
            .archive_copies
            .retain(|copy| copy.item_id != "hydrology_skill");
        assert!(matches!(
            dataset.validate(),
            Err(KnowledgeError::ArchiveAcquisitionWithoutVerifiedCopy { .. })
        ));
    }
}
