//! Asynchronous two-modality microcosm for ErgoStream v0.1.
//!
//! The generator separates clock registration from cross-modal event
//! correspondence. Public views contain only modality-local observations,
//! sensor timestamps and arrival times. Event identities, generator regime,
//! symbol maps and valid correspondence classes remain evaluator-owned.
//!
//! This is a finite synthetic reference, not a sensor driver, a physical
//! synchronization service, an OS isolation boundary, or evidence of causal
//! relation between modalities.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

pub const MULTIMODAL_SCHEMA_VERSION_V0: u16 = 1;
pub const MIN_EVENTS_V0: usize = 4;
pub const MAX_EVENTS_V0: usize = 256;
const AMBIGUOUS_FAMILIES_V0: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ModalityV0 {
    Alpha,
    Beta,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ObservationKindV0 {
    Event,
    /// An explicit shared calibration pulse. Its identity is public because
    /// it is a legitimate synchronization anchor, not a hidden event label.
    ClockAnchor {
        anchor_id: u16,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssociationRegimeV0 {
    /// Both modalities observe the same latent event at the same event slot.
    Aligned,
    /// Beta carries a permutation of Alpha's event identities. The per-
    /// modality symbol and timestamp marginals are preserved exactly.
    MarginalMatchedShuffled,
    /// A periodic stream admits four equally valid event-phase correspondences.
    /// Clock-anchor presence is an independent configuration factor.
    PeriodicAmbiguous,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ClockModelV0 {
    pub offset: i64,
    pub rate_numerator: u64,
    pub rate_denominator: u64,
    pub jitter_bound: u64,
    pub max_arrival_delay: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MultimodalConfigV0 {
    /// Evaluator-only deterministic seed. Never copied into PolicyViewV0.
    pub seed: u64,
    pub event_count: usize,
    pub event_period: u64,
    pub regime: AssociationRegimeV0,
    pub alpha_clock: ClockModelV0,
    pub beta_clock: ClockModelV0,
    pub alpha_drop_per_mille: u16,
    pub beta_drop_per_mille: u16,
    /// Emit two public shared calibration pulses independently of the
    /// semantic association regime. This orthogonalizes clock and event
    /// correspondence identifiability.
    pub emit_shared_clock_anchors: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MultimodalWorldErrorV0 {
    EventCountOutOfRange,
    AmbiguousEventCountNotMultipleOfFour,
    ZeroEventPeriod,
    ZeroClockRate,
    ClockRateTooLarge,
    JitterBoundTooLarge,
    ArrivalDelayTooLarge,
    InvalidDropProbability,
    TimeOverflow,
    ObservationIdOverflow,
}

impl fmt::Display for MultimodalWorldErrorV0 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::EventCountOutOfRange => "event count must be within the v0 finite bound",
            Self::AmbiguousEventCountNotMultipleOfFour => {
                "periodic ambiguous worlds require an event count divisible by four"
            }
            Self::ZeroEventPeriod => "event period must be nonzero",
            Self::ZeroClockRate => "clock rate numerator and denominator must be nonzero",
            Self::ClockRateTooLarge => "clock rate components exceed the v0 arithmetic bound",
            Self::JitterBoundTooLarge => "jitter bound exceeds the signed timestamp range",
            Self::ArrivalDelayTooLarge => "arrival delay exceeds the v0 arithmetic bound",
            Self::InvalidDropProbability => "drop probability must be at most 500 per mille",
            Self::TimeOverflow => "clock timestamp or arrival tick overflow",
            Self::ObservationIdOverflow => "observation identifier overflow",
        };
        formatter.write_str(message)
    }
}

impl std::error::Error for MultimodalWorldErrorV0 {}

/// A modality-local observation. No event identity, family identity, clock
/// parameters, generator label, seed or future observation is exposed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObservationV0 {
    pub observation_id: u32,
    pub modality: ModalityV0,
    pub kind: ObservationKindV0,
    pub local_timestamp: i64,
    pub arrival_tick: u64,
    pub symbol: u16,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyViewV0 {
    pub schema_version: u16,
    pub observations: Vec<ObservationV0>,
    pub as_of_arrival_tick: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct EventPairV0 {
    pub alpha_observation_id: u32,
    pub beta_observation_id: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AlignmentClaimV0 {
    /// The learner asserts one complete event correspondence.
    Unique(Vec<EventPairV0>),
    /// The learner preserves several complete admissible correspondences.
    Alternatives(Vec<Vec<EventPairV0>>),
    /// The learner does not claim a unique correspondence.
    Unresolved,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PairMetricsV0 {
    pub true_positive: usize,
    pub false_positive: usize,
    pub false_negative: usize,
    pub precision: Option<f64>,
    pub recall: Option<f64>,
    pub f1: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AlignmentEvaluationV0 {
    pub valid_ground_truth_alternatives: usize,
    pub proposed_alternatives: usize,
    pub matching_alternatives: usize,
    pub false_alternatives: usize,
    pub missed_alternatives: usize,
    pub accepted_valid: bool,
    pub false_unique: bool,
    pub correctly_set_valued: bool,
    pub correct_abstention: bool,
    pub unnecessary_abstention: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClockMapProposalV0 {
    /// Map Beta local time into Alpha local time:
    /// alpha_time ~= offset + scale * beta_time.
    pub offset: f64,
    pub scale: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClockDispositionV0 {
    IdentifiableFromSharedAnchors,
    NonIdentifiableFromAvailableAnchors,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ClockEvaluationV0 {
    pub disposition: ClockDispositionV0,
    pub anchor_pairs: usize,
    pub correct_abstention: bool,
    pub false_unique: bool,
    pub unnecessary_abstention: bool,
    pub invalid_proposal: bool,
    pub rms_residual: Option<f64>,
    pub max_abs_residual: Option<f64>,
}

#[derive(Debug, Clone, Copy)]
struct HiddenEventLabelV0 {
    modality: ModalityV0,
    observation_id: u32,
    event_identity: usize,
    event_slot: usize,
}

#[derive(Debug, Clone)]
pub struct MultimodalWorldV0 {
    observations: Vec<ObservationV0>,
    hidden_event_labels: Vec<HiddenEventLabelV0>,
    regime: AssociationRegimeV0,
    event_count: usize,
}

#[derive(Debug, Clone, Copy)]
struct DeterministicRngV0 {
    state: u64,
}

impl DeterministicRngV0 {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        value ^ (value >> 31)
    }

    fn below(&mut self, upper_exclusive: u64) -> u64 {
        if upper_exclusive == 0 {
            0
        } else {
            self.next() % upper_exclusive
        }
    }

    fn shuffle<T>(&mut self, values: &mut [T]) {
        for index in (1..values.len()).rev() {
            let upper = u64::try_from(index + 1).unwrap_or(u64::MAX);
            let swap_index = usize::try_from(self.below(upper)).unwrap_or(0);
            values.swap(index, swap_index);
        }
    }

    fn signed_jitter(&mut self, bound: u64) -> i128 {
        if bound == 0 {
            return 0;
        }
        let width = u128::from(bound) * 2 + 1;
        let sampled = u128::from(self.next()) % width;
        i128::try_from(sampled).unwrap_or(0) - i128::from(bound)
    }
}

fn validate_config(config: MultimodalConfigV0) -> Result<(), MultimodalWorldErrorV0> {
    if !(MIN_EVENTS_V0..=MAX_EVENTS_V0).contains(&config.event_count) {
        return Err(MultimodalWorldErrorV0::EventCountOutOfRange);
    }
    if config.regime == AssociationRegimeV0::PeriodicAmbiguous
        && !config.event_count.is_multiple_of(AMBIGUOUS_FAMILIES_V0)
    {
        return Err(MultimodalWorldErrorV0::AmbiguousEventCountNotMultipleOfFour);
    }
    if config.event_period == 0 {
        return Err(MultimodalWorldErrorV0::ZeroEventPeriod);
    }

    for clock in [config.alpha_clock, config.beta_clock] {
        if clock.rate_numerator == 0 || clock.rate_denominator == 0 {
            return Err(MultimodalWorldErrorV0::ZeroClockRate);
        }
        if clock.rate_numerator > i64::MAX as u64 || clock.rate_denominator > i64::MAX as u64 {
            return Err(MultimodalWorldErrorV0::ClockRateTooLarge);
        }
        if clock.jitter_bound > i64::MAX as u64 {
            return Err(MultimodalWorldErrorV0::JitterBoundTooLarge);
        }
        if clock.max_arrival_delay > i64::MAX as u64 {
            return Err(MultimodalWorldErrorV0::ArrivalDelayTooLarge);
        }
    }

    if config.alpha_drop_per_mille > 500 || config.beta_drop_per_mille > 500 {
        return Err(MultimodalWorldErrorV0::InvalidDropProbability);
    }
    Ok(())
}

fn local_timestamp(
    true_tick: u64,
    clock: ClockModelV0,
    jitter_rng: &mut DeterministicRngV0,
) -> Result<i64, MultimodalWorldErrorV0> {
    let product = i128::from(true_tick)
        .checked_mul(i128::from(clock.rate_numerator))
        .ok_or(MultimodalWorldErrorV0::TimeOverflow)?;
    let scaled = product / i128::from(clock.rate_denominator);
    let jitter = jitter_rng.signed_jitter(clock.jitter_bound);
    let timestamp = i128::from(clock.offset)
        .checked_add(scaled)
        .and_then(|value| value.checked_add(jitter))
        .ok_or(MultimodalWorldErrorV0::TimeOverflow)?;
    i64::try_from(timestamp).map_err(|_| MultimodalWorldErrorV0::TimeOverflow)
}

fn make_observation(
    observation_id: u32,
    modality: ModalityV0,
    kind: ObservationKindV0,
    true_tick: u64,
    symbol: u16,
    clock: ClockModelV0,
    rng: &mut DeterministicRngV0,
) -> Result<ObservationV0, MultimodalWorldErrorV0> {
    let local_timestamp = local_timestamp(true_tick, clock, rng)?;
    let delay = rng.below(clock.max_arrival_delay.saturating_add(1));
    let arrival_tick = true_tick
        .checked_add(delay)
        .ok_or(MultimodalWorldErrorV0::TimeOverflow)?;
    Ok(ObservationV0 {
        observation_id,
        modality,
        kind,
        local_timestamp,
        arrival_tick,
        symbol,
    })
}

fn shuffled_symbol_map(symbol_count: usize, rng: &mut DeterministicRngV0) -> Vec<u16> {
    let mut mapping = (0..symbol_count)
        .map(|symbol| u16::try_from(symbol).unwrap_or(u16::MAX))
        .collect::<Vec<_>>();
    rng.shuffle(&mut mapping);
    mapping
}

impl MultimodalWorldV0 {
    /// Generate a finite hidden world. Independent PRNG substreams ensure
    /// that switching between aligned and shuffled association regimes does
    /// not alter either modality's timestamp or missingness marginals.
    pub fn generate(config: MultimodalConfigV0) -> Result<Self, MultimodalWorldErrorV0> {
        validate_config(config)?;

        let count = config.event_count;
        let mut alpha_rng = DeterministicRngV0::new(config.seed ^ 0xA11C_E001);
        let mut beta_rng = DeterministicRngV0::new(config.seed ^ 0xBE7A_0002);
        let mut shuffle_rng = DeterministicRngV0::new(config.seed ^ 0x5A11_F1E3);
        let mut symbol_rng = DeterministicRngV0::new(config.seed ^ 0x51B0_1A55);

        let alpha_visible = (0..count)
            .map(|_| alpha_rng.below(1_000) >= u64::from(config.alpha_drop_per_mille))
            .collect::<Vec<_>>();
        let beta_visible = (0..count)
            .map(|_| beta_rng.below(1_000) >= u64::from(config.beta_drop_per_mille))
            .collect::<Vec<_>>();

        let beta_visible_slots = beta_visible
            .iter()
            .enumerate()
            .filter_map(|(index, visible)| visible.then_some(index))
            .collect::<Vec<_>>();

        let mut beta_source_event_by_slot = vec![None; count];
        let mut beta_source_events = beta_visible_slots.clone();
        if config.regime == AssociationRegimeV0::MarginalMatchedShuffled {
            shuffle_rng.shuffle(&mut beta_source_events);
            if beta_source_events.len() > 1 && beta_source_events == beta_visible_slots {
                beta_source_events.rotate_left(1);
            }
        }
        for (slot, source_event) in beta_visible_slots.iter().zip(beta_source_events.iter()) {
            beta_source_event_by_slot[*slot] = Some(*source_event);
        }

        let symbol_count = match config.regime {
            AssociationRegimeV0::PeriodicAmbiguous => AMBIGUOUS_FAMILIES_V0,
            AssociationRegimeV0::Aligned | AssociationRegimeV0::MarginalMatchedShuffled => count,
        };
        let beta_symbol_by_family = shuffled_symbol_map(symbol_count, &mut symbol_rng);

        let mut observations = Vec::with_capacity(count.saturating_mul(2).saturating_add(4));
        let mut hidden_event_labels = Vec::with_capacity(count.saturating_mul(2));

        let mut alpha_timing_rng = DeterministicRngV0::new(config.seed ^ 0xA11C_71AE);
        let mut beta_timing_rng = DeterministicRngV0::new(config.seed ^ 0xBE7A_71AE);

        for event_index in 0..count {
            let event_number =
                u64::try_from(event_index + 1).map_err(|_| MultimodalWorldErrorV0::TimeOverflow)?;
            let true_tick = event_number
                .checked_mul(config.event_period)
                .ok_or(MultimodalWorldErrorV0::TimeOverflow)?;
            let observation_id = u32::try_from(event_index)
                .map_err(|_| MultimodalWorldErrorV0::ObservationIdOverflow)?;

            if alpha_visible[event_index] {
                let symbol = match config.regime {
                    AssociationRegimeV0::PeriodicAmbiguous => {
                        u16::try_from(event_index % AMBIGUOUS_FAMILIES_V0).unwrap_or(u16::MAX)
                    }
                    AssociationRegimeV0::Aligned | AssociationRegimeV0::MarginalMatchedShuffled => {
                        u16::try_from(event_index)
                            .map_err(|_| MultimodalWorldErrorV0::ObservationIdOverflow)?
                    }
                };
                observations.push(make_observation(
                    observation_id,
                    ModalityV0::Alpha,
                    ObservationKindV0::Event,
                    true_tick,
                    symbol,
                    config.alpha_clock,
                    &mut alpha_timing_rng,
                )?);
                hidden_event_labels.push(HiddenEventLabelV0 {
                    modality: ModalityV0::Alpha,
                    observation_id,
                    event_identity: event_index,
                    event_slot: event_index,
                });
            }

            if let Some(source_event) = beta_source_event_by_slot[event_index] {
                let family = match config.regime {
                    AssociationRegimeV0::PeriodicAmbiguous => event_index % AMBIGUOUS_FAMILIES_V0,
                    AssociationRegimeV0::Aligned | AssociationRegimeV0::MarginalMatchedShuffled => {
                        source_event
                    }
                };
                let symbol = beta_symbol_by_family[family];
                observations.push(make_observation(
                    observation_id,
                    ModalityV0::Beta,
                    ObservationKindV0::Event,
                    true_tick,
                    symbol,
                    config.beta_clock,
                    &mut beta_timing_rng,
                )?);
                hidden_event_labels.push(HiddenEventLabelV0 {
                    modality: ModalityV0::Beta,
                    observation_id,
                    event_identity: source_event,
                    event_slot: event_index,
                });
            }
        }

        if config.emit_shared_clock_anchors {
            let end_event_number =
                u64::try_from(count + 1).map_err(|_| MultimodalWorldErrorV0::TimeOverflow)?;
            let end_tick = end_event_number
                .checked_mul(config.event_period)
                .ok_or(MultimodalWorldErrorV0::TimeOverflow)?;
            let first_anchor_id =
                u32::try_from(count).map_err(|_| MultimodalWorldErrorV0::ObservationIdOverflow)?;
            let second_anchor_id = first_anchor_id
                .checked_add(1)
                .ok_or(MultimodalWorldErrorV0::ObservationIdOverflow)?;

            for (anchor_id, observation_id, true_tick) in [
                (0_u16, first_anchor_id, 0_u64),
                (1_u16, second_anchor_id, end_tick),
            ] {
                observations.push(make_observation(
                    observation_id,
                    ModalityV0::Alpha,
                    ObservationKindV0::ClockAnchor { anchor_id },
                    true_tick,
                    u16::MAX,
                    config.alpha_clock,
                    &mut alpha_timing_rng,
                )?);
                observations.push(make_observation(
                    observation_id,
                    ModalityV0::Beta,
                    ObservationKindV0::ClockAnchor { anchor_id },
                    true_tick,
                    u16::MAX,
                    config.beta_clock,
                    &mut beta_timing_rng,
                )?);
            }
        }

        Ok(Self {
            observations,
            hidden_event_labels,
            regime: config.regime,
            event_count: count,
        })
    }

    /// Expose only observations whose arrival tick is within the requested
    /// causal cutoff. Measurement timestamps are not used to order arrival.
    pub fn policy_view(&self, as_of_arrival_tick: u64) -> PolicyViewV0 {
        let mut observations = self
            .observations
            .iter()
            .copied()
            .filter(|observation| observation.arrival_tick <= as_of_arrival_tick)
            .collect::<Vec<_>>();
        observations.sort_by_key(|observation| {
            (
                observation.arrival_tick,
                observation.modality,
                observation.observation_id,
            )
        });
        PolicyViewV0 {
            schema_version: MULTIMODAL_SCHEMA_VERSION_V0,
            observations,
            as_of_arrival_tick,
        }
    }

    /// Reconstruct evaluator truth through a separate correspondence pass over
    /// hidden identity labels. Generation never serializes these labels into
    /// PolicyViewV0; the learner process receives only its public observations.
    fn exact_alignment_oracle(&self) -> Vec<Vec<EventPairV0>> {
        match self.regime {
            AssociationRegimeV0::Aligned | AssociationRegimeV0::MarginalMatchedShuffled => {
                let mut alpha_by_event = BTreeMap::new();
                let mut beta_by_event = BTreeMap::new();
                for label in &self.hidden_event_labels {
                    match label.modality {
                        ModalityV0::Alpha => {
                            alpha_by_event.insert(label.event_identity, label.observation_id);
                        }
                        ModalityV0::Beta => {
                            beta_by_event.insert(label.event_identity, label.observation_id);
                        }
                    }
                }
                let pairs = alpha_by_event
                    .iter()
                    .filter_map(|(event_id, alpha_id)| {
                        beta_by_event.get(event_id).map(|beta_id| EventPairV0 {
                            alpha_observation_id: *alpha_id,
                            beta_observation_id: *beta_id,
                        })
                    })
                    .collect::<Vec<_>>();
                vec![pairs]
            }
            AssociationRegimeV0::PeriodicAmbiguous => {
                let mut alpha_by_slot = BTreeMap::new();
                let mut beta_by_slot = BTreeMap::new();
                for label in &self.hidden_event_labels {
                    match label.modality {
                        ModalityV0::Alpha => {
                            alpha_by_slot.insert(label.event_slot, label.observation_id);
                        }
                        ModalityV0::Beta => {
                            beta_by_slot.insert(label.event_slot, label.observation_id);
                        }
                    }
                }
                let mut alternatives = Vec::with_capacity(AMBIGUOUS_FAMILIES_V0);
                for phase in 0..AMBIGUOUS_FAMILIES_V0 {
                    let mut pairs = Vec::new();
                    for (alpha_slot, alpha_id) in &alpha_by_slot {
                        let beta_slot = (*alpha_slot + phase) % self.event_count;
                        if let Some(beta_id) = beta_by_slot.get(&beta_slot) {
                            pairs.push(EventPairV0 {
                                alpha_observation_id: *alpha_id,
                                beta_observation_id: *beta_id,
                            });
                        }
                    }
                    alternatives.push(pairs);
                }
                alternatives
            }
        }
    }

    /// Score a frozen correspondence proposal against evaluator-owned
    /// equivalence classes. The learner receives only PolicyViewV0.
    pub fn evaluate_alignment(
        &self,
        claim: &AlignmentClaimV0,
    ) -> (AlignmentEvaluationV0, Option<PairMetricsV0>) {
        let truth = normalize_alternatives(&self.exact_alignment_oracle());

        let (proposed, is_unique, is_unresolved) = match claim {
            AlignmentClaimV0::Unique(pairs) => (vec![normalize_pairs(pairs)], true, false),
            AlignmentClaimV0::Alternatives(alternatives) => {
                (normalize_alternatives(alternatives), false, false)
            }
            AlignmentClaimV0::Unresolved => (Vec::new(), false, true),
        };

        let matching_alternatives = proposed
            .iter()
            .filter(|proposal| truth.contains(proposal))
            .count();
        let false_alternatives = proposed.len().saturating_sub(matching_alternatives);
        let missed_alternatives = truth.len().saturating_sub(matching_alternatives);
        let false_unique = is_unique && truth.len() > 1;
        let correct_abstention = is_unresolved && truth.len() > 1;
        let unnecessary_abstention = is_unresolved && truth.len() == 1;
        let correctly_set_valued = matches!(claim, AlignmentClaimV0::Alternatives(_))
            && truth.len() > 1
            && false_alternatives == 0
            && missed_alternatives == 0;

        let accepted_valid = match claim {
            AlignmentClaimV0::Unique(_) => matching_alternatives == 1,
            AlignmentClaimV0::Alternatives(_) => {
                truth.len() > 0 && false_alternatives == 0 && missed_alternatives == 0
            }
            AlignmentClaimV0::Unresolved => false,
        };

        let pair_metrics = proposed
            .first()
            .map(|proposal| best_pair_metrics(proposal, &truth));

        (
            AlignmentEvaluationV0 {
                valid_ground_truth_alternatives: truth.len(),
                proposed_alternatives: proposed.len(),
                matching_alternatives,
                false_alternatives,
                missed_alternatives,
                accepted_valid,
                false_unique,
                correctly_set_valued,
                correct_abstention,
                unnecessary_abstention,
            },
            pair_metrics,
        )
    }

    /// Score a proposed affine map from Beta's local clock into Alpha's.
    /// Shared calibration pulses establish timing independently of event
    /// semantics. Without two distinct shared anchors, a singleton clock
    /// estimate is not credited as identifiable.
    pub fn evaluate_clock_map(
        &self,
        view: &PolicyViewV0,
        proposal: Option<ClockMapProposalV0>,
    ) -> ClockEvaluationV0 {
        let mut alpha_anchors = BTreeMap::new();
        let mut beta_anchors = BTreeMap::new();
        for observation in &view.observations {
            if let ObservationKindV0::ClockAnchor { anchor_id } = observation.kind {
                match observation.modality {
                    ModalityV0::Alpha => {
                        alpha_anchors.insert(anchor_id, observation.local_timestamp);
                    }
                    ModalityV0::Beta => {
                        beta_anchors.insert(anchor_id, observation.local_timestamp);
                    }
                }
            }
        }

        let paired = alpha_anchors
            .iter()
            .filter_map(|(anchor_id, alpha_time)| {
                beta_anchors
                    .get(anchor_id)
                    .map(|beta_time| (*alpha_time, *beta_time))
            })
            .collect::<Vec<_>>();
        let distinct = paired.len() >= 2
            && paired
                .windows(2)
                .any(|window| window[0].0 != window[1].0 && window[0].1 != window[1].1);
        let identifiable = distinct;

        let (rms_residual, max_abs_residual, invalid_proposal) = match (identifiable, proposal) {
            (true, Some(candidate)) => {
                if !candidate.offset.is_finite()
                    || !candidate.scale.is_finite()
                    || candidate.scale <= 0.0
                {
                    (None, None, true)
                } else {
                    let residuals = paired
                        .iter()
                        .map(|(alpha, beta)| {
                            candidate.offset + candidate.scale * (*beta as f64) - (*alpha as f64)
                        })
                        .collect::<Vec<_>>();
                    let mean_square = residuals
                        .iter()
                        .map(|residual| residual * residual)
                        .sum::<f64>()
                        / residuals.len() as f64;
                    let max_abs = residuals
                        .iter()
                        .map(|residual| residual.abs())
                        .fold(0.0_f64, f64::max);
                    (Some(mean_square.sqrt()), Some(max_abs), false)
                }
            }
            _ => (None, None, false),
        };

        ClockEvaluationV0 {
            disposition: if identifiable {
                ClockDispositionV0::IdentifiableFromSharedAnchors
            } else {
                ClockDispositionV0::NonIdentifiableFromAvailableAnchors
            },
            anchor_pairs: paired.len(),
            correct_abstention: !identifiable && proposal.is_none(),
            false_unique: !identifiable && proposal.is_some(),
            unnecessary_abstention: identifiable && proposal.is_none(),
            invalid_proposal,
            rms_residual,
            max_abs_residual,
        }
    }
}

fn normalize_pairs(pairs: &[EventPairV0]) -> Vec<EventPairV0> {
    let mut normalized = pairs.to_vec();
    normalized.sort_unstable();
    normalized.dedup();
    normalized
}

fn normalize_alternatives(alternatives: &[Vec<EventPairV0>]) -> Vec<Vec<EventPairV0>> {
    let mut normalized = alternatives
        .iter()
        .map(|pairs| normalize_pairs(pairs))
        .collect::<Vec<_>>();
    normalized.sort_unstable();
    normalized.dedup();
    normalized
}

fn best_pair_metrics(proposal: &[EventPairV0], truths: &[Vec<EventPairV0>]) -> PairMetricsV0 {
    truths
        .iter()
        .map(|truth| pair_metrics(proposal, truth))
.max_by(|left, right| left.f1.unwrap_or(0.0).total_cmp(&right.f1.unwrap_or(0.0)))
        .unwrap_or(PairMetricsV0 {
            true_positive: 0,
            false_positive: proposal.len(),
            false_negative: 0,
            precision: None,
            recall: None,
            f1: None,
        })
}

fn pair_metrics(proposal: &[EventPairV0], truth: &[EventPairV0]) -> PairMetricsV0 {
    let proposed = proposal.iter().copied().collect::<BTreeSet<_>>();
    let expected = truth.iter().copied().collect::<BTreeSet<_>>();
    let true_positive = proposed.intersection(&expected).count();
    let false_positive = proposed.difference(&expected).count();
    let false_negative = expected.difference(&proposed).count();

    let precision = if true_positive + false_positive == 0 {
        None
    } else {
        Some(true_positive as f64 / (true_positive + false_positive) as f64)
    };
    let recall = if true_positive + false_negative == 0 {
        None
    } else {
        Some(true_positive as f64 / (true_positive + false_negative) as f64)
    };
    let f1 = match (precision, recall) {
        (Some(p), Some(r)) if p + r > 0.0 => Some(2.0 * p * r / (p + r)),
        (Some(_), Some(_)) => Some(0.0),
        _ => None,
    };

    PairMetricsV0 {
        true_positive,
        false_positive,
        false_negative,
        precision,
        recall,
        f1,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clock(offset: i64, rate_numerator: u64, rate_denominator: u64) -> ClockModelV0 {
        ClockModelV0 {
            offset,
            rate_numerator,
            rate_denominator,
            jitter_bound: 0,
            max_arrival_delay: 0,
        }
    }

    fn config(regime: AssociationRegimeV0) -> MultimodalConfigV0 {
        MultimodalConfigV0 {
            seed: 0xE260_2026,
            event_count: 12,
            event_period: 100,
            regime,
            alpha_clock: clock(20, 1, 1),
            beta_clock: clock(100, 3, 2),
            alpha_drop_per_mille: 0,
            beta_drop_per_mille: 0,
            emit_shared_clock_anchors: true,
        }
    }

    fn all_observations(world: &MultimodalWorldV0) -> PolicyViewV0 {
        world.policy_view(u64::MAX)
    }

    #[test]
    fn aligned_world_has_one_truth_and_identifiable_clock() {
        let world = MultimodalWorldV0::generate(config(AssociationRegimeV0::Aligned)).unwrap();
        let view = all_observations(&world);
        let claim = AlignmentClaimV0::Unique(world.exact_alignment_oracle()[0].clone());
        let (score, pairs) = world.evaluate_alignment(&claim);

        assert!(score.accepted_valid);
        assert!(!score.false_unique);
        assert_eq!(score.valid_ground_truth_alternatives, 1);
        assert_eq!(pairs.and_then(|metrics| metrics.f1), Some(1.0));

        let clock_score = world.evaluate_clock_map(
            &view,
            Some(ClockMapProposalV0 {
                offset: 20.0 - (2.0 / 3.0) * 100.0,
                scale: 2.0 / 3.0,
            }),
        );
        assert_eq!(
            clock_score.disposition,
            ClockDispositionV0::IdentifiableFromSharedAnchors
        );
        assert_eq!(clock_score.anchor_pairs, 2);
        assert!(clock_score.rms_residual.unwrap() < 1.0);

        let one_anchor_view = world.policy_view(1);
        let one_anchor_score = world.evaluate_clock_map(&one_anchor_view, None);
        assert_eq!(one_anchor_score.anchor_pairs, 1);
        assert_eq!(
            one_anchor_score.disposition,
            ClockDispositionV0::NonIdentifiableFromAvailableAnchors
        );
        assert!(one_anchor_score.correct_abstention);
    }

    #[test]
    fn shuffled_negative_preserves_each_modality_marginals() {
        let mut aligned_config = config(AssociationRegimeV0::Aligned);
        aligned_config.alpha_drop_per_mille = 100;
        aligned_config.beta_drop_per_mille = 125;
        let mut shuffled_config = aligned_config;
        shuffled_config.regime = AssociationRegimeV0::MarginalMatchedShuffled;
        let aligned = MultimodalWorldV0::generate(aligned_config).unwrap();
        let shuffled = MultimodalWorldV0::generate(shuffled_config).unwrap();
        let aligned_view = all_observations(&aligned);
        let shuffled_view = all_observations(&shuffled);

        for modality in [ModalityV0::Alpha, ModalityV0::Beta] {
            let mut aligned_symbols = aligned_view
                .observations
                .iter()
                .filter(|observation| {
                    observation.modality == modality && observation.kind == ObservationKindV0::Event
                })
                .map(|observation| observation.symbol)
                .collect::<Vec<_>>();
            let mut shuffled_symbols = shuffled_view
                .observations
                .iter()
                .filter(|observation| {
                    observation.modality == modality
                        && observation.kind == ObservationKindV0::Event
                })
                .map(|observation| observation.symbol)
                .collect::<Vec<_>>();
            aligned_symbols.sort_unstable();
            shuffled_symbols.sort_unstable();
            assert_eq!(aligned_symbols, shuffled_symbols);

            let mut aligned_times = aligned_view
                .observations
                .iter()
                .filter(|observation| {
                    observation.modality == modality
                        && observation.kind == ObservationKindV0::Event
                })
                .map(|observation| observation.local_timestamp)
                .collect::<Vec<_>>();
            let mut shuffled_times = shuffled_view
                .observations
                .iter()
                .filter(|observation| {
                    observation.modality == modality
                        && observation.kind == ObservationKindV0::Event
                })
                .map(|observation| observation.local_timestamp)
                .collect::<Vec<_>>();
            aligned_times.sort_unstable();
            shuffled_times.sort_unstable();
            assert_eq!(aligned_times, shuffled_times);
        }

        assert_ne!(
            aligned.exact_alignment_oracle()[0],
            shuffled.exact_alignment_oracle()[0]
        );

        let same_slot_claim = AlignmentClaimV0::Unique(
            (0..12)
                .map(|index| EventPairV0 {
                    alpha_observation_id: index,
                    beta_observation_id: index,
                })
                .collect(),
        );
        let (score, _) = shuffled.evaluate_alignment(&same_slot_claim);
        assert!(!score.accepted_valid);
    }

    #[test]
    fn periodic_world_requires_set_valued_claim_or_abstention() {
        let world =
            MultimodalWorldV0::generate(config(AssociationRegimeV0::PeriodicAmbiguous)).unwrap();
        assert_eq!(world.exact_alignment_oracle().len(), AMBIGUOUS_FAMILIES_V0);

        let (abstain_score, _) = world.evaluate_alignment(&AlignmentClaimV0::Unresolved);
        assert!(abstain_score.correct_abstention);
        assert!(!abstain_score.false_unique);

        let false_unique = AlignmentClaimV0::Unique(world.exact_alignment_oracle()[0].clone());
        let (unique_score, _) = world.evaluate_alignment(&false_unique);
        assert!(unique_score.accepted_valid);
        assert!(unique_score.false_unique);

        let alternatives = AlignmentClaimV0::Alternatives(world.exact_alignment_oracle());
        let (set_score, _) = world.evaluate_alignment(&alternatives);
        assert!(set_score.correctly_set_valued);
        assert!(set_score.accepted_valid);

        // The same periodic event stream can have independently identifiable
        // clocks when public calibration anchors are present.
        let clock_score = world.evaluate_clock_map(
            &all_observations(&world),
            Some(ClockMapProposalV0 {
                offset: 20.0 - (2.0 / 3.0) * 100.0,
                scale: 2.0 / 3.0,
            }),
        );
        assert_eq!(
            clock_score.disposition,
            ClockDispositionV0::IdentifiableFromSharedAnchors
        );
    }

    #[test]
    fn clock_score_is_independent_of_semantic_shuffling() {
        let proposal = Some(ClockMapProposalV0 {
            offset: 20.0 - (2.0 / 3.0) * 100.0,
            scale: 2.0 / 3.0,
        });
        let aligned = MultimodalWorldV0::generate(config(AssociationRegimeV0::Aligned)).unwrap();
        let shuffled =
            MultimodalWorldV0::generate(config(AssociationRegimeV0::MarginalMatchedShuffled))
                .unwrap();

        let aligned_score = aligned.evaluate_clock_map(&all_observations(&aligned), proposal);
        let shuffled_score = shuffled.evaluate_clock_map(&all_observations(&shuffled), proposal);
        assert_eq!(aligned_score.anchor_pairs, shuffled_score.anchor_pairs);
        assert_eq!(aligned_score.rms_residual, shuffled_score.rms_residual);
        assert_eq!(aligned_score.disposition, shuffled_score.disposition);
    }

    #[test]
    fn periodic_world_correctly_abstains_on_clock_registration() {
        let mut ambiguous_config = config(AssociationRegimeV0::PeriodicAmbiguous);
        ambiguous_config.emit_shared_clock_anchors = false;
        let world = MultimodalWorldV0::generate(ambiguous_config).unwrap();
        let score = world.evaluate_clock_map(&all_observations(&world), None);
        assert_eq!(
            score.disposition,
            ClockDispositionV0::NonIdentifiableFromAvailableAnchors
        );
        assert!(score.correct_abstention);
        assert!(!score.false_unique);

        let overconfident = world.evaluate_clock_map(
            &all_observations(&world),
            Some(ClockMapProposalV0 {
                offset: 0.0,
                scale: 1.0,
            }),
        );
        assert!(overconfident.false_unique);
    }

    #[test]
    fn policy_view_is_causal_and_sorted_by_arrival_not_sensor_time() {
        let mut config = config(AssociationRegimeV0::Aligned);
        config.alpha_clock.jitter_bound = 20;
        config.beta_clock.jitter_bound = 20;
        config.alpha_clock.max_arrival_delay = 250;
        config.beta_clock.max_arrival_delay = 250;
        let world = MultimodalWorldV0::generate(config).unwrap();
        let cutoff = 350;
        let view = world.policy_view(cutoff);

        assert!(
            view.observations
                .iter()
                .all(|observation| observation.arrival_tick <= cutoff)
        );
        assert!(view.observations.windows(2).all(|window| {
            (
                window[0].arrival_tick,
                window[0].modality,
                window[0].observation_id,
            ) <= (
                window[1].arrival_tick,
                window[1].modality,
                window[1].observation_id,
            )
        }));
        assert!(view.observations.len() < world.observations.len());
    }

    #[test]
    fn arrival_order_can_differ_from_sensor_time_order() {
        let mut config = config(AssociationRegimeV0::Aligned);
        config.alpha_clock.jitter_bound = 20;
        config.beta_clock.jitter_bound = 20;
        config.alpha_clock.max_arrival_delay = 500;
        config.beta_clock.max_arrival_delay = 500;
        let world = MultimodalWorldV0::generate(config).unwrap();
        let view = all_observations(&world);

        let reordered = view.observations.windows(2).any(|window| {
            window[0].modality == window[1].modality
                && window[0].kind == ObservationKindV0::Event
                && window[1].kind == ObservationKindV0::Event
                && window[0].local_timestamp > window[1].local_timestamp
        });
        assert!(
            reordered,
            "the deterministic fixture must contain out-of-order sensor-time observations"
        );
    }

    #[test]
    fn invalid_clock_and_missingness_parameters_fail_closed() {
        let mut bad_clock = config(AssociationRegimeV0::Aligned);
        bad_clock.beta_clock.rate_denominator = 0;
        assert_eq!(
            MultimodalWorldV0::generate(bad_clock).unwrap_err(),
            MultimodalWorldErrorV0::ZeroClockRate
        );

        let mut bad_missingness = config(AssociationRegimeV0::Aligned);
        bad_missingness.beta_drop_per_mille = 501;
        assert_eq!(
            MultimodalWorldV0::generate(bad_missingness).unwrap_err(),
            MultimodalWorldErrorV0::InvalidDropProbability
        );
    }
}
