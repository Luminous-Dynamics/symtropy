// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Open projects, independent contributions, and evidence-bearing acceptance.
//!
//! Publishing a project does not create organization membership or employment.
//! Submitting a contribution does not mean it is accepted. Acceptance does not
//! execute payment or settlement. Human actors, institutions, autonomous agents,
//! and other contributors use the same contribution primitive.

use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    error::Error,
    fmt,
};
use symtropy_game_state::StableId;

/// Exact evidence reference owned by another authority/domain.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ProjectEvidenceRef {
    /// Causal-history event.
    Event(StableId),
    /// Economy process completion.
    EconomyProcessCompletion(StableId),
    /// Economy shipment arrival.
    EconomyShipmentArrival(StableId),
    /// Asset relation or asset-domain receipt.
    AssetRecord(StableId),
    /// Design/analysis/fabrication/other external evidence identity.
    External {
        namespace: String,
        id: StableId,
    },
}

/// One explicit deliverable required by a project.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeliverableSpec {
    /// Stable project-local deliverable identity.
    pub id: StableId,
    /// Machine-readable deliverable family such as `cargo-delivery`,
    /// `repair-evidence`, `survey`, `design`, or scenario-specific vocabulary.
    pub kind: String,
    /// Human/tool-readable requirement summary.
    pub description: String,
    /// Whether this deliverable must be accepted before the project is complete.
    pub required: bool,
}

/// Immutable project specification published by one issuer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectSpec {
    /// Stable project identity.
    pub id: StableId,
    /// Actor/institution publishing the work.
    pub issuer_id: StableId,
    /// Human-readable title.
    pub title: String,
    /// Canonical tick at which submissions may begin.
    pub opens_tick: u64,
    /// Optional last tick at which new contributions may be submitted, inclusive.
    pub closes_tick: Option<u64>,
    /// Explicit deliverables keyed by stable identity.
    pub deliverables: BTreeMap<StableId, DeliverableSpec>,
    /// Optional reviewers allowed to issue acceptance decisions.
    ///
    /// If empty, only the issuer may review in V0.
    pub reviewer_ids: BTreeSet<StableId>,
    /// Causal-history event that published this exact project specification.
    pub source_event_id: StableId,
}

impl ProjectSpec {
    pub fn is_open(&self, tick: u64) -> bool {
        tick >= self.opens_tick && self.closes_tick.is_none_or(|close| tick <= close)
    }

    pub fn reviewer_allowed(&self, reviewer_id: &StableId) -> bool {
        reviewer_id == &self.issuer_id
            || (!self.reviewer_ids.is_empty() && self.reviewer_ids.contains(reviewer_id))
    }
}

/// Immutable contribution by any actor/institution/agent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Contribution {
    pub id: StableId,
    pub project_id: StableId,
    /// No membership relation is required or implied.
    pub contributor_id: StableId,
    /// Deliverables this contribution claims to address.
    pub deliverable_ids: BTreeSet<StableId>,
    /// Evidence supporting the claimed contribution.
    pub evidence: BTreeSet<ProjectEvidenceRef>,
    pub submitted_tick: u64,
    pub source_event_id: StableId,
}

/// Per-deliverable disposition in an immutable review decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DeliverableDisposition {
    Accepted,
    Rejected,
}

/// Immutable reviewer decision over one contribution.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContributionReview {
    pub id: StableId,
    pub contribution_id: StableId,
    pub reviewer_id: StableId,
    /// Only deliverables actually claimed by the contribution may be decided.
    pub dispositions: BTreeMap<StableId, DeliverableDisposition>,
    /// Optional evidence or record refs cited by the reviewer.
    pub review_evidence: BTreeSet<ProjectEvidenceRef>,
    pub reviewed_tick: u64,
    pub source_event_id: StableId,
}

/// Derived completion state. It has no payment/settlement semantics.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectCompletion {
    /// Required deliverables that have at least one accepted contribution.
    pub accepted_required: BTreeSet<StableId>,
    /// Required deliverables still lacking any accepted contribution.
    pub missing_required: BTreeSet<StableId>,
}

impl ProjectCompletion {
    pub fn is_complete(&self) -> bool {
        self.missing_required.is_empty()
    }
}

/// Append-only project/contribution/review state.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ProjectLedger {
    projects: BTreeMap<StableId, ProjectSpec>,
    contributions: BTreeMap<StableId, Contribution>,
    reviews: BTreeMap<StableId, ContributionReview>,
    review_ids_by_contribution: BTreeMap<StableId, BTreeSet<StableId>>,
}

impl ProjectLedger {
    pub fn new() -> Self {
        Self::default()
    }

    /// Publishes an immutable project. This does not add any contributor to an institution.
    pub fn publish_project(&mut self, project: ProjectSpec) -> Result<(), ProjectError> {
        validate_project(&project)?;
        if self.projects.contains_key(&project.id) {
            return Err(ProjectError::DuplicateProject(project.id));
        }
        self.projects.insert(project.id.clone(), project);
        Ok(())
    }

    pub fn project(&self, project_id: &StableId) -> Option<&ProjectSpec> {
        self.projects.get(project_id)
    }

    /// Submits a contribution without granting membership, office, authority, or payment.
    pub fn submit(&mut self, contribution: Contribution) -> Result<(), ProjectError> {
        if self.contributions.contains_key(&contribution.id) {
            return Err(ProjectError::DuplicateContribution(contribution.id));
        }
        let project = self
            .projects
            .get(&contribution.project_id)
            .ok_or_else(|| ProjectError::UnknownProject(contribution.project_id.clone()))?;
        if !project.is_open(contribution.submitted_tick) {
            return Err(ProjectError::ProjectClosedAtSubmission {
                project_id: contribution.project_id,
                submitted_tick: contribution.submitted_tick,
            });
        }
        if contribution.deliverable_ids.is_empty() {
            return Err(ProjectError::EmptyContribution(contribution.id));
        }
        for deliverable_id in &contribution.deliverable_ids {
            if !project.deliverables.contains_key(deliverable_id) {
                return Err(ProjectError::UnknownDeliverable {
                    project_id: contribution.project_id.clone(),
                    deliverable_id: deliverable_id.clone(),
                });
            }
        }
        validate_evidence_refs(&contribution.evidence)?;
        self.contributions
            .insert(contribution.id.clone(), contribution);
        Ok(())
    }

    pub fn contribution(&self, contribution_id: &StableId) -> Option<&Contribution> {
        self.contributions.get(contribution_id)
    }

    /// Records a review. The reviewer may accept/reject only deliverables claimed
    /// by the referenced contribution. This still does not settle money/rewards.
    pub fn review(&mut self, review: ContributionReview) -> Result<(), ProjectError> {
        if self.reviews.contains_key(&review.id) {
            return Err(ProjectError::DuplicateReview(review.id));
        }
        let contribution = self
            .contributions
            .get(&review.contribution_id)
            .ok_or_else(|| ProjectError::UnknownContribution(review.contribution_id.clone()))?;
        let project = self
            .projects
            .get(&contribution.project_id)
            .expect("retained contribution references retained project");
        if !project.reviewer_allowed(&review.reviewer_id) {
            return Err(ProjectError::ReviewerNotAuthorized {
                project_id: project.id.clone(),
                reviewer_id: review.reviewer_id,
            });
        }
        if review.reviewed_tick < contribution.submitted_tick {
            return Err(ProjectError::ReviewBeforeSubmission {
                contribution_id: contribution.id.clone(),
                submitted_tick: contribution.submitted_tick,
                reviewed_tick: review.reviewed_tick,
            });
        }
        if review.dispositions.is_empty() {
            return Err(ProjectError::EmptyReview(review.id));
        }
        for deliverable_id in review.dispositions.keys() {
            if !contribution.deliverable_ids.contains(deliverable_id) {
                return Err(ProjectError::ReviewOfUnclaimedDeliverable {
                    contribution_id: contribution.id.clone(),
                    deliverable_id: deliverable_id.clone(),
                });
            }
        }
        validate_evidence_refs(&review.review_evidence)?;
        self.review_ids_by_contribution
            .entry(review.contribution_id.clone())
            .or_default()
            .insert(review.id.clone());
        self.reviews.insert(review.id.clone(), review);
        Ok(())
    }

    pub fn review_by_id(&self, review_id: &StableId) -> Option<&ContributionReview> {
        self.reviews.get(review_id)
    }

    pub fn reviews_for_contribution<'a>(
        &'a self,
        contribution_id: &'a StableId,
    ) -> impl Iterator<Item = &'a ContributionReview> {
        self.review_ids_by_contribution
            .get(contribution_id)
            .into_iter()
            .flat_map(|ids| ids.iter())
            .filter_map(|id| self.reviews.get(id))
    }

    /// Derives project completion solely from accepted deliverable decisions.
    ///
    /// Multiple contributors can satisfy different or the same deliverables.
    pub fn completion(&self, project_id: &StableId) -> Result<ProjectCompletion, ProjectError> {
        let project = self
            .projects
            .get(project_id)
            .ok_or_else(|| ProjectError::UnknownProject(project_id.clone()))?;
        let required = project
            .deliverables
            .values()
            .filter(|deliverable| deliverable.required)
            .map(|deliverable| deliverable.id.clone())
            .collect::<BTreeSet<_>>();
        let mut accepted = BTreeSet::new();

        for review in self.reviews.values() {
            let Some(contribution) = self.contributions.get(&review.contribution_id) else {
                continue;
            };
            if contribution.project_id != *project_id {
                continue;
            }
            for (deliverable_id, disposition) in &review.dispositions {
                if *disposition == DeliverableDisposition::Accepted && required.contains(deliverable_id) {
                    accepted.insert(deliverable_id.clone());
                }
            }
        }

        let missing_required = required.difference(&accepted).cloned().collect();
        Ok(ProjectCompletion {
            accepted_required: accepted,
            missing_required,
        })
    }
}

fn validate_project(project: &ProjectSpec) -> Result<(), ProjectError> {
    if project.title.trim().is_empty() {
        return Err(ProjectError::EmptyProjectTitle(project.id.clone()));
    }
    if project
        .closes_tick
        .is_some_and(|close| close < project.opens_tick)
    {
        return Err(ProjectError::InvalidProjectWindow {
            project_id: project.id.clone(),
            opens_tick: project.opens_tick,
            closes_tick: project.closes_tick,
        });
    }
    if project.deliverables.is_empty() {
        return Err(ProjectError::ProjectHasNoDeliverables(project.id.clone()));
    }
    for (key, deliverable) in &project.deliverables {
        if key != &deliverable.id {
            return Err(ProjectError::DeliverableKeyMismatch {
                project_id: project.id.clone(),
                key: key.clone(),
                actual: deliverable.id.clone(),
            });
        }
        if deliverable.kind.trim().is_empty() || deliverable.description.trim().is_empty() {
            return Err(ProjectError::InvalidDeliverable(deliverable.id.clone()));
        }
    }
    Ok(())
}

fn validate_evidence_refs(evidence: &BTreeSet<ProjectEvidenceRef>) -> Result<(), ProjectError> {
    for reference in evidence {
        if let ProjectEvidenceRef::External { namespace, .. } = reference
            && namespace.trim().is_empty()
        {
            return Err(ProjectError::EmptyExternalEvidenceNamespace);
        }
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProjectError {
    DuplicateProject(StableId),
    UnknownProject(StableId),
    EmptyProjectTitle(StableId),
    InvalidProjectWindow {
        project_id: StableId,
        opens_tick: u64,
        closes_tick: Option<u64>,
    },
    ProjectHasNoDeliverables(StableId),
    DeliverableKeyMismatch {
        project_id: StableId,
        key: StableId,
        actual: StableId,
    },
    InvalidDeliverable(StableId),
    DuplicateContribution(StableId),
    ProjectClosedAtSubmission {
        project_id: StableId,
        submitted_tick: u64,
    },
    EmptyContribution(StableId),
    UnknownDeliverable {
        project_id: StableId,
        deliverable_id: StableId,
    },
    DuplicateReview(StableId),
    UnknownContribution(StableId),
    ReviewerNotAuthorized {
        project_id: StableId,
        reviewer_id: StableId,
    },
    ReviewBeforeSubmission {
        contribution_id: StableId,
        submitted_tick: u64,
        reviewed_tick: u64,
    },
    EmptyReview(StableId),
    ReviewOfUnclaimedDeliverable {
        contribution_id: StableId,
        deliverable_id: StableId,
    },
    EmptyExternalEvidenceNamespace,
}

impl fmt::Display for ProjectError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateProject(id) => write!(formatter, "project {id} already exists"),
            Self::UnknownProject(id) => write!(formatter, "unknown project {id}"),
            Self::EmptyProjectTitle(id) => write!(formatter, "project {id} has an empty title"),
            Self::InvalidProjectWindow { project_id, opens_tick, closes_tick } => write!(
                formatter,
                "project {project_id} has invalid window {opens_tick}..={closes_tick:?}"
            ),
            Self::ProjectHasNoDeliverables(id) => write!(formatter, "project {id} has no deliverables"),
            Self::DeliverableKeyMismatch { project_id, key, actual } => write!(
                formatter,
                "project {project_id} deliverable map key {key} does not match {actual}"
            ),
            Self::InvalidDeliverable(id) => write!(formatter, "deliverable {id} is invalid"),
            Self::DuplicateContribution(id) => write!(formatter, "contribution {id} already exists"),
            Self::ProjectClosedAtSubmission { project_id, submitted_tick } => write!(
                formatter,
                "project {project_id} is not open at contribution tick {submitted_tick}"
            ),
            Self::EmptyContribution(id) => write!(formatter, "contribution {id} claims no deliverables"),
            Self::UnknownDeliverable { project_id, deliverable_id } => write!(
                formatter,
                "project {project_id} has no deliverable {deliverable_id}"
            ),
            Self::DuplicateReview(id) => write!(formatter, "review {id} already exists"),
            Self::UnknownContribution(id) => write!(formatter, "unknown contribution {id}"),
            Self::ReviewerNotAuthorized { project_id, reviewer_id } => write!(
                formatter,
                "reviewer {reviewer_id} is not authorized for project {project_id}"
            ),
            Self::ReviewBeforeSubmission { contribution_id, submitted_tick, reviewed_tick } => write!(
                formatter,
                "contribution {contribution_id} submitted at {submitted_tick} cannot be reviewed at earlier tick {reviewed_tick}"
            ),
            Self::EmptyReview(id) => write!(formatter, "review {id} has no deliverable decisions"),
            Self::ReviewOfUnclaimedDeliverable { contribution_id, deliverable_id } => write!(
                formatter,
                "review for contribution {contribution_id} references unclaimed deliverable {deliverable_id}"
            ),
            Self::EmptyExternalEvidenceNamespace => formatter.write_str("external project evidence namespace is empty"),
        }
    }
}

impl Error for ProjectError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(value: &str) -> StableId {
        StableId::parse(value).expect("test id is valid")
    }

    fn deliverable(value: &str, required: bool) -> DeliverableSpec {
        DeliverableSpec {
            id: id(value),
            kind: "cargo-delivery".into(),
            description: format!("satisfy {value}"),
            required,
        }
    }

    fn project() -> ProjectSpec {
        let food = deliverable("deliverable:food", true);
        let medicine = deliverable("deliverable:medicine", true);
        ProjectSpec {
            id: id("project:relief"),
            issuer_id: id("institution:relief-office"),
            title: "Resupply Station Twelve".into(),
            opens_tick: 10,
            closes_tick: Some(100),
            deliverables: BTreeMap::from([
                (food.id.clone(), food),
                (medicine.id.clone(), medicine),
            ]),
            reviewer_ids: BTreeSet::from([id("actor:quartermaster")]),
            source_event_id: id("event:project-published"),
        }
    }

    #[test]
    fn unrelated_contributors_can_jointly_complete_project() {
        let mut ledger = ProjectLedger::new();
        ledger.publish_project(project()).expect("project");
        for (cid, contributor, did, evidence) in [
            (
                "contribution:food",
                "actor:independent-hauler",
                "deliverable:food",
                ProjectEvidenceRef::EconomyShipmentArrival(id("arrival:food")),
            ),
            (
                "contribution:medicine",
                "agent:autonomous-logistics",
                "deliverable:medicine",
                ProjectEvidenceRef::EconomyShipmentArrival(id("arrival:medicine")),
            ),
        ] {
            ledger
                .submit(Contribution {
                    id: id(cid),
                    project_id: id("project:relief"),
                    contributor_id: id(contributor),
                    deliverable_ids: BTreeSet::from([id(did)]),
                    evidence: BTreeSet::from([evidence]),
                    submitted_tick: 30,
                    source_event_id: id(&format!("event:{cid}")),
                })
                .expect("contribution");
            ledger
                .review(ContributionReview {
                    id: id(&format!("review:{cid}")),
                    contribution_id: id(cid),
                    reviewer_id: id("actor:quartermaster"),
                    dispositions: BTreeMap::from([(id(did), DeliverableDisposition::Accepted)]),
                    review_evidence: BTreeSet::new(),
                    reviewed_tick: 31,
                    source_event_id: id(&format!("event:review:{cid}")),
                })
                .expect("review");
        }

        let completion = ledger.completion(&id("project:relief")).expect("completion");
        assert!(completion.is_complete());
        assert_eq!(completion.accepted_required.len(), 2);
    }

    #[test]
    fn submission_is_not_acceptance() {
        let mut ledger = ProjectLedger::new();
        ledger.publish_project(project()).expect("project");
        ledger
            .submit(Contribution {
                id: id("contribution:food"),
                project_id: id("project:relief"),
                contributor_id: id("actor:hauler"),
                deliverable_ids: BTreeSet::from([id("deliverable:food")]),
                evidence: BTreeSet::from([ProjectEvidenceRef::EconomyShipmentArrival(id("arrival:food"))]),
                submitted_tick: 20,
                source_event_id: id("event:submit"),
            })
            .expect("submit");
        let completion = ledger.completion(&id("project:relief")).expect("completion");
        assert!(completion.accepted_required.is_empty());
        assert_eq!(completion.missing_required.len(), 2);
    }

    #[test]
    fn unauthorized_reviewer_cannot_accept_work() {
        let mut ledger = ProjectLedger::new();
        ledger.publish_project(project()).expect("project");
        ledger
            .submit(Contribution {
                id: id("contribution:food"),
                project_id: id("project:relief"),
                contributor_id: id("actor:hauler"),
                deliverable_ids: BTreeSet::from([id("deliverable:food")]),
                evidence: BTreeSet::new(),
                submitted_tick: 20,
                source_event_id: id("event:submit"),
            })
            .expect("submit");
        let result = ledger.review(ContributionReview {
            id: id("review:bad"),
            contribution_id: id("contribution:food"),
            reviewer_id: id("actor:random"),
            dispositions: BTreeMap::from([(
                id("deliverable:food"),
                DeliverableDisposition::Accepted,
            )]),
            review_evidence: BTreeSet::new(),
            reviewed_tick: 21,
            source_event_id: id("event:bad-review"),
        });
        assert!(matches!(
            result,
            Err(ProjectError::ReviewerNotAuthorized { .. })
        ));
        assert!(ledger.completion(&id("project:relief")).expect("completion").accepted_required.is_empty());
    }

    #[test]
    fn review_cannot_accept_unclaimed_deliverable() {
        let mut ledger = ProjectLedger::new();
        ledger.publish_project(project()).expect("project");
        ledger
            .submit(Contribution {
                id: id("contribution:food"),
                project_id: id("project:relief"),
                contributor_id: id("actor:hauler"),
                deliverable_ids: BTreeSet::from([id("deliverable:food")]),
                evidence: BTreeSet::new(),
                submitted_tick: 20,
                source_event_id: id("event:submit"),
            })
            .expect("submit");
        assert!(matches!(
            ledger.review(ContributionReview {
                id: id("review:wrong"),
                contribution_id: id("contribution:food"),
                reviewer_id: id("actor:quartermaster"),
                dispositions: BTreeMap::from([(
                    id("deliverable:medicine"),
                    DeliverableDisposition::Accepted,
                )]),
                review_evidence: BTreeSet::new(),
                reviewed_tick: 21,
                source_event_id: id("event:wrong-review"),
            }),
            Err(ProjectError::ReviewOfUnclaimedDeliverable { .. })
        ));
    }

    #[test]
    fn contribution_after_close_rejects() {
        let mut ledger = ProjectLedger::new();
        ledger.publish_project(project()).expect("project");
        assert!(matches!(
            ledger.submit(Contribution {
                id: id("contribution:late"),
                project_id: id("project:relief"),
                contributor_id: id("actor:late"),
                deliverable_ids: BTreeSet::from([id("deliverable:food")]),
                evidence: BTreeSet::new(),
                submitted_tick: 101,
                source_event_id: id("event:late"),
            }),
            Err(ProjectError::ProjectClosedAtSubmission { .. })
        ));
    }
}
