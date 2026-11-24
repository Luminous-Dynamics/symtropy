use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BenchmarkCase {
    pub format_version: String,
    pub case_id: String,
    pub maturity: CaseMaturity,
    pub provenance: Provenance,
    pub units: UnitSystem,
    pub timebase: Timebase,
    pub model_profile: ModelProfile,
    pub network: PowerNetwork,
    pub road_network: RoadNetwork,
    pub faults: Vec<Fault>,
    pub repair_crews: Vec<RepairCrew>,
    pub mobile_power_sources: Vec<MobilePowerSource>,
    pub objective: Objective,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CaseMaturity {
    SyntheticFixture,
    HistoricalReplay,
    FieldObserved,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Provenance {
    pub dataset_id: String,
    pub source_repository: String,
    pub source_commit: String,
    pub source_license_ref: String,
    pub conversion_receipt_ref: String,
    pub source_artifacts: Vec<SourceArtifact>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceArtifact {
    pub path: String,
    pub vcs_object_id: String,
    pub vcs_object_format: VcsObjectFormat,
    pub raw_sha256: Option<String>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VcsObjectFormat {
    GitSha1,
    GitSha256,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UnitSystem {
    pub length: String,
    pub time: String,
    pub active_power: String,
    pub reactive_power: String,
    pub apparent_power: String,
    pub energy: String,
    pub work: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Timebase {
    pub unit: String,
    pub step_size: u32,
    pub horizon_steps: u32,
    pub first_step_semantics: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ModelProfile {
    pub electrical_data_profile: ElectricalDataProfile,
    pub topology_constraints: Vec<TopologyConstraint>,
    pub mps_constraints: Vec<String>,
    pub independent_validator_ref: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ElectricalDataProfile {
    TopologyOnly,
    BranchImpedanceAndLimits,
    CompleteAcModelInputs,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TopologyConstraint {
    Radiality,
    SourceReachability,
    FaultIsolation,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PowerNetwork {
    pub nodes: Vec<PowerNode>,
    pub branches: Vec<PowerBranch>,
    pub switches: Vec<Switch>,
    pub loads: Vec<Load>,
    pub grid_sources: Vec<GridSource>,
    pub electrical_settings: Option<ElectricalSettings>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PowerNode {
    pub node_id: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PowerBranch {
    pub branch_id: String,
    pub from_node_id: String,
    pub to_node_id: String,
    pub length_km: f64,
    pub normally_energized: bool,
    pub electrical: Option<BranchElectricalData>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BranchElectricalData {
    pub resistance_ohm: f64,
    pub reactance_ohm: f64,
    pub ampacity_a: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ElectricalSettings {
    pub base_mva: f64,
    pub base_kv: f64,
    pub voltage_min_pu: f64,
    pub voltage_max_pu: f64,
    pub model_reference: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Switch {
    pub switch_id: String,
    pub from_node_id: String,
    pub to_node_id: String,
    pub initially_closed: bool,
    pub normally_open_tie: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Load {
    pub load_id: String,
    pub node_id: String,
    pub active_power_kw: f64,
    pub reactive_power_kvar: f64,
    pub priority_weight: f64,
    pub essential_service_class: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GridSource {
    pub source_id: String,
    pub node_id: String,
    pub maximum_active_power_kw: f64,
    pub maximum_reactive_power_kvar: f64,
    pub maximum_apparent_power_kva: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RoadNetwork {
    pub nodes: Vec<RoadNode>,
    pub edges: Vec<RoadEdge>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RoadNode {
    pub node_id: String,
    pub power_node_id: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RoadEdge {
    pub edge_id: String,
    pub from_node_id: String,
    pub to_node_id: String,
    pub length_km: f64,
    pub nominal_speed_kph: f64,
    pub closed_at_start: bool,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Fault {
    pub fault_id: String,
    pub power_branch_id: String,
    pub position_fraction: f64,
    pub work_units: f64,
    pub maximum_parallel_crews: u32,
    pub required_skill_ref: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RepairCrew {
    pub crew_id: String,
    pub starting_road_node_id: String,
    pub travel_speed_kph: f64,
    pub work_units_per_step: f64,
    pub skill_refs: Vec<String>,
    pub resource_capacities: Vec<ResourceCapacity>,
    pub calendar: ShiftCalendar,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ResourceCapacity {
    pub resource_ref: String,
    pub capacity_units: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ShiftCalendar {
    pub timezone_ref: String,
    pub windows: Vec<ShiftWindow>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ShiftWindow {
    pub start_step: u32,
    pub end_step: u32,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MobilePowerSource {
    pub mobile_source_id: String,
    pub starting_road_node_id: String,
    pub maximum_active_power_kw: f64,
    pub maximum_apparent_power_kva: f64,
    pub available_energy_kwh: f64,
    pub fuel_or_energy_model_ref: String,
    pub permitted_connection_node_ids: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Objective {
    pub objective_id: String,
    pub metric: String,
    pub expression_ref: String,
    pub horizon_steps: u32,
    pub load_priority_weight_semantics: String,
}

pub fn canonical_case_sha256(case: &BenchmarkCase) -> Result<String, serde_json::Error> {
    let canonical = serde_json::to_value(case)?;
    let bytes = serde_json::to_vec(&canonical)?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn valid_id(value: &str) -> bool {
    !value.trim().is_empty()
}

fn finite_positive(value: f64) -> bool {
    value.is_finite() && value > 0.0
}

fn finite_nonnegative(value: f64) -> bool {
    value.is_finite() && value >= 0.0
}

fn unique<'a>(values: impl Iterator<Item = &'a str>) -> bool {
    let mut seen = BTreeSet::new();
    values.into_iter().all(|value| valid_id(value) && seen.insert(value))
}

pub fn validate_case(case: &BenchmarkCase) -> Vec<String> {
    let mut errors = Vec::new();
    if case.format_version != "0.1.0" || !valid_id(&case.case_id) {
        errors.push("case format version or identity is invalid".to_string());
    }

    let provenance = &case.provenance;
    if !valid_id(&provenance.dataset_id)
        || !valid_id(&provenance.source_repository)
        || provenance.source_commit.len() != 40
        || !provenance.source_commit.bytes().all(|byte| byte.is_ascii_hexdigit())
        || !valid_id(&provenance.source_license_ref)
        || !valid_id(&provenance.conversion_receipt_ref)
    {
        errors.push("provenance identity, commit or licence reference is invalid".to_string());
    }
    if !unique(provenance.source_artifacts.iter().map(|artifact| artifact.path.as_str())) {
        errors.push("source artifact paths must be non-empty and unique".to_string());
    }
    for artifact in &provenance.source_artifacts {
        let expected_length = match artifact.vcs_object_format {
            VcsObjectFormat::GitSha1 => 40,
            VcsObjectFormat::GitSha256 => 64,
        };
        if artifact.vcs_object_id.len() != expected_length
            || !artifact.vcs_object_id.bytes().all(|byte| byte.is_ascii_hexdigit())
            || artifact.raw_sha256.as_ref().is_some_and(|digest| {
                digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
            })
        {
            errors.push(format!("source artifact {} has an invalid object ID or raw SHA-256", artifact.path));
        }
    }

    let units = &case.units;
    if [ &units.length, &units.time, &units.active_power, &units.reactive_power,
        &units.apparent_power, &units.energy, &units.work ].iter().any(|unit| !valid_id(unit))
    {
        errors.push("all physical and work units must be explicitly declared".to_string());
    }
    if !valid_id(&case.timebase.unit)
        || case.timebase.step_size == 0
        || case.timebase.horizon_steps == 0
        || !valid_id(&case.timebase.first_step_semantics)
        || case.objective.horizon_steps != case.timebase.horizon_steps
    {
        errors.push("timebase and objective horizon are invalid or inconsistent".to_string());
    }
    if !valid_id(&case.objective.objective_id)
        || !valid_id(&case.objective.metric)
        || !valid_id(&case.objective.expression_ref)
        || !valid_id(&case.objective.load_priority_weight_semantics)
    {
        errors.push("objective identity or semantics are incomplete".to_string());
    }

    let model = &case.model_profile;
    if !valid_id(&model.independent_validator_ref) {
        errors.push("independent validator reference is required".to_string());
    }
    if model.topology_constraints.is_empty() || model.mps_constraints.is_empty() {
        errors.push("topology and mobile-source constraints must be declared".to_string());
    }

    let network = &case.network;
    if network.nodes.is_empty() || network.branches.is_empty() || network.grid_sources.is_empty() {
        errors.push("network requires nodes, branches and at least one grid source".to_string());
    }
    if !unique(network.nodes.iter().map(|node| node.node_id.as_str())) {
        errors.push("power node IDs must be non-empty and unique".to_string());
    }
    let node_ids: BTreeSet<&str> = network.nodes.iter().map(|node| node.node_id.as_str()).collect();
    let mut branch_ids = BTreeSet::new();
    for branch in &network.branches {
        if !valid_id(&branch.branch_id)
            || !branch_ids.insert(branch.branch_id.as_str())
            || !node_ids.contains(branch.from_node_id.as_str())
            || !node_ids.contains(branch.to_node_id.as_str())
            || branch.from_node_id == branch.to_node_id
            || !finite_positive(branch.length_km)
        {
            errors.push(format!("power branch {} has invalid identity, endpoints or length", branch.branch_id));
        }
        if let Some(electrical) = &branch.electrical {
            if !finite_nonnegative(electrical.resistance_ohm)
                || !finite_nonnegative(electrical.reactance_ohm)
                || !finite_positive(electrical.ampacity_a)
            {
                errors.push(format!("power branch {} has invalid electrical parameters", branch.branch_id));
            }
        }
    }
    let mut switch_ids = BTreeSet::new();
    for switch in &network.switches {
        if !valid_id(&switch.switch_id)
            || !switch_ids.insert(switch.switch_id.as_str())
            || !node_ids.contains(switch.from_node_id.as_str())
            || !node_ids.contains(switch.to_node_id.as_str())
            || switch.from_node_id == switch.to_node_id
            || (switch.normally_open_tie && switch.initially_closed)
        {
            errors.push(format!("switch {} has invalid endpoints, ID or normally-open state", switch.switch_id));
        }
    }
    let mut load_ids = BTreeSet::new();
    for load in &network.loads {
        if !valid_id(&load.load_id)
            || !load_ids.insert(load.load_id.as_str())
            || !node_ids.contains(load.node_id.as_str())
            || !finite_nonnegative(load.active_power_kw)
            || !load.reactive_power_kvar.is_finite()
            || !finite_nonnegative(load.priority_weight)
            || !valid_id(&load.essential_service_class)
        {
            errors.push(format!("load {} has invalid identity, node, demand or priority", load.load_id));
        }
    }
    let mut source_ids = BTreeSet::new();
    for source in &network.grid_sources {
        if !valid_id(&source.source_id)
            || !source_ids.insert(source.source_id.as_str())
            || !node_ids.contains(source.node_id.as_str())
            || !finite_nonnegative(source.maximum_active_power_kw)
            || !source.maximum_reactive_power_kvar.is_finite()
            || !finite_positive(source.maximum_apparent_power_kva)
        {
            errors.push(format!("grid source {} has invalid identity, node or capability", source.source_id));
        }
    }

    match model.electrical_data_profile {
        ElectricalDataProfile::TopologyOnly => {
            if network.electrical_settings.is_some() || network.branches.iter().any(|branch| branch.electrical.is_some()) {
                errors.push("topology-only profile cannot carry partial electrical settings/branch parameters".to_string());
            }
        }
        ElectricalDataProfile::BranchImpedanceAndLimits => {
            if network.electrical_settings.is_some()
                || network.branches.iter().any(|branch| branch.electrical.is_none())
            {
                errors.push("branch-impedance profile requires electrical data on every power branch and no incomplete network settings".to_string());
            }
        }
        ElectricalDataProfile::CompleteAcModelInputs => {
            let settings = network.electrical_settings.as_ref();
            if settings.is_none()
                || network.branches.iter().any(|branch| branch.electrical.is_none())
                || settings.is_some_and(|item| {
                    !finite_positive(item.base_mva)
                        || !finite_positive(item.base_kv)
                        || !item.voltage_min_pu.is_finite()
                        || !item.voltage_max_pu.is_finite()
                        || item.voltage_min_pu <= 0.0
                        || item.voltage_max_pu <= item.voltage_min_pu
                        || !valid_id(&item.model_reference)
                })
            {
                errors.push("complete AC input profile requires base quantities, voltage limits, model reference and electrical data on every branch".to_string());
            }
        }
    }

    let roads = &case.road_network;
    if roads.nodes.is_empty() {
        errors.push("road network requires at least one road node".to_string());
    }
    if !unique(roads.nodes.iter().map(|node| node.node_id.as_str())) {
        errors.push("road node IDs must be non-empty and unique".to_string());
    }
    let road_ids: BTreeSet<&str> = roads.nodes.iter().map(|node| node.node_id.as_str()).collect();
    if roads.nodes.iter().any(|node| !node_ids.contains(node.power_node_id.as_str())) {
        errors.push("every road node must map to a known power node".to_string());
    }
    let mut road_edge_ids = BTreeSet::new();
    for edge in &roads.edges {
        if !valid_id(&edge.edge_id)
            || !road_edge_ids.insert(edge.edge_id.as_str())
            || !road_ids.contains(edge.from_node_id.as_str())
            || !road_ids.contains(edge.to_node_id.as_str())
            || edge.from_node_id == edge.to_node_id
            || !finite_positive(edge.length_km)
            || !finite_positive(edge.nominal_speed_kph)
        {
            errors.push(format!("road edge {} has invalid identity, endpoints or travel parameters", edge.edge_id));
        }
    }

    let mut fault_ids = BTreeSet::new();
    for fault in &case.faults {
        if !valid_id(&fault.fault_id)
            || !fault_ids.insert(fault.fault_id.as_str())
            || !branch_ids.contains(fault.power_branch_id.as_str())
            || !fault.position_fraction.is_finite()
            || !(0.0..=1.0).contains(&fault.position_fraction)
            || !finite_positive(fault.work_units)
            || fault.maximum_parallel_crews == 0
            || !valid_id(&fault.required_skill_ref)
        {
            errors.push(format!("fault {} has invalid branch, location, work or capacity", fault.fault_id));
        }
    }

    let mut crew_ids = BTreeSet::new();
    for crew in &case.repair_crews {
        if !valid_id(&crew.crew_id)
            || !crew_ids.insert(crew.crew_id.as_str())
            || !road_ids.contains(crew.starting_road_node_id.as_str())
            || !finite_positive(crew.travel_speed_kph)
            || !finite_positive(crew.work_units_per_step)
            || crew.skill_refs.is_empty()
            || !unique(crew.skill_refs.iter().map(String::as_str))
            || !valid_id(&crew.calendar.timezone_ref)
            || crew.calendar.windows.is_empty()
        {
            errors.push(format!("crew {} has invalid identity, location, skills, rate or calendar", crew.crew_id));
        }
        for window in &crew.calendar.windows {
            if window.start_step >= window.end_step || window.end_step > case.timebase.horizon_steps {
                errors.push(format!("crew {} has a shift window outside the case horizon", crew.crew_id));
            }
        }
        let mut resource_ids = BTreeSet::new();
        for resource in &crew.resource_capacities {
            if !valid_id(&resource.resource_ref)
                || resource.capacity_units == 0
                || !resource_ids.insert(resource.resource_ref.as_str())
            {
                errors.push(format!("crew {} has an invalid or duplicate resource capacity", crew.crew_id));
            }
        }
    }
    for fault in &case.faults {
        if !case.repair_crews.iter().any(|crew| crew.skill_refs.iter().any(|skill| skill == &fault.required_skill_ref)) {
            errors.push(format!("no declared repair crew has required skill {}", fault.required_skill_ref));
        }
    }

    let mut mobile_ids = BTreeSet::new();
    for mobile in &case.mobile_power_sources {
        if !valid_id(&mobile.mobile_source_id)
            || !mobile_ids.insert(mobile.mobile_source_id.as_str())
            || !road_ids.contains(mobile.starting_road_node_id.as_str())
            || !finite_nonnegative(mobile.maximum_active_power_kw)
            || !finite_positive(mobile.maximum_apparent_power_kva)
            || !finite_nonnegative(mobile.available_energy_kwh)
            || !valid_id(&mobile.fuel_or_energy_model_ref)
            || mobile.permitted_connection_node_ids.is_empty()
            || mobile.permitted_connection_node_ids.iter().any(|node| !node_ids.contains(node.as_str()))
        {
            errors.push(format!("mobile source {} has invalid capability, location or connection nodes", mobile.mobile_source_id));
        }
    }

    errors
}

pub fn validate_json(input: &str) -> Result<BenchmarkCase, Vec<String>> {
    let case: BenchmarkCase = serde_json::from_str(input)
        .map_err(|error| vec![format!("case JSON decode error: {error}")])?;
    let errors = validate_case(&case);
    if errors.is_empty() { Ok(case) } else { Err(errors) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> BenchmarkCase {
        serde_json::from_str(include_str!("../fixtures/synthetic-restoration-case-v0.1.json"))
            .expect("fixture must deserialize")
    }

    #[test]
    fn synthetic_case_passes_structural_contract() {
        assert!(validate_case(&fixture()).is_empty());
    }

    #[test]
    fn deterministic_case_digest_is_stable() {
        let case = fixture();
        assert_eq!(canonical_case_sha256(&case).unwrap(), canonical_case_sha256(&case).unwrap());
    }

    #[test]
    fn maturity_is_explicit_and_does_not_imply_field_validation() {
        assert_eq!(fixture().maturity, CaseMaturity::SyntheticFixture);
    }

    #[test]
    fn all_network_references_resolve() {
        let case = fixture();
        let errors = validate_case(&case);
        assert!(errors.is_empty(), "{errors:?}");
    }

    #[test]
    fn unknown_fault_branch_is_rejected() {
        let mut case = fixture();
        case.faults[0].power_branch_id = "missing-branch".to_string();
        assert!(!validate_case(&case).is_empty());
    }

    #[test]
    fn duplicate_power_node_id_is_rejected() {
        let mut case = fixture();
        case.network.nodes.push(case.network.nodes[0].clone());
        assert!(!validate_case(&case).is_empty());
    }

    #[test]
    fn out_of_range_fault_position_is_rejected() {
        let mut case = fixture();
        case.faults[0].position_fraction = 1.01;
        assert!(!validate_case(&case).is_empty());
    }

    #[test]
    fn fault_requires_a_crew_with_matching_skill() {
        let mut case = fixture();
        case.repair_crews[0].skill_refs = vec!["unqualified-skill".to_string()];
        case.repair_crews[1].skill_refs = vec!["unqualified-skill".to_string()];
        assert!(!validate_case(&case).is_empty());
    }

    #[test]
    fn shift_windows_cannot_exceed_horizon() {
        let mut case = fixture();
        case.repair_crews[0].calendar.windows[0].end_step = 999;
        assert!(!validate_case(&case).is_empty());
    }

    #[test]
    fn mobile_connection_must_be_a_known_power_node() {
        let mut case = fixture();
        case.mobile_power_sources[0].permitted_connection_node_ids = vec!["N999".to_string()];
        assert!(!validate_case(&case).is_empty());
    }

    #[test]
    fn negative_or_nonfinite_load_demand_is_rejected() {
        let mut case = fixture();
        case.network.loads[0].active_power_kw = f64::NAN;
        assert!(!validate_case(&case).is_empty());
    }

    #[test]
    fn topology_only_profile_cannot_claim_partial_ac_data() {
        let mut case = fixture();
        case.network.branches[0].electrical = Some(BranchElectricalData {
            resistance_ohm: 0.2,
            reactance_ohm: 0.1,
            ampacity_a: 400.0,
        });
        assert!(!validate_case(&case).is_empty());
    }

    #[test]
    fn branch_impedance_profile_requires_data_on_every_branch() {
        let mut case = fixture();
        case.model_profile.electrical_data_profile = ElectricalDataProfile::BranchImpedanceAndLimits;
        assert!(!validate_case(&case).is_empty());
    }

    #[test]
    fn complete_ac_input_profile_requires_complete_input_declaration() {
        let mut case = fixture();
        case.model_profile.electrical_data_profile = ElectricalDataProfile::CompleteAcModelInputs;
        assert!(!validate_case(&case).is_empty());
    }

    #[test]
    fn normally_open_tie_cannot_start_closed() {
        let mut case = fixture();
        case.network.switches[0].initially_closed = true;
        assert!(!validate_case(&case).is_empty());
    }

    #[test]
    fn objective_and_time_horizon_must_match() {
        let mut case = fixture();
        case.objective.horizon_steps += 1;
        assert!(!validate_case(&case).is_empty());
    }

    #[test]
    fn artifact_hash_shapes_are_explicit() {
        let mut case = fixture();
        case.provenance.source_artifacts.push(SourceArtifact {
            path: "source-case.json".to_string(),
            vcs_object_id: "short".to_string(),
            vcs_object_format: VcsObjectFormat::GitSha1,
            raw_sha256: None,
        });
        assert!(!validate_case(&case).is_empty());
    }
}
