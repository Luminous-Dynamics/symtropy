//! Receiver-side cargo outcome and conservative service-floor evaluator.
//!
//! Receiver acceptance is the service endpoint. Vessel departure and terminal
//! exit alone do not establish cargo delivery. Censored, disputed and
//! unverified outcomes remain unresolved; they are not silently counted as
//! success or failure.

use std::collections::{BTreeMap, BTreeSet};

const BASIS_POINTS: u128 = 10_000;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum OutcomeEvidence {
    ReceiverAccepted {
        accepted_at: i64,
        independently_verified: bool,
    },
    NotAcceptedByHorizon {
        independently_verified: bool,
    },
    TerminalExitOnly {
        terminal_exit_at: i64,
    },
    Censored {
        last_observed_at: i64,
    },
    Disputed,
    LostOrDamaged {
        observed_at: i64,
        independently_verified: bool,
    },
    UnitReconciliationFailure {
        reason: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CargoUnitOutcome {
    pub unit_id: String,
    pub cohort_id: String,
    pub quantity: u64,
    pub unit: String,
    pub deadline_at: i64,
    pub evidence: OutcomeEvidence,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProtectedServiceFloor {
    pub cohort_id: String,
    pub unit: String,
    /// 0..=10_000 basis points; 10_000 is 100%.
    pub minimum_basis_points: u16,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FloorDisposition {
    Pass,
    Fail,
    Unknown,
    InvalidEvidence,
    NotConfigured,
}

impl FloorDisposition {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pass => "PASS",
            Self::Fail => "FAIL",
            Self::Unknown => "UNKNOWN",
            Self::InvalidEvidence => "INVALID_EVIDENCE",
            Self::NotConfigured => "NOT_CONFIGURED",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CohortServiceSummary {
    pub cohort_id: String,
    pub unit: String,
    pub scheduled_quantity: u64,
    pub adjudicated_quantity: u64,
    pub on_time_quantity: u64,
    pub late_quantity: u64,
    pub not_accepted_quantity: u64,
    pub lost_or_damaged_quantity: u64,
    pub unresolved_quantity: u64,
    pub reconciliation_failure_quantity: u64,
    pub on_time_observed_numerator: u64,
    pub on_time_observed_denominator: u64,
    pub conservative_lower_numerator: u64,
    pub conservative_lower_denominator: u64,
    pub best_case_upper_numerator: u64,
    pub best_case_upper_denominator: u64,
    pub floor_basis_points: Option<u16>,
    pub floor_disposition: FloorDisposition,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReceiverServiceReport {
    pub horizon_at: i64,
    pub unit_count: usize,
    pub cohorts: Vec<CohortServiceSummary>,
}

impl ReceiverServiceReport {
    pub fn all_configured_floors_pass(&self) -> bool {
        self.cohorts.iter().all(|cohort| {
            matches!(
                cohort.floor_disposition,
                FloorDisposition::Pass | FloorDisposition::NotConfigured
            )
        })
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ServiceError {
    EmptyIdentifier,
    ZeroQuantity,
    DuplicateUnitId(String),
    DuplicateFloor {
        cohort_id: String,
        unit: String,
    },
    FloorWithoutScheduledUnits {
        cohort_id: String,
        unit: String,
    },
    InvalidFloor {
        cohort_id: String,
        unit: String,
        basis_points: u16,
    },
    QuantityOverflow {
        cohort_id: String,
        unit: String,
    },
    EvidenceAfterHorizon {
        unit_id: String,
    },
}

impl std::fmt::Display for ServiceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ServiceError {}

#[derive(Clone, Debug, Default)]
struct Accumulator {
    scheduled: u64,
    adjudicated: u64,
    on_time: u64,
    late: u64,
    not_accepted: u64,
    lost_or_damaged: u64,
    unresolved: u64,
    reconciliation_failure: u64,
}

impl Accumulator {
    fn add(&mut self, field: &'static str, quantity: u64) -> Result<(), ()> {
        let value = match field {
            "scheduled" => &mut self.scheduled,
            "adjudicated" => &mut self.adjudicated,
            "on_time" => &mut self.on_time,
            "late" => &mut self.late,
            "not_accepted" => &mut self.not_accepted,
            "lost_or_damaged" => &mut self.lost_or_damaged,
            "unresolved" => &mut self.unresolved,
            "reconciliation_failure" => &mut self.reconciliation_failure,
            _ => unreachable!("closed set of accumulator fields"),
        };
        *value = value.checked_add(quantity).ok_or(())?;
        Ok(())
    }
}

fn classify(
    record: &CargoUnitOutcome,
    horizon_at: i64,
    accumulator: &mut Accumulator,
) -> Result<(), ServiceError> {
    accumulator
        .add("scheduled", record.quantity)
        .map_err(|_| ServiceError::QuantityOverflow {
            cohort_id: record.cohort_id.clone(),
            unit: record.unit.clone(),
        })?;

    match &record.evidence {
        OutcomeEvidence::ReceiverAccepted {
            accepted_at,
            independently_verified: true,
        } => {
            accumulator
                .add("adjudicated", record.quantity)
                .map_err(|_| ServiceError::QuantityOverflow {
                    cohort_id: record.cohort_id.clone(),
                    unit: record.unit.clone(),
                })?;
            if *accepted_at <= horizon_at && *accepted_at <= record.deadline_at {
                accumulator.add("on_time", record.quantity).map_err(|_| {
                    ServiceError::QuantityOverflow {
                        cohort_id: record.cohort_id.clone(),
                        unit: record.unit.clone(),
                    }
                })?;
            } else if *accepted_at <= horizon_at {
                accumulator.add("late", record.quantity).map_err(|_| {
                    ServiceError::QuantityOverflow {
                        cohort_id: record.cohort_id.clone(),
                        unit: record.unit.clone(),
                    }
                })?;
            } else {
                // A verified acceptance after the measurement horizon means
                // the cargo was not receiver-accepted by that horizon.
                accumulator
                    .add("not_accepted", record.quantity)
                    .map_err(|_| ServiceError::QuantityOverflow {
                        cohort_id: record.cohort_id.clone(),
                        unit: record.unit.clone(),
                    })?;
            }
        }
        OutcomeEvidence::ReceiverAccepted {
            independently_verified: false,
            ..
        }
        | OutcomeEvidence::Disputed
        | OutcomeEvidence::Censored { .. }
        | OutcomeEvidence::TerminalExitOnly { .. } => {
            if let OutcomeEvidence::Censored { last_observed_at } = &record.evidence {
                if *last_observed_at > horizon_at {
                    return Err(ServiceError::EvidenceAfterHorizon {
                        unit_id: record.unit_id.clone(),
                    });
                }
            }
            if let OutcomeEvidence::TerminalExitOnly { terminal_exit_at } = &record.evidence {
                if *terminal_exit_at > horizon_at {
                    return Err(ServiceError::EvidenceAfterHorizon {
                        unit_id: record.unit_id.clone(),
                    });
                }
            }
            accumulator
                .add("unresolved", record.quantity)
                .map_err(|_| ServiceError::QuantityOverflow {
                    cohort_id: record.cohort_id.clone(),
                    unit: record.unit.clone(),
                })?;
        }
        OutcomeEvidence::NotAcceptedByHorizon {
            independently_verified: true,
        } => {
            accumulator
                .add("adjudicated", record.quantity)
                .and_then(|_| accumulator.add("not_accepted", record.quantity))
                .map_err(|_| ServiceError::QuantityOverflow {
                    cohort_id: record.cohort_id.clone(),
                    unit: record.unit.clone(),
                })?;
        }
        OutcomeEvidence::NotAcceptedByHorizon {
            independently_verified: false,
        } => {
            accumulator
                .add("unresolved", record.quantity)
                .map_err(|_| ServiceError::QuantityOverflow {
                    cohort_id: record.cohort_id.clone(),
                    unit: record.unit.clone(),
                })?;
        }
        OutcomeEvidence::LostOrDamaged {
            observed_at,
            independently_verified: true,
        } => {
            if *observed_at > horizon_at {
                return Err(ServiceError::EvidenceAfterHorizon {
                    unit_id: record.unit_id.clone(),
                });
            }
            accumulator
                .add("adjudicated", record.quantity)
                .and_then(|_| accumulator.add("lost_or_damaged", record.quantity))
                .map_err(|_| ServiceError::QuantityOverflow {
                    cohort_id: record.cohort_id.clone(),
                    unit: record.unit.clone(),
                })?;
        }
        OutcomeEvidence::LostOrDamaged {
            independently_verified: false,
            ..
        } => {
            accumulator
                .add("unresolved", record.quantity)
                .map_err(|_| ServiceError::QuantityOverflow {
                    cohort_id: record.cohort_id.clone(),
                    unit: record.unit.clone(),
                })?;
        }
        OutcomeEvidence::UnitReconciliationFailure { .. } => {
            accumulator
                .add("reconciliation_failure", record.quantity)
                .map_err(|_| ServiceError::QuantityOverflow {
                    cohort_id: record.cohort_id.clone(),
                    unit: record.unit.clone(),
                })?;
        }
    }

    Ok(())
}

fn floor_disposition(
    accumulator: &Accumulator,
    floor_basis_points: Option<u16>,
) -> FloorDisposition {
    let Some(floor) = floor_basis_points else {
        return FloorDisposition::NotConfigured;
    };
    if accumulator.reconciliation_failure > 0 {
        return FloorDisposition::InvalidEvidence;
    }

    let lower = u128::from(accumulator.on_time) * BASIS_POINTS;
    let threshold = u128::from(accumulator.scheduled) * u128::from(floor);
    if lower >= threshold {
        return FloorDisposition::Pass;
    }

    let upper_quantity = accumulator.on_time.saturating_add(accumulator.unresolved);
    let upper = u128::from(upper_quantity) * BASIS_POINTS;
    if upper < threshold {
        FloorDisposition::Fail
    } else {
        FloorDisposition::Unknown
    }
}

/// Aggregates outcomes separately by (cohort_id, unit). No cross-unit
/// conversion or global averaging is performed. Only independently verified
/// receiver acceptance can enter the on-time numerator.
pub fn evaluate_receiver_service(
    records: &[CargoUnitOutcome],
    floors: &[ProtectedServiceFloor],
    horizon_at: i64,
) -> Result<ReceiverServiceReport, ServiceError> {
    let mut floor_map: BTreeMap<(String, String), u16> = BTreeMap::new();
    for floor in floors {
        if floor.cohort_id.is_empty() || floor.unit.is_empty() {
            return Err(ServiceError::EmptyIdentifier);
        }
        if floor.minimum_basis_points > 10_000 {
            return Err(ServiceError::InvalidFloor {
                cohort_id: floor.cohort_id.clone(),
                unit: floor.unit.clone(),
                basis_points: floor.minimum_basis_points,
            });
        }
        let key = (floor.cohort_id.clone(), floor.unit.clone());
        if floor_map
            .insert(key.clone(), floor.minimum_basis_points)
            .is_some()
        {
            return Err(ServiceError::DuplicateFloor {
                cohort_id: key.0,
                unit: key.1,
            });
        }
    }

    let mut unit_ids = BTreeSet::new();
    let mut groups: BTreeMap<(String, String), Accumulator> = BTreeMap::new();
    for record in records {
        if record.unit_id.is_empty() || record.cohort_id.is_empty() || record.unit.is_empty() {
            return Err(ServiceError::EmptyIdentifier);
        }
        if record.quantity == 0 {
            return Err(ServiceError::ZeroQuantity);
        }
        if !unit_ids.insert(record.unit_id.clone()) {
            return Err(ServiceError::DuplicateUnitId(record.unit_id.clone()));
        }

        let key = (record.cohort_id.clone(), record.unit.clone());
        let accumulator = groups.entry(key).or_default();
        classify(record, horizon_at, accumulator)?;
    }

    for (cohort_id, unit) in floor_map.keys() {
        if !groups.contains_key(&(cohort_id.clone(), unit.clone())) {
            return Err(ServiceError::FloorWithoutScheduledUnits {
                cohort_id: cohort_id.clone(),
                unit: unit.clone(),
            });
        }
    }

    let cohorts = groups
        .into_iter()
        .map(|((cohort_id, unit), accumulator)| {
            let floor = floor_map.get(&(cohort_id.clone(), unit.clone())).copied();
            let disposition = floor_disposition(&accumulator, floor);
            CohortServiceSummary {
                cohort_id,
                unit,
                scheduled_quantity: accumulator.scheduled,
                adjudicated_quantity: accumulator.adjudicated,
                on_time_quantity: accumulator.on_time,
                late_quantity: accumulator.late,
                not_accepted_quantity: accumulator.not_accepted,
                lost_or_damaged_quantity: accumulator.lost_or_damaged,
                unresolved_quantity: accumulator.unresolved,
                reconciliation_failure_quantity: accumulator.reconciliation_failure,
                on_time_observed_numerator: accumulator.on_time,
                on_time_observed_denominator: accumulator.adjudicated,
                conservative_lower_numerator: accumulator.on_time,
                conservative_lower_denominator: accumulator.scheduled,
                best_case_upper_numerator: accumulator
                    .on_time
                    .saturating_add(accumulator.unresolved),
                best_case_upper_denominator: accumulator.scheduled,
                floor_basis_points: floor,
                floor_disposition: disposition,
            }
        })
        .collect();

    Ok(ReceiverServiceReport {
        horizon_at,
        unit_count: unit_ids.len(),
        cohorts,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unit(
        id: &str,
        cohort: &str,
        quantity: u64,
        unit_name: &str,
        deadline: i64,
        evidence: OutcomeEvidence,
    ) -> CargoUnitOutcome {
        CargoUnitOutcome {
            unit_id: id.to_owned(),
            cohort_id: cohort.to_owned(),
            quantity,
            unit: unit_name.to_owned(),
            deadline_at: deadline,
            evidence,
        }
    }

    fn floor(cohort: &str, unit_name: &str, basis_points: u16) -> ProtectedServiceFloor {
        ProtectedServiceFloor {
            cohort_id: cohort.to_owned(),
            unit: unit_name.to_owned(),
            minimum_basis_points: basis_points,
        }
    }

    #[test]
    fn receiver_acceptance_is_the_only_on_time_numerator_source() {
        let records = vec![
            unit(
                "C1",
                "priority",
                1,
                "container",
                100,
                OutcomeEvidence::ReceiverAccepted {
                    accepted_at: 90,
                    independently_verified: true,
                },
            ),
            unit(
                "C2",
                "priority",
                1,
                "container",
                100,
                OutcomeEvidence::TerminalExitOnly {
                    terminal_exit_at: 80,
                },
            ),
        ];
        let report =
            evaluate_receiver_service(&records, &[floor("priority", "container", 5_000)], 120)
                .unwrap();
        let group = &report.cohorts[0];
        assert_eq!(group.on_time_quantity, 1);
        assert_eq!(group.scheduled_quantity, 2);
        assert_eq!(group.unresolved_quantity, 1);
        assert_eq!(group.floor_disposition, FloorDisposition::Pass);
    }

    #[test]
    fn terminal_exit_without_receiver_proof_remains_unresolved() {
        let records = vec![unit(
            "C1",
            "general",
            1,
            "container",
            100,
            OutcomeEvidence::TerminalExitOnly {
                terminal_exit_at: 80,
            },
        )];
        let report = evaluate_receiver_service(&records, &[], 120).unwrap();
        assert_eq!(report.cohorts[0].on_time_quantity, 0);
        assert_eq!(report.cohorts[0].unresolved_quantity, 1);
    }

    #[test]
    fn acceptance_after_deadline_is_adjudicated_late() {
        let records = vec![unit(
            "C1",
            "general",
            1,
            "container",
            100,
            OutcomeEvidence::ReceiverAccepted {
                accepted_at: 110,
                independently_verified: true,
            },
        )];
        let report = evaluate_receiver_service(&records, &[], 120).unwrap();
        assert_eq!(report.cohorts[0].late_quantity, 1);
        assert_eq!(report.cohorts[0].on_time_quantity, 0);
        assert_eq!(report.cohorts[0].adjudicated_quantity, 1);
    }

    #[test]
    fn unverified_acceptance_is_not_counted_as_delivery() {
        let records = vec![unit(
            "C1",
            "general",
            1,
            "container",
            100,
            OutcomeEvidence::ReceiverAccepted {
                accepted_at: 90,
                independently_verified: false,
            },
        )];
        let report = evaluate_receiver_service(&records, &[], 120).unwrap();
        assert_eq!(report.cohorts[0].on_time_quantity, 0);
        assert_eq!(report.cohorts[0].unresolved_quantity, 1);
    }

    #[test]
    fn censored_outcome_produces_conservative_bounds() {
        let records = vec![
            unit(
                "C1",
                "essential",
                6,
                "container",
                100,
                OutcomeEvidence::ReceiverAccepted {
                    accepted_at: 90,
                    independently_verified: true,
                },
            ),
            unit(
                "C2",
                "essential",
                4,
                "container",
                100,
                OutcomeEvidence::Censored {
                    last_observed_at: 95,
                },
            ),
        ];
        let report =
            evaluate_receiver_service(&records, &[floor("essential", "container", 8_000)], 120)
                .unwrap();
        let group = &report.cohorts[0];
        assert_eq!(group.conservative_lower_numerator, 6);
        assert_eq!(group.conservative_lower_denominator, 10);
        assert_eq!(group.best_case_upper_numerator, 10);
        assert_eq!(group.floor_disposition, FloorDisposition::Unknown);
    }

    #[test]
    fn verified_nonacceptance_causes_a_conservative_floor_failure() {
        let records = vec![
            unit(
                "C1",
                "essential",
                2,
                "container",
                100,
                OutcomeEvidence::ReceiverAccepted {
                    accepted_at: 90,
                    independently_verified: true,
                },
            ),
            unit(
                "C2",
                "essential",
                8,
                "container",
                100,
                OutcomeEvidence::NotAcceptedByHorizon {
                    independently_verified: true,
                },
            ),
        ];
        let report =
            evaluate_receiver_service(&records, &[floor("essential", "container", 5_000)], 120)
                .unwrap();
        assert_eq!(report.cohorts[0].floor_disposition, FloorDisposition::Fail);
    }

    #[test]
    fn unresolved_group_yields_unknown_not_failure() {
        let records = vec![unit(
            "C1",
            "essential",
            1,
            "container",
            100,
            OutcomeEvidence::Disputed,
        )];
        let report =
            evaluate_receiver_service(&records, &[floor("essential", "container", 5_000)], 120)
                .unwrap();
        assert_eq!(
            report.cohorts[0].floor_disposition,
            FloorDisposition::Unknown
        );
    }

    #[test]
    fn verified_lost_or_damaged_is_not_counted_as_potential_success() {
        let records = vec![
            unit(
                "C1",
                "essential",
                1,
                "container",
                100,
                OutcomeEvidence::LostOrDamaged {
                    observed_at: 80,
                    independently_verified: true,
                },
            ),
            unit(
                "C2",
                "essential",
                1,
                "container",
                100,
                OutcomeEvidence::Censored {
                    last_observed_at: 90,
                },
            ),
        ];
        let report =
            evaluate_receiver_service(&records, &[floor("essential", "container", 7_500)], 120)
                .unwrap();
        let group = &report.cohorts[0];
        assert_eq!(group.lost_or_damaged_quantity, 1);
        assert_eq!(group.best_case_upper_numerator, 1);
        assert_eq!(group.floor_disposition, FloorDisposition::Fail);
    }

    #[test]
    fn unit_types_are_never_aggregated_together() {
        let records = vec![
            unit(
                "C1",
                "essential",
                1,
                "container",
                100,
                OutcomeEvidence::ReceiverAccepted {
                    accepted_at: 90,
                    independently_verified: true,
                },
            ),
            unit(
                "T1",
                "essential",
                2,
                "TEU",
                100,
                OutcomeEvidence::NotAcceptedByHorizon {
                    independently_verified: true,
                },
            ),
        ];
        let report = evaluate_receiver_service(
            &records,
            &[
                floor("essential", "container", 10_000),
                floor("essential", "TEU", 0),
            ],
            120,
        )
        .unwrap();
        assert_eq!(report.cohorts.len(), 2);
        assert_ne!(report.cohorts[0].unit, report.cohorts[1].unit);
    }

    #[test]
    fn duplicate_unit_identity_is_rejected() {
        let records = vec![
            unit("C1", "a", 1, "container", 100, OutcomeEvidence::Disputed),
            unit("C1", "b", 1, "container", 100, OutcomeEvidence::Disputed),
        ];
        assert_eq!(
            evaluate_receiver_service(&records, &[], 120),
            Err(ServiceError::DuplicateUnitId("C1".to_owned()))
        );
    }

    #[test]
    fn reconciliation_failure_invalidates_protected_floor() {
        let records = vec![unit(
            "C1",
            "essential",
            1,
            "container",
            100,
            OutcomeEvidence::UnitReconciliationFailure {
                reason: "quantity mismatch".to_owned(),
            },
        )];
        let report =
            evaluate_receiver_service(&records, &[floor("essential", "container", 0)], 120)
                .unwrap();
        assert_eq!(
            report.cohorts[0].floor_disposition,
            FloorDisposition::InvalidEvidence
        );
    }

    #[test]
    fn floor_that_cannot_be_reached_even_with_all_unresolved_items_fails() {
        let records = vec![
            unit(
                "C1",
                "essential",
                1,
                "container",
                100,
                OutcomeEvidence::NotAcceptedByHorizon {
                    independently_verified: true,
                },
            ),
            unit(
                "C2",
                "essential",
                1,
                "container",
                100,
                OutcomeEvidence::Censored {
                    last_observed_at: 90,
                },
            ),
        ];
        let report =
            evaluate_receiver_service(&records, &[floor("essential", "container", 6_000)], 120)
                .unwrap();
        assert_eq!(report.cohorts[0].floor_disposition, FloorDisposition::Fail);
    }

    #[test]
    fn evidence_after_the_measurement_horizon_is_not_used_as_an_observation() {
        let records = vec![unit(
            "C1",
            "general",
            1,
            "container",
            100,
            OutcomeEvidence::Censored {
                last_observed_at: 130,
            },
        )];
        assert_eq!(
            evaluate_receiver_service(&records, &[], 120),
            Err(ServiceError::EvidenceAfterHorizon {
                unit_id: "C1".to_owned()
            })
        );
    }

    #[test]
    fn duplicate_floor_configuration_is_rejected() {
        let records = vec![unit(
            "C1",
            "general",
            1,
            "container",
            100,
            OutcomeEvidence::Disputed,
        )];
        let floors = vec![
            floor("general", "container", 5000),
            floor("general", "container", 6000),
        ];
        assert_eq!(
            evaluate_receiver_service(&records, &floors, 120),
            Err(ServiceError::DuplicateFloor {
                cohort_id: "general".to_owned(),
                unit: "container".to_owned(),
            })
        );
    }


    #[test]
    fn protected_floor_without_scheduled_units_is_rejected() {
        let records = vec![unit(
            "C1",
            "general",
            1,
            "container",
            100,
            OutcomeEvidence::Disputed,
        )];
        let floors = vec![floor("essential", "container", 5_000)];
        assert_eq!(
            evaluate_receiver_service(&records, &floors, 120),
            Err(ServiceError::FloorWithoutScheduledUnits {
                cohort_id: "essential".to_owned(),
                unit: "container".to_owned(),
            })
        );
    }

    #[test]
    fn quantity_overflow_is_rejected() {
        let records = vec![
            unit(
                "C1",
                "general",
                u64::MAX,
                "container",
                100,
                OutcomeEvidence::Disputed,
            ),
            unit(
                "C2",
                "general",
                1,
                "container",
                100,
                OutcomeEvidence::Disputed,
            ),
        ];
        assert!(matches!(
            evaluate_receiver_service(&records, &[], 120),
            Err(ServiceError::QuantityOverflow { .. })
        ));
    }
}
