//! Reusable knowledge-indexed replay primitives for historical decision evaluation.
//!
//! This module selects immutable evidence vintages using only timestamps and
//! dispositions available at a decision cutoff. It is not an authentication,
//! rights-granting, authorization, or production evidence-store implementation.

use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Gate {
    Yes,
    No,
    Indeterminate,
    NotApplicable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RightsState {
    Granted,
    Denied,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AuthorityState {
    Valid,
    Revoked,
    Conflict,
    Unknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvidenceState {
    Observed,
    Censored,
    Conflict,
    Duplicate,
    Missing,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClockQuality {
    Trusted,
    Uncertain,
}

/// An immutable source vintage. Corrections append a new revision and point to
/// the exact predecessor; they never overwrite an earlier version.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvidenceVintage {
    pub logical_id: String,
    pub vintage_id: String,
    pub revision: u64,
    pub supersedes_vintage_id: Option<String>,
    pub event_time: u64,
    pub source_observed_at: u64,
    pub first_available_at: u64,
    pub ingested_at: u64,
    pub validated_at: u64,
    pub valid_until: Option<u64>,
    pub rights: RightsState,
    pub authority: AuthorityState,
    pub evidence_state: EvidenceState,
    pub clock_quality: ClockQuality,
    pub feature_uses_future_data: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum VintageDisposition {
    Selected,
    NotYetAvailable,
    NotYetIngested,
    NotYetValidated,
    SupersededAsOfCutoff,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvidenceAssessment {
    pub logical_id: String,
    pub vintage_id: String,
    pub revision: u64,
    pub visible: Gate,
    pub admissible: Gate,
    pub disposition: VintageDisposition,
    pub reason: &'static str,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AsOfSnapshot {
    pub decision_cutoff: u64,
    pub selected: BTreeMap<String, EvidenceAssessment>,
    pub withheld: Vec<EvidenceAssessment>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionCandidate {
    pub action_id: String,
    pub required_evidence: Vec<String>,
    pub plan_published_at: Option<u64>,
    pub authorized_at: Option<u64>,
    pub action_started_at: Option<u64>,
    pub service_deadline: Option<u64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActionAssessment {
    pub action_id: String,
    pub executable_at_cutoff: Gate,
    pub reason: &'static str,
}

/// Validate a complete, unambiguous, append-only revision lineage.
pub fn validate_vintages(vintages: &[EvidenceVintage]) -> Result<(), String> {
    let mut vintage_ids = BTreeSet::new();
    let mut by_logical: BTreeMap<&str, BTreeMap<u64, &EvidenceVintage>> = BTreeMap::new();

    for vintage in vintages {
        if vintage.logical_id.trim().is_empty() || vintage.vintage_id.trim().is_empty() {
            return Err("logical_id and vintage_id must be non-empty".to_owned());
        }
        if vintage.revision == 0 {
            return Err(format!("{}: revision must be positive", vintage.vintage_id));
        }
        if !vintage_ids.insert(vintage.vintage_id.as_str()) {
            return Err(format!("duplicate vintage_id '{}'", vintage.vintage_id));
        }
        if vintage.source_observed_at > vintage.first_available_at
            || vintage.first_available_at > vintage.ingested_at
            || vintage.ingested_at > vintage.validated_at
        {
            return Err(format!(
                "{}: evidence clock order is inconsistent",
                vintage.vintage_id
            ));
        }
        if vintage
            .valid_until
            .is_some_and(|expiry| expiry < vintage.first_available_at)
        {
            return Err(format!(
                "{}: validity ends before first availability",
                vintage.vintage_id
            ));
        }
        let revisions = by_logical.entry(&vintage.logical_id).or_default();
        if revisions.insert(vintage.revision, vintage).is_some() {
            return Err(format!(
                "{}: duplicate logical revision",
                vintage.vintage_id
            ));
        }
    }

    for (logical_id, revisions) in by_logical {
        let mut previous: Option<&EvidenceVintage> = None;
        for number in 1..=revisions.len() as u64 {
            let current = revisions.get(&number).ok_or_else(|| {
                format!("{logical_id}: missing revision {number} in complete lineage")
            })?;
            match previous {
                None if current.supersedes_vintage_id.is_some() => {
                    return Err(format!(
                        "{}: revision 1 has a predecessor",
                        current.vintage_id
                    ));
                }
                None => {}
                Some(prior) => {
                    if current.supersedes_vintage_id.as_deref() != Some(prior.vintage_id.as_str()) {
                        return Err(format!(
                            "{}: revision {} must supersede '{}'",
                            current.vintage_id, number, prior.vintage_id
                        ));
                    }
                    if current.first_available_at <= prior.first_available_at {
                        return Err(format!(
                            "{}: correction must become available after its predecessor",
                            current.vintage_id
                        ));
                    }
                }
            }
            previous = Some(current);
        }
    }
    Ok(())
}

/// Assess visibility and admissibility without consulting event time for visibility.
pub fn assess_vintage(vintage: &EvidenceVintage, cutoff: u64) -> EvidenceAssessment {
    let (visible, admissible, reason) = if vintage.clock_quality == ClockQuality::Uncertain {
        (
            Gate::Indeterminate,
            Gate::Indeterminate,
            "clock ordering is uncertain",
        )
    } else if vintage.first_available_at > cutoff {
        (Gate::No, Gate::No, "vintage was not yet available")
    } else if vintage.ingested_at > cutoff {
        (Gate::Yes, Gate::No, "vintage was not yet ingested")
    } else if vintage.validated_at > cutoff {
        (Gate::Yes, Gate::No, "vintage was not yet validated")
    } else if vintage.valid_until.is_some_and(|expiry| expiry <= cutoff) {
        (
            Gate::Yes,
            Gate::No,
            "vintage expired at the decision cutoff",
        )
    } else if vintage.rights == RightsState::Denied {
        (Gate::Yes, Gate::No, "rights were not granted")
    } else if vintage.rights == RightsState::Unknown {
        (Gate::Yes, Gate::Indeterminate, "rights state is unknown")
    } else if vintage.authority == AuthorityState::Revoked {
        (Gate::Yes, Gate::No, "authority was revoked")
    } else if matches!(
        vintage.authority,
        AuthorityState::Conflict | AuthorityState::Unknown
    ) {
        (
            Gate::Yes,
            Gate::Indeterminate,
            "authority state is unresolved",
        )
    } else if vintage.feature_uses_future_data {
        (
            Gate::Yes,
            Gate::No,
            "feature provenance uses post-cutoff information",
        )
    } else {
        match vintage.evidence_state {
            EvidenceState::Observed => (Gate::Yes, Gate::Yes, "observed evidence is admissible"),
            EvidenceState::Censored => (Gate::Yes, Gate::Indeterminate, "outcome is censored"),
            EvidenceState::Conflict => (Gate::Yes, Gate::Indeterminate, "evidence is conflicted"),
            EvidenceState::Duplicate => (
                Gate::Yes,
                Gate::NotApplicable,
                "duplicate contributes no additional evidence",
            ),
            EvidenceState::Missing => (Gate::Yes, Gate::Indeterminate, "evidence is missing"),
        }
    };

    EvidenceAssessment {
        logical_id: vintage.logical_id.clone(),
        vintage_id: vintage.vintage_id.clone(),
        revision: vintage.revision,
        visible,
        admissible,
        disposition: VintageDisposition::Selected,
        reason,
    }
}

/// Select the newest fully available and validated revision for each logical ID.
/// Later corrections are reported as withheld and cannot rewrite earlier snapshots.
pub fn select_as_of(vintages: &[EvidenceVintage], cutoff: u64) -> Result<AsOfSnapshot, String> {
    validate_vintages(vintages)?;
    let mut grouped: BTreeMap<&str, Vec<&EvidenceVintage>> = BTreeMap::new();
    for vintage in vintages {
        grouped
            .entry(&vintage.logical_id)
            .or_default()
            .push(vintage);
    }

    let mut selected = BTreeMap::new();
    let mut withheld = Vec::new();

    for (logical_id, mut revisions) in grouped {
        revisions.sort_by_key(|vintage| vintage.revision);
        let selected_revision = revisions
            .iter()
            .rev()
            .find(|vintage| {
                vintage.first_available_at <= cutoff
                    && vintage.ingested_at <= cutoff
                    && vintage.validated_at <= cutoff
            })
            .map(|vintage| vintage.revision);

        for vintage in revisions {
            if selected_revision == Some(vintage.revision) {
                selected.insert(logical_id.to_owned(), assess_vintage(vintage, cutoff));
            } else {
                let mut assessment = assess_vintage(vintage, cutoff);
                assessment.disposition = if vintage.first_available_at > cutoff {
                    VintageDisposition::NotYetAvailable
                } else if vintage.ingested_at > cutoff {
                    VintageDisposition::NotYetIngested
                } else if vintage.validated_at > cutoff {
                    VintageDisposition::NotYetValidated
                } else {
                    VintageDisposition::SupersededAsOfCutoff
                };
                withheld.push(assessment);
            }
        }
    }

    withheld.sort_by(|left, right| {
        (&left.logical_id, left.revision).cmp(&(&right.logical_id, right.revision))
    });
    Ok(AsOfSnapshot {
        decision_cutoff: cutoff,
        selected,
        withheld,
    })
}

/// Check as-of evidence, plan publication and authority timing for one candidate action.
/// This is a benchmark gate, not a substitute for owner-side authorization.
pub fn assess_action(
    snapshot: &AsOfSnapshot,
    action: &ActionCandidate,
) -> Result<ActionAssessment, String> {
    if action.action_id.trim().is_empty() {
        return Err("action_id must be non-empty".to_owned());
    }
    if action
        .required_evidence
        .iter()
        .any(|key| key.trim().is_empty())
    {
        return Err(format!(
            "{}: required evidence IDs must be non-empty",
            action.action_id
        ));
    }
    if action.required_evidence.is_empty() {
        return Ok(ActionAssessment {
            action_id: action.action_id.clone(),
            executable_at_cutoff: Gate::Indeterminate,
            reason: "action has no declared required evidence",
        });
    }

    let mut indeterminate = false;
    for key in &action.required_evidence {
        match snapshot.selected.get(key) {
            None => {
                return Ok(ActionAssessment {
                    action_id: action.action_id.clone(),
                    executable_at_cutoff: Gate::No,
                    reason: "required evidence is absent from the as-of snapshot",
                });
            }
            Some(item) if item.admissible == Gate::No => {
                return Ok(ActionAssessment {
                    action_id: action.action_id.clone(),
                    executable_at_cutoff: Gate::No,
                    reason: "required evidence is inadmissible",
                });
            }
            Some(item) if item.admissible == Gate::Indeterminate => indeterminate = true,
            Some(item) if item.admissible == Gate::NotApplicable => {
                return Ok(ActionAssessment {
                    action_id: action.action_id.clone(),
                    executable_at_cutoff: Gate::No,
                    reason: "required evidence is duplicate or not applicable",
                });
            }
            Some(_) => {}
        }
    }
    if indeterminate {
        return Ok(ActionAssessment {
            action_id: action.action_id.clone(),
            executable_at_cutoff: Gate::Indeterminate,
            reason: "one or more required evidence items are indeterminate",
        });
    }

    let (Some(published), Some(authorized), Some(deadline)) = (
        action.plan_published_at,
        action.authorized_at,
        action.service_deadline,
    ) else {
        return Ok(ActionAssessment {
            action_id: action.action_id.clone(),
            executable_at_cutoff: Gate::No,
            reason: "plan publication, authorization, or deadline is missing",
        });
    };
    if published > snapshot.decision_cutoff || authorized > snapshot.decision_cutoff {
        return Ok(ActionAssessment {
            action_id: action.action_id.clone(),
            executable_at_cutoff: Gate::No,
            reason: "plan or authorization was not available by the decision cutoff",
        });
    }
    if authorized < published {
        return Err(format!(
            "{}: authorization precedes plan publication",
            action.action_id
        ));
    }
    if published > deadline {
        return Ok(ActionAssessment {
            action_id: action.action_id.clone(),
            executable_at_cutoff: Gate::No,
            reason: "plan was published after the service deadline",
        });
    }
    if action
        .action_started_at
        .is_some_and(|started| started < authorized)
    {
        return Err(format!(
            "{}: action start precedes authorization",
            action.action_id
        ));
    }
    Ok(ActionAssessment {
        action_id: action.action_id.clone(),
        executable_at_cutoff: Gate::Yes,
        reason: "required evidence, publication, and authorization gates pass",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vintage(
        logical_id: &str,
        vintage_id: &str,
        revision: u64,
        supersedes: Option<&str>,
        first_available_at: u64,
        authority: AuthorityState,
    ) -> EvidenceVintage {
        EvidenceVintage {
            logical_id: logical_id.to_owned(),
            vintage_id: vintage_id.to_owned(),
            revision,
            supersedes_vintage_id: supersedes.map(str::to_owned),
            event_time: 40,
            source_observed_at: first_available_at.saturating_sub(1),
            first_available_at,
            ingested_at: first_available_at + 1,
            validated_at: first_available_at + 2,
            valid_until: None,
            rights: RightsState::Granted,
            authority,
            evidence_state: EvidenceState::Observed,
            clock_quality: ClockQuality::Trusted,
            feature_uses_future_data: false,
        }
    }

    #[test]
    fn late_correction_does_not_rewrite_an_earlier_snapshot() {
        let original = vintage("OUTAGE", "OUTAGE-V1", 1, None, 60, AuthorityState::Valid);
        let correction = vintage(
            "OUTAGE",
            "OUTAGE-V2",
            2,
            Some("OUTAGE-V1"),
            106,
            AuthorityState::Revoked,
        );
        let records = vec![original, correction];
        let historical = select_as_of(&records, 100).expect("historical snapshot");
        assert_eq!(historical.selected["OUTAGE"].vintage_id, "OUTAGE-V1");
        assert_eq!(historical.selected["OUTAGE"].admissible, Gate::Yes);
        assert_eq!(
            historical.withheld[0].disposition,
            VintageDisposition::NotYetAvailable
        );

        let current = select_as_of(&records, 120).expect("current snapshot");
        assert_eq!(current.selected["OUTAGE"].vintage_id, "OUTAGE-V2");
        assert_eq!(current.selected["OUTAGE"].admissible, Gate::No);
    }

    #[test]
    fn event_time_does_not_make_a_backfilled_vintage_visible() {
        let mut item = vintage(
            "BACKFILL",
            "BACKFILL-V1",
            1,
            None,
            116,
            AuthorityState::Valid,
        );
        item.event_time = 40;
        let snapshot = select_as_of(&[item], 100).expect("snapshot");
        assert!(!snapshot.selected.contains_key("BACKFILL"));
        assert_eq!(
            snapshot.withheld[0].disposition,
            VintageDisposition::NotYetAvailable
        );
    }

    #[test]
    fn ingestion_and_validation_are_independent_gates() {
        let mut late_ingestion = vintage("INGEST", "INGEST-V1", 1, None, 92, AuthorityState::Valid);
        late_ingestion.ingested_at = 105;
        late_ingestion.validated_at = 106;
        let snapshot = select_as_of(&[late_ingestion], 100).expect("snapshot");
        assert_eq!(
            snapshot.withheld[0].disposition,
            VintageDisposition::NotYetIngested
        );

        let mut late_validation = vintage(
            "VALIDATE",
            "VALIDATE-V1",
            1,
            None,
            92,
            AuthorityState::Valid,
        );
        late_validation.ingested_at = 94;
        late_validation.validated_at = 105;
        let snapshot = select_as_of(&[late_validation], 100).expect("snapshot");
        assert_eq!(
            snapshot.withheld[0].disposition,
            VintageDisposition::NotYetValidated
        );
    }

    #[test]
    fn uncertain_clocks_censored_and_conflicting_evidence_never_pass() {
        let mut uncertain = vintage("CLOCK", "CLOCK-V1", 1, None, 92, AuthorityState::Valid);
        uncertain.clock_quality = ClockQuality::Uncertain;
        let mut censored = vintage(
            "CENSORED",
            "CENSORED-V1",
            1,
            None,
            92,
            AuthorityState::Valid,
        );
        censored.evidence_state = EvidenceState::Censored;
        let mut conflicted = vintage(
            "CONFLICT",
            "CONFLICT-V1",
            1,
            None,
            92,
            AuthorityState::Conflict,
        );
        conflicted.evidence_state = EvidenceState::Conflict;
        let snapshot = select_as_of(&[uncertain, censored, conflicted], 100).expect("snapshot");
        for key in ["CLOCK", "CENSORED", "CONFLICT"] {
            assert_eq!(snapshot.selected[key].admissible, Gate::Indeterminate);
        }
    }

    #[test]
    fn duplicate_evidence_does_not_contribute_a_second_fact() {
        let mut duplicate = vintage(
            "DUPLICATE",
            "DUPLICATE-V1",
            1,
            None,
            92,
            AuthorityState::Valid,
        );
        duplicate.evidence_state = EvidenceState::Duplicate;
        let snapshot = select_as_of(&[duplicate], 100).expect("snapshot");
        assert_eq!(
            snapshot.selected["DUPLICATE"].admissible,
            Gate::NotApplicable
        );
    }

    #[test]
    fn future_derived_feature_is_rejected_even_when_visible() {
        let mut feature = vintage("FEATURE", "FEATURE-V1", 1, None, 92, AuthorityState::Valid);
        feature.feature_uses_future_data = true;
        feature.event_time = 120;
        let snapshot = select_as_of(&[feature], 100).expect("snapshot");
        assert_eq!(snapshot.selected["FEATURE"].visible, Gate::Yes);
        assert_eq!(snapshot.selected["FEATURE"].admissible, Gate::No);
    }

    #[test]
    fn action_needs_evidence_publication_and_authorization_by_cutoff() {
        let record = vintage(
            "CAPABILITY",
            "CAPABILITY-V1",
            1,
            None,
            90,
            AuthorityState::Valid,
        );
        let snapshot = select_as_of(&[record], 100).expect("snapshot");
        let action = ActionCandidate {
            action_id: "RECOVER".to_owned(),
            required_evidence: vec!["CAPABILITY".to_owned()],
            plan_published_at: Some(94),
            authorized_at: Some(98),
            action_started_at: Some(101),
            service_deadline: Some(120),
        };
        assert_eq!(
            assess_action(&snapshot, &action)
                .expect("assessment")
                .executable_at_cutoff,
            Gate::Yes
        );

        let mut late = action.clone();
        late.authorized_at = Some(101);
        assert_eq!(
            assess_action(&snapshot, &late)
                .expect("late assessment")
                .executable_at_cutoff,
            Gate::No
        );

        let mut missing = action.clone();
        missing.required_evidence.push("MISSING".to_owned());
        assert_eq!(
            assess_action(&snapshot, &missing)
                .expect("missing assessment")
                .executable_at_cutoff,
            Gate::No
        );
    }

    #[test]
    fn incomplete_or_wrong_revision_lineage_is_rejected() {
        let first = vintage("EVENT", "EVENT-V1", 1, None, 10, AuthorityState::Valid);
        let skipped = vintage(
            "EVENT",
            "EVENT-V3",
            3,
            Some("EVENT-V2"),
            30,
            AuthorityState::Valid,
        );
        assert!(validate_vintages(&[first.clone(), skipped]).is_err());

        let wrong = vintage(
            "EVENT",
            "EVENT-V2",
            2,
            Some("WRONG"),
            30,
            AuthorityState::Valid,
        );
        assert!(validate_vintages(&[first, wrong]).is_err());
    }

    #[test]
    fn appending_a_future_vintage_preserves_the_historical_snapshot() {
        let first = vintage("EVENT", "EVENT-V1", 1, None, 20, AuthorityState::Valid);
        let before = select_as_of(std::slice::from_ref(&first), 50).expect("snapshot");
        let second = vintage(
            "EVENT",
            "EVENT-V2",
            2,
            Some("EVENT-V1"),
            80,
            AuthorityState::Revoked,
        );
        let after = select_as_of(&[first, second], 50).expect("snapshot after append");
        assert_eq!(before, after);
    }

    #[test]
    fn expiry_and_rights_are_checked_at_the_decision_cutoff() {
        let mut expired = vintage("EXPIRED", "EXPIRED-V1", 1, None, 60, AuthorityState::Valid);
        expired.valid_until = Some(90);
        let mut denied = vintage("DENIED", "DENIED-V1", 1, None, 60, AuthorityState::Valid);
        denied.rights = RightsState::Denied;
        let snapshot = select_as_of(&[expired, denied], 100).expect("snapshot");
        assert_eq!(snapshot.selected["EXPIRED"].admissible, Gate::No);
        assert_eq!(snapshot.selected["DENIED"].admissible, Gate::No);
    }
}
