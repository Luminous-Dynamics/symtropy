use std::collections::{BTreeMap, BTreeSet};

#[cfg(test)]
const FIXTURE: &str = include_str!(
    "../../../docs/research/maritime-capability-to-service/fixtures/mcsc-counterfactual-tiny-v0.tsv"
);

#[derive(Clone, Debug)]
struct Shock {
    start_day: u64,
    end_day: u64,
}

#[derive(Clone, Debug)]
struct Dependency {
    available_from_day: u64,
    shocks: Vec<Shock>,
}

#[derive(Clone, Debug)]
struct Service {
    id: String,
    expected_units: u64,
    unit: String,
    deadline_day: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum InspectionStatus {
    Pass,
    Fail,
    Unknown,
}

impl InspectionStatus {
    fn parse(value: &str, line: usize) -> Result<Self, String> {
        match value {
            "PASS" => Ok(Self::Pass),
            "FAIL" => Ok(Self::Fail),
            "UNKNOWN" => Ok(Self::Unknown),
            _ => Err(format!("line {line}: unsupported inspection status '{value}'")),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Pass => "PASS",
            Self::Fail => "FAIL",
            Self::Unknown => "UNKNOWN",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ReceiverStatus {
    Accepted,
    Rejected,
    Unknown,
}

impl ReceiverStatus {
    fn parse(value: &str, line: usize) -> Result<Self, String> {
        match value {
            "ACCEPTED" => Ok(Self::Accepted),
            "REJECTED" => Ok(Self::Rejected),
            "UNKNOWN" => Ok(Self::Unknown),
            _ => Err(format!("line {line}: unsupported receiver status '{value}'")),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Accepted => "ACCEPTED",
            Self::Rejected => "REJECTED",
            Self::Unknown => "UNKNOWN",
        }
    }
}

#[derive(Clone, Debug)]
struct Inspection {
    status: InspectionStatus,
    qualified_units: u64,
    revision: String,
}

#[derive(Clone, Debug)]
struct ReceiverAcceptance {
    status: ReceiverStatus,
    accepted_units: u64,
    revision: String,
}

#[derive(Clone, Debug)]
struct PortfolioOption {
    id: String,
    region: String,
    cost_units: u64,
    production_days: u64,
    inspection_days: u64,
    commissioning_days: u64,
    handoff_days: u64,
    emissions_units: u64,
    regional_value_units: u64,
    revision: String,
    dependencies: Vec<String>,
}

#[derive(Clone, Debug)]
struct Instance {
    id: String,
    version: String,
    service: Service,
    dependencies: BTreeMap<String, Dependency>,
    options: BTreeMap<String, PortfolioOption>,
    inspections: BTreeMap<String, Inspection>,
    receiver_acceptances: BTreeMap<String, ReceiverAcceptance>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Outcome {
    pub instance_id: String,
    pub instance_version: String,
    pub service_id: String,
    pub deadline_day: u64,
    pub option_id: String,
    pub region: String,
    pub receiver_day_without_shock: u64,
    pub receiver_day_with_shock: u64,
    pub expected_units: u64,
    pub inspection_qualified_units: u64,
    pub receiver_accepted_units: u64,
    pub on_time_accepted_units_with_shock: u64,
    pub inspection_status: String,
    pub receiver_status: String,
    pub revision: String,
    pub cost_units: u64,
    pub emissions_units: u64,
    pub regional_value_units: u64,
}

fn parse_u64(value: &str, label: &str, line: usize) -> Result<u64, String> {
    value
        .parse::<u64>()
        .map_err(|error| format!("line {line}: invalid {label} '{value}': {error}"))
}

fn parse_instance(input: &str) -> Result<Instance, String> {
    let mut identity: Option<(String, String)> = None;
    let mut service: Option<Service> = None;
    let mut dependencies: BTreeMap<String, Dependency> = BTreeMap::new();
    let mut options: BTreeMap<String, PortfolioOption> = BTreeMap::new();
    let mut inspections: BTreeMap<String, Inspection> = BTreeMap::new();
    let mut receiver_acceptances: BTreeMap<String, ReceiverAcceptance> = BTreeMap::new();
    let mut pending_shocks: Vec<(String, Shock)> = Vec::new();

    for (index, raw_line) in input.lines().enumerate() {
        let line_number = index + 1;
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let fields: Vec<&str> = line.split('\t').collect();
        match fields.first().copied().unwrap_or_default() {
            "INSTANCE" if fields.len() == 3 => {
                if identity.is_some() || fields[1].is_empty() || fields[2].is_empty() {
                    return Err(format!("line {line_number}: duplicate or empty INSTANCE"));
                }
                identity = Some((fields[1].to_owned(), fields[2].to_owned()));
            }
            "SERVICE" if fields.len() == 5 => {
                if service.is_some() || fields[1].is_empty() || fields[3].is_empty() {
                    return Err(format!("line {line_number}: duplicate or empty SERVICE"));
                }
                let expected_units = parse_u64(fields[2], "expected units", line_number)?;
                if expected_units == 0 {
                    return Err(format!(
                        "line {line_number}: expected units must be positive"
                    ));
                }
                service = Some(Service {
                    id: fields[1].to_owned(),
                    expected_units,
                    unit: fields[3].to_owned(),
                    deadline_day: parse_u64(fields[4], "deadline day", line_number)?,
                });
            }
            "DEPENDENCY" if fields.len() == 3 => {
                if fields[1].is_empty() {
                    return Err(format!("line {line_number}: empty dependency identity"));
                }
                let dependency = Dependency {
                    available_from_day: parse_u64(
                        fields[2],
                        "dependency availability day",
                        line_number,
                    )?,
                    shocks: Vec::new(),
                };
                if dependencies
                    .insert(fields[1].to_owned(), dependency)
                    .is_some()
                {
                    return Err(format!(
                        "line {line_number}: duplicate dependency '{}'",
                        fields[1]
                    ));
                }
            }
            "SHOCK" if fields.len() == 4 => {
                if fields[1].is_empty() {
                    return Err(format!("line {line_number}: empty shock dependency"));
                }
                let start_day = parse_u64(fields[2], "shock start day", line_number)?;
                let end_day = parse_u64(fields[3], "shock end day", line_number)?;
                // v0.2 only admits disruptions already active at the common decision time.
                // Later-starting shocks require an event simulator; do not approximate them.
                if start_day != 0 || end_day <= start_day {
                    return Err(format!(
                        "line {line_number}: v0.2 shock must begin at day 0 and have positive duration"
                    ));
                }
                pending_shocks.push((fields[1].to_owned(), Shock { start_day, end_day }));
            }
            "OPTION" if fields.len() == 12 => {
                if fields[1].is_empty() || fields[2].is_empty() || fields[11].is_empty() {
                    return Err(format!(
                        "line {line_number}: empty option identity, region or configuration revision"
                    ));
                }
                let mut dependency_ids: Vec<String> = if fields[10] == "-" {
                    Vec::new()
                } else {
                    fields[10].split(',').map(str::to_owned).collect()
                };
                dependency_ids.sort();
                if dependency_ids.iter().any(String::is_empty) {
                    return Err(format!("line {line_number}: empty dependency in option"));
                }
                let unique: BTreeSet<&str> =
                    dependency_ids.iter().map(String::as_str).collect();
                if unique.len() != dependency_ids.len() {
                    return Err(format!(
                        "line {line_number}: duplicate dependency in option '{}'",
                        fields[1]
                    ));
                }
                let option = PortfolioOption {
                    id: fields[1].to_owned(),
                    region: fields[2].to_owned(),
                    cost_units: parse_u64(fields[3], "cost units", line_number)?,
                    production_days: parse_u64(fields[4], "production/supply days", line_number)?,
                    inspection_days: parse_u64(fields[5], "inspection days", line_number)?,
                    commissioning_days: parse_u64(fields[6], "commissioning days", line_number)?,
                    handoff_days: parse_u64(fields[7], "receiver handoff days", line_number)?,
                    emissions_units: parse_u64(fields[8], "emissions units", line_number)?,
                    regional_value_units: parse_u64(
                        fields[9],
                        "regional value units",
                        line_number,
                    )?,
                    revision: fields[11].to_owned(),
                    dependencies: dependency_ids,
                };
                if options.insert(option.id.clone(), option).is_some() {
                    return Err(format!(
                        "line {line_number}: duplicate option '{}'",
                        fields[1]
                    ));
                }
            }
            "INSPECTION" if fields.len() == 5 => {
                if fields[1].is_empty() || fields[4].is_empty() {
                    return Err(format!("line {line_number}: empty inspection option or revision"));
                }
                let inspection = Inspection {
                    status: InspectionStatus::parse(fields[2], line_number)?,
                    qualified_units: parse_u64(fields[3], "qualified units", line_number)?,
                    revision: fields[4].to_owned(),
                };
                if inspections.insert(fields[1].to_owned(), inspection).is_some() {
                    return Err(format!(
                        "line {line_number}: duplicate inspection record for '{}'",
                        fields[1]
                    ));
                }
            }
            "RECEIVER_ACCEPTANCE" if fields.len() == 5 => {
                if fields[1].is_empty() || fields[4].is_empty() {
                    return Err(format!(
                        "line {line_number}: empty receiver-acceptance option or revision"
                    ));
                }
                let receiver = ReceiverAcceptance {
                    status: ReceiverStatus::parse(fields[2], line_number)?,
                    accepted_units: parse_u64(fields[3], "receiver-accepted units", line_number)?,
                    revision: fields[4].to_owned(),
                };
                if receiver_acceptances
                    .insert(fields[1].to_owned(), receiver)
                    .is_some()
                {
                    return Err(format!(
                        "line {line_number}: duplicate receiver-acceptance record for '{}'",
                        fields[1]
                    ));
                }
            }
            record => {
                return Err(format!(
                    "line {line_number}: unknown record or wrong field count for '{record}'"
                ));
            }
        }
    }

    // Resolve shocks after parsing so record ordering is semantically irrelevant.
    for (dependency_id, shock) in pending_shocks {
        let dependency = dependencies
            .get_mut(&dependency_id)
            .ok_or_else(|| format!("shock references undeclared dependency '{dependency_id}'"))?;
        dependency.shocks.push(shock);
    }

    let (id, version) = identity.ok_or_else(|| "missing INSTANCE record".to_owned())?;
    let service = service.ok_or_else(|| "missing SERVICE record".to_owned())?;
    if options.is_empty() {
        return Err("fixture requires portfolio options".to_owned());
    }

    for option in options.values() {
        for dependency_id in &option.dependencies {
            if !dependencies.contains_key(dependency_id) {
                return Err(format!(
                    "option '{}' references undeclared dependency '{}'",
                    option.id, dependency_id
                ));
            }
        }

        let inspection = inspections
            .get(&option.id)
            .ok_or_else(|| format!("option '{}' has no inspection disposition", option.id))?;
        if inspection.revision != option.revision {
            return Err(format!(
                "option '{}' inspection revision '{}' does not match configuration revision '{}'",
                option.id, inspection.revision, option.revision
            ));
        }
        if inspection.qualified_units > service.expected_units {
            return Err(format!(
                "option '{}' qualified units exceed expected service units",
                option.id
            ));
        }
        if inspection.status != InspectionStatus::Pass && inspection.qualified_units != 0 {
            return Err(format!(
                "option '{}' has qualified units without a passing inspection",
                option.id
            ));
        }

        let receiver = receiver_acceptances.get(&option.id).ok_or_else(|| {
            format!("option '{}' has no receiver-acceptance disposition", option.id)
        })?;
        if receiver.revision != option.revision {
            return Err(format!(
                "option '{}' receiver-acceptance revision '{}' does not match configuration revision '{}'",
                option.id, receiver.revision, option.revision
            ));
        }
        if receiver.accepted_units > inspection.qualified_units {
            return Err(format!(
                "option '{}' receiver-accepted units exceed inspection-qualified units",
                option.id
            ));
        }
        if receiver.status != ReceiverStatus::Accepted && receiver.accepted_units != 0 {
            return Err(format!(
                "option '{}' has receiver-accepted units without receiver acceptance",
                option.id
            ));
        }
    }

    for option_id in inspections.keys() {
        if !options.contains_key(option_id) {
            return Err(format!("inspection references unknown option '{option_id}'"));
        }
    }
    for option_id in receiver_acceptances.keys() {
        if !options.contains_key(option_id) {
            return Err(format!(
                "receiver acceptance references unknown option '{option_id}'"
            ));
        }
    }

    Ok(Instance {
        id,
        version,
        service,
        dependencies,
        options,
        inspections,
        receiver_acceptances,
    })
}

fn effective_dependency_day(dependency: &Dependency, include_shocks: bool) -> u64 {
    let mut available = dependency.available_from_day;
    if include_shocks {
        for shock in &dependency.shocks {
            if shock.start_day == 0 {
                available = available.max(shock.end_day);
            }
        }
    }
    available
}

fn receiver_day(
    instance: &Instance,
    option: &PortfolioOption,
    include_shocks: bool,
) -> Result<u64, String> {
    let ready_day = option
        .dependencies
        .iter()
        .map(|dependency_id| {
            instance
                .dependencies
                .get(dependency_id)
                .expect("parser validates every option dependency")
        })
        .map(|dependency| effective_dependency_day(dependency, include_shocks))
        .max()
        .unwrap_or(0);

    [
        option.production_days,
        option.inspection_days,
        option.commissioning_days,
        option.handoff_days,
    ]
    .into_iter()
    .try_fold(ready_day, |day, duration| {
        day.checked_add(duration)
            .ok_or_else(|| format!("receiver-day overflow for option '{}'", option.id))
    })
}

fn evaluate(instance: &Instance) -> Result<Vec<Outcome>, String> {
    instance
        .options
        .values()
        .map(|option| {
            let receiver_day_without_shock = receiver_day(instance, option, false)?;
            let receiver_day_with_shock = receiver_day(instance, option, true)?;
            let inspection = &instance.inspections[&option.id];
            let receiver = &instance.receiver_acceptances[&option.id];
            let inspection_qualified_units = if inspection.status == InspectionStatus::Pass {
                inspection.qualified_units
            } else {
                0
            };
            let receiver_accepted_units =
                if receiver.status == ReceiverStatus::Accepted {
                    receiver.accepted_units
                } else {
                    0
                };
            let on_time_accepted_units_with_shock =
                if receiver_day_with_shock <= instance.service.deadline_day
                    && inspection.status == InspectionStatus::Pass
                    && receiver.status == ReceiverStatus::Accepted
                {
                    receiver.accepted_units
                } else {
                    0
                };

            Ok(Outcome {
                instance_id: instance.id.clone(),
                instance_version: instance.version.clone(),
                service_id: instance.service.id.clone(),
                deadline_day: instance.service.deadline_day,
                option_id: option.id.clone(),
                region: option.region.clone(),
                receiver_day_without_shock,
                receiver_day_with_shock,
                expected_units: instance.service.expected_units,
                inspection_qualified_units,
                receiver_accepted_units,
                on_time_accepted_units_with_shock,
                inspection_status: inspection.status.as_str().to_owned(),
                receiver_status: receiver.status.as_str().to_owned(),
                revision: option.revision.clone(),
                cost_units: option.cost_units,
                emissions_units: option.emissions_units,
                regional_value_units: option.regional_value_units,
            })
        })
        .collect()
}

/// Vector-valued marginal outcomes of adding one qualified portfolio option
/// relative to another option under the exact same frozen instance and obligation.
/// Negative receiver-day/cost/emissions deltas are improvements; positive service and
/// regional-value deltas are improvements. Dimensions are never collapsed into a score.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ServiceOptionValue {
    pub instance_id: String,
    pub service_id: String,
    pub baseline_option_id: String,
    pub candidate_option_id: String,
    pub receiver_day_delta: i128,
    pub inspection_qualified_units_delta: i128,
    pub receiver_accepted_units_delta: i128,
    pub on_time_accepted_units_delta: i128,
    pub cost_units_delta: i128,
    pub emissions_units_delta: i128,
    pub regional_value_units_delta: i128,
}

pub fn compare_service_options(
    baseline: &Outcome,
    candidate: &Outcome,
) -> Result<ServiceOptionValue, String> {
    if baseline.instance_id != candidate.instance_id
        || baseline.instance_version != candidate.instance_version
        || baseline.service_id != candidate.service_id
        || baseline.deadline_day != candidate.deadline_day
        || baseline.expected_units != candidate.expected_units
    {
        return Err(
            "service option comparison context does not match the frozen instance"
                .to_owned(),
        );
    }

    Ok(ServiceOptionValue {
        instance_id: baseline.instance_id.clone(),
        service_id: baseline.service_id.clone(),
        baseline_option_id: baseline.option_id.clone(),
        candidate_option_id: candidate.option_id.clone(),
        receiver_day_delta: i128::from(candidate.receiver_day_with_shock)
            - i128::from(baseline.receiver_day_with_shock),
        inspection_qualified_units_delta: i128::from(candidate.inspection_qualified_units)
            - i128::from(baseline.inspection_qualified_units),
        receiver_accepted_units_delta: i128::from(candidate.receiver_accepted_units)
            - i128::from(baseline.receiver_accepted_units),
        on_time_accepted_units_delta: i128::from(candidate.on_time_accepted_units_with_shock)
            - i128::from(baseline.on_time_accepted_units_with_shock),
        cost_units_delta: i128::from(candidate.cost_units) - i128::from(baseline.cost_units),
        emissions_units_delta: i128::from(candidate.emissions_units)
            - i128::from(baseline.emissions_units),
        regional_value_units_delta: i128::from(candidate.regional_value_units)
            - i128::from(baseline.regional_value_units),
    })
}

/// Per-scenario recovery outcome. The reference outcome may be a clairvoyant
/// upper bound; it is not an implementable policy and carries no probability weight.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoveryServiceOutcome {
    pub instance_id: String,
    pub instance_version: String,
    pub scenario_id: String,
    pub policy_id: String,
    pub service_id: String,
    pub deadline_day: u64,
    pub horizon_day: u64,
    pub expected_units: u64,
    pub inspection_qualified_units: u64,
    pub receiver_accepted_units: u64,
    pub on_time_accepted_units: u64,
    pub unresolved_units: u64,
    pub service_deficit_unit_hours: u64,
    pub time_to_minimum_viable_service_day: u64,
    pub time_to_full_service_day: u64,
    pub recovery_cost_units: u64,
    pub recovery_emissions_units: u64,
    pub requalification_actions: u64,
}

/// Candidate-minus-reference regret vector. Positive values mean the candidate
/// incurred more loss/cost/time than the reference on that dimension. This is
/// deliberately not scalarized.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoveryRegretVector {
    pub instance_id: String,
    pub instance_version: String,
    pub scenario_id: String,
    pub service_id: String,
    pub deadline_day: u64,
    pub horizon_day: u64,
    pub expected_units: u64,
    pub candidate_policy_id: String,
    pub reference_policy_id: String,
    pub unmet_units_at_deadline_delta: i128,
    pub inspection_qualified_units_delta: i128,
    pub receiver_accepted_units_delta: i128,
    pub late_accepted_units_delta: i128,
    pub unresolved_units_delta: i128,
    pub service_deficit_unit_hours_delta: i128,
    pub time_to_minimum_viable_service_day_delta: i128,
    pub time_to_full_service_day_delta: i128,
    pub recovery_cost_units_delta: i128,
    pub recovery_emissions_units_delta: i128,
    pub requalification_actions_delta: i128,
}

const RECOVERY_UNITS_ERROR: &str =
    "recovery outcome has inconsistent expected/qualified/accepted/on-time/unresolved units";
const RECOVERY_TIME_ERROR: &str =
    "recovery outcome has inconsistent minimum/full recovery times or horizon";
const RECOVERY_CONTEXT_ERROR: &str =
    "recovery regret comparison context does not match the frozen scenario and service obligation";

fn validate_recovery_outcome(outcome: &RecoveryServiceOutcome) -> Result<(), String> {
    if outcome.instance_id.is_empty()
        || outcome.instance_version.is_empty()
        || outcome.scenario_id.is_empty()
        || outcome.policy_id.is_empty()
        || outcome.service_id.is_empty()
    {
        return Err("recovery outcome has an empty identity field".to_owned());
    }
    if outcome.expected_units == 0
        || outcome.inspection_qualified_units > outcome.expected_units
        || outcome.receiver_accepted_units > outcome.inspection_qualified_units
        || outcome.on_time_accepted_units > outcome.receiver_accepted_units
        || outcome.unresolved_units > outcome.expected_units - outcome.receiver_accepted_units
    {
        return Err(RECOVERY_UNITS_ERROR.to_owned());
    }
    if outcome.time_to_minimum_viable_service_day > outcome.time_to_full_service_day
        || outcome.time_to_full_service_day > outcome.horizon_day
    {
        return Err(RECOVERY_TIME_ERROR.to_owned());
    }
    Ok(())
}

pub fn compare_recovery_regret(
    candidate: &RecoveryServiceOutcome,
    reference: &RecoveryServiceOutcome,
) -> Result<RecoveryRegretVector, String> {
    validate_recovery_outcome(candidate)?;
    validate_recovery_outcome(reference)?;
    if candidate.instance_id != reference.instance_id
        || candidate.instance_version != reference.instance_version
        || candidate.scenario_id != reference.scenario_id
        || candidate.service_id != reference.service_id
        || candidate.deadline_day != reference.deadline_day
        || candidate.horizon_day != reference.horizon_day
        || candidate.expected_units != reference.expected_units
    {
        return Err(RECOVERY_CONTEXT_ERROR.to_owned());
    }

    let candidate_late =
        candidate.receiver_accepted_units - candidate.on_time_accepted_units;
    let reference_late =
        reference.receiver_accepted_units - reference.on_time_accepted_units;
    let candidate_unmet =
        candidate.expected_units - candidate.on_time_accepted_units;
    let reference_unmet =
        reference.expected_units - reference.on_time_accepted_units;

    Ok(RecoveryRegretVector {
        instance_id: candidate.instance_id.clone(),
        instance_version: candidate.instance_version.clone(),
        scenario_id: candidate.scenario_id.clone(),
        service_id: candidate.service_id.clone(),
        deadline_day: candidate.deadline_day,
        horizon_day: candidate.horizon_day,
        expected_units: candidate.expected_units,
        candidate_policy_id: candidate.policy_id.clone(),
        reference_policy_id: reference.policy_id.clone(),
        unmet_units_at_deadline_delta: i128::from(candidate_unmet)
            - i128::from(reference_unmet),
        inspection_qualified_units_delta: i128::from(candidate.inspection_qualified_units)
            - i128::from(reference.inspection_qualified_units),
        receiver_accepted_units_delta: i128::from(candidate.receiver_accepted_units)
            - i128::from(reference.receiver_accepted_units),
        late_accepted_units_delta: i128::from(candidate_late)
            - i128::from(reference_late),
        unresolved_units_delta: i128::from(candidate.unresolved_units)
            - i128::from(reference.unresolved_units),
        service_deficit_unit_hours_delta: i128::from(candidate.service_deficit_unit_hours)
            - i128::from(reference.service_deficit_unit_hours),
        time_to_minimum_viable_service_day_delta: i128::from(
            candidate.time_to_minimum_viable_service_day,
        ) - i128::from(reference.time_to_minimum_viable_service_day),
        time_to_full_service_day_delta: i128::from(candidate.time_to_full_service_day)
            - i128::from(reference.time_to_full_service_day),
        recovery_cost_units_delta: i128::from(candidate.recovery_cost_units)
            - i128::from(reference.recovery_cost_units),
        recovery_emissions_units_delta: i128::from(candidate.recovery_emissions_units)
            - i128::from(reference.recovery_emissions_units),
        requalification_actions_delta: i128::from(candidate.requalification_actions)
            - i128::from(reference.requalification_actions),
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecoveryRegretMetric {
    UnmetUnitsAtDeadline,
    InspectionQualifiedUnits,
    ReceiverAcceptedUnits,
    LateAcceptedUnits,
    UnresolvedUnits,
    ServiceDeficitUnitHours,
    TimeToMinimumViableServiceDay,
    TimeToFullServiceDay,
    RecoveryCostUnits,
    RecoveryEmissionsUnits,
    RequalificationActions,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorstCaseRecoveryRegret {
    pub metric: RecoveryRegretMetric,
    pub delta: i128,
    pub scenario_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoveryRegretEnvelope {
    pub instance_id: String,
    pub instance_version: String,
    pub service_id: String,
    pub deadline_day: u64,
    pub horizon_day: u64,
    pub expected_units: u64,
    pub candidate_policy_id: String,
    pub reference_policy_id: String,
    pub scenario_count: usize,
    pub worst_case: Vec<WorstCaseRecoveryRegret>,
}

/// Builds a probability-free component-wise worst-case regret envelope over a
/// frozen finite scenario set. Lower qualification/acceptance is adverse; higher
/// loss, delay, cost and recovery burden is adverse. The dimensions remain separate.
pub fn worst_case_recovery_regret(
    vectors: &[RecoveryRegretVector],
) -> Result<RecoveryRegretEnvelope, String> {
    let first = vectors
        .first()
        .ok_or_else(|| "recovery regret envelope requires at least one scenario".to_owned())?;

    let mut seen = BTreeSet::new();
    for vector in vectors {
        if vector.instance_id != first.instance_id
            || vector.instance_version != first.instance_version
            || vector.service_id != first.service_id
            || vector.deadline_day != first.deadline_day
            || vector.horizon_day != first.horizon_day
            || vector.expected_units != first.expected_units
            || vector.candidate_policy_id != first.candidate_policy_id
            || vector.reference_policy_id != first.reference_policy_id
        {
            return Err(
                "recovery regret envelope contains mismatched policy or service context".to_owned(),
            );
        }
        if !seen.insert(vector.scenario_id.as_str()) {
            return Err(format!(
                "recovery regret envelope contains duplicate scenario '{}'",
                vector.scenario_id
            ));
        }
    }

    let mut worst_case = vec![
        WorstCaseRecoveryRegret {
            metric: RecoveryRegretMetric::UnmetUnitsAtDeadline,
            delta: first.unmet_units_at_deadline_delta,
            scenario_id: first.scenario_id.clone(),
        },
        WorstCaseRecoveryRegret {
            metric: RecoveryRegretMetric::InspectionQualifiedUnits,
            delta: first.inspection_qualified_units_delta,
            scenario_id: first.scenario_id.clone(),
        },
        WorstCaseRecoveryRegret {
            metric: RecoveryRegretMetric::ReceiverAcceptedUnits,
            delta: first.receiver_accepted_units_delta,
            scenario_id: first.scenario_id.clone(),
        },
        WorstCaseRecoveryRegret {
            metric: RecoveryRegretMetric::LateAcceptedUnits,
            delta: first.late_accepted_units_delta,
            scenario_id: first.scenario_id.clone(),
        },
        WorstCaseRecoveryRegret {
            metric: RecoveryRegretMetric::UnresolvedUnits,
            delta: first.unresolved_units_delta,
            scenario_id: first.scenario_id.clone(),
        },
        WorstCaseRecoveryRegret {
            metric: RecoveryRegretMetric::ServiceDeficitUnitHours,
            delta: first.service_deficit_unit_hours_delta,
            scenario_id: first.scenario_id.clone(),
        },
        WorstCaseRecoveryRegret {
            metric: RecoveryRegretMetric::TimeToMinimumViableServiceDay,
            delta: first.time_to_minimum_viable_service_day_delta,
            scenario_id: first.scenario_id.clone(),
        },
        WorstCaseRecoveryRegret {
            metric: RecoveryRegretMetric::TimeToFullServiceDay,
            delta: first.time_to_full_service_day_delta,
            scenario_id: first.scenario_id.clone(),
        },
        WorstCaseRecoveryRegret {
            metric: RecoveryRegretMetric::RecoveryCostUnits,
            delta: first.recovery_cost_units_delta,
            scenario_id: first.scenario_id.clone(),
        },
        WorstCaseRecoveryRegret {
            metric: RecoveryRegretMetric::RecoveryEmissionsUnits,
            delta: first.recovery_emissions_units_delta,
            scenario_id: first.scenario_id.clone(),
        },
        WorstCaseRecoveryRegret {
            metric: RecoveryRegretMetric::RequalificationActions,
            delta: first.requalification_actions_delta,
            scenario_id: first.scenario_id.clone(),
        },
    ];

    for vector in vectors.iter().skip(1) {
        for bound in &mut worst_case {
            let next_delta = match bound.metric {
                RecoveryRegretMetric::UnmetUnitsAtDeadline => {
                    vector.unmet_units_at_deadline_delta
                }
                RecoveryRegretMetric::InspectionQualifiedUnits => {
                    vector.inspection_qualified_units_delta
                }
                RecoveryRegretMetric::ReceiverAcceptedUnits => {
                    vector.receiver_accepted_units_delta
                }
                RecoveryRegretMetric::LateAcceptedUnits => vector.late_accepted_units_delta,
                RecoveryRegretMetric::UnresolvedUnits => vector.unresolved_units_delta,
                RecoveryRegretMetric::ServiceDeficitUnitHours => {
                    vector.service_deficit_unit_hours_delta
                }
                RecoveryRegretMetric::TimeToMinimumViableServiceDay => {
                    vector.time_to_minimum_viable_service_day_delta
                }
                RecoveryRegretMetric::TimeToFullServiceDay => {
                    vector.time_to_full_service_day_delta
                }
                RecoveryRegretMetric::RecoveryCostUnits => vector.recovery_cost_units_delta,
                RecoveryRegretMetric::RecoveryEmissionsUnits => {
                    vector.recovery_emissions_units_delta
                }
                RecoveryRegretMetric::RequalificationActions => {
                    vector.requalification_actions_delta
                }
            };
            let candidate_is_worse = match bound.metric {
                RecoveryRegretMetric::InspectionQualifiedUnits
                | RecoveryRegretMetric::ReceiverAcceptedUnits => next_delta < bound.delta,
                _ => next_delta > bound.delta,
            };
            if candidate_is_worse {
                bound.delta = next_delta;
                bound.scenario_id.clone_from(&vector.scenario_id);
            }
        }
    }

    Ok(RecoveryRegretEnvelope {
        instance_id: first.instance_id.clone(),
        instance_version: first.instance_version.clone(),
        service_id: first.service_id.clone(),
        deadline_day: first.deadline_day,
        horizon_day: first.horizon_day,
        expected_units: first.expected_units,
        candidate_policy_id: first.candidate_policy_id.clone(),
        reference_policy_id: first.reference_policy_id.clone(),
        scenario_count: vectors.len(),
        worst_case,
    })
}

fn render_receipt(instance: &Instance, outcomes: &[Outcome]) -> String {
    let mut lines = vec![
        "MCSC_RECEIPT\t0.2".to_owned(),
        format!("INSTANCE\t{}\t{}", instance.id, instance.version),
        format!(
            "SERVICE\t{}\t{}\t{}\t{}",
            instance.service.id,
            instance.service.expected_units,
            instance.service.unit,
            instance.service.deadline_day
        ),
    ];
    for (dependency_id, dependency) in &instance.dependencies {
        lines.push(format!(
            "DEPENDENCY\t{}\t{}",
            dependency_id, dependency.available_from_day
        ));
        for shock in &dependency.shocks {
            lines.push(format!(
                "SHOCK\t{}\t{}\t{}",
                dependency_id, shock.start_day, shock.end_day
            ));
        }
    }
    for option in instance.options.values() {
        let dependencies = if option.dependencies.is_empty() {
            "-".to_owned()
        } else {
            option.dependencies.join(",")
        };
        lines.push(format!(
            "OPTION\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            option.id,
            option.region,
            option.cost_units,
            option.production_days,
            option.inspection_days,
            option.commissioning_days,
            option.handoff_days,
            option.emissions_units,
            option.regional_value_units,
            dependencies,
            option.revision
        ));
        let inspection = &instance.inspections[&option.id];
        lines.push(format!(
            "INSPECTION\t{}\t{}\t{}\t{}",
            option.id,
            inspection.status.as_str(),
            inspection.qualified_units,
            inspection.revision
        ));
        let receiver = &instance.receiver_acceptances[&option.id];
        lines.push(format!(
            "RECEIVER_ACCEPTANCE\t{}\t{}\t{}\t{}",
            option.id,
            receiver.status.as_str(),
            receiver.accepted_units,
            receiver.revision
        ));
    }
    for outcome in outcomes {
        lines.push(format!(
            "OUTCOME\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            outcome.option_id,
            outcome.region,
            outcome.receiver_day_without_shock,
            outcome.receiver_day_with_shock,
            outcome.expected_units,
            outcome.inspection_qualified_units,
            outcome.receiver_accepted_units,
            outcome.on_time_accepted_units_with_shock,
            outcome.inspection_status,
            outcome.receiver_status,
            outcome.revision,
            outcome.cost_units,
            outcome.emissions_units,
            outcome.regional_value_units
        ));
    }
    lines.join("\n") + "\n"
}

pub fn run_audit(input: &str) -> Result<String, String> {
    let instance = parse_instance(input)?;
    let outcomes = evaluate(&instance)?;
    Ok(render_receipt(&instance, &outcomes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frozen_microcase_matches_all_five_expected_outcomes() {
        let instance = parse_instance(FIXTURE).expect("fixture parses");
        let outcomes = evaluate(&instance).expect("evaluation succeeds");
        let expected = [
            ("EXISTING", 23, 23, 10, 10, 0, 90, 45, 0),
            ("IMPORT", 13, 13, 10, 10, 0, 100, 40, 5),
            ("LOCAL-A", 8, 14, 10, 10, 0, 130, 30, 80),
            ("LOCAL-B", 9, 15, 10, 10, 0, 125, 35, 70),
            ("LOCAL-C", 10, 10, 10, 10, 10, 145, 32, 60),
        ];
        assert_eq!(outcomes.len(), expected.len());
        for (
            outcome,
            (id, normal, disrupted, qualified, accepted, on_time, cost, emissions, regional),
        ) in outcomes.iter().zip(expected)
        {
            assert_eq!(outcome.option_id, id);
            assert_eq!(outcome.receiver_day_without_shock, normal);
            assert_eq!(outcome.receiver_day_with_shock, disrupted);
            assert_eq!(outcome.expected_units, 10);
            assert_eq!(outcome.inspection_qualified_units, qualified);
            assert_eq!(outcome.receiver_accepted_units, accepted);
            assert_eq!(outcome.on_time_accepted_units_with_shock, on_time);
            assert_eq!(outcome.inspection_status, "PASS");
            assert_eq!(outcome.receiver_status, "ACCEPTED");
            assert_eq!(outcome.revision, "REV-001");
            assert_eq!(outcome.cost_units, cost);
            assert_eq!(outcome.emissions_units, emissions);
            assert_eq!(outcome.regional_value_units, regional);
        }
    }

    #[test]
    fn late_outcome_keeps_expected_qualified_and_accepted_denominators() {
        let instance = parse_instance(FIXTURE).expect("fixture parses");
        let import = evaluate(&instance)
            .expect("evaluation succeeds")
            .into_iter()
            .find(|outcome| outcome.option_id == "IMPORT")
            .expect("import option exists");
        assert_eq!(import.expected_units, 10);
        assert_eq!(import.inspection_qualified_units, 10);
        assert_eq!(import.receiver_accepted_units, 10);
        assert_eq!(import.on_time_accepted_units_with_shock, 0);
    }

    #[test]
    fn input_record_order_does_not_change_the_receipt() {
        let reversed = FIXTURE.lines().rev().collect::<Vec<_>>().join("\n");
        assert_eq!(run_audit(FIXTURE).unwrap(), run_audit(&reversed).unwrap());
    }

    #[test]
    fn unknown_dependency_fails_closed() {
        let changed = FIXTURE.replace("MILL-01,GRID-A,NDT-01", "MILL-UNKNOWN,GRID-A,NDT-01");
        assert!(
            parse_instance(&changed)
                .unwrap_err()
                .contains("undeclared dependency")
        );
    }

    #[test]
    fn duplicate_dependency_fails_closed() {
        let changed = FIXTURE.replace("MILL-01,GRID-A,NDT-01", "MILL-01,MILL-01,NDT-01");
        assert!(
            parse_instance(&changed)
                .unwrap_err()
                .contains("duplicate dependency")
        );
    }

    #[test]
    fn partial_receiver_acceptance_never_expands_to_qualified_units() {
        let changed = FIXTURE.replace(
            "RECEIVER_ACCEPTANCE\tLOCAL-C\tACCEPTED\t10\tREV-001",
            "RECEIVER_ACCEPTANCE\tLOCAL-C\tACCEPTED\t6\tREV-001",
        );
        let local_c = evaluate(&parse_instance(&changed).unwrap())
            .unwrap()
            .into_iter()
            .find(|outcome| outcome.option_id == "LOCAL-C")
            .unwrap();
        assert_eq!(local_c.expected_units, 10);
        assert_eq!(local_c.inspection_qualified_units, 10);
        assert_eq!(local_c.receiver_accepted_units, 6);
        assert_eq!(local_c.on_time_accepted_units_with_shock, 6);
    }

    #[test]
    fn failed_or_unknown_inspection_cannot_earn_service_credit() {
        for (status, receiver_status) in [("FAIL", "UNKNOWN"), ("UNKNOWN", "UNKNOWN")] {
            let changed = FIXTURE
                .replace(
                    "INSPECTION\tLOCAL-C\tPASS\t10\tREV-001",
                    &format!("INSPECTION\tLOCAL-C\t{status}\t0\tREV-001"),
                )
                .replace(
                    "RECEIVER_ACCEPTANCE\tLOCAL-C\tACCEPTED\t10\tREV-001",
                    &format!("RECEIVER_ACCEPTANCE\tLOCAL-C\t{receiver_status}\t0\tREV-001"),
                );
            let local_c = evaluate(&parse_instance(&changed).unwrap())
                .unwrap()
                .into_iter()
                .find(|outcome| outcome.option_id == "LOCAL-C")
                .unwrap();
            assert_eq!(local_c.expected_units, 10);
            assert_eq!(local_c.inspection_qualified_units, 0);
            assert_eq!(local_c.receiver_accepted_units, 0);
            assert_eq!(local_c.on_time_accepted_units_with_shock, 0);
        }
    }

    #[test]
    fn unknown_or_rejected_receiver_cannot_earn_service_credit() {
        for status in ["UNKNOWN", "REJECTED"] {
            let changed = FIXTURE.replace(
                "RECEIVER_ACCEPTANCE\tLOCAL-C\tACCEPTED\t10\tREV-001",
                &format!("RECEIVER_ACCEPTANCE\tLOCAL-C\t{status}\t0\tREV-001"),
            );
            let local_c = evaluate(&parse_instance(&changed).unwrap())
                .unwrap()
                .into_iter()
                .find(|outcome| outcome.option_id == "LOCAL-C")
                .unwrap();
            assert_eq!(local_c.inspection_qualified_units, 10);
            assert_eq!(local_c.receiver_accepted_units, 0);
            assert_eq!(local_c.on_time_accepted_units_with_shock, 0);
        }
    }

    #[test]
    fn inspection_revision_mismatch_fails_closed() {
        let changed = FIXTURE.replace(
            "INSPECTION\tLOCAL-C\tPASS\t10\tREV-001",
            "INSPECTION\tLOCAL-C\tPASS\t10\tREV-002",
        );
        assert!(
            parse_instance(&changed)
                .unwrap_err()
                .contains("inspection revision")
        );
    }

    #[test]
    fn receiver_revision_mismatch_fails_closed() {
        let changed = FIXTURE.replace(
            "RECEIVER_ACCEPTANCE\tLOCAL-C\tACCEPTED\t10\tREV-001",
            "RECEIVER_ACCEPTANCE\tLOCAL-C\tACCEPTED\t10\tREV-002",
        );
        assert!(
            parse_instance(&changed)
                .unwrap_err()
                .contains("receiver-acceptance revision")
        );
    }

    #[test]
    fn missing_or_duplicate_inspection_fails_closed() {
        let missing = FIXTURE.replace("INSPECTION\tLOCAL-C\tPASS\t10\tREV-001\n", "");
        assert!(
            parse_instance(&missing)
                .unwrap_err()
                .contains("no inspection disposition")
        );

        let duplicate = FIXTURE.replace(
            "INSPECTION\tLOCAL-C\tPASS\t10\tREV-001",
            "INSPECTION\tLOCAL-C\tPASS\t10\tREV-001\nINSPECTION\tLOCAL-C\tPASS\t10\tREV-001",
        );
        assert!(
            parse_instance(&duplicate)
                .unwrap_err()
                .contains("duplicate inspection record")
        );
    }

    #[test]
    fn missing_or_duplicate_receiver_acceptance_fails_closed() {
        let missing =
            FIXTURE.replace("RECEIVER_ACCEPTANCE\tLOCAL-C\tACCEPTED\t10\tREV-001\n", "");
        assert!(
            parse_instance(&missing)
                .unwrap_err()
                .contains("no receiver-acceptance disposition")
        );

        let duplicate = FIXTURE.replace(
            "RECEIVER_ACCEPTANCE\tLOCAL-C\tACCEPTED\t10\tREV-001",
            "RECEIVER_ACCEPTANCE\tLOCAL-C\tACCEPTED\t10\tREV-001\nRECEIVER_ACCEPTANCE\tLOCAL-C\tACCEPTED\t10\tREV-001",
        );
        assert!(
            parse_instance(&duplicate)
                .unwrap_err()
                .contains("duplicate receiver-acceptance record")
        );
    }

    #[test]
    fn inspection_cannot_qualify_more_than_expected_units() {
        let changed = FIXTURE.replace(
            "INSPECTION\tLOCAL-C\tPASS\t10\tREV-001",
            "INSPECTION\tLOCAL-C\tPASS\t11\tREV-001",
        );
        assert!(
            parse_instance(&changed)
                .unwrap_err()
                .contains("qualified units exceed expected")
        );
    }

    #[test]
    fn receiver_cannot_accept_more_than_inspection_qualified_units() {
        let changed = FIXTURE.replace(
            "RECEIVER_ACCEPTANCE\tLOCAL-C\tACCEPTED\t10\tREV-001",
            "RECEIVER_ACCEPTANCE\tLOCAL-C\tACCEPTED\t11\tREV-001",
        );
        assert!(
            parse_instance(&changed)
                .unwrap_err()
                .contains("receiver-accepted units exceed inspection-qualified units")
        );
    }

    #[test]
    fn nonaccepted_receiver_status_cannot_report_accepted_units() {
        let changed = FIXTURE.replace(
            "RECEIVER_ACCEPTANCE\tLOCAL-C\tACCEPTED\t10\tREV-001",
            "RECEIVER_ACCEPTANCE\tLOCAL-C\tUNKNOWN\t1\tREV-001",
        );
        assert!(
            parse_instance(&changed)
                .unwrap_err()
                .contains("without receiver acceptance")
        );
    }

    #[test]
    fn nonzero_start_shock_is_not_approximated() {
        let changed = FIXTURE.replace("SHOCK\tGRID-A\t0\t6", "SHOCK\tGRID-A\t2\t6");
        assert!(
            parse_instance(&changed)
                .unwrap_err()
                .contains("must begin at day 0")
        );
    }

    #[test]
    fn capability_receipt_can_supply_the_only_admitted_plan_evidence() {
        let fixture = concat!(
            "INSTANCE\tMCSC-NO-DEPENDENCY\tv0.4\n",
            "SERVICE\tS\t10\tcargo_unit\t10\n",
            "OPTION\tO\tIMPORT\t1\t1\t1\t1\t1\t1\t0\t-\tREV-001\n",
            "INSPECTION\tO\tPASS\t10\tREV-001\n",
            "RECEIVER_ACCEPTANCE\tO\tACCEPTED\t10\tREV-001\n",
        );
        let receipt = run_audit(fixture)
            .expect("an option can be sourced through an external capability receipt");
        assert!(receipt.contains(
            "OUTCOME\tO\tIMPORT\t4\t4\t10\t10\t10\t10"
        ));
    }

    #[test]
    fn service_option_value_reports_each_tradeoff_without_scalarization() {
        let instance = parse_instance(FIXTURE).unwrap();
        let outcomes = evaluate(&instance).unwrap();
        let existing = outcomes
            .iter()
            .find(|outcome| outcome.option_id == "EXISTING")
            .unwrap();
        let local_c = outcomes
            .iter()
            .find(|outcome| outcome.option_id == "LOCAL-C")
            .unwrap();

        let value = compare_service_options(existing, local_c).unwrap();
        assert_eq!(value.instance_id, "MCSC-TINY-001");
        assert_eq!(value.service_id, "SERVICE-001");
        assert_eq!(value.receiver_day_delta, -13);
        assert_eq!(value.inspection_qualified_units_delta, 0);
        assert_eq!(value.receiver_accepted_units_delta, 0);
        assert_eq!(value.on_time_accepted_units_delta, 10);
        assert_eq!(value.cost_units_delta, 55);
        assert_eq!(value.emissions_units_delta, -13);
        assert_eq!(value.regional_value_units_delta, 60);

        // Partial receiver acceptance changes accepted quantity independently from
        // inspection qualification and from on-time delivery.
        let partial_fixture = FIXTURE.replace(
            "RECEIVER_ACCEPTANCE\tLOCAL-C\tACCEPTED\t10\tREV-001",
            "RECEIVER_ACCEPTANCE\tLOCAL-C\tACCEPTED\t6\tREV-001",
        );
        let partial_instance = parse_instance(&partial_fixture).unwrap();
        let partial_outcomes = evaluate(&partial_instance).unwrap();
        let partial_local_c = partial_outcomes
            .iter()
            .find(|outcome| outcome.option_id == "LOCAL-C")
            .unwrap();
        let partial_value = compare_service_options(existing, partial_local_c).unwrap();
        assert_eq!(partial_value.inspection_qualified_units_delta, 0);
        assert_eq!(partial_value.receiver_accepted_units_delta, -4);
        assert_eq!(partial_value.on_time_accepted_units_delta, 6);
    }

    #[test]
    fn service_option_value_rejects_different_instances_or_service_denominators() {
        let original = parse_instance(FIXTURE).unwrap();
        let original_outcomes = evaluate(&original).unwrap();
        let baseline = original_outcomes
            .iter()
            .find(|outcome| outcome.option_id == "IMPORT")
            .unwrap();

        let changed_units = FIXTURE.replace(
            "SERVICE-001\t10\tcargo_unit",
            "SERVICE-001\t11\tcargo_unit",
        );
        let changed = parse_instance(&changed_units).unwrap();
        let changed_outcomes = evaluate(&changed).unwrap();
        assert!(
            compare_service_options(
                baseline,
                changed_outcomes
                    .iter()
                    .find(|outcome| outcome.option_id == "IMPORT")
                    .unwrap()
            )
            .unwrap_err()
            .contains("same frozen instance")
        );

        let changed_deadline = FIXTURE.replace(
            "SERVICE-001\t10\tcargo_unit\t10",
            "SERVICE-001\t10\tcargo_unit\t11",
        );

        let changed_version =
            FIXTURE.replace("MCSC-TINY-001\tv0.1", "MCSC-TINY-001\tv0.2");
        let changed = parse_instance(&changed_version).unwrap();
        let changed_outcomes = evaluate(&changed).unwrap();
        assert!(
            compare_service_options(
                baseline,
                changed_outcomes
                    .iter()
                    .find(|outcome| outcome.option_id == "IMPORT")
                    .unwrap()
            )
            .unwrap_err()
            .contains("same frozen instance")
        );
        let changed = parse_instance(&changed_deadline).unwrap();
        let changed_outcomes = evaluate(&changed).unwrap();
        assert!(
            compare_service_options(
                baseline,
                changed_outcomes
                    .iter()
                    .find(|outcome| outcome.option_id == "IMPORT")
                    .unwrap()
            )
            .unwrap_err()
            .contains("same frozen instance")
        );
    }

    fn recovery_outcome(policy_id: &str) -> RecoveryServiceOutcome {
        RecoveryServiceOutcome {
            instance_id: "MCSC-RECOVERY-001".to_owned(),
            instance_version: "v0.1".to_owned(),
            scenario_id: "SHARED-FEEDER-AND-MILL".to_owned(),
            policy_id: policy_id.to_owned(),
            service_id: "ESSENTIAL-CARGO".to_owned(),
            deadline_day: 10,
            horizon_day: 20,
            expected_units: 100,
            inspection_qualified_units: 90,
            receiver_accepted_units: 80,
            on_time_accepted_units: 60,
            unresolved_units: 10,
            service_deficit_unit_hours: 1_200,
            time_to_minimum_viable_service_day: 3,
            time_to_full_service_day: 8,
            recovery_cost_units: 500,
            recovery_emissions_units: 300,
            requalification_actions: 2,
        }
    }

    #[test]
    fn recovery_regret_is_a_vector_against_the_same_scenario_oracle() {
        let candidate = recovery_outcome("ROBUST-POLICY");
        let mut oracle = recovery_outcome("CLAIRVOYANT-BOUND");
        oracle.inspection_qualified_units = 90;
        oracle.receiver_accepted_units = 90;
        oracle.on_time_accepted_units = 85;
        oracle.unresolved_units = 5;
        oracle.service_deficit_unit_hours = 300;
        oracle.time_to_minimum_viable_service_day = 2;
        oracle.time_to_full_service_day = 4;
        oracle.recovery_cost_units = 400;
        oracle.recovery_emissions_units = 250;
        oracle.requalification_actions = 1;

        let regret = compare_recovery_regret(&candidate, &oracle).unwrap();
        assert_eq!(regret.unmet_units_at_deadline_delta, 25);
        assert_eq!(regret.inspection_qualified_units_delta, 0);
        assert_eq!(regret.receiver_accepted_units_delta, -10);
        assert_eq!(regret.late_accepted_units_delta, 15);
        assert_eq!(regret.unresolved_units_delta, 5);
        assert_eq!(regret.service_deficit_unit_hours_delta, 900);
        assert_eq!(regret.time_to_minimum_viable_service_day_delta, 1);
        assert_eq!(regret.time_to_full_service_day_delta, 4);
        assert_eq!(regret.recovery_cost_units_delta, 100);
        assert_eq!(regret.recovery_emissions_units_delta, 50);
        assert_eq!(regret.requalification_actions_delta, 1);
    }

    #[test]
    fn recovery_regret_rejects_mismatched_scenario_version_deadline_and_denominator() {
        let candidate = recovery_outcome("ROBUST-POLICY");
        for mutation in 0..4 {
            let mut reference = recovery_outcome("CLAIRVOYANT-BOUND");
            match mutation {
                0 => reference.instance_version = "v0.2".to_owned(),
                1 => reference.scenario_id = "DIFFERENT-SCENARIO".to_owned(),
                2 => reference.deadline_day = 11,
                3 => reference.expected_units = 101,
                _ => unreachable!(),
            }
            assert!(
                compare_recovery_regret(&candidate, &reference)
                    .unwrap_err()
                    .contains("context does not match")
            );
        }
    }

    #[test]
    fn recovery_regret_rejects_inconsistent_service_and_recovery_inputs() {
        let mut invalid_units = recovery_outcome("INVALID");
        invalid_units.on_time_accepted_units = 81;
        assert!(
            compare_recovery_regret(&invalid_units, &recovery_outcome("ORACLE"))
                .unwrap_err()
                .contains("inconsistent expected/accepted")
        );

        let mut invalid_qualification = recovery_outcome("INVALID-QUALIFICATION");
        invalid_qualification.receiver_accepted_units = 91;
        assert!(
            compare_recovery_regret(
                &invalid_qualification,
                &recovery_outcome("ORACLE")
            )
            .unwrap_err()
            .contains("expected/qualified/accepted")
        );

        let mut invalid_time = recovery_outcome("INVALID-TIME");
        invalid_time.time_to_full_service_day = 21;
        assert!(
            compare_recovery_regret(&invalid_time, &recovery_outcome("ORACLE"))
                .unwrap_err()
                .contains("recovery times or horizon")
        );
    }

    #[test]
    fn identical_candidate_and_reference_have_zero_vector_regret() {
        let candidate = recovery_outcome("POLICY-A");
        let reference = recovery_outcome("ORACLE-A");
        let regret = compare_recovery_regret(&candidate, &reference).unwrap();
        assert_eq!(regret.unmet_units_at_deadline_delta, 0);
        assert_eq!(regret.inspection_qualified_units_delta, 0);
        assert_eq!(regret.receiver_accepted_units_delta, 0);
        assert_eq!(regret.late_accepted_units_delta, 0);
        assert_eq!(regret.unresolved_units_delta, 0);
        assert_eq!(regret.service_deficit_unit_hours_delta, 0);
        assert_eq!(regret.time_to_minimum_viable_service_day_delta, 0);
        assert_eq!(regret.time_to_full_service_day_delta, 0);
        assert_eq!(regret.recovery_cost_units_delta, 0);
        assert_eq!(regret.recovery_emissions_units_delta, 0);
        assert_eq!(regret.requalification_actions_delta, 0);
    }

    #[test]
    fn receiver_day_overflow_is_reported_instead_of_wrapping() {
        let changed = FIXTURE.replace(
            "OPTION\tEXISTING\tEXISTING\t90\t20",
            "OPTION\tEXISTING\tEXISTING\t90\t18446744073709551615",
        );
        let instance = parse_instance(&changed).unwrap();
        assert!(
            evaluate(&instance)
                .unwrap_err()
                .contains("receiver-day overflow")
        );
    }
}
