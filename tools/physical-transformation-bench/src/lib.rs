use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Corpus {
    pub schema_version: u32,
    pub cases: Vec<BenchmarkCase>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct BenchmarkCase {
    pub id: String,
    pub scenario: Scenario,
    pub expected: ExpectedResult,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Scenario {
    pub id: String,
    pub units: Units,
    pub nodes: Vec<Node>,
    pub materials: Vec<MaterialSpec>,
    pub members: Vec<Member>,
    pub events: Vec<Event>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Units {
    pub length: String,
    pub mass: String,
    pub force: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Node {
    pub id: String,
    pub position_mm: [i64; 3],
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct MaterialSpec {
    pub id: String,
    pub allowed_recovery_routes: Vec<RecoveryRoute>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryRoute {
    Reuse,
    Recycle,
    Waste,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MemberState {
    Installed,
    Rubble,
    Reused,
    Recycled,
    Waste,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Member {
    pub id: String,
    pub from_node: String,
    pub to_node: String,
    pub material_id: String,
    pub mass_g: u64,
    pub load_bearing: bool,
    pub connected: bool,
    pub state: MemberState,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct Event {
    pub sequence: u64,
    pub id: String,
    #[serde(flatten)]
    pub action: Action,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Action {
    ConnectMember {
        member_id: String,
    },
    DisconnectMember {
        member_id: String,
    },
    RemoveSupport {
        member_id: String,
    },
    InstallMember {
        member: Member,
    },
    ApplyLoad {
        load: AppliedLoad,
    },
    RecoverMaterial {
        member_id: String,
        route: RecoveryRoute,
    },
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct AppliedLoad {
    pub id: String,
    pub node_id: String,
    /// Signed milli-newtons in x, y, z. This event is recorded and bounded;
    /// this kernel does not solve equilibrium or deformation.
    pub force_mn: [i64; 3],
    pub max_abs_component_mn: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq, Default)]
pub struct MaterialTotals {
    pub installed_mass_g: u64,
    pub rubble_mass_g: u64,
    pub reused_mass_g: u64,
    pub recycled_mass_g: u64,
    pub waste_mass_g: u64,
}

impl MaterialTotals {
    pub fn total_mass_g(&self) -> Option<u64> {
        self.installed_mass_g
            .checked_add(self.rubble_mass_g)?
            .checked_add(self.reused_mass_g)?
            .checked_add(self.recycled_mass_g)?
            .checked_add(self.waste_mass_g)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct ExpectedResult {
    pub component_count: usize,
    pub connected_components: Vec<Vec<String>>,
    pub material_totals: MaterialTotals,
    pub applied_load_count: usize,
    pub mass_balance_residual_g: i128,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct ScenarioReceipt {
    pub scenario_id: String,
    pub canonical_input_sha256: String,
    pub final_state_sha256: String,
    pub event_count_applied: usize,
    pub connected_components: Vec<Vec<String>>,
    pub material_totals: MaterialTotals,
    pub input_mass_g: u64,
    pub added_mass_g: u64,
    pub accounted_mass_g: u64,
    pub mass_balance_residual_g: i128,
    pub applied_loads: Vec<AppliedLoad>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct CaseReceipt {
    pub id: String,
    pub passed: bool,
    pub errors: Vec<String>,
    pub actual: Option<ScenarioReceipt>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct CorpusReceipt {
    pub schema_version: u32,
    pub passed: bool,
    pub case_count: usize,
    pub passed_cases: usize,
    pub failed_cases: usize,
    pub cases: Vec<CaseReceipt>,
}

#[derive(Clone, Debug, Serialize)]
struct CanonicalInput {
    schema_version: u32,
    id: String,
    units: Units,
    nodes: BTreeMap<String, Node>,
    materials: BTreeMap<String, MaterialSpec>,
    members: BTreeMap<String, Member>,
    events: Vec<Event>,
}

#[derive(Clone, Debug, Serialize)]
struct CanonicalState<'a> {
    schema_version: u32,
    id: &'a str,
    units: &'a Units,
    nodes: &'a BTreeMap<String, Node>,
    materials: &'a BTreeMap<String, MaterialSpec>,
    members: &'a BTreeMap<String, Member>,
    loads: &'a BTreeMap<String, AppliedLoad>,
    initial_input_mass_g: u64,
    added_mass_g: u64,
}

struct State {
    id: String,
    units: Units,
    nodes: BTreeMap<String, Node>,
    materials: BTreeMap<String, MaterialSpec>,
    members: BTreeMap<String, Member>,
    loads: BTreeMap<String, AppliedLoad>,
    initial_input_mass_g: u64,
    added_mass_g: u64,
    canonical_input_sha256: String,
    applied_event_ids: BTreeSet<String>,
    event_count_applied: usize,
}

pub fn execute_scenario(scenario: &Scenario) -> Result<ScenarioReceipt, String> {
    let mut state = initialize(scenario)?;

    for (index, event) in scenario.events.iter().enumerate() {
        let expected_sequence = u64::try_from(index)
            .ok()
            .and_then(|value| value.checked_add(1))
            .ok_or_else(|| "event sequence overflow".to_string())?;
        if event.sequence != expected_sequence {
            return Err(format!(
                "event {} has sequence {}; expected {}",
                event.id, event.sequence, expected_sequence
            ));
        }
        if event.id.trim().is_empty() {
            return Err("event id must not be empty".to_string());
        }
        if !state.applied_event_ids.insert(event.id.clone()) {
            return Err(format!("duplicate event id: {}", event.id));
        }

        apply_event(&mut state, event)?;
        state.event_count_applied = state
            .event_count_applied
            .checked_add(1)
            .ok_or_else(|| "event count overflow".to_string())?;
    }

    let components = connected_components(&state);
    let totals = material_totals(&state)?;
    let accounted_mass_g = totals
        .total_mass_g()
        .ok_or_else(|| "material total overflow".to_string())?;
    let declared_input_mass_g = state
        .initial_input_mass_g
        .checked_add(state.added_mass_g)
        .ok_or_else(|| "declared input mass overflow".to_string())?;
    let mass_balance_residual_g = i128::from(declared_input_mass_g) - i128::from(accounted_mass_g);

    let canonical_state = CanonicalState {
        schema_version: SCHEMA_VERSION,
        id: &state.id,
        units: &state.units,
        nodes: &state.nodes,
        materials: &state.materials,
        members: &state.members,
        loads: &state.loads,
        initial_input_mass_g: state.initial_input_mass_g,
        added_mass_g: state.added_mass_g,
    };
    let state_bytes = serde_json::to_vec(&canonical_state)
        .map_err(|error| format!("serialize canonical state: {error}"))?;

    Ok(ScenarioReceipt {
        scenario_id: state.id,
        canonical_input_sha256: state.canonical_input_sha256,
        final_state_sha256: sha256_hex(&state_bytes),
        event_count_applied: state.event_count_applied,
        connected_components: components,
        material_totals: totals,
        input_mass_g: state.initial_input_mass_g,
        added_mass_g: state.added_mass_g,
        accounted_mass_g,
        mass_balance_residual_g,
        applied_loads: state.loads.into_values().collect(),
    })
}

pub fn evaluate_corpus(corpus: &Corpus) -> Result<CorpusReceipt, String> {
    if corpus.schema_version != SCHEMA_VERSION {
        return Err(format!(
            "unsupported corpus schema version {}; expected {}",
            corpus.schema_version, SCHEMA_VERSION
        ));
    }

    let mut case_ids = BTreeSet::new();
    let mut cases = Vec::with_capacity(corpus.cases.len());

    for case in &corpus.cases {
        if case.id.trim().is_empty() {
            return Err("case id must not be empty".to_string());
        }
        if !case_ids.insert(case.id.clone()) {
            return Err(format!("duplicate case id: {}", case.id));
        }

        let mut errors = Vec::new();
        let actual = match execute_scenario(&case.scenario) {
            Ok(receipt) => {
                compare_expected(&case.expected, &receipt, &mut errors);
                Some(receipt)
            }
            Err(error) => {
                errors.push(error);
                None
            }
        };
        cases.push(CaseReceipt {
            id: case.id.clone(),
            passed: errors.is_empty(),
            errors,
            actual,
        });
    }

    let passed_cases = cases.iter().filter(|case| case.passed).count();
    let failed_cases = cases.len() - passed_cases;
    Ok(CorpusReceipt {
        schema_version: SCHEMA_VERSION,
        passed: failed_cases == 0,
        case_count: cases.len(),
        passed_cases,
        failed_cases,
        cases,
    })
}

fn compare_expected(expected: &ExpectedResult, actual: &ScenarioReceipt, errors: &mut Vec<String>) {
    if actual.connected_components.len() != expected.component_count {
        errors.push(format!(
            "component count: expected {}, got {}",
            expected.component_count,
            actual.connected_components.len()
        ));
    }
    if actual.connected_components != expected.connected_components {
        errors.push(format!(
            "connected components differ: expected {:?}, got {:?}",
            expected.connected_components, actual.connected_components
        ));
    }
    if actual.material_totals != expected.material_totals {
        errors.push(format!(
            "material totals differ: expected {:?}, got {:?}",
            expected.material_totals, actual.material_totals
        ));
    }
    if actual.applied_loads.len() != expected.applied_load_count {
        errors.push(format!(
            "applied load count: expected {}, got {}",
            expected.applied_load_count,
            actual.applied_loads.len()
        ));
    }
    if actual.mass_balance_residual_g != expected.mass_balance_residual_g {
        errors.push(format!(
            "mass balance residual: expected {}, got {}",
            expected.mass_balance_residual_g, actual.mass_balance_residual_g
        ));
    }
}

fn initialize(scenario: &Scenario) -> Result<State, String> {
    if scenario.id.trim().is_empty() {
        return Err("scenario id must not be empty".to_string());
    }
    if scenario.units.length != "mm" || scenario.units.mass != "g" || scenario.units.force != "mN" {
        return Err(format!(
            "unsupported or missing units: required length=mm, mass=g, force=mN; got {:?}",
            scenario.units
        ));
    }

    let mut nodes = BTreeMap::new();
    for node in &scenario.nodes {
        require_id("node", &node.id)?;
        insert_unique(&mut nodes, node.id.clone(), node.clone(), "node")?;
    }

    let mut materials = BTreeMap::new();
    for material in &scenario.materials {
        require_id("material", &material.id)?;
        let mut routes = BTreeSet::new();
        for route in &material.allowed_recovery_routes {
            if !routes.insert(*route) {
                return Err(format!(
                    "material {} repeats recovery route {:?}",
                    material.id, route
                ));
            }
        }
        insert_unique(
            &mut materials,
            material.id.clone(),
            material.clone(),
            "material",
        )?;
    }

    let mut members = BTreeMap::new();
    for member in &scenario.members {
        validate_member(member, &nodes, &materials)?;
        insert_unique(&mut members, member.id.clone(), member.clone(), "member")?;
    }

    let mut event_ids = BTreeSet::new();
    for (index, event) in scenario.events.iter().enumerate() {
        require_id("event", &event.id)?;
        let expected_sequence = u64::try_from(index)
            .ok()
            .and_then(|value| value.checked_add(1))
            .ok_or_else(|| "event sequence overflow".to_string())?;
        if event.sequence != expected_sequence {
            return Err(format!(
                "event {} has sequence {}; expected {}",
                event.id, event.sequence, expected_sequence
            ));
        }
        if !event_ids.insert(event.id.clone()) {
            return Err(format!("duplicate event id: {}", event.id));
        }
    }

    let initial_input_mass_g = members.values().try_fold(0_u64, |sum, member| {
        sum.checked_add(member.mass_g)
            .ok_or_else(|| "initial material mass overflow".to_string())
    })?;

    let canonical_input = CanonicalInput {
        schema_version: SCHEMA_VERSION,
        id: scenario.id.clone(),
        units: scenario.units.clone(),
        nodes: nodes.clone(),
        materials: materials.clone(),
        members: members.clone(),
        events: scenario.events.clone(),
    };
    let canonical_bytes = serde_json::to_vec(&canonical_input)
        .map_err(|error| format!("serialize canonical input: {error}"))?;

    Ok(State {
        id: scenario.id.clone(),
        units: scenario.units.clone(),
        nodes,
        materials,
        members,
        loads: BTreeMap::new(),
        initial_input_mass_g,
        added_mass_g: 0,
        canonical_input_sha256: sha256_hex(&canonical_bytes),
        applied_event_ids: BTreeSet::new(),
        event_count_applied: 0,
    })
}

fn validate_member(
    member: &Member,
    nodes: &BTreeMap<String, Node>,
    materials: &BTreeMap<String, MaterialSpec>,
) -> Result<(), String> {
    require_id("member", &member.id)?;
    if member.mass_g == 0 {
        return Err(format!("member {} must have positive mass_g", member.id));
    }
    if member.from_node == member.to_node {
        return Err(format!(
            "member {} cannot connect a node to itself",
            member.id
        ));
    }
    if !nodes.contains_key(&member.from_node) {
        return Err(format!(
            "member {} references unknown from_node {}",
            member.id, member.from_node
        ));
    }
    if !nodes.contains_key(&member.to_node) {
        return Err(format!(
            "member {} references unknown to_node {}",
            member.id, member.to_node
        ));
    }
    if !materials.contains_key(&member.material_id) {
        return Err(format!(
            "member {} references unknown material {}",
            member.id, member.material_id
        ));
    }
    if member.state != MemberState::Installed && member.connected {
        return Err(format!(
            "member {} cannot be connected when state is {:?}",
            member.id, member.state
        ));
    }
    Ok(())
}

fn apply_event(state: &mut State, event: &Event) -> Result<(), String> {
    match &event.action {
        Action::ConnectMember { member_id } => {
            let member = state
                .members
                .get_mut(member_id)
                .ok_or_else(|| format!("connect references unknown member {member_id}"))?;
            if member.state != MemberState::Installed || member.connected {
                return Err(format!(
                    "member {member_id} cannot be connected from state {:?}, connected={}",
                    member.state, member.connected
                ));
            }
            member.connected = true;
        }
        Action::DisconnectMember { member_id } => {
            let member = state
                .members
                .get_mut(member_id)
                .ok_or_else(|| format!("disconnect references unknown member {member_id}"))?;
            if member.state != MemberState::Installed || !member.connected {
                return Err(format!(
                    "member {member_id} cannot be disconnected from state {:?}, connected={}",
                    member.state, member.connected
                ));
            }
            member.connected = false;
        }
        Action::RemoveSupport { member_id } => {
            let member = state
                .members
                .get_mut(member_id)
                .ok_or_else(|| format!("support removal references unknown member {member_id}"))?;
            if member.state != MemberState::Installed || !member.load_bearing || !member.connected {
                return Err(format!(
                    "member {member_id} is not an installed, connected load-bearing support"
                ));
            }
            member.state = MemberState::Rubble;
            member.connected = false;
        }
        Action::InstallMember { member } => {
            if member.state != MemberState::Installed || !member.connected {
                return Err(format!(
                    "installed member {} must enter the scene connected and in installed state",
                    member.id
                ));
            }
            validate_member(member, &state.nodes, &state.materials)?;
            if state.members.contains_key(&member.id) {
                return Err(format!("member id already exists: {}", member.id));
            }
            state.added_mass_g = state
                .added_mass_g
                .checked_add(member.mass_g)
                .ok_or_else(|| "added material mass overflow".to_string())?;
            state.members.insert(member.id.clone(), member.clone());
        }
        Action::ApplyLoad { load } => {
            require_id("load", &load.id)?;
            if !state.nodes.contains_key(&load.node_id) {
                return Err(format!(
                    "load {} references unknown node {}",
                    load.id, load.node_id
                ));
            }
            if load.max_abs_component_mn == 0 {
                return Err(format!(
                    "load {} must declare a positive force bound",
                    load.id
                ));
            }
            for component in load.force_mn {
                let magnitude = component
                    .checked_abs()
                    .ok_or_else(|| format!("load {} force magnitude overflows", load.id))?;
                if u64::try_from(magnitude).unwrap_or(u64::MAX) > load.max_abs_component_mn {
                    return Err(format!(
                        "load {} exceeds max_abs_component_mn {}",
                        load.id, load.max_abs_component_mn
                    ));
                }
            }
            if state.loads.contains_key(&load.id) {
                return Err(format!("load id already exists: {}", load.id));
            }
            state.loads.insert(load.id.clone(), load.clone());
        }
        Action::RecoverMaterial { member_id, route } => {
            let member = state.members.get(member_id).ok_or_else(|| {
                format!("material recovery references unknown member {member_id}")
            })?;
            if member.state != MemberState::Rubble {
                return Err(format!(
                    "member {member_id} must be rubble before material recovery; got {:?}",
                    member.state
                ));
            }
            let material_id = member.material_id.clone();
            let material = state
                .materials
                .get(&material_id)
                .ok_or_else(|| format!("unknown material {material_id}"))?;
            if !material.allowed_recovery_routes.contains(route) {
                return Err(format!(
                    "material {} does not permit recovery route {:?}",
                    material.id, route
                ));
            }
            let member = state
                .members
                .get_mut(member_id)
                .expect("member was validated above");
            member.state = match route {
                RecoveryRoute::Reuse => MemberState::Reused,
                RecoveryRoute::Recycle => MemberState::Recycled,
                RecoveryRoute::Waste => MemberState::Waste,
            };
        }
    }
    Ok(())
}

fn connected_components(state: &State) -> Vec<Vec<String>> {
    let mut adjacency: BTreeMap<String, BTreeSet<String>> = state
        .nodes
        .keys()
        .cloned()
        .map(|id| (id, BTreeSet::new()))
        .collect();

    for member in state.members.values() {
        if member.state == MemberState::Installed && member.connected {
            if let Some(neighbors) = adjacency.get_mut(&member.from_node) {
                neighbors.insert(member.to_node.clone());
            }
            if let Some(neighbors) = adjacency.get_mut(&member.to_node) {
                neighbors.insert(member.from_node.clone());
            }
        }
    }

    let mut unseen: BTreeSet<String> = state.nodes.keys().cloned().collect();
    let mut components = Vec::new();
    while let Some(start) = unseen.iter().next().cloned() {
        unseen.remove(&start);
        let mut queue = VecDeque::from([start]);
        let mut component = Vec::new();
        while let Some(node) = queue.pop_front() {
            component.push(node.clone());
            if let Some(neighbors) = adjacency.get(&node) {
                for neighbor in neighbors {
                    if unseen.remove(neighbor) {
                        queue.push_back(neighbor.clone());
                    }
                }
            }
        }
        component.sort();
        components.push(component);
    }
    components.sort();
    components
}

fn material_totals(state: &State) -> Result<MaterialTotals, String> {
    let mut totals = MaterialTotals::default();
    for member in state.members.values() {
        let target = match member.state {
            MemberState::Installed => &mut totals.installed_mass_g,
            MemberState::Rubble => &mut totals.rubble_mass_g,
            MemberState::Reused => &mut totals.reused_mass_g,
            MemberState::Recycled => &mut totals.recycled_mass_g,
            MemberState::Waste => &mut totals.waste_mass_g,
        };
        *target = target
            .checked_add(member.mass_g)
            .ok_or_else(|| "material category mass overflow".to_string())?;
    }
    Ok(totals)
}

fn require_id(label: &str, id: &str) -> Result<(), String> {
    if id.trim().is_empty() {
        Err(format!("{label} id must not be empty"))
    } else {
        Ok(())
    }
}

fn insert_unique<T>(
    map: &mut BTreeMap<String, T>,
    id: String,
    value: T,
    label: &str,
) -> Result<(), String> {
    if map.contains_key(&id) {
        return Err(format!("duplicate {label} id: {id}"));
    }
    map.insert(id, value);
    Ok(())
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn corpus() -> Corpus {
        let bytes = include_str!("../fixtures/physical_transformation_v0_1.json");
        serde_json::from_str(bytes).expect("frozen fixture corpus must parse")
    }

    #[test]
    fn frozen_fixtures_match_independent_known_answers() {
        let receipt = evaluate_corpus(&corpus()).expect("corpus should evaluate");
        assert!(receipt.passed, "{:#?}", receipt.cases);
        assert_eq!(receipt.case_count, 4);
        assert_eq!(receipt.passed_cases, 4);
        assert_eq!(receipt.failed_cases, 0);
    }

    #[test]
    fn input_order_permutations_preserve_canonical_receipt() {
        let source = corpus();
        let mut scenario = source.cases[1].scenario.clone();
        let original = execute_scenario(&scenario).expect("original scenario valid");
        scenario.nodes.reverse();
        scenario.materials.reverse();
        scenario.members.reverse();
        let permuted = execute_scenario(&scenario).expect("permuted scenario valid");
        assert_eq!(
            original.canonical_input_sha256,
            permuted.canonical_input_sha256
        );
        assert_eq!(original.final_state_sha256, permuted.final_state_sha256);
        assert_eq!(original.connected_components, permuted.connected_components);
        assert_eq!(original.material_totals, permuted.material_totals);
    }

    #[test]
    fn duplicate_member_ids_fail_closed() {
        let mut scenario = corpus().cases[0].scenario.clone();
        scenario.members.push(scenario.members[0].clone());
        let error = execute_scenario(&scenario).expect_err("duplicate member must fail");
        assert!(error.contains("duplicate member id"));
    }

    #[test]
    fn unknown_material_fails_closed() {
        let mut scenario = corpus().cases[0].scenario.clone();
        scenario.members[0].material_id = "unregistered_material".to_string();
        let error = execute_scenario(&scenario).expect_err("unknown material must fail");
        assert!(error.contains("unknown material"));
    }

    #[test]
    fn unsupported_units_fail_closed() {
        let mut scenario = corpus().cases[0].scenario.clone();
        scenario.units.length = "metres".to_string();
        let error = execute_scenario(&scenario).expect_err("unit mismatch must fail");
        assert!(error.contains("unsupported or missing units"));
    }

    #[test]
    fn broken_recovery_permission_fails_closed() {
        let mut scenario = corpus().cases[2].scenario.clone();
        scenario
            .materials
            .iter_mut()
            .find(|material| material.id == "concrete")
            .expect("fixture concrete material")
            .allowed_recovery_routes
            .clear();
        let error = execute_scenario(&scenario).expect_err("recovery must be authorized");
        assert!(error.contains("does not permit recovery route"));
    }

    #[test]
    fn loads_outside_declared_bounds_fail_closed() {
        let mut scenario = corpus().cases[3].scenario.clone();
        if let Action::ApplyLoad { load } = &mut scenario.events[0].action {
            load.force_mn[1] = -5_001;
        } else {
            panic!("fixture must start with an apply_load event");
        }
        let error = execute_scenario(&scenario).expect_err("over-bound force must fail");
        assert!(error.contains("exceeds max_abs_component_mn"));
    }

    #[test]
    fn duplicate_event_ids_fail_closed() {
        let mut scenario = corpus().cases[1].scenario.clone();
        scenario.events[1].id = scenario.events[0].id.clone();
        let error = execute_scenario(&scenario).expect_err("duplicate event must fail");
        assert!(error.contains("duplicate event id"));
    }

    #[test]
    fn invalid_state_transition_fails_closed() {
        let mut scenario = corpus().cases[0].scenario.clone();
        scenario.events[0].action = Action::RecoverMaterial {
            member_id: "beam_bc".to_string(),
            route: RecoveryRoute::Reuse,
        };
        let error = execute_scenario(&scenario).expect_err("installed material is not rubble");
        assert!(error.contains("must be rubble before material recovery"));
    }

    #[test]
    fn mass_is_conserved_across_install_remove_and_recovery() {
        let mut scenario = corpus().cases[0].scenario.clone();
        scenario.events[0].action = Action::RemoveSupport {
            member_id: "support_ab".to_string(),
        };
        let after_removal = execute_scenario(&scenario).expect("support removal is valid");
        assert_eq!(after_removal.mass_balance_residual_g, 0);
        assert_eq!(after_removal.accounted_mass_g, 2_000);
        assert_eq!(after_removal.material_totals.rubble_mass_g, 1_200);
    }

    #[test]
    fn installing_material_increases_declared_input_and_inventory_equally() {
        let mut scenario = corpus().cases[1].scenario.clone();
        let member = Member {
            id: "new_support".to_string(),
            from_node: "node_a".to_string(),
            to_node: "node_d".to_string(),
            material_id: "steel".to_string(),
            mass_g: 250,
            load_bearing: true,
            connected: true,
            state: MemberState::Installed,
        };
        scenario.events.push(Event {
            sequence: 4,
            id: "install-4".to_string(),
            action: Action::InstallMember { member },
        });
        let receipt = execute_scenario(&scenario).expect("valid installed member");
        assert_eq!(receipt.added_mass_g, 250);
        assert_eq!(receipt.accounted_mass_g, receipt.input_mass_g + 250);
        assert_eq!(receipt.mass_balance_residual_g, 0);
    }

    #[test]
    fn duplicate_case_ids_fail_closed() {
        let mut value = corpus();
        value.cases[1].id = value.cases[0].id.clone();
        let error = evaluate_corpus(&value).expect_err("duplicate case ids must fail");
        assert!(error.contains("duplicate case id"));
    }
}
