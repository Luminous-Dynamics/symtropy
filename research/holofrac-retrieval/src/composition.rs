//! Finite typed-module composition and held-out grammar benchmark.
//!
//! This qualifies exact port-contract composition only. It does not generate
//! geometry or certify structural, environmental, code or life-safety outcomes.

use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ModuleFamily {
    StructuralFrame,
    HabitableSpace,
    AirHandler,
    WaterRiser,
}

impl ModuleFamily {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::StructuralFrame => "structural_frame",
            Self::HabitableSpace => "habitable_space",
            Self::AirHandler => "air_handler",
            Self::WaterRiser => "water_riser",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum PortKind {
    StructuralLoad,
    AirSupply,
    WaterSupply,
}

impl PortKind {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::StructuralLoad => "structural_load",
            Self::AirSupply => "air_supply",
            Self::WaterSupply => "water_supply",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PortDirection {
    Input,
    Output,
    Bidirectional,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpatialLevel {
    Building,
    Storey,
    Room,
    Assembly,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PortContract {
    pub id: String,
    pub kind: PortKind,
    pub direction: PortDirection,
    pub unit: String,
    pub coordinate_frame: String,
    pub maximum_capacity: f64,
    pub alignment_tolerance_mm: f64,
    pub required: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ModuleContract {
    pub id: String,
    pub family: ModuleFamily,
    pub scale: SpatialLevel,
    pub ports: Vec<PortContract>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PortConnection {
    pub source_module: String,
    pub source_port: String,
    pub target_module: String,
    pub target_port: String,
    pub demand: f64,
    pub alignment_error_mm: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Composition {
    pub modules: Vec<ModuleContract>,
    pub connections: Vec<PortConnection>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CompositionViolation {
    EmptyModuleId,
    DuplicateModuleId(String),
    EmptyPortId(String),
    DuplicatePortId(String, String),
    InvalidPortCapacity(String, String),
    InvalidPortTolerance(String, String),
    UnknownModule(String),
    UnknownPort(String, String),
    SelfConnection(String),
    DuplicateConnection(String),
    PortKindMismatch(String),
    UnitMismatch(String),
    CoordinateFrameMismatch(String),
    DirectionMismatch(String),
    InvalidDemand(String),
    CapacityExceeded(String, String),
    AlignmentOutsideTolerance(String),
    RequiredPortUnconnected(String, String),
}

impl CompositionViolation {
    pub const fn code(&self) -> &'static str {
        match self {
            Self::EmptyModuleId => "empty_module_id",
            Self::DuplicateModuleId(_) => "duplicate_module_id",
            Self::EmptyPortId(_) => "empty_port_id",
            Self::DuplicatePortId(_, _) => "duplicate_port_id",
            Self::InvalidPortCapacity(_, _) => "invalid_port_capacity",
            Self::InvalidPortTolerance(_, _) => "invalid_port_tolerance",
            Self::UnknownModule(_) => "unknown_module",
            Self::UnknownPort(_, _) => "unknown_port",
            Self::SelfConnection(_) => "self_connection",
            Self::DuplicateConnection(_) => "duplicate_connection",
            Self::PortKindMismatch(_) => "port_kind_mismatch",
            Self::UnitMismatch(_) => "unit_mismatch",
            Self::CoordinateFrameMismatch(_) => "coordinate_frame_mismatch",
            Self::DirectionMismatch(_) => "direction_mismatch",
            Self::InvalidDemand(_) => "invalid_demand",
            Self::CapacityExceeded(_, _) => "capacity_exceeded",
            Self::AlignmentOutsideTolerance(_) => "alignment_outside_tolerance",
            Self::RequiredPortUnconnected(_, _) => "required_port_unconnected",
        }
    }
}

impl Composition {
    /// Exact deterministic signature used only to define the finite split.
    /// It is not a cryptographic digest or a substitute for a source manifest.
    pub fn signature(&self) -> String {
        let mut families: Vec<_> = self.modules.iter().map(|m| m.family.as_str()).collect();
        families.sort_unstable();

        let family_by_module: HashMap<_, _> = self
            .modules
            .iter()
            .map(|module| (module.id.as_str(), module.family))
            .collect();
        let port_by_key: HashMap<_, _> = self
            .modules
            .iter()
            .flat_map(|module| {
                module
                    .ports
                    .iter()
                    .map(move |port| ((module.id.as_str(), port.id.as_str()), port.kind))
            })
            .collect();

        let mut edges = Vec::new();
        for connection in &self.connections {
            let Some(source_family) = family_by_module.get(connection.source_module.as_str())
            else {
                continue;
            };
            let Some(target_family) = family_by_module.get(connection.target_module.as_str())
            else {
                continue;
            };
            let Some(kind) = port_by_key.get(&(
                connection.source_module.as_str(),
                connection.source_port.as_str(),
            )) else {
                continue;
            };
            edges.push(format!(
                "{}>{}:{}",
                source_family.as_str(),
                target_family.as_str(),
                kind.as_str()
            ));
        }
        edges.sort();

        format!("modules={}|edges={}", families.join(","), edges.join(","))
    }

    pub fn validate(&self) -> Vec<CompositionViolation> {
        let mut violations = Vec::new();
        let mut modules = HashMap::with_capacity(self.modules.len());
        let mut ports = HashMap::new();

        for module in &self.modules {
            if module.id.trim().is_empty() {
                violations.push(CompositionViolation::EmptyModuleId);
                continue;
            }
            if modules.insert(module.id.as_str(), module).is_some() {
                violations.push(CompositionViolation::DuplicateModuleId(
                    module.id.clone(),
                ));
                continue;
            }

            let mut local_port_ids = HashSet::new();
            for port in &module.ports {
                if port.id.trim().is_empty() {
                    violations.push(CompositionViolation::EmptyPortId(
                        module.id.clone(),
                    ));
                    continue;
                }
                if !local_port_ids.insert(port.id.as_str()) {
                    violations.push(CompositionViolation::DuplicatePortId(
                        module.id.clone(),
                        port.id.clone(),
                    ));
                    continue;
                }
                if !port.maximum_capacity.is_finite() || port.maximum_capacity <= 0.0 {
                    violations.push(CompositionViolation::InvalidPortCapacity(
                        module.id.clone(),
                        port.id.clone(),
                    ));
                }
                if !port.alignment_tolerance_mm.is_finite()
                    || port.alignment_tolerance_mm < 0.0
                {
                    violations.push(CompositionViolation::InvalidPortTolerance(
                        module.id.clone(),
                        port.id.clone(),
                    ));
                }
                ports.insert((module.id.as_str(), port.id.as_str()), port);
            }
        }

        let mut connected = HashSet::new();
        let mut usage: HashMap<(String, String), f64> = HashMap::new();
        let mut connection_ids = HashSet::new();

        for connection in &self.connections {
            let display = format!(
                "{}.{} -> {}.{}",
                connection.source_module,
                connection.source_port,
                connection.target_module,
                connection.target_port
            );

            if connection.source_module == connection.target_module {
                violations.push(CompositionViolation::SelfConnection(display));
                continue;
            }

            if !modules.contains_key(connection.source_module.as_str()) {
                violations.push(CompositionViolation::UnknownModule(
                    connection.source_module.clone(),
                ));
                continue;
            }
            if !modules.contains_key(connection.target_module.as_str()) {
                violations.push(CompositionViolation::UnknownModule(
                    connection.target_module.clone(),
                ));
                continue;
            }

            let source_key = (
                connection.source_module.as_str(),
                connection.source_port.as_str(),
            );
            let target_key = (
                connection.target_module.as_str(),
                connection.target_port.as_str(),
            );
            let Some(source_port) = ports.get(&source_key).copied() else {
                violations.push(CompositionViolation::UnknownPort(
                    connection.source_module.clone(),
                    connection.source_port.clone(),
                ));
                continue;
            };
            let Some(target_port) = ports.get(&target_key).copied() else {
                violations.push(CompositionViolation::UnknownPort(
                    connection.target_module.clone(),
                    connection.target_port.clone(),
                ));
                continue;
            };

            if !connection_ids.insert(display.clone()) {
                violations.push(CompositionViolation::DuplicateConnection(display));
                continue;
            }

            if source_port.kind != target_port.kind {
                violations.push(CompositionViolation::PortKindMismatch(display.clone()));
            }
            if source_port.unit != target_port.unit {
                violations.push(CompositionViolation::UnitMismatch(display.clone()));
            }
            if source_port.coordinate_frame != target_port.coordinate_frame {
                violations.push(CompositionViolation::CoordinateFrameMismatch(
                    display.clone(),
                ));
            }

            let direction_ok = matches!(
                (source_port.direction, target_port.direction),
                (PortDirection::Output, PortDirection::Input)
                    | (PortDirection::Output, PortDirection::Bidirectional)
                    | (PortDirection::Bidirectional, PortDirection::Input)
                    | (PortDirection::Bidirectional, PortDirection::Bidirectional)
            );
            if !direction_ok {
                violations.push(CompositionViolation::DirectionMismatch(display.clone()));
            }

            if !connection.demand.is_finite() || connection.demand <= 0.0 {
                violations.push(CompositionViolation::InvalidDemand(display.clone()));
            } else {
                *usage
                    .entry((connection.source_module.clone(), connection.source_port.clone()))
                    .or_default() += connection.demand;
                *usage
                    .entry((connection.target_module.clone(), connection.target_port.clone()))
                    .or_default() += connection.demand;
                if connection.demand > source_port.maximum_capacity
                    || connection.demand > target_port.maximum_capacity
                {
                    violations.push(CompositionViolation::CapacityExceeded(
                        connection.source_module.clone(),
                        connection.source_port.clone(),
                    ));
                }
            }

            if !connection.alignment_error_mm.is_finite()
                || connection.alignment_error_mm < 0.0
                || connection.alignment_error_mm > source_port.alignment_tolerance_mm
                || connection.alignment_error_mm > target_port.alignment_tolerance_mm
            {
                violations.push(CompositionViolation::AlignmentOutsideTolerance(
                    display.clone(),
                ));
            }

            connected.insert((
                connection.source_module.clone(),
                connection.source_port.clone(),
            ));
            connected.insert((
                connection.target_module.clone(),
                connection.target_port.clone(),
            ));
        }

        for module in &self.modules {
            for port in &module.ports {
                let key = (module.id.clone(), port.id.clone());
                if port.required && !connected.contains(&key) {
                    violations.push(CompositionViolation::RequiredPortUnconnected(
                        module.id.clone(),
                        port.id.clone(),
                    ));
                }
                if usage.get(&key).copied().unwrap_or(0.0) > port.maximum_capacity {
                    violations.push(CompositionViolation::CapacityExceeded(
                        module.id.clone(),
                        port.id.clone(),
                    ));
                }
            }
        }

        violations
    }
}

fn port(
    id: &str,
    kind: PortKind,
    direction: PortDirection,
    unit: &str,
    required: bool,
) -> PortContract {
    PortContract {
        id: id.to_owned(),
        kind,
        direction,
        unit: unit.to_owned(),
        coordinate_frame: "building-grid-v1".to_owned(),
        maximum_capacity: 100.0,
        alignment_tolerance_mm: 5.0,
        required,
    }
}

fn room(required: [bool; 3]) -> ModuleContract {
    ModuleContract {
        id: "room-a".to_owned(),
        family: ModuleFamily::HabitableSpace,
        scale: SpatialLevel::Room,
        ports: vec![
            port(
                "support-in",
                PortKind::StructuralLoad,
                PortDirection::Input,
                "kN",
                required[0],
            ),
            port(
                "air-in",
                PortKind::AirSupply,
                PortDirection::Input,
                "m3/s",
                required[1],
            ),
            port(
                "water-in",
                PortKind::WaterSupply,
                PortDirection::Input,
                "L/s",
                required[2],
            ),
        ],
    }
}

fn source_module(
    id: &str,
    family: ModuleFamily,
    port_id: &str,
    kind: PortKind,
    unit: &str,
) -> ModuleContract {
    ModuleContract {
        id: id.to_owned(),
        family,
        scale: SpatialLevel::Building,
        ports: vec![port(
            port_id,
            kind,
            PortDirection::Output,
            unit,
            true,
        )],
    }
}

fn connection(
    source_module: &str,
    source_port: &str,
    target_module: &str,
    target_port: &str,
) -> PortConnection {
    PortConnection {
        source_module: source_module.to_owned(),
        source_port: source_port.to_owned(),
        target_module: target_module.to_owned(),
        target_port: target_port.to_owned(),
        demand: 10.0,
        alignment_error_mm: 1.0,
    }
}

fn training_compositions() -> Vec<Composition> {
    vec![
        Composition {
            modules: vec![
                source_module(
                    "frame-a",
                    ModuleFamily::StructuralFrame,
                    "load-out",
                    PortKind::StructuralLoad,
                    "kN",
                ),
                room([true, false, false]),
            ],
            connections: vec![connection(
                "frame-a",
                "load-out",
                "room-a",
                "support-in",
            )],
        },
        Composition {
            modules: vec![
                source_module(
                    "ahu-a",
                    ModuleFamily::AirHandler,
                    "air-out",
                    PortKind::AirSupply,
                    "m3/s",
                ),
                room([false, true, false]),
            ],
            connections: vec![connection(
                "ahu-a",
                "air-out",
                "room-a",
                "air-in",
            )],
        },
        Composition {
            modules: vec![
                source_module(
                    "riser-a",
                    ModuleFamily::WaterRiser,
                    "water-out",
                    PortKind::WaterSupply,
                    "L/s",
                ),
                room([false, false, true]),
            ],
            connections: vec![connection(
                "riser-a",
                "water-out",
                "room-a",
                "water-in",
            )],
        },
    ]
}

fn held_out_composition() -> Composition {
    Composition {
        modules: vec![
            source_module(
                "frame-b",
                ModuleFamily::StructuralFrame,
                "load-out",
                PortKind::StructuralLoad,
                "kN",
            ),
            room([true, true, true]),
            source_module(
                "ahu-b",
                ModuleFamily::AirHandler,
                "air-out",
                PortKind::AirSupply,
                "m3/s",
            ),
            source_module(
                "riser-b",
                ModuleFamily::WaterRiser,
                "water-out",
                PortKind::WaterSupply,
                "L/s",
            ),
        ],
        connections: vec![
            connection("frame-b", "load-out", "room-a", "support-in"),
            connection("ahu-b", "air-out", "room-a", "air-in"),
            connection("riser-b", "water-out", "room-a", "water-in"),
        ],
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CompositionGeneralizationReport {
    pub training_composition_count: usize,
    pub held_out_module_count: usize,
    pub held_out_connection_count: usize,
    pub held_out_signature_unseen: bool,
    pub all_held_out_pair_rules_seen: bool,
    pub exact_port_contract_valid: bool,
    pub violation_count: usize,
}

pub fn run_compositional_generalization_fixture() -> CompositionGeneralizationReport {
    let training = training_compositions();
    let training_signatures: HashSet<_> =
        training.iter().map(Composition::signature).collect();
    let training_pair_rules: HashSet<_> = training
        .iter()
        .flat_map(|composition| {
            let families: HashMap<_, _> = composition
                .modules
                .iter()
                .map(|module| (module.id.as_str(), module.family))
                .collect();
            composition.connections.iter().filter_map(move |edge| {
                let source = families.get(edge.source_module.as_str())?;
                let target = families.get(edge.target_module.as_str())?;
                let source_module = composition
                    .modules
                    .iter()
                    .find(|module| module.id == edge.source_module)?;
                let source_port = source_module
                    .ports
                    .iter()
                    .find(|port| port.id == edge.source_port)?;
                Some(format!(
                    "{}>{}:{}",
                    source.as_str(),
                    target.as_str(),
                    source_port.kind.as_str()
                ))
            })
        })
        .collect();

    let held_out = held_out_composition();
    let held_out_signature_unseen = !training_signatures.contains(&held_out.signature());
    let family_by_module: HashMap<_, _> = held_out
        .modules
        .iter()
        .map(|module| (module.id.as_str(), module.family))
        .collect();
    let held_out_pair_rules_seen = held_out.connections.iter().all(|edge| {
        let Some(source) = family_by_module.get(edge.source_module.as_str()) else {
            return false;
        };
        let Some(target) = family_by_module.get(edge.target_module.as_str()) else {
            return false;
        };
        let Some(source_module) = held_out
            .modules
            .iter()
            .find(|module| module.id == edge.source_module)
        else {
            return false;
        };
        let Some(source_port) = source_module
            .ports
            .iter()
            .find(|port| port.id == edge.source_port)
        else {
            return false;
        };
        training_pair_rules.contains(&format!(
            "{}>{}:{}",
            source.as_str(),
            target.as_str(),
            source_port.kind.as_str()
        ))
    });
    let violations = held_out.validate();

    CompositionGeneralizationReport {
        training_composition_count: training.len(),
        held_out_module_count: held_out.modules.len(),
        held_out_connection_count: held_out.connections.len(),
        held_out_signature_unseen,
        all_held_out_pair_rules_seen: held_out_pair_rules_seen,
        exact_port_contract_valid: violations.is_empty(),
        violation_count: violations.len(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn held_out_three_system_composition_uses_known_pairwise_rules() {
        let report = run_compositional_generalization_fixture();
        assert_eq!(report.training_composition_count, 3);
        assert_eq!(report.held_out_module_count, 4);
        assert_eq!(report.held_out_connection_count, 3);
        assert!(report.held_out_signature_unseen);
        assert!(report.all_held_out_pair_rules_seen);
        assert!(report.exact_port_contract_valid);
        assert_eq!(report.violation_count, 0);
    }

    #[test]
    fn unit_frame_direction_capacity_and_tolerance_near_misses_fail_closed() {
        let mut fixture = held_out_composition();
        fixture.modules[1].ports[1].unit = "Pa".to_owned();
        assert!(fixture
            .validate()
            .iter()
            .any(|violation| violation.code() == "unit_mismatch"));

        let mut fixture = held_out_composition();
        fixture.modules[1].ports[1].coordinate_frame = "storey-local-v2".to_owned();
        assert!(fixture
            .validate()
            .iter()
            .any(|violation| violation.code() == "coordinate_frame_mismatch"));

        let mut fixture = held_out_composition();
        fixture.modules[2].ports[0].direction = PortDirection::Input;
        assert!(fixture
            .validate()
            .iter()
            .any(|violation| violation.code() == "direction_mismatch"));

        let mut fixture = held_out_composition();
        fixture.connections[0].demand = 101.0;
        assert!(fixture
            .validate()
            .iter()
            .any(|violation| violation.code() == "capacity_exceeded"));

        let mut fixture = held_out_composition();
        fixture.connections[0].alignment_error_mm = 6.0;
        assert!(fixture
            .validate()
            .iter()
            .any(|violation| violation.code() == "alignment_outside_tolerance"));
    }

    #[test]
    fn required_ports_and_unknown_endpoints_fail_closed() {
        let mut fixture = held_out_composition();
        fixture.connections.pop();
        let violations = fixture.validate();
        assert!(violations
            .iter()
            .any(|violation| violation.code() == "required_port_unconnected"));

        let mut fixture = held_out_composition();
        fixture.connections[0].target_module = "missing-room".to_owned();
        assert!(fixture
            .validate()
            .iter()
            .any(|violation| violation.code() == "unknown_module"));
    }
}
