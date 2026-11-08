//! Source-qualified, fixed-point energy accounting for embodied locomotion.
//!
//! The crate integrates one already time-aligned signed-power stream at a time.
//! It uses exact rational trapezoidal integration and keeps battery, DC bus,
//! actuator-electrical, joint-mechanical, auxiliary and standby boundaries
//! separate. It never constructs a cross-boundary "total energy" that could
//! double-count nested measurements.

use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum EnergyBoundary {
    /// Positive current means energy leaves the battery.
    BatteryTerminal,
    /// Positive power means energy is delivered from the DC bus to loads.
    DcBus,
    /// Positive power means electrical power enters the actuator drive.
    ActuatorElectrical,
    /// Positive power means mechanical work is delivered by the actuator.
    JointMechanical,
    /// Positive power means energy is consumed by auxiliary equipment.
    AuxiliaryLoad,
    /// Positive power means idle/control-on standby consumption.
    StandbyBaseline,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnergyError {
    EmptyIdentity,
    EmptySamples,
    TooFewSamples,
    EmptySampleId,
    DuplicateSampleId(String),
    DuplicateStreamId(String),
    StreamRunMismatch(String),
    NonMonotonicTimestamp(String),
    GapExceedsPolicy {
        stream_id: String,
        gap_ns: u64,
        maximum_gap_ns: u64,
    },
    ZeroMaximumGap,
    NoStreams,
    InvalidMass,
    InvalidGravity,
    InvalidRatioDenominator,
    ArithmeticOverflow,
}

/// A reduced, non-negative exact rational number.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExactRational {
    numerator: u128,
    denominator: u128,
}

impl ExactRational {
    pub fn zero() -> Self {
        Self {
            numerator: 0,
            denominator: 1,
        }
    }

    pub fn new(numerator: u128, denominator: u128) -> Result<Self, EnergyError> {
        if denominator == 0 {
            return Err(EnergyError::InvalidRatioDenominator);
        }
        let divisor = gcd(numerator, denominator);
        Ok(Self {
            numerator: numerator / divisor,
            denominator: denominator / divisor,
        })
    }

    pub fn numerator(&self) -> u128 {
        self.numerator
    }

    pub fn denominator(&self) -> u128 {
        self.denominator
    }

    pub fn checked_add(self, other: Self) -> Result<Self, EnergyError> {
        let common = gcd(self.denominator, other.denominator);
        let left_scale = other.denominator / common;
        let right_scale = self.denominator / common;
        let left = checked_mul(self.numerator, left_scale)?;
        let right = checked_mul(other.numerator, right_scale)?;
        let numerator = checked_add(left, right)?;
        let denominator = checked_mul(self.denominator, left_scale)?;
        Self::new(numerator, denominator)
    }

    pub fn checked_cmp(self, other: Self) -> Result<std::cmp::Ordering, EnergyError> {
        let left = checked_mul(self.numerator, other.denominator)?;
        let right = checked_mul(other.numerator, self.denominator)?;
        Ok(left.cmp(&right))
    }

    pub fn checked_sub(self, other: Self) -> Result<Self, EnergyError> {
        let common = gcd(self.denominator, other.denominator);
        let left_scale = other.denominator / common;
        let right_scale = self.denominator / common;
        let left = checked_mul(self.numerator, left_scale)?;
        let right = checked_mul(other.numerator, right_scale)?;
        let denominator = checked_mul(self.denominator, left_scale)?;
        let numerator = left
            .checked_sub(right)
            .ok_or(EnergyError::ArithmeticOverflow)?;
        Self::new(numerator, denominator)
    }

    fn divide_by_integer(self, divisor: u128) -> Result<Self, EnergyError> {
        if divisor == 0 {
            return Err(EnergyError::InvalidRatioDenominator);
        }
        Self::new(
            self.numerator,
            checked_mul(self.denominator, divisor)?,
        )
    }
}

/// Exact signed difference, represented without signed-integer overflow.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SignedRational {
    negative: bool,
    magnitude: ExactRational,
}

impl SignedRational {
    fn positive_minus(positive: ExactRational, negative: ExactRational) -> Result<Self, EnergyError> {
        match positive.checked_cmp(negative)? {
            std::cmp::Ordering::Greater => Ok(Self {
                negative: false,
                magnitude: positive.checked_sub(negative)?,
            }),
            std::cmp::Ordering::Less => Ok(Self {
                negative: true,
                magnitude: negative.checked_sub(positive)?,
            }),
            std::cmp::Ordering::Equal => Ok(Self {
                negative: false,
                magnitude: ExactRational::zero(),
            }),
        }
    }

    pub fn is_negative(&self) -> bool {
        self.negative && self.magnitude.numerator != 0
    }

    pub fn magnitude(&self) -> ExactRational {
        self.magnitude
    }

    fn divide_by_integer(self, divisor: u128) -> Result<Self, EnergyError> {
        Ok(Self {
            negative: self.negative,
            magnitude: self.magnitude.divide_by_integer(divisor)?,
        })
    }
}

fn gcd(mut left: u128, mut right: u128) -> u128 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left.max(1)
}

fn checked_add(left: u128, right: u128) -> Result<u128, EnergyError> {
    left.checked_add(right).ok_or(EnergyError::ArithmeticOverflow)
}

fn checked_mul(left: u128, right: u128) -> Result<u128, EnergyError> {
    left.checked_mul(right).ok_or(EnergyError::ArithmeticOverflow)
}

fn nonblank(value: &str) -> bool {
    !value.trim().is_empty()
}

/// Signed power in picowatts. One pW multiplied by one ns is exactly one zJ
/// (zeptojoule), so trapezoidal integration can be represented as a rational
/// number of zJ without floating-point rounding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PowerSample {
    sample_id: String,
    timestamp_ns: u64,
    signed_power_pw: i128,
}

impl PowerSample {
    pub fn new(
        sample_id: String,
        timestamp_ns: u64,
        signed_power_pw: i128,
    ) -> Result<Self, EnergyError> {
        if !nonblank(&sample_id) {
            return Err(EnergyError::EmptySampleId);
        }
        Ok(Self {
            sample_id,
            timestamp_ns,
            signed_power_pw,
        })
    }

    /// Convert battery-terminal voltage/current samples to signed pW.
    /// Positive current means discharge from the battery; negative current
    /// means energy returned to the battery.
    pub fn from_battery_vi(
        sample_id: String,
        timestamp_ns: u64,
        voltage_uv: u64,
        current_ua: i64,
    ) -> Result<Self, EnergyError> {
        let signed_power_pw = i128::from(voltage_uv)
            .checked_mul(i128::from(current_ua))
            .ok_or(EnergyError::ArithmeticOverflow)?;
        Self::new(sample_id, timestamp_ns, signed_power_pw)
    }

    /// Convert joint torque/angular-velocity samples to signed mechanical pW.
    /// Inputs are micro-newton-metres and microradians per second. Positive
    /// tau*omega is actuator-to-joint mechanical work; negative is absorbed
    /// mechanical work, not proof of electrical regeneration.
    pub fn from_joint_torque_velocity(
        sample_id: String,
        timestamp_ns: u64,
        torque_micro_nm: i64,
        angular_velocity_microrad_s: i64,
    ) -> Result<Self, EnergyError> {
        let signed_power_pw = i128::from(torque_micro_nm)
            .checked_mul(i128::from(angular_velocity_microrad_s))
            .ok_or(EnergyError::ArithmeticOverflow)?;
        Self::new(sample_id, timestamp_ns, signed_power_pw)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PowerStream {
    stream_id: String,
    run_id: String,
    boundary: EnergyBoundary,
    sensor_id: String,
    source_artifact_digest: String,
    calibration_digest: String,
    clock_basis_digest: String,
    maximum_sample_gap_ns: u64,
    samples: Vec<PowerSample>,
}

impl PowerStream {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        stream_id: String,
        run_id: String,
        boundary: EnergyBoundary,
        sensor_id: String,
        source_artifact_digest: String,
        calibration_digest: String,
        clock_basis_digest: String,
        maximum_sample_gap_ns: u64,
        samples: Vec<PowerSample>,
    ) -> Result<Self, EnergyError> {
        if [
            stream_id.as_str(),
            run_id.as_str(),
            sensor_id.as_str(),
            source_artifact_digest.as_str(),
            calibration_digest.as_str(),
            clock_basis_digest.as_str(),
        ]
        .iter()
        .any(|value| !nonblank(value))
        {
            return Err(EnergyError::EmptyIdentity);
        }
        if maximum_sample_gap_ns == 0 {
            return Err(EnergyError::ZeroMaximumGap);
        }
        if samples.is_empty() {
            return Err(EnergyError::EmptySamples);
        }
        if samples.len() < 2 {
            return Err(EnergyError::TooFewSamples);
        }

        let mut sample_ids = BTreeMap::<String, ()>::new();
        let mut previous_timestamp = None;
        for sample in &samples {
            if sample_ids.insert(sample.sample_id.clone(), ()).is_some() {
                return Err(EnergyError::DuplicateSampleId(sample.sample_id.clone()));
            }
            if let Some(previous) = previous_timestamp {
                if sample.timestamp_ns <= previous {
                    return Err(EnergyError::NonMonotonicTimestamp(stream_id.clone()));
                }
                let gap_ns = sample.timestamp_ns - previous;
                if gap_ns > maximum_sample_gap_ns {
                    return Err(EnergyError::GapExceedsPolicy {
                        stream_id: stream_id.clone(),
                        gap_ns,
                        maximum_gap_ns: maximum_sample_gap_ns,
                    });
                }
            }
            previous_timestamp = Some(sample.timestamp_ns);
        }

        Ok(Self {
            stream_id,
            run_id,
            boundary,
            sensor_id,
            source_artifact_digest,
            calibration_digest,
            clock_basis_digest,
            maximum_sample_gap_ns,
            samples,
        })
    }

    pub fn stream_id(&self) -> &str {
        &self.stream_id
    }

    pub fn boundary(&self) -> EnergyBoundary {
        self.boundary
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PowerStreamReceipt {
    pub stream_id: String,
    pub boundary: EnergyBoundary,
    pub sensor_id: String,
    pub source_artifact_digest: String,
    pub calibration_digest: String,
    pub clock_basis_digest: String,
    pub configured_maximum_sample_gap_ns: u64,
    pub observed_maximum_sample_gap_ns: u64,
    pub first_timestamp_ns: u64,
    pub last_timestamp_ns: u64,
    pub sample_count: u128,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoundaryEnergyLedger {
    pub boundary: EnergyBoundary,
    pub stream_count: u128,
    pub sample_count: u128,
    /// For BatteryTerminal: discharge energy. For other boundaries: positive
    /// energy into that subsystem.
    pub positive_energy_zeptojoule: ExactRational,
    /// For BatteryTerminal: energy returned to the battery. For JointMechanical:
    /// negative/absorbed mechanical work. It is not cross-boundary regeneration.
    pub negative_energy_magnitude_zeptojoule: ExactRational,
    /// Positive minus negative at this one boundary only.
    pub net_energy_zeptojoule: SignedRational,
}

impl BoundaryEnergyLedger {
    fn empty(boundary: EnergyBoundary) -> Self {
        Self {
            boundary,
            stream_count: 0,
            sample_count: 0,
            positive_energy_zeptojoule: ExactRational::zero(),
            negative_energy_magnitude_zeptojoule: ExactRational::zero(),
            net_energy_zeptojoule: SignedRational {
                negative: false,
                magnitude: ExactRational::zero(),
            },
        }
    }

    fn add_stream(
        &mut self,
        positive: ExactRational,
        negative: ExactRational,
        sample_count: u128,
    ) -> Result<(), EnergyError> {
        self.positive_energy_zeptojoule =
            self.positive_energy_zeptojoule.checked_add(positive)?;
        self.negative_energy_magnitude_zeptojoule =
            self.negative_energy_magnitude_zeptojoule.checked_add(negative)?;
        self.net_energy_zeptojoule = SignedRational::positive_minus(
            self.positive_energy_zeptojoule,
            self.negative_energy_magnitude_zeptojoule,
        )?;
        self.stream_count = checked_add(self.stream_count, 1)?;
        self.sample_count = checked_add(self.sample_count, sample_count)?;
        Ok(())
    }
}

/// Cost of transport, normalized with total configured/measured mass, gravity
/// and measured path distance. All three ratios are exact and dimensionless.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CostOfTransport {
    pub discharge: ExactRational,
    pub returned: ExactRational,
    pub net: SignedRational,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportMetricStatus {
    Available,
    NoProgress,
    MissingBatteryTerminal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransportMetrics {
    pub status: TransportMetricStatus,
    pub cost_of_transport: Option<CostOfTransport>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnergyRun {
    run_id: String,
    robot_digest: String,
    controller_digest: String,
    mass_configuration_digest: String,
    mass_mg: u64,
    gravity_um_s2: u64,
    measured_path_distance_um: u64,
}

impl EnergyRun {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        run_id: String,
        robot_digest: String,
        controller_digest: String,
        mass_configuration_digest: String,
        mass_mg: u64,
        gravity_um_s2: u64,
        measured_path_distance_um: u64,
    ) -> Result<Self, EnergyError> {
        if [
            run_id.as_str(),
            robot_digest.as_str(),
            controller_digest.as_str(),
            mass_configuration_digest.as_str(),
        ]
        .iter()
        .any(|value| !nonblank(value))
        {
            return Err(EnergyError::EmptyIdentity);
        }
        if mass_mg == 0 {
            return Err(EnergyError::InvalidMass);
        }
        if gravity_um_s2 == 0 {
            return Err(EnergyError::InvalidGravity);
        }
        Ok(Self {
            run_id,
            robot_digest,
            controller_digest,
            mass_configuration_digest,
            mass_mg,
            gravity_um_s2,
            measured_path_distance_um,
        })
    }

    /// Integrate every stream independently and aggregate only within identical
    /// boundaries. Battery, bus, actuator, mechanical and auxiliary ledgers are
    /// intentionally not summed into a fictitious total.
    pub fn evaluate(&self, streams: &[PowerStream]) -> Result<EnergyRunReport, EnergyError> {
        if streams.is_empty() {
            return Err(EnergyError::NoStreams);
        }

        let mut stream_ids = BTreeMap::<String, ()>::new();
        let mut ledgers = BTreeMap::<EnergyBoundary, BoundaryEnergyLedger>::new();
        let mut stream_receipts = Vec::with_capacity(streams.len());

        for stream in streams {
            if stream.run_id != self.run_id {
                return Err(EnergyError::StreamRunMismatch(stream.stream_id.clone()));
            }
            if stream_ids.insert(stream.stream_id.clone(), ()).is_some() {
                return Err(EnergyError::DuplicateStreamId(stream.stream_id.clone()));
            }

            let (positive, negative) = integrate_stream(stream)?;
            let ledger = ledgers
                .entry(stream.boundary)
                .or_insert_with(|| BoundaryEnergyLedger::empty(stream.boundary));
            let sample_count = u128::try_from(stream.samples.len())
                .map_err(|_| EnergyError::ArithmeticOverflow)?;
            ledger.add_stream(positive, negative, sample_count)?;
            let observed_maximum_sample_gap_ns = stream
                .samples
                .windows(2)
                .map(|pair| pair[1].timestamp_ns - pair[0].timestamp_ns)
                .max()
                .unwrap_or(0);
            stream_receipts.push(PowerStreamReceipt {
                stream_id: stream.stream_id.clone(),
                boundary: stream.boundary,
                sensor_id: stream.sensor_id.clone(),
                source_artifact_digest: stream.source_artifact_digest.clone(),
                calibration_digest: stream.calibration_digest.clone(),
                clock_basis_digest: stream.clock_basis_digest.clone(),
                configured_maximum_sample_gap_ns: stream.maximum_sample_gap_ns,
                observed_maximum_sample_gap_ns,
                first_timestamp_ns: stream.samples[0].timestamp_ns,
                last_timestamp_ns: stream.samples[stream.samples.len() - 1].timestamp_ns,
                sample_count,
            });
        }

        let boundary_ledgers = ledgers.into_values().collect::<Vec<_>>();
        stream_receipts.sort_by(|left, right| left.stream_id.cmp(&right.stream_id));
        let transport_metrics = self.compute_transport_metrics(&boundary_ledgers)?;

        Ok(EnergyRunReport {
            run_id: self.run_id.clone(),
            robot_digest: self.robot_digest.clone(),
            controller_digest: self.controller_digest.clone(),
            mass_configuration_digest: self.mass_configuration_digest.clone(),
            mass_mg: self.mass_mg,
            gravity_um_s2: self.gravity_um_s2,
            measured_path_distance_um: self.measured_path_distance_um,
            stream_receipts,
            boundary_ledgers,
            transport_metrics,
        })
    }

    fn compute_transport_metrics(
        &self,
        ledgers: &[BoundaryEnergyLedger],
    ) -> Result<TransportMetrics, EnergyError> {
        if self.measured_path_distance_um == 0 {
            return Ok(TransportMetrics {
                status: TransportMetricStatus::NoProgress,
                cost_of_transport: None,
            });
        }
        let Some(battery) = ledgers
            .iter()
            .find(|ledger| ledger.boundary == EnergyBoundary::BatteryTerminal)
        else {
            return Ok(TransportMetrics {
                status: TransportMetricStatus::MissingBatteryTerminal,
                cost_of_transport: None,
            });
        };

        // mass[mg] * gravity[um/s^2] * distance[um] * 1000 gives the
        // normalizing denominator in zJ, because 1 mg*um*um/s^2 = 1e-18 J.
        let denominator_zeptojoule = checked_mul(
            checked_mul(
                checked_mul(
                    u128::from(self.mass_mg),
                    u128::from(self.gravity_um_s2),
                )?,
                u128::from(self.measured_path_distance_um),
            )?,
            1_000,
        )?;

        let discharge = battery
            .positive_energy_zeptojoule
            .divide_by_integer(denominator_zeptojoule)?;
        let returned = battery
            .negative_energy_magnitude_zeptojoule
            .divide_by_integer(denominator_zeptojoule)?;
        let net = battery
            .net_energy_zeptojoule
            .divide_by_integer(denominator_zeptojoule)?;

        Ok(TransportMetrics {
            status: TransportMetricStatus::Available,
            cost_of_transport: Some(CostOfTransport {
                discharge,
                returned,
                net,
            }),
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnergyRunReport {
    pub run_id: String,
    pub robot_digest: String,
    pub controller_digest: String,
    pub mass_configuration_digest: String,
    pub mass_mg: u64,
    pub gravity_um_s2: u64,
    pub measured_path_distance_um: u64,
    /// Sorted by stream ID and bound to source, calibration and clock identity.
    pub stream_receipts: Vec<PowerStreamReceipt>,
    /// Sorted by boundary. No aggregate across nested boundaries is provided.
    pub boundary_ledgers: Vec<BoundaryEnergyLedger>,
    pub transport_metrics: TransportMetrics,
}

impl EnergyRunReport {
    pub fn boundary(&self, boundary: EnergyBoundary) -> Option<&BoundaryEnergyLedger> {
        self.boundary_ledgers
            .iter()
            .find(|ledger| ledger.boundary == boundary)
    }
}

/// Integrate a piecewise-linear signed-power signal by exact trapezoids.
/// Positive and negative lobes are split analytically when a segment crosses
/// zero; no negative samples are silently clipped.
fn integrate_stream(
    stream: &PowerStream,
) -> Result<(ExactRational, ExactRational), EnergyError> {
    let mut positive = ExactRational::zero();
    let mut negative = ExactRational::zero();

    for pair in stream.samples.windows(2) {
        let left = &pair[0];
        let right = &pair[1];
        let duration_ns = right
            .timestamp_ns
            .checked_sub(left.timestamp_ns)
            .ok_or_else(|| EnergyError::NonMonotonicTimestamp(stream.stream_id.clone()))?;
        if duration_ns == 0 {
            return Err(EnergyError::NonMonotonicTimestamp(
                stream.stream_id.clone(),
            ));
        }
        if duration_ns > stream.maximum_sample_gap_ns {
            return Err(EnergyError::GapExceedsPolicy {
                stream_id: stream.stream_id.clone(),
                gap_ns: duration_ns,
                maximum_gap_ns: stream.maximum_sample_gap_ns,
            });
        }

        let (positive_segment, negative_segment) = integrate_segment(
            left.signed_power_pw,
            right.signed_power_pw,
            u128::from(duration_ns),
        )?;
        positive = positive.checked_add(positive_segment)?;
        negative = negative.checked_add(negative_segment)?;
    }
    Ok((positive, negative))
}

fn integrate_segment(
    left_power_pw: i128,
    right_power_pw: i128,
    duration_ns: u128,
) -> Result<(ExactRational, ExactRational), EnergyError> {
    let left_positive = left_power_pw >= 0;
    let right_positive = right_power_pw >= 0;
    let left_magnitude = left_power_pw.unsigned_abs();
    let right_magnitude = right_power_pw.unsigned_abs();

    if left_positive && right_positive {
        let sum = checked_add(left_magnitude, right_magnitude)?;
        let numerator = checked_mul(sum, duration_ns)?;
        return Ok((
            ExactRational::new(numerator, 2)?,
            ExactRational::zero(),
        ));
    }

    if !left_positive && !right_positive {
        let sum = checked_add(left_magnitude, right_magnitude)?;
        let numerator = checked_mul(sum, duration_ns)?;
        return Ok((
            ExactRational::zero(),
            ExactRational::new(numerator, 2)?,
        ));
    }

    // A strict sign change produces one positive and one negative triangle.
    // The zero endpoint is included in one of the same-sign cases above.
    let (positive_power, negative_power) = if left_positive {
        (left_magnitude, right_magnitude)
    } else {
        (right_magnitude, left_magnitude)
    };
    let sum = checked_add(positive_power, negative_power)?;
    let denominator = checked_mul(2, sum)?;
    let positive_numerator = checked_mul(
        checked_mul(positive_power, positive_power)?,
        duration_ns,
    )?;
    let negative_numerator = checked_mul(
        checked_mul(negative_power, negative_power)?,
        duration_ns,
    )?;

    Ok((
        ExactRational::new(positive_numerator, denominator)?,
        ExactRational::new(negative_numerator, denominator)?,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(id: &str, time: u64, power_pw: i128) -> PowerSample {
        PowerSample::new(id.to_owned(), time, power_pw).unwrap()
    }

    fn stream(
        id: &str,
        boundary: EnergyBoundary,
        samples: Vec<PowerSample>,
        maximum_gap_ns: u64,
    ) -> PowerStream {
        PowerStream::new(
            id.to_owned(),
            "run-01".to_owned(),
            boundary,
            format!("sensor-{id}"),
            format!("source-{id}"),
            format!("calibration-{id}"),
            "clock-monotonic-01".to_owned(),
            maximum_gap_ns,
            samples,
        )
        .unwrap()
    }

    fn run(distance_um: u64) -> EnergyRun {
        EnergyRun::new(
            "run-01".to_owned(),
            "robot-digest".to_owned(),
            "controller-digest".to_owned(),
            "mass-config".to_owned(),
            1_000_000, // 1 kg
            9_806_650, // standard gravity in um/s^2
            distance_um,
        )
        .unwrap()
    }

    #[test]
    fn constant_positive_power_integrates_exactly() {
        let input = stream(
            "battery",
            EnergyBoundary::BatteryTerminal,
            vec![sample("a", 0, 1_000_000), sample("b", 1_000_000_000, 1_000_000)],
            1_000_000_000,
        );
        let report = run(1_000_000).evaluate(&[input]).unwrap();
        let ledger = report.boundary(EnergyBoundary::BatteryTerminal).unwrap();
        assert_eq!(
            ledger.positive_energy_zeptojoule,
            ExactRational::new(1_000_000_000_000_000, 1).unwrap()
        );
        assert_eq!(
            ledger.negative_energy_magnitude_zeptojoule,
            ExactRational::zero()
        );
        assert_eq!(ledger.net_energy_zeptojoule.magnitude, ledger.positive_energy_zeptojoule);
    }

    #[test]
    fn constant_negative_power_is_reported_as_returned_energy_not_clipped() {
        let input = stream(
            "battery",
            EnergyBoundary::BatteryTerminal,
            vec![sample("a", 0, -2_000), sample("b", 100, -2_000)],
            100,
        );
        let report = run(1).evaluate(&[input]).unwrap();
        let ledger = report.boundary(EnergyBoundary::BatteryTerminal).unwrap();
        assert_eq!(ledger.positive_energy_zeptojoule, ExactRational::zero());
        assert_eq!(
            ledger.negative_energy_magnitude_zeptojoule,
            ExactRational::new(200_000, 1).unwrap()
        );
        assert!(ledger.net_energy_zeptojoule.is_negative());
    }

    #[test]
    fn sign_crossing_splits_positive_and_negative_triangles_exactly() {
        let input = stream(
            "battery",
            EnergyBoundary::BatteryTerminal,
            vec![sample("a", 0, 1_000), sample("b", 10, -1_000)],
            10,
        );
        let report = run(1).evaluate(&[input]).unwrap();
        let ledger = report.boundary(EnergyBoundary::BatteryTerminal).unwrap();
        assert_eq!(
            ledger.positive_energy_zeptojoule,
            ExactRational::new(2_500, 1).unwrap()
        );
        assert_eq!(
            ledger.negative_energy_magnitude_zeptojoule,
            ExactRational::new(2_500, 1).unwrap()
        );
        assert_eq!(
            ledger.net_energy_zeptojoule,
            SignedRational {
                negative: false,
                magnitude: ExactRational::zero()
            }
        );
    }

    #[test]
    fn nonuniform_timestamps_use_each_segment_duration() {
        let input = stream(
            "battery",
            EnergyBoundary::BatteryTerminal,
            vec![
                sample("a", 0, 100),
                sample("b", 10, 100),
                sample("c", 30, 300),
            ],
            20,
        );
        let report = run(1).evaluate(&[input]).unwrap();
        let ledger = report.boundary(EnergyBoundary::BatteryTerminal).unwrap();
        // Segment 0..10: 100*10 = 1,000 zJ.
        // Segment 10..30: average power 200 pW for 20 ns = 4,000 zJ.
        assert_eq!(
            ledger.positive_energy_zeptojoule,
            ExactRational::new(5_000, 1).unwrap()
        );
    }

    #[test]
    fn battery_voltage_current_units_convert_to_picowatts_exactly() {
        let sample = PowerSample::from_battery_vi("draw".to_owned(), 0, 24_000_000, 2_000_000)
            .unwrap();
        assert_eq!(sample.signed_power_pw, 48_000_000_000_000);
        let returned =
            PowerSample::from_battery_vi("return".to_owned(), 1, 24_000_000, -100_000)
                .unwrap();
        assert_eq!(returned.signed_power_pw, -2_400_000_000_000);
    }

    #[test]
    fn joint_torque_velocity_is_mechanical_not_electrical_energy() {
        let positive = PowerSample::from_joint_torque_velocity(
            "joint-positive".to_owned(),
            0,
            2_000_000,
            3_000_000,
        )
        .unwrap();
        let negative = PowerSample::from_joint_torque_velocity(
            "joint-negative".to_owned(),
            1,
            -2_000_000,
            3_000_000,
        )
        .unwrap();
        assert_eq!(positive.signed_power_pw, 6_000_000_000_000);
        assert_eq!(negative.signed_power_pw, -6_000_000_000_000);
    }

    #[test]
    fn battery_bus_and_joint_boundaries_are_kept_separate() {
        let battery = stream(
            "battery",
            EnergyBoundary::BatteryTerminal,
            vec![sample("b0", 0, 1_000), sample("b1", 10, 1_000)],
            10,
        );
        let bus = stream(
            "bus",
            EnergyBoundary::DcBus,
            vec![sample("d0", 0, 800), sample("d1", 10, 800)],
            10,
        );
        let joint = stream(
            "joint-1",
            EnergyBoundary::JointMechanical,
            vec![sample("j0", 0, 500), sample("j1", 10, 500)],
            10,
        );
        let report = run(100).evaluate(&[battery, bus, joint]).unwrap();

        assert_eq!(
            report
                .boundary(EnergyBoundary::BatteryTerminal)
                .unwrap()
                .positive_energy_zeptojoule,
            ExactRational::new(10_000, 1).unwrap()
        );
        assert_eq!(
            report
                .boundary(EnergyBoundary::DcBus)
                .unwrap()
                .positive_energy_zeptojoule,
            ExactRational::new(8_000, 1).unwrap()
        );
        assert_eq!(
            report
                .boundary(EnergyBoundary::JointMechanical)
                .unwrap()
                .positive_energy_zeptojoule,
            ExactRational::new(5_000, 1).unwrap()
        );
        assert_eq!(report.boundary_ledgers.len(), 3);
        // There is deliberately no API that adds these nested measurements.
    }

    #[test]
    fn cost_of_transport_is_exact_for_one_kg_over_ten_metres() {
        let battery = stream(
            "battery",
            EnergyBoundary::BatteryTerminal,
            vec![
                sample("a", 0, 9_806_650_000_000_000_000_000),
                sample("b", 10, 9_806_650_000_000_000_000_000),
            ],
            10,
        );
        let report = run(10_000_000).evaluate(&[battery]).unwrap();
        assert_eq!(
            report.transport_metrics.status,
            TransportMetricStatus::Available
        );
        let cot = report.transport_metrics.cost_of_transport.unwrap();
        assert_eq!(cot.discharge, ExactRational::new(1, 1).unwrap());
        assert_eq!(cot.returned, ExactRational::zero());
        assert_eq!(
            cot.net.magnitude,
            ExactRational::new(1, 1).unwrap()
        );
    }

    #[test]
    fn zero_progress_preserves_energy_and_has_no_finite_cot() {
        let battery = stream(
            "battery",
            EnergyBoundary::BatteryTerminal,
            vec![sample("a", 0, 1_000), sample("b", 10, 1_000)],
            10,
        );
        let report = run(0).evaluate(&[battery]).unwrap();
        assert_eq!(
            report.transport_metrics.status,
            TransportMetricStatus::NoProgress
        );
        assert_eq!(report.transport_metrics.cost_of_transport, None);
        assert_eq!(
            report
                .boundary(EnergyBoundary::BatteryTerminal)
                .unwrap()
                .positive_energy_zeptojoule,
            ExactRational::new(10_000, 1).unwrap()
        );
    }

    #[test]
    fn missing_battery_stream_does_not_mislabel_mechanical_cot() {
        let joint = stream(
            "joint",
            EnergyBoundary::JointMechanical,
            vec![sample("a", 0, 500), sample("b", 10, 500)],
            10,
        );
        let report = run(1_000_000).evaluate(&[joint]).unwrap();
        assert_eq!(
            report.transport_metrics.status,
            TransportMetricStatus::MissingBatteryTerminal
        );
        assert_eq!(report.transport_metrics.cost_of_transport, None);
    }

    #[test]
    fn duplicate_samples_and_streams_are_rejected() {
        let duplicate_sample = PowerStream::new(
            "battery".to_owned(),
            "run-01".to_owned(),
            EnergyBoundary::BatteryTerminal,
            "sensor".to_owned(),
            "source".to_owned(),
            "calibration".to_owned(),
            "clock".to_owned(),
            10,
            vec![sample("same", 0, 1), sample("same", 10, 1)],
        );
        assert_eq!(
            duplicate_sample,
            Err(EnergyError::DuplicateSampleId("same".to_owned()))
        );

        let input = stream(
            "battery",
            EnergyBoundary::BatteryTerminal,
            vec![sample("a", 0, 1), sample("b", 10, 1)],
            10,
        );
        assert_eq!(
            run(1).evaluate(&[input.clone(), input]),
            Err(EnergyError::DuplicateStreamId("battery".to_owned()))
        );
    }

    #[test]
    fn reversed_or_duplicate_timestamps_are_rejected() {
        let result = PowerStream::new(
            "battery".to_owned(),
            "run-01".to_owned(),
            EnergyBoundary::BatteryTerminal,
            "sensor".to_owned(),
            "source".to_owned(),
            "calibration".to_owned(),
            "clock".to_owned(),
            10,
            vec![sample("a", 10, 1), sample("b", 10, 1)],
        );
        assert_eq!(
            result,
            Err(EnergyError::NonMonotonicTimestamp("battery".to_owned()))
        );
    }

    #[test]
    fn sample_gaps_beyond_declared_policy_are_rejected() {
        let result = PowerStream::new(
            "battery".to_owned(),
            "run-01".to_owned(),
            EnergyBoundary::BatteryTerminal,
            "sensor".to_owned(),
            "source".to_owned(),
            "calibration".to_owned(),
            "clock".to_owned(),
            9,
            vec![sample("a", 0, 1), sample("b", 10, 1)],
        );
        assert_eq!(
            result,
            Err(EnergyError::GapExceedsPolicy {
                stream_id: "battery".to_owned(),
                gap_ns: 10,
                maximum_gap_ns: 9,
            })
        );
    }

    #[test]
    fn stream_run_identity_must_match_energy_run() {
        let mismatched = PowerStream::new(
            "battery".to_owned(),
            "other-run".to_owned(),
            EnergyBoundary::BatteryTerminal,
            "sensor".to_owned(),
            "source".to_owned(),
            "calibration".to_owned(),
            "clock".to_owned(),
            10,
            vec![sample("a", 0, 1), sample("b", 10, 1)],
        )
        .unwrap();
        assert_eq!(
            run(1).evaluate(&[mismatched]),
            Err(EnergyError::StreamRunMismatch("battery".to_owned()))
        );
    }

    #[test]
    fn empty_identity_mass_gravity_and_ratio_denominator_fail_closed() {
        assert_eq!(
            EnergyRun::new(
                " ".to_owned(),
                "robot".to_owned(),
                "controller".to_owned(),
                "mass".to_owned(),
                1,
                1,
                1,
            ),
            Err(EnergyError::EmptyIdentity)
        );
        assert_eq!(
            EnergyRun::new(
                "run".to_owned(),
                "robot".to_owned(),
                "controller".to_owned(),
                "mass".to_owned(),
                0,
                1,
                1,
            ),
            Err(EnergyError::InvalidMass)
        );
        assert_eq!(
            EnergyRun::new(
                "run".to_owned(),
                "robot".to_owned(),
                "controller".to_owned(),
                "mass".to_owned(),
                1,
                0,
                1,
            ),
            Err(EnergyError::InvalidGravity)
        );
        assert_eq!(
            ExactRational::new(1, 0),
            Err(EnergyError::InvalidRatioDenominator)
        );
    }

    #[test]
    fn arithmetic_overflow_is_rejected() {
        assert_eq!(checked_add(u128::MAX, 1), Err(EnergyError::ArithmeticOverflow));
        assert_eq!(checked_mul(u128::MAX, 2), Err(EnergyError::ArithmeticOverflow));
        assert_eq!(
            ExactRational::new(u128::MAX, 1).unwrap().checked_add(
                ExactRational::new(1, 1).unwrap()
            ),
            Err(EnergyError::ArithmeticOverflow)
        );
    }

    #[test]
    fn rational_addition_reduces_exactly() {
        let a = ExactRational::new(1, 3).unwrap();
        let b = ExactRational::new(1, 6).unwrap();
        assert_eq!(a.checked_add(b).unwrap(), ExactRational::new(1, 2).unwrap());
    }

    #[test]
    fn source_order_does_not_change_the_aggregated_energy() {
        let a = stream(
            "battery-a",
            EnergyBoundary::BatteryTerminal,
            vec![sample("a0", 0, 100), sample("a1", 10, 100)],
            10,
        );
        let b = stream(
            "battery-b",
            EnergyBoundary::BatteryTerminal,
            vec![sample("b0", 0, 200), sample("b1", 10, 200)],
            10,
        );
        let first = run(1_000_000).evaluate(&[a.clone(), b.clone()]).unwrap();
        let second = run(1_000_000).evaluate(&[b, a]).unwrap();
        assert_eq!(first.boundary_ledgers, second.boundary_ledgers);
        assert_eq!(
            first.transport_metrics,
            second.transport_metrics
        );
    }
}
