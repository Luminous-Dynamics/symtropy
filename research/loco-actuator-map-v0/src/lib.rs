//! Bounded, evidence-qualified actuator electrical/mechanical loss-map reference.
//!
//! V0 evaluates a caller-supplied rectilinear four-axis calibration grid over
//! signed torque, signed angular velocity, temperature and DC-bus voltage.
//! Each node carries independent sample, input-artifact and calibration
//! identity plus lower/nominal/upper electrical and mechanical power bounds.
//!
//! The map is not an actuator-identification routine. Provenance, calibration,
//! sign conventions, time alignment and physical validity must be established
//! by the upstream measurement process and independently authenticated.

pub const MAX_AXIS_POINTS: usize = 16;
pub const MAX_GRID_POINTS: usize = 4_096;
const Q16: u128 = 65_536;
const TOTAL_WEIGHT_Q64: u128 = 1_u128 << 64;
const TOTAL_WEIGHT_I128: i128 = 1_i128 << 64;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapAxis {
    Torque,
    AngularVelocity,
    Temperature,
    DcBusVoltage,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MapError {
    EmptyIdentity,
    EmptySampleId,
    DuplicateSampleId(String),
    InvalidAxisLength(MapAxis),
    AxisNotStrictlyIncreasing(MapAxis),
    AxisRangeTooWide(MapAxis),
    GridSizeOverflow,
    GridTooLarge,
    GridValueCountMismatch,
    InvalidPowerInterval,
    MechanicalNominalMismatch {
        node_index: usize,
        expected_pw: i64,
        observed_pw: i64,
    },
    MechanicalInterpolationDoesNotEncloseNominal,
    OutsideCalibrationEnvelope {
        axis: MapAxis,
        value: i32,
        minimum: i32,
        maximum: i32,
    },
    ArithmeticOverflow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PowerInterval {
    lower_pw: i64,
    nominal_pw: i64,
    upper_pw: i64,
}

impl PowerInterval {
    pub fn new(lower_pw: i64, nominal_pw: i64, upper_pw: i64) -> Result<Self, MapError> {
        if lower_pw > nominal_pw || nominal_pw > upper_pw {
            return Err(MapError::InvalidPowerInterval);
        }
        Ok(Self {
            lower_pw,
            nominal_pw,
            upper_pw,
        })
    }

    pub fn lower_pw(self) -> i64 {
        self.lower_pw
    }

    pub fn nominal_pw(self) -> i64 {
        self.nominal_pw
    }

    pub fn upper_pw(self) -> i64 {
        self.upper_pw
    }
}

/// One measured/modelled calibration node. Mechanical nominal power must
/// equal tau*omega at the node coordinate; its interval records measurement
/// uncertainty around that nominal value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalibrationNode {
    sample_id: String,
    input_artifact_digest: String,
    calibration_digest: String,
    electrical_input_power_pw: PowerInterval,
    mechanical_joint_power_pw: PowerInterval,
}

impl CalibrationNode {
    pub fn new(
        sample_id: String,
        input_artifact_digest: String,
        calibration_digest: String,
        electrical_input_power_pw: PowerInterval,
        mechanical_joint_power_pw: PowerInterval,
    ) -> Result<Self, MapError> {
        if sample_id.trim().is_empty()
            || input_artifact_digest.trim().is_empty()
            || calibration_digest.trim().is_empty()
        {
            return Err(MapError::EmptyIdentity);
        }
        Ok(Self {
            sample_id,
            input_artifact_digest,
            calibration_digest,
            electrical_input_power_pw,
            mechanical_joint_power_pw,
        })
    }

    pub fn sample_id(&self) -> &str {
        &self.sample_id
    }

    pub fn input_artifact_digest(&self) -> &str {
        &self.input_artifact_digest
    }

    pub fn calibration_digest(&self) -> &str {
        &self.calibration_digest
    }

    pub fn electrical_input_power_pw(&self) -> PowerInterval {
        self.electrical_input_power_pw
    }

    pub fn mechanical_joint_power_pw(&self) -> PowerInterval {
        self.mechanical_joint_power_pw
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActuatorAxes {
    /// Signed micro-newton-metres.
    torque_micro_nm: Vec<i32>,
    /// Signed microradians per second.
    angular_velocity_microrad_s: Vec<i32>,
    /// Milli-degrees Celsius.
    temperature_millic: Vec<i32>,
    /// Millivolts DC bus voltage.
    dc_bus_voltage_mv: Vec<i32>,
}

impl ActuatorAxes {
    pub fn new(
        torque_micro_nm: Vec<i32>,
        angular_velocity_microrad_s: Vec<i32>,
        temperature_millic: Vec<i32>,
        dc_bus_voltage_mv: Vec<i32>,
    ) -> Result<Self, MapError> {
        let axes = [
            (&torque_micro_nm, MapAxis::Torque),
            (&angular_velocity_microrad_s, MapAxis::AngularVelocity),
            (&temperature_millic, MapAxis::Temperature),
            (&dc_bus_voltage_mv, MapAxis::DcBusVoltage),
        ];
        for (values, axis) in axes {
            validate_axis(values, axis)?;
        }
        Ok(Self {
            torque_micro_nm,
            angular_velocity_microrad_s,
            temperature_millic,
            dc_bus_voltage_mv,
        })
    }

    fn axis(&self, axis: MapAxis) -> &[i32] {
        match axis {
            MapAxis::Torque => &self.torque_micro_nm,
            MapAxis::AngularVelocity => &self.angular_velocity_microrad_s,
            MapAxis::Temperature => &self.temperature_millic,
            MapAxis::DcBusVoltage => &self.dc_bus_voltage_mv,
        }
    }

    fn dimensions(&self) -> [usize; 4] {
        [
            self.torque_micro_nm.len(),
            self.angular_velocity_microrad_s.len(),
            self.temperature_millic.len(),
            self.dc_bus_voltage_mv.len(),
        ]
    }
}

fn validate_axis(values: &[i32], axis: MapAxis) -> Result<(), MapError> {
    if values.len() < 2 || values.len() > MAX_AXIS_POINTS {
        return Err(MapError::InvalidAxisLength(axis));
    }
    for pair in values.windows(2) {
        if pair[1] <= pair[0] {
            return Err(MapError::AxisNotStrictlyIncreasing(axis));
        }
        let span = i64::from(pair[1]) - i64::from(pair[0]);
        if span > 1_000_000_000 {
            return Err(MapError::AxisRangeTooWide(axis));
        }
    }
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OperatingPoint {
    pub torque_micro_nm: i32,
    pub angular_velocity_microrad_s: i32,
    pub temperature_millic: i32,
    pub dc_bus_voltage_mv: i32,
}

impl OperatingPoint {
    pub fn coordinates(self) -> [i32; 4] {
        [
            self.torque_micro_nm,
            self.angular_velocity_microrad_s,
            self.temperature_millic,
            self.dc_bus_voltage_mv,
        ]
    }

    /// tau[µN·m] × omega[µrad/s] is exactly pW because 10^-6 × 10^-6 W = 10^-12 W.
    pub fn mechanical_power_pw(self) -> Result<i64, MapError> {
        i64::from(self.torque_micro_nm)
            .checked_mul(i64::from(self.angular_velocity_microrad_s))
            .ok_or(MapError::ArithmeticOverflow)
    }

    pub fn quadrant(self) -> TorqueSpeedQuadrant {
        match (
            self.torque_micro_nm.signum(),
            self.angular_velocity_microrad_s.signum(),
        ) {
            (1, 1) => TorqueSpeedQuadrant::Q1,
            (-1, 1) => TorqueSpeedQuadrant::Q2,
            (-1, -1) => TorqueSpeedQuadrant::Q3,
            (1, -1) => TorqueSpeedQuadrant::Q4,
            _ => TorqueSpeedQuadrant::Boundary,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TorqueSpeedQuadrant {
    /// Positive torque and positive angular velocity.
    Q1,
    /// Negative torque and positive angular velocity.
    Q2,
    /// Negative torque and negative angular velocity.
    Q3,
    /// Positive torque and negative angular velocity.
    Q4,
    /// Zero torque or zero angular velocity; not a powered quadrant.
    Boundary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LossConsistency {
    /// The entire predicted electrical-minus-mechanical interval is >= 0.
    Consistent,
    /// The interval spans zero; current uncertainty cannot establish a
    /// non-negative loss lower bound.
    Uncertain,
    /// The entire interval is < 0; the declared map violates the passive
    /// electrical-minus-mechanical sign convention over this interval.
    Inconsistent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SignedPowerInterval {
    lower_pw: i128,
    nominal_pw: i128,
    upper_pw: i128,
}

impl SignedPowerInterval {
    pub fn lower_pw(self) -> i128 {
        self.lower_pw
    }

    pub fn nominal_pw(self) -> i128 {
        self.nominal_pw
    }

    pub fn upper_pw(self) -> i128 {
        self.upper_pw
    }

    fn consistency(self) -> LossConsistency {
        if self.lower_pw >= 0 {
            LossConsistency::Consistent
        } else if self.upper_pw < 0 {
            LossConsistency::Inconsistent
        } else {
            LossConsistency::Uncertain
        }
    }
}

/// A non-negative power decomposition derived from a signed source interval.
/// It does not infer electrical regeneration from negative mechanical work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NonNegativePowerInterval {
    lower_pw: i128,
    nominal_pw: i128,
    upper_pw: i128,
}

impl NonNegativePowerInterval {
    pub fn lower_pw(self) -> i128 {
        self.lower_pw
    }

    pub fn nominal_pw(self) -> i128 {
        self.nominal_pw
    }

    pub fn upper_pw(self) -> i128 {
        self.upper_pw
    }

    fn positive_part(value: PowerInterval) -> Self {
        Self {
            lower_pw: i128::from(value.lower_pw.max(0)),
            nominal_pw: i128::from(value.nominal_pw.max(0)),
            upper_pw: i128::from(value.upper_pw.max(0)),
        }
    }

    fn negative_part(value: PowerInterval) -> Self {
        // Convert to i128 before negation so i64::MIN is representable.
        Self {
            lower_pw: (-i128::from(value.upper_pw)).max(0),
            nominal_pw: (-i128::from(value.nominal_pw)).max(0),
            upper_pw: (-i128::from(value.lower_pw)).max(0),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeEvidence {
    pub node_index: usize,
    pub corner_mask: u8,
    pub interpolation_weight_q64: u128,
    pub sample_id: String,
    pub input_artifact_digest: String,
    pub calibration_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActuatorPrediction {
    pub actuator_id: String,
    pub drive_id: String,
    pub gearbox_id: String,
    pub firmware_digest: String,
    pub domain_digest: String,
    pub map_digest: String,
    pub operating_point: OperatingPoint,
    pub quadrant: TorqueSpeedQuadrant,
    pub interpolation_cell: [usize; 4],
    pub interpolation_fraction_q16: [u32; 4],
    /// Signed actuator electrical input. Negative values mean measured/modelled
    /// energy returned to the electrical input boundary.
    pub electrical_input_power_pw: PowerInterval,
    /// Non-negative electrical draw; derived from electrical input only.
    pub electrical_draw_power_pw: NonNegativePowerInterval,
    /// Non-negative electrical return; derived from electrical input only.
    pub electrical_returned_power_pw: NonNegativePowerInterval,
    /// Mechanical tau*omega, with the exact query-coordinate nominal.
    pub mechanical_joint_power_pw: PowerInterval,
    /// Non-negative mechanical work delivered to the joint.
    pub mechanical_output_power_pw: NonNegativePowerInterval,
    /// Non-negative mechanical work absorbed by the actuator.
    pub mechanical_absorbed_power_pw: NonNegativePowerInterval,
    /// Signed electrical input minus mechanical output. This remains an
    /// unresolved total residual unless motor/driver/gearbox boundaries are
    /// independently instrumented.
    pub electrical_minus_mechanical_loss_pw: SignedPowerInterval,
    pub loss_consistency: LossConsistency,
    /// All 16 cell corners, including zero-weight corners, with source and
    /// calibration identities. Fractions/weights are Q16/Q64 fixed point.
    pub node_evidence: Vec<NodeEvidence>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActuatorPowerMap {
    actuator_id: String,
    drive_id: String,
    gearbox_id: String,
    firmware_digest: String,
    domain_digest: String,
    map_digest: String,
    axes: ActuatorAxes,
    /// Row-major order: torque, angular velocity, temperature, bus voltage.
    nodes: Vec<CalibrationNode>,
}

impl ActuatorPowerMap {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        actuator_id: String,
        drive_id: String,
        gearbox_id: String,
        firmware_digest: String,
        domain_digest: String,
        map_digest: String,
        axes: ActuatorAxes,
        nodes: Vec<CalibrationNode>,
    ) -> Result<Self, MapError> {
        if [
            actuator_id.as_str(),
            drive_id.as_str(),
            gearbox_id.as_str(),
            firmware_digest.as_str(),
            domain_digest.as_str(),
            map_digest.as_str(),
        ]
        .iter()
        .any(|value| value.trim().is_empty())
        {
            return Err(MapError::EmptyIdentity);
        }

        let dimensions = axes.dimensions();
        let mut expected = 1_usize;
        for dimension in dimensions {
            expected = expected
                .checked_mul(dimension)
                .ok_or(MapError::GridSizeOverflow)?;
        }
        if expected > MAX_GRID_POINTS {
            return Err(MapError::GridTooLarge);
        }
        if nodes.len() != expected {
            return Err(MapError::GridValueCountMismatch);
        }

        let mut sample_ids = std::collections::BTreeSet::new();
        for (node_index, node) in nodes.iter().enumerate() {
            if node.sample_id.trim().is_empty()
                || node.input_artifact_digest.trim().is_empty()
                || node.calibration_digest.trim().is_empty()
            {
                return Err(MapError::EmptyIdentity);
            }
            if !sample_ids.insert(node.sample_id.clone()) {
                return Err(MapError::DuplicateSampleId(node.sample_id.clone()));
            }

            let coordinates = coordinates_from_flat_index(dimensions, node_index)?;
            let torque = axes.torque_micro_nm[coordinates[0]];
            let angular_velocity = axes.angular_velocity_microrad_s[coordinates[1]];
            let expected_mechanical_pw = i64::from(torque)
                .checked_mul(i64::from(angular_velocity))
                .ok_or(MapError::ArithmeticOverflow)?;
            if node.mechanical_joint_power_pw.nominal_pw != expected_mechanical_pw {
                return Err(MapError::MechanicalNominalMismatch {
                    node_index,
                    expected_pw: expected_mechanical_pw,
                    observed_pw: node.mechanical_joint_power_pw.nominal_pw,
                });
            }
        }

        Ok(Self {
            actuator_id,
            drive_id,
            gearbox_id,
            firmware_digest,
            domain_digest,
            map_digest,
            axes,
            nodes,
        })
    }

    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Query one point inside the calibrated four-axis envelope. No
    /// extrapolation, edge clamping or default temperature/voltage is allowed.
    ///
    /// Q16 fractions are truncated downward. The lower/upper output bounds
    /// are widened over every adjacent Q16 fraction corner and rounded
    /// outward. Nominal electrical power uses nearest-ties-to-even.
    pub fn predict(&self, operating_point: OperatingPoint) -> Result<ActuatorPrediction, MapError> {
        let query = operating_point.coordinates();
        let axis_ids = [
            MapAxis::Torque,
            MapAxis::AngularVelocity,
            MapAxis::Temperature,
            MapAxis::DcBusVoltage,
        ];
        let mut cells = [0_usize; 4];
        let mut fractions = [0_u32; 4];
        let mut fraction_remainders = [false; 4];

        for (axis_index, axis) in axis_ids.iter().enumerate() {
            let (cell, fraction, has_remainder) =
                locate_cell(self.axes.axis(*axis), query[axis_index], *axis)?;
            cells[axis_index] = cell;
            fractions[axis_index] = fraction;
            fraction_remainders[axis_index] = has_remainder;
        }

        let electrical_nominal = interpolate_component(
            &self.nodes,
            &self.axes,
            cells,
            fractions,
            PowerField::Electrical,
            PowerComponent::Nominal,
            Rounding::NearestTiesEven,
        )?;
        let (electrical_lower, electrical_upper) = interpolate_quantization_bounds(
            &self.nodes,
            &self.axes,
            cells,
            fractions,
            fraction_remainders,
            PowerField::Electrical,
        )?;
        let electrical_input_power_pw =
            PowerInterval::new(electrical_lower, electrical_nominal, electrical_upper)?;

        let mechanical_nominal = operating_point.mechanical_power_pw()?;
        let (mechanical_lower, mechanical_upper) = interpolate_quantization_bounds(
            &self.nodes,
            &self.axes,
            cells,
            fractions,
            fraction_remainders,
            PowerField::Mechanical,
        )?;
        if mechanical_lower > mechanical_nominal || mechanical_upper < mechanical_nominal {
            return Err(MapError::MechanicalInterpolationDoesNotEncloseNominal);
        }
        let mechanical_joint_power_pw =
            PowerInterval::new(mechanical_lower, mechanical_nominal, mechanical_upper)?;

        let electrical_minus_mechanical_loss_pw = SignedPowerInterval {
            lower_pw: i128::from(electrical_input_power_pw.lower_pw)
                .checked_sub(i128::from(mechanical_joint_power_pw.upper_pw))
                .ok_or(MapError::ArithmeticOverflow)?,
            nominal_pw: i128::from(electrical_input_power_pw.nominal_pw)
                .checked_sub(i128::from(mechanical_joint_power_pw.nominal_pw))
                .ok_or(MapError::ArithmeticOverflow)?,
            upper_pw: i128::from(electrical_input_power_pw.upper_pw)
                .checked_sub(i128::from(mechanical_joint_power_pw.lower_pw))
                .ok_or(MapError::ArithmeticOverflow)?,
        };

        Ok(ActuatorPrediction {
            actuator_id: self.actuator_id.clone(),
            drive_id: self.drive_id.clone(),
            gearbox_id: self.gearbox_id.clone(),
            firmware_digest: self.firmware_digest.clone(),
            domain_digest: self.domain_digest.clone(),
            map_digest: self.map_digest.clone(),
            operating_point,
            quadrant: operating_point.quadrant(),
            interpolation_cell: cells,
            interpolation_fraction_q16: fractions,
            electrical_input_power_pw,
            electrical_draw_power_pw: NonNegativePowerInterval::positive_part(
                electrical_input_power_pw,
            ),
            electrical_returned_power_pw: NonNegativePowerInterval::negative_part(
                electrical_input_power_pw,
            ),
            mechanical_joint_power_pw,
            mechanical_output_power_pw: NonNegativePowerInterval::positive_part(
                mechanical_joint_power_pw,
            ),
            mechanical_absorbed_power_pw: NonNegativePowerInterval::negative_part(
                mechanical_joint_power_pw,
            ),
            loss_consistency: electrical_minus_mechanical_loss_pw.consistency(),
            electrical_minus_mechanical_loss_pw,
            node_evidence: collect_node_evidence(&self.nodes, &self.axes, cells, fractions)?,
        })
    }
}

#[derive(Debug, Clone, Copy)]
enum PowerField {
    Electrical,
    Mechanical,
}

#[derive(Debug, Clone, Copy)]
enum PowerComponent {
    Lower,
    Nominal,
    Upper,
}

#[derive(Debug, Clone, Copy)]
enum Rounding {
    Floor,
    Ceil,
    NearestTiesEven,
}

fn locate_cell(
    axis_values: &[i32],
    query: i32,
    axis: MapAxis,
) -> Result<(usize, u32, bool), MapError> {
    let minimum = axis_values[0];
    let maximum = axis_values[axis_values.len() - 1];
    if query < minimum || query > maximum {
        return Err(MapError::OutsideCalibrationEnvelope {
            axis,
            value: query,
            minimum,
            maximum,
        });
    }

    let lower_index = if query == maximum {
        axis_values.len() - 2
    } else {
        axis_values.partition_point(|value| *value <= query) - 1
    };
    let lower = i64::from(axis_values[lower_index]);
    let upper = i64::from(axis_values[lower_index + 1]);
    let query = i64::from(query);
    let numerator = (query - lower)
        .checked_mul(Q16 as i64)
        .ok_or(MapError::ArithmeticOverflow)?;
    let denominator = upper - lower;
    let fraction = numerator / denominator;
    let has_remainder = numerator % denominator != 0;
    let fraction = u32::try_from(fraction).map_err(|_| MapError::ArithmeticOverflow)?;
    Ok((lower_index, fraction, has_remainder))
}

fn flat_index(dimensions: [usize; 4], coordinates: [usize; 4]) -> Result<usize, MapError> {
    coordinates[0]
        .checked_mul(dimensions[1])
        .and_then(|value| value.checked_add(coordinates[1]))
        .and_then(|value| value.checked_mul(dimensions[2]))
        .and_then(|value| value.checked_add(coordinates[2]))
        .and_then(|value| value.checked_mul(dimensions[3]))
        .and_then(|value| value.checked_add(coordinates[3]))
        .ok_or(MapError::GridSizeOverflow)
}

fn coordinates_from_flat_index(
    dimensions: [usize; 4],
    flat: usize,
) -> Result<[usize; 4], MapError> {
    let mut remainder = flat;
    let mut coordinates = [0_usize; 4];
    for reverse_index in 0..4 {
        let axis_index = 3 - reverse_index;
        let dimension = dimensions[axis_index];
        if dimension == 0 {
            return Err(MapError::GridSizeOverflow);
        }
        coordinates[axis_index] = remainder % dimension;
        remainder /= dimension;
    }
    if remainder != 0 {
        return Err(MapError::GridSizeOverflow);
    }
    Ok(coordinates)
}

fn interval_component(interval: PowerInterval, component: PowerComponent) -> i64 {
    match component {
        PowerComponent::Lower => interval.lower_pw,
        PowerComponent::Nominal => interval.nominal_pw,
        PowerComponent::Upper => interval.upper_pw,
    }
}

fn field_interval(node: &CalibrationNode, field: PowerField) -> PowerInterval {
    match field {
        PowerField::Electrical => node.electrical_input_power_pw,
        PowerField::Mechanical => node.mechanical_joint_power_pw,
    }
}

fn corner_weight(fractions: [u32; 4], corner_mask: u32) -> Result<u128, MapError> {
    let mut weight = 1_u128;
    for (axis_index, fraction_value) in fractions.iter().enumerate() {
        let fraction = u128::from(*fraction_value);
        let axis_weight = if (corner_mask & (1 << axis_index)) != 0 {
            fraction
        } else {
            Q16 - fraction
        };
        weight = weight
            .checked_mul(axis_weight)
            .ok_or(MapError::ArithmeticOverflow)?;
    }
    Ok(weight)
}

fn interpolate_component(
    nodes: &[CalibrationNode],
    axes: &ActuatorAxes,
    cells: [usize; 4],
    fractions: [u32; 4],
    field: PowerField,
    component: PowerComponent,
    rounding: Rounding,
) -> Result<i64, MapError> {
    let dimensions = axes.dimensions();
    let mut weighted_sum = 0_i128;
    let mut total_weight = 0_u128;

    for corner_mask in 0_u32..16 {
        let mut coordinates = [0_usize; 4];
        for axis_index in 0..4 {
            let use_upper = (corner_mask & (1 << axis_index)) != 0;
            coordinates[axis_index] = cells[axis_index] + usize::from(use_upper);
        }

        let index = flat_index(dimensions, coordinates)?;
        let weight = corner_weight(fractions, corner_mask)?;
        let power = i128::from(interval_component(
            field_interval(&nodes[index], field),
            component,
        ));
        let weighted = power
            .checked_mul(i128::try_from(weight).map_err(|_| MapError::ArithmeticOverflow)?)
            .ok_or(MapError::ArithmeticOverflow)?;
        weighted_sum = weighted_sum
            .checked_add(weighted)
            .ok_or(MapError::ArithmeticOverflow)?;
        total_weight = total_weight
            .checked_add(weight)
            .ok_or(MapError::ArithmeticOverflow)?;
    }

    if total_weight != TOTAL_WEIGHT_Q64 {
        return Err(MapError::ArithmeticOverflow);
    }
    round_divide(weighted_sum, TOTAL_WEIGHT_I128, rounding)
}

fn interpolate_quantization_bounds(
    nodes: &[CalibrationNode],
    axes: &ActuatorAxes,
    cells: [usize; 4],
    base_fractions: [u32; 4],
    uncertain_fractions: [bool; 4],
    field: PowerField,
) -> Result<(i64, i64), MapError> {
    let mut lower = i64::MAX;
    let mut upper = i64::MIN;

    // The multi-linear interpolant is affine in each axis separately. Its
    // extrema over the Q16 quantization box therefore occur at its 16 corners.
    for mask in 0_u32..16 {
        let mut fractions = base_fractions;
        for (axis_index, uncertain) in uncertain_fractions.iter().enumerate() {
            if *uncertain && (mask & (1 << axis_index)) != 0 {
                fractions[axis_index] = fractions[axis_index]
                    .checked_add(1)
                    .ok_or(MapError::ArithmeticOverflow)?;
            }
        }
        let lower_candidate = interpolate_component(
            nodes,
            axes,
            cells,
            fractions,
            field,
            PowerComponent::Lower,
            Rounding::Floor,
        )?;
        let upper_candidate = interpolate_component(
            nodes,
            axes,
            cells,
            fractions,
            field,
            PowerComponent::Upper,
            Rounding::Ceil,
        )?;
        lower = lower.min(lower_candidate);
        upper = upper.max(upper_candidate);
    }
    Ok((lower, upper))
}

fn collect_node_evidence(
    nodes: &[CalibrationNode],
    axes: &ActuatorAxes,
    cells: [usize; 4],
    fractions: [u32; 4],
) -> Result<Vec<NodeEvidence>, MapError> {
    let dimensions = axes.dimensions();
    let mut evidence = Vec::with_capacity(16);
    for corner_mask in 0_u32..16 {
        let mut coordinates = [0_usize; 4];
        for axis_index in 0..4 {
            let use_upper = (corner_mask & (1 << axis_index)) != 0;
            coordinates[axis_index] = cells[axis_index] + usize::from(use_upper);
        }
        let node_index = flat_index(dimensions, coordinates)?;
        let node = &nodes[node_index];
        evidence.push(NodeEvidence {
            node_index,
            corner_mask: u8::try_from(corner_mask).map_err(|_| MapError::ArithmeticOverflow)?,
            interpolation_weight_q64: corner_weight(fractions, corner_mask)?,
            sample_id: node.sample_id.clone(),
            input_artifact_digest: node.input_artifact_digest.clone(),
            calibration_digest: node.calibration_digest.clone(),
        });
    }
    Ok(evidence)
}

fn round_divide(numerator: i128, denominator: i128, rounding: Rounding) -> Result<i64, MapError> {
    if denominator <= 0 {
        return Err(MapError::ArithmeticOverflow);
    }
    let floor = numerator.div_euclid(denominator);
    let remainder = numerator.rem_euclid(denominator);
    let rounded = match rounding {
        Rounding::Floor => floor,
        Rounding::Ceil => {
            if remainder == 0 {
                floor
            } else {
                floor.checked_add(1).ok_or(MapError::ArithmeticOverflow)?
            }
        }
        Rounding::NearestTiesEven => {
            let doubled_remainder = remainder
                .checked_mul(2)
                .ok_or(MapError::ArithmeticOverflow)?;
            match doubled_remainder.cmp(&denominator) {
                std::cmp::Ordering::Less => floor,
                std::cmp::Ordering::Greater => {
                    floor.checked_add(1).ok_or(MapError::ArithmeticOverflow)?
                }
                std::cmp::Ordering::Equal => {
                    if floor.rem_euclid(2) == 0 {
                        floor
                    } else {
                        floor.checked_add(1).ok_or(MapError::ArithmeticOverflow)?
                    }
                }
            }
        }
    };
    i64::try_from(rounded).map_err(|_| MapError::ArithmeticOverflow)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn axes(
        torque: Vec<i32>,
        speed: Vec<i32>,
        temperature: Vec<i32>,
        voltage: Vec<i32>,
    ) -> ActuatorAxes {
        ActuatorAxes::new(torque, speed, temperature, voltage).unwrap()
    }

    fn map_from<F>(axes: ActuatorAxes, mut electrical: F) -> ActuatorPowerMap
    where
        F: FnMut(i32, i32, i32, i32) -> i64,
    {
        let mut nodes = Vec::new();
        let mut index = 0_usize;
        for torque in &axes.torque_micro_nm {
            for speed in &axes.angular_velocity_microrad_s {
                for temperature in &axes.temperature_millic {
                    for voltage in &axes.dc_bus_voltage_mv {
                        let mechanical = i64::from(*torque) * i64::from(*speed);
                        let electrical_nominal =
                            electrical(*torque, *speed, *temperature, *voltage);
                        nodes.push(
                            CalibrationNode::new(
                                format!("sample-{index}"),
                                format!("input-artifact-{index}"),
                                format!("calibration-{index}"),
                                PowerInterval::new(
                                    electrical_nominal - 2,
                                    electrical_nominal,
                                    electrical_nominal + 2,
                                )
                                .unwrap(),
                                PowerInterval::new(mechanical - 1, mechanical, mechanical + 1)
                                    .unwrap(),
                            )
                            .unwrap(),
                        );
                        index += 1;
                    }
                }
            }
        }
        ActuatorPowerMap::new(
            "actuator-A".to_owned(),
            "drive-A".to_owned(),
            "gearbox-A".to_owned(),
            "firmware-7".to_owned(),
            "domain-rough-v1".to_owned(),
            "map-digest-5".to_owned(),
            axes,
            nodes,
        )
        .unwrap()
    }

    fn operating_point(torque: i32, speed: i32, temperature: i32, voltage: i32) -> OperatingPoint {
        OperatingPoint {
            torque_micro_nm: torque,
            angular_velocity_microrad_s: speed,
            temperature_millic: temperature,
            dc_bus_voltage_mv: voltage,
        }
    }

    fn standard_axes() -> ActuatorAxes {
        axes(vec![0, 2], vec![0, 2], vec![0, 2], vec![24_000, 26_000])
    }

    #[test]
    fn exact_four_axis_linear_interpolation_matches_known_answer() {
        let map = map_from(standard_axes(), |torque, speed, temperature, voltage| {
            100 + 2 * i64::from(torque)
                + 3 * i64::from(speed)
                + 4 * i64::from(temperature)
                + i64::from(voltage - 24_000) / 1_000
        });
        let prediction = map.predict(operating_point(1, 1, 1, 25_000)).unwrap();

        assert_eq!(
            prediction.electrical_input_power_pw,
            PowerInterval::new(108, 110, 112).unwrap()
        );
        assert_eq!(
            prediction.mechanical_joint_power_pw,
            PowerInterval::new(0, 1, 2).unwrap()
        );
        assert_eq!(prediction.quadrant, TorqueSpeedQuadrant::Q1);
        assert_eq!(prediction.interpolation_cell, [0, 0, 0, 0]);
        assert_eq!(
            prediction.interpolation_fraction_q16,
            [32_768, 32_768, 32_768, 32_768]
        );
        assert_eq!(
            prediction.electrical_minus_mechanical_loss_pw,
            SignedPowerInterval {
                lower_pw: 106,
                nominal_pw: 109,
                upper_pw: 112
            }
        );
        assert_eq!(prediction.loss_consistency, LossConsistency::Consistent);
        assert_eq!(prediction.node_evidence.len(), 16);
    }

    #[test]
    fn all_four_torque_speed_quadrants_are_preserved() {
        let map = map_from(
            axes(vec![-2, 2], vec![-3, 3], vec![0, 1], vec![24_000, 26_000]),
            |torque, speed, _temperature, _voltage| i64::from(torque) * i64::from(speed) + 1,
        );

        let cases = [
            (operating_point(1, 2, 0, 24_000), TorqueSpeedQuadrant::Q1, 3),
            (
                operating_point(-1, 2, 0, 24_000),
                TorqueSpeedQuadrant::Q2,
                -1,
            ),
            (
                operating_point(-1, -2, 0, 24_000),
                TorqueSpeedQuadrant::Q3,
                3,
            ),
            (
                operating_point(1, -2, 0, 24_000),
                TorqueSpeedQuadrant::Q4,
                -1,
            ),
        ];

        for (point, expected_quadrant, expected_electrical) in cases {
            let prediction = map.predict(point).unwrap();
            assert_eq!(prediction.quadrant, expected_quadrant);
            assert_eq!(
                prediction.electrical_input_power_pw.nominal_pw(),
                expected_electrical
            );
        }
    }

    #[test]
    fn negative_mechanical_work_does_not_invent_electrical_regeneration() {
        let map = map_from(
            axes(vec![-2, 2], vec![-2, 2], vec![0, 1], vec![24_000, 26_000]),
            |_torque, _speed, _temperature, _voltage| 0,
        );
        let prediction = map.predict(operating_point(-1, 2, 0, 24_000)).unwrap();

        assert!(prediction.mechanical_joint_power_pw.nominal_pw() < 0);
        assert_eq!(prediction.electrical_returned_power_pw.nominal_pw(), 0);
        assert_eq!(prediction.mechanical_absorbed_power_pw.nominal_pw(), 2);
    }

    #[test]
    fn electrical_return_is_derived_only_from_signed_electrical_power() {
        let map = map_from(
            axes(vec![-2, 2], vec![-2, 2], vec![0, 1], vec![24_000, 26_000]),
            |torque, speed, _temperature, _voltage| i64::from(torque) * i64::from(speed) - 3,
        );
        let prediction = map.predict(operating_point(-1, 2, 0, 24_000)).unwrap();

        assert_eq!(prediction.mechanical_joint_power_pw.nominal_pw(), -2);
        assert_eq!(prediction.electrical_input_power_pw.nominal_pw(), -5);
        assert_eq!(prediction.electrical_returned_power_pw.nominal_pw(), 5);
        assert_eq!(prediction.electrical_draw_power_pw.nominal_pw(), 0);
    }

    #[test]
    fn query_outside_each_calibrated_axis_is_rejected() {
        let map = map_from(standard_axes(), |_torque, _speed, _temp, _voltage| 10);
        let cases = [
            (operating_point(-1, 1, 1, 25_000), MapAxis::Torque),
            (operating_point(1, 3, 1, 25_000), MapAxis::AngularVelocity),
            (operating_point(1, 1, -1, 25_000), MapAxis::Temperature),
            (operating_point(1, 1, 1, 27_000), MapAxis::DcBusVoltage),
        ];
        for (point, expected_axis) in cases {
            let result = map.predict(point).unwrap_err();
            assert!(matches!(
                result,
                MapError::OutsideCalibrationEnvelope { axis, .. } if axis == expected_axis
            ));
        }
    }

    #[test]
    fn inclusive_upper_boundary_uses_last_cell_with_unit_fraction() {
        let map = map_from(standard_axes(), |torque, speed, temperature, _voltage| {
            i64::from(torque) + 2 * i64::from(speed) + 4 * i64::from(temperature)
        });
        let prediction = map.predict(operating_point(2, 2, 2, 26_000)).unwrap();
        assert_eq!(prediction.interpolation_cell, [0, 0, 0, 0]);
        assert_eq!(prediction.interpolation_fraction_q16, [65_536; 4]);
        assert_eq!(prediction.electrical_input_power_pw.nominal_pw(), 14);
    }

    #[test]
    fn source_and_calibration_provenance_is_retained_for_all_corners() {
        let map = map_from(standard_axes(), |_torque, _speed, _temp, _voltage| 50);
        let prediction = map.predict(operating_point(1, 1, 1, 25_000)).unwrap();

        assert_eq!(prediction.node_evidence.len(), 16);
        assert_eq!(prediction.node_evidence[0].sample_id, "sample-0");
        assert_eq!(
            prediction.node_evidence[0].input_artifact_digest,
            "input-artifact-0"
        );
        assert_eq!(
            prediction.node_evidence[0].calibration_digest,
            "calibration-0"
        );
        let weight_sum = prediction
            .node_evidence
            .iter()
            .map(|node| node.interpolation_weight_q64)
            .sum::<u128>();
        assert_eq!(weight_sum, TOTAL_WEIGHT_Q64);
    }

    #[test]
    fn duplicate_sample_ids_are_rejected() {
        let axes = standard_axes();
        let mut nodes = Vec::new();
        for index in 0..16 {
            nodes.push(
                CalibrationNode::new(
                    if index < 2 {
                        "duplicate".to_owned()
                    } else {
                        format!("sample-{index}")
                    },
                    format!("artifact-{index}"),
                    format!("calibration-{index}"),
                    PowerInterval::new(0, 1, 2).unwrap(),
                    PowerInterval::new(0, 0, 1).unwrap(),
                )
                .unwrap(),
            );
        }
        assert_eq!(
            ActuatorPowerMap::new(
                "actuator".to_owned(),
                "drive".to_owned(),
                "gearbox".to_owned(),
                "firmware".to_owned(),
                "domain".to_owned(),
                "map".to_owned(),
                axes,
                nodes,
            ),
            Err(MapError::DuplicateSampleId("duplicate".to_owned()))
        );
    }

    #[test]
    fn mechanical_nominal_must_match_signed_tau_times_omega() {
        let axes = standard_axes();
        let mut nodes = Vec::new();
        for index in 0..16 {
            let coordinates = coordinates_from_flat_index(axes.dimensions(), index).unwrap();
            let torque = axes.torque_micro_nm[coordinates[0]];
            let speed = axes.angular_velocity_microrad_s[coordinates[1]];
            let expected = i64::from(torque) * i64::from(speed);
            nodes.push(
                CalibrationNode::new(
                    format!("sample-{index}"),
                    format!("artifact-{index}"),
                    format!("calibration-{index}"),
                    PowerInterval::new(10, 10, 10).unwrap(),
                    PowerInterval::new(expected - 1, expected + 1, expected + 2).unwrap(),
                )
                .unwrap(),
            );
        }
        assert_eq!(
            ActuatorPowerMap::new(
                "actuator".to_owned(),
                "drive".to_owned(),
                "gearbox".to_owned(),
                "firmware".to_owned(),
                "domain".to_owned(),
                "map".to_owned(),
                axes,
                nodes,
            ),
            Err(MapError::MechanicalNominalMismatch {
                node_index: 0,
                expected_pw: 0,
                observed_pw: 1
            })
        );
    }

    #[test]
    fn invalid_identities_and_power_intervals_are_rejected() {
        assert_eq!(
            PowerInterval::new(2, 1, 3),
            Err(MapError::InvalidPowerInterval)
        );
        assert_eq!(
            CalibrationNode::new(
                "sample".to_owned(),
                " ".to_owned(),
                "calibration".to_owned(),
                PowerInterval::new(0, 0, 0).unwrap(),
                PowerInterval::new(0, 0, 0).unwrap()
            ),
            Err(MapError::EmptyIdentity)
        );
    }

    #[test]
    fn nonmonotonic_and_undersized_axes_are_rejected() {
        assert_eq!(
            ActuatorAxes::new(vec![0, 0], vec![0, 1], vec![0, 1], vec![24_000, 26_000]),
            Err(MapError::AxisNotStrictlyIncreasing(MapAxis::Torque))
        );
        assert_eq!(
            ActuatorAxes::new(vec![0], vec![0, 1], vec![0, 1], vec![24_000, 26_000]),
            Err(MapError::InvalidAxisLength(MapAxis::Torque))
        );
    }

    #[test]
    fn axis_with_oversized_interval_is_rejected() {
        assert_eq!(
            ActuatorAxes::new(
                vec![i32::MIN, 0],
                vec![0, 1],
                vec![0, 1],
                vec![24_000, 26_000]
            ),
            Err(MapError::AxisRangeTooWide(MapAxis::Torque))
        );
    }

    #[test]
    fn wrong_grid_tensor_size_is_rejected() {
        assert_eq!(
            ActuatorPowerMap::new(
                "actuator".to_owned(),
                "drive".to_owned(),
                "gearbox".to_owned(),
                "firmware".to_owned(),
                "domain".to_owned(),
                "map".to_owned(),
                standard_axes(),
                vec![],
            ),
            Err(MapError::GridValueCountMismatch)
        );
    }

    #[test]
    fn oversized_grid_is_rejected_before_node_allocation_is_trusted() {
        let large_axis = (0..16).collect::<Vec<i32>>();
        let axes = ActuatorAxes::new(
            large_axis.clone(),
            large_axis.clone(),
            large_axis.clone(),
            large_axis,
        )
        .unwrap();
        assert_eq!(
            ActuatorPowerMap::new(
                "actuator".to_owned(),
                "drive".to_owned(),
                "gearbox".to_owned(),
                "firmware".to_owned(),
                "domain".to_owned(),
                "map".to_owned(),
                axes,
                Vec::new(),
            ),
            Err(MapError::GridTooLarge)
        );
    }

    #[test]
    fn loss_interval_reports_consistent_uncertain_and_inconsistent_cases() {
        let consistent = SignedPowerInterval {
            lower_pw: 1,
            nominal_pw: 2,
            upper_pw: 3,
        };
        let uncertain = SignedPowerInterval {
            lower_pw: -1,
            nominal_pw: 0,
            upper_pw: 2,
        };
        let inconsistent = SignedPowerInterval {
            lower_pw: -3,
            nominal_pw: -2,
            upper_pw: -1,
        };
        assert_eq!(consistent.consistency(), LossConsistency::Consistent);
        assert_eq!(uncertain.consistency(), LossConsistency::Uncertain);
        assert_eq!(inconsistent.consistency(), LossConsistency::Inconsistent);
    }

    #[test]
    fn q16_rounding_is_nearest_ties_even_and_bounds_are_outward() {
        assert_eq!(round_divide(5, 2, Rounding::NearestTiesEven).unwrap(), 2);
        assert_eq!(round_divide(7, 2, Rounding::NearestTiesEven).unwrap(), 4);
        assert_eq!(round_divide(-5, 2, Rounding::NearestTiesEven).unwrap(), -2);
        assert_eq!(round_divide(-7, 2, Rounding::NearestTiesEven).unwrap(), -4);
        assert_eq!(round_divide(-5, 2, Rounding::Floor).unwrap(), -3);
        assert_eq!(round_divide(-5, 2, Rounding::Ceil).unwrap(), -2);
    }

    #[test]
    fn negative_interpolated_power_rounds_bounds_outward() {
        let map = map_from(standard_axes(), |_torque, _speed, _temp, _voltage| -1);
        let prediction = map.predict(operating_point(1, 1, 1, 25_000)).unwrap();
        assert!(prediction.electrical_input_power_pw.lower_pw() <= -1);
        assert!(prediction.electrical_input_power_pw.upper_pw() >= -1);
        assert_eq!(prediction.electrical_input_power_pw.nominal_pw(), -1);
    }

    #[test]
    fn prediction_exposes_actuator_domain_and_map_identity() {
        let map = map_from(standard_axes(), |_torque, _speed, _temp, _voltage| 100);
        let prediction = map.predict(operating_point(1, 1, 1, 25_000)).unwrap();
        assert_eq!(prediction.actuator_id, "actuator-A");
        assert_eq!(prediction.drive_id, "drive-A");
        assert_eq!(prediction.gearbox_id, "gearbox-A");
        assert_eq!(prediction.firmware_digest, "firmware-7");
        assert_eq!(prediction.domain_digest, "domain-rough-v1");
        assert_eq!(prediction.map_digest, "map-digest-5");
    }

    #[test]
    fn zero_torque_or_speed_is_a_boundary_not_a_powered_quadrant() {
        assert_eq!(
            operating_point(0, 2, 0, 24_000).quadrant(),
            TorqueSpeedQuadrant::Boundary
        );
        assert_eq!(
            operating_point(2, 0, 0, 24_000).quadrant(),
            TorqueSpeedQuadrant::Boundary
        );
    }

    #[test]
    fn electrical_interval_that_straddles_zero_preserves_both_possible_flows() {
        let map = map_from(standard_axes(), |_torque, _speed, _temp, _voltage| 0);
        let prediction = map.predict(operating_point(1, 1, 1, 25_000)).unwrap();
        assert_eq!(
            prediction.electrical_input_power_pw,
            PowerInterval::new(-2, 0, 2).unwrap()
        );
        assert_eq!(prediction.electrical_draw_power_pw.lower_pw(), 0);
        assert_eq!(prediction.electrical_draw_power_pw.upper_pw(), 2);
        assert_eq!(prediction.electrical_returned_power_pw.lower_pw(), 0);
        assert_eq!(prediction.electrical_returned_power_pw.upper_pw(), 2);
        assert_eq!(prediction.loss_consistency, LossConsistency::Uncertain);
    }
}
