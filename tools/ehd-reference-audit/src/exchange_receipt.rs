// SPDX-License-Identifier: AGPL-3.0-or-later

//! Event-level conservation receipts for plasma/neutral momentum exchange.
//!
//! This module validates an in-memory batch of physical exchange events. It
//! checks equal-and-opposite impulses per event (not merely after global
//! aggregation), rejects duplicate event identities, prevents one transfer
//! path from selecting mutually exclusive force-transfer methods, and rejects
//! implicit coordinate-frame mixing. It is not a durable exactly-once event
//! store: persistence, replay deduplication across process restarts and atomic
//! solver-side consumption belong to a later transaction adapter.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

/// Mutually exclusive representations of one physical plasma-to-neutral
/// momentum-transfer path.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum TransferMethod {
    ExplicitSpeciesCollisionDrag,
    ReducedCoulombDeposition,
}

/// One physical exchange contribution in a declared inertial coordinate
/// frame. Both impulses are already integrated over the same space-time
/// control volume and use N*s.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ExchangeEvent {
    pub event_id: u64,
    pub transfer_path_id: u64,
    pub method: TransferMethod,
    pub charged_solver_id: u64,
    pub neutral_solver_id: u64,
    pub charged_frame_id: u64,
    pub neutral_frame_id: u64,
    pub charged_reaction_impulse_ns: [f64; 3],
    pub neutral_impulse_ns: [f64; 3],
}

/// Case-specific componentwise equal-and-opposite tolerance.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ExchangeTolerance {
    pub absolute_impulse_ns: f64,
    pub relative: f64,
}

/// Deterministic result of validating a batch of exchange events.
#[derive(Clone, Debug, PartialEq)]
pub struct ExchangeReceipt {
    pub event_count: usize,
    pub path_methods: BTreeMap<u64, TransferMethod>,
    pub total_charged_reaction_impulse_ns: [f64; 3],
    pub total_neutral_impulse_ns: [f64; 3],
    /// Aggregate neutral plus charged impulse, component by component.
    pub aggregate_residual_ns: [f64; 3],
    /// Largest absolute component residual observed on any one event.
    pub max_event_residual_ns: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ReceiptError {
    EmptyBatch,
    InvalidTolerance,
    DuplicateEventId(u64),
    CoordinateFrameMismatch {
        event_id: u64,
        charged_frame_id: u64,
        neutral_frame_id: u64,
    },
    TransferMethodConflict {
        transfer_path_id: u64,
        first: TransferMethod,
        second: TransferMethod,
    },
    TransferPathIdentityConflict {
        transfer_path_id: u64,
    },
    NonFiniteImpulse(u64),
    EventBalanceFailure {
        event_id: u64,
        component: usize,
        residual_ns: f64,
        allowed_ns: f64,
    },
    AggregateBalanceFailure {
        component: usize,
        residual_ns: f64,
        allowed_ns: f64,
    },
    NonFiniteAccumulation,
}

impl fmt::Display for ReceiptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyBatch => write!(f, "no exchange events supplied"),
            Self::InvalidTolerance => write!(
                f,
                "exchange receipt tolerances must be finite and non-negative"
            ),
            Self::DuplicateEventId(id) => write!(f, "duplicate physical exchange event id: {id}"),
            Self::CoordinateFrameMismatch {
                event_id,
                charged_frame_id,
                neutral_frame_id,
            } => write!(
                f,
                "exchange event {event_id} mixes charged frame {charged_frame_id} and neutral frame {neutral_frame_id} without an explicit transform"
            ),
            Self::TransferMethodConflict {
                transfer_path_id,
                first,
                second,
            } => write!(
                f,
                "physical transfer path {transfer_path_id} selects conflicting methods {first:?} and {second:?}"
            ),
            Self::TransferPathIdentityConflict { transfer_path_id } => write!(
                f,
                "physical transfer path {transfer_path_id} changed solver-pair or coordinate-frame identity between events"
            ),
            Self::NonFiniteImpulse(id) => {
                write!(f, "exchange event {id} has a non-finite impulse component")
            }
            Self::EventBalanceFailure {
                event_id,
                component,
                residual_ns,
                allowed_ns,
            } => write!(
                f,
                "exchange event {event_id} component {component} residual {residual_ns} N*s exceeds allowed {allowed_ns} N*s"
            ),
            Self::AggregateBalanceFailure {
                component,
                residual_ns,
                allowed_ns,
            } => write!(
                f,
                "aggregate exchange component {component} residual {residual_ns} N*s exceeds allowed {allowed_ns} N*s"
            ),
            Self::NonFiniteAccumulation => {
                write!(f, "exchange receipt accumulation became non-finite")
            }
        }
    }
}

impl Error for ReceiptError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct TransferPathIdentity {
    method: TransferMethod,
    charged_solver_id: u64,
    neutral_solver_id: u64,
    frame_id: u64,
}

#[derive(Clone, Copy, Debug, Default)]
struct CompensatedSum {
    sum: f64,
    correction: f64,
}

impl CompensatedSum {
    fn add(&mut self, value: f64) {
        let adjusted = value - self.correction;
        let next = self.sum + adjusted;
        self.correction = (next - self.sum) - adjusted;
        self.sum = next;
    }

    fn value(self) -> f64 {
        self.sum
    }
}

/// Validate one batch of physical exchange events and produce a deterministic
/// conservation receipt.
///
/// Acceptance is event-local and path-local:
/// - event IDs must be unique within the batch;
/// - charged and neutral impulses must use the same declared frame;
/// - one physical transfer path must use exactly one closure method;
/// - each event's charged and neutral impulse vectors must cancel within the
///   componentwise absolute-plus-relative tolerance.
///
/// Aggregate conservation is checked as an additional invariant, never as a
/// replacement for the per-event checks.
pub fn validate_exchange_events(
    events: &[ExchangeEvent],
    tolerance: ExchangeTolerance,
) -> Result<ExchangeReceipt, ReceiptError> {
    if events.is_empty() {
        return Err(ReceiptError::EmptyBatch);
    }
    if !tolerance.absolute_impulse_ns.is_finite()
        || !tolerance.relative.is_finite()
        || tolerance.absolute_impulse_ns < 0.0
        || tolerance.relative < 0.0
    {
        return Err(ReceiptError::InvalidTolerance);
    }

    let mut ordered = events.to_vec();
    ordered.sort_by_key(|event| event.event_id);

    let mut seen_event_ids = BTreeSet::new();
    let mut path_methods = BTreeMap::new();
    let mut path_identities: BTreeMap<u64, TransferPathIdentity> = BTreeMap::new();
    let mut charged_totals = [CompensatedSum::default(); 3];
    let mut neutral_totals = [CompensatedSum::default(); 3];
    let mut aggregate_scales = [CompensatedSum::default(); 3];
    let mut max_event_residual_ns = 0.0_f64;

    for event in &ordered {
        if !seen_event_ids.insert(event.event_id) {
            return Err(ReceiptError::DuplicateEventId(event.event_id));
        }

        if event.charged_frame_id != event.neutral_frame_id {
            return Err(ReceiptError::CoordinateFrameMismatch {
                event_id: event.event_id,
                charged_frame_id: event.charged_frame_id,
                neutral_frame_id: event.neutral_frame_id,
            });
        }

        match path_methods.get(&event.transfer_path_id).copied() {
            Some(first) if first != event.method => {
                return Err(ReceiptError::TransferMethodConflict {
                    transfer_path_id: event.transfer_path_id,
                    first,
                    second: event.method,
                });
            }
            Some(_) => {}
            None => {
                path_methods.insert(event.transfer_path_id, event.method);
            }
        }

        let path_identity = TransferPathIdentity {
            method: event.method,
            charged_solver_id: event.charged_solver_id,
            neutral_solver_id: event.neutral_solver_id,
            frame_id: event.charged_frame_id,
        };
        match path_identities.get(&event.transfer_path_id) {
            Some(previous) if *previous != path_identity => {
                return Err(ReceiptError::TransferPathIdentityConflict {
                    transfer_path_id: event.transfer_path_id,
                });
            }
            Some(_) => {}
            None => {
                path_identities.insert(event.transfer_path_id, path_identity);
            }
        }

        if !event
            .charged_reaction_impulse_ns
            .iter()
            .all(|value| value.is_finite())
            || !event
                .neutral_impulse_ns
                .iter()
                .all(|value| value.is_finite())
        {
            return Err(ReceiptError::NonFiniteImpulse(event.event_id));
        }

        for component in 0..3 {
            let charged = event.charged_reaction_impulse_ns[component];
            let neutral = event.neutral_impulse_ns[component];
            let residual = charged + neutral;
            let scale = charged.abs().max(neutral.abs());
            let allowed = tolerance.absolute_impulse_ns + tolerance.relative * scale;

            if !residual.is_finite() || !allowed.is_finite() {
                return Err(ReceiptError::NonFiniteAccumulation);
            }
            max_event_residual_ns = max_event_residual_ns.max(residual.abs());
            if residual.abs() > allowed {
                return Err(ReceiptError::EventBalanceFailure {
                    event_id: event.event_id,
                    component,
                    residual_ns: residual,
                    allowed_ns: allowed,
                });
            }

            charged_totals[component].add(charged);
            neutral_totals[component].add(neutral);
            aggregate_scales[component].add(charged.abs() + neutral.abs());
        }
    }

    let total_charged_reaction_impulse_ns = [
        charged_totals[0].value(),
        charged_totals[1].value(),
        charged_totals[2].value(),
    ];
    let total_neutral_impulse_ns = [
        neutral_totals[0].value(),
        neutral_totals[1].value(),
        neutral_totals[2].value(),
    ];
    let mut aggregate_residual_ns = [0.0_f64; 3];

    for component in 0..3 {
        let residual =
            total_charged_reaction_impulse_ns[component] + total_neutral_impulse_ns[component];
        let scale = aggregate_scales[component].value();
        let allowed = tolerance.absolute_impulse_ns + tolerance.relative * scale;
        if !residual.is_finite() || !allowed.is_finite() {
            return Err(ReceiptError::NonFiniteAccumulation);
        }
        aggregate_residual_ns[component] = residual;
        if residual.abs() > allowed {
            return Err(ReceiptError::AggregateBalanceFailure {
                component,
                residual_ns: residual,
                allowed_ns: allowed,
            });
        }
    }

    Ok(ExchangeReceipt {
        event_count: ordered.len(),
        path_methods,
        total_charged_reaction_impulse_ns,
        total_neutral_impulse_ns,
        aggregate_residual_ns,
        max_event_residual_ns,
    })
}

/// Small deterministic reference fixture for the offline audit CLI.
pub fn reference_self_check() -> Result<(), ReceiptError> {
    let events = [
        ExchangeEvent {
            event_id: 1,
            transfer_path_id: 50,
            method: TransferMethod::ExplicitSpeciesCollisionDrag,
            charged_solver_id: 10,
            neutral_solver_id: 20,
            charged_frame_id: 7,
            neutral_frame_id: 7,
            charged_reaction_impulse_ns: [-1.0, 2.0, -3.0],
            neutral_impulse_ns: [1.0, -2.0, 3.0],
        },
        ExchangeEvent {
            event_id: 2,
            transfer_path_id: 50,
            method: TransferMethod::ExplicitSpeciesCollisionDrag,
            charged_solver_id: 10,
            neutral_solver_id: 20,
            charged_frame_id: 7,
            neutral_frame_id: 7,
            charged_reaction_impulse_ns: [0.5, -1.0, 0.0],
            neutral_impulse_ns: [-0.5, 1.0, 0.0],
        },
    ];
    let receipt = validate_exchange_events(
        &events,
        ExchangeTolerance {
            absolute_impulse_ns: 1.0e-12,
            relative: 1.0e-12,
        },
    )?;

    if receipt.event_count != 2
        || receipt.aggregate_residual_ns != [0.0; 3]
        || receipt.total_charged_reaction_impulse_ns != [-0.5, 1.0, -3.0]
        || receipt.total_neutral_impulse_ns != [0.5, -1.0, 3.0]
        || receipt.max_event_residual_ns != 0.0
    {
        return Err(ReceiptError::AggregateBalanceFailure {
            component: 0,
            residual_ns: receipt.aggregate_residual_ns[0],
            allowed_ns: 0.0,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tolerance() -> ExchangeTolerance {
        ExchangeTolerance {
            absolute_impulse_ns: 1.0e-12,
            relative: 1.0e-12,
        }
    }

    fn event(
        event_id: u64,
        path_id: u64,
        method: TransferMethod,
        charged_frame_id: u64,
        neutral_frame_id: u64,
        charged: [f64; 3],
        neutral: [f64; 3],
    ) -> ExchangeEvent {
        ExchangeEvent {
            event_id,
            transfer_path_id: path_id,
            method,
            charged_solver_id: 10,
            neutral_solver_id: 20,
            charged_frame_id,
            neutral_frame_id,
            charged_reaction_impulse_ns: charged,
            neutral_impulse_ns: neutral,
        }
    }

    #[test]
    fn accepts_exact_equal_and_opposite_exchange_events() {
        let receipt = validate_exchange_events(
            &[
                event(
                    1,
                    5,
                    TransferMethod::ExplicitSpeciesCollisionDrag,
                    7,
                    7,
                    [-1.0, 2.0, -3.0],
                    [1.0, -2.0, 3.0],
                ),
                event(
                    2,
                    5,
                    TransferMethod::ExplicitSpeciesCollisionDrag,
                    7,
                    7,
                    [0.5, -1.0, 0.0],
                    [-0.5, 1.0, 0.0],
                ),
            ],
            tolerance(),
        )
        .unwrap();
        assert_eq!(receipt.event_count, 2);
        assert_eq!(receipt.aggregate_residual_ns, [0.0; 3]);
        assert_eq!(
            receipt.path_methods.get(&5),
            Some(&TransferMethod::ExplicitSpeciesCollisionDrag)
        );
    }

    #[test]
    fn rejects_duplicate_event_ids() {
        let error = validate_exchange_events(
            &[
                event(
                    1,
                    5,
                    TransferMethod::ExplicitSpeciesCollisionDrag,
                    7,
                    7,
                    [-1.0, 0.0, 0.0],
                    [1.0, 0.0, 0.0],
                ),
                event(
                    1,
                    6,
                    TransferMethod::ExplicitSpeciesCollisionDrag,
                    7,
                    7,
                    [-2.0, 0.0, 0.0],
                    [2.0, 0.0, 0.0],
                ),
            ],
            tolerance(),
        )
        .unwrap_err();
        assert_eq!(error, ReceiptError::DuplicateEventId(1));
    }

    #[test]
    fn rejects_event_errors_that_cancel_in_the_global_sum() {
        let error = validate_exchange_events(
            &[
                event(
                    1,
                    5,
                    TransferMethod::ExplicitSpeciesCollisionDrag,
                    7,
                    7,
                    [-0.9, 0.0, 0.0],
                    [1.0, 0.0, 0.0],
                ),
                event(
                    2,
                    5,
                    TransferMethod::ExplicitSpeciesCollisionDrag,
                    7,
                    7,
                    [0.9, 0.0, 0.0],
                    [-1.0, 0.0, 0.0],
                ),
            ],
            ExchangeTolerance {
                absolute_impulse_ns: 1.0e-6,
                relative: 0.0,
            },
        )
        .unwrap_err();
        assert!(matches!(
            error,
            ReceiptError::EventBalanceFailure {
                event_id: 1,
                component: 0,
                ..
            }
        ));
    }

    #[test]
    fn rejects_mutually_exclusive_methods_on_one_physical_path() {
        let error = validate_exchange_events(
            &[
                event(
                    1,
                    5,
                    TransferMethod::ExplicitSpeciesCollisionDrag,
                    7,
                    7,
                    [-1.0, 0.0, 0.0],
                    [1.0, 0.0, 0.0],
                ),
                event(
                    2,
                    5,
                    TransferMethod::ReducedCoulombDeposition,
                    7,
                    7,
                    [-2.0, 0.0, 0.0],
                    [2.0, 0.0, 0.0],
                ),
            ],
            tolerance(),
        )
        .unwrap_err();
        assert!(matches!(
            error,
            ReceiptError::TransferMethodConflict {
                transfer_path_id: 5,
                ..
            }
        ));
    }

    #[test]
    fn rejects_solver_or_frame_identity_changes_within_one_path() {
        let mut second = event(
            2,
            5,
            TransferMethod::ExplicitSpeciesCollisionDrag,
            7,
            7,
            [-1.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
        );
        second.neutral_solver_id = 21;
        let error = validate_exchange_events(
            &[
                event(
                    1,
                    5,
                    TransferMethod::ExplicitSpeciesCollisionDrag,
                    7,
                    7,
                    [-1.0, 0.0, 0.0],
                    [1.0, 0.0, 0.0],
                ),
                second,
            ],
            tolerance(),
        )
        .unwrap_err();
        assert_eq!(
            error,
            ReceiptError::TransferPathIdentityConflict {
                transfer_path_id: 5
            }
        );
    }

    #[test]
    fn rejects_implicit_coordinate_frame_mixing() {
        let error = validate_exchange_events(
            &[event(
                1,
                5,
                TransferMethod::ExplicitSpeciesCollisionDrag,
                7,
                8,
                [-1.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
            )],
            tolerance(),
        )
        .unwrap_err();
        assert!(matches!(
            error,
            ReceiptError::CoordinateFrameMismatch {
                event_id: 1,
                charged_frame_id: 7,
                neutral_frame_id: 8
            }
        ));
    }

    #[test]
    fn rejects_non_finite_impulses() {
        let error = validate_exchange_events(
            &[event(
                1,
                5,
                TransferMethod::ExplicitSpeciesCollisionDrag,
                7,
                7,
                [f64::NAN, 0.0, 0.0],
                [0.0, 0.0, 0.0],
            )],
            tolerance(),
        )
        .unwrap_err();
        assert_eq!(error, ReceiptError::NonFiniteImpulse(1));
    }

    #[test]
    fn result_is_deterministic_under_event_input_permutation() {
        let a = event(
            2,
            5,
            TransferMethod::ExplicitSpeciesCollisionDrag,
            7,
            7,
            [0.5, -1.0, 0.0],
            [-0.5, 1.0, 0.0],
        );
        let b = event(
            1,
            5,
            TransferMethod::ExplicitSpeciesCollisionDrag,
            7,
            7,
            [-1.0, 2.0, -3.0],
            [1.0, -2.0, 3.0],
        );
        let forward = validate_exchange_events(&[a, b], tolerance()).unwrap();
        let reverse = validate_exchange_events(&[b, a], tolerance()).unwrap();
        assert_eq!(forward, reverse);
    }

    #[test]
    fn allows_multiple_events_on_one_path_when_method_is_constant() {
        let receipt = validate_exchange_events(
            &[
                event(
                    1,
                    5,
                    TransferMethod::ReducedCoulombDeposition,
                    9,
                    9,
                    [-1.0, 0.0, 0.0],
                    [1.0, 0.0, 0.0],
                ),
                event(
                    2,
                    5,
                    TransferMethod::ReducedCoulombDeposition,
                    9,
                    9,
                    [-2.0, 0.0, 0.0],
                    [2.0, 0.0, 0.0],
                ),
            ],
            tolerance(),
        )
        .unwrap();
        assert_eq!(
            receipt.path_methods.get(&5),
            Some(&TransferMethod::ReducedCoulombDeposition)
        );
    }

    #[test]
    fn rejects_empty_batches() {
        assert_eq!(
            validate_exchange_events(&[], tolerance()).unwrap_err(),
            ReceiptError::EmptyBatch
        );
    }

    #[test]
    fn rejects_invalid_tolerances() {
        let invalid = ExchangeTolerance {
            absolute_impulse_ns: 0.0,
            relative: f64::NAN,
        };
        assert_eq!(
            validate_exchange_events(
                &[event(
                    1,
                    5,
                    TransferMethod::ExplicitSpeciesCollisionDrag,
                    7,
                    7,
                    [-1.0, 0.0, 0.0],
                    [1.0, 0.0, 0.0]
                )],
                invalid,
            )
            .unwrap_err(),
            ReceiptError::InvalidTolerance
        );
    }

    #[test]
    fn reference_self_check_passes() {
        reference_self_check().unwrap();
    }
}
