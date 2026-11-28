#![forbid(unsafe_code)]

//! Compiles evidence lifecycle timing into actor-specific mobility observations.
//! Physical capacity is compiled independently and continues to constrain every
//! scenario from its effective period, even before the actor learns of the fact.

use crate::capacity_admission::{CapacityEvidenceRecord, CapacityRole};
use crate::mobility_capacity::{
    MobilityCapacityEnvelopeInput, MobilityCapacityEnvelopeReport,
    compile_mobility_capacity_envelope, verify_mobility_capacity_envelope,
};
use crate::mobility_oracle::{
    MobilityInstance, ObservationEvent, mobility_instance_digest, validate_mobility_instance,
    validate_mobility_instance_for_verification,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const MOBILITY_EVIDENCE_TIME_INPUT_SCHEMA: &str = "safrb-mobility-evidence-time-input-v0.1";
pub const MOBILITY_EVIDENCE_TIME_REPORT_SCHEMA: &str = "safrb-mobility-evidence-time-report-v0.1";
pub const MOBILITY_EVIDENCE_TIME_CLAIM_CEILING: &str = "Structurally derives actor-scoped observation visibility from externally supplied observation, admission, eligibility and revocation timing. Physical route capacities remain independently constrained by the scenario model. This does not authenticate actors, prove legal authority, establish source truth, prove that external timestamps are accurate, or authorize operational dispatch.";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceKnowledgeTiming {
    pub scenario_id: String,
    pub evidence_id: String,
    pub actor_ref: String,
    /// When the underlying fact was observed by an identified source.
    pub observed_at_period: u64,
    /// When the evidence record completed admission/verification.
    pub admitted_at_period: u64,
    /// Earliest period the specified actor is externally declared eligible to use it.
    pub actor_eligible_at_period: u64,
    /// When the evidence is no longer authoritative, independent of when the
    /// actor receives the revocation notice.
    pub revoked_effective_at_period: Option<u64>,
    /// When the actor receives the revocation observation.
    pub revocation_observed_at_period: Option<u64>,
    /// When the revocation notice completes admission/verification.
    pub revocation_admitted_at_period: Option<u64>,
    /// Earliest period this actor is externally declared eligible to use the
    /// revocation notice.
    pub revocation_actor_eligible_at_period: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MobilityEvidenceTimeInput {
    pub schema_version: String,
    pub actor_ref: String,
    pub capacity_envelope: MobilityCapacityEnvelopeInput,
    /// Exactly one lifecycle timing row for every source capacity evidence row.
    pub evidence_timings: Vec<EvidenceKnowledgeTiming>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RevocationNoticeGap {
    pub scenario_id: String,
    pub evidence_id: String,
    pub actor_ref: String,
    pub revoked_effective_at_period: u64,
    pub notice_eligible_at_period: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MobilityEvidenceTimeReport {
    pub schema_version: String,
    pub case_id: String,
    pub actor_ref: String,
    pub source_capacity_report: MobilityCapacityEnvelopeReport,
    pub evidence_timings_sha256: String,
    /// Intervals where source authority has already revoked a fact but the
    /// actor has not yet received an eligible revocation notice.
    pub revocation_notice_gaps: Vec<RevocationNoticeGap>,
    pub mobility_instance_sha256: String,
    pub mobility_instance: MobilityInstance,
    pub claim_ceiling: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
struct CanonicalCapacityFact<'a> {
    role: CapacityRole,
    subject_id: &'a str,
    canonical_resource_ref: &'a str,
    effective_from_period: u64,
    effective_until_period_exclusive: u64,
    fresh_until_period_exclusive: u64,
    capacity: u64,
    unit_ref: &'a str,
    quantity_boundary_ref: &'a str,
    timebase_ref: &'a str,
    evidence_class: crate::capacity_admission::CapacityEvidenceClass,
    admission_status: crate::capacity_admission::CapacityAdmissionStatus,
    issuer_ref: &'a str,
    authority_ref: &'a str,
    source_bytes_sha256: &'a str,
    admission_receipt_sha256: &'a str,
}

fn role_key(role: CapacityRole) -> &'static str {
    match role {
        CapacityRole::Storage => "storage",
        CapacityRole::Handling => "handling",
        CapacityRole::ServiceIssue => "service_issue",
        CapacityRole::Route => "route",
    }
}

fn domain_digest(domain: &[u8], bytes: &[u8]) -> String {
    let mut input = Vec::with_capacity(domain.len() + 8 + bytes.len());
    input.extend_from_slice(domain);
    input.extend_from_slice(&(bytes.len() as u64).to_be_bytes());
    input.extend_from_slice(bytes);
    format!("sha256:{:x}", Sha256::digest(input))
}

fn canonical_capacity_fact(record: &CapacityEvidenceRecord) -> CanonicalCapacityFact<'_> {
    CanonicalCapacityFact {
        role: record.role,
        subject_id: &record.subject_id,
        canonical_resource_ref: &record.canonical_resource_ref,
        effective_from_period: record.effective_from_period,
        effective_until_period_exclusive: record.effective_until_period_exclusive,
        fresh_until_period_exclusive: record.fresh_until_period_exclusive,
        capacity: record.capacity,
        unit_ref: &record.unit_ref,
        quantity_boundary_ref: &record.quantity_boundary_ref,
        timebase_ref: &record.timebase_ref,
        evidence_class: record.evidence_class,
        admission_status: record.admission_status,
        issuer_ref: &record.issuer_ref,
        authority_ref: &record.authority_ref,
        source_bytes_sha256: &record.source_bytes_sha256,
        admission_receipt_sha256: &record.admission_receipt_sha256,
    }
}

fn capacity_fact_digest(record: &CapacityEvidenceRecord) -> Result<String, String> {
    let bytes = serde_json::to_vec(&canonical_capacity_fact(record))
        .map_err(|error| format!("could not encode canonical evidence fact: {error}"))?;
    Ok(domain_digest(
        b"LUMINOUS-SAFRB-CAPACITY-KNOWLEDGE-FACT-V1\0",
        &bytes,
    ))
}

fn revocation_notice_gaps(
    timings: &[EvidenceKnowledgeTiming],
) -> Result<Vec<RevocationNoticeGap>, String> {
    let mut gaps = Vec::new();
    for timing in timings {
        if let Some(revoked_effective_at_period) = timing.revoked_effective_at_period {
            let notice_eligible_at_period = revocation_notice_period(timing)
                .ok_or_else(|| "revocation notice timeline became incomplete".to_string())?;
            if revoked_effective_at_period < notice_eligible_at_period {
                gaps.push(RevocationNoticeGap {
                    scenario_id: timing.scenario_id.clone(),
                    evidence_id: timing.evidence_id.clone(),
                    actor_ref: timing.actor_ref.clone(),
                    revoked_effective_at_period,
                    notice_eligible_at_period,
                });
            }
        }
    }
    gaps.sort_by(|left, right| {
        (&left.scenario_id, &left.evidence_id, &left.actor_ref).cmp(&(
            &right.scenario_id,
            &right.evidence_id,
            &right.actor_ref,
        ))
    });
    Ok(gaps)
}

fn timing_digest(timings: &[EvidenceKnowledgeTiming]) -> Result<String, String> {
    let mut rows = timings.to_vec();
    rows.sort_by(|left, right| {
        (&left.scenario_id, &left.evidence_id, &left.actor_ref).cmp(&(
            &right.scenario_id,
            &right.evidence_id,
            &right.actor_ref,
        ))
    });
    let bytes = serde_json::to_vec(&rows)
        .map_err(|error| format!("could not encode canonical evidence timings: {error}"))?;
    Ok(domain_digest(
        b"LUMINOUS-SAFRB-EVIDENCE-KNOWLEDGE-TIMINGS-V1\0",
        &bytes,
    ))
}

fn usable_from(timing: &EvidenceKnowledgeTiming) -> u64 {
    timing
        .observed_at_period
        .max(timing.admitted_at_period)
        .max(timing.actor_eligible_at_period)
}

fn revocation_notice_period(timing: &EvidenceKnowledgeTiming) -> Option<u64> {
    Some(
        timing
            .revocation_observed_at_period?
            .max(timing.revocation_admitted_at_period?)
            .max(timing.revocation_actor_eligible_at_period?),
    )
}

fn usable_until(
    record: &CapacityEvidenceRecord,
    timing: &EvidenceKnowledgeTiming,
    horizon_end: u64,
) -> u64 {
    record
        .effective_until_period_exclusive
        .min(record.fresh_until_period_exclusive)
        .min(revocation_notice_period(timing).unwrap_or(horizon_end))
}

fn validate_timing_inventory(
    input: &MobilityEvidenceTimeInput,
) -> Result<BTreeMap<(String, String), &EvidenceKnowledgeTiming>, String> {
    if input.schema_version != MOBILITY_EVIDENCE_TIME_INPUT_SCHEMA
        || input.actor_ref.trim().is_empty()
    {
        return Err("unsupported evidence-time schema or empty actor reference".to_string());
    }
    let horizon = input
        .capacity_envelope
        .mobility_instance
        .horizon_last_period;
    let horizon_end = horizon
        .checked_add(1)
        .ok_or_else(|| "evidence-time horizon end overflows u64".to_string())?;
    let expected = input
        .capacity_envelope
        .capacity_schedules
        .iter()
        .flat_map(|schedule| {
            schedule.records.iter().map(move |record| {
                (
                    (schedule.scenario_id.clone(), record.evidence_id.clone()),
                    record,
                )
            })
        })
        .collect::<BTreeMap<_, _>>();
    let expected_count = input
        .capacity_envelope
        .capacity_schedules
        .iter()
        .map(|schedule| schedule.records.len())
        .sum::<usize>();
    if expected.len() != expected_count {
        return Err("capacity evidence identity is duplicated within a scenario".to_string());
    }
    if input.evidence_timings.len() != expected.len() {
        return Err("evidence-time rows must cover every capacity record exactly once".to_string());
    }

    let mut result = BTreeMap::new();
    for timing in &input.evidence_timings {
        if timing.actor_ref != input.actor_ref
            || timing.scenario_id.trim().is_empty()
            || timing.evidence_id.trim().is_empty()
            || timing.observed_at_period > horizon
            || timing.admitted_at_period > horizon
            || timing.actor_eligible_at_period > horizon
        {
            return Err(
                "evidence-time row has an actor, identity or primary timestamp mismatch"
                    .to_string(),
            );
        }
        let key = (timing.scenario_id.clone(), timing.evidence_id.clone());
        if !expected.contains_key(&key) || result.insert(key, timing).is_some() {
            return Err("evidence-time row is unknown or duplicated".to_string());
        }
        match timing.revoked_effective_at_period {
            Some(effective_at) => {
                if effective_at > horizon_end
                    || timing.revocation_observed_at_period.is_none()
                    || timing.revocation_admitted_at_period.is_none()
                    || timing.revocation_actor_eligible_at_period.is_none()
                {
                    return Err(
                        "revocation requires an effective period and all three notice times"
                            .to_string(),
                    );
                }
                for period in [
                    timing.revocation_observed_at_period.unwrap(),
                    timing.revocation_admitted_at_period.unwrap(),
                    timing.revocation_actor_eligible_at_period.unwrap(),
                ] {
                    if period > horizon {
                        return Err(
                            "revocation notice timestamp is outside the horizon".to_string()
                        );
                    }
                }
            }
            None => {
                if timing.revocation_observed_at_period.is_some()
                    || timing.revocation_admitted_at_period.is_some()
                    || timing.revocation_actor_eligible_at_period.is_some()
                {
                    return Err(
                        "revocation notice times cannot exist without an effective revocation"
                            .to_string(),
                    );
                }
            }
        }
    }
    if result.len() != expected.len() {
        return Err("evidence-time inventory is incomplete".to_string());
    }
    Ok(result)
}

fn build_actor_observations(
    actor_ref: &str,
    scenario_id: &str,
    records: &[CapacityEvidenceRecord],
    timings: &BTreeMap<(String, String), &EvidenceKnowledgeTiming>,
    horizon_end: u64,
) -> Result<Vec<ObservationEvent>, String> {
    let mut events = Vec::new();
    for record in records {
        let timing = timings
            .get(&(scenario_id.to_string(), record.evidence_id.clone()))
            .ok_or_else(|| format!("missing actor timing for evidence {}", record.evidence_id))?;
        if timing.actor_ref != actor_ref {
            return Err("actor timing changed during observation compilation".to_string());
        }
        let fact_digest = capacity_fact_digest(record)?;
        let reveal_period = usable_from(timing);
        let valid_until = usable_until(record, timing, horizon_end);
        let already_revoked_before_actor_eligibility = timing
            .revoked_effective_at_period
            .is_some_and(|revoked_at| revoked_at <= reveal_period);
        if reveal_period < valid_until && !already_revoked_before_actor_eligibility {
            let key = format!(
                "capacity:{}:{}:{}:{}",
                role_key(record.role),
                record.subject_id,
                record.effective_from_period,
                record.effective_until_period_exclusive
            );
            events.push(ObservationEvent {
                reveal_period,
                valid_until_period_exclusive: Some(valid_until),
                event_id: format!("capacity-fact:{scenario_id}:{}", record.evidence_id),
                key,
                value: fact_digest.clone(),
            });
        }
        if let Some(revoked_effective_at) = timing.revoked_effective_at_period {
            let revocation_reveal = revocation_notice_period(timing)
                .ok_or_else(|| "revocation notice timeline became incomplete".to_string())?;
            if revocation_reveal <= horizon_end.saturating_sub(1) {
                events.push(ObservationEvent {
                    reveal_period: revocation_reveal,
                    valid_until_period_exclusive: None,
                    event_id: format!("capacity-revocation:{scenario_id}:{}", record.evidence_id),
                    key: format!(
                        "capacity-revocation:{}:{}:{}",
                        role_key(record.role),
                        record.subject_id,
                        record.effective_from_period
                    ),
                    value: format!(
                        "revoked-effective-at:{revoked_effective_at};fact:{fact_digest}"
                    ),
                });
            }
        }
    }
    Ok(events)
}

/// Compile physical capacities and a single actor's admissible observation
/// timeline. The physical model keeps its scenario capacities from their
/// effective periods, regardless of when the actor learns of them.
pub fn compile_mobility_evidence_time(
    input: &MobilityEvidenceTimeInput,
) -> Result<MobilityEvidenceTimeReport, String> {
    let timing_map = validate_timing_inventory(input)?;
    let capacity_report = compile_mobility_capacity_envelope(&input.capacity_envelope)?;
    let horizon_end = input
        .capacity_envelope
        .mobility_instance
        .horizon_last_period
        .checked_add(1)
        .ok_or_else(|| "evidence-time horizon end overflows u64".to_string())?;
    let schedule_by_scenario = input
        .capacity_envelope
        .capacity_schedules
        .iter()
        .map(|schedule| (schedule.scenario_id.as_str(), schedule))
        .collect::<BTreeMap<_, _>>();
    let mut mobility_instance = capacity_report.compiled_mobility_instance.clone();

    for scenario in &mut mobility_instance.scenarios {
        let schedule = schedule_by_scenario
            .get(scenario.scenario_id.as_str())
            .ok_or_else(|| "missing capacity schedule for evidence-time scenario".to_string())?;
        let mut events = build_actor_observations(
            &input.actor_ref,
            &scenario.scenario_id,
            &schedule.records,
            &timing_map,
            horizon_end,
        )?;
        events.sort_by(|left, right| {
            (&left.reveal_period, &left.key, &left.value, &left.event_id).cmp(&(
                &right.reveal_period,
                &right.key,
                &right.value,
                &right.event_id,
            ))
        });
        scenario.observations.extend(events);
    }

    validate_mobility_instance(&mobility_instance)?;
    let timings_sha256 = timing_digest(&input.evidence_timings)?;
    let mobility_sha256 = mobility_instance_digest(&mobility_instance)?;
    Ok(MobilityEvidenceTimeReport {
        schema_version: MOBILITY_EVIDENCE_TIME_REPORT_SCHEMA.to_string(),
        case_id: input.capacity_envelope.mobility_instance.case_id.clone(),
        actor_ref: input.actor_ref.clone(),
        source_capacity_report: capacity_report,
        evidence_timings_sha256: timings_sha256,
        revocation_notice_gaps: revocation_notice_gaps(&input.evidence_timings)?,
        mobility_instance_sha256: mobility_sha256,
        mobility_instance,
        claim_ceiling: MOBILITY_EVIDENCE_TIME_CLAIM_CEILING.to_string(),
    })
}

#[derive(Serialize)]
struct VerifierCapacityFact<'a> {
    role: CapacityRole,
    subject_id: &'a str,
    canonical_resource_ref: &'a str,
    effective_from_period: u64,
    effective_until_period_exclusive: u64,
    fresh_until_period_exclusive: u64,
    capacity: u64,
    unit_ref: &'a str,
    quantity_boundary_ref: &'a str,
    timebase_ref: &'a str,
    evidence_class: crate::capacity_admission::CapacityEvidenceClass,
    admission_status: crate::capacity_admission::CapacityAdmissionStatus,
    issuer_ref: &'a str,
    authority_ref: &'a str,
    source_bytes_sha256: &'a str,
    admission_receipt_sha256: &'a str,
}

fn verifier_role_key(role: CapacityRole) -> &'static str {
    match role {
        CapacityRole::Storage => "storage",
        CapacityRole::Handling => "handling",
        CapacityRole::ServiceIssue => "service_issue",
        CapacityRole::Route => "route",
    }
}

fn verifier_fact_digest(record: &CapacityEvidenceRecord) -> Result<String, String> {
    let fact = VerifierCapacityFact {
        role: record.role,
        subject_id: &record.subject_id,
        canonical_resource_ref: &record.canonical_resource_ref,
        effective_from_period: record.effective_from_period,
        effective_until_period_exclusive: record.effective_until_period_exclusive,
        fresh_until_period_exclusive: record.fresh_until_period_exclusive,
        capacity: record.capacity,
        unit_ref: &record.unit_ref,
        quantity_boundary_ref: &record.quantity_boundary_ref,
        timebase_ref: &record.timebase_ref,
        evidence_class: record.evidence_class,
        admission_status: record.admission_status,
        issuer_ref: &record.issuer_ref,
        authority_ref: &record.authority_ref,
        source_bytes_sha256: &record.source_bytes_sha256,
        admission_receipt_sha256: &record.admission_receipt_sha256,
    };
    let bytes = serde_json::to_vec(&fact)
        .map_err(|error| format!("independent verifier could not encode capacity fact: {error}"))?;
    Ok(domain_digest(
        b"LUMINOUS-SAFRB-CAPACITY-KNOWLEDGE-FACT-V1\0",
        &bytes,
    ))
}

fn verifier_timing_digest(timings: &[EvidenceKnowledgeTiming]) -> Result<String, String> {
    let mut rows = timings.to_vec();
    rows.sort_by(|a, b| {
        a.scenario_id
            .cmp(&b.scenario_id)
            .then(a.evidence_id.cmp(&b.evidence_id))
            .then(a.actor_ref.cmp(&b.actor_ref))
    });
    let bytes = serde_json::to_vec(&rows).map_err(|error| {
        format!("independent verifier could not encode evidence timing rows: {error}")
    })?;
    Ok(domain_digest(
        b"LUMINOUS-SAFRB-EVIDENCE-KNOWLEDGE-TIMINGS-V1\0",
        &bytes,
    ))
}

fn verify_timing_inventory(
    input: &MobilityEvidenceTimeInput,
) -> Result<BTreeMap<(String, String), &EvidenceKnowledgeTiming>, String> {
    if input.schema_version != MOBILITY_EVIDENCE_TIME_INPUT_SCHEMA
        || input.actor_ref.trim().is_empty()
        || input.evidence_timings.len()
            != input
                .capacity_envelope
                .capacity_schedules
                .iter()
                .map(|schedule| schedule.records.len())
                .sum::<usize>()
    {
        return Err(
            "independent verifier rejected evidence-time schema or inventory size".to_string(),
        );
    }
    let horizon = input
        .capacity_envelope
        .mobility_instance
        .horizon_last_period;
    let horizon_end = horizon
        .checked_add(1)
        .ok_or_else(|| "independent verifier horizon overflowed".to_string())?;
    let mut expected = BTreeSet::new();
    for schedule in &input.capacity_envelope.capacity_schedules {
        for record in &schedule.records {
            if !expected.insert((schedule.scenario_id.clone(), record.evidence_id.clone())) {
                return Err(
                    "independent verifier found duplicated source evidence identity".to_string(),
                );
            }
        }
    }
    let mut result = BTreeMap::new();
    for timing in &input.evidence_timings {
        if timing.actor_ref != input.actor_ref
            || timing.observed_at_period > horizon
            || timing.admitted_at_period > horizon
            || timing.actor_eligible_at_period > horizon
            || timing.scenario_id.trim().is_empty()
            || timing.evidence_id.trim().is_empty()
        {
            return Err(
                "independent verifier rejected actor or primary evidence timing".to_string(),
            );
        }
        let key = (timing.scenario_id.clone(), timing.evidence_id.clone());
        if !expected.contains(&key) || result.insert(key, timing).is_some() {
            return Err(
                "independent verifier found unexpected or duplicate timing identity".to_string(),
            );
        }
        match timing.revoked_effective_at_period {
            Some(period) => {
                if period > horizon_end
                    || timing.revocation_observed_at_period.is_none()
                    || timing.revocation_admitted_at_period.is_none()
                    || timing.revocation_actor_eligible_at_period.is_none()
                    || timing.revocation_observed_at_period.unwrap() > horizon
                    || timing.revocation_admitted_at_period.unwrap() > horizon
                    || timing.revocation_actor_eligible_at_period.unwrap() > horizon
                {
                    return Err(
                        "independent verifier rejected incomplete revocation timeline".to_string(),
                    );
                }
            }
            None => {
                if timing.revocation_observed_at_period.is_some()
                    || timing.revocation_admitted_at_period.is_some()
                    || timing.revocation_actor_eligible_at_period.is_some()
                {
                    return Err(
                        "independent verifier found revocation timestamps without revocation"
                            .to_string(),
                    );
                }
            }
        }
    }
    if result.len() != expected.len() {
        return Err("independent verifier found missing actor/evidence timing".to_string());
    }
    Ok(result)
}

fn independently_build_observations(
    actor_ref: &str,
    scenario_id: &str,
    records: &[CapacityEvidenceRecord],
    timings: &BTreeMap<(String, String), &EvidenceKnowledgeTiming>,
    horizon_end: u64,
) -> Result<Vec<ObservationEvent>, String> {
    let mut output = Vec::new();
    for record in records {
        let timing = timings
            .get(&(scenario_id.to_string(), record.evidence_id.clone()))
            .ok_or_else(|| "independent verifier cannot resolve evidence timing row".to_string())?;
        if timing.actor_ref != actor_ref {
            return Err("independent verifier actor mismatch".to_string());
        }
        let fact = verifier_fact_digest(record)?;
        let reveal = [
            timing.observed_at_period,
            timing.admitted_at_period,
            timing.actor_eligible_at_period,
        ]
        .into_iter()
        .max()
        .unwrap_or(0);
        let notice_period = timing
            .revocation_observed_at_period
            .zip(timing.revocation_admitted_at_period)
            .zip(timing.revocation_actor_eligible_at_period)
            .map(|((observed, admitted), eligible)| observed.max(admitted).max(eligible));
        let until = record
            .effective_until_period_exclusive
            .min(record.fresh_until_period_exclusive)
            .min(notice_period.unwrap_or(horizon_end));
        let revoked_before_eligible = timing
            .revoked_effective_at_period
            .is_some_and(|revoked_at| revoked_at <= reveal);
        if reveal < until && !revoked_before_eligible {
            output.push(ObservationEvent {
                reveal_period: reveal,
                valid_until_period_exclusive: Some(until),
                event_id: format!("capacity-fact:{scenario_id}:{}", record.evidence_id),
                key: format!(
                    "capacity:{}:{}:{}:{}",
                    verifier_role_key(record.role),
                    record.subject_id,
                    record.effective_from_period,
                    record.effective_until_period_exclusive
                ),
                value: fact.clone(),
            });
        }
        if let Some(revoked_at) = timing.revoked_effective_at_period {
            let notice = notice_period.unwrap_or(0);
            if notice < horizon_end {
                output.push(ObservationEvent {
                    reveal_period: notice,
                    valid_until_period_exclusive: None,
                    event_id: format!("capacity-revocation:{scenario_id}:{}", record.evidence_id),
                    key: format!(
                        "capacity-revocation:{}:{}:{}",
                        verifier_role_key(record.role),
                        record.subject_id,
                        record.effective_from_period
                    ),
                    value: format!("revoked-effective-at:{revoked_at};fact:{fact}"),
                });
            }
        }
    }
    output.sort_by(|a, b| {
        a.reveal_period
            .cmp(&b.reveal_period)
            .then(a.key.cmp(&b.key))
            .then(a.value.cmp(&b.value))
            .then(a.event_id.cmp(&b.event_id))
    });
    Ok(output)
}

/// Independently verifies source schedule receipts and reconstructs the
/// actor-specific observation timeline without invoking the producer compiler.
pub fn verify_mobility_evidence_time(
    input: &MobilityEvidenceTimeInput,
    report: &MobilityEvidenceTimeReport,
) -> Result<(), String> {
    if report.schema_version != MOBILITY_EVIDENCE_TIME_REPORT_SCHEMA
        || report.case_id != input.capacity_envelope.mobility_instance.case_id
        || report.actor_ref != input.actor_ref
        || report.claim_ceiling != MOBILITY_EVIDENCE_TIME_CLAIM_CEILING
    {
        return Err("evidence-time report identity or claim ceiling mismatch".to_string());
    }
    verify_mobility_capacity_envelope(&input.capacity_envelope, &report.source_capacity_report)?;
    let timing_map = verify_timing_inventory(input)?;
    let horizon_end = input
        .capacity_envelope
        .mobility_instance
        .horizon_last_period
        .checked_add(1)
        .ok_or_else(|| "independent verifier horizon overflowed".to_string())?;
    let schedules = input
        .capacity_envelope
        .capacity_schedules
        .iter()
        .map(|schedule| (schedule.scenario_id.as_str(), schedule))
        .collect::<BTreeMap<_, _>>();
    let mut expected_instance = report
        .source_capacity_report
        .compiled_mobility_instance
        .clone();

    for scenario in &mut expected_instance.scenarios {
        let schedule = schedules
            .get(scenario.scenario_id.as_str())
            .ok_or_else(|| "independent verifier missing scenario capacity schedule".to_string())?;
        let events = independently_build_observations(
            &input.actor_ref,
            &scenario.scenario_id,
            &schedule.records,
            &timing_map,
            horizon_end,
        )?;
        scenario.observations.extend(events);
    }
    validate_mobility_instance_for_verification(&expected_instance)?;
    let mut expected_gaps = Vec::new();
    for timing in &input.evidence_timings {
        if let Some(effective) = timing.revoked_effective_at_period {
            let observed = timing.revocation_observed_at_period.ok_or_else(|| {
                "independent verifier lacks revocation observation time".to_string()
            })?;
            let admitted = timing.revocation_admitted_at_period.ok_or_else(|| {
                "independent verifier lacks revocation admission time".to_string()
            })?;
            let eligible = timing.revocation_actor_eligible_at_period.ok_or_else(|| {
                "independent verifier lacks revocation actor eligibility time".to_string()
            })?;
            let notice = observed.max(admitted).max(eligible);
            if effective < notice {
                expected_gaps.push(RevocationNoticeGap {
                    scenario_id: timing.scenario_id.clone(),
                    evidence_id: timing.evidence_id.clone(),
                    actor_ref: timing.actor_ref.clone(),
                    revoked_effective_at_period: effective,
                    notice_eligible_at_period: notice,
                });
            }
        }
    }
    expected_gaps.sort_by(|a, b| {
        a.scenario_id
            .cmp(&b.scenario_id)
            .then(a.evidence_id.cmp(&b.evidence_id))
            .then(a.actor_ref.cmp(&b.actor_ref))
    });
    if report.evidence_timings_sha256 != verifier_timing_digest(&input.evidence_timings)?
        || report.revocation_notice_gaps != expected_gaps
        || report.mobility_instance_sha256 != mobility_instance_digest(&expected_instance)?
        || report.mobility_instance != expected_instance
    {
        return Err(
            "evidence-time report differs from independent world/knowledge reconstruction"
                .to_string(),
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capacity_admission::{
        CapacityAdmissionStatus, CapacityEvidenceRecord, CapacitySubject,
    };
    use crate::mobility_capacity::{
        MOBILITY_CAPACITY_INPUT_SCHEMA, MobilityCapacityBinding, MobilityCapacityEnvelopeInput,
    };
    use crate::mobility_oracle::{
        MOBILITY_INSTANCE_SCHEMA, MobilityDisposition, MobilityScenario, MobilityUnit, MovementArc,
        ServiceFloor, solve_mobility_policy, verify_mobility_report,
    };

    fn record(
        scenario: &str,
        subject: &str,
        resource: &str,
        from: u64,
        until: u64,
        capacity: u64,
        suffix: &str,
    ) -> CapacityEvidenceRecord {
        CapacityEvidenceRecord {
            evidence_id: format!("{scenario}:{subject}:{suffix}"),
            scenario_id: scenario.to_string(),
            subject_id: subject.to_string(),
            role: CapacityRole::Route,
            canonical_resource_ref: resource.to_string(),
            effective_from_period: from,
            effective_until_period_exclusive: until,
            fresh_until_period_exclusive: until,
            capacity,
            unit_ref: "mobile-units".to_string(),
            quantity_boundary_ref: "departure-count".to_string(),
            timebase_ref: "synthetic:periods".to_string(),
            evidence_class:
                crate::capacity_admission::CapacityEvidenceClass::ConservativeOperationalBound,
            admission_status: CapacityAdmissionStatus::Admitted,
            issuer_ref: "issuer:test".to_string(),
            authority_ref: "authority:test".to_string(),
            source_bytes_sha256: format!("sha256:{}", "a".repeat(64)),
            admission_receipt_sha256: format!("sha256:{}", "b".repeat(64)),
        }
    }

    fn input(scenario_ids: &[&str]) -> MobilityEvidenceTimeInput {
        let mut scenarios = Vec::new();
        let mut schedules = Vec::new();
        let mut timings = Vec::new();
        for scenario in scenario_ids {
            scenarios.push(MobilityScenario {
                scenario_id: (*scenario).to_string(),
                observations: Vec::new(),
                route_closures: Vec::new(),
                service_floors: vec![
                    ServiceFloor {
                        site_id: "site-a".to_string(),
                        period: 1,
                        minimum_units: 1,
                    },
                    ServiceFloor {
                        site_id: "site-b".to_string(),
                        period: 1,
                        minimum_units: 1,
                    },
                ],
                route_capacity_overrides: Vec::new(),
            });
            let records = vec![
                record(
                    scenario,
                    "arc:route-a",
                    "network-edge:route-a",
                    0,
                    1,
                    2,
                    "a0",
                ),
                record(
                    scenario,
                    "arc:route-a",
                    "network-edge:route-a",
                    1,
                    5,
                    1,
                    "a1",
                ),
                record(
                    scenario,
                    "arc:route-b",
                    "network-edge:route-b",
                    0,
                    1,
                    2,
                    "b0",
                ),
                record(
                    scenario,
                    "arc:route-b",
                    "network-edge:route-b",
                    1,
                    5,
                    1,
                    "b1",
                ),
                record(scenario, "corridor:main", "road:main", 0, 1, 2, "c0"),
                record(scenario, "corridor:main", "road:main", 1, 5, 1, "c1"),
            ];
            for row in &records {
                let is_later = row.effective_from_period == 1;
                timings.push(EvidenceKnowledgeTiming {
                    scenario_id: (*scenario).to_string(),
                    evidence_id: row.evidence_id.clone(),
                    actor_ref: "dispatcher:west".to_string(),
                    observed_at_period: if is_later { 1 } else { 0 },
                    admitted_at_period: if is_later { 2 } else { 0 },
                    actor_eligible_at_period: if is_later { 3 } else { 0 },
                    revoked_effective_at_period: None,
                    revocation_observed_at_period: None,
                    revocation_admitted_at_period: None,
                    revocation_actor_eligible_at_period: None,
                });
            }
            schedules.push(crate::capacity_admission::CapacityScheduleInput {
                schema_version: crate::capacity_admission::CAPACITY_ADMISSION_SCHEMA.to_string(),
                case_id: "evidence-time-known-answer".to_string(),
                scenario_id: (*scenario).to_string(),
                timebase_ref: "synthetic:periods".to_string(),
                unit_ref: "mobile-units".to_string(),
                quantity_boundary_ref: "departure-count".to_string(),
                horizon_last_period: 4,
                required_subjects: vec![
                    CapacitySubject {
                        subject_id: "arc:route-a".to_string(),
                        role: CapacityRole::Route,
                    },
                    CapacitySubject {
                        subject_id: "arc:route-b".to_string(),
                        role: CapacityRole::Route,
                    },
                    CapacitySubject {
                        subject_id: "corridor:main".to_string(),
                        role: CapacityRole::Route,
                    },
                ],
                records,
            });
        }
        let envelope = MobilityCapacityEnvelopeInput {
            schema_version: MOBILITY_CAPACITY_INPUT_SCHEMA.to_string(),
            timebase_ref: "synthetic:periods".to_string(),
            unit_ref: "mobile-units".to_string(),
            quantity_boundary_ref: "departure-count".to_string(),
            mobility_instance: MobilityInstance {
                schema_version: MOBILITY_INSTANCE_SCHEMA.to_string(),
                case_id: "evidence-time-known-answer".to_string(),
                horizon_last_period: 4,
                sites: vec![
                    "depot".to_string(),
                    "site-a".to_string(),
                    "site-b".to_string(),
                ],
                units: vec![
                    MobilityUnit {
                        unit_id: "unit-1".to_string(),
                        initial_site_id: "depot".to_string(),
                    },
                    MobilityUnit {
                        unit_id: "unit-2".to_string(),
                        initial_site_id: "depot".to_string(),
                    },
                ],
                arcs: vec![
                    MovementArc {
                        arc_id: "route-a".to_string(),
                        from_site_id: "depot".to_string(),
                        to_site_id: "site-a".to_string(),
                        travel_time_periods: 1,
                        arc_capacity_per_departure: 2,
                        corridor_ref: "road:main".to_string(),
                        corridor_capacity_per_departure: 2,
                        available_departure_periods: vec![0, 1, 2, 3],
                    },
                    MovementArc {
                        arc_id: "route-b".to_string(),
                        from_site_id: "depot".to_string(),
                        to_site_id: "site-b".to_string(),
                        travel_time_periods: 1,
                        arc_capacity_per_departure: 2,
                        corridor_ref: "road:main".to_string(),
                        corridor_capacity_per_departure: 2,
                        available_departure_periods: vec![0, 1, 2, 3],
                    },
                ],
                scenarios,
            },
            bindings: vec![
                MobilityCapacityBinding {
                    arc_id: "route-a".to_string(),
                    arc_capacity_subject_id: "arc:route-a".to_string(),
                    arc_resource_ref: "network-edge:route-a".to_string(),
                    corridor_capacity_subject_id: "corridor:main".to_string(),
                },
                MobilityCapacityBinding {
                    arc_id: "route-b".to_string(),
                    arc_capacity_subject_id: "arc:route-b".to_string(),
                    arc_resource_ref: "network-edge:route-b".to_string(),
                    corridor_capacity_subject_id: "corridor:main".to_string(),
                },
            ],
            capacity_schedules: schedules,
        };
        MobilityEvidenceTimeInput {
            schema_version: MOBILITY_EVIDENCE_TIME_INPUT_SCHEMA.to_string(),
            actor_ref: "dispatcher:west".to_string(),
            capacity_envelope: envelope,
            evidence_timings: timings,
        }
    }

    #[test]
    fn evidence_is_not_visible_before_all_observation_admission_and_actor_eligibility_gates() {
        let source = input(&["one"]);
        let report = compile_mobility_evidence_time(&source).unwrap();
        verify_mobility_evidence_time(&source, &report).unwrap();
        let scenario = &report.mobility_instance.scenarios[0];
        let late_fact = scenario
            .observations
            .iter()
            .find(|event| event.event_id.ends_with("one:arc:route-a:a1"))
            .unwrap();
        assert_eq!(late_fact.reveal_period, 3);
        assert_eq!(late_fact.valid_until_period_exclusive, Some(5));
        let base = &report.source_capacity_report.compiled_mobility_instance;
        let period_one_capacity = base.scenarios[0]
            .route_capacity_overrides
            .iter()
            .find(|row| row.arc_id == "route-a" && row.departure_period == 1)
            .unwrap();
        assert_eq!(period_one_capacity.arc_capacity, 1);
        assert_eq!(late_fact.reveal_period, 3);
        let solved = solve_mobility_policy(&report.mobility_instance).unwrap();
        verify_mobility_report(&report.mobility_instance, &solved).unwrap();
        assert_eq!(solved.disposition, MobilityDisposition::HereAndNowFeasible);
        assert_eq!(base.scenarios[0].route_capacity_overrides.len(), 8);
    }

    #[test]
    fn revocation_expiry_is_distinct_from_revocation_notice_time() {
        let mut source = input(&["one"]);
        let row = source.capacity_envelope.capacity_schedules[0]
            .records
            .iter()
            .find(|record| record.evidence_id.ends_with(":a1"))
            .unwrap()
            .evidence_id
            .clone();
        let timing = source
            .evidence_timings
            .iter_mut()
            .find(|timing| timing.evidence_id == row)
            .unwrap();
        timing.observed_at_period = 1;
        timing.admitted_at_period = 1;
        timing.actor_eligible_at_period = 1;
        timing.revoked_effective_at_period = Some(2);
        timing.revocation_observed_at_period = Some(3);
        timing.revocation_admitted_at_period = Some(4);
        timing.revocation_actor_eligible_at_period = Some(4);
        let report = compile_mobility_evidence_time(&source).unwrap();
        verify_mobility_evidence_time(&source, &report).unwrap();
        let scenario = &report.mobility_instance.scenarios[0];
        let fact = scenario
            .observations
            .iter()
            .find(|event| {
                event.event_id.starts_with("capacity-fact:") && event.event_id.ends_with(&row)
            })
            .unwrap();
        assert_eq!(fact.reveal_period, 1);
        assert_eq!(fact.valid_until_period_exclusive, Some(4));
        let notice = scenario
            .observations
            .iter()
            .find(|event| event.event_id.starts_with("capacity-revocation:"))
            .unwrap();
        assert_eq!(notice.reveal_period, 4);
        assert!(notice.valid_until_period_exclusive.is_none());
        assert_eq!(report.revocation_notice_gaps.len(), 1);
        assert_eq!(
            report.revocation_notice_gaps[0].revoked_effective_at_period,
            2
        );
        assert_eq!(
            report.revocation_notice_gaps[0].notice_eligible_at_period,
            4
        );
        let mut tampered = report.clone();
        tampered.revocation_notice_gaps[0].notice_eligible_at_period = 3;
        assert!(verify_mobility_evidence_time(&source, &tampered).is_err());
    }

    #[test]
    fn actor_visible_capacity_fact_binds_the_admission_receipt_digest() {
        let source = input(&["one"]);
        let first = compile_mobility_evidence_time(&source).unwrap();
        let first_value = first.mobility_instance.scenarios[0]
            .observations
            .iter()
            .find(|event| event.event_id.ends_with("one:arc:route-a:a0"))
            .unwrap()
            .value
            .clone();

        let mut changed = source.clone();
        changed.capacity_envelope.capacity_schedules[0].records[0].admission_receipt_sha256 =
            format!("sha256:{}", "c".repeat(64));
        let second = compile_mobility_evidence_time(&changed).unwrap();
        let second_value = second.mobility_instance.scenarios[0]
            .observations
            .iter()
            .find(|event| event.event_id.ends_with("one:arc:route-a:a0"))
            .unwrap()
            .value
            .clone();

        assert_ne!(first_value, second_value);
        verify_mobility_evidence_time(&source, &first).unwrap();
        verify_mobility_evidence_time(&changed, &second).unwrap();
        assert!(verify_mobility_evidence_time(&source, &second).is_err());
    }

    #[test]
    fn evidence_revoked_before_actor_eligibility_is_never_presented_as_usable() {
        let mut source = input(&["one"]);
        let evidence_id = source.capacity_envelope.capacity_schedules[0]
            .records
            .iter()
            .find(|record| record.evidence_id.ends_with(":a1"))
            .unwrap()
            .evidence_id
            .clone();
        let timing = source
            .evidence_timings
            .iter_mut()
            .find(|timing| timing.evidence_id == evidence_id)
            .unwrap();
        timing.revoked_effective_at_period = Some(2);
        timing.revocation_observed_at_period = Some(3);
        timing.revocation_admitted_at_period = Some(4);
        timing.revocation_actor_eligible_at_period = Some(4);
        let report = compile_mobility_evidence_time(&source).unwrap();
        verify_mobility_evidence_time(&source, &report).unwrap();
        assert!(
            !report.mobility_instance.scenarios[0]
                .observations
                .iter()
                .any(|event| {
                    event.event_id.starts_with("capacity-fact:")
                        && event.event_id.ends_with(&evidence_id)
                })
        );
        assert!(
            report.mobility_instance.scenarios[0]
                .observations
                .iter()
                .any(|event| {
                    event.event_id.starts_with("capacity-revocation:")
                        && event.event_id.ends_with(&evidence_id)
                })
        );
    }

    #[test]
    fn equivalent_canonical_evidence_with_different_ids_has_same_policy_value() {
        let source = input(&["alpha", "beta"]);
        let report = compile_mobility_evidence_time(&source).unwrap();
        verify_mobility_evidence_time(&source, &report).unwrap();
        let alpha = &report.mobility_instance.scenarios[0];
        let beta = &report.mobility_instance.scenarios[1];
        let alpha_events = alpha
            .observations
            .iter()
            .filter(|event| event.key.starts_with("capacity:"))
            .map(|event| (event.reveal_period, event.key.clone(), event.value.clone()))
            .collect::<Vec<_>>();
        let beta_events = beta
            .observations
            .iter()
            .filter(|event| event.key.starts_with("capacity:"))
            .map(|event| (event.reveal_period, event.key.clone(), event.value.clone()))
            .collect::<Vec<_>>();
        assert_eq!(alpha_events, beta_events);
    }

    #[test]
    fn timing_identity_and_mutated_observation_reports_are_rejected() {
        let source = input(&["one"]);
        let mut report = compile_mobility_evidence_time(&source).unwrap();
        report.mobility_instance.scenarios[0].observations[0].reveal_period = 4;
        assert!(verify_mobility_evidence_time(&source, &report).is_err());

        let mut source = input(&["one"]);
        source.evidence_timings[0].actor_ref = "dispatcher:east".to_string();
        assert!(compile_mobility_evidence_time(&source).is_err());

        let mut source = input(&["one"]);
        source.evidence_timings.pop();
        assert!(compile_mobility_evidence_time(&source).is_err());
    }

    #[test]
    fn portable_schema_documents_parse_as_json() {
        for schema in [
            include_str!("../schemas/mobility-instance-v0.1.schema.json"),
            include_str!("../schemas/mobility-capacity-envelope-input-v0.1.schema.json"),
            include_str!("../schemas/mobility-capacity-report-v0.1.schema.json"),
            include_str!("../schemas/mobility-evidence-time-input-v0.1.schema.json"),
            include_str!("../schemas/mobility-evidence-time-report-v0.1.schema.json"),
            include_str!("../schemas/mobility-report-v0.1.schema.json"),
        ] {
            serde_json::from_str::<serde_json::Value>(schema).unwrap();
        }
    }

    #[test]
    fn revocation_timestamps_must_be_complete_and_consistent() {
        let mut source = input(&["one"]);
        source.evidence_timings[0].revoked_effective_at_period = Some(2);
        assert!(compile_mobility_evidence_time(&source).is_err());
        let mut source = input(&["one"]);
        source.evidence_timings[0].revocation_observed_at_period = Some(2);
        assert!(compile_mobility_evidence_time(&source).is_err());
    }
}
