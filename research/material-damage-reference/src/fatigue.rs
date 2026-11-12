//! Stress-life interpolation and linear Miner damage accumulation.
//!
//! The caller supplies an already-counted load spectrum. This module does not
//! perform rainflow counting, mean-stress correction, sequence interaction,
//! or material calibration.

use crate::{DamageError, DamageResult, require_positive_finite};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SnPoint {
    /// Stress amplitude in MPa.
    pub stress_amplitude_mpa: f64,
    /// Cycles to failure for this material/test condition.
    pub cycles_to_failure: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SnCurve {
    points: Vec<SnPoint>,
}

impl SnCurve {
    /// Construct an S-N curve ordered by increasing life and decreasing stress.
    ///
    /// At least two points are required. The model performs log-log
    /// interpolation only; extrapolation outside the supplied range is rejected.
    pub fn new(points: Vec<SnPoint>) -> DamageResult<Self> {
        if points.len() < 2 {
            return Err(DamageError::InsufficientSnPoints {
                count: points.len(),
            });
        }

        for point in &points {
            require_positive_finite(point.stress_amplitude_mpa, "S-N stress amplitude")?;
            if point.cycles_to_failure == 0 {
                return Err(DamageError::InvalidValue {
                    field: "S-N cycles to failure",
                });
            }
        }

        for pair in points.windows(2) {
            if pair[1].cycles_to_failure <= pair[0].cycles_to_failure
                || pair[1].stress_amplitude_mpa >= pair[0].stress_amplitude_mpa
            {
                return Err(DamageError::InvalidSnOrdering);
            }
        }

        Ok(Self { points })
    }

    pub fn points(&self) -> &[SnPoint] {
        &self.points
    }

    pub fn minimum_stress_amplitude_mpa(&self) -> f64 {
        self.points[self.points.len() - 1].stress_amplitude_mpa
    }

    pub fn maximum_stress_amplitude_mpa(&self) -> f64 {
        self.points[0].stress_amplitude_mpa
    }

    /// Estimate cycles to failure by piecewise log-log interpolation.
    ///
    /// The query must lie within the measured/calibrated stress range. The
    /// returned value is an interpolated model estimate, not a statistical
    /// lower bound or a guaranteed service life.
    pub fn cycles_to_failure(&self, stress_amplitude_mpa: f64) -> DamageResult<f64> {
        require_positive_finite(stress_amplitude_mpa, "query stress amplitude")?;

        let maximum = self.maximum_stress_amplitude_mpa();
        let minimum = self.minimum_stress_amplitude_mpa();
        if stress_amplitude_mpa > maximum || stress_amplitude_mpa < minimum {
            return Err(DamageError::StressOutsideSnRange {
                stress_amplitude_mpa,
                minimum_mpa: minimum,
                maximum_mpa: maximum,
            });
        }

        for point in &self.points {
            if stress_amplitude_mpa == point.stress_amplitude_mpa {
                return Ok(point.cycles_to_failure as f64);
            }
        }

        let pair = self
            .points
            .windows(2)
            .find(|pair| {
                stress_amplitude_mpa <= pair[0].stress_amplitude_mpa
                    && stress_amplitude_mpa >= pair[1].stress_amplitude_mpa
            })
            .ok_or(DamageError::ArithmeticOverflow {
                field: "S-N interpolation bracket",
            })?;

        let ln_stress_a = pair[0].stress_amplitude_mpa.ln();
        let ln_stress_b = pair[1].stress_amplitude_mpa.ln();
        let ln_query = stress_amplitude_mpa.ln();
        let fraction = (ln_query - ln_stress_a) / (ln_stress_b - ln_stress_a);

        let ln_cycles_a = (pair[0].cycles_to_failure as f64).ln();
        let ln_cycles_b = (pair[1].cycles_to_failure as f64).ln();
        let cycles = (ln_cycles_a + fraction * (ln_cycles_b - ln_cycles_a)).exp();

        if !cycles.is_finite() || cycles <= 0.0 {
            return Err(DamageError::ArithmeticOverflow {
                field: "S-N interpolated cycles",
            });
        }
        Ok(cycles)
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LoadBlock {
    /// Stress amplitude in MPa, already corrected for the chosen stress measure.
    pub stress_amplitude_mpa: f64,
    /// Counted cycles represented by this block.
    pub cycles: u64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BlockDamage {
    pub block_index: usize,
    pub stress_amplitude_mpa: f64,
    pub cycles: u64,
    pub interpolated_cycles_to_failure: f64,
    pub damage_increment: f64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MinerAssessment {
    BelowUnity,
    AtOrAboveUnity,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MinerDamageReport {
    /// Sum of n_i / N_i under the linear Miner rule.
    pub damage_fraction: f64,
    pub total_cycles: u64,
    pub assessment: MinerAssessment,
    pub blocks: Vec<BlockDamage>,
}

/// Accumulate linear Miner damage over caller-supplied, already-counted blocks.
///
/// AtOrAboveUnity means only that the declared Miner model has reached its
/// conventional unit-damage threshold. It is not proof that a real component
/// has failed, and BelowUnity is not a safety certification.
pub fn miner_damage(curve: &SnCurve, load_blocks: &[LoadBlock]) -> DamageResult<MinerDamageReport> {
    if load_blocks.is_empty() {
        return Err(DamageError::EmptyLoadSpectrum);
    }

    let mut damage_fraction = 0.0_f64;
    let mut total_cycles = 0_u64;
    let mut results = Vec::with_capacity(load_blocks.len());

    for (block_index, block) in load_blocks.iter().enumerate() {
        if block.cycles == 0 {
            return Err(DamageError::ZeroCycleCount);
        }
        let cycles_to_failure = curve.cycles_to_failure(block.stress_amplitude_mpa)?;
        let damage_increment = block.cycles as f64 / cycles_to_failure;
        damage_fraction += damage_increment;
        total_cycles = total_cycles
            .checked_add(block.cycles)
            .ok_or(DamageError::ArithmeticOverflow {
                field: "total cycle count",
            })?;

        if !damage_increment.is_finite() || !damage_fraction.is_finite() {
            return Err(DamageError::ArithmeticOverflow {
                field: "Miner damage fraction",
            });
        }

        results.push(BlockDamage {
            block_index,
            stress_amplitude_mpa: block.stress_amplitude_mpa,
            cycles: block.cycles,
            interpolated_cycles_to_failure: cycles_to_failure,
            damage_increment,
        });
    }

    Ok(MinerDamageReport {
        damage_fraction,
        total_cycles,
        assessment: if damage_fraction >= 1.0 {
            MinerAssessment::AtOrAboveUnity
        } else {
            MinerAssessment::BelowUnity
        },
        blocks: results,
    })
}
