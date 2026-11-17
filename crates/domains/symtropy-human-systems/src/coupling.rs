// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Typed, fail-closed semantics for coupling independently specified subsystems.
//!
//! This module validates port/edge contracts. It does not implement a
//! physiological model, infer missing unit conversions, or prove that a
//! source model is biologically valid.

use std::collections::HashSet;

/// SI base dimensions in the order kg, m, s, A, K, mol, cd.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Dimension(pub [i8; 7]);

impl Dimension {
    pub const DIMENSIONLESS: Self = Self([0; 7]);
    pub const TEMPERATURE: Self = Self([0, 0, 0, 0, 1, 0, 0]);
    pub const LENGTH: Self = Self([0, 1, 0, 0, 0, 0, 0]);
    pub const VELOCITY: Self = Self([0, 1, -1, 0, 0, 0, 0]);
    pub const VOLUME_FLOW: Self = Self([0, 3, -1, 0, 0, 0, 0]);
    pub const POWER: Self = Self([1, 2, -3, 0, 0, 0, 0]);
}

/// Quantity kind is required in addition to dimensional exponents.
///
/// Absolute temperatures and temperature differences share dimensions but
/// have different affine-conversion rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QuantityKind {
    Linear,
    AbsoluteTemperature,
    TemperatureDifference,
    Dimensionless,
    Symbolic,
}

/// Unit descriptor. scale_to_si and offset_to_si describe the affine
/// conversion SI = value * scale_to_si + offset_to_si.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UnitSpec {
    pub symbol: &'static str,
    pub semantic_name: &'static str,
    pub dimension: Dimension,
    pub kind: QuantityKind,
    pub scale_to_si: f64,
    pub offset_to_si: f64,
}

impl UnitSpec {
    pub const fn new(
        symbol: &'static str,
        semantic_name: &'static str,
        dimension: Dimension,
        kind: QuantityKind,
        scale_to_si: f64,
        offset_to_si: f64,
    ) -> Self {
        Self {
            symbol,
            semantic_name,
            dimension,
            kind,
            scale_to_si,
            offset_to_si,
        }
    }

    pub const fn celsius() -> Self {
        Self::new(
            "degC",
            "temperature_absolute",
            Dimension::TEMPERATURE,
            QuantityKind::AbsoluteTemperature,
            1.0,
            273.15,
        )
    }

    pub const fn kelvin() -> Self {
        Self::new(
            "K",
            "temperature_absolute",
            Dimension::TEMPERATURE,
            QuantityKind::AbsoluteTemperature,
            1.0,
            0.0,
        )
    }

    pub const fn delta_celsius() -> Self {
        Self::new(
            "delta_degC",
            "temperature_difference",
            Dimension::TEMPERATURE,
            QuantityKind::TemperatureDifference,
            1.0,
            0.0,
        )
    }
}

/// Whether a port receives or emits a model value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortDirection {
    Input,
    Output,
    Bidirectional,
}

/// Semantic role of a port.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortClass {
    State,
    ConservedFlux,
    Rate,
    ControlSignal,
    Mechanical,
    BoundaryCondition,
    Observation,
    Configuration,
}

/// A model-specific, unit-bearing port.
#[derive(Debug, Clone, PartialEq)]
pub struct PortSpec {
    pub port_id: &'static str,
    pub subsystem_id: &'static str,
    pub model_id: &'static str,
    pub variable_name: &'static str,
    pub semantic_name: &'static str,
    pub direction: PortDirection,
    pub class: PortClass,
    pub unit: UnitSpec,
    pub min_step_seconds: f64,
    pub default_step_seconds: f64,
    pub max_step_seconds: f64,
}

/// A direction of physical bookkeeping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BalanceLaw {
    Mass,
    Volume,
    Energy,
    Heat,
    Charge,
    Species,
    Momentum,
    SignalKinetics,
}

/// Balance contract required for a conserved-flow edge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BalanceSpec {
    pub group_id: &'static str,
    pub law: BalanceLaw,
    pub sign_convention: &'static str,
    pub residual_unit: &'static str,
    pub closure_definition: &'static str,
}

/// Unit mapping from source-port numeric values to destination-port values.
///
/// scale and offset are explicit. No conversion is inferred from matching
/// unit labels. reviewed records whether the mapping has been checked against
/// the source equations.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PortMapping {
    pub scale: f64,
    pub offset: f64,
    pub reviewed: bool,
}

/// Directed connection between two subsystem ports.
#[derive(Debug, Clone, PartialEq)]
pub struct CouplingEdge {
    pub edge_id: &'static str,
    pub from_port: &'static str,
    pub to_port: &'static str,
    pub mapping: PortMapping,
    pub delay_seconds: f64,
    pub balance: Option<BalanceSpec>,
}

/// Model identity admitted to this graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelIdentity {
    pub model_id: &'static str,
    pub revision: &'static str,
}

/// Subsystem identity admitted to this graph.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubsystemIdentity {
    pub subsystem_id: &'static str,
    pub model_id: &'static str,
}

/// Proposed graph. Validation is read-only and never mutates a model state.
#[derive(Debug, Clone, PartialEq)]
pub struct CouplingGraph {
    pub models: Vec<ModelIdentity>,
    pub subsystems: Vec<SubsystemIdentity>,
    pub ports: Vec<PortSpec>,
    pub edges: Vec<CouplingEdge>,
}

/// Fail-closed graph validation errors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CouplingError {
    EmptyIdentifier,
    DuplicateModel,
    DuplicateSubsystem,
    DuplicatePort,
    DuplicateEdge,
    UnknownModel,
    UnknownSubsystem,
    UnknownPort,
    PortModelMismatch,
    PortDirectionMismatch,
    IncompatibleDimensions,
    IncompatibleQuantityKinds,
    IncompatibleSemanticNames,
    InvalidUnitScale,
    InvalidUnitOffset,
    UnreviewedMapping,
    InvalidMapping,
    InvalidTimeContract,
    NegativeDelay,
    MissingBalanceContract,
    UnexpectedBalanceContract,
    InvalidBalanceContract,
}

impl std::fmt::Display for CouplingError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for CouplingError {}

impl CouplingGraph {
    /// Validate model references, port semantics and directed coupling edges.
    ///
    /// The validator deliberately admits only reviewed mappings. It does not
    /// mutate any state on failure and does not auto-create an edge.
    pub fn validate(&self) -> Result<(), CouplingError> {
        let mut model_ids = HashSet::new();
        for model in &self.models {
            validate_identifier(model.model_id)?;
            if model.revision.is_empty() {
                return Err(CouplingError::EmptyIdentifier);
            }
            if !model_ids.insert(model.model_id) {
                return Err(CouplingError::DuplicateModel);
            }
        }

        let mut subsystem_ids = HashSet::new();
        for subsystem in &self.subsystems {
            validate_identifier(subsystem.subsystem_id)?;
            if !subsystem_ids.insert(subsystem.subsystem_id) {
                return Err(CouplingError::DuplicateSubsystem);
            }
            if !model_ids.contains(subsystem.model_id) {
                return Err(CouplingError::UnknownModel);
            }
        }

        let mut port_ids = HashSet::new();
        for port in &self.ports {
            validate_identifier(port.port_id)?;
            if !port_ids.insert(port.port_id) {
                return Err(CouplingError::DuplicatePort);
            }
            if !subsystem_ids.contains(port.subsystem_id) {
                return Err(CouplingError::UnknownSubsystem);
            }
            if !model_ids.contains(port.model_id) {
                return Err(CouplingError::UnknownModel);
            }
            let subsystem = self
                .subsystems
                .iter()
                .find(|candidate| candidate.subsystem_id == port.subsystem_id)
                .ok_or(CouplingError::UnknownSubsystem)?;
            if subsystem.model_id != port.model_id {
                return Err(CouplingError::PortModelMismatch);
            }
            if port.variable_name.is_empty() || port.semantic_name.is_empty() {
                return Err(CouplingError::EmptyIdentifier);
            }
            validate_unit(port.unit)?;
            if !port.min_step_seconds.is_finite()
                || !port.default_step_seconds.is_finite()
                || !port.max_step_seconds.is_finite()
                || port.min_step_seconds <= 0.0
                || port.min_step_seconds > port.default_step_seconds
                || port.default_step_seconds > port.max_step_seconds
            {
                return Err(CouplingError::InvalidTimeContract);
            }
        }

        let mut edge_ids = HashSet::new();
        for edge in &self.edges {
            validate_identifier(edge.edge_id)?;
            if !edge_ids.insert(edge.edge_id) {
                return Err(CouplingError::DuplicateEdge);
            }
            if !edge.delay_seconds.is_finite() || edge.delay_seconds < 0.0 {
                return Err(CouplingError::NegativeDelay);
            }
            let source = self
                .ports
                .iter()
                .find(|port| port.port_id == edge.from_port)
                .ok_or(CouplingError::UnknownPort)?;
            let destination = self
                .ports
                .iter()
                .find(|port| port.port_id == edge.to_port)
                .ok_or(CouplingError::UnknownPort)?;

            if !matches!(
                source.direction,
                PortDirection::Output | PortDirection::Bidirectional
            ) || !matches!(
                destination.direction,
                PortDirection::Input | PortDirection::Bidirectional
            ) {
                return Err(CouplingError::PortDirectionMismatch);
            }

            validate_mapping(source.unit, destination.unit, edge.mapping)?;
            validate_balance(source.class, destination.class, edge.balance.as_ref())?;
        }

        Ok(())
    }
}

fn validate_identifier(value: &str) -> Result<(), CouplingError> {
    if value.trim().is_empty() {
        Err(CouplingError::EmptyIdentifier)
    } else {
        Ok(())
    }
}

fn validate_unit(unit: UnitSpec) -> Result<(), CouplingError> {
    if !unit.scale_to_si.is_finite() || unit.scale_to_si <= 0.0 {
        return Err(CouplingError::InvalidUnitScale);
    }
    if !unit.offset_to_si.is_finite() {
        return Err(CouplingError::InvalidUnitOffset);
    }
    if unit.kind != QuantityKind::AbsoluteTemperature && unit.offset_to_si != 0.0 {
        return Err(CouplingError::InvalidUnitOffset);
    }
    if unit.kind == QuantityKind::Symbolic && unit.semantic_name.is_empty() {
        return Err(CouplingError::IncompatibleSemanticNames);
    }
    Ok(())
}

fn validate_mapping(
    source: UnitSpec,
    destination: UnitSpec,
    mapping: PortMapping,
) -> Result<(), CouplingError> {
    if !mapping.reviewed {
        return Err(CouplingError::UnreviewedMapping);
    }
    if !mapping.scale.is_finite() || mapping.scale == 0.0 || !mapping.offset.is_finite() {
        return Err(CouplingError::InvalidMapping);
    }
    if source.dimension != destination.dimension {
        return Err(CouplingError::IncompatibleDimensions);
    }
    if source.kind != destination.kind {
        return Err(CouplingError::IncompatibleQuantityKinds);
    }
    if source.semantic_name != destination.semantic_name {
        return Err(CouplingError::IncompatibleSemanticNames);
    }
    if source.kind == QuantityKind::TemperatureDifference && mapping.offset != 0.0 {
        return Err(CouplingError::InvalidMapping);
    }
    if source.kind == QuantityKind::Symbolic && mapping.offset != 0.0 {
        return Err(CouplingError::InvalidMapping);
    }

    // Validate the submitted conversion against the two declared affine SI
    // mappings. This checks a conversion proposal; it does not create one.
    let expected_scale = source.scale_to_si / destination.scale_to_si;
    let expected_offset =
        (source.offset_to_si - destination.offset_to_si) / destination.scale_to_si;
    if !approximately_equal(mapping.scale, expected_scale)
        || !approximately_equal(mapping.offset, expected_offset)
    {
        return Err(CouplingError::InvalidMapping);
    }
    Ok(())
}


fn approximately_equal(left: f64, right: f64) -> bool {
    let scale = 1.0_f64.max(left.abs()).max(right.abs());
    (left - right).abs() <= 1.0e-12 * scale
}

fn validate_balance(
    source_class: PortClass,
    destination_class: PortClass,
    balance: Option<&BalanceSpec>,
) -> Result<(), CouplingError> {
    let has_conserved_flow =
        source_class == PortClass::ConservedFlux || destination_class == PortClass::ConservedFlux;
    match (has_conserved_flow, balance) {
        (true, None) => Err(CouplingError::MissingBalanceContract),
        (false, Some(_)) => Err(CouplingError::UnexpectedBalanceContract),
        (true, Some(balance)) => {
            if balance.group_id.is_empty()
                || balance.sign_convention.is_empty()
                || balance.residual_unit.is_empty()
                || balance.closure_definition.is_empty()
            {
                return Err(CouplingError::InvalidBalanceContract);
            }
            Ok(())
        }
        (false, None) => Ok(()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model_graph(ports: Vec<PortSpec>, edges: Vec<CouplingEdge>) -> CouplingGraph {
        CouplingGraph {
            models: vec![
                ModelIdentity {
                    model_id: "JOS3",
                    revision: "3c74ee2",
                },
                ModelIdentity {
                    model_id: "SYMTROPY-ENV",
                    revision: "synthetic-v0.1",
                },
            ],
            subsystems: vec![
                SubsystemIdentity {
                    subsystem_id: "thermo",
                    model_id: "JOS3",
                },
                SubsystemIdentity {
                    subsystem_id: "environment",
                    model_id: "SYMTROPY-ENV",
                },
            ],
            ports,
            edges,
        }
    }

    fn env_temperature_output() -> PortSpec {
        PortSpec {
            port_id: "env-ta",
            subsystem_id: "environment",
            model_id: "SYMTROPY-ENV",
            variable_name: "air_temp",
            semantic_name: "air_temperature",
            direction: PortDirection::Output,
            class: PortClass::BoundaryCondition,
            unit: UnitSpec::celsius(),
            min_step_seconds: 1.0,
            default_step_seconds: 60.0,
            max_step_seconds: 3_600.0,
        }
    }

    fn jos3_temperature_input() -> PortSpec {
        PortSpec {
            port_id: "jos3-ta",
            subsystem_id: "thermo",
            model_id: "JOS3",
            variable_name: "Ta",
            semantic_name: "air_temperature",
            direction: PortDirection::Input,
            class: PortClass::BoundaryCondition,
            unit: UnitSpec::celsius(),
            min_step_seconds: 1.0,
            default_step_seconds: 60.0,
            max_step_seconds: 3_600.0,
        }
    }

    fn identity_mapping() -> PortMapping {
        PortMapping {
            scale: 1.0,
            offset: 0.0,
            reviewed: true,
        }
    }

    #[test]
    fn empty_edge_set_is_valid_and_does_not_auto_connect_ports() {
        let graph = model_graph(
            vec![env_temperature_output(), jos3_temperature_input()],
            Vec::new(),
        );
        assert_eq!(graph.edges.len(), 0);
        assert_eq!(graph.validate(), Ok(()));
    }

    #[test]
    fn matching_reviewed_temperature_ports_can_connect() {
        let graph = model_graph(
            vec![env_temperature_output(), jos3_temperature_input()],
            vec![CouplingEdge {
                edge_id: "ambient-to-jos3",
                from_port: "env-ta",
                to_port: "jos3-ta",
                mapping: identity_mapping(),
                delay_seconds: 0.0,
                balance: None,
            }],
        );
        assert_eq!(graph.validate(), Ok(()));
    }

    #[test]
    fn celsius_to_kelvin_requires_and_accepts_the_declared_affine_conversion() {
        let mut kelvin_input = jos3_temperature_input();
        kelvin_input.unit = UnitSpec::kelvin();
        let graph = model_graph(
            vec![env_temperature_output(), kelvin_input],
            vec![CouplingEdge {
                edge_id: "celsius-to-kelvin",
                from_port: "env-ta",
                to_port: "jos3-ta",
                mapping: PortMapping {
                    scale: 1.0,
                    offset: 273.15,
                    reviewed: true,
                },
                delay_seconds: 0.0,
                balance: None,
            }],
        );
        assert_eq!(graph.validate(), Ok(()));
    }

    #[test]
    fn wrong_temperature_affine_conversion_is_rejected() {
        let mut kelvin_input = jos3_temperature_input();
        kelvin_input.unit = UnitSpec::kelvin();
        let graph = model_graph(
            vec![env_temperature_output(), kelvin_input],
            vec![CouplingEdge {
                edge_id: "wrong-celsius-to-kelvin",
                from_port: "env-ta",
                to_port: "jos3-ta",
                mapping: identity_mapping(),
                delay_seconds: 0.0,
                balance: None,
            }],
        );
        assert_eq!(graph.validate(), Err(CouplingError::InvalidMapping));
    }

    #[test]
    fn absolute_temperature_cannot_connect_to_temperature_difference() {
        let mut delta = jos3_temperature_input();
        delta.unit = UnitSpec::delta_celsius();
        let graph = model_graph(
            vec![env_temperature_output(), delta],
            vec![CouplingEdge {
                edge_id: "bad-temperature-kind",
                from_port: "env-ta",
                to_port: "jos3-ta",
                mapping: identity_mapping(),
                delay_seconds: 0.0,
                balance: None,
            }],
        );
        assert_eq!(
            graph.validate(),
            Err(CouplingError::IncompatibleQuantityKinds)
        );
    }

    #[test]
    fn incompatible_physical_dimensions_reject_before_admission() {
        let mut velocity = jos3_temperature_input();
        velocity.unit = UnitSpec::new(
            "m/s",
            "air_velocity",
            Dimension::VELOCITY,
            QuantityKind::Linear,
            1.0,
            0.0,
        );
        let graph = model_graph(
            vec![env_temperature_output(), velocity],
            vec![CouplingEdge {
                edge_id: "bad-dimensions",
                from_port: "env-ta",
                to_port: "jos3-ta",
                mapping: identity_mapping(),
                delay_seconds: 0.0,
                balance: None,
            }],
        );
        assert_eq!(graph.validate(), Err(CouplingError::IncompatibleDimensions));
    }

    #[test]
    fn unreviewed_conversion_is_not_admitted() {
        let graph = model_graph(
            vec![env_temperature_output(), jos3_temperature_input()],
            vec![CouplingEdge {
                edge_id: "unreviewed",
                from_port: "env-ta",
                to_port: "jos3-ta",
                mapping: PortMapping {
                    reviewed: false,
                    ..identity_mapping()
                },
                delay_seconds: 0.0,
                balance: None,
            }],
        );
        assert_eq!(graph.validate(), Err(CouplingError::UnreviewedMapping));
    }

    #[test]
    fn conserved_flux_requires_a_balance_contract() {
        let mut flow_out = env_temperature_output();
        flow_out.port_id = "flow-out";
        flow_out.semantic_name = "water_volume_flow";
        flow_out.class = PortClass::ConservedFlux;
        flow_out.unit = UnitSpec::new(
            "L/s",
            "water_volume_flow",
            Dimension::VOLUME_FLOW,
            QuantityKind::Linear,
            0.001,
            0.0,
        );

        let mut flow_in = jos3_temperature_input();
        flow_in.port_id = "flow-in";
        flow_in.semantic_name = "water_volume_flow";
        flow_in.class = PortClass::ConservedFlux;
        flow_in.unit = flow_out.unit;

        let graph = model_graph(
            vec![flow_out, flow_in],
            vec![CouplingEdge {
                edge_id: "unbalanced-flow",
                from_port: "flow-out",
                to_port: "flow-in",
                mapping: identity_mapping(),
                delay_seconds: 0.0,
                balance: None,
            }],
        );
        assert_eq!(graph.validate(), Err(CouplingError::MissingBalanceContract));
    }

    #[test]
    fn wrong_port_direction_rejects_connection() {
        let mut source = env_temperature_output();
        source.direction = PortDirection::Input;
        let graph = model_graph(
            vec![source, jos3_temperature_input()],
            vec![CouplingEdge {
                edge_id: "wrong-direction",
                from_port: "env-ta",
                to_port: "jos3-ta",
                mapping: identity_mapping(),
                delay_seconds: 0.0,
                balance: None,
            }],
        );
        assert_eq!(graph.validate(), Err(CouplingError::PortDirectionMismatch));
    }

    #[test]
    fn invalid_time_contract_rejects_before_edge_validation() {
        let mut port = jos3_temperature_input();
        port.default_step_seconds = 4_000.0;
        let graph = model_graph(vec![port], Vec::new());
        assert_eq!(graph.validate(), Err(CouplingError::InvalidTimeContract));
    }
}
