//! Bounded, calibrated actuator electrical-power map reference.
//!
//! V0 uses a rectilinear four-axis grid over torque, angular velocity,
//! temperature and DC-bus voltage. It interpolates measured/modelled electrical
//! input power with fixed-point Q16 weights, carries per-node uncertainty
//! bounds, and rejects every query outside the calibrated envelope.
//!
//! This is a map-evaluation primitive, not a motor-identification routine or a
//! hardware-certified drive model. Map provenance and calibration are supplied
//! by the caller and must be independently authenticated.

pub const MAX_AXIS_POINTS: usize = 16;
pub const MAX_GRID_POINTS: usize = 65_536;
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
    InvalidAxisLength(MapAxis),
    AxisNotStrictlyIncreasing(MapAxis),
    AxisRangeTooWide(MapAxis),
    GridSizeOverflow,
    GridTooLarge,
    GridValueCountMismatch,
    InvalidPowerInterval,
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

    pub fn lower_pw(&self) -> i64 {
        self.lower_pw
    }

    pub fn nominal_pw(&self) -> i64 {
        self.nominal_pw
    }

    pub fn upper_pw(&self) -> i64 {
        self.upper_pw
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActuatorAxes {
    torque_micro_nm: Vec<i32>,
    angular_velocity_microrad_s: Vec<i32>,
    temperature_millic: Vec<i32>,
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LossConsistency {
    /// The entire predicted electrical-minus-mechanical interval is >= 0.
    Consistent,
    /// The interval straddles zero; available uncertainty cannot establish
    /// passivity or a loss lower bound.
    Uncertain,
    /// The entire interval is < 0; the declared map/measurement combination
    /// violates the passive loss sign convention.
    Inconsistent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActuatorPrediction {
    pub actuator_id: String,
    pub drive_id: String,
    pub firmware_digest: String,
    pub calibration_digest: String,
    pub map_digest: String,
    pub interpolation_cell: [usize; 4],
    pub interpolation_fraction_q16: [u32; 4],
    /// Signed electrical input power. Negative values are returned electrical
    /// power according to the drive's declared sign convention.
    pub electrical_input_power_pw: PowerInterval,
    /// Mechanical joint power tau*omega, exactly in pW for the declared units.
    pub mechanical_joint_power_pw: i128,
    /// Electrical input minus mechanical joint power. This is not split into
    /// motor/driver/gearbox losses unless each boundary is independently known.
    pub electrical_minus_mechanical_loss_pw: SignedPowerInterval,
    pub loss_consistency: LossConsistency,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SignedPowerInterval {
    lower_pw: i128,
    nominal_pw: i128,
    upper_pw: i128,
}

impl SignedPowerInterval {
    pub fn lower_pw(&self) -> i128 {
        self.lower_pw
    }

    pub fn nominal_pw(&self) -> i128 {
        self.nominal_pw
    }

    pub fn upper_pw(&self) -> i128 {
        self.upper_pw
    }

    fn from_electrical_minus_mechanical(
        electrical: PowerInterval,
        mechanical_pw: i128,
    ) -> Result<Self, MapError> {
        let lower_pw = i128::from(electrical.lower_pw)
            .checked_sub(mechanical_pw)
            .ok_or(MapError::ArithmeticOverflow)?;
        let nominal_pw = i128::from(electrical.nominal_pw)
            .checked_sub(mechanical_pw)
            .ok_or(MapError::ArithmeticOverflow)?;
        let upper_pw = i128::from(electrical.upper_pw)
            .checked_sub(mechanical_pw)
            .ok_or(MapError::ArithmeticOverflow)?;
        Ok(Self {
            lower_pw,
            nominal_pw,
            upper_pw,
        })
    }

    fn consistency(&self) -> LossConsistency {
        if self.lower_pw >= 0 {
            LossConsistency::Consistent
        } else if self.upper_pw < 0 {
            LossConsistency::Inconsistent
        } else {
            LossConsistency::Uncertain
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActuatorPowerMap {
    actuator_id: String,
    drive_id: String,
    firmware_digest: String,
    calibration_digest: String,
    source_artifact_digest: String,
    map_digest: String,
    axes: ActuatorAxes,
    /// Row-major order: torque, angular velocity, temperature, bus voltage.
    electrical_input_power: Vec<PowerInterval>,
}

impl ActuatorPowerMap {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        actuator_id: String,
        drive_id: String,
        firmware_digest: String,
        calibration_digest: String,
        source_artifact_digest: String,
        map_digest: String,
        axes: ActuatorAxes,
        electrical_input_power: Vec<PowerInterval>,
    ) -> Result<Self, MapError> {
        if [
            actuator_id.as_str(),
            drive_id.as_str(),
            firmware_digest.as_str(),
            calibration_digest.as_str(),
            source_artifact_digest.as_str(),
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
        if electrical_input_power.len() != expected {
            return Err(MapError::GridValueCountMismatch);
        }

        Ok(Self {
            actuator_id,
            drive_id,
            firmware_digest,
            calibration_digest,
            source_artifact_digest,
            map_digest,
            axes,
            electrical_input_power,
        })
    }

    pub fn source_artifact_digest(&self) -> &str {
        &self.source_artifact_digest
    }

    /// Query one point inside the calibrated four-axis envelope.
    ///
    /// Interpolation fractions use Q16 per axis. The 16 non-negative weights
    /// sum exactly to 2^64. Lower power is rounded outward/down, upper power
    /// outward/up, and nominal power to nearest with ties-to-even.
    pub fn predict(
        &self,
        operating_point: OperatingPoint,
    ) -> Result<ActuatorPrediction, MapError> {
        let query = operating_point.coordinates();
        let axes = [
            MapAxis::Torque,
            MapAxis::AngularVelocity,
            MapAxis::Temperature,
            MapAxis::DcBusVoltage,
        ];
        let mut cells = [0_usize; 4];
        let mut fractions = [0_u32; 4];
        let mut fraction_remainders = [false; 4];

        for (index, axis) in axes.iter().enumerate() {
            let (cell, fraction, has_remainder) =
                locate_cell(self.axes.axis(*axis), query[index], *axis)?;
            cells[index] = cell;
            fractions[index] = fraction;
            fraction_remainders[index] = has_remainder;
        }

        let nominal_pw = interpolate_component(
            &self.electrical_input_power,
            &self.axes,
            cells,
            fractions,
            PowerComponent::Nominal,
            Rounding::NearestTiesEven,
        )?;
        let (lower_pw, upper_pw) = interpolate_quantization_bounds(
            &self.electrical_input_power,
            &self.axes,
            cells,
            fractions,
            fraction_remainders,
        )?;
        let electrical_input_power_pw =
            PowerInterval::new(lower_pw, nominal_pw, upper_pw)?;

        let mechanical_joint_power_pw = i128::from(operating_point.torque_micro_nm)
            .checked_mul(i128::from(
                operating_point.angular_velocity_microrad_s,
            ))
            .ok_or(MapError::ArithmeticOverflow)?;
        let electrical_minus_mechanical_loss_pw =
            SignedPowerInterval::from_electrical_minus_mechanical(
                electrical_input_power_pw,
                mechanical_joint_power_pw,
            )?;
        let loss_consistency = electrical_minus_mechanical_loss_pw.consistency();

        Ok(ActuatorPrediction {
            actuator_id: self.actuator_id.clone(),
            drive_id: self.drive_id.clone(),
            firmware_digest: self.firmware_digest.clone(),
            calibration_digest: self.calibration_digest.clone(),
            map_digest: self.map_digest.clone(),
            interpolation_cell: cells,
            interpolation_fraction_q16: fractions,
            electrical_input_power_pw,
            mechanical_joint_power_pw,
            electrical_minus_mechanical_loss_pw,
            loss_consistency,
        })
    }
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

fn component_value(interval: PowerInterval, component: PowerComponent) -> i64 {
    match component {
        PowerComponent::Lower => interval.lower_pw,
        PowerComponent::Nominal => interval.nominal_pw,
        PowerComponent::Upper => interval.upper_pw,
    }
}

fn interpolate_component(
    values: &[PowerInterval],
    axes: &ActuatorAxes,
    cells: [usize; 4],
    fractions: [u32; 4],
    component: PowerComponent,
    rounding: Rounding,
) -> Result<i64, MapError> {
    let dimensions = axes.dimensions();
    let mut weighted_sum = 0_i128;

    for corner_mask in 0_u32..16 {
        let mut coordinates = [0_usize; 4];
        let mut weight = 1_u128;

        for axis_index in 0..4 {
            let use_upper = (corner_mask & (1 << axis_index)) != 0;
            let fraction = u128::from(fractions[axis_index]);
            let axis_weight = if use_upper { fraction } else { Q16 - fraction };
            weight = weight
                .checked_mul(axis_weight)
                .ok_or(MapError::ArithmeticOverflow)?;
            coordinates[axis_index] = cells[axis_index] + usize::from(use_upper);
        }

        let index = flat_index(dimensions, coordinates)?;
        let power = i128::from(component_value(values[index], component));
        let weighted = power
            .checked_mul(i128::try_from(weight).map_err(|_| MapError::ArithmeticOverflow)?)
            .ok_or(MapError::ArithmeticOverflow)?;
        weighted_sum = weighted_sum
            .checked_add(weighted)
            .ok_or(MapError::ArithmeticOverflow)?;
    }

    if weight_sum(fractions)? != TOTAL_WEIGHT_Q64 {
        return Err(MapError::ArithmeticOverflow);
    }
    round_divide(weighted_sum, TOTAL_WEIGHT_I128, rounding)
}

fn interpolate_quantization_bounds(
    values: &[PowerInterval],
    axes: &ActuatorAxes,
    cells: [usize; 4],
    base_fractions: [u32; 4],
    uncertain_fractions: [bool; 4],
) -> Result<(i64, i64), MapError> {
    let mut lower = i64::MAX;
    let mut upper = i64::MIN;

    // A multi-linear interpolant is affine in each coordinate separately.
    // Therefore its extrema over the Q16 quantization box occur at the 16
    // corners. Evaluating all corners bounds coordinate quantization error.
    for mask in 0_u32..16 {
        let mut fractions = base_fractions;
        for axis_index in 0..4 {
            if uncertain_fractions[axis_index] && (mask & (1 << axis_index)) != 0 {
                fractions[axis_index] = fractions[axis_index]
                    .checked_add(1)
                    .ok_or(MapError::ArithmeticOverflow)?;
            }
        }
        let lower_candidate = interpolate_component(
            values,
            axes,
            cells,
            fractions,
            PowerComponent::Lower,
            Rounding::Floor,
        )?;
        let upper_candidate = interpolate_component(
            values,
            axes,
            cells,
            fractions,
            PowerComponent::Upper,
            Rounding::Ceil,
        )?;
        lower = lower.min(lower_candidate);
        upper = upper.max(upper_candidate);
    }
    Ok((lower, upper))
}

fn weight_sum(fractions: [u32; 4]) -> Result<u128, MapError> {
    // Sum of all 16 corner weights must be the product of four Q16 unity terms.
    let mut sum = 0_u128;
    for mask in 0_u32..16 {
        let mut weight = 1_u128;
        for axis_index in 0..4 {
            let fraction = u128::from(fractions[axis_index]);
            let axis_weight = if (mask & (1 << axis_index)) != 0 {
                fraction
            } else {
                Q16 - fraction
            };
            weight = weight
                .checked_mul(axis_weight)
                .ok_or(MapError::ArithmeticOverflow)?;
        }
        sum = sum.checked_add(weight).ok_or(MapError::ArithmeticOverflow)?;
    }
    Ok(sum)
}

fn round_divide(
    numerator: i128,
    denominator: i128,
    rounding: Rounding,
) -> Result<i64, MapError> {
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

    fn axes() -> ActuatorAxes {
        ActuatorAxes::new(
            vec![0, 2],
            vec![0, 2],
            vec![0, 2],
            vec![0, 2],
        )
        .unwrap()
    }

    fn map_with(values: Vec<PowerInterval>) -> ActuatorPowerMap {
        ActuatorPowerMap::new(
            "actuator-A".to_owned(),
            "drive-A".to_owned(),
            "firmware-7".to_owned(),
            "calibration-3".to_owned(),
            "source-artifact-4".to_owned(),
            "map-digest-5".to_owned(),
            axes(),
            values,
        )
        .unwrap()
    }

    fn interval(lower: i64, nominal: i64, upper: i64) -> PowerInterval {
        PowerInterval::new(lower, nominal, upper).unwrap()
    }

    fn operating_point(
        torque: i32,
        speed: i32,
        temperature: i32,
        bus_voltage: i32,
    ) -> OperatingPoint {
        OperatingPoint {
            torque_micro_nm: torque,
            angular_velocity_microrad_s: speed,
            temperature_millic: temperature,
            dc_bus_voltage_mv: bus_voltage,
        }
    }

    fn linear_grid() -> Vec<PowerInterval> {
        let mut values = Vec::new();
        for torque in [0_i64, 2] {
            for speed in [0_i64, 2] {
                for temperature in [0_i64, 2] {
                    for voltage in [0_i64, 2] {
                        let nominal = torque + 2 * speed + 4 * temperature + 8 * voltage;
                        values.push(interval(nominal - 1, nominal, nominal + 1));
                    }
                }
            }
        }
        values
    }

    #[test]
    fn exact_four_axis_linear_interpolation_matches_known_answer() {
        let map = map_with(linear_grid());
        let prediction = map
            .predict(operating_point(1, 1, 1, 1))
            .unwrap();
        assert_eq!(prediction.electrical_input_power_pw, interval(14, 15, 16));
        assert_eq!(prediction.interpolation_cell, [0, 0, 0, 0]);
        assert_eq!(
            prediction.interpolation_fraction_q16,
            [32_768, 32_768, 32_768, 32_768]
        );
        assert_eq!(prediction.mechanical_joint_power_pw, 1);
        assert_eq!(
            prediction.electrical_minus_mechanical_loss_pw,
            SignedPowerInterval {
                lower_pw: 13,
                nominal_pw: 14,
                upper_pw: 15
            }
        );
        assert_eq!(prediction.loss_consistency, LossConsistency::Consistent);
    }

    #[test]
    fn map_preserves_negative_power_and_four_quadrant_values() {
        let mut values = Vec::new();
        for torque in [0_i64, 2] {
            for speed in [0_i64, 2] {
                for _temperature in [0_i64, 2] {
                    for _voltage in [0_i64, 2] {
                        let signed = if torque == 0 && speed == 2 {
                            -100
                        } else if torque == 2 && speed == 0 {
                            100
                        } else {
                            10
                        };
                        values.push(interval(signed - 1, signed, signed + 1));
                    }
                }
            }
        }
        let map = map_with(values);
        let negative = map
            .predict(operating_point(0, 2, 1, 1))
            .unwrap();
        let positive = map
            .predict(operating_point(2, 0, 1, 1))
            .unwrap();
        assert!(negative.electrical_input_power_pw.nominal_pw() < 0);
        assert!(positive.electrical_input_power_pw.nominal_pw() > 0);
    }

    #[test]
    fn map_rejects_extrapolation_on_each_axis() {
        let map = map_with(vec![interval(10, 12, 14); 16]);
        let cases = [
            (operating_point(-1, 1, 1, 1), MapAxis::Torque),
            (operating_point(1, 3, 1, 1), MapAxis::AngularVelocity),
            (operating_point(1, 1, -1, 1), MapAxis::Temperature),
            (operating_point(1, 1, 1, 3), MapAxis::DcBusVoltage),
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
        let map = map_with(linear_grid());
        let prediction = map
            .predict(operating_point(2, 2, 2, 2))
            .unwrap();
        assert_eq!(prediction.interpolation_cell, [0, 0, 0, 0]);
        assert_eq!(prediction.interpolation_fraction_q16, [65_536; 4]);
        assert_eq!(prediction.electrical_input_power_pw, interval(29, 30, 31));
    }

    #[test]
    fn lower_and_upper_interpolation_round_outward_for_negative_values() {
        let values = vec![interval(-2, -1, 0); 16];
        let map = map_with(values);
        let prediction = map
            .predict(operating_point(1, 1, 1, 1))
            .unwrap();
        assert_eq!(prediction.electrical_input_power_pw, interval(-2, -1, 0));
    }

    #[test]
    fn loss_consistency_distinguishes_safe_uncertain_and_inconsistent_signs() {
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
    fn constructor_rejects_invalid_identity_and_power_interval() {
        assert_eq!(
            PowerInterval::new(2, 1, 3),
            Err(MapError::InvalidPowerInterval)
        );
        assert_eq!(
            ActuatorPowerMap::new(
                " ".to_owned(),
                "drive".to_owned(),
                "firmware".to_owned(),
                "calibration".to_owned(),
                "source".to_owned(),
                "map".to_owned(),
                axes(),
                vec![interval(0, 0, 0); 16],
            ),
            Err(MapError::EmptyIdentity)
        );
    }

    #[test]
    fn constructor_rejects_nonmonotonic_and_undersized_axes() {
        assert_eq!(
            ActuatorAxes::new(vec![0, 0], vec![0, 1], vec![0, 1], vec![0, 1]),
            Err(MapError::AxisNotStrictlyIncreasing(MapAxis::Torque))
        );
        assert_eq!(
            ActuatorAxes::new(vec![0], vec![0, 1], vec![0, 1], vec![0, 1]),
            Err(MapError::InvalidAxisLength(MapAxis::Torque))
        );
    }

    #[test]
    fn constructor_rejects_wrong_grid_tensor_size() {
        assert_eq!(
            ActuatorPowerMap::new(
                "actuator".to_owned(),
                "drive".to_owned(),
                "firmware".to_owned(),
                "calibration".to_owned(),
                "source".to_owned(),
                "map".to_owned(),
                axes(),
                vec![interval(0, 0, 0); 15],
            ),
            Err(MapError::GridValueCountMismatch)
        );
    }

    #[test]
    fn fixed_point_interpolation_preserves_signed_power_at_axis_midpoint() {
        let mut values = Vec::new();
        for torque in [-100_i64, 100] {
            for _speed in [0_i64, 2] {
                for _temperature in [0_i64, 2] {
                    for _voltage in [0_i64, 2] {
                        values.push(interval(torque - 1, torque, torque + 1));
                    }
                }
            }
        }
        let axes = ActuatorAxes::new(
            vec![-100, 100],
            vec![0, 2],
            vec![0, 2],
            vec![0, 2],
        )
        .unwrap();
        let map = ActuatorPowerMap::new(
            "actuator".to_owned(),
            "drive".to_owned(),
            "firmware".to_owned(),
            "calibration".to_owned(),
            "source".to_owned(),
            "map".to_owned(),
            axes,
            values,
        )
        .unwrap();
        let prediction = map
            .predict(operating_point(0, 1, 1, 1))
            .unwrap();
        assert_eq!(prediction.electrical_input_power_pw, interval(-1, 0, 1));
    }

    #[test]
    fn source_and_calibration_digests_are_returned_with_each_prediction() {
        let map = map_with(vec![interval(10, 12, 14); 16]);
        let prediction = map
            .predict(operating_point(1, 1, 1, 1))
            .unwrap();
        assert_eq!(prediction.actuator_id, "actuator-A");
        assert_eq!(prediction.drive_id, "drive-A");
        assert_eq!(prediction.firmware_digest, "firmware-7");
        assert_eq!(prediction.calibration_digest, "calibration-3");
        assert_eq!(prediction.map_digest, "map-digest-5");
        assert_eq!(map.source_artifact_digest(), "source-artifact-4");
    }

    #[test]
    fn arithmetic_rounding_ties_are_to_even_and_outward_bounds_are_safe() {
        assert_eq!(round_divide(5, 2, Rounding::NearestTiesEven).unwrap(), 2);
        assert_eq!(round_divide(7, 2, Rounding::NearestTiesEven).unwrap(), 4);
        assert_eq!(round_divide(-5, 2, Rounding::NearestTiesEven).unwrap(), -2);
        assert_eq!(round_divide(-7, 2, Rounding::NearestTiesEven).unwrap(), -4);
        assert_eq!(round_divide(-5, 2, Rounding::Floor).unwrap(), -3);
        assert_eq!(round_divide(-5, 2, Rounding::Ceil).unwrap(), -2);
    }
}
