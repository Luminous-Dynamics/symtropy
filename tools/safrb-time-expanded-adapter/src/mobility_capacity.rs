#![forbid(unsafe_code)]

//! Compiles externally pre-admitted, time-varying route-capacity windows into
//! the bounded physical-mobility policy model. Source authenticity, issuer
//! authority and physical truth remain upstream responsibilities.

use crate::capacity_admission::{
    CapacityRole, CapacityScheduleInput, CapacityScheduleReport, validate_capacity_schedule,
    verify_capacity_schedule,
};
use crate::mobility_oracle::{
    MobilityInstance, RouteCapacityOverride, mobility_instance_digest, validate_mobility_instance,
    validate_mobility_instance_for_verification,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub const MOBILITY_CAPACITY_INPUT_SCHEMA: &str = "safrb-mobility-capacity-envelope-v0.1";
pub const MOBILITY_CAPACITY_REPORT_SCHEMA: &str = "safrb-mobility-capacity-report-v0.1";
pub const MOBILITY_CAPACITY_CLAIM_CEILING: &str = "Structurally compiles externally marked-admitted route-capacity records into scenario- and departure-period-specific bounded mobility constraints. This does not authenticate source bytes, issuer authority, physical identity, measurement truth, route condition, scenario completeness, dispatch authorization or delivered field service.";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MobilityCapacityBinding {
    pub arc_id: String,
    /// Subject whose Route-role capacity limits this directed arc.
    pub arc_capacity_subject_id: String,
    /// Canonical physical route identity expected in the admitted schedule.
    pub arc_resource_ref: String,
    /// Shared Route-role subject for all directed aliases of one corridor.
    pub corridor_capacity_subject_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MobilityCapacityEnvelopeInput {
    pub schema_version: String,
    pub timebase_ref: String,
    pub unit_ref: String,
    pub quantity_boundary_ref: String,
    pub mobility_instance: MobilityInstance,
    pub bindings: Vec<MobilityCapacityBinding>,
    /// Exactly one fully covering capacity schedule for every mobility scenario.
    pub capacity_schedules: Vec<CapacityScheduleInput>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MobilityCapacityEnvelopeReport {
    pub schema_version: String,
    pub case_id: String,
    pub source_mobility_instance_sha256: String,
    pub bindings_sha256: String,
    pub compiled_mobility_instance_sha256: String,
    pub compiled_mobility_instance: MobilityInstance,
    pub schedule_reports: Vec<CapacityScheduleReport>,
    pub claim_ceiling: String,
}

fn binding_digest(bindings: &[MobilityCapacityBinding]) -> Result<String, String> {
    let mut canonical = bindings.to_vec();
    canonical.sort_by(|left, right| {
        left.arc_id
            .cmp(&right.arc_id)
            .then_with(|| {
                left.arc_capacity_subject_id
                    .cmp(&right.arc_capacity_subject_id)
            })
            .then_with(|| left.arc_resource_ref.cmp(&right.arc_resource_ref))
            .then_with(|| {
                left.corridor_capacity_subject_id
                    .cmp(&right.corridor_capacity_subject_id)
            })
    });
    let encoded = serde_json::to_vec(&canonical).map_err(|error| {
        format!("could not encode canonical mobility-capacity bindings: {error}")
    })?;
    let mut bytes = b"LUMINOUS-SAFRB-MOBILITY-CAPACITY-BINDINGS-V1\0".to_vec();
    bytes.extend_from_slice(&(encoded.len() as u64).to_be_bytes());
    bytes.extend_from_slice(&encoded);
    Ok(format!("sha256:{:x}", Sha256::digest(bytes)))
}

fn expected_binding_resources(
    instance: &MobilityInstance,
    bindings: &[MobilityCapacityBinding],
) -> Result<BTreeMap<String, String>, String> {
    if bindings.len() != instance.arcs.len() {
        return Err(
            "every mobility arc must have exactly one explicit capacity binding".to_string(),
        );
    }
    let arcs = instance
        .arcs
        .iter()
        .map(|arc| (arc.arc_id.as_str(), arc))
        .collect::<BTreeMap<_, _>>();
    let mut seen_arcs = BTreeSet::new();
    let mut arc_subjects = BTreeSet::new();
    let mut corridor_subject_by_ref = BTreeMap::<String, String>::new();
    let mut corridor_ref_by_subject = BTreeMap::<String, String>::new();
    let mut resources = BTreeMap::<String, String>::new();

    for binding in bindings {
        if binding.arc_id.trim().is_empty()
            || binding.arc_capacity_subject_id.trim().is_empty()
            || binding.arc_resource_ref.trim().is_empty()
            || binding.corridor_capacity_subject_id.trim().is_empty()
        {
            return Err("mobility capacity binding contains an empty identity".to_string());
        }
        let arc = arcs
            .get(binding.arc_id.as_str())
            .ok_or_else(|| format!("capacity binding references unknown arc {}", binding.arc_id))?;
        if !seen_arcs.insert(binding.arc_id.as_str()) {
            return Err(format!(
                "duplicate capacity binding for arc {}",
                binding.arc_id
            ));
        }
        if !arc_subjects.insert(binding.arc_capacity_subject_id.as_str()) {
            return Err("each directed arc must have a distinct arc-capacity subject".to_string());
        }
        if binding.arc_capacity_subject_id == binding.corridor_capacity_subject_id {
            return Err("arc and corridor capacity subjects must be distinct".to_string());
        }

        match resources.get(&binding.arc_capacity_subject_id) {
            Some(existing) if existing != &binding.arc_resource_ref => {
                return Err("arc-capacity subject maps to multiple physical identities".to_string());
            }
            Some(_) => return Err("arc-capacity subject is reused".to_string()),
            None => {
                resources.insert(
                    binding.arc_capacity_subject_id.clone(),
                    binding.arc_resource_ref.clone(),
                );
            }
        }

        match corridor_subject_by_ref.get(&arc.corridor_ref) {
            Some(existing) if existing != &binding.corridor_capacity_subject_id => {
                return Err(format!(
                    "all aliases of corridor {} must use one corridor-capacity subject",
                    arc.corridor_ref
                ));
            }
            Some(_) => {}
            None => {
                corridor_subject_by_ref.insert(
                    arc.corridor_ref.clone(),
                    binding.corridor_capacity_subject_id.clone(),
                );
            }
        }
        match corridor_ref_by_subject.get(&binding.corridor_capacity_subject_id) {
            Some(existing) if existing != &arc.corridor_ref => {
                return Err(
                    "corridor-capacity subject maps to multiple physical corridors".to_string(),
                );
            }
            Some(_) => {}
            None => {
                corridor_ref_by_subject.insert(
                    binding.corridor_capacity_subject_id.clone(),
                    arc.corridor_ref.clone(),
                );
            }
        }
        match resources.get(&binding.corridor_capacity_subject_id) {
            Some(existing) if existing != &arc.corridor_ref => {
                return Err("corridor-capacity subject maps to conflicting resources".to_string());
            }
            Some(_) => {}
            None => {
                resources.insert(
                    binding.corridor_capacity_subject_id.clone(),
                    arc.corridor_ref.clone(),
                );
            }
        }
    }

    if seen_arcs.len() != arcs.len() {
        return Err("one or more mobility arcs have no capacity binding".to_string());
    }
    if arc_subjects
        .iter()
        .any(|subject| corridor_ref_by_subject.contains_key(*subject))
    {
        return Err("arc and corridor subject identities must not overlap".to_string());
    }
    Ok(resources)
}

fn validate_schedule_bundle(
    input: &MobilityCapacityEnvelopeInput,
    expected_resources: &BTreeMap<String, String>,
) -> Result<BTreeMap<String, CapacityScheduleReport>, String> {
    if input.timebase_ref.trim().is_empty()
        || input.unit_ref.trim().is_empty()
        || input.quantity_boundary_ref.trim().is_empty()
    {
        return Err(
            "mobility capacity envelope must declare timebase, unit and quantity boundary"
                .to_string(),
        );
    }
    if input.capacity_schedules.len() != input.mobility_instance.scenarios.len() {
        return Err(
            "capacity schedules must cover every mobility scenario exactly once".to_string(),
        );
    }
    let expected_subjects = expected_resources
        .keys()
        .map(|subject_id| (CapacityRole::Route, subject_id.clone()))
        .collect::<BTreeSet<_>>();
    let scenarios = input
        .mobility_instance
        .scenarios
        .iter()
        .map(|scenario| scenario.scenario_id.as_str())
        .collect::<BTreeSet<_>>();
    if scenarios.len() != input.mobility_instance.scenarios.len() {
        return Err("mobility scenario identities are not unique".to_string());
    }

    let mut result = BTreeMap::new();
    let mut common_timebase: Option<&str> = None;
    let mut common_unit: Option<&str> = None;
    let mut common_boundary: Option<&str> = None;

    for schedule in &input.capacity_schedules {
        if schedule.case_id != input.mobility_instance.case_id
            || schedule.horizon_last_period != input.mobility_instance.horizon_last_period
            || schedule.timebase_ref != input.timebase_ref
            || schedule.unit_ref != input.unit_ref
            || schedule.quantity_boundary_ref != input.quantity_boundary_ref
            || !scenarios.contains(schedule.scenario_id.as_str())
        {
            return Err(format!(
                "capacity schedule {} has mismatched case, horizon or scenario identity",
                schedule.scenario_id
            ));
        }
        if result.contains_key(&schedule.scenario_id) {
            return Err(format!(
                "duplicate capacity schedule for scenario {}",
                schedule.scenario_id
            ));
        }
        if common_timebase.is_some_and(|value| value != schedule.timebase_ref.as_str())
            || common_unit.is_some_and(|value| value != schedule.unit_ref.as_str())
            || common_boundary.is_some_and(|value| value != schedule.quantity_boundary_ref.as_str())
        {
            return Err(
                "capacity schedules use incomparable timebases, units or quantity boundaries"
                    .to_string(),
            );
        }
        common_timebase = Some(&schedule.timebase_ref);
        common_unit = Some(&schedule.unit_ref);
        common_boundary = Some(&schedule.quantity_boundary_ref);

        let supplied_subjects = schedule
            .required_subjects
            .iter()
            .map(|subject| (subject.role, subject.subject_id.clone()))
            .collect::<BTreeSet<_>>();
        if supplied_subjects.len() != schedule.required_subjects.len()
            || supplied_subjects != expected_subjects
            || schedule
                .required_subjects
                .iter()
                .any(|subject| subject.role != CapacityRole::Route)
        {
            return Err(format!(
                "scenario {} required subjects do not exactly match the explicit mobility bindings",
                schedule.scenario_id
            ));
        }
        for record in &schedule.records {
            let expected_resource =
                expected_resources.get(&record.subject_id).ok_or_else(|| {
                    format!(
                        "capacity record {} targets an unmapped subject",
                        record.evidence_id
                    )
                })?;
            if record.role != CapacityRole::Route
                || record.canonical_resource_ref != *expected_resource
            {
                return Err(format!(
                    "capacity record {} has a route role or physical-resource binding mismatch",
                    record.evidence_id
                ));
            }
        }

        let report = validate_capacity_schedule(schedule)?;
        verify_capacity_schedule(schedule, &report)?;
        result.insert(schedule.scenario_id.clone(), report);
    }

    if result.len() != scenarios.len() {
        return Err("one or more mobility scenarios lack a unique capacity schedule".to_string());
    }
    Ok(result)
}

fn capacity_at(
    schedule: &CapacityScheduleInput,
    subject_id: &str,
    departure_period: u64,
) -> Result<u64, String> {
    let mut found = None;
    for record in &schedule.records {
        if record.role == CapacityRole::Route
            && record.subject_id == subject_id
            && record.effective_from_period <= departure_period
            && departure_period < record.effective_until_period_exclusive
        {
            if found.is_some() {
                return Err(format!(
                    "overlapping admitted capacity windows for subject {subject_id} at period {departure_period}"
                ));
            }
            found = Some(record.capacity);
        }
    }
    found.ok_or_else(|| {
        format!("missing admitted capacity for subject {subject_id} at period {departure_period}")
    })
}

/// Compile complete externally admitted route and corridor schedules into
/// exact scenario/departure-period capacity overrides. Legacy static capacity
/// values remain a ceiling and are never used to fill missing evidence.
pub fn compile_mobility_capacity_envelope(
    input: &MobilityCapacityEnvelopeInput,
) -> Result<MobilityCapacityEnvelopeReport, String> {
    if input.schema_version != MOBILITY_CAPACITY_INPUT_SCHEMA {
        return Err("unsupported mobility-capacity input schema".to_string());
    }
    validate_mobility_instance(&input.mobility_instance)?;
    if input
        .mobility_instance
        .scenarios
        .iter()
        .any(|scenario| !scenario.route_capacity_overrides.is_empty())
    {
        return Err("source mobility instance already contains capacity overrides".to_string());
    }

    let expected_resources = expected_binding_resources(&input.mobility_instance, &input.bindings)?;
    let schedule_reports = validate_schedule_bundle(input, &expected_resources)?;
    let schedules = input
        .capacity_schedules
        .iter()
        .map(|schedule| (schedule.scenario_id.as_str(), schedule))
        .collect::<BTreeMap<_, _>>();
    let mut compiled = input.mobility_instance.clone();

    for scenario in &mut compiled.scenarios {
        let schedule = schedules
            .get(scenario.scenario_id.as_str())
            .ok_or_else(|| format!("missing schedule for scenario {}", scenario.scenario_id))?;
        let mut overrides = Vec::new();
        for binding in &input.bindings {
            let arc = compiled
                .arcs
                .iter()
                .find(|arc| arc.arc_id == binding.arc_id)
                .ok_or_else(|| {
                    format!(
                        "bound arc {} disappeared during compilation",
                        binding.arc_id
                    )
                })?;
            for period in 0..compiled.horizon_last_period {
                let arc_capacity = capacity_at(schedule, &binding.arc_capacity_subject_id, period)?;
                let corridor_capacity =
                    capacity_at(schedule, &binding.corridor_capacity_subject_id, period)?;
                if arc_capacity > arc.arc_capacity_per_departure
                    || corridor_capacity > arc.corridor_capacity_per_departure
                {
                    return Err(format!(
                        "admitted schedule for arc {} exceeds the declared structural capacity ceiling at period {period}",
                        arc.arc_id
                    ));
                }
                overrides.push(RouteCapacityOverride {
                    arc_id: arc.arc_id.clone(),
                    departure_period: period,
                    arc_capacity,
                    corridor_capacity,
                });
            }
        }
        overrides.sort_by(|left, right| {
            (&left.departure_period, &left.arc_id).cmp(&(&right.departure_period, &right.arc_id))
        });
        scenario.route_capacity_overrides = overrides;
    }

    validate_mobility_instance(&compiled)?;
    let source_digest = mobility_instance_digest(&input.mobility_instance)?;
    let compiled_digest = mobility_instance_digest(&compiled)?;
    let mut ordered_reports = schedule_reports.into_values().collect::<Vec<_>>();
    ordered_reports.sort_by(|left, right| left.scenario_id.cmp(&right.scenario_id));

    Ok(MobilityCapacityEnvelopeReport {
        schema_version: MOBILITY_CAPACITY_REPORT_SCHEMA.to_string(),
        case_id: input.mobility_instance.case_id.clone(),
        source_mobility_instance_sha256: source_digest,
        bindings_sha256: binding_digest(&input.bindings)?,
        compiled_mobility_instance_sha256: compiled_digest,
        compiled_mobility_instance: compiled,
        schedule_reports: ordered_reports,
        claim_ceiling: MOBILITY_CAPACITY_CLAIM_CEILING.to_string(),
    })
}

fn verifier_binding_digest(bindings: &[MobilityCapacityBinding]) -> Result<String, String> {
    let mut rows = bindings.to_vec();
    rows.sort_by(|a, b| {
        a.arc_id
            .cmp(&b.arc_id)
            .then(a.arc_capacity_subject_id.cmp(&b.arc_capacity_subject_id))
            .then(a.arc_resource_ref.cmp(&b.arc_resource_ref))
            .then(
                a.corridor_capacity_subject_id
                    .cmp(&b.corridor_capacity_subject_id),
            )
    });
    let encoded = serde_json::to_vec(&rows).map_err(|error| {
        format!("mobility-capacity verifier could not encode bindings: {error}")
    })?;
    let mut digest_input = Vec::new();
    digest_input.extend_from_slice(b"LUMINOUS-SAFRB-MOBILITY-CAPACITY-BINDINGS-V1\0");
    digest_input.extend_from_slice(&(encoded.len() as u64).to_be_bytes());
    digest_input.extend_from_slice(&encoded);
    Ok(format!("sha256:{:x}", Sha256::digest(digest_input)))
}

fn independent_capacity_at(
    schedule: &CapacityScheduleInput,
    subject_id: &str,
    period: u64,
) -> Result<u64, String> {
    let mut value = None;
    for row in &schedule.records {
        if row.role == CapacityRole::Route
            && row.subject_id.as_str() == subject_id
            && period >= row.effective_from_period
            && period < row.effective_until_period_exclusive
        {
            if value.replace(row.capacity).is_some() {
                return Err(
                    "mobility-capacity verifier found overlapping route evidence".to_string(),
                );
            }
        }
    }
    value
        .ok_or_else(|| "mobility-capacity verifier found an uncovered departure period".to_string())
}

/// Independently re-admit source and schedules, reconstruct every override
/// from original records, and compare the full compiled model byte-for-byte
/// at the typed-structure level. This does not call the producer compiler.
pub fn verify_mobility_capacity_envelope(
    input: &MobilityCapacityEnvelopeInput,
    report: &MobilityCapacityEnvelopeReport,
) -> Result<(), String> {
    if input.schema_version != MOBILITY_CAPACITY_INPUT_SCHEMA
        || input.timebase_ref.trim().is_empty()
        || input.unit_ref.trim().is_empty()
        || input.quantity_boundary_ref.trim().is_empty()
        || report.schema_version != MOBILITY_CAPACITY_REPORT_SCHEMA
        || report.case_id != input.mobility_instance.case_id
        || report.claim_ceiling != MOBILITY_CAPACITY_CLAIM_CEILING
    {
        return Err("mobility-capacity report identity or claim ceiling mismatch".to_string());
    }
    validate_mobility_instance_for_verification(&input.mobility_instance)?;
    if input
        .mobility_instance
        .scenarios
        .iter()
        .any(|scenario| !scenario.route_capacity_overrides.is_empty())
    {
        return Err(
            "mobility-capacity verifier requires an uncompiled source instance".to_string(),
        );
    }
    if input.bindings.len() != input.mobility_instance.arcs.len()
        || input.capacity_schedules.len() != input.mobility_instance.scenarios.len()
    {
        return Err(
            "mobility-capacity verifier found incomplete binding or schedule inventory".to_string(),
        );
    }

    let mut arc_ids = BTreeSet::new();
    let mut route_subjects = BTreeSet::new();
    let mut resource_by_subject = BTreeMap::<String, String>::new();
    let mut corridor_subject_by_ref = BTreeMap::<String, String>::new();
    let mut corridor_ref_by_subject = BTreeMap::<String, String>::new();
    for binding in &input.bindings {
        let arc = input
            .mobility_instance
            .arcs
            .iter()
            .find(|candidate| candidate.arc_id.as_str() == binding.arc_id.as_str())
            .ok_or_else(|| {
                "mobility-capacity verifier found a binding for an unknown arc".to_string()
            })?;
        if binding.arc_id.trim().is_empty()
            || binding.arc_resource_ref.trim().is_empty()
            || binding.arc_capacity_subject_id.trim().is_empty()
            || binding.corridor_capacity_subject_id.trim().is_empty()
            || !arc_ids.insert(binding.arc_id.as_str())
            || !route_subjects.insert(binding.arc_capacity_subject_id.as_str())
            || binding.arc_capacity_subject_id == binding.corridor_capacity_subject_id
        {
            return Err(
                "mobility-capacity verifier rejected duplicate or empty binding identity"
                    .to_string(),
            );
        }
        if resource_by_subject
            .insert(
                binding.arc_capacity_subject_id.clone(),
                binding.arc_resource_ref.clone(),
            )
            .is_some()
        {
            return Err("mobility-capacity verifier found reused arc subject".to_string());
        }
        match corridor_subject_by_ref.get(&arc.corridor_ref) {
            Some(existing) if existing != &binding.corridor_capacity_subject_id => {
                return Err(
                    "mobility-capacity verifier found split aliases for a physical corridor"
                        .to_string(),
                );
            }
            Some(_) => {}
            None => {
                corridor_subject_by_ref.insert(
                    arc.corridor_ref.clone(),
                    binding.corridor_capacity_subject_id.clone(),
                );
            }
        }
        match corridor_ref_by_subject.get(&binding.corridor_capacity_subject_id) {
            Some(existing) if existing != &arc.corridor_ref => {
                return Err("mobility-capacity verifier found one corridor subject mapped to different resources".to_string());
            }
            Some(_) => {}
            None => {
                corridor_ref_by_subject.insert(
                    binding.corridor_capacity_subject_id.clone(),
                    arc.corridor_ref.clone(),
                );
            }
        }
        match resource_by_subject.get(&binding.corridor_capacity_subject_id) {
            Some(existing) if existing != &arc.corridor_ref => {
                return Err(
                    "mobility-capacity verifier found conflicting corridor resource identity"
                        .to_string(),
                );
            }
            Some(_) => {}
            None => {
                resource_by_subject.insert(
                    binding.corridor_capacity_subject_id.clone(),
                    arc.corridor_ref.clone(),
                );
            }
        }
    }
    if arc_ids.len() != input.mobility_instance.arcs.len() {
        return Err("mobility-capacity verifier found an unbound movement arc".to_string());
    }
    if route_subjects
        .iter()
        .any(|subject| corridor_ref_by_subject.contains_key(*subject))
    {
        return Err(
            "mobility-capacity verifier found overlapping arc and corridor subject identities"
                .to_string(),
        );
    }

    let expected_subjects = resource_by_subject.keys().cloned().collect::<BTreeSet<_>>();
    let mut schedules = BTreeMap::<String, &CapacityScheduleInput>::new();
    let mut common_timebase: Option<&str> = None;
    let mut common_unit: Option<&str> = None;
    let mut common_boundary: Option<&str> = None;
    let scenario_ids = input
        .mobility_instance
        .scenarios
        .iter()
        .map(|scenario| scenario.scenario_id.as_str())
        .collect::<BTreeSet<_>>();
    for schedule in &input.capacity_schedules {
        if schedule.case_id != input.mobility_instance.case_id
            || schedule.horizon_last_period != input.mobility_instance.horizon_last_period
            || schedule.timebase_ref != input.timebase_ref
            || schedule.unit_ref != input.unit_ref
            || schedule.quantity_boundary_ref != input.quantity_boundary_ref
            || !scenario_ids.contains(schedule.scenario_id.as_str())
            || common_timebase.is_some_and(|value| value != schedule.timebase_ref.as_str())
            || common_unit.is_some_and(|value| value != schedule.unit_ref.as_str())
            || common_boundary.is_some_and(|value| value != schedule.quantity_boundary_ref.as_str())
        {
            return Err(
                "mobility-capacity verifier found case, scenario, horizon or dimensional mismatch"
                    .to_string(),
            );
        }
        if schedules
            .insert(schedule.scenario_id.clone(), schedule)
            .is_some()
        {
            return Err("mobility-capacity verifier found duplicate scenario schedule".to_string());
        }
        common_timebase = Some(&schedule.timebase_ref);
        common_unit = Some(&schedule.unit_ref);
        common_boundary = Some(&schedule.quantity_boundary_ref);

        let subjects = schedule
            .required_subjects
            .iter()
            .filter(|subject| subject.role == CapacityRole::Route)
            .map(|subject| subject.subject_id.clone())
            .collect::<BTreeSet<_>>();
        if subjects != expected_subjects || subjects.len() != schedule.required_subjects.len() {
            return Err(
                "mobility-capacity verifier found route subjects that differ from the binding map"
                    .to_string(),
            );
        }
        for row in &schedule.records {
            let expected_ref = resource_by_subject.get(&row.subject_id).ok_or_else(|| {
                "mobility-capacity verifier found an undeclared evidence subject".to_string()
            })?;
            if row.role != CapacityRole::Route || row.canonical_resource_ref != *expected_ref {
                return Err(
                    "mobility-capacity verifier found a resource-binding mismatch".to_string(),
                );
            }
        }
    }
    if schedules.len() != scenario_ids.len() {
        return Err("mobility-capacity verifier found missing scenario schedule".to_string());
    }

    if report.schedule_reports.len() != schedules.len() {
        return Err("mobility-capacity report has an incomplete schedule receipt set".to_string());
    }
    let mut reports_by_scenario = BTreeMap::new();
    for schedule_report in &report.schedule_reports {
        if reports_by_scenario
            .insert(schedule_report.scenario_id.as_str(), schedule_report)
            .is_some()
        {
            return Err("mobility-capacity report repeats a scenario receipt".to_string());
        }
    }

    let mut expected_instance = input.mobility_instance.clone();
    for scenario in &mut expected_instance.scenarios {
        let schedule = schedules.get(&scenario.scenario_id).ok_or_else(|| {
            "mobility-capacity verifier cannot find scenario's source schedule".to_string()
        })?;
        let schedule_report = reports_by_scenario
            .get(scenario.scenario_id.as_str())
            .ok_or_else(|| {
                "mobility-capacity report omits a scenario schedule receipt".to_string()
            })?;
        verify_capacity_schedule(schedule, schedule_report)?;

        let mut overrides = Vec::new();
        for binding in &input.bindings {
            let arc = expected_instance
                .arcs
                .iter()
                .find(|candidate| candidate.arc_id == binding.arc_id)
                .ok_or_else(|| "mobility-capacity verifier lost a bound arc".to_string())?;
            for period in 0..expected_instance.horizon_last_period {
                let route_cap =
                    independent_capacity_at(schedule, &binding.arc_capacity_subject_id, period)?;
                let corridor_cap = independent_capacity_at(
                    schedule,
                    &binding.corridor_capacity_subject_id,
                    period,
                )?;
                if route_cap > arc.arc_capacity_per_departure
                    || corridor_cap > arc.corridor_capacity_per_departure
                {
                    return Err("mobility-capacity verifier found evidence above the structural capacity ceiling".to_string());
                }
                overrides.push(RouteCapacityOverride {
                    arc_id: arc.arc_id.clone(),
                    departure_period: period,
                    arc_capacity: route_cap,
                    corridor_capacity: corridor_cap,
                });
            }
        }
        overrides.sort_by(|a, b| {
            a.departure_period
                .cmp(&b.departure_period)
                .then(a.arc_id.cmp(&b.arc_id))
        });
        scenario.route_capacity_overrides = overrides;
    }

    validate_mobility_instance_for_verification(&expected_instance)?;
    let source_digest = mobility_instance_digest(&input.mobility_instance)?;
    let expected_compiled_digest = mobility_instance_digest(&expected_instance)?;
    if report.source_mobility_instance_sha256 != source_digest
        || report.bindings_sha256 != verifier_binding_digest(&input.bindings)?
        || report.compiled_mobility_instance_sha256 != expected_compiled_digest
        || report.compiled_mobility_instance != expected_instance
    {
        return Err(
            "mobility-capacity report does not match independently reconstructed model".to_string(),
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capacity_admission::{
        CapacityAdmissionStatus, CapacityEvidenceClass, CapacityEvidenceRecord, CapacitySubject,
    };
    use crate::mobility_oracle::{
        MOBILITY_INSTANCE_SCHEMA, MobilityDisposition, MobilityScenario, MobilityUnit, MovementArc,
        ServiceFloor, solve_mobility_policy, verify_mobility_report,
    };

    fn base_instance(scenario_ids: &[&str]) -> MobilityInstance {
        let scenarios = scenario_ids
            .iter()
            .map(|id| MobilityScenario {
                scenario_id: (*id).to_string(),
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
            })
            .collect();
        MobilityInstance {
            schema_version: MOBILITY_INSTANCE_SCHEMA.to_string(),
            case_id: "mobility-capacity-known-answer".to_string(),
            horizon_last_period: 2,
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
                    available_departure_periods: vec![0, 1],
                },
                MovementArc {
                    arc_id: "route-b".to_string(),
                    from_site_id: "depot".to_string(),
                    to_site_id: "site-b".to_string(),
                    travel_time_periods: 1,
                    arc_capacity_per_departure: 2,
                    corridor_ref: "road:main".to_string(),
                    corridor_capacity_per_departure: 2,
                    available_departure_periods: vec![0, 1],
                },
            ],
            scenarios,
        }
    }

    fn bindings() -> Vec<MobilityCapacityBinding> {
        vec![
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
        ]
    }

    fn record(
        scenario: &str,
        subject: &str,
        resource: &str,
        from: u64,
        until: u64,
        capacity: u64,
        ordinal: u64,
    ) -> CapacityEvidenceRecord {
        CapacityEvidenceRecord {
            evidence_id: format!("{scenario}:{subject}:{ordinal}"),
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
            evidence_class: CapacityEvidenceClass::ConservativeOperationalBound,
            admission_status: CapacityAdmissionStatus::Admitted,
            issuer_ref: "issuer:test".to_string(),
            authority_ref: "authority:test".to_string(),
            source_bytes_sha256: format!("sha256:{}", "a".repeat(64)),
            admission_receipt_sha256: format!("sha256:{}", "b".repeat(64)),
        }
    }

    fn schedule(scenario: &str, period_zero: u64, later_periods: u64) -> CapacityScheduleInput {
        let mut records = Vec::new();
        for (subject, resource) in [
            ("arc:route-a", "network-edge:route-a"),
            ("arc:route-b", "network-edge:route-b"),
            ("corridor:main", "road:main"),
        ] {
            records.push(record(scenario, subject, resource, 0, 1, period_zero, 0));
            records.push(record(scenario, subject, resource, 1, 3, later_periods, 1));
        }
        CapacityScheduleInput {
            schema_version: crate::capacity_admission::CAPACITY_ADMISSION_SCHEMA.to_string(),
            case_id: "mobility-capacity-known-answer".to_string(),
            scenario_id: scenario.to_string(),
            timebase_ref: "synthetic:periods".to_string(),
            unit_ref: "mobile-units".to_string(),
            quantity_boundary_ref: "departure-count".to_string(),
            horizon_last_period: 2,
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
        }
    }

    fn input(
        scenario_ids: &[&str],
        capacities: &[(&str, u64, u64)],
    ) -> MobilityCapacityEnvelopeInput {
        MobilityCapacityEnvelopeInput {
            schema_version: MOBILITY_CAPACITY_INPUT_SCHEMA.to_string(),
            timebase_ref: "synthetic:periods".to_string(),
            unit_ref: "mobile-units".to_string(),
            quantity_boundary_ref: "departure-count".to_string(),
            mobility_instance: base_instance(scenario_ids),
            bindings: bindings(),
            capacity_schedules: capacities
                .iter()
                .map(|(scenario, first, later)| schedule(scenario, *first, *later))
                .collect(),
        }
    }

    #[test]
    fn compiles_per_period_arc_and_shared_corridor_capacity_and_verifies_independently() {
        let source = input(&["normal"], &[("normal", 2, 1)]);
        let report = compile_mobility_capacity_envelope(&source).unwrap();
        verify_mobility_capacity_envelope(&source, &report).unwrap();
        let compiled = &report.compiled_mobility_instance;
        let scenario = &compiled.scenarios[0];
        assert_eq!(scenario.route_capacity_overrides.len(), 4);
        assert_eq!(
            scenario
                .route_capacity_overrides
                .iter()
                .find(|row| row.arc_id == "route-a" && row.departure_period == 0)
                .unwrap()
                .arc_capacity,
            2
        );
        assert_eq!(
            scenario
                .route_capacity_overrides
                .iter()
                .find(|row| row.arc_id == "route-a" && row.departure_period == 1)
                .unwrap()
                .arc_capacity,
            1
        );
        assert_eq!(
            scenario
                .route_capacity_overrides
                .iter()
                .find(|row| row.arc_id == "route-b" && row.departure_period == 1)
                .unwrap()
                .corridor_capacity,
            1
        );
        let mobility_report = solve_mobility_policy(compiled).unwrap();
        assert_eq!(
            mobility_report.disposition,
            MobilityDisposition::HereAndNowFeasible
        );
        verify_mobility_report(compiled, &mobility_report).unwrap();
    }

    #[test]
    fn low_capacity_in_one_scenario_cannot_be_spliced_with_another_scenarios_high_capacity() {
        let source = input(&["low", "high"], &[("low", 1, 1), ("high", 2, 2)]);
        let report = compile_mobility_capacity_envelope(&source).unwrap();
        verify_mobility_capacity_envelope(&source, &report).unwrap();
        let mobility_report = solve_mobility_policy(&report.compiled_mobility_instance).unwrap();
        assert_eq!(
            mobility_report.disposition,
            MobilityDisposition::InformationRelaxationOnly
        );
        verify_mobility_report(&report.compiled_mobility_instance, &mobility_report).unwrap();
    }

    #[test]
    fn gaps_stale_records_and_dimensional_mismatch_fail_closed() {
        let mut source = input(&["normal"], &[("normal", 2, 1)]);
        source.capacity_schedules[0].records[0].effective_from_period = 1;
        assert!(compile_mobility_capacity_envelope(&source).is_err());

        let mut source = input(&["normal"], &[("normal", 2, 1)]);
        source.capacity_schedules[0].records[0].admission_status = CapacityAdmissionStatus::Stale;
        assert!(compile_mobility_capacity_envelope(&source).is_err());

        let mut source = input(&["normal"], &[("normal", 2, 1)]);
        source.capacity_schedules[0].records[0].timebase_ref = "other:clock".to_string();
        assert!(compile_mobility_capacity_envelope(&source).is_err());
    }

    #[test]
    fn overlapping_or_revoked_capacity_records_fail_closed() {
        let mut source = input(&["normal"], &[("normal", 2, 1)]);
        source.capacity_schedules[0].records.push(record(
            "normal",
            "arc:route-a",
            "network-edge:route-a",
            0,
            1,
            2,
            99,
        ));
        assert!(compile_mobility_capacity_envelope(&source).is_err());

        let mut source = input(&["normal"], &[("normal", 2, 1)]);
        source.capacity_schedules[0].records[0].admission_status = CapacityAdmissionStatus::Revoked;
        assert!(compile_mobility_capacity_envelope(&source).is_err());
    }

    #[test]
    fn schedule_cannot_raise_capacity_above_the_structural_graph_ceiling() {
        let source = input(&["normal"], &[("normal", 3, 1)]);
        assert!(compile_mobility_capacity_envelope(&source).is_err());
    }

    #[test]
    fn binding_mismatch_and_alias_split_are_rejected() {
        let mut source = input(&["normal"], &[("normal", 2, 1)]);
        source.bindings[0].arc_resource_ref = "network-edge:forged".to_string();
        assert!(compile_mobility_capacity_envelope(&source).is_err());

        let mut source = input(&["normal"], &[("normal", 2, 1)]);
        source.bindings[1].corridor_capacity_subject_id = "corridor:split-alias".to_string();
        assert!(compile_mobility_capacity_envelope(&source).is_err());
    }

    #[test]
    fn forged_compiled_override_is_rejected_by_independent_reconstruction() {
        let source = input(&["normal"], &[("normal", 2, 1)]);
        let mut report = compile_mobility_capacity_envelope(&source).unwrap();
        report.compiled_mobility_instance.scenarios[0].route_capacity_overrides[0].arc_capacity = 1;
        assert!(verify_mobility_capacity_envelope(&source, &report).is_err());
    }

    #[test]
    fn schedule_receipt_mutation_is_rejected() {
        let source = input(&["normal"], &[("normal", 2, 1)]);
        let mut report = compile_mobility_capacity_envelope(&source).unwrap();
        report.schedule_reports[0].windows[0].capacity += 1;
        assert!(verify_mobility_capacity_envelope(&source, &report).is_err());
    }

    #[test]
    fn zero_capacity_is_an_explicit_departure_prohibition() {
        let source = input(&["outage"], &[("outage", 0, 1)]);
        let report = compile_mobility_capacity_envelope(&source).unwrap();
        verify_mobility_capacity_envelope(&source, &report).unwrap();
        assert!(
            report.compiled_mobility_instance.scenarios[0]
                .route_capacity_overrides
                .iter()
                .filter(|row| row.departure_period == 0)
                .all(|row| row.arc_capacity == 0 && row.corridor_capacity == 0)
        );
        let solved = solve_mobility_policy(&report.compiled_mobility_instance).unwrap();
        assert_eq!(
            solved.disposition,
            MobilityDisposition::InformationRelaxationOnly
        );
        verify_mobility_report(&report.compiled_mobility_instance, &solved).unwrap();
    }

    #[test]
    fn schema_documents_parse_as_strict_json_documents() {
        for document in [
            include_str!("../schemas/mobility-instance-v0.1.schema.json"),
            include_str!("../schemas/mobility-report-v0.1.schema.json"),
            include_str!("../schemas/mobility-capacity-envelope-input-v0.1.schema.json"),
            include_str!("../schemas/mobility-capacity-report-v0.1.schema.json"),
        ] {
            serde_json::from_str::<serde_json::Value>(document).unwrap();
        }
    }

    #[test]
    fn envelope_rejects_zero_or_missing_schedule_identity_mappings() {
        let mut source = input(&["normal"], &[("normal", 2, 1)]);
        source.capacity_schedules[0].unit_ref = "different-unit".to_string();
        assert!(compile_mobility_capacity_envelope(&source).is_err());

        let mut source = input(&["normal"], &[("normal", 2, 1)]);
        source.bindings.pop();
        assert!(compile_mobility_capacity_envelope(&source).is_err());
    }

    #[test]
    fn unknown_fields_and_unknown_scenarios_do_not_get_implicitly_bound() {
        let mut source = input(&["normal"], &[("normal", 2, 1)]);
        source.capacity_schedules[0].scenario_id = "unmodeled".to_string();
        assert!(compile_mobility_capacity_envelope(&source).is_err());
    }
}
