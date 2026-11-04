// SPDX-License-Identifier: AGPL-3.0-or-later
//! Standards-aligned segment efficiency and extended route energy accounting.
//!
//! This crate keeps IEC-style SPC->EVPC segment transfer efficiency separate
//! from inverter-DC->vehicle-DC and grid->battery route metrics. It validates
//! event timing, phase energy reconciliation and deterministic uncertainty.
//! It does not authenticate external receipts or establish physical safety.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

pub const SCHEMA_VERSION: &str = "wpt-segment-energy-ledger-v1";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceClass {
    SyntheticKnownTruth,
    PublishedDataReplay,
    ExperimentalDataReplay,
    FieldMeasurement,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PowerFlowDirection {
    GridToVehicle,
    VehicleToGrid,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum Phase {
    InverterRise,
    PrePower,
    TransferRampUp,
    ActiveTransfer,
    TransferRampDown,
    InverterFall,
}

const PHASE_ORDER: [Phase; 6] = [
    Phase::InverterRise,
    Phase::PrePower,
    Phase::TransferRampUp,
    Phase::ActiveTransfer,
    Phase::TransferRampDown,
    Phase::InverterFall,
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum EnergyBoundary {
    GridAcInput,
    BessDcOutput,
    RoadwayInverterDcInput,
    VehicleDcBusOutput,
    BatteryTerminalInput,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EnergyQuantity {
    pub measurement_id: String,
    pub meter_id: String,
    pub measurement_plane_id: String,
    pub energy_wh: f64,
    pub deterministic_absolute_bound_wh: f64,
    pub calibration_receipt_digest: String,
    pub source_artifact_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PhaseEnergy {
    pub phase: Phase,
    pub start_ns: u64,
    pub end_ns: u64,
    pub spc_input: EnergyQuantity,
    pub evpc_output: EnergyQuantity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BoundaryMeasurement {
    pub boundary: EnergyBoundary,
    pub quantity: EnergyQuantity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SegmentEvent {
    pub segment_id: String,
    pub vehicle_id: String,
    pub geometry_digest: String,
    pub start_ns: u64,
    pub end_ns: u64,
    pub phases: Vec<PhaseEnergy>,
    /// Independent total meters for the complete inverter-rise to inverter-fall segment window.
    pub segment_spc_input: EnergyQuantity,
    pub segment_evpc_output: EnergyQuantity,
    pub boundary_measurements: Vec<BoundaryMeasurement>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct IdleInterval {
    pub interval_id: String,
    pub start_ns: u64,
    pub end_ns: u64,
    pub boundary_measurements: Vec<BoundaryMeasurement>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RouteLedger {
    pub schema_version: String,
    pub route_run_id: String,
    pub evidence_class: EvidenceClass,
    pub power_flow_direction: PowerFlowDirection,
    pub route_geometry_digest: String,
    pub route_start_ns: u64,
    pub route_end_ns: u64,
    pub segments: Vec<SegmentEvent>,
    pub idle_intervals: Vec<IdleInterval>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EfficiencyInterval {
    pub disposition: String,
    pub point_estimate: Option<f64>,
    pub lower_bound: Option<f64>,
    pub upper_bound: Option<f64>,
    pub input_energy_wh: f64,
    pub output_energy_wh: f64,
    pub input_uncertainty_wh: f64,
    pub output_uncertainty_wh: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PhaseReport {
    pub phase: Phase,
    pub start_ns: u64,
    pub end_ns: u64,
    pub spc_measurement_id: String,
    pub evpc_measurement_id: String,
    pub spc_input_wh: f64,
    pub evpc_output_wh: f64,
    pub efficiency: EfficiencyInterval,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SegmentReport {
    pub segment_id: String,
    pub geometry_digest: String,
    pub segment_spc_measurement_id: String,
    pub segment_evpc_measurement_id: String,
    pub phases: Vec<PhaseReport>,
    pub spc_input_wh: f64,
    pub evpc_output_wh: f64,
    pub phase_reconciliation_input_error_wh: f64,
    pub phase_reconciliation_output_error_wh: f64,
    pub standard_segment_efficiency: EfficiencyInterval,
    pub inverter_dc_to_vehicle_dc_efficiency: Option<EfficiencyInterval>,
    pub grid_to_battery_efficiency: Option<EfficiencyInterval>,
    pub battery_terminal_input_wh: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MeasurementProvenance {
    pub measurement_id: String,
    pub meter_id: String,
    pub measurement_plane_id: String,
    pub calibration_receipt_digest: String,
    pub source_artifact_digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RouteReport {
    pub schema_version: String,
    pub route_run_id: String,
    pub evidence_class: EvidenceClass,
    pub power_flow_direction: PowerFlowDirection,
    pub route_geometry_digest: String,
    pub total_segment_spc_input_wh: f64,
    pub total_segment_evpc_output_wh: f64,
    pub total_idle_grid_input_wh: f64,
    pub total_idle_roadway_inverter_dc_input_wh: f64,
    pub total_bess_dc_output_wh: Option<f64>,
    pub total_bess_dc_output_bound_wh: Option<f64>,
    pub standard_route_segment_efficiency: EfficiencyInterval,
    pub inverter_dc_to_vehicle_dc_efficiency: EfficiencyInterval,
    pub grid_to_battery_efficiency: EfficiencyInterval,
    pub battery_terminal_input_wh: f64,
    pub segments: Vec<SegmentReport>,
    pub measurement_provenance: Vec<MeasurementProvenance>,
    pub claim_ceiling: String,
}

impl RouteLedger {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != SCHEMA_VERSION {
            return Err("unsupported schema version".into());
        }
        if self.route_run_id.trim().is_empty() {
            return Err("route_run_id must be non-empty".into());
        }
        if !valid_sha256(&self.route_geometry_digest) {
            return Err("route geometry digest must be sha256".into());
        }
        if self.route_end_ns <= self.route_start_ns
        {
            return Err("route time range must be increasing".into());
        }
        if self.power_flow_direction != PowerFlowDirection::GridToVehicle {
            return Err("v1 supports grid_to_vehicle only; vehicle_to_grid requires a separate sign/metric contract".into());
        }
        if self.segments.is_empty() {
            return Err("route requires at least one energized segment".into());
        }

        let mut segment_ids = HashSet::with_capacity(self.segments.len());
        let mut intervals: Vec<(u64, u64, String)> = Vec::new();
        let mut measurement_ids = HashSet::new();
        for segment in &self.segments {
            if segment.segment_id.trim().is_empty()
                || !segment_ids.insert(&segment.segment_id)
                || segment.vehicle_id.trim().is_empty()
            {
                return Err("segment IDs must be unique/non-empty and vehicle_id non-empty".into());
            }
            if !valid_sha256(&segment.geometry_digest) {
                return Err("segment geometry digest must be sha256".into());
            }
            if segment.start_ns < self.route_start_ns
                || segment.end_ns > self.route_end_ns
                || segment.end_ns <= segment.start_ns
            {
                return Err("segment time interval is invalid or outside route bounds".into());
            }
            if segment.phases.len() != PHASE_ORDER.len() {
                return Err("each segment requires exactly six standard phase records".into());
            }
            for (index, phase) in segment.phases.iter().enumerate() {
                if phase.phase != PHASE_ORDER[index] {
                    return Err("segment phase order must follow inverter-rise through inverter-fall".into());
                }
                if phase.end_ns <= phase.start_ns
                {
                    return Err("phase time range must be positive".into());
                }
                if index == 0 && phase.start_ns != segment.start_ns {
                    return Err("first phase must start at segment start".into());
                }
                if index > 0 && phase.start_ns != segment.phases[index - 1].end_ns {
                    return Err("phase windows must be contiguous; represent gaps as idle intervals".into());
                }
                if index == PHASE_ORDER.len() - 1 && phase.end_ns != segment.end_ns {
                    return Err("final phase must end at segment end".into());
                }
                validate_quantity(&phase.spc_input, "SPC_INPUT")?;
                validate_quantity(&phase.evpc_output, "EVPC_OUTPUT")?;
            }

            validate_quantity(&segment.segment_spc_input, "SPC_INPUT")?;
            validate_quantity(&segment.segment_evpc_output, "EVPC_OUTPUT")?;
            let phase_spc = sum_quantity(segment.phases.iter().map(|phase| &phase.spc_input))?;
            let phase_evpc = sum_quantity(segment.phases.iter().map(|phase| &phase.evpc_output))?;
            if (phase_spc.0 - segment.segment_spc_input.energy_wh).abs()
                > phase_spc.1 + segment.segment_spc_input.deterministic_absolute_bound_wh
            {
                return Err("SPC phase energy does not reconcile with independent segment total within uncertainty".into());
            }
            if (phase_evpc.0 - segment.segment_evpc_output.energy_wh).abs()
                > phase_evpc.1 + segment.segment_evpc_output.deterministic_absolute_bound_wh
            {
                return Err("EVPC phase energy does not reconcile with independent segment total within uncertainty".into());
            }
            validate_boundary_measurements(&segment.boundary_measurements)?;
            for quantity in segment.phases.iter().flat_map(|phase| [&phase.spc_input, &phase.evpc_output])
                .chain([&segment.segment_spc_input, &segment.segment_evpc_output])
                .chain(segment.boundary_measurements.iter().map(|measurement| &measurement.quantity))
            {
                if !measurement_ids.insert(&quantity.measurement_id) {
                    return Err("measurement IDs must be globally unique within a route".into());
                }
            }
            intervals.push((segment.start_ns, segment.end_ns, segment.segment_id.clone()));
        }

        let mut idle_ids = HashSet::with_capacity(self.idle_intervals.len());
        for idle in &self.idle_intervals {
            if idle.interval_id.trim().is_empty() || !idle_ids.insert(&idle.interval_id) {
                return Err("idle interval IDs must be unique and non-empty".into());
            }
            if idle.start_ns < self.route_start_ns
                || idle.end_ns > self.route_end_ns
                || idle.end_ns <= idle.start_ns
            {
                return Err("idle interval time range is invalid or outside route bounds".into());
            }
            validate_boundary_measurements(&idle.boundary_measurements)?;
            for measurement in &idle.boundary_measurements {
                if !measurement_ids.insert(&measurement.quantity.measurement_id) {
                    return Err("measurement IDs must be globally unique within a route".into());
                }
            }
            if !idle.boundary_measurements.iter().any(|m| m.boundary == EnergyBoundary::GridAcInput) {
                return Err("each idle interval requires a grid AC input measurement to preserve standby energy".into());
            }
            intervals.push((idle.start_ns, idle.end_ns, idle.interval_id.clone()));
        }

        intervals.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));
        if intervals.first().map(|interval| interval.0) != Some(self.route_start_ns)
            || intervals.last().map(|interval| interval.1) != Some(self.route_end_ns)
        {
            return Err("segments and idle intervals must cover the complete route time range".into());
        }
        for pair in intervals.windows(2) {
            if pair[1].0 < pair[0].1 {
                return Err(format!("route intervals overlap: {} and {}", pair[0].2, pair[1].2));
            }
            if pair[1].0 > pair[0].1 {
                return Err(format!("unaccounted route time between {} and {}", pair[0].2, pair[1].2));
            }
        }
        Ok(())
    }

    pub fn evaluate(&self) -> Result<RouteReport, String> {
        self.validate()?;
        let mut segment_reports = Vec::with_capacity(self.segments.len());
        let mut total_spc = 0.0;
        let mut total_evpc = 0.0;
        let mut total_spc_bound = 0.0;
        let mut total_evpc_bound = 0.0;
        let mut total_idle_grid = 0.0;
        let mut total_bess = 0.0;
        let mut total_bess_bound = 0.0;
        let mut total_bess_present = false;

        let mut total_inverter = 0.0;
        let mut total_inverter_bound = 0.0;
        let mut total_idle_inverter = 0.0;
        let mut total_vehicle_dc = 0.0;
        let mut total_vehicle_dc_bound = 0.0;
        let mut total_grid = 0.0;
        let mut total_grid_bound = 0.0;
        let mut total_battery = 0.0;
        let mut total_battery_bound = 0.0;

        let mut boundary_coverage = BTreeMap::<EnergyBoundary, usize>::new();
        for segment in &self.segments {
            let spc = sum_quantity(segment.phases.iter().map(|phase| &phase.spc_input))?;
            let evpc = sum_quantity(segment.phases.iter().map(|phase| &phase.evpc_output))?;
            let standard = efficiency_interval(spc.0, evpc.0, spc.1, evpc.1);
            let phase_metrics = segment
                .phases
                .iter()
                .map(|phase| PhaseReport {
                    phase: phase.phase.clone(),
                    start_ns: phase.start_ns,
                    end_ns: phase.end_ns,
                    spc_measurement_id: phase.spc_input.measurement_id.clone(),
                    evpc_measurement_id: phase.evpc_output.measurement_id.clone(),
                    spc_input_wh: phase.spc_input.energy_wh,
                    evpc_output_wh: phase.evpc_output.energy_wh,
                    efficiency: efficiency_interval(
                        phase.spc_input.energy_wh,
                        phase.evpc_output.energy_wh,
                        phase.spc_input.deterministic_absolute_bound_wh,
                        phase.evpc_output.deterministic_absolute_bound_wh,
                    ),
                })
                .collect();

            total_spc = checked_add(total_spc, spc.0)?;
            total_evpc = checked_add(total_evpc, evpc.0)?;
            total_spc_bound = checked_add(total_spc_bound, spc.1)?;
            total_evpc_bound = checked_add(total_evpc_bound, evpc.1)?;

            let map = boundary_map(&segment.boundary_measurements);
            for (boundary, quantity) in &map {
                *boundary_coverage.entry(boundary.clone()).or_insert(0) += 1;
                match boundary {
                    EnergyBoundary::RoadwayInverterDcInput => {
                        total_inverter = checked_add(total_inverter, quantity.energy_wh)?;
                        total_inverter_bound = checked_add(total_inverter_bound, quantity.deterministic_absolute_bound_wh)?;
                    }
                    EnergyBoundary::BessDcOutput => {
                        total_bess = checked_add(total_bess, quantity.energy_wh)?;
                        total_bess_bound = checked_add(
                            total_bess_bound,
                            quantity.deterministic_absolute_bound_wh,
                        )?;
                        total_bess_present = true;
                    }
                    EnergyBoundary::VehicleDcBusOutput => {
                        total_vehicle_dc = checked_add(total_vehicle_dc, quantity.energy_wh)?;
                        total_vehicle_dc_bound = checked_add(total_vehicle_dc_bound, quantity.deterministic_absolute_bound_wh)?;
                    }
                    EnergyBoundary::GridAcInput => {
                        total_grid = checked_add(total_grid, quantity.energy_wh)?;
                        total_grid_bound = checked_add(total_grid_bound, quantity.deterministic_absolute_bound_wh)?;
                    }
                    EnergyBoundary::BatteryTerminalInput => {
                        total_battery = checked_add(total_battery, quantity.energy_wh)?;
                        total_battery_bound = checked_add(total_battery_bound, quantity.deterministic_absolute_bound_wh)?;
                    }
                    EnergyBoundary::BessDcOutput => {}
                }
            }

            let inverter_to_vehicle = paired_segment_metric(
                &map,
                &EnergyBoundary::RoadwayInverterDcInput,
                &EnergyBoundary::VehicleDcBusOutput,
            );
            let grid_to_battery = paired_segment_metric(
                &map,
                &EnergyBoundary::GridAcInput,
                &EnergyBoundary::BatteryTerminalInput,
            );
            let battery_terminal_input_wh = map
                .get(&EnergyBoundary::BatteryTerminalInput)
                .map(|measurement| measurement.energy_wh);

            segment_reports.push(SegmentReport {
                segment_id: segment.segment_id.clone(),
                geometry_digest: segment.geometry_digest.clone(),
                segment_spc_measurement_id: segment.segment_spc_input.measurement_id.clone(),
                segment_evpc_measurement_id: segment.segment_evpc_output.measurement_id.clone(),
                phases: phase_metrics,
                spc_input_wh: spc.0,
                evpc_output_wh: evpc.0,
                phase_reconciliation_input_error_wh: (spc.0 - segment.segment_spc_input.energy_wh).abs(),
                phase_reconciliation_output_error_wh: (evpc.0 - segment.segment_evpc_output.energy_wh).abs(),
                standard_segment_efficiency: standard,
                inverter_dc_to_vehicle_dc_efficiency: inverter_to_vehicle,
                grid_to_battery_efficiency: grid_to_battery,
                battery_terminal_input_wh,
            });
        }

        for idle in &self.idle_intervals {
            let map = boundary_map(&idle.boundary_measurements);
            if let Some(grid) = map.get(&EnergyBoundary::GridAcInput) {
                total_grid = checked_add(total_grid, grid.energy_wh)?;
                total_grid_bound = checked_add(total_grid_bound, grid.deterministic_absolute_bound_wh)?;
                total_idle_grid = checked_add(total_idle_grid, grid.energy_wh)?;
            }
            if let Some(inverter) = map.get(&EnergyBoundary::RoadwayInverterDcInput) {
                total_idle_inverter = checked_add(total_idle_inverter, inverter.energy_wh)?;
            }
            if let Some(bess) = map.get(&EnergyBoundary::BessDcOutput) {
                total_bess = checked_add(total_bess, bess.energy_wh)?;
                total_bess_bound =
                    checked_add(total_bess_bound, bess.deterministic_absolute_bound_wh)?;
                total_bess_present = true;
            }
        }

        require_full_segment_boundary(&boundary_coverage, EnergyBoundary::RoadwayInverterDcInput, self.segments.len())?;
        require_full_segment_boundary(&boundary_coverage, EnergyBoundary::VehicleDcBusOutput, self.segments.len())?;
        require_full_segment_boundary(&boundary_coverage, EnergyBoundary::GridAcInput, self.segments.len())?;
        require_full_segment_boundary(&boundary_coverage, EnergyBoundary::BatteryTerminalInput, self.segments.len())?;

        let standard_route = efficiency_interval(total_spc, total_evpc, total_spc_bound, total_evpc_bound);
        let inverter_route = efficiency_interval(
            total_inverter,
            total_vehicle_dc,
            total_inverter_bound,
            total_vehicle_dc_bound,
        );
        let grid_route = efficiency_interval(
            total_grid,
            total_battery,
            total_grid_bound,
            total_battery_bound,
        );

        let mut measurement_provenance = Vec::new();
        for segment in &self.segments {
            let quantities = segment
                .phases
                .iter()
                .flat_map(|phase| [&phase.spc_input, &phase.evpc_output])
                .chain([&segment.segment_spc_input, &segment.segment_evpc_output])
                .chain(
                    segment
                        .boundary_measurements
                        .iter()
                        .map(|measurement| &measurement.quantity),
                );
            measurement_provenance.extend(quantities.map(provenance_record));
        }
        for idle in &self.idle_intervals {
            measurement_provenance.extend(
                idle.boundary_measurements
                    .iter()
                    .map(|measurement| provenance_record(&measurement.quantity)),
            );
        }
        measurement_provenance.sort_by(|left, right| {
            left.measurement_id.cmp(&right.measurement_id)
        });

        Ok(RouteReport {
            schema_version: SCHEMA_VERSION.into(),
            route_run_id: self.route_run_id.clone(),
            evidence_class: self.evidence_class.clone(),
            power_flow_direction: self.power_flow_direction.clone(),
            route_geometry_digest: self.route_geometry_digest.clone(),
            total_segment_spc_input_wh: total_spc,
            total_segment_evpc_output_wh: total_evpc,
            total_idle_grid_input_wh: total_idle_grid,
            total_idle_roadway_inverter_dc_input_wh: total_idle_inverter,
            total_bess_dc_output_wh: total_bess_present.then_some(total_bess),
            total_bess_dc_output_bound_wh: total_bess_present.then_some(total_bess_bound),
            standard_route_segment_efficiency: standard_route,
            inverter_dc_to_vehicle_dc_efficiency: inverter_route,
            grid_to_battery_efficiency: grid_route,
            battery_terminal_input_wh: total_battery,
            segments: segment_reports,
            measurement_provenance,
            claim_ceiling: "Energy accounting and efficiency arithmetic only. This report does not prove meter calibration authenticity, electromagnetic safety, thermal safety, foreign-object detection, interoperability, hardware integrity, or standards compliance. IEC-style segment result and extended route ratios are distinct boundaries.".into(),
        })
    }
}

fn provenance_record(quantity: &EnergyQuantity) -> MeasurementProvenance {
    MeasurementProvenance {
        measurement_id: quantity.measurement_id.clone(),
        meter_id: quantity.meter_id.clone(),
        measurement_plane_id: quantity.measurement_plane_id.clone(),
        calibration_receipt_digest: quantity.calibration_receipt_digest.clone(),
        source_artifact_digest: quantity.source_artifact_digest.clone(),
    }
}

fn validate_quantity(quantity: &EnergyQuantity, expected_plane: &str) -> Result<(), String> {
    if quantity.measurement_id.trim().is_empty()
        || quantity.meter_id.trim().is_empty()
        || quantity.measurement_plane_id != expected_plane
    {
        return Err(format!("measurement identity/plane invalid; expected {expected_plane}"));
    }
    if !quantity.energy_wh.is_finite()
        || quantity.energy_wh < 0.0
        || !quantity.deterministic_absolute_bound_wh.is_finite()
        || quantity.deterministic_absolute_bound_wh < 0.0
    {
        return Err("energy and deterministic absolute bound must be finite and non-negative".into());
    }
    if !valid_sha256(&quantity.calibration_receipt_digest)
        || !valid_sha256(&quantity.source_artifact_digest)
    {
        return Err("calibration and source artifact digests must be sha256".into());
    }
    Ok(())
}

fn validate_boundary_measurements(measurements: &[BoundaryMeasurement]) -> Result<(), String> {
    let mut seen = HashSet::with_capacity(measurements.len());
    for measurement in measurements {
        let expected = match measurement.boundary {
            EnergyBoundary::GridAcInput => "GRID_AC_INPUT",
            EnergyBoundary::BessDcOutput => "BESS_DC_OUTPUT",
            EnergyBoundary::RoadwayInverterDcInput => "ROADWAY_INVERTER_DC_INPUT",
            EnergyBoundary::VehicleDcBusOutput => "VEHICLE_DC_BUS_OUTPUT",
            EnergyBoundary::BatteryTerminalInput => "BATTERY_TERMINAL_INPUT",
        };
        if !seen.insert(&measurement.boundary) {
            return Err("duplicate energy boundary measurement".into());
        }
        validate_quantity(&measurement.quantity, expected)?;
    }
    Ok(())
}

fn boundary_map(measurements: &[BoundaryMeasurement]) -> BTreeMap<EnergyBoundary, &EnergyQuantity> {
    measurements
        .iter()
        .map(|measurement| (measurement.boundary.clone(), &measurement.quantity))
        .collect()
}

fn paired_segment_metric(
    values: &BTreeMap<EnergyBoundary, &EnergyQuantity>,
    input_boundary: &EnergyBoundary,
    output_boundary: &EnergyBoundary,
) -> Option<EfficiencyInterval> {
    let input = values.get(input_boundary)?;
    let output = values.get(output_boundary)?;
    Some(efficiency_interval(
        input.energy_wh,
        output.energy_wh,
        input.deterministic_absolute_bound_wh,
        output.deterministic_absolute_bound_wh,
    ))
}

fn require_full_segment_boundary(
    coverage: &BTreeMap<EnergyBoundary, usize>,
    boundary: EnergyBoundary,
    segment_count: usize,
) -> Result<(), String> {
    if coverage.get(&boundary).copied().unwrap_or(0) != segment_count {
        return Err(format!("every segment requires {boundary:?} measurement for route aggregation"));
    }
    Ok(())
}

fn sum_quantity<'a>(
    quantities: impl Iterator<Item = &'a EnergyQuantity>,
) -> Result<(f64, f64), String> {
    let mut total = 0.0;
    let mut bound = 0.0;
    for quantity in quantities {
        total = checked_add(total, quantity.energy_wh)?;
        bound = checked_add(bound, quantity.deterministic_absolute_bound_wh)?;
    }
    Ok((total, bound))
}

fn checked_add(left: f64, right: f64) -> Result<f64, String> {
    let value = left + right;
    if value.is_finite() {
        Ok(value)
    } else {
        Err("energy sum overflow".into())
    }
}

fn efficiency_interval(input_wh: f64, output_wh: f64, input_bound_wh: f64, output_bound_wh: f64) -> EfficiencyInterval {
    let input_lower = input_wh - input_bound_wh;
    let output_lower = output_wh - output_bound_wh;
    let denominator_lower = input_lower;
    if !input_wh.is_finite()
        || !output_wh.is_finite()
        || !input_bound_wh.is_finite()
        || !output_bound_wh.is_finite()
        || input_wh <= 0.0
        || input_bound_wh < 0.0
        || output_bound_wh < 0.0
        || denominator_lower <= 0.0
        || output_lower < 0.0
    {
        return EfficiencyInterval {
            disposition: "UNRESOLVED".into(),
            point_estimate: None,
            lower_bound: None,
            upper_bound: None,
            input_energy_wh: input_wh,
            output_energy_wh: output_wh,
            input_uncertainty_wh: input_bound_wh,
            output_uncertainty_wh: output_bound_wh,
        };
    }

    let point = output_wh / input_wh;
    let lower = output_lower / (input_wh + input_bound_wh);
    let upper = (output_wh + output_bound_wh) / denominator_lower;
    if !point.is_finite() || !lower.is_finite() || !upper.is_finite() {
        return EfficiencyInterval {
            disposition: "UNRESOLVED".into(),
            point_estimate: None,
            lower_bound: None,
            upper_bound: None,
            input_energy_wh: input_wh,
            output_energy_wh: output_wh,
            input_uncertainty_wh: input_bound_wh,
            output_uncertainty_wh: output_bound_wh,
        };
    }

    EfficiencyInterval {
        disposition: "COMPUTED_INTERVAL".into(),
        point_estimate: Some(point),
        lower_bound: Some(lower),
        upper_bound: Some(upper),
        input_energy_wh: input_wh,
        output_energy_wh: output_wh,
        input_uncertainty_wh: input_bound_wh,
        output_uncertainty_wh: output_bound_wh,
    }
}

fn valid_sha256(value: &str) -> bool {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return false;
    };
    hex.len() == 64
        && hex
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(value: char) -> String {
        format!("sha256:{}", value.to_string().repeat(64))
    }

    fn quantity(id: &str, plane: &str, energy: f64) -> EnergyQuantity {
        EnergyQuantity {
            measurement_id: id.into(),
            meter_id: format!("METER-{id}"),
            measurement_plane_id: plane.into(),
            energy_wh: energy,
            deterministic_absolute_bound_wh: 0.01,
            calibration_receipt_digest: digest('a'),
            source_artifact_digest: digest('b'),
        }
    }

    fn segment(id: &str, start_ns: u64) -> SegmentEvent {
        let phases = PHASE_ORDER
            .iter()
            .enumerate()
            .map(|(index, phase)| PhaseEnergy {
                phase: phase.clone(),
                start_ns: start_ns + index as u64 * 1_000_000_000,
                end_ns: start_ns + (index as u64 + 1) * 1_000_000_000,
                spc_input: quantity(&format!("{id}-SPC-{index}"), "SPC_INPUT", 10.0),
                evpc_output: quantity(&format!("{id}-EVPC-{index}"), "EVPC_OUTPUT", 9.0),
            })
            .collect();
        SegmentEvent {
            segment_id: id.into(),
            vehicle_id: "TRUCK-1".into(),
            geometry_digest: digest('c'),
            start_ns,
            end_ns: start_ns + 6_000_000_000,
            phases,
            segment_spc_input: quantity(&format!("{id}-SPC-TOTAL"), "SPC_INPUT", 60.0),
            segment_evpc_output: quantity(&format!("{id}-EVPC-TOTAL"), "EVPC_OUTPUT", 54.0),
            boundary_measurements: vec![
                BoundaryMeasurement {
                    boundary: EnergyBoundary::GridAcInput,
                    quantity: quantity(&format!("{id}-GRID"), "GRID_AC_INPUT", 90.0),
                },
                BoundaryMeasurement {
                    boundary: EnergyBoundary::RoadwayInverterDcInput,
                    quantity: quantity(&format!("{id}-INV"), "ROADWAY_INVERTER_DC_INPUT", 70.0),
                },
                BoundaryMeasurement {
                    boundary: EnergyBoundary::VehicleDcBusOutput,
                    quantity: quantity(&format!("{id}-VDC"), "VEHICLE_DC_BUS_OUTPUT", 65.0),
                },
                BoundaryMeasurement {
                    boundary: EnergyBoundary::BatteryTerminalInput,
                    quantity: quantity(&format!("{id}-BAT"), "BATTERY_TERMINAL_INPUT", 48.0),
                },
                BoundaryMeasurement {
                    boundary: EnergyBoundary::BessDcOutput,
                    quantity: quantity(&format!("{id}-BESS"), "BESS_DC_OUTPUT", 55.0),
                },
            ],
        }
    }

    fn fixture() -> RouteLedger {
        RouteLedger {
            schema_version: SCHEMA_VERSION.into(),
            route_run_id: "SYNTHETIC-HIGHWAY-PASS-001".into(),
            evidence_class: EvidenceClass::SyntheticKnownTruth,
            power_flow_direction: PowerFlowDirection::GridToVehicle,
            route_geometry_digest: digest('d'),
            route_start_ns: 0,
            route_end_ns: 16_000_000_000,
            segments: vec![segment("SEG-1", 0), segment("SEG-2", 10_000_000_000)],
            idle_intervals: vec![IdleInterval {
                interval_id: "WAIT-1".into(),
                start_ns: 6_000_000_000,
                end_ns: 10_000_000_000,
                boundary_measurements: vec![BoundaryMeasurement {
                    boundary: EnergyBoundary::GridAcInput,
                    quantity: quantity("WAIT-GRID", "GRID_AC_INPUT", 5.0),
                },
                BoundaryMeasurement {
                    boundary: EnergyBoundary::RoadwayInverterDcInput,
                    quantity: quantity("WAIT-INVERTER", "ROADWAY_INVERTER_DC_INPUT", 20.0),
                },
                BoundaryMeasurement {
                    boundary: EnergyBoundary::BessDcOutput,
                    quantity: quantity("WAIT-BESS", "BESS_DC_OUTPUT", 2.0),
                }],
            }],
        }
    }


    #[test]
    fn synthetic_route_fixture_matches_schema_and_replays_all_distinct_energy_boundaries() {
        const FIXTURE: &str = include_str!(
            "../../../benchmarks/wpt-segment-energy-ledger/v1/synthetic-highway-pass.json"
        );
        const SCHEMA: &str = include_str!(
            "../../../benchmarks/wpt-segment-energy-ledger/v1/schema.json"
        );
        let ledger: RouteLedger =
            serde_json::from_str(FIXTURE).expect("synthetic segment-energy ledger fixture");
        let schema: serde_json::Value =
            serde_json::from_str(SCHEMA).expect("segment-energy JSON schema");
        assert_eq!(
            schema["properties"]["schema_version"]["const"],
            SCHEMA_VERSION
        );
        assert_eq!(schema["additionalProperties"], false);

        let report = ledger.evaluate().unwrap();
        assert!((report.standard_route_segment_efficiency.point_estimate.unwrap() - 0.9).abs() < 1e-12);
        assert!((report.inverter_dc_to_vehicle_dc_efficiency.point_estimate.unwrap() - (130.0 / 140.0)).abs() < 1e-12);
        assert!((report.grid_to_battery_efficiency.point_estimate.unwrap() - (96.0 / 185.0)).abs() < 1e-12);
        assert_eq!(report.total_idle_grid_input_wh, 5.0);
        assert_eq!(report.total_idle_roadway_inverter_dc_input_wh, 20.0);
        assert_eq!(report.total_bess_dc_output_wh, Some(112.0));
        assert_eq!(report.total_bess_dc_output_bound_wh, Some(0.03));
        assert_eq!(report.segments[0].phases.len(), 6);
        assert_eq!(report.segments[0].phases[0].phase, Phase::InverterRise);
        assert_eq!(report.segments[0].phases[5].phase, Phase::InverterFall);
    }

    #[test]
    fn reports_distinct_standard_segment_and_end_to_end_boundaries() {
        let report = fixture().evaluate().unwrap();
        assert!((report.standard_route_segment_efficiency.point_estimate.unwrap() - 0.9).abs() < 1e-12);
        assert!((report.inverter_dc_to_vehicle_dc_efficiency.point_estimate.unwrap() - (130.0 / 140.0)).abs() < 1e-12);
        assert!((report.grid_to_battery_efficiency.point_estimate.unwrap() - (96.0 / 185.0)).abs() < 1e-12);
        assert_eq!(report.total_idle_grid_input_wh, 5.0);
        assert_eq!(report.battery_terminal_input_wh, 96.0);
    }

    #[test]
    fn route_efficiency_is_ratio_of_sums_not_mean_of_segment_ratios() {
        let mut ledger = fixture();
        for phase in &mut ledger.segments[0].phases {
            phase.spc_input.energy_wh = 90.0 / 6.0;
            phase.evpc_output.energy_wh = 81.0 / 6.0;
        }
        for phase in &mut ledger.segments[1].phases {
            phase.spc_input.energy_wh = 10.0 / 6.0;
            phase.evpc_output.energy_wh = 5.0 / 6.0;
        }
        ledger.segments[0].segment_spc_input.energy_wh = 90.0;
        ledger.segments[0].segment_evpc_output.energy_wh = 81.0;
        ledger.segments[1].segment_spc_input.energy_wh = 10.0;
        ledger.segments[1].segment_evpc_output.energy_wh = 5.0;
        let report = ledger.evaluate().unwrap();
        assert!((report.standard_route_segment_efficiency.point_estimate.unwrap() - 0.86).abs() < 1e-12);
        let mean_of_ratios = (0.9 + 0.5) / 2.0;
        assert!((mean_of_ratios - 0.86).abs() > 0.0);
    }

    #[test]
    fn uncertainty_denominator_crossing_zero_is_unresolved() {
        let mut ledger = fixture();
        for measurement in &mut ledger.segments[0].boundary_measurements {
            if measurement.boundary == EnergyBoundary::RoadwayInverterDcInput {
                measurement.quantity.deterministic_absolute_bound_wh = 100.0;
            }
        }
        let report = ledger.evaluate().unwrap();
        assert_eq!(
            report.segments[0].inverter_dc_to_vehicle_dc_efficiency.as_ref().unwrap().disposition,
            "UNRESOLVED"
        );
        assert_eq!(report.inverter_dc_to_vehicle_dc_efficiency.disposition, "UNRESOLVED");
    }

    #[test]
    fn rejects_noncontiguous_phase_windows_and_wrong_phase_order() {
        let mut ledger = fixture();
        ledger.segments[0].phases[2].start_ns += 100_000_000;
        assert!(ledger.validate().unwrap_err().contains("contiguous"));
        let mut ledger = fixture();
        ledger.segments[0].phases.swap(0, 1);
        assert!(ledger.validate().unwrap_err().contains("phase order"));
    }

    #[test]
    fn rejects_route_overlap_and_missing_idle_grid_energy() {
        let mut ledger = fixture();
        ledger.segments[1].start_ns = 5_000_000_000;
        assert!(ledger.validate().unwrap_err().contains("overlap"));
        let mut ledger = fixture();
        ledger.idle_intervals[0].boundary_measurements.clear();
        assert!(ledger.validate().unwrap_err().contains("standby energy"));
    }

    #[test]
    fn rejects_invalid_calibration_source_digest_and_measurement_plane() {
        let mut ledger = fixture();
        ledger.segments[0].phases[0].spc_input.calibration_receipt_digest = "bad".into();
        assert!(ledger.validate().unwrap_err().contains("digests"));
        let mut ledger = fixture();
        ledger.segments[0].phases[0].evpc_output.measurement_plane_id = "UNKNOWN".into();
        assert!(ledger.validate().unwrap_err().contains("plane"));
    }

    #[test]
    fn rejects_duplicate_boundary_measurements_and_route_ids() {
        let mut ledger = fixture();
        ledger.segments[0].boundary_measurements.push(ledger.segments[0].boundary_measurements[0].clone());
        assert!(ledger.validate().unwrap_err().contains("duplicate energy boundary"));
        let mut ledger = fixture();
        ledger.segments[1].segment_id = ledger.segments[0].segment_id.clone();
        assert!(ledger.validate().unwrap_err().contains("unique"));
    }


    #[test]
    fn rejects_phase_energy_that_does_not_reconcile_with_independent_segment_meter() {
        let mut ledger = fixture();
        ledger.segments[0].segment_evpc_output.energy_wh = 40.0;
        assert!(
            ledger.validate().unwrap_err().contains("does not reconcile with independent segment total")
        );
    }

    #[test]
    fn rejects_unaccounted_time_between_segment_and_idle_events() {
        let mut ledger = fixture();
        ledger.idle_intervals[0].start_ns = 7_000_000_000;
        assert!(ledger.validate().unwrap_err().contains("unaccounted route time"));
    }

    #[test]
    fn separates_idle_inverter_energy_from_active_transfer_efficiency() {
        let mut ledger = fixture();
        ledger.idle_intervals[0].boundary_measurements.push(BoundaryMeasurement {
            boundary: EnergyBoundary::RoadwayInverterDcInput,
            quantity: quantity("WAIT-INVERTER", "ROADWAY_INVERTER_DC_INPUT", 20.0),
        });
        let baseline = ledger.evaluate().unwrap();
        assert!((baseline.inverter_dc_to_vehicle_dc_efficiency.point_estimate.unwrap() - (130.0 / 140.0)).abs() < 1e-12);
        assert_eq!(baseline.total_idle_roadway_inverter_dc_input_wh, 20.0);
    }
    #[test]
    fn reports_bess_output_separately_and_does_not_add_idle_inverter_to_transfer_denominator() {
        let report = fixture().evaluate().unwrap();
        assert_eq!(report.total_bess_dc_output_wh, Some(112.0));
        assert_eq!(report.total_idle_roadway_inverter_dc_input_wh, 20.0);
        assert!(
            (report.inverter_dc_to_vehicle_dc_efficiency.point_estimate.unwrap()
                - 130.0 / 140.0)
                .abs()
                < 1e-12
        );
    }

    #[test]
    fn rejects_missing_segment_level_boundary_measurements_for_route_metric() {
        let mut ledger = fixture();
        ledger.segments[1]
            .boundary_measurements
            .retain(|measurement| measurement.boundary != EnergyBoundary::BatteryTerminalInput);
        let error = ledger.evaluate().unwrap_err();
        assert!(error.contains("every segment requires BatteryTerminalInput"));
    }

    #[test]
    fn preserves_standby_energy_outside_the_standard_segment_metric() {
        let mut ledger = fixture();
        let baseline = ledger.evaluate().unwrap();
        ledger.idle_intervals[0].boundary_measurements[0].quantity.energy_wh = 20.0;
        let with_wait = ledger.evaluate().unwrap();
        assert_eq!(baseline.standard_route_segment_efficiency, with_wait.standard_route_segment_efficiency);
        assert!(with_wait.grid_to_battery_efficiency.input_energy_wh > baseline.grid_to_battery_efficiency.input_energy_wh);
        assert!(with_wait.total_idle_grid_input_wh > baseline.total_idle_grid_input_wh);
    }
}
