// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Exact finite-scenario non-anticipative mission-replenishment policies.
//! All scenario values are caller-supplied. No probabilities or controllers exist here.

use std::collections::{BTreeMap, BTreeSet};
use std::{env, fs, process::ExitCode};

const HEADER: [&str; 30] = [
    "case_id",
    "evidence_class",
    "deadline_slots",
    "capacity_mwh",
    "initial_battery_mwh",
    "reserve_mwh",
    "site_energy_plane_id",
    "battery_energy_plane_id",
    "start_node",
    "observation_node",
    "terminal_node",
    "scenario_id",
    "observation_label",
    "stage",
    "action_id",
    "from_node",
    "to_node",
    "is_terminal",
    "edge_kind",
    "mode_id",
    "available",
    "duration_slots",
    "battery_gain_mwh",
    "aircraft_draw_mwh",
    "site_energy_mwh",
    "asset_id",
    "installed_mass_g",
    "observation_source_receipt_id",
    "observation_source_receipt_sha256",
    "observation_partition_sha256",
];

const MAX_SCENARIOS: usize = 64;
const MAX_ACTIONS_PER_STAGE: usize = 32;
const MAX_POLICIES: usize = 100_000;

#[derive(Clone, Debug, PartialEq, Eq)]
struct Metadata {
    case_id: String,
    evidence_class: String,
    deadline_slots: u64,
    capacity_mwh: u64,
    initial_battery_mwh: u64,
    reserve_mwh: u64,
    site_energy_plane_id: String,
    battery_energy_plane_id: String,
    start_node: String,
    observation_node: String,
    terminal_node: String,
    observation_source_receipt_id: String,
    observation_source_receipt_sha256: String,
    observation_partition_sha256: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Edge {
    scenario_id: String,
    observation_label: String,
    stage: u8,
    action_id: String,
    from_node: String,
    to_node: String,
    is_terminal: bool,
    edge_kind: String,
    mode_id: String,
    available: bool,
    duration_slots: u64,
    battery_gain_mwh: u64,
    aircraft_draw_mwh: u64,
    site_energy_mwh: u64,
    asset_id: String,
    installed_mass_g: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Scenario {
    scenario_id: String,
    observation_label: String,
    stage0: BTreeMap<String, Edge>,
    stage1: BTreeMap<String, Edge>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Case {
    meta: Metadata,
    scenarios: BTreeMap<String, Scenario>,
    stage0_action_ids: BTreeSet<String>,
    stage1_action_ids: BTreeSet<String>,
    asset_masses: BTreeMap<String, u64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum InformationStructure {
    OpenLoopRobust,
    ObservationRecourse,
    PerfectInformation,
}

impl InformationStructure {
    fn label(self) -> &'static str {
        match self {
            Self::OpenLoopRobust => "OPEN_LOOP_ROBUST",
            Self::ObservationRecourse => "OBSERVATION_RECOURSE",
            Self::PerfectInformation => "PERFECT_INFORMATION_NON_DEPLOYABLE_UPPER_BOUND",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ScenarioResult {
    scenario_id: String,
    observation_label: String,
    stage0_action_id: String,
    stage1_action_id: String,
    elapsed_slots: u64,
    site_energy_mwh: u64,
    aircraft_draw_mwh: u64,
    terminal_battery_mwh: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Policy {
    information_structure: InformationStructure,
    stage0_action_id: String,
    stage1_decisions: BTreeMap<String, String>,
    worst_case_elapsed_slots: u64,
    worst_case_site_energy_mwh: u64,
    worst_case_aircraft_draw_mwh: u64,
    minimum_terminal_battery_mwh: u64,
    installed_mass_g: u64,
    scenarios: Vec<ScenarioResult>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct StructureCounts {
    candidate_policies: usize,
    feasible_policies: usize,
    rejected_policies: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Evaluation {
    policies: BTreeMap<InformationStructure, Vec<Policy>>,
    counts: BTreeMap<InformationStructure, StructureCounts>,
}

fn valid_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value.trim() == value
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._:-".contains(&byte))
}

fn parse_u64(value: &str, field: &str) -> Result<u64, String> {
    if value.is_empty() || value.trim() != value {
        return Err(format!("{field}: empty or untrimmed integer"));
    }
    value
        .parse::<u64>()
        .map_err(|_| format!("{field}: expected non-negative integer"))
}

fn parse_bool(value: &str, field: &str) -> Result<bool, String> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(format!("{field}: expected true or false")),
    }
}

fn parse_case(input: &str) -> Result<Case, Vec<String>> {
    let mut lines = input.split_terminator('\n');
    let Some(raw_header) = lines.next() else {
        return Err(vec!["input is empty".to_string()]);
    };
    let header = raw_header.strip_suffix('\r').unwrap_or(raw_header);
    if header.split('\t').collect::<Vec<_>>().as_slice() != HEADER.as_slice() {
        return Err(vec![format!(
            "header must contain exactly {} columns",
            HEADER.len()
        )]);
    }

    let mut metadata: Option<Vec<String>> = None;
    let mut parsed_meta: Option<Metadata> = None;
    let mut scenarios = BTreeMap::<String, Scenario>::new();
    let mut edge_keys = BTreeSet::<(String, u8, String)>::new();
    let mut asset_masses = BTreeMap::<String, u64>::new();
    let mut errors = Vec::new();

    for (offset, raw_line) in lines.enumerate() {
        let line_no = offset + 2;
        let line = raw_line.strip_suffix('\r').unwrap_or(raw_line);
        if line.is_empty() {
            errors.push(format!("line {line_no}: blank rows are not allowed"));
            continue;
        }
        let fields: Vec<&str> = line.split('\t').collect();
        if fields.len() != HEADER.len() {
            errors.push(format!(
                "line {line_no}: expected {} columns, found {}",
                HEADER.len(),
                fields.len()
            ));
            continue;
        }

        let mut row_metadata: Vec<String> =
            fields[..11].iter().map(|field| (*field).to_string()).collect();
        row_metadata.extend(fields[27..30].iter().map(|field| (*field).to_string()));
        if let Some(first) = &metadata {
            if *first != row_metadata {
                errors.push(format!("line {line_no}: case metadata differs from first row"));
                continue;
            }
        } else {
            metadata = Some(row_metadata);
            let numbers = (2..=5)
                .map(|index| parse_u64(fields[index], &format!("column {}", index + 1)))
                .collect::<Result<Vec<_>, _>>();
            let numbers = match numbers {
                Ok(values) => values,
                Err(error) => {
                    errors.push(format!("line {line_no}: {error}"));
                    continue;
                }
            };
            let candidate = Metadata {
                case_id: fields[0].to_string(),
                evidence_class: fields[1].to_string(),
                deadline_slots: numbers[0],
                capacity_mwh: numbers[1],
                initial_battery_mwh: numbers[2],
                reserve_mwh: numbers[3],
                site_energy_plane_id: fields[6].to_string(),
                battery_energy_plane_id: fields[7].to_string(),
                start_node: fields[8].to_string(),
                observation_node: fields[9].to_string(),
                terminal_node: fields[10].to_string(),
                observation_source_receipt_id: fields[27].to_string(),
                observation_source_receipt_sha256: fields[28].to_string(),
                observation_partition_sha256: fields[29].to_string(),
            };
            if !valid_id(&candidate.case_id)
                || candidate.evidence_class != "SYNTHETIC_KNOWN_TRUTH"
                || candidate.deadline_slots == 0
                || candidate.capacity_mwh == 0
                || candidate.initial_battery_mwh > candidate.capacity_mwh
                || candidate.reserve_mwh > candidate.initial_battery_mwh
                || !valid_id(&candidate.site_energy_plane_id)
                || !valid_id(&candidate.battery_energy_plane_id)
                || candidate.battery_energy_plane_id != "BATTERY_TERMINAL"
                || !valid_id(&candidate.start_node)
                || !valid_id(&candidate.observation_node)
                || !valid_id(&candidate.terminal_node)
                || !valid_id(&candidate.observation_source_receipt_id)
                || candidate.observation_source_receipt_sha256.len() != 64
                || !candidate.observation_source_receipt_sha256.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
                || candidate.observation_partition_sha256.len() != 64
                || !candidate.observation_partition_sha256.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
                || candidate.start_node == candidate.observation_node
                || candidate.observation_node == candidate.terminal_node
                || candidate.start_node == candidate.terminal_node
            {
                errors.push(format!("line {line_no}: invalid case metadata or energy bounds"));
                continue;
            }
            parsed_meta = Some(candidate);
        }

        let meta = parsed_meta.as_ref().expect("metadata initialized");
        let scenario_id = fields[11];
        let observation_label = fields[12];
        let stage = match fields[13] {
            "0" => 0,
            "1" => 1,
            other => {
                errors.push(format!("line {line_no}: stage must be 0 or 1, found {other:?}"));
                continue;
            }
        };
        let action_id = fields[14];
        let from_node = fields[15];
        let to_node = fields[16];
        let is_terminal = match parse_bool(fields[17], "is_terminal") {
            Ok(value) => value,
            Err(error) => {
                errors.push(format!("line {line_no}: {error}"));
                continue;
            }
        };
        let edge_kind = fields[18];
        let mode_id = fields[19];
        let available = match parse_bool(fields[20], "available") {
            Ok(value) => value,
            Err(error) => {
                errors.push(format!("line {line_no}: {error}"));
                continue;
            }
        };
        let quantities = (21..=24)
            .map(|index| parse_u64(fields[index], &format!("column {}", index + 1)))
            .chain(std::iter::once(parse_u64(fields[26], "installed_mass_g")))
            .collect::<Result<Vec<_>, _>>();
        let quantities = match quantities {
            Ok(values) => values,
            Err(error) => {
                errors.push(format!("line {line_no}: {error}"));
                continue;
            }
        };

        if !valid_id(scenario_id)
            || !valid_id(observation_label)
            || !valid_id(action_id)
            || !valid_id(from_node)
            || !valid_id(to_node)
            || from_node == to_node
            || !valid_id(mode_id)
            || !(edge_kind == "MISSION_LEG" || edge_kind == "REPLENISHMENT")
        {
            errors.push(format!("line {line_no}: invalid scenario, action, topology or mode identifier"));
            continue;
        }
        if stage == 0
            && (from_node != meta.start_node
                || to_node != meta.observation_node
                || is_terminal
                || edge_kind != "MISSION_LEG")
        {
            errors.push(format!("line {line_no}: stage-0 topology must be start -> observation mission leg"));
            continue;
        }
        if stage == 1
            && (from_node != meta.observation_node
                || to_node != meta.terminal_node
                || !is_terminal
                || edge_kind != "REPLENISHMENT")
        {
            errors.push(format!("line {line_no}: stage-1 topology must be observation -> terminal replenishment"));
            continue;
        }

        let duration_slots = quantities[0];
        let battery_gain_mwh = quantities[1];
        let aircraft_draw_mwh = quantities[2];
        let site_energy_mwh = quantities[3];
        let installed_mass_g = quantities[4];

        if available {
            if duration_slots == 0 {
                errors.push(format!("line {line_no}: available action must have positive duration"));
                continue;
            }
            if stage == 0
                && (battery_gain_mwh != 0
                    || site_energy_mwh != 0
                    || fields[25] != "NONE"
                    || installed_mass_g != 0)
            {
                errors.push(format!("line {line_no}: stage-0 mission action cannot add energy or receiver mass"));
                continue;
            }
            if stage == 1 && battery_gain_mwh == 0 {
                errors.push(format!("line {line_no}: replenishment must declare positive battery-terminal gain"));
                continue;
            }
        } else if duration_slots != 0
            || battery_gain_mwh != 0
            || aircraft_draw_mwh != 0
            || site_energy_mwh != 0
        {
            errors.push(format!("line {line_no}: unavailable alternative must carry zero duration/energy quantities"));
            continue;
        }

        let asset_id = fields[25];
        if stage == 0 && asset_id != "NONE" {
            errors.push(format!("line {line_no}: stage-0 mission leg must use asset_id NONE"));
            continue;
        }
        if asset_id == "NONE" && installed_mass_g != 0 {
            errors.push(format!("line {line_no}: asset_id NONE requires zero installed mass"));
            continue;
        }
        if asset_id != "NONE" {
            if !valid_id(asset_id) {
                errors.push(format!("line {line_no}: invalid receiver asset ID"));
                continue;
            }
            if let Some(existing) = asset_masses.get(asset_id) {
                if *existing != installed_mass_g {
                    errors.push(format!("line {line_no}: asset {asset_id} has inconsistent mass"));
                    continue;
                }
            } else {
                asset_masses.insert(asset_id.to_string(), installed_mass_g);
            }
        }

        let key = (scenario_id.to_string(), stage, action_id.to_string());
        if !edge_keys.insert(key) {
            errors.push(format!("line {line_no}: duplicate scenario/stage/action alternative"));
            continue;
        }

        let scenario = scenarios
            .entry(scenario_id.to_string())
            .or_insert_with(|| Scenario {
                scenario_id: scenario_id.to_string(),
                observation_label: observation_label.to_string(),
                stage0: BTreeMap::new(),
                stage1: BTreeMap::new(),
            });
        if scenario.observation_label != observation_label {
            errors.push(format!("line {line_no}: scenario observation label changes between stages"));
            continue;
        }

        let edge = Edge {
            scenario_id: scenario_id.to_string(),
            observation_label: observation_label.to_string(),
            stage,
            action_id: action_id.to_string(),
            from_node: from_node.to_string(),
            to_node: to_node.to_string(),
            is_terminal,
            edge_kind: edge_kind.to_string(),
            mode_id: mode_id.to_string(),
            available,
            duration_slots,
            battery_gain_mwh,
            aircraft_draw_mwh,
            site_energy_mwh,
            asset_id: asset_id.to_string(),
            installed_mass_g,
        };
        if stage == 0 {
            scenario.stage0.insert(action_id.to_string(), edge);
        } else {
            scenario.stage1.insert(action_id.to_string(), edge);
        }

        if scenarios.len() > MAX_SCENARIOS {
            errors.push(format!("case exceeds {MAX_SCENARIOS} scenarios"));
            break;
        }
    }

    let Some(meta) = parsed_meta else {
        return Err(if errors.is_empty() {
            vec!["no scenario rows found".to_string()]
        } else {
            errors
        });
    };
    if scenarios.len() < 2 {
        errors.push("at least two explicit scenarios are required".to_string());
    }
    if !errors.is_empty() {
        return Err(errors);
    }

    let first = scenarios.values().next().expect("nonempty scenarios");
    let stage0_action_ids: BTreeSet<String> = first.stage0.keys().cloned().collect();
    let stage1_action_ids: BTreeSet<String> = first.stage1.keys().cloned().collect();
    if stage0_action_ids.is_empty()
        || stage1_action_ids.is_empty()
        || stage0_action_ids.len() > MAX_ACTIONS_PER_STAGE
        || stage1_action_ids.len() > MAX_ACTIONS_PER_STAGE
    {
        return Err(vec!["stage action set is empty or exceeds the declared limit".to_string()]);
    }

    for scenario in scenarios.values() {
        if scenario.stage0.keys().cloned().collect::<BTreeSet<_>>() != stage0_action_ids
            || scenario.stage1.keys().cloned().collect::<BTreeSet<_>>() != stage1_action_ids
        {
            errors.push(format!(
                "scenario {} does not declare the complete common action universe for both stages",
                scenario.scenario_id
            ));
            continue;
        }
        for action_id in &stage0_action_ids {
            let edge = &scenario.stage0[action_id];
            let reference = &first.stage0[action_id];
            if edge.mode_id != reference.mode_id
                || edge.from_node != reference.from_node
                || edge.to_node != reference.to_node
                || edge.asset_id != reference.asset_id
                || edge.installed_mass_g != reference.installed_mass_g
            {
                errors.push(format!(
                    "stage-0 action {action_id} changes its identity/topology/asset across scenarios"
                ));
            }
        }
        for action_id in &stage1_action_ids {
            let edge = &scenario.stage1[action_id];
            let reference = &first.stage1[action_id];
            if edge.mode_id != reference.mode_id
                || edge.from_node != reference.from_node
                || edge.to_node != reference.to_node
                || edge.asset_id != reference.asset_id
                || edge.installed_mass_g != reference.installed_mass_g
            {
                errors.push(format!(
                    "stage-1 action {action_id} changes its identity/topology/asset across scenarios"
                ));
            }
        }
    }
    if !errors.is_empty() {
        return Err(errors);
    }

    Ok(Case {
        meta,
        scenarios,
        stage0_action_ids,
        stage1_action_ids,
        asset_masses,
    })
}

fn common_available_actions<'a>(
    scenarios: impl IntoIterator<Item = &'a Scenario>,
    stage: u8,
    all_action_ids: &BTreeSet<String>,
) -> BTreeSet<String> {
    let mut iter = scenarios.into_iter();
    let Some(first) = iter.next() else {
        return BTreeSet::new();
    };
    let first_map = if stage == 0 { &first.stage0 } else { &first.stage1 };
    let mut common: BTreeSet<String> = all_action_ids
        .iter()
        .filter(|id| first_map.get(*id).is_some_and(|edge| edge.available))
        .cloned()
        .collect();
    for scenario in iter {
        let map = if stage == 0 { &scenario.stage0 } else { &scenario.stage1 };
        common.retain(|id| map.get(id).is_some_and(|edge| edge.available));
    }
    common
}

fn cartesian_assignments(
    groups: &[(String, Vec<String>)],
    index: usize,
    current: &mut BTreeMap<String, String>,
    output: &mut Vec<BTreeMap<String, String>>,
) -> Result<(), String> {
    if index == groups.len() {
        output.push(current.clone());
        if output.len() > MAX_POLICIES {
            return Err(format!("policy product exceeds {MAX_POLICIES}; no truncated result emitted"));
        }
        return Ok(());
    }
    let (key, choices) = &groups[index];
    for choice in choices {
        current.insert(key.clone(), choice.clone());
        cartesian_assignments(groups, index + 1, current, output)?;
        current.remove(key);
    }
    Ok(())
}

fn stage1_assignment_sets(
    case: &Case,
    information_structure: InformationStructure,
) -> Result<Vec<BTreeMap<String, String>>, String> {
    let mut groups: Vec<(String, Vec<String>)> = Vec::new();
    match information_structure {
        InformationStructure::OpenLoopRobust => {
            let choices = common_available_actions(
                case.scenarios.values(),
                1,
                &case.stage1_action_ids,
            );
            groups.push(("*".to_string(), choices.into_iter().collect()));
        }
        InformationStructure::ObservationRecourse => {
            let mut by_observation = BTreeMap::<String, Vec<&Scenario>>::new();
            for scenario in case.scenarios.values() {
                by_observation
                    .entry(scenario.observation_label.clone())
                    .or_default()
                    .push(scenario);
            }
            for (observation, scenarios) in by_observation {
                let choices =
                    common_available_actions(scenarios, 1, &case.stage1_action_ids);
                groups.push((observation, choices.into_iter().collect()));
            }
        }
        InformationStructure::PerfectInformation => {
            for scenario in case.scenarios.values() {
                let choices = common_available_actions(
                    std::iter::once(scenario),
                    1,
                    &case.stage1_action_ids,
                );
                groups.push((scenario.scenario_id.clone(), choices.into_iter().collect()));
            }
        }
    }

    if groups.iter().any(|(_, choices)| choices.is_empty()) {
        return Ok(Vec::new());
    }
    let mut output = Vec::new();
    cartesian_assignments(&groups, 0, &mut BTreeMap::new(), &mut output)?;
    Ok(output)
}

fn stage1_action_key(
    scenario: &Scenario,
    information_structure: InformationStructure,
) -> String {
    match information_structure {
        InformationStructure::OpenLoopRobust => "*".to_string(),
        InformationStructure::ObservationRecourse => scenario.observation_label.clone(),
        InformationStructure::PerfectInformation => scenario.scenario_id.clone(),
    }
}

fn evaluate_policy(
    case: &Case,
    information_structure: InformationStructure,
    stage0_action_id: &str,
    stage1_decisions: &BTreeMap<String, String>,
) -> Option<Policy> {
    let mut scenario_results = Vec::new();
    let mut installed_assets = BTreeSet::new();
    let mut worst_case_elapsed_slots = 0u64;
    let mut worst_case_site_energy_mwh = 0u64;
    let mut worst_case_aircraft_draw_mwh = 0u64;
    let mut minimum_terminal_battery_mwh = u64::MAX;

    for scenario in case.scenarios.values() {
        let stage0 = scenario.stage0.get(stage0_action_id)?;
        if !stage0.available {
            return None;
        }
        let stage1_key = stage1_action_key(scenario, information_structure);
        let stage1_action_id = stage1_decisions.get(&stage1_key)?;
        let stage1 = scenario.stage1.get(stage1_action_id)?;
        if !stage1.available {
            return None;
        }

        let mut battery_mwh = i128::from(case.meta.initial_battery_mwh);
        let mut elapsed_slots = 0u64;
        let mut site_energy_mwh = 0u64;
        let mut aircraft_draw_mwh = 0u64;

        for edge in [stage0, stage1] {
            elapsed_slots = elapsed_slots.checked_add(edge.duration_slots)?;
            if elapsed_slots > case.meta.deadline_slots {
                return None;
            }
            battery_mwh = battery_mwh + i128::from(edge.battery_gain_mwh)
                - i128::from(edge.aircraft_draw_mwh);
            if battery_mwh < i128::from(case.meta.reserve_mwh)
                || battery_mwh > i128::from(case.meta.capacity_mwh)
            {
                return None;
            }
            site_energy_mwh = site_energy_mwh.checked_add(edge.site_energy_mwh)?;
            aircraft_draw_mwh =
                aircraft_draw_mwh.checked_add(edge.aircraft_draw_mwh)?;
            if edge.asset_id != "NONE" {
                installed_assets.insert(edge.asset_id.clone());
            }
        }

        let terminal_battery_mwh = u64::try_from(battery_mwh).ok()?;
        worst_case_elapsed_slots = worst_case_elapsed_slots.max(elapsed_slots);
        worst_case_site_energy_mwh =
            worst_case_site_energy_mwh.max(site_energy_mwh);
        worst_case_aircraft_draw_mwh =
            worst_case_aircraft_draw_mwh.max(aircraft_draw_mwh);
        minimum_terminal_battery_mwh =
            minimum_terminal_battery_mwh.min(terminal_battery_mwh);

        scenario_results.push(ScenarioResult {
            scenario_id: scenario.scenario_id.clone(),
            observation_label: scenario.observation_label.clone(),
            stage0_action_id: stage0_action_id.to_string(),
            stage1_action_id: stage1_action_id.clone(),
            elapsed_slots,
            site_energy_mwh,
            aircraft_draw_mwh,
            terminal_battery_mwh,
        });
    }

    let installed_mass_g = installed_assets.iter().try_fold(0u64, |sum, asset_id| {
        sum.checked_add(*case.asset_masses.get(asset_id)?)
    })?;

    scenario_results.sort_by(|left, right| left.scenario_id.cmp(&right.scenario_id));
    Some(Policy {
        information_structure,
        stage0_action_id: stage0_action_id.to_string(),
        stage1_decisions: stage1_decisions.clone(),
        worst_case_elapsed_slots,
        worst_case_site_energy_mwh,
        worst_case_aircraft_draw_mwh,
        minimum_terminal_battery_mwh,
        installed_mass_g,
        scenarios: scenario_results,
    })
}

fn policy_score(policy: &Policy) -> (u64, u64, u64, u64, std::cmp::Reverse<u64>, String) {
    (
        policy.worst_case_elapsed_slots,
        policy.worst_case_site_energy_mwh,
        policy.worst_case_aircraft_draw_mwh,
        policy.installed_mass_g,
        std::cmp::Reverse(policy.minimum_terminal_battery_mwh),
        format!("{}|{:?}", policy.stage0_action_id, policy.stage1_decisions),
    )
}

fn evaluate(case: &Case) -> Result<Evaluation, String> {
    let structures = [
        InformationStructure::OpenLoopRobust,
        InformationStructure::ObservationRecourse,
        InformationStructure::PerfectInformation,
    ];
    let mut policies = BTreeMap::new();
    let mut counts = BTreeMap::new();

    let common_stage0 = common_available_actions(
        case.scenarios.values(),
        0,
        &case.stage0_action_ids,
    );
    for structure in structures {
        let assignments = stage1_assignment_sets(case, structure)?;
        let mut feasible = Vec::new();
        let mut candidate_count = 0usize;
        let mut rejected_count = 0usize;

        for stage0_action_id in &common_stage0 {
            for assignment in &assignments {
                candidate_count += 1;
                if candidate_count > MAX_POLICIES {
                    return Err(format!(
                        "{} policy product exceeds {MAX_POLICIES}; no truncated result emitted",
                        structure.label()
                    ));
                }
                match evaluate_policy(case, structure, stage0_action_id, assignment) {
                    Some(policy) => feasible.push(policy),
                    None => rejected_count += 1,
                }
            }
        }

        feasible.sort_by_key(policy_score);
        feasible.dedup_by(|left, right| {
            left.stage0_action_id == right.stage0_action_id
                && left.stage1_decisions == right.stage1_decisions
        });
        counts.insert(
            structure,
            StructureCounts {
                candidate_policies: candidate_count,
                feasible_policies: feasible.len(),
                rejected_policies: rejected_count,
            },
        );
        policies.insert(structure, feasible);
    }

    Ok(Evaluation { policies, counts })
}

fn run(path: &str) -> Result<(), String> {
    let input = fs::read_to_string(path)
        .map_err(|error| format!("cannot read {path}: {error}"))?;
    let case = parse_case(&input).map_err(|errors| errors.join("\n"))?;
    let report = evaluate(&case)?;

    println!("case_id={}", case.meta.case_id);
    println!("observation_source_receipt_id={}", case.meta.observation_source_receipt_id);
    println!("observation_source_receipt_sha256={}", case.meta.observation_source_receipt_sha256);
    println!("observation_partition_sha256={}", case.meta.observation_partition_sha256);
    println!("observation_partition_claim=UPSTREAM_RECEIPT_BOUND_NOT_VERIFIED_BY_THIS_CRATE");
    println!("evidence_class={}", case.meta.evidence_class);
    println!("scenario_count={}", case.scenarios.len());
    println!("observation_count={}", case.scenarios.values().map(|s| s.observation_label.clone()).collect::<BTreeSet<_>>().len());
    println!("deadline_slots={}", case.meta.deadline_slots);
    println!("capacity_mwh={}", case.meta.capacity_mwh);
    println!("initial_battery_mwh={}", case.meta.initial_battery_mwh);
    println!("reserve_mwh={}", case.meta.reserve_mwh);

    for structure in [
        InformationStructure::OpenLoopRobust,
        InformationStructure::ObservationRecourse,
        InformationStructure::PerfectInformation,
    ] {
        let count = &report.counts[&structure];
        println!(
            "structure={}|candidate_policies={}|feasible_policies={}|rejected_policies={}",
            structure.label(),
            count.candidate_policies,
            count.feasible_policies,
            count.rejected_policies
        );
        for (index, policy) in report.policies[&structure].iter().enumerate() {
            println!(
                "policy={}|index={}|stage0={}|stage1_decisions={:?}|worst_elapsed_slots={}|worst_site_energy_mwh={}|worst_aircraft_draw_mwh={}|minimum_terminal_battery_mwh={}|installed_mass_g={}",
                structure.label(),
                index + 1,
                policy.stage0_action_id,
                policy.stage1_decisions,
                policy.worst_case_elapsed_slots,
                policy.worst_case_site_energy_mwh,
                policy.worst_case_aircraft_draw_mwh,
                policy.minimum_terminal_battery_mwh,
                policy.installed_mass_g
            );
            for scenario in &policy.scenarios {
                println!(
                    "scenario_path={}|observation={}|stage0={}|stage1={}|elapsed_slots={}|site_energy_mwh={}|aircraft_draw_mwh={}|terminal_battery_mwh={}",
                    scenario.scenario_id,
                    scenario.observation_label,
                    scenario.stage0_action_id,
                    scenario.stage1_action_id,
                    scenario.elapsed_slots,
                    scenario.site_energy_mwh,
                    scenario.aircraft_draw_mwh,
                    scenario.terminal_battery_mwh
                );
            }
        }
    }

    println!("scenario_probabilities=NOT_MODELED");
    println!("expected_energy=NOT_COMPUTED");
    println!("perfect_information_is_deployable=false");
    println!("operational_permit=false");
    println!("authority=NONE_FINITE_POLICY_ORACLE_ONLY");
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("usage: aero-replenishment-recourse <scenarios.tsv>");
        return ExitCode::from(2);
    }
    match run(&args[1]) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("REJECTED_AERO_REPLENISHMENT_RECOURSE {error}");
            ExitCode::from(1)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str =
        include_str!("../../../research/aero-replenishment-recourse-v0/synthetic-scenarios.tsv");

    fn parsed(input: &str) -> Case {
        parse_case(input).unwrap_or_else(|errors| panic!("{}", errors.join("\n")))
    }

    #[test]
    fn scenario_counts_and_information_structures_match_hand_enumeration() {
        let case = parsed(FIXTURE);
        let report = evaluate(&case).expect("evaluation");
        assert_eq!(case.scenarios.len(), 3);
        assert_eq!(
            report.counts[&InformationStructure::OpenLoopRobust].feasible_policies,
            2
        );
        assert_eq!(
            report.counts[&InformationStructure::ObservationRecourse].feasible_policies,
            4
        );
        assert_eq!(
            report.counts[&InformationStructure::PerfectInformation].feasible_policies,
            10
        );
    }

    #[test]
    fn recourse_never_uses_hidden_scenario_identity_for_shared_observation() {
        let case = parsed(FIXTURE);
        let report = evaluate(&case).expect("evaluation");
        for policy in &report.policies[&InformationStructure::ObservationRecourse] {
            assert_eq!(policy.stage1_decisions.len(), 2);
            assert!(policy.stage1_decisions.contains_key("NORMAL"));
            assert!(policy.stage1_decisions.contains_key("DEGRADED"));
            let normal_actions: BTreeSet<String> = policy
                .scenarios
                .iter()
                .filter(|scenario| scenario.observation_label == "NORMAL")
                .map(|scenario| scenario.stage1_action_id.clone())
                .collect();
            assert_eq!(normal_actions.len(), 1);
        }
    }

    #[test]
    fn open_loop_plan_must_use_one_stage1_action_for_every_scenario() {
        let case = parsed(FIXTURE);
        let report = evaluate(&case).expect("evaluation");
        assert_eq!(
            report.policies[&InformationStructure::OpenLoopRobust].len(),
            2
        );
        for policy in &report.policies[&InformationStructure::OpenLoopRobust] {
            let actions: BTreeSet<String> = policy
                .scenarios
                .iter()
                .map(|scenario| scenario.stage1_action_id.clone())
                .collect();
            assert_eq!(actions.len(), 1);
        }
    }

    #[test]
    fn perfect_information_is_an_optimistic_non_deployable_upper_bound() {
        let case = parsed(FIXTURE);
        let report = evaluate(&case).expect("evaluation");
        assert!(
            report.policies[&InformationStructure::PerfectInformation].len()
                >= report.policies[&InformationStructure::ObservationRecourse].len()
        );
        assert!(report.policies[&InformationStructure::PerfectInformation]
            .iter()
            .any(|policy| {
                let s1 = policy.scenarios.iter().find(|s| s.scenario_id == "S1").unwrap();
                let s2 = policy.scenarios.iter().find(|s| s.scenario_id == "S2").unwrap();
                s1.observation_label == s2.observation_label
                    && s1.stage1_action_id != s2.stage1_action_id
            }));
    }

    #[test]
    fn every_reported_policy_preserves_reserve_capacity_and_deadline_in_every_scenario() {
        let case = parsed(FIXTURE);
        let report = evaluate(&case).expect("evaluation");
        for policies in report.policies.values() {
            for policy in policies {
                assert!(policy.scenarios.iter().all(|scenario| {
                    scenario.elapsed_slots <= case.meta.deadline_slots
                        && scenario.terminal_battery_mwh >= case.meta.reserve_mwh
                        && scenario.terminal_battery_mwh <= case.meta.capacity_mwh
                }));
            }
        }
    }

    #[test]
    fn receiver_mass_is_the_union_of_assets_needed_by_any_policy_branch() {
        let case = parsed(FIXTURE);
        let report = evaluate(&case).expect("evaluation");
        let policy = report.policies[&InformationStructure::ObservationRecourse]
            .iter()
            .find(|policy| {
                policy.stage1_decisions.get("NORMAL").map(String::as_str) == Some("CONTACT")
                    && policy.stage1_decisions.get("DEGRADED").map(String::as_str) == Some("IPT")
            })
            .expect("contact plus IPT recourse policy");
        assert_eq!(policy.installed_mass_g, 550);
    }

    #[test]
    fn row_permutation_preserves_policy_results() {
        let mut rows: Vec<&str> = FIXTURE.lines().collect();
        let header = rows.remove(0);
        rows.reverse();
        let permuted = format!("{header}\n{}\n", rows.join("\n"));
        assert_eq!(
            evaluate(&parsed(FIXTURE)).expect("original"),
            evaluate(&parsed(&permuted)).expect("permuted")
        );
    }

    #[test]
    fn unavailable_edge_must_have_zero_delivered_energy_and_never_selected() {
        let invalid = FIXTURE.replace(
            "BATTERY_EXCHANGE\tfalse\t0\t0\t0\t0\tSWAP_RX",
            "BATTERY_EXCHANGE\tfalse\t0\t45\t4\t72\tSWAP_RX",
        );
        assert!(parse_case(&invalid).is_err());
    }

    #[test]
    fn incomplete_scenario_action_universe_is_rejected() {
        let incomplete = FIXTURE
            .lines()
            .filter(|line| !line.contains("\tS3\tDEGRADED\t1\tSWAP\t"))
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        assert!(parse_case(&incomplete).is_err());
    }

    #[test]
    fn inconsistent_observation_label_across_rows_is_rejected() {
        let inconsistent = FIXTURE.replacen(
            "S1\tNORMAL\t1\tCONTACT",
            "S1\tDEGRADED\t1\tCONTACT",
            1,
        );
        assert!(parse_case(&inconsistent).is_err());
    }

    #[test]
    fn action_identity_and_receiver_mass_cannot_change_by_scenario() {
        let inconsistent = FIXTURE.replacen(
            "S2\tNORMAL\t1\tCONTACT\tDECISION\tTERMINAL\ttrue\tREPLENISHMENT\tCONDUCTIVE_CONTACT\ttrue\t2\t32\t5\t65\tCONTACT_RX\t100",
            "S2\tNORMAL\t1\tCONTACT\tDECISION\tTERMINAL\ttrue\tREPLENISHMENT\tCONDUCTIVE_CONTACT\ttrue\t2\t32\t5\t65\tCONTACT_RX\t101",
            1,
        );
        assert!(parse_case(&inconsistent).is_err());
    }

    #[test]
    fn no_robust_policy_is_reported_when_all_stage1_services_fail_one_scenario() {
        let failed = FIXTURE.replace(
            "IPT_DOCK\ttrue\t3\t30\t6\t50\tIPT_RX\t450",
            "IPT_DOCK\tfalse\t0\t0\t0\t0\tIPT_RX\t450",
        );
        let case = parsed(&failed);
        let report = evaluate(&case).expect("evaluation");
        assert!(report.policies[&InformationStructure::OpenLoopRobust].is_empty());
        assert!(report.policies[&InformationStructure::ObservationRecourse].is_empty());
    }


    #[test]
    fn observation_partition_is_bound_to_one_upstream_receipt_and_digest() {
        let changed = FIXTURE.replacen(
            "obs_partition_synthetic_v0",
            "obs_partition_alternate",
            1,
        );
        assert!(parse_case(&changed).is_err());
    }

    #[test]
    fn malformed_observation_receipt_or_partition_digest_is_rejected() {
        let bad_receipt = FIXTURE.replacen(
            "1111111111111111111111111111111111111111111111111111111111111111",
            "not-a-digest",
            1,
        );
        assert!(parse_case(&bad_receipt).is_err());
        let bad_partition = FIXTURE.replacen(
            "2222222222222222222222222222222222222222222222222222222222222222",
            "invalid",
            1,
        );
        assert!(parse_case(&bad_partition).is_err());
    }

    #[test]
    fn observation_partition_fields_are_preserved_in_case_metadata() {
        let case = parsed(FIXTURE);
        assert_eq!(
            case.meta.observation_source_receipt_id,
            "obs_partition_synthetic_v0"
        );
        assert_eq!(
            case.meta.observation_partition_sha256,
            "2222222222222222222222222222222222222222222222222222222222222222"
        );
    }

    #[test]
    fn scenario_counts_are_not_used_as_probabilities() {
        let case = parsed(FIXTURE);
        let report = evaluate(&case).expect("evaluation");
        assert_eq!(report.counts[&InformationStructure::ObservationRecourse].candidate_policies, 4);
    }
}
