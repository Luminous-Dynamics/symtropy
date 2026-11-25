// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Quantity-weighted receiver-service evaluation with explicit physical units.
//!
//! This module consumes canonical expected cargo-unit IDs from the lineage
//! owner and receiver outcomes from the service evidence layer. It does not
//! resolve aliases, infer weights, convert between physical unit systems, or
//! authenticate receiver receipts.
//!
//! Each evaluation uses exactly one quantity basis. Container count, TEU and
//! tonnes are reported separately and are never added into one opaque total.
//! Quantities are integer milli-units: 1_000 means one unit according to the
//! explicit unit enum.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as FmtWrite;

use crate::service_variability::{
    DeliveryDisposition, ServiceObservation, ServiceReliabilityReport,
};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ServiceQuantityUnit {
    Container,
    Teu,
    Tonne,
}

impl ServiceQuantityUnit {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Container => "CONTAINER",
            Self::Teu => "TEU",
            Self::Tonne => "TONNE",
        }
    }

    pub const fn milli_unit(self) -> &'static str {
        match self {
            Self::Container => "MILLI_CONTAINER",
            Self::Teu => "MILLI_TEU",
            Self::Tonne => "MILLI_TONNE",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExpectedUnitQuantity {
    /// Canonical physical cargo-unit ID supplied by the lineage owner.
    pub expected_unit_id: String,
    pub unit: ServiceQuantityUnit,
    /// Positive integer milli-units. Never inferred from a booking or source row.
    pub quantity_milli: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum QuantityWeightedError {
    EmptyObservations,
    EmptyQuantitySet,
    InvalidServiceObservations,
    EmptyExpectedUnitIdentity,
    DuplicateExpectedUnitIdentity,
    QuantityUnitMismatch,
    ZeroQuantity,
    DuplicateQuantityIdentity,
    MissingQuantityForExpectedUnit,
    QuantityForUnknownExpectedUnit,
    ReceiverAcceptanceEvidenceMissing,
    NonAcceptedOutcomeHasReceiverEvidence,
    QuantityAccumulatorOverflow,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QuantityWeightedServiceReport {
    pub unit: ServiceQuantityUnit,
    pub milli_unit: &'static str,
    pub expected_unit_count: usize,
    pub receiver_accepted_unit_count: usize,
    pub expected_quantity_milli: u128,
    pub receiver_accepted_quantity_milli: u128,
    pub verified_on_time_quantity_milli: u128,
    /// floor(verified on-time quantity / expected quantity × 1000).
    pub verified_on_time_rate_per_mille: u16,
}

/// Evaluates a quantity basis without mixing units or dropping expected cargo.
///
/// The exact canonical expected-unit set in the quantity vector must match the
/// observations one-for-one. Receiver-accepted observations need their
/// separately qualified receipt reference and elapsed time. Source-reported
/// delivered, inbound, not-delivered, diverted, cancelled and unknown states
/// remain in the denominator but not in the verified numerator.
pub fn evaluate_quantity_weighted_service(
    observations: &[ServiceObservation],
    quantities: &[ExpectedUnitQuantity],
) -> Result<QuantityWeightedServiceReport, QuantityWeightedError> {
    if observations.is_empty() {
        return Err(QuantityWeightedError::EmptyObservations);
    }
    if quantities.is_empty() {
        return Err(QuantityWeightedError::EmptyQuantitySet);
    }
    // Reuse the canonical service-row validator; do not accept a weaker
    // quantity-only subset of the source/cargo/receiver contract.
    ServiceReliabilityReport::evaluate(observations, 1)
        .map_err(|_| QuantityWeightedError::InvalidServiceObservations)?;

    let unit = quantities[0].unit;
    if quantities.iter().any(|quantity| quantity.unit != unit) {
        return Err(QuantityWeightedError::QuantityUnitMismatch);
    }

    let mut expected_ids = BTreeSet::new();
    for observation in observations {
        if observation.expected_unit_id.trim().is_empty() {
            return Err(QuantityWeightedError::EmptyExpectedUnitIdentity);
        }
        if !expected_ids.insert(observation.expected_unit_id.as_str()) {
            return Err(QuantityWeightedError::DuplicateExpectedUnitIdentity);
        }
        match observation.delivery_disposition {
            DeliveryDisposition::ReceiverAccepted => {
                if observation.receiver_delivery_minutes.is_none()
                    || observation
                        .receiver_acceptance_ref
                        .as_deref()
                        .is_none_or(|reference| reference.trim().is_empty())
                {
                    return Err(QuantityWeightedError::ReceiverAcceptanceEvidenceMissing);
                }
            }
            _ => {
                if observation.receiver_delivery_minutes.is_some()
                    || observation.receiver_acceptance_ref.is_some()
                {
                    return Err(QuantityWeightedError::NonAcceptedOutcomeHasReceiverEvidence);
                }
            }
        }
    }

    let mut by_unit = BTreeMap::<&str, u64>::new();
    for quantity in quantities {
        if quantity.expected_unit_id.trim().is_empty() {
            return Err(QuantityWeightedError::EmptyExpectedUnitIdentity);
        }
        if quantity.quantity_milli == 0 {
            return Err(QuantityWeightedError::ZeroQuantity);
        }
        if by_unit
            .insert(quantity.expected_unit_id.as_str(), quantity.quantity_milli)
            .is_some()
        {
            return Err(QuantityWeightedError::DuplicateQuantityIdentity);
        }
    }

    if by_unit.len() != expected_ids.len() {
        if expected_ids
            .iter()
            .any(|expected_id| !by_unit.contains_key(expected_id))
        {
            return Err(QuantityWeightedError::MissingQuantityForExpectedUnit);
        }
        return Err(QuantityWeightedError::QuantityForUnknownExpectedUnit);
    }
    if by_unit.keys().any(|quantity_id| !expected_ids.contains(quantity_id)) {
        return Err(QuantityWeightedError::QuantityForUnknownExpectedUnit);
    }

    let mut expected_quantity = 0_u128;
    let mut accepted_quantity = 0_u128;
    let mut on_time_quantity = 0_u128;
    let mut accepted_units = 0_usize;

    for observation in observations {
        let quantity = u128::from(
            *by_unit
                .get(observation.expected_unit_id.as_str())
                .ok_or(QuantityWeightedError::MissingQuantityForExpectedUnit)?,
        );
        expected_quantity = expected_quantity
            .checked_add(quantity)
            .ok_or(QuantityWeightedError::QuantityAccumulatorOverflow)?;

        if observation.delivery_disposition == DeliveryDisposition::ReceiverAccepted {
            accepted_units += 1;
            accepted_quantity = accepted_quantity
                .checked_add(quantity)
                .ok_or(QuantityWeightedError::QuantityAccumulatorOverflow)?;
            if observation.on_time() {
                on_time_quantity = on_time_quantity
                    .checked_add(quantity)
                    .ok_or(QuantityWeightedError::QuantityAccumulatorOverflow)?;
            }
        }
    }

    let rate = on_time_quantity
        .checked_mul(1_000)
        .ok_or(QuantityWeightedError::QuantityAccumulatorOverflow)?
        / expected_quantity;

    Ok(QuantityWeightedServiceReport {
        unit,
        milli_unit: unit.milli_unit(),
        expected_unit_count: observations.len(),
        receiver_accepted_unit_count: accepted_units,
        expected_quantity_milli: expected_quantity,
        receiver_accepted_quantity_milli: accepted_quantity,
        verified_on_time_quantity_milli: on_time_quantity,
        verified_on_time_rate_per_mille: rate as u16,
    })
}

fn synthetic_observations() -> Vec<ServiceObservation> {
    let accepted_on_time = ServiceObservation {
        observation_id: "source-record-1".to_owned(),
        expected_unit_id: "urn:example:box:20ft-ontime".to_owned(),
        cohort_id: "CRITICAL".to_owned(),
        vessel_wait_minutes: 10,
        berth_work_minutes: 20,
        terminal_dwell_minutes: 10,
        inland_transfer_minutes: 10,
        receiver_deadline_minutes: 100,
        receiver_delivery_minutes: Some(50),
        receiver_acceptance_ref: Some("receiver-receipt-1".to_owned()),
        delivery_disposition: DeliveryDisposition::ReceiverAccepted,
    };
    let mut accepted_late = accepted_on_time.clone();
    accepted_late.observation_id = "source-record-2".to_owned();
    accepted_late.expected_unit_id = "urn:example:box:40ft-late".to_owned();
    accepted_late.receiver_delivery_minutes = Some(150);
    accepted_late.receiver_acceptance_ref = Some("receiver-receipt-2".to_owned());

    let mut not_delivered = accepted_on_time.clone();
    not_delivered.observation_id = "source-record-3".to_owned();
    not_delivered.expected_unit_id = "urn:example:box:20ft-not-delivered".to_owned();
    not_delivered.receiver_delivery_minutes = None;
    not_delivered.receiver_acceptance_ref = None;
    not_delivered.delivery_disposition = DeliveryDisposition::NotDelivered;

    let mut unknown = accepted_on_time;
    unknown.observation_id = "source-record-4".to_owned();
    unknown.expected_unit_id = "urn:example:box:20ft-unknown".to_owned();
    unknown.receiver_delivery_minutes = None;
    unknown.receiver_acceptance_ref = None;
    unknown.delivery_disposition = DeliveryDisposition::Unknown;

    vec![unknown, not_delivered, accepted_late, synthetic_on_time()]
}

fn synthetic_on_time() -> ServiceObservation {
    ServiceObservation {
        observation_id: "source-record-1".to_owned(),
        expected_unit_id: "urn:example:box:20ft-ontime".to_owned(),
        cohort_id: "CRITICAL".to_owned(),
        vessel_wait_minutes: 10,
        berth_work_minutes: 20,
        terminal_dwell_minutes: 10,
        inland_transfer_minutes: 10,
        receiver_deadline_minutes: 100,
        receiver_delivery_minutes: Some(50),
        receiver_acceptance_ref: Some("receiver-receipt-1".to_owned()),
        delivery_disposition: DeliveryDisposition::ReceiverAccepted,
    }
}

fn synthetic_quantities(unit: ServiceQuantityUnit) -> Vec<ExpectedUnitQuantity> {
    let values = match unit {
        ServiceQuantityUnit::Container => [1_000, 1_000, 1_000, 1_000],
        ServiceQuantityUnit::Teu => [1_000, 2_000, 1_000, 1_000],
        ServiceQuantityUnit::Tonne => [10_000, 20_000, 5_000, 5_000],
    };
    let ids = [
        "urn:example:box:20ft-ontime",
        "urn:example:box:40ft-late",
        "urn:example:box:20ft-not-delivered",
        "urn:example:box:20ft-unknown",
    ];
    ids.into_iter()
        .zip(values)
        .map(|(expected_unit_id, quantity_milli)| ExpectedUnitQuantity {
            expected_unit_id: expected_unit_id.to_owned(),
            unit,
            quantity_milli,
        })
        .collect()
}

/// Synthetic known-answer receipt. The aggregate DGT 41-container report is
/// deliberately excluded because it has no per-unit IDs or quantity data.
pub fn render_csv() -> String {
    let observations = synthetic_observations();
    let mut output = String::from(
        "quantity_unit,expected_units,receiver_accepted_units,expected_quantity_milli,receiver_accepted_quantity_milli,verified_on_time_quantity_milli,verified_on_time_rate_per_mille,claim_ceiling\n",
    );
    for unit in [
        ServiceQuantityUnit::Container,
        ServiceQuantityUnit::Teu,
        ServiceQuantityUnit::Tonne,
    ] {
        let report = evaluate_quantity_weighted_service(
            &observations,
            &synthetic_quantities(unit),
        )
        .expect("valid quantity-weighted fixture");
        let _ = writeln!(
            output,
            "{},{},{},{},{},{},{},SYNTHETIC_ONLY",
            report.unit.label(),
            report.expected_unit_count,
            report.receiver_accepted_unit_count,
            report.expected_quantity_milli,
            report.receiver_accepted_quantity_milli,
            report.verified_on_time_quantity_milli,
            report.verified_on_time_rate_per_mille
        );
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn container_teu_and_tonne_service_rates_remain_separate() {
        let observations = synthetic_observations();
        let containers = evaluate_quantity_weighted_service(
            &observations,
            &synthetic_quantities(ServiceQuantityUnit::Container),
        )
        .expect("container-weighted");
        let teu = evaluate_quantity_weighted_service(
            &observations,
            &synthetic_quantities(ServiceQuantityUnit::Teu),
        )
        .expect("TEU-weighted");
        let tonnes = evaluate_quantity_weighted_service(
            &observations,
            &synthetic_quantities(ServiceQuantityUnit::Tonne),
        )
        .expect("tonne-weighted");

        assert_eq!(containers.expected_quantity_milli, 4_000);
        assert_eq!(containers.receiver_accepted_quantity_milli, 2_000);
        assert_eq!(containers.verified_on_time_quantity_milli, 1_000);
        assert_eq!(containers.verified_on_time_rate_per_mille, 250);

        assert_eq!(teu.expected_quantity_milli, 5_000);
        assert_eq!(teu.receiver_accepted_quantity_milli, 3_000);
        assert_eq!(teu.verified_on_time_quantity_milli, 1_000);
        assert_eq!(teu.verified_on_time_rate_per_mille, 200);

        assert_eq!(tonnes.expected_quantity_milli, 40_000);
        assert_eq!(tonnes.receiver_accepted_quantity_milli, 30_000);
        assert_eq!(tonnes.verified_on_time_quantity_milli, 10_000);
        assert_eq!(tonnes.verified_on_time_rate_per_mille, 250);
    }

    #[test]
    fn source_reported_delivered_without_receiver_receipt_is_not_weighted_as_accepted() {
        let mut observations = synthetic_observations();
        let on_time = observations
            .iter_mut()
            .find(|observation| observation.expected_unit_id.ends_with("ontime"))
            .expect("on-time expected box");
        on_time.delivery_disposition = DeliveryDisposition::SourceReportedDeliveredUnverified;
        on_time.receiver_delivery_minutes = None;
        on_time.receiver_acceptance_ref = None;
        let report = evaluate_quantity_weighted_service(
            &observations,
            &synthetic_quantities(ServiceQuantityUnit::Container),
        )
        .expect("source-reported delivery remains unverified");
        assert_eq!(report.receiver_accepted_unit_count, 1);
        assert_eq!(report.verified_on_time_quantity_milli, 0);
        assert_eq!(report.verified_on_time_rate_per_mille, 0);
    }

    #[test]
    fn mixed_quantity_units_are_rejected_not_converted() {
        let observations = synthetic_observations();
        let mut quantities = synthetic_quantities(ServiceQuantityUnit::Teu);
        quantities[0].unit = ServiceQuantityUnit::Tonne;
        assert_eq!(
            evaluate_quantity_weighted_service(&observations, &quantities),
            Err(QuantityWeightedError::QuantityUnitMismatch)
        );
    }

    #[test]
    fn quantity_map_must_exactly_match_expected_cargo_units() {
        let observations = synthetic_observations();
        let mut quantities = synthetic_quantities(ServiceQuantityUnit::Container);
        quantities.pop();
        assert_eq!(
            evaluate_quantity_weighted_service(&observations, &quantities),
            Err(QuantityWeightedError::MissingQuantityForExpectedUnit)
        );

        let mut quantities = synthetic_quantities(ServiceQuantityUnit::Container);
        quantities[3].expected_unit_id = "urn:example:box:extra".to_owned();
        assert_eq!(
            evaluate_quantity_weighted_service(&observations, &quantities),
            Err(QuantityWeightedError::MissingQuantityForExpectedUnit)
        );
    }

    #[test]
    fn duplicate_expected_or_quantity_ids_and_zero_quantities_are_rejected() {
        let mut observations = synthetic_observations();
        observations[1].expected_unit_id = observations[0].expected_unit_id.clone();
        assert_eq!(
            evaluate_quantity_weighted_service(
                &observations,
                &synthetic_quantities(ServiceQuantityUnit::Container),
            ),
            Err(QuantityWeightedError::DuplicateExpectedUnitIdentity)
        );

        let observations = synthetic_observations();
        let mut quantities = synthetic_quantities(ServiceQuantityUnit::Container);
        quantities[1].expected_unit_id = quantities[0].expected_unit_id.clone();
        assert_eq!(
            evaluate_quantity_weighted_service(&observations, &quantities),
            Err(QuantityWeightedError::DuplicateQuantityIdentity)
        );

        let mut quantities = synthetic_quantities(ServiceQuantityUnit::Container);
        quantities[0].quantity_milli = 0;
        assert_eq!(
            evaluate_quantity_weighted_service(&observations, &quantities),
            Err(QuantityWeightedError::ZeroQuantity)
        );
    }

    #[test]
    fn missing_receiver_receipt_blocks_weighted_acceptance() {
        let mut observations = synthetic_observations();
        let accepted = observations
            .iter_mut()
            .find(|observation| observation.delivery_disposition == DeliveryDisposition::ReceiverAccepted)
            .expect("accepted observation");
        accepted.receiver_acceptance_ref = None;
        assert_eq!(
            evaluate_quantity_weighted_service(
                &observations,
                &synthetic_quantities(ServiceQuantityUnit::Container),
            ),
            Err(QuantityWeightedError::ReceiverAcceptanceEvidenceMissing)
        );
    }

    #[test]
    fn deterministic_receipt_has_three_unit_systems_and_fixed_schema() {
        let first = render_csv();
        assert_eq!(first, render_csv());
        assert_eq!(first.lines().count(), 4);
        let width = first.lines().next().expect("header").split(',').count();
        assert_eq!(width, 8);
        for row in first.lines().skip(1) {
            assert_eq!(row.split(',').count(), width);
        }
        assert!(first.contains("CONTAINER"));
        assert!(first.contains("TEU"));
        assert!(first.contains("TONNE"));
    }
}
