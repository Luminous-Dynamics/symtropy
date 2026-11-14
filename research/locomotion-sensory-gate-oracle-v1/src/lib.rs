// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Known-answer oracle for modality- and state-dependent proprioceptive gating.
//!
//! This is a synthetic factorial mechanism-signature test. It is not a neural,
//! calcium-imaging, biomechanics or statistical-power simulator.

#![forbid(unsafe_code)]
#![allow(clippy::needless_range_loop)]

const FACTORIAL_TERMS: usize = 8;
const RANK_TOLERANCE: f64 = 1.0e-10;

/// Proprioceptive channel represented by one factorial condition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SensorModality {
    /// Position-encoding proprioceptive feedback.
    Position,
    /// Movement-encoding proprioceptive feedback.
    Movement,
}

/// Whether movement is self-generated or externally imposed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum MovementContext {
    /// Passive externally imposed movement.
    Passive,
    /// Self-generated movement during a motor behaviour.
    SelfGenerated,
}

/// Whether the presynaptic inhibitory gate is blocked or intact.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PresynapticGate {
    /// Gate is blocked or selectively silenced.
    Blocked,
    /// Gate is intact and available to suppress the movement-encoding channel.
    Intact,
}

/// One condition in the 2×2×2 modality × context × gate design.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SensoryGateCondition {
    pub modality: SensorModality,
    pub context: MovementContext,
    pub gate: PresynapticGate,
}

impl SensoryGateCondition {
    /// Creates one fully specified condition.
    pub const fn new(
        modality: SensorModality,
        context: MovementContext,
        gate: PresynapticGate,
    ) -> Self {
        Self {
            modality,
            context,
            gate,
        }
    }

    /// Returns all eight conditions in deterministic order.
    #[must_use]
    pub fn full_factorial() -> Vec<Self> {
        let mut cells = Vec::with_capacity(FACTORIAL_TERMS);
        for modality in [SensorModality::Position, SensorModality::Movement] {
            for context in [MovementContext::Passive, MovementContext::SelfGenerated] {
                for gate in [PresynapticGate::Blocked, PresynapticGate::Intact] {
                    cells.push(Self::new(modality, context, gate));
                }
            }
        }
        cells
    }

    /// Returns the intercept, main-effect, pairwise and three-way design row.
    ///
    /// Position/passive/blocked are coded -1; movement/self-generated/intact +1.
    #[must_use]
    pub fn design_row(self) -> [f64; FACTORIAL_TERMS] {
        let modality = if self.modality == SensorModality::Movement {
            1.0
        } else {
            -1.0
        };
        let context = if self.context == MovementContext::SelfGenerated {
            1.0
        } else {
            -1.0
        };
        let gate = if self.gate == PresynapticGate::Intact {
            1.0
        } else {
            -1.0
        };

        [
            1.0,
            modality,
            context,
            gate,
            modality * context,
            modality * gate,
            context * gate,
            modality * context * gate,
        ]
    }
}

/// One cell-level, source-qualified activity estimate.
///
/// Individual/session-level observations must be retained and analyzed before
/// reduction to this one value per factorial condition.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SensoryGateObservation {
    pub condition: SensoryGateCondition,
    pub mean_activity: f64,
}

/// Coefficients of the complete 2×2×2 response surface.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SensoryGateCoefficients {
    pub intercept: f64,
    pub movement_modality: f64,
    pub self_generated_context: f64,
    pub intact_gate: f64,
    pub modality_by_context: f64,
    pub modality_by_gate: f64,
    pub context_by_gate: f64,
    pub three_way: f64,
}

impl SensoryGateCoefficients {
    /// Returns coefficients in the declared design-row order.
    #[must_use]
    pub const fn as_array(self) -> [f64; FACTORIAL_TERMS] {
        [
            self.intercept,
            self.movement_modality,
            self.self_generated_context,
            self.intact_gate,
            self.modality_by_context,
            self.modality_by_gate,
            self.context_by_gate,
            self.three_way,
        ]
    }

    /// Predicts one condition's mean activity.
    #[must_use]
    pub fn predict(self, condition: SensoryGateCondition) -> f64 {
        let row = condition.design_row();
        let coefficients = self.as_array();
        let mut output = 0.0;
        for index in 0..FACTORIAL_TERMS {
            output += coefficients[index] * row[index];
        }
        output
    }
}

/// Algebraic rank diagnostics for the supplied factorial design.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DesignDiagnostics {
    pub observed_conditions: usize,
    pub missing_conditions: usize,
    pub matrix_rank: usize,
    pub all_terms_identifiable: bool,
}

/// Explicit validation and identifiability failures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SensoryGateError {
    EmptyDesign,
    TooManyConditions,
    DuplicateCondition,
    NonFiniteActivity,
    NonIdentifiable { rank: usize },
    InvalidThresholds,
}

impl std::fmt::Display for SensoryGateError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyDesign => write!(formatter, "sensory-gate design is empty"),
            Self::TooManyConditions => {
                write!(formatter, "more than eight factorial conditions supplied")
            }
            Self::DuplicateCondition => write!(formatter, "duplicate sensory-gate condition"),
            Self::NonFiniteActivity => {
                write!(formatter, "sensory activity contains NaN or infinity")
            }
            Self::NonIdentifiable { rank } => write!(
                formatter,
                "full sensory-gate response surface is not identifiable (rank {rank}/8)"
            ),
            Self::InvalidThresholds => write!(
                formatter,
                "suppression and non-effect thresholds must be finite, positive and ordered"
            ),
        }
    }
}

impl std::error::Error for SensoryGateError {}

/// Validates observations and reports design-matrix rank.
pub fn diagnose_design(
    observations: &[SensoryGateObservation],
) -> Result<DesignDiagnostics, SensoryGateError> {
    if observations.is_empty() {
        return Err(SensoryGateError::EmptyDesign);
    }
    if observations.len() > FACTORIAL_TERMS {
        return Err(SensoryGateError::TooManyConditions);
    }

    let mut conditions = Vec::with_capacity(observations.len());
    let mut matrix = Vec::with_capacity(observations.len());
    for observation in observations {
        if !observation.mean_activity.is_finite() {
            return Err(SensoryGateError::NonFiniteActivity);
        }
        if conditions.contains(&observation.condition) {
            return Err(SensoryGateError::DuplicateCondition);
        }
        conditions.push(observation.condition);
        matrix.push(observation.condition.design_row());
    }

    let matrix_rank = numerical_rank(matrix);
    Ok(DesignDiagnostics {
        observed_conditions: conditions.len(),
        missing_conditions: FACTORIAL_TERMS - conditions.len(),
        matrix_rank,
        all_terms_identifiable: matrix_rank == FACTORIAL_TERMS,
    })
}

/// Recovers coefficients only when all eight conditions are present and full-rank.
pub fn recover_full_factorial(
    observations: &[SensoryGateObservation],
) -> Result<SensoryGateCoefficients, SensoryGateError> {
    let diagnostics = diagnose_design(observations)?;
    if !diagnostics.all_terms_identifiable {
        return Err(SensoryGateError::NonIdentifiable {
            rank: diagnostics.matrix_rank,
        });
    }

    let mut coefficients = [0.0_f64; FACTORIAL_TERMS];
    for observation in observations {
        let row = observation.condition.design_row();
        for index in 0..FACTORIAL_TERMS {
            coefficients[index] += observation.mean_activity * row[index];
        }
    }
    for coefficient in &mut coefficients {
        *coefficient /= FACTORIAL_TERMS as f64;
    }

    Ok(SensoryGateCoefficients {
        intercept: coefficients[0],
        movement_modality: coefficients[1],
        self_generated_context: coefficients[2],
        intact_gate: coefficients[3],
        modality_by_context: coefficients[4],
        modality_by_gate: coefficients[5],
        context_by_gate: coefficients[6],
        three_way: coefficients[7],
    })
}

/// Preregistered response thresholds in the declared activity unit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SensoryGateThresholds {
    /// Minimum active movement-channel suppression when the gate is intact.
    pub minimum_movement_suppression: f64,
    /// Maximum acceptable change in a channel expected not to be gated.
    pub maximum_non_effect: f64,
}

/// Separate, interpretable falsification checks for the proposed gate.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SensoryGateSignature {
    /// Position signal is stable between passive and self-generated movement.
    pub position_channel_context_invariant: bool,
    /// Position signal is stable when the presynaptic gate is blocked/intact.
    pub position_channel_gate_invariant: bool,
    /// Movement signal is suppressed during active movement with the gate intact.
    pub movement_channel_suppressed_when_active_and_gated: bool,
    /// Blocking the gate removes active movement-channel suppression.
    pub movement_suppression_requires_gate: bool,
    /// Passive movement does not show the same movement-channel suppression.
    pub passive_movement_negative_control_passes: bool,
}

impl SensoryGateSignature {
    /// True only when every modality/context/gate prediction passes.
    #[must_use]
    pub const fn qualified(self) -> bool {
        self.position_channel_context_invariant
            && self.position_channel_gate_invariant
            && self.movement_channel_suppressed_when_active_and_gated
            && self.movement_suppression_requires_gate
            && self.passive_movement_negative_control_passes
    }
}

fn value(
    observations: &[SensoryGateObservation],
    condition: SensoryGateCondition,
) -> Result<f64, SensoryGateError> {
    observations
        .iter()
        .find(|observation| observation.condition == condition)
        .map(|observation| observation.mean_activity)
        .ok_or(SensoryGateError::NonIdentifiable {
            rank: observations.len(),
        })
}

/// Tests the selective presynaptic-gating signature on a complete design.
///
/// Thresholds must be fixed before inspecting candidate outcomes. This is a
/// mechanism-pattern check, not a statistical significance or equivalence test.
pub fn classify_sensory_gate_signature(
    observations: &[SensoryGateObservation],
    thresholds: SensoryGateThresholds,
) -> Result<SensoryGateSignature, SensoryGateError> {
    if !thresholds.minimum_movement_suppression.is_finite()
        || !thresholds.maximum_non_effect.is_finite()
        || thresholds.maximum_non_effect <= 0.0
        || thresholds.minimum_movement_suppression <= thresholds.maximum_non_effect
    {
        return Err(SensoryGateError::InvalidThresholds);
    }
    recover_full_factorial(observations)?;

    let position_passive_blocked = value(
        observations,
        SensoryGateCondition::new(
            SensorModality::Position,
            MovementContext::Passive,
            PresynapticGate::Blocked,
        ),
    )?;
    let position_active_blocked = value(
        observations,
        SensoryGateCondition::new(
            SensorModality::Position,
            MovementContext::SelfGenerated,
            PresynapticGate::Blocked,
        ),
    )?;
    let position_passive_intact = value(
        observations,
        SensoryGateCondition::new(
            SensorModality::Position,
            MovementContext::Passive,
            PresynapticGate::Intact,
        ),
    )?;
    let position_active_intact = value(
        observations,
        SensoryGateCondition::new(
            SensorModality::Position,
            MovementContext::SelfGenerated,
            PresynapticGate::Intact,
        ),
    )?;

    let movement_passive_blocked = value(
        observations,
        SensoryGateCondition::new(
            SensorModality::Movement,
            MovementContext::Passive,
            PresynapticGate::Blocked,
        ),
    )?;
    let movement_active_blocked = value(
        observations,
        SensoryGateCondition::new(
            SensorModality::Movement,
            MovementContext::SelfGenerated,
            PresynapticGate::Blocked,
        ),
    )?;
    let movement_passive_intact = value(
        observations,
        SensoryGateCondition::new(
            SensorModality::Movement,
            MovementContext::Passive,
            PresynapticGate::Intact,
        ),
    )?;
    let movement_active_intact = value(
        observations,
        SensoryGateCondition::new(
            SensorModality::Movement,
            MovementContext::SelfGenerated,
            PresynapticGate::Intact,
        ),
    )?;

    let position_channel_context_invariant =
        (position_active_intact - position_passive_intact).abs() <= thresholds.maximum_non_effect
            && (position_active_blocked - position_passive_blocked).abs()
                <= thresholds.maximum_non_effect;

    let position_channel_gate_invariant = (position_active_intact - position_active_blocked).abs()
        <= thresholds.maximum_non_effect
        && (position_passive_intact - position_passive_blocked).abs()
            <= thresholds.maximum_non_effect;

    let movement_suppression = movement_passive_intact - movement_active_intact;
    let movement_channel_suppressed_when_active_and_gated =
        movement_suppression >= thresholds.minimum_movement_suppression;

    let blocked_suppression = movement_passive_blocked - movement_active_blocked;
    let movement_suppression_requires_gate = blocked_suppression.abs()
        <= thresholds.maximum_non_effect
        && movement_suppression - blocked_suppression >= thresholds.minimum_movement_suppression;

    let passive_movement_negative_control_passes =
        (movement_passive_intact - movement_passive_blocked).abs() <= thresholds.maximum_non_effect;

    Ok(SensoryGateSignature {
        position_channel_context_invariant,
        position_channel_gate_invariant,
        movement_channel_suppressed_when_active_and_gated,
        movement_suppression_requires_gate,
        passive_movement_negative_control_passes,
    })
}

fn numerical_rank(mut matrix: Vec<[f64; FACTORIAL_TERMS]>) -> usize {
    let row_count = matrix.len();
    let mut rank = 0;

    for column in 0..FACTORIAL_TERMS {
        if rank >= row_count {
            break;
        }
        let pivot = (rank..row_count).max_by(|left, right| {
            matrix[*left][column]
                .abs()
                .total_cmp(&matrix[*right][column].abs())
        });
        let Some(pivot_row) = pivot else {
            continue;
        };
        if matrix[pivot_row][column].abs() <= RANK_TOLERANCE {
            continue;
        }

        matrix.swap(rank, pivot_row);
        let pivot_value = matrix[rank][column];
        for entry in column..FACTORIAL_TERMS {
            matrix[rank][entry] /= pivot_value;
        }

        for row in 0..row_count {
            if row == rank {
                continue;
            }
            let factor = matrix[row][column];
            if factor.abs() <= RANK_TOLERANCE {
                continue;
            }
            for entry in column..FACTORIAL_TERMS {
                matrix[row][entry] -= factor * matrix[rank][entry];
            }
        }
        rank += 1;
    }

    rank
}

#[cfg(test)]
mod tests {
    use super::*;

    fn selective_pattern() -> Vec<SensoryGateObservation> {
        SensoryGateCondition::full_factorial()
            .into_iter()
            .map(|condition| {
                let activity = match (condition.modality, condition.context, condition.gate) {
                    (
                        SensorModality::Movement,
                        MovementContext::SelfGenerated,
                        PresynapticGate::Intact,
                    ) => 4.0,
                    _ => 10.0,
                };
                SensoryGateObservation {
                    condition,
                    mean_activity: activity,
                }
            })
            .collect()
    }

    fn thresholds() -> SensoryGateThresholds {
        SensoryGateThresholds {
            minimum_movement_suppression: 3.0,
            maximum_non_effect: 0.5,
        }
    }

    #[test]
    fn full_factorial_design_is_identifiable() {
        let diagnostics = diagnose_design(&selective_pattern()).expect("valid factorial");
        assert_eq!(diagnostics.observed_conditions, 8);
        assert_eq!(diagnostics.missing_conditions, 0);
        assert_eq!(diagnostics.matrix_rank, 8);
        assert!(diagnostics.all_terms_identifiable);
    }

    #[test]
    fn selective_movement_channel_gate_passes_all_negative_controls() {
        let signature = classify_sensory_gate_signature(&selective_pattern(), thresholds())
            .expect("complete selective-gating design");
        assert!(signature.position_channel_context_invariant);
        assert!(signature.position_channel_gate_invariant);
        assert!(signature.movement_channel_suppressed_when_active_and_gated);
        assert!(signature.movement_suppression_requires_gate);
        assert!(signature.passive_movement_negative_control_passes);
        assert!(signature.qualified());
    }

    #[test]
    fn global_proprioceptive_damping_fails_position_channel_control() {
        let observations = SensoryGateCondition::full_factorial()
            .into_iter()
            .map(|condition| {
                let activity = if condition.context == MovementContext::SelfGenerated
                    && condition.gate == PresynapticGate::Intact
                {
                    4.0
                } else {
                    10.0
                };
                SensoryGateObservation {
                    condition,
                    mean_activity: activity,
                }
            })
            .collect::<Vec<_>>();
        let signature =
            classify_sensory_gate_signature(&observations, thresholds()).expect("complete design");
        assert!(!signature.position_channel_context_invariant);
        assert!(!signature.qualified());
    }

    #[test]
    fn movement_suppression_without_gate_dependence_fails() {
        let observations = SensoryGateCondition::full_factorial()
            .into_iter()
            .map(|condition| {
                let activity = if condition.modality == SensorModality::Movement
                    && condition.context == MovementContext::SelfGenerated
                {
                    4.0
                } else {
                    10.0
                };
                SensoryGateObservation {
                    condition,
                    mean_activity: activity,
                }
            })
            .collect::<Vec<_>>();
        let signature =
            classify_sensory_gate_signature(&observations, thresholds()).expect("complete design");
        assert!(signature.movement_channel_suppressed_when_active_and_gated);
        assert!(!signature.movement_suppression_requires_gate);
        assert!(!signature.qualified());
    }

    #[test]
    fn passive_movement_suppression_fails_negative_control() {
        let observations = SensoryGateCondition::full_factorial()
            .into_iter()
            .map(|condition| {
                let activity = if condition.modality == SensorModality::Movement
                    && condition.gate == PresynapticGate::Intact
                {
                    4.0
                } else {
                    10.0
                };
                SensoryGateObservation {
                    condition,
                    mean_activity: activity,
                }
            })
            .collect::<Vec<_>>();
        let signature =
            classify_sensory_gate_signature(&observations, thresholds()).expect("complete design");
        assert!(!signature.passive_movement_negative_control_passes);
        assert!(!signature.qualified());
    }

    #[test]
    fn missing_condition_forces_non_identifiability_abstention() {
        let mut observations = selective_pattern();
        observations.pop();
        let diagnostics =
            diagnose_design(&observations).expect("incomplete design is valid to diagnose");
        assert_eq!(diagnostics.matrix_rank, 7);
        assert!(!diagnostics.all_terms_identifiable);
        assert_eq!(
            recover_full_factorial(&observations),
            Err(SensoryGateError::NonIdentifiable { rank: 7 })
        );
    }

    #[test]
    fn duplicate_conditions_and_non_finite_activity_fail_closed() {
        let condition = SensoryGateCondition::new(
            SensorModality::Position,
            MovementContext::Passive,
            PresynapticGate::Blocked,
        );
        let duplicates = [
            SensoryGateObservation {
                condition,
                mean_activity: 1.0,
            },
            SensoryGateObservation {
                condition,
                mean_activity: 2.0,
            },
        ];
        assert_eq!(
            diagnose_design(&duplicates),
            Err(SensoryGateError::DuplicateCondition)
        );

        let non_finite = [SensoryGateObservation {
            condition,
            mean_activity: f64::NAN,
        }];
        assert_eq!(
            diagnose_design(&non_finite),
            Err(SensoryGateError::NonFiniteActivity)
        );
    }

    #[test]
    fn invalid_thresholds_are_rejected() {
        let observations = selective_pattern();
        assert_eq!(
            classify_sensory_gate_signature(
                &observations,
                SensoryGateThresholds {
                    minimum_movement_suppression: 0.2,
                    maximum_non_effect: 0.5,
                }
            ),
            Err(SensoryGateError::InvalidThresholds)
        );
        assert_eq!(
            classify_sensory_gate_signature(
                &observations,
                SensoryGateThresholds {
                    minimum_movement_suppression: f64::NAN,
                    maximum_non_effect: 0.5,
                }
            ),
            Err(SensoryGateError::InvalidThresholds)
        );
    }

    #[test]
    fn recovers_known_full_factorial_coefficients() {
        let expected = SensoryGateCoefficients {
            intercept: 5.0,
            movement_modality: 0.5,
            self_generated_context: -0.25,
            intact_gate: 0.75,
            modality_by_context: 1.0,
            modality_by_gate: -0.5,
            context_by_gate: 0.25,
            three_way: 2.0,
        };
        let observations = SensoryGateCondition::full_factorial()
            .into_iter()
            .map(|condition| SensoryGateObservation {
                condition,
                mean_activity: expected.predict(condition),
            })
            .collect::<Vec<_>>();
        let actual = recover_full_factorial(&observations).expect("known-answer recovery");
        for (left, right) in actual.as_array().into_iter().zip(expected.as_array()) {
            assert!((left - right).abs() < 1.0e-12, "{left} != {right}");
        }
    }
}
