// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Owner-controlled market-facing flexibility offers and dispatch authorization.
//!
//! This module is a local policy gate, not a market operator, protection
//! controller, SCADA/EMS interface, or actuator. External market messages may
//! reserve capability and request dispatch, but this module only emits a
//! bounded authorization receipt for a site-local controller to evaluate.
//! It never emits an equipment command.
//!
//! Cryptographic verification is injected through DispatchSignatureVerifier.
//! A production caller must provide an audited signature implementation and
//! durable key/replay-state management. This module does not implement
//! cryptography, persist replay state, or authenticate upstream qualification
//! receipt identifiers.

use core::fmt;
use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

const MAX_ID_BYTES: usize = 256;
const MAX_SIGNATURE_BYTES: usize = 4_096;
const MAX_QUALIFICATION_RECEIPTS: usize = 64;
const MAX_POLICY_ALLOWLIST_ITEMS: usize = 128;
const MAX_REPLAY_ENTRIES: usize = 65_536;
const MAX_RESERVATION_ENTRIES: usize = 65_536;

/// Direction is defined at the declared PCC grid-import boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, Serialize)]
pub enum GridFlexDirection {
    /// Reduce site grid import (upward system response in the common convention).
    ReduceGridImport,
    /// Increase site grid import (downward system response in the common convention).
    IncreaseGridImport,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub enum OwnerGuardStatus {
    Ready,
    Blocked,
    Unknown,
}

/// Local readiness snapshot supplied by the site-local trusted agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
pub struct OwnerLocalReadiness {
    last_pcc_telemetry_unix_seconds: i64,
    scheduler_guard: OwnerGuardStatus,
    physical_safety_guard: OwnerGuardStatus,
    local_controller_guard: OwnerGuardStatus,
}

impl OwnerLocalReadiness {
    pub const fn new(
        last_pcc_telemetry_unix_seconds: i64,
        scheduler_guard: OwnerGuardStatus,
        physical_safety_guard: OwnerGuardStatus,
        local_controller_guard: OwnerGuardStatus,
    ) -> Self {
        Self {
            last_pcc_telemetry_unix_seconds,
            scheduler_guard,
            physical_safety_guard,
            local_controller_guard,
        }
    }

    pub const fn last_pcc_telemetry_unix_seconds(&self) -> i64 {
        self.last_pcc_telemetry_unix_seconds
    }
}

/// Owner policy: allowed products, commitment budgets and message bounds.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(try_from = "OwnerFlexibilityPolicyWire")]
pub struct OwnerFlexibilityPolicy {
    policy_id: String,
    policy_version: String,
    site_id: String,
    pcc_boundary_id: String,
    portfolio_id: String,
    allowed_product_ids: BTreeSet<String>,
    authorized_signing_key_ids: BTreeSet<String>,
    maximum_concurrent_reserved_kw: f64,
    maximum_reduce_import_reserved_kw: f64,
    maximum_increase_import_reserved_kw: f64,
    maximum_response_latency_seconds: u64,
    maximum_sustain_seconds: u64,
    maximum_recovery_power_kw: f64,
    maximum_recovery_energy_kwh: f64,
    maximum_dispatch_ttl_seconds: u64,
    allowed_clock_skew_seconds: u64,
    maximum_telemetry_age_seconds: u64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct OwnerFlexibilityPolicyWire {
    policy_id: String,
    policy_version: String,
    site_id: String,
    pcc_boundary_id: String,
    portfolio_id: String,
    allowed_product_ids: BTreeSet<String>,
    authorized_signing_key_ids: BTreeSet<String>,
    maximum_concurrent_reserved_kw: f64,
    maximum_reduce_import_reserved_kw: f64,
    maximum_increase_import_reserved_kw: f64,
    maximum_response_latency_seconds: u64,
    maximum_sustain_seconds: u64,
    maximum_recovery_power_kw: f64,
    maximum_recovery_energy_kwh: f64,
    maximum_dispatch_ttl_seconds: u64,
    allowed_clock_skew_seconds: u64,
    maximum_telemetry_age_seconds: u64,
}

impl TryFrom<OwnerFlexibilityPolicyWire> for OwnerFlexibilityPolicy {
    type Error = OwnerFlexibilityError;

    fn try_from(wire: OwnerFlexibilityPolicyWire) -> Result<Self, Self::Error> {
        Self::new(
            wire.policy_id,
            wire.policy_version,
            wire.site_id,
            wire.pcc_boundary_id,
            wire.portfolio_id,
            wire.allowed_product_ids,
            wire.authorized_signing_key_ids,
            wire.maximum_concurrent_reserved_kw,
            wire.maximum_reduce_import_reserved_kw,
            wire.maximum_increase_import_reserved_kw,
            wire.maximum_response_latency_seconds,
            wire.maximum_sustain_seconds,
            wire.maximum_recovery_power_kw,
            wire.maximum_recovery_energy_kwh,
            wire.maximum_dispatch_ttl_seconds,
            wire.allowed_clock_skew_seconds,
            wire.maximum_telemetry_age_seconds,
        )
    }
}

impl OwnerFlexibilityPolicy {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        policy_id: impl Into<String>,
        policy_version: impl Into<String>,
        site_id: impl Into<String>,
        pcc_boundary_id: impl Into<String>,
        portfolio_id: impl Into<String>,
        allowed_product_ids: impl IntoIterator<Item = String>,
        authorized_signing_key_ids: impl IntoIterator<Item = String>,
        maximum_concurrent_reserved_kw: f64,
        maximum_reduce_import_reserved_kw: f64,
        maximum_increase_import_reserved_kw: f64,
        maximum_response_latency_seconds: u64,
        maximum_sustain_seconds: u64,
        maximum_recovery_power_kw: f64,
        maximum_recovery_energy_kwh: f64,
        maximum_dispatch_ttl_seconds: u64,
        allowed_clock_skew_seconds: u64,
        maximum_telemetry_age_seconds: u64,
    ) -> Result<Self, OwnerFlexibilityError> {
        let policy = Self {
            policy_id: policy_id.into(),
            policy_version: policy_version.into(),
            site_id: site_id.into(),
            pcc_boundary_id: pcc_boundary_id.into(),
            portfolio_id: portfolio_id.into(),
            allowed_product_ids: allowed_product_ids.into_iter().collect(),
            authorized_signing_key_ids: authorized_signing_key_ids.into_iter().collect(),
            maximum_concurrent_reserved_kw,
            maximum_reduce_import_reserved_kw,
            maximum_increase_import_reserved_kw,
            maximum_response_latency_seconds,
            maximum_sustain_seconds,
            maximum_recovery_power_kw,
            maximum_recovery_energy_kwh,
            maximum_dispatch_ttl_seconds,
            allowed_clock_skew_seconds,
            maximum_telemetry_age_seconds,
        };
        for (name, value) in [
            ("policy ID", policy.policy_id.as_str()),
            ("policy version", policy.policy_version.as_str()),
            ("site ID", policy.site_id.as_str()),
            ("PCC boundary ID", policy.pcc_boundary_id.as_str()),
            ("portfolio ID", policy.portfolio_id.as_str()),
        ] {
            require_id(name, value)?;
        }
        if policy.allowed_product_ids.is_empty()
            || policy.authorized_signing_key_ids.is_empty()
            || policy.allowed_product_ids.len() > MAX_POLICY_ALLOWLIST_ITEMS
            || policy.authorized_signing_key_ids.len() > MAX_POLICY_ALLOWLIST_ITEMS
            || policy.allowed_product_ids.iter().any(|x| x.trim().is_empty())
            || policy.authorized_signing_key_ids.iter().any(|x| x.trim().is_empty())
        {
            return Err(OwnerFlexibilityError::EmptyPolicyAllowlist);
        }
        for (name, value) in [
            ("shared reservation budget kW", maximum_concurrent_reserved_kw),
            ("reduce-import reservation budget kW", maximum_reduce_import_reserved_kw),
            ("increase-import reservation budget kW", maximum_increase_import_reserved_kw),
            ("maximum recovery power kW", maximum_recovery_power_kw),
            ("maximum recovery energy kWh", maximum_recovery_energy_kwh),
        ] {
            require_finite_non_negative(name, value)?;
        }
        let max_i64 = i64::MAX as u64;
        if maximum_concurrent_reserved_kw <= 0.0
            || maximum_dispatch_ttl_seconds == 0
            || maximum_telemetry_age_seconds == 0
            || maximum_sustain_seconds == 0
            || maximum_response_latency_seconds > max_i64
            || maximum_sustain_seconds > max_i64
            || maximum_dispatch_ttl_seconds > max_i64
            || allowed_clock_skew_seconds > max_i64
            || maximum_telemetry_age_seconds > max_i64
        {
            return Err(OwnerFlexibilityError::InvalidPolicyEnvelope);
        }
        if maximum_reduce_import_reserved_kw > maximum_concurrent_reserved_kw
            || maximum_increase_import_reserved_kw > maximum_concurrent_reserved_kw
        {
            return Err(OwnerFlexibilityError::DirectionalBudgetExceedsSharedBudget);
        }
        Ok(policy)
    }

    pub fn policy_id(&self) -> &str { &self.policy_id }
    pub fn policy_version(&self) -> &str { &self.policy_version }
    pub fn site_id(&self) -> &str { &self.site_id }
    pub fn pcc_boundary_id(&self) -> &str { &self.pcc_boundary_id }
    pub fn portfolio_id(&self) -> &str { &self.portfolio_id }
}

/// Market-facing offer derived from an already qualified capability envelope.
///
/// Qualification receipt identifiers are pinned but not authenticated here.
/// The offer is a promise envelope, not an execution command.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(try_from = "OwnerFlexibilityOfferWire")]
pub struct OwnerFlexibilityOffer {
    offer_id: String,
    portfolio_id: String,
    site_id: String,
    pcc_boundary_id: String,
    product_id: String,
    direction: GridFlexDirection,
    maximum_power_kw: f64,
    maximum_response_latency_seconds: u64,
    maximum_sustain_seconds: u64,
    maximum_recovery_power_kw: f64,
    maximum_recovery_energy_kwh: f64,
    valid_from_unix_seconds: i64,
    valid_until_unix_seconds: i64,
    qualification_receipt_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct OwnerFlexibilityOfferWire {
    offer_id: String,
    portfolio_id: String,
    site_id: String,
    pcc_boundary_id: String,
    product_id: String,
    direction: GridFlexDirection,
    maximum_power_kw: f64,
    maximum_response_latency_seconds: u64,
    maximum_sustain_seconds: u64,
    maximum_recovery_power_kw: f64,
    maximum_recovery_energy_kwh: f64,
    valid_from_unix_seconds: i64,
    valid_until_unix_seconds: i64,
    qualification_receipt_ids: Vec<String>,
}

impl TryFrom<OwnerFlexibilityOfferWire> for OwnerFlexibilityOffer {
    type Error = OwnerFlexibilityError;

    fn try_from(wire: OwnerFlexibilityOfferWire) -> Result<Self, Self::Error> {
        Self::new(
            wire.offer_id,
            wire.portfolio_id,
            wire.site_id,
            wire.pcc_boundary_id,
            wire.product_id,
            wire.direction,
            wire.maximum_power_kw,
            wire.maximum_response_latency_seconds,
            wire.maximum_sustain_seconds,
            wire.maximum_recovery_power_kw,
            wire.maximum_recovery_energy_kwh,
            wire.valid_from_unix_seconds,
            wire.valid_until_unix_seconds,
            wire.qualification_receipt_ids,
        )
    }
}

impl OwnerFlexibilityOffer {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        offer_id: impl Into<String>,
        portfolio_id: impl Into<String>,
        site_id: impl Into<String>,
        pcc_boundary_id: impl Into<String>,
        product_id: impl Into<String>,
        direction: GridFlexDirection,
        maximum_power_kw: f64,
        maximum_response_latency_seconds: u64,
        maximum_sustain_seconds: u64,
        maximum_recovery_power_kw: f64,
        maximum_recovery_energy_kwh: f64,
        valid_from_unix_seconds: i64,
        valid_until_unix_seconds: i64,
        qualification_receipt_ids: Vec<String>,
    ) -> Result<Self, OwnerFlexibilityError> {
        let offer = Self {
            offer_id: offer_id.into(),
            portfolio_id: portfolio_id.into(),
            site_id: site_id.into(),
            pcc_boundary_id: pcc_boundary_id.into(),
            product_id: product_id.into(),
            direction,
            maximum_power_kw,
            maximum_response_latency_seconds,
            maximum_sustain_seconds,
            maximum_recovery_power_kw,
            maximum_recovery_energy_kwh,
            valid_from_unix_seconds,
            valid_until_unix_seconds,
            qualification_receipt_ids,
        };
        for (name, value) in [
            ("offer ID", offer.offer_id.as_str()),
            ("portfolio ID", offer.portfolio_id.as_str()),
            ("site ID", offer.site_id.as_str()),
            ("PCC boundary ID", offer.pcc_boundary_id.as_str()),
            ("product ID", offer.product_id.as_str()),
        ] {
            require_id(name, value)?;
        }
        for (name, value) in [
            ("offer power kW", maximum_power_kw),
            ("offer recovery power kW", maximum_recovery_power_kw),
            ("offer recovery energy kWh", maximum_recovery_energy_kwh),
        ] {
            require_finite_non_negative(name, value)?;
        }
        if maximum_power_kw <= 0.0
            || maximum_sustain_seconds == 0
            || maximum_response_latency_seconds > i64::MAX as u64
            || maximum_sustain_seconds > i64::MAX as u64
            || valid_from_unix_seconds >= valid_until_unix_seconds
            || qualification_receipt_ids.is_empty()
            || qualification_receipt_ids.len() > MAX_QUALIFICATION_RECEIPTS
            || qualification_receipt_ids.iter().any(|id| id.trim().is_empty() || id.len() > MAX_ID_BYTES)
        {
            return Err(OwnerFlexibilityError::InvalidOfferEnvelope);
        }
        let unique = qualification_receipt_ids.iter().collect::<BTreeSet<_>>();
        if unique.len() != qualification_receipt_ids.len() {
            return Err(OwnerFlexibilityError::DuplicateQualificationReceipt);
        }
        Ok(offer)
    }

    pub fn offer_id(&self) -> &str { &self.offer_id }
    pub fn product_id(&self) -> &str { &self.product_id }
    pub fn portfolio_id(&self) -> &str { &self.portfolio_id }
    pub const fn direction(&self) -> GridFlexDirection { self.direction }
    pub const fn maximum_power_kw(&self) -> f64 { self.maximum_power_kw }
    pub const fn valid_from_unix_seconds(&self) -> i64 { self.valid_from_unix_seconds }
    pub const fn valid_until_unix_seconds(&self) -> i64 { self.valid_until_unix_seconds }
}

/// Owner-local ledger prevents overlapping product offers from reserving the
/// same shared portfolio MW more than once.
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
#[serde(try_from = "OwnerOfferReservationLedgerWire")]
pub struct OwnerOfferReservationLedger {
    reservations: Vec<OwnerOfferReservation>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(try_from = "OwnerOfferReservationWire")]
pub struct OwnerOfferReservation {
    pub offer_id: String,
    pub owner_policy_id: String,
    pub owner_policy_version: String,
    policy_snapshot: OwnerFlexibilityPolicy,
    pub portfolio_id: String,
    pub product_id: String,
    pub direction: GridFlexDirection,
    pub reserved_power_kw: f64,
    pub maximum_response_latency_seconds: u64,
    pub maximum_sustain_seconds: u64,
    pub maximum_recovery_power_kw: f64,
    pub maximum_recovery_energy_kwh: f64,
    pub valid_from_unix_seconds: i64,
    pub valid_until_unix_seconds: i64,
    pub qualification_receipt_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct OwnerOfferReservationWire {
    offer_id: String,
    owner_policy_id: String,
    owner_policy_version: String,
    policy_snapshot: OwnerFlexibilityPolicy,
    portfolio_id: String,
    product_id: String,
    direction: GridFlexDirection,
    reserved_power_kw: f64,
    maximum_response_latency_seconds: u64,
    maximum_sustain_seconds: u64,
    maximum_recovery_power_kw: f64,
    maximum_recovery_energy_kwh: f64,
    valid_from_unix_seconds: i64,
    valid_until_unix_seconds: i64,
    qualification_receipt_ids: Vec<String>,
}

impl TryFrom<OwnerOfferReservationWire> for OwnerOfferReservation {
    type Error = OwnerFlexibilityError;

    fn try_from(wire: OwnerOfferReservationWire) -> Result<Self, Self::Error> {
        let offer = OwnerFlexibilityOffer::new(
            wire.offer_id.clone(),
            wire.portfolio_id.clone(),
            wire.policy_snapshot.site_id.clone(),
            wire.policy_snapshot.pcc_boundary_id.clone(),
            wire.product_id.clone(),
            wire.direction,
            wire.reserved_power_kw,
            wire.maximum_response_latency_seconds,
            wire.maximum_sustain_seconds,
            wire.maximum_recovery_power_kw,
            wire.maximum_recovery_energy_kwh,
            wire.valid_from_unix_seconds,
            wire.valid_until_unix_seconds,
            wire.qualification_receipt_ids.clone(),
        )?;
        if wire.owner_policy_id != wire.policy_snapshot.policy_id
            || wire.owner_policy_version != wire.policy_snapshot.policy_version
        {
            return Err(OwnerFlexibilityError::InvalidPersistedReservation);
        }
        validate_offer_against_policy(&offer, &wire.policy_snapshot)?;
        Ok(Self {
            offer_id: wire.offer_id,
            owner_policy_id: wire.owner_policy_id,
            owner_policy_version: wire.owner_policy_version,
            policy_snapshot: wire.policy_snapshot,
            portfolio_id: wire.portfolio_id,
            product_id: wire.product_id,
            direction: wire.direction,
            reserved_power_kw: wire.reserved_power_kw,
            maximum_response_latency_seconds: wire.maximum_response_latency_seconds,
            maximum_sustain_seconds: wire.maximum_sustain_seconds,
            maximum_recovery_power_kw: wire.maximum_recovery_power_kw,
            maximum_recovery_energy_kwh: wire.maximum_recovery_energy_kwh,
            valid_from_unix_seconds: wire.valid_from_unix_seconds,
            valid_until_unix_seconds: wire.valid_until_unix_seconds,
            qualification_receipt_ids: wire.qualification_receipt_ids,
        })
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct OwnerOfferReservationLedgerWire {
    reservations: Vec<OwnerOfferReservation>,
}

impl TryFrom<OwnerOfferReservationLedgerWire> for OwnerOfferReservationLedger {
    type Error = OwnerFlexibilityError;

    fn try_from(wire: OwnerOfferReservationLedgerWire) -> Result<Self, Self::Error> {
        if wire.reservations.len() > MAX_RESERVATION_ENTRIES {
            return Err(OwnerFlexibilityError::ReservationLedgerCapacityExceeded);
        }
        let mut ledger = Self::default();
        for stored in wire.reservations {
            let offer = OwnerFlexibilityOffer::new(
                stored.offer_id.clone(),
                stored.portfolio_id.clone(),
                stored.policy_snapshot.site_id.clone(),
                stored.policy_snapshot.pcc_boundary_id.clone(),
                stored.product_id.clone(),
                stored.direction,
                stored.reserved_power_kw,
                stored.maximum_response_latency_seconds,
                stored.maximum_sustain_seconds,
                stored.maximum_recovery_power_kw,
                stored.maximum_recovery_energy_kwh,
                stored.valid_from_unix_seconds,
                stored.valid_until_unix_seconds,
                stored.qualification_receipt_ids.clone(),
            )?;
            let reconstructed = ledger.reserve_offer(&offer, &stored.policy_snapshot)?;
            if reconstructed != stored {
                return Err(OwnerFlexibilityError::InvalidPersistedReservation);
            }
        }
        Ok(ledger)
    }
}

impl OwnerOfferReservationLedger {
    pub fn reserve_offer(
        &mut self,
        offer: &OwnerFlexibilityOffer,
        policy: &OwnerFlexibilityPolicy,
    ) -> Result<OwnerOfferReservation, OwnerFlexibilityError> {
        validate_offer_against_policy(offer, policy)?;
        if self.reservations.iter().any(|r| r.offer_id == offer.offer_id) {
            return Err(OwnerFlexibilityError::OfferAlreadyReserved(offer.offer_id.clone()));
        }

        let mut change_points = vec![offer.valid_from_unix_seconds];
        for existing in &self.reservations {
            if existing.portfolio_id != offer.portfolio_id
                || existing.valid_until_unix_seconds <= offer.valid_from_unix_seconds
                || existing.valid_from_unix_seconds >= offer.valid_until_unix_seconds
            {
                continue;
            }
            if existing.valid_from_unix_seconds > offer.valid_from_unix_seconds
                && existing.valid_from_unix_seconds < offer.valid_until_unix_seconds
            {
                change_points.push(existing.valid_from_unix_seconds);
            }
        }
        change_points.sort_unstable();
        change_points.dedup();

        for point in change_points {
            let mut total = offer.maximum_power_kw;
            let mut reduce_import = if offer.direction == GridFlexDirection::ReduceGridImport {
                offer.maximum_power_kw
            } else { 0.0 };
            let mut increase_import = if offer.direction == GridFlexDirection::IncreaseGridImport {
                offer.maximum_power_kw
            } else { 0.0 };
            for existing in &self.reservations {
                if existing.portfolio_id == offer.portfolio_id
                    && existing.valid_from_unix_seconds <= point
                    && point < existing.valid_until_unix_seconds
                {
                    total += existing.reserved_power_kw;
                    match existing.direction {
                        GridFlexDirection::ReduceGridImport => reduce_import += existing.reserved_power_kw,
                        GridFlexDirection::IncreaseGridImport => increase_import += existing.reserved_power_kw,
                    }
                }
            }
            if !total.is_finite() || !reduce_import.is_finite() || !increase_import.is_finite() {
                return Err(OwnerFlexibilityError::NumericalOverflow("offer reservation aggregation"));
            }
            if total > policy.maximum_concurrent_reserved_kw {
                return Err(OwnerFlexibilityError::SharedCapacityOvercommitted {
                    observed_kw: total,
                    maximum_kw: policy.maximum_concurrent_reserved_kw,
                });
            }
            if reduce_import > policy.maximum_reduce_import_reserved_kw {
                return Err(OwnerFlexibilityError::DirectionalCapacityOvercommitted {
                    direction: GridFlexDirection::ReduceGridImport,
                    observed_kw: reduce_import,
                    maximum_kw: policy.maximum_reduce_import_reserved_kw,
                });
            }
            if increase_import > policy.maximum_increase_import_reserved_kw {
                return Err(OwnerFlexibilityError::DirectionalCapacityOvercommitted {
                    direction: GridFlexDirection::IncreaseGridImport,
                    observed_kw: increase_import,
                    maximum_kw: policy.maximum_increase_import_reserved_kw,
                });
            }
        }

        if self.reservations.len() >= MAX_RESERVATION_ENTRIES {
            return Err(OwnerFlexibilityError::ReservationLedgerCapacityExceeded);
        }
        let reservation = OwnerOfferReservation {
            offer_id: offer.offer_id.clone(),
            owner_policy_id: policy.policy_id.clone(),
            owner_policy_version: policy.policy_version.clone(),
            policy_snapshot: policy.clone(),
            portfolio_id: offer.portfolio_id.clone(),
            product_id: offer.product_id.clone(),
            direction: offer.direction,
            reserved_power_kw: offer.maximum_power_kw,
            maximum_response_latency_seconds: offer.maximum_response_latency_seconds,
            maximum_sustain_seconds: offer.maximum_sustain_seconds,
            maximum_recovery_power_kw: offer.maximum_recovery_power_kw,
            maximum_recovery_energy_kwh: offer.maximum_recovery_energy_kwh,
            valid_from_unix_seconds: offer.valid_from_unix_seconds,
            valid_until_unix_seconds: offer.valid_until_unix_seconds,
            qualification_receipt_ids: offer.qualification_receipt_ids.clone(),
        };
        self.reservations.push(reservation.clone());
        Ok(reservation)
    }

    pub fn release_offer(&mut self, offer_id: &str) -> bool {
        let before = self.reservations.len();
        self.reservations.retain(|r| r.offer_id != offer_id);
        self.reservations.len() != before
    }

    pub fn contains_offer(&self, offer_id: &str) -> bool {
        self.reservations.iter().any(|r| r.offer_id == offer_id)
    }

    /// Checks that authorization is bound to the exact terms that were reserved.
    pub fn matches_offer(
        &self,
        offer: &OwnerFlexibilityOffer,
        policy: &OwnerFlexibilityPolicy,
    ) -> bool {
        self.reservations.iter().any(|reservation| {
            reservation.owner_policy_id == policy.policy_id
                && reservation.owner_policy_version == policy.policy_version
                && reservation.policy_snapshot == *policy
                && reservation.offer_id == offer.offer_id
                && reservation.portfolio_id == offer.portfolio_id
                && reservation.product_id == offer.product_id
                && reservation.direction == offer.direction
                && reservation.reserved_power_kw.to_bits() == offer.maximum_power_kw.to_bits()
                && reservation.maximum_response_latency_seconds == offer.maximum_response_latency_seconds
                && reservation.maximum_sustain_seconds == offer.maximum_sustain_seconds
                && reservation.maximum_recovery_power_kw.to_bits() == offer.maximum_recovery_power_kw.to_bits()
                && reservation.maximum_recovery_energy_kwh.to_bits() == offer.maximum_recovery_energy_kwh.to_bits()
                && reservation.valid_from_unix_seconds == offer.valid_from_unix_seconds
                && reservation.valid_until_unix_seconds == offer.valid_until_unix_seconds
                && reservation.qualification_receipt_ids == offer.qualification_receipt_ids
        })
    }

    pub fn reservations(&self) -> &[OwnerOfferReservation] {
        &self.reservations
    }
}

/// Typed dispatch request: no arbitrary script, setpoint, or equipment address.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(try_from = "OwnerDispatchRequestWire")]
pub struct OwnerDispatchRequest {
    dispatch_id: String,
    offer_id: String,
    product_id: String,
    site_id: String,
    pcc_boundary_id: String,
    direction: GridFlexDirection,
    requested_power_kw: f64,
    requested_sustain_seconds: u64,
    declared_recovery_power_kw: f64,
    declared_recovery_energy_kwh: f64,
    issued_at_unix_seconds: i64,
    expires_at_unix_seconds: i64,
    response_deadline_unix_seconds: i64,
    sequence_number: u64,
    nonce: String,
    signing_key_id: String,
    signature: Vec<u8>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct OwnerDispatchRequestWire {
    dispatch_id: String,
    offer_id: String,
    product_id: String,
    site_id: String,
    pcc_boundary_id: String,
    direction: GridFlexDirection,
    requested_power_kw: f64,
    requested_sustain_seconds: u64,
    declared_recovery_power_kw: f64,
    declared_recovery_energy_kwh: f64,
    issued_at_unix_seconds: i64,
    expires_at_unix_seconds: i64,
    response_deadline_unix_seconds: i64,
    sequence_number: u64,
    nonce: String,
    signing_key_id: String,
    signature: Vec<u8>,
}

impl TryFrom<OwnerDispatchRequestWire> for OwnerDispatchRequest {
    type Error = OwnerFlexibilityError;

    fn try_from(wire: OwnerDispatchRequestWire) -> Result<Self, Self::Error> {
        Self::new(
            wire.dispatch_id,
            wire.offer_id,
            wire.product_id,
            wire.site_id,
            wire.pcc_boundary_id,
            wire.direction,
            wire.requested_power_kw,
            wire.requested_sustain_seconds,
            wire.declared_recovery_power_kw,
            wire.declared_recovery_energy_kwh,
            wire.issued_at_unix_seconds,
            wire.expires_at_unix_seconds,
            wire.response_deadline_unix_seconds,
            wire.sequence_number,
            wire.nonce,
            wire.signing_key_id,
            wire.signature,
        )
    }
}

impl OwnerDispatchRequest {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        dispatch_id: impl Into<String>,
        offer_id: impl Into<String>,
        product_id: impl Into<String>,
        site_id: impl Into<String>,
        pcc_boundary_id: impl Into<String>,
        direction: GridFlexDirection,
        requested_power_kw: f64,
        requested_sustain_seconds: u64,
        declared_recovery_power_kw: f64,
        declared_recovery_energy_kwh: f64,
        issued_at_unix_seconds: i64,
        expires_at_unix_seconds: i64,
        response_deadline_unix_seconds: i64,
        sequence_number: u64,
        nonce: impl Into<String>,
        signing_key_id: impl Into<String>,
        signature: Vec<u8>,
    ) -> Result<Self, OwnerFlexibilityError> {
        let request = Self {
            dispatch_id: dispatch_id.into(),
            offer_id: offer_id.into(),
            product_id: product_id.into(),
            site_id: site_id.into(),
            pcc_boundary_id: pcc_boundary_id.into(),
            direction,
            requested_power_kw,
            requested_sustain_seconds,
            declared_recovery_power_kw,
            declared_recovery_energy_kwh,
            issued_at_unix_seconds,
            expires_at_unix_seconds,
            response_deadline_unix_seconds,
            sequence_number,
            nonce: nonce.into(),
            signing_key_id: signing_key_id.into(),
            signature,
        };
        for (name, value) in [
            ("dispatch ID", request.dispatch_id.as_str()),
            ("offer ID", request.offer_id.as_str()),
            ("product ID", request.product_id.as_str()),
            ("site ID", request.site_id.as_str()),
            ("PCC boundary ID", request.pcc_boundary_id.as_str()),
            ("nonce", request.nonce.as_str()),
            ("signing key ID", request.signing_key_id.as_str()),
        ] {
            require_id(name, value)?;
        }
        for (name, value) in [
            ("requested dispatch power kW", requested_power_kw),
            ("declared recovery power kW", declared_recovery_power_kw),
            ("declared recovery energy kWh", declared_recovery_energy_kwh),
        ] {
            require_finite_non_negative(name, value)?;
        }
        if requested_power_kw <= 0.0
            || requested_sustain_seconds == 0
            || requested_sustain_seconds > i64::MAX as u64
            || sequence_number == 0
            || expires_at_unix_seconds <= issued_at_unix_seconds
            || response_deadline_unix_seconds < issued_at_unix_seconds
            || response_deadline_unix_seconds > expires_at_unix_seconds
            || signature.is_empty()
            || signature.len() > MAX_SIGNATURE_BYTES
        {
            return Err(OwnerFlexibilityError::InvalidDispatchMessage);
        }
        Ok(request)
    }

    /// Stable length-prefixed encoding. Signatures bind these bytes, not JSON
    /// serialization whose field ordering and whitespace may vary.
    pub fn canonical_payload(&self) -> Vec<u8> {
        let mut bytes = b"SYMTROPY-OWNER-DISPATCH-V1\0".to_vec();
        push_string(&mut bytes, &self.dispatch_id);
        push_string(&mut bytes, &self.offer_id);
        push_string(&mut bytes, &self.product_id);
        push_string(&mut bytes, &self.site_id);
        push_string(&mut bytes, &self.pcc_boundary_id);
        bytes.push(match self.direction {
            GridFlexDirection::ReduceGridImport => 1,
            GridFlexDirection::IncreaseGridImport => 2,
        });
        bytes.extend_from_slice(&self.requested_power_kw.to_bits().to_le_bytes());
        bytes.extend_from_slice(&self.requested_sustain_seconds.to_le_bytes());
        bytes.extend_from_slice(&self.declared_recovery_power_kw.to_bits().to_le_bytes());
        bytes.extend_from_slice(&self.declared_recovery_energy_kwh.to_bits().to_le_bytes());
        bytes.extend_from_slice(&self.issued_at_unix_seconds.to_le_bytes());
        bytes.extend_from_slice(&self.expires_at_unix_seconds.to_le_bytes());
        bytes.extend_from_slice(&self.response_deadline_unix_seconds.to_le_bytes());
        bytes.extend_from_slice(&self.sequence_number.to_le_bytes());
        push_string(&mut bytes, &self.nonce);
        push_string(&mut bytes, &self.signing_key_id);
        bytes
    }
}

fn push_string(bytes: &mut Vec<u8>, value: &str) {
    let raw = value.as_bytes();
    bytes.extend_from_slice(&(raw.len() as u64).to_le_bytes());
    bytes.extend_from_slice(raw);
}

/// Production callers must inject an audited cryptographic verifier.
pub trait DispatchSignatureVerifier {
    fn verify(&self, signing_key_id: &str, canonical_payload: &[u8], signature: &[u8]) -> bool;
}

/// Monotonic replay ledger. Persist it across agent restarts in a trusted store;
/// an in-memory instance protects only the lifetime of that process.
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
#[serde(try_from = "DispatchReplayLedgerWire")]
pub struct DispatchReplayLedger {
    per_offer: BTreeMap<String, DispatchReplayState>,
    seen_key_nonces: BTreeSet<(String, String)>,
    seen_dispatch_ids: BTreeSet<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
struct DispatchReplayState {
    highest_sequence_number: u64,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DispatchReplayLedgerWire {
    per_offer: BTreeMap<String, DispatchReplayState>,
    seen_key_nonces: BTreeSet<(String, String)>,
    seen_dispatch_ids: BTreeSet<String>,
}

impl TryFrom<DispatchReplayLedgerWire> for DispatchReplayLedger {
    type Error = OwnerFlexibilityError;

    fn try_from(wire: DispatchReplayLedgerWire) -> Result<Self, Self::Error> {
        if wire.per_offer.len() > MAX_REPLAY_ENTRIES
            || wire.seen_key_nonces.len() > MAX_REPLAY_ENTRIES
            || wire.seen_dispatch_ids.len() > MAX_REPLAY_ENTRIES
            || wire.seen_key_nonces.len() != wire.seen_dispatch_ids.len()
            || wire.per_offer.len() > wire.seen_key_nonces.len()
        {
            return Err(OwnerFlexibilityError::InvalidPersistedReplayState);
        }
        for (offer_id, state) in &wire.per_offer {
            require_id("replay offer ID", offer_id)?;
            if state.highest_sequence_number == 0 {
                return Err(OwnerFlexibilityError::InvalidPersistedReplayState);
            }
        }
        for (key_id, nonce) in &wire.seen_key_nonces {
            require_id("replay signing key ID", key_id)?;
            require_id("replay nonce", nonce)?;
        }
        for dispatch_id in &wire.seen_dispatch_ids {
            require_id("replay dispatch ID", dispatch_id)?;
        }
        if wire.seen_key_nonces.is_empty() != wire.per_offer.is_empty()
            || wire.seen_dispatch_ids.is_empty() != wire.per_offer.is_empty()
        {
            return Err(OwnerFlexibilityError::InvalidPersistedReplayState);
        }
        Ok(Self {
            per_offer: wire.per_offer,
            seen_key_nonces: wire.seen_key_nonces,
            seen_dispatch_ids: wire.seen_dispatch_ids,
        })
    }
}

impl DispatchReplayLedger {
    fn observe_authenticated(&mut self, request: &OwnerDispatchRequest) -> Result<(), DispatchReplayViolation> {
        let highest_sequence = self
            .per_offer
            .get(&request.offer_id)
            .map_or(0, |state| state.highest_sequence_number);
        if request.sequence_number <= highest_sequence {
            return Err(DispatchReplayViolation::NonIncreasingSequence);
        }
        let nonce_key = (request.signing_key_id.clone(), request.nonce.clone());
        if self.seen_key_nonces.contains(&nonce_key) {
            return Err(DispatchReplayViolation::ReusedNonce);
        }
        if self.seen_dispatch_ids.contains(&request.dispatch_id) {
            return Err(DispatchReplayViolation::ReusedDispatchId);
        }
        if self.seen_key_nonces.len() >= MAX_REPLAY_ENTRIES
            || self.seen_dispatch_ids.len() >= MAX_REPLAY_ENTRIES
        {
            return Err(DispatchReplayViolation::LedgerCapacityExceeded);
        }
        let state = self.per_offer.entry(request.offer_id.clone()).or_default();
        state.highest_sequence_number = request.sequence_number;
        self.seen_key_nonces.insert(nonce_key);
        self.seen_dispatch_ids.insert(request.dispatch_id.clone());
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DispatchReplayViolation {
    NonIncreasingSequence,
    ReusedNonce,
    ReusedDispatchId,
    LedgerCapacityExceeded,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
pub enum OwnerDispatchRejectionReason {
    OfferNotReserved,
    UnauthorizedSigningKey,
    InvalidSignature,
    ReplayedDispatch,
    ReplayLedgerCapacityExceeded,
    DispatchIssuedInFuture,
    DispatchExpired,
    DispatchTtlExceeded,
    ResponseDeadlineMissed,
    OfferOutsideValidityWindow,
    ProductNotAllowed,
    OfferIdentityMismatch,
    DirectionMismatch,
    RequestedPowerExceedsOffer,
    SustainDurationExceedsOffer,
    ResponseDeadlineExceedsOffer,
    RecoveryEnvelopeExceeded,
    StalePccTelemetry,
    FuturePccTelemetry,
    SchedulerGuardNotReady,
    PhysicalSafetyGuardNotReady,
    LocalControllerNotReady,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct OwnerDispatchAuthorizationReceipt {
    pub dispatch_id: String,
    pub offer_id: String,
    pub product_id: String,
    pub policy_id: String,
    pub policy_version: String,
    pub qualification_receipt_ids: Vec<String>,
    pub site_id: String,
    pub pcc_boundary_id: String,
    pub direction: GridFlexDirection,
    pub authorized_power_kw: f64,
    pub authorized_sustain_seconds: u64,
    pub authorized_at_unix_seconds: i64,
    pub sequence_number: u64,
    pub nonce: String,
    pub signing_key_id: String,
    pub canonical_payload: Vec<u8>,
    pub signature: Vec<u8>,
    pub decision_scope: String,
    /// True: only a site-local controller may consider execution.
    pub local_controller_only: bool,
    /// Always false: this module is not an actuator.
    pub actuator_command_emitted: bool,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub enum OwnerDispatchDecision {
    Authorized(OwnerDispatchAuthorizationReceipt),
    Rejected {
        dispatch_id: String,
        reason: OwnerDispatchRejectionReason,
    },
}

/// Authenticate and apply local owner policy to a dispatch message.
///
/// Authenticated requests consume their sequence/nonce even when later denied,
/// preventing a policy-rejected message from being replayed after state changes.
/// Invalid signatures do not mutate replay state.
pub fn authorize_owner_dispatch(
    request: &OwnerDispatchRequest,
    offer: &OwnerFlexibilityOffer,
    policy: &OwnerFlexibilityPolicy,
    reservations: &OwnerOfferReservationLedger,
    replay_ledger: &mut DispatchReplayLedger,
    readiness: OwnerLocalReadiness,
    now_unix_seconds: i64,
    verifier: &impl DispatchSignatureVerifier,
) -> OwnerDispatchDecision {
    let reject = |reason| OwnerDispatchDecision::Rejected {
        dispatch_id: request.dispatch_id.clone(),
        reason,
    };

    if !reservations.matches_offer(offer, policy) {
        return reject(OwnerDispatchRejectionReason::OfferNotReserved);
    }
    if !policy.authorized_signing_key_ids.contains(&request.signing_key_id) {
        return reject(OwnerDispatchRejectionReason::UnauthorizedSigningKey);
    }
    let canonical_payload = request.canonical_payload();
    if !verifier.verify(&request.signing_key_id, &canonical_payload, &request.signature) {
        return reject(OwnerDispatchRejectionReason::InvalidSignature);
    }
    match replay_ledger.observe_authenticated(request) {
        Ok(()) => {}
        Err(DispatchReplayViolation::LedgerCapacityExceeded) => {
            return reject(OwnerDispatchRejectionReason::ReplayLedgerCapacityExceeded);
        }
        Err(_) => return reject(OwnerDispatchRejectionReason::ReplayedDispatch),
    }

    if request.issued_at_unix_seconds
        > now_unix_seconds.saturating_add(policy.allowed_clock_skew_seconds as i64)
    {
        return reject(OwnerDispatchRejectionReason::DispatchIssuedInFuture);
    }
    if now_unix_seconds >= request.expires_at_unix_seconds {
        return reject(OwnerDispatchRejectionReason::DispatchExpired);
    }
    if now_unix_seconds > request.response_deadline_unix_seconds {
        return reject(OwnerDispatchRejectionReason::ResponseDeadlineMissed);
    }
    if request.expires_at_unix_seconds
        .saturating_sub(request.issued_at_unix_seconds)
        > policy.maximum_dispatch_ttl_seconds as i64
    {
        return reject(OwnerDispatchRejectionReason::DispatchTtlExceeded);
    }
    if now_unix_seconds < offer.valid_from_unix_seconds
        || now_unix_seconds >= offer.valid_until_unix_seconds
        || request.issued_at_unix_seconds < offer.valid_from_unix_seconds
        || request.expires_at_unix_seconds > offer.valid_until_unix_seconds
    {
        return reject(OwnerDispatchRejectionReason::OfferOutsideValidityWindow);
    }
    if !policy.allowed_product_ids.contains(&request.product_id) {
        return reject(OwnerDispatchRejectionReason::ProductNotAllowed);
    }
    if offer.product_id != request.product_id
        || offer.offer_id != request.offer_id
        || offer.site_id != request.site_id
        || offer.pcc_boundary_id != request.pcc_boundary_id
        || offer.site_id != policy.site_id
        || offer.pcc_boundary_id != policy.pcc_boundary_id
        || offer.portfolio_id != policy.portfolio_id
    {
        return reject(OwnerDispatchRejectionReason::OfferIdentityMismatch);
    }
    if offer.direction != request.direction {
        return reject(OwnerDispatchRejectionReason::DirectionMismatch);
    }
    if request.requested_power_kw > offer.maximum_power_kw {
        return reject(OwnerDispatchRejectionReason::RequestedPowerExceedsOffer);
    }
    if request.requested_sustain_seconds > offer.maximum_sustain_seconds
        || request.requested_sustain_seconds > policy.maximum_sustain_seconds
    {
        return reject(OwnerDispatchRejectionReason::SustainDurationExceedsOffer);
    }
    if request.response_deadline_unix_seconds
        .saturating_sub(request.issued_at_unix_seconds)
        > offer.maximum_response_latency_seconds as i64
        || offer.maximum_response_latency_seconds > policy.maximum_response_latency_seconds
    {
        return reject(OwnerDispatchRejectionReason::ResponseDeadlineExceedsOffer);
    }
    if request.declared_recovery_power_kw > offer.maximum_recovery_power_kw
        || request.declared_recovery_power_kw > policy.maximum_recovery_power_kw
        || request.declared_recovery_energy_kwh > offer.maximum_recovery_energy_kwh
        || request.declared_recovery_energy_kwh > policy.maximum_recovery_energy_kwh
    {
        return reject(OwnerDispatchRejectionReason::RecoveryEnvelopeExceeded);
    }
    if readiness.last_pcc_telemetry_unix_seconds
        > now_unix_seconds.saturating_add(policy.allowed_clock_skew_seconds as i64)
    {
        return reject(OwnerDispatchRejectionReason::FuturePccTelemetry);
    }
    if now_unix_seconds.saturating_sub(readiness.last_pcc_telemetry_unix_seconds)
        > policy.maximum_telemetry_age_seconds as i64
    {
        return reject(OwnerDispatchRejectionReason::StalePccTelemetry);
    }
    if readiness.scheduler_guard != OwnerGuardStatus::Ready {
        return reject(OwnerDispatchRejectionReason::SchedulerGuardNotReady);
    }
    if readiness.physical_safety_guard != OwnerGuardStatus::Ready {
        return reject(OwnerDispatchRejectionReason::PhysicalSafetyGuardNotReady);
    }
    if readiness.local_controller_guard != OwnerGuardStatus::Ready {
        return reject(OwnerDispatchRejectionReason::LocalControllerNotReady);
    }

    OwnerDispatchDecision::Authorized(OwnerDispatchAuthorizationReceipt {
        dispatch_id: request.dispatch_id.clone(),
        offer_id: request.offer_id.clone(),
        product_id: request.product_id.clone(),
        policy_id: policy.policy_id.clone(),
        policy_version: policy.policy_version.clone(),
        qualification_receipt_ids: offer.qualification_receipt_ids.clone(),
        site_id: request.site_id.clone(),
        pcc_boundary_id: request.pcc_boundary_id.clone(),
        direction: request.direction,
        authorized_power_kw: request.requested_power_kw,
        authorized_sustain_seconds: request.requested_sustain_seconds,
        authorized_at_unix_seconds: now_unix_seconds,
        sequence_number: request.sequence_number,
        nonce: request.nonce.clone(),
        signing_key_id: request.signing_key_id.clone(),
        canonical_payload,
        signature: request.signature.clone(),
        decision_scope: "OWNER_POLICY_AND_CAPABILITY_ENVELOPE_ACCEPTED_LOCAL_CONTROLLER_MUST_STILL_CHECK_CURRENT_PHYSICAL_STATE_AND_MAY_DECLINE".to_string(),
        local_controller_only: true,
        actuator_command_emitted: false,
    })
}

fn validate_offer_against_policy(
    offer: &OwnerFlexibilityOffer,
    policy: &OwnerFlexibilityPolicy,
) -> Result<(), OwnerFlexibilityError> {
    if offer.site_id != policy.site_id
        || offer.pcc_boundary_id != policy.pcc_boundary_id
        || offer.portfolio_id != policy.portfolio_id
    {
        return Err(OwnerFlexibilityError::OfferPolicyIdentityMismatch);
    }
    if !policy.allowed_product_ids.contains(&offer.product_id) {
        return Err(OwnerFlexibilityError::ProductNotAllowed(offer.product_id.clone()));
    }
    if offer.maximum_response_latency_seconds > policy.maximum_response_latency_seconds
        || offer.maximum_sustain_seconds > policy.maximum_sustain_seconds
        || offer.maximum_recovery_power_kw > policy.maximum_recovery_power_kw
        || offer.maximum_recovery_energy_kwh > policy.maximum_recovery_energy_kwh
    {
        return Err(OwnerFlexibilityError::OfferExceedsOwnerPolicy);
    }
    let directional_budget = match offer.direction {
        GridFlexDirection::ReduceGridImport => policy.maximum_reduce_import_reserved_kw,
        GridFlexDirection::IncreaseGridImport => policy.maximum_increase_import_reserved_kw,
    };
    if offer.maximum_power_kw > directional_budget
        || offer.maximum_power_kw > policy.maximum_concurrent_reserved_kw
    {
        return Err(OwnerFlexibilityError::OfferExceedsOwnerPolicy);
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq)]
pub enum OwnerFlexibilityError {
    MissingIdentity { name: &'static str },
    IdentityTooLong { name: &'static str, maximum_bytes: usize, observed_bytes: usize },
    InvalidQuantity { name: &'static str, value: f64 },
    EmptyPolicyAllowlist,
    InvalidPolicyEnvelope,
    DirectionalBudgetExceedsSharedBudget,
    InvalidOfferEnvelope,
    DuplicateQualificationReceipt,
    InvalidDispatchMessage,
    OfferPolicyIdentityMismatch,
    ProductNotAllowed(String),
    OfferExceedsOwnerPolicy,
    OfferAlreadyReserved(String),
    ReservationLedgerCapacityExceeded,
    InvalidPersistedReservation,
    InvalidPersistedReplayState,
    SharedCapacityOvercommitted { observed_kw: f64, maximum_kw: f64 },
    DirectionalCapacityOvercommitted {
        direction: GridFlexDirection,
        observed_kw: f64,
        maximum_kw: f64,
    },
    NumericalOverflow(&'static str),
}

impl fmt::Display for OwnerFlexibilityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingIdentity { name } => write!(f, "{name} must not be empty"),
            Self::IdentityTooLong { name, maximum_bytes, observed_bytes } => write!(
                f, "{name} exceeds {maximum_bytes} bytes; got {observed_bytes}"
            ),
            Self::InvalidQuantity { name, value } => write!(f, "{name} must be finite and non-negative; got {value}"),
            Self::EmptyPolicyAllowlist => write!(f, "owner policy product and signing-key allowlists must not be empty"),
            Self::InvalidPolicyEnvelope => write!(f, "owner policy envelope is invalid"),
            Self::DirectionalBudgetExceedsSharedBudget => write!(f, "directional reservation budget exceeds shared portfolio budget"),
            Self::InvalidOfferEnvelope => write!(f, "owner flexibility offer envelope is invalid"),
            Self::DuplicateQualificationReceipt => write!(f, "offer repeats a qualification receipt identifier"),
            Self::InvalidDispatchMessage => write!(f, "dispatch message fields or time bounds are invalid"),
            Self::OfferPolicyIdentityMismatch => write!(f, "offer site, PCC or portfolio does not match owner policy"),
            Self::ProductNotAllowed(product) => write!(f, "product {product} is not allowed by owner policy"),
            Self::OfferExceedsOwnerPolicy => write!(f, "offer exceeds an owner-defined capability or recovery bound"),
            Self::OfferAlreadyReserved(id) => write!(f, "offer {id} is already reserved"),
            Self::ReservationLedgerCapacityExceeded => write!(f, "reservation ledger is at its configured capacity"),
            Self::InvalidPersistedReservation => write!(f, "persisted reservation does not match its policy or offer invariants"),
            Self::InvalidPersistedReplayState => write!(f, "persisted replay state is inconsistent or exceeds configured bounds"),
            Self::SharedCapacityOvercommitted { observed_kw, maximum_kw } => write!(f, "overlapping offers reserve {observed_kw} kW against shared budget {maximum_kw} kW"),
            Self::DirectionalCapacityOvercommitted { direction, observed_kw, maximum_kw } => write!(f, "{direction:?} offers reserve {observed_kw} kW against directional budget {maximum_kw} kW"),
            Self::NumericalOverflow(operation) => write!(f, "non-finite arithmetic during {operation}"),
        }
    }
}

impl std::error::Error for OwnerFlexibilityError {}

fn require_id(name: &'static str, value: &str) -> Result<(), OwnerFlexibilityError> {
    if value.trim().is_empty() {
        Err(OwnerFlexibilityError::MissingIdentity { name })
    } else if value.len() > MAX_ID_BYTES {
        Err(OwnerFlexibilityError::IdentityTooLong {
            name,
            maximum_bytes: MAX_ID_BYTES,
            observed_bytes: value.len(),
        })
    } else {
        Ok(())
    }
}

fn require_finite_non_negative(name: &'static str, value: f64) -> Result<(), OwnerFlexibilityError> {
    if !value.is_finite() || value < 0.0 {
        Err(OwnerFlexibilityError::InvalidQuantity { name, value })
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct AcceptSignature;
    impl DispatchSignatureVerifier for AcceptSignature {
        fn verify(&self, signing_key_id: &str, canonical_payload: &[u8], signature: &[u8]) -> bool {
            signing_key_id == "market-key-1"
                && canonical_payload.starts_with(b"SYMTROPY-OWNER-DISPATCH-V1\0")
                && signature == b"test-signature"
        }
    }

    struct RejectSignature;
    impl DispatchSignatureVerifier for RejectSignature {
        fn verify(&self, _key: &str, _payload: &[u8], _signature: &[u8]) -> bool { false }
    }

    fn policy() -> OwnerFlexibilityPolicy {
        OwnerFlexibilityPolicy::new(
            "owner-policy-1", "v1", "site-1", "pcc-1", "portfolio-1",
            vec!["reserve-product".to_string(), "local-congestion".to_string()],
            vec!["market-key-1".to_string()],
            100.0, 80.0, 60.0, 30, 3_600, 20.0, 50.0,
            120, 2, 30,
        ).expect("policy")
    }

    fn offer(
        id: &str,
        product: &str,
        direction: GridFlexDirection,
        power_kw: f64,
        from: i64,
        until: i64,
    ) -> OwnerFlexibilityOffer {
        OwnerFlexibilityOffer::new(
            id, "portfolio-1", "site-1", "pcc-1", product, direction,
            power_kw, 20, 1_800, 10.0, 30.0, from, until,
            vec![format!("qualification-{id}")],
        ).expect("offer")
    }

    fn request(
        dispatch_id: &str,
        offer_id: &str,
        seq: u64,
        issued_at: i64,
        expires_at: i64,
        _telemetry: i64,
    ) -> OwnerDispatchRequest {
        OwnerDispatchRequest::new(
            dispatch_id, offer_id, "reserve-product", "site-1", "pcc-1",
            GridFlexDirection::ReduceGridImport, 40.0, 600, 5.0, 10.0,
            issued_at, expires_at, issued_at + 10, seq,
            format!("nonce-{seq}"), "market-key-1", b"test-signature".to_vec(),
        ).expect("request")
    }

    fn readiness(telemetry: i64) -> OwnerLocalReadiness {
        OwnerLocalReadiness::new(
            telemetry, OwnerGuardStatus::Ready, OwnerGuardStatus::Ready, OwnerGuardStatus::Ready,
        )
    }

    fn reserve_default(ledger: &mut OwnerOfferReservationLedger) -> OwnerFlexibilityOffer {
        let o = offer("offer-1", "reserve-product", GridFlexDirection::ReduceGridImport, 50.0, 900, 2_000);
        ledger.reserve_offer(&o, &policy()).expect("reservation");
        o
    }

    #[test]
    fn reserves_one_offer_and_releases_capacity() {
        let mut ledger = OwnerOfferReservationLedger::default();
        let o = reserve_default(&mut ledger);
        assert!(ledger.contains_offer("offer-1"));
        assert_eq!(ledger.reservations()[0].reserved_power_kw, 50.0);
        assert!(ledger.release_offer("offer-1"));
        assert!(!ledger.contains_offer(o.offer_id()));
    }

    #[test]
    fn prevents_overlapping_double_sale_across_products() {
        let mut ledger = OwnerOfferReservationLedger::default();
        ledger.reserve_offer(
            &offer("offer-a", "reserve-product", GridFlexDirection::ReduceGridImport, 70.0, 1_000, 2_000),
            &policy(),
        ).expect("first reservation");
        let second = ledger.reserve_offer(
            &offer("offer-b", "local-congestion", GridFlexDirection::ReduceGridImport, 40.0, 1_500, 2_500),
            &policy(),
        );
        assert!(matches!(second, Err(OwnerFlexibilityError::SharedCapacityOvercommitted { .. })));
    }

    #[test]
    fn permits_non_overlapping_reservations_on_the_same_portfolio() {
        let mut ledger = OwnerOfferReservationLedger::default();
        ledger.reserve_offer(
            &offer("offer-a", "reserve-product", GridFlexDirection::ReduceGridImport, 80.0, 1_000, 2_000),
            &policy(),
        ).expect("first");
        ledger.reserve_offer(
            &offer("offer-b", "local-congestion", GridFlexDirection::ReduceGridImport, 80.0, 2_000, 3_000),
            &policy(),
        ).expect("non-overlapping reservation");
        assert_eq!(ledger.reservations().len(), 2);
    }

    #[test]
    fn changed_policy_contents_with_reused_version_invalidate_reservation() {
        let mut ledger = OwnerOfferReservationLedger::default();
        let o = reserve_default(&mut ledger);
        let altered_same_version = OwnerFlexibilityPolicy::new(
            "owner-policy-1", "v1", "site-1", "pcc-1", "portfolio-1",
            vec!["reserve-product".to_string()],
            vec!["market-key-1".to_string()],
            90.0, 70.0, 60.0, 30, 3_600, 20.0, 50.0,
            120, 2, 30,
        ).expect("same-version but altered policy");
        assert!(!ledger.matches_offer(&o, &altered_same_version));
        let mut replay = DispatchReplayLedger::default();
        let decision = authorize_owner_dispatch(
            &request("dispatch-policy-reuse", "offer-1", 1, 1_000, 1_060, 990),
            &o, &altered_same_version, &ledger, &mut replay,
            readiness(990), 1_005, &AcceptSignature,
        );
        assert!(matches!(decision, OwnerDispatchDecision::Rejected {
            reason: OwnerDispatchRejectionReason::OfferNotReserved, ..
        }));
    }

    #[test]
    fn policy_revision_change_invalidates_old_reservation() {
        let mut ledger = OwnerOfferReservationLedger::default();
        let o = reserve_default(&mut ledger);
        let stricter_policy = OwnerFlexibilityPolicy::new(
            "owner-policy-1", "v2", "site-1", "pcc-1", "portfolio-1",
            vec!["reserve-product".to_string()],
            vec!["market-key-1".to_string()],
            90.0, 70.0, 60.0, 30, 3_600, 20.0, 50.0,
            120, 2, 30,
        ).expect("new policy revision");
        assert!(!ledger.matches_offer(&o, &stricter_policy));
        let mut replay = DispatchReplayLedger::default();
        let decision = authorize_owner_dispatch(
            &request("dispatch-old-policy", "offer-1", 1, 1_000, 1_060, 990),
            &o, &stricter_policy, &ledger, &mut replay,
            readiness(990), 1_005, &AcceptSignature,
        );
        assert!(matches!(decision, OwnerDispatchDecision::Rejected {
            reason: OwnerDispatchRejectionReason::OfferNotReserved, ..
        }));
    }

    #[test]
    fn dispatch_cannot_substitute_a_larger_offer_with_the_same_identifier() {
        let mut ledger = OwnerOfferReservationLedger::default();
        let reserved = offer(
            "offer-1", "reserve-product", GridFlexDirection::ReduceGridImport,
            50.0, 900, 2_000,
        );
        ledger.reserve_offer(&reserved, &policy()).expect("reserve original");
        let substituted = offer(
            "offer-1", "reserve-product", GridFlexDirection::ReduceGridImport,
            80.0, 900, 2_000,
        );
        let mut replay = DispatchReplayLedger::default();
        let decision = authorize_owner_dispatch(
            &request("dispatch-substitute", "offer-1", 1, 1_000, 1_060, 990),
            &substituted, &policy(), &ledger, &mut replay,
            readiness(990), 1_005, &AcceptSignature,
        );
        assert!(matches!(decision, OwnerDispatchDecision::Rejected {
            reason: OwnerDispatchRejectionReason::OfferNotReserved, ..
        }));
        assert!(replay.per_offer.is_empty());
    }

    #[test]
    fn offer_cannot_be_dispatched_before_its_validity_window() {
        let mut ledger = OwnerOfferReservationLedger::default();
        let o = offer(
            "future-offer", "reserve-product", GridFlexDirection::ReduceGridImport,
            50.0, 1_100, 2_000,
        );
        ledger.reserve_offer(&o, &policy()).expect("reserve future offer");
        let future_request = OwnerDispatchRequest::new(
            "dispatch-future-offer", "future-offer", "reserve-product", "site-1", "pcc-1",
            GridFlexDirection::ReduceGridImport, 40.0, 600, 5.0, 10.0,
            1_095, 1_155, 1_105, 1, "future-offer-nonce", "market-key-1",
            b"test-signature".to_vec(),
        ).expect("future request");
        let mut replay = DispatchReplayLedger::default();
        let decision = authorize_owner_dispatch(
            &future_request, &o, &policy(), &ledger, &mut replay,
            readiness(1_095), 1_095, &AcceptSignature,
        );
        assert!(matches!(decision, OwnerDispatchDecision::Rejected {
            reason: OwnerDispatchRejectionReason::OfferOutsideValidityWindow, ..
        }));
    }

    #[test]
    fn valid_signed_dispatch_returns_local_only_authorization() {
        let mut ledger = OwnerOfferReservationLedger::default();
        let o = reserve_default(&mut ledger);
        let mut replay = DispatchReplayLedger::default();
        let decision = authorize_owner_dispatch(
            &request("dispatch-1", "offer-1", 1, 1_000, 1_060, 990),
            &o, &policy(), &ledger, &mut replay, readiness(990), 1_005, &AcceptSignature,
        );
        match decision {
            OwnerDispatchDecision::Authorized(receipt) => {
                assert_eq!(receipt.authorized_power_kw, 40.0);
                assert!(receipt.local_controller_only);
                assert!(!receipt.actuator_command_emitted);
                assert!(!receipt.canonical_payload.is_empty());
                assert_eq!(receipt.signature, b"test-signature");
            }
            _ => panic!("expected local authorization"),
        }
    }

    #[test]
    fn invalid_signature_does_not_advance_replay_state() {
        let mut ledger = OwnerOfferReservationLedger::default();
        let o = reserve_default(&mut ledger);
        let mut replay = DispatchReplayLedger::default();
        let decision = authorize_owner_dispatch(
            &request("dispatch-1", "offer-1", 1, 1_000, 1_060, 990),
            &o, &policy(), &ledger, &mut replay, readiness(990), 1_005, &RejectSignature,
        );
        assert!(matches!(decision, OwnerDispatchDecision::Rejected {
            reason: OwnerDispatchRejectionReason::InvalidSignature, ..
        }));
        assert!(replay.per_offer.is_empty());
    }

    #[test]
    fn duplicate_sequence_nonce_or_dispatch_is_rejected() {
        let mut ledger = OwnerOfferReservationLedger::default();
        let o = reserve_default(&mut ledger);
        let mut replay = DispatchReplayLedger::default();
        let first = authorize_owner_dispatch(
            &request("dispatch-1", "offer-1", 2, 1_000, 1_060, 990),
            &o, &policy(), &ledger, &mut replay, readiness(990), 1_005, &AcceptSignature,
        );
        assert!(matches!(first, OwnerDispatchDecision::Authorized(_)));
        let replayed = authorize_owner_dispatch(
            &request("dispatch-2", "offer-1", 2, 1_000, 1_060, 990),
            &o, &policy(), &ledger, &mut replay, readiness(990), 1_005, &AcceptSignature,
        );
        assert!(matches!(replayed, OwnerDispatchDecision::Rejected {
            reason: OwnerDispatchRejectionReason::ReplayedDispatch, ..
        }));
    }

    #[test]
    fn expired_dispatch_is_rejected_after_authenticated_replay_record() {
        let mut ledger = OwnerOfferReservationLedger::default();
        let o = reserve_default(&mut ledger);
        let mut replay = DispatchReplayLedger::default();
        let decision = authorize_owner_dispatch(
            &request("dispatch-expired", "offer-1", 1, 1_000, 1_010, 990),
            &o, &policy(), &ledger, &mut replay, readiness(990), 1_011, &AcceptSignature,
        );
        assert!(matches!(decision, OwnerDispatchDecision::Rejected {
            reason: OwnerDispatchRejectionReason::DispatchExpired, ..
        }));
        assert!(replay.per_offer.contains_key("offer-1"));
    }

    #[test]
    fn stale_telemetry_fails_closed() {
        let mut ledger = OwnerOfferReservationLedger::default();
        let o = reserve_default(&mut ledger);
        let mut replay = DispatchReplayLedger::default();
        let decision = authorize_owner_dispatch(
            &request("dispatch-stale", "offer-1", 1, 1_000, 1_060, 900),
            &o, &policy(), &ledger, &mut replay, readiness(900), 1_005, &AcceptSignature,
        );
        assert!(matches!(decision, OwnerDispatchDecision::Rejected {
            reason: OwnerDispatchRejectionReason::StalePccTelemetry, ..
        }));
    }

    #[test]
    fn unknown_or_blocked_safety_guard_fails_closed() {
        let mut ledger = OwnerOfferReservationLedger::default();
        let o = reserve_default(&mut ledger);
        let mut replay = DispatchReplayLedger::default();
        let local = OwnerLocalReadiness::new(
            990, OwnerGuardStatus::Ready, OwnerGuardStatus::Unknown, OwnerGuardStatus::Ready,
        );
        let decision = authorize_owner_dispatch(
            &request("dispatch-unknown", "offer-1", 1, 1_000, 1_060, 990),
            &o, &policy(), &ledger, &mut replay, local, 1_005, &AcceptSignature,
        );
        assert!(matches!(decision, OwnerDispatchDecision::Rejected {
            reason: OwnerDispatchRejectionReason::PhysicalSafetyGuardNotReady, ..
        }));
    }

    #[test]
    fn unreserved_offer_and_unauthorized_product_are_rejected() {
        let mut ledger = OwnerOfferReservationLedger::default();
        let mut replay = DispatchReplayLedger::default();
        let o = offer("offer-1", "reserve-product", GridFlexDirection::ReduceGridImport, 50.0, 900, 2_000);
        let unreserved = authorize_owner_dispatch(
            &request("dispatch-unreserved", "offer-1", 1, 1_000, 1_060, 990),
            &o, &policy(), &ledger, &mut replay, readiness(990), 1_005, &AcceptSignature,
        );
        assert!(matches!(unreserved, OwnerDispatchDecision::Rejected {
            reason: OwnerDispatchRejectionReason::OfferNotReserved, ..
        }));
        ledger.reserve_offer(&o, &policy()).expect("reserve");
        let unauthorized_request = OwnerDispatchRequest::new(
            "dispatch-product", "offer-1", "unknown-product", "site-1", "pcc-1",
            GridFlexDirection::ReduceGridImport, 40.0, 600, 5.0, 10.0,
            1_000, 1_060, 1_010, 1, "nonce-prod", "market-key-1", b"test-signature".to_vec(),
        ).expect("request");
        let denied = authorize_owner_dispatch(
            &unauthorized_request, &o, &policy(), &ledger, &mut replay,
            readiness(990), 1_005, &AcceptSignature,
        );
        assert!(matches!(denied, OwnerDispatchDecision::Rejected {
            reason: OwnerDispatchRejectionReason::ProductNotAllowed, ..
        }));
    }

    #[test]
    fn response_deadline_and_recovery_must_fit_offer() {
        let mut ledger = OwnerOfferReservationLedger::default();
        let o = reserve_default(&mut ledger);
        let mut replay = DispatchReplayLedger::default();
        let too_much_recovery = OwnerDispatchRequest::new(
            "dispatch-recovery", "offer-1", "reserve-product", "site-1", "pcc-1",
            GridFlexDirection::ReduceGridImport, 40.0, 600, 15.0, 10.0,
            1_000, 1_060, 1_010, 1, "nonce-recovery", "market-key-1", b"test-signature".to_vec(),
        ).expect("request");
        let denied = authorize_owner_dispatch(
            &too_much_recovery, &o, &policy(), &ledger, &mut replay,
            readiness(990), 1_005, &AcceptSignature,
        );
        assert!(matches!(denied, OwnerDispatchDecision::Rejected {
            reason: OwnerDispatchRejectionReason::RecoveryEnvelopeExceeded, ..
        }));
    }

    #[test]
    fn controller_unavailable_blocks_even_authenticated_dispatch() {
        let mut ledger = OwnerOfferReservationLedger::default();
        let o = reserve_default(&mut ledger);
        let mut replay = DispatchReplayLedger::default();
        let local = OwnerLocalReadiness::new(
            990, OwnerGuardStatus::Ready, OwnerGuardStatus::Ready, OwnerGuardStatus::Blocked,
        );
        let decision = authorize_owner_dispatch(
            &request("dispatch-no-controller", "offer-1", 1, 1_000, 1_060, 990),
            &o, &policy(), &ledger, &mut replay, local, 1_005, &AcceptSignature,
        );
        assert!(matches!(decision, OwnerDispatchDecision::Rejected {
            reason: OwnerDispatchRejectionReason::LocalControllerNotReady, ..
        }));
    }

    #[test]
    fn same_signer_nonce_cannot_be_reused_across_different_offers() {
        let mut ledger = OwnerOfferReservationLedger::default();
        let offer_a = offer("offer-a", "reserve-product", GridFlexDirection::ReduceGridImport, 30.0, 900, 2_000);
        let offer_b = offer("offer-b", "local-congestion", GridFlexDirection::ReduceGridImport, 30.0, 900, 2_000);
        ledger.reserve_offer(&offer_a, &policy()).expect("reserve A");
        ledger.reserve_offer(&offer_b, &policy()).expect("reserve B");
        let mut replay = DispatchReplayLedger::default();
        let request_a = OwnerDispatchRequest::new(
            "dispatch-a", "offer-a", "reserve-product", "site-1", "pcc-1",
            GridFlexDirection::ReduceGridImport, 20.0, 600, 5.0, 10.0,
            1_000, 1_060, 1_010, 1, "shared-nonce", "market-key-1", b"test-signature".to_vec(),
        ).expect("request A");
        let request_b = OwnerDispatchRequest::new(
            "dispatch-b", "offer-b", "local-congestion", "site-1", "pcc-1",
            GridFlexDirection::ReduceGridImport, 20.0, 600, 5.0, 10.0,
            1_000, 1_060, 1_010, 1, "shared-nonce", "market-key-1", b"test-signature".to_vec(),
        ).expect("request B");
        let first = authorize_owner_dispatch(
            &request_a, &offer_a, &policy(), &ledger, &mut replay,
            readiness(990), 1_005, &AcceptSignature,
        );
        assert!(matches!(first, OwnerDispatchDecision::Authorized(_)));
        let second = authorize_owner_dispatch(
            &request_b, &offer_b, &policy(), &ledger, &mut replay,
            readiness(990), 1_005, &AcceptSignature,
        );
        assert!(matches!(second, OwnerDispatchDecision::Rejected {
            reason: OwnerDispatchRejectionReason::ReplayedDispatch, ..
        }));
    }

    #[test]
    fn missed_response_deadline_is_rejected_even_before_message_expiry() {
        let mut ledger = OwnerOfferReservationLedger::default();
        let o = reserve_default(&mut ledger);
        let mut replay = DispatchReplayLedger::default();
        let decision = authorize_owner_dispatch(
            &request("dispatch-late-response", "offer-1", 1, 1_000, 1_060, 990),
            &o, &policy(), &ledger, &mut replay, readiness(990), 1_020, &AcceptSignature,
        );
        assert!(matches!(decision, OwnerDispatchDecision::Rejected {
            reason: OwnerDispatchRejectionReason::ResponseDeadlineMissed, ..
        }));
    }

    #[test]
    fn exact_offer_id_does_not_authorize_a_different_recovery_envelope() {
        let mut ledger = OwnerOfferReservationLedger::default();
        let reserved = offer(
            "offer-1", "reserve-product", GridFlexDirection::ReduceGridImport,
            50.0, 900, 2_000,
        );
        ledger.reserve_offer(&reserved, &policy()).expect("reserve original");
        let substituted = OwnerFlexibilityOffer::new(
            "offer-1", "portfolio-1", "site-1", "pcc-1", "reserve-product",
            GridFlexDirection::ReduceGridImport, 50.0, 20, 1_800, 11.0, 30.0,
            900, 2_000, vec!["qualification-offer-1".to_string()],
        ).expect("alternate recovery envelope");
        let mut replay = DispatchReplayLedger::default();
        let decision = authorize_owner_dispatch(
            &request("dispatch-substitute-recovery", "offer-1", 1, 1_000, 1_060, 990),
            &substituted, &policy(), &ledger, &mut replay,
            readiness(990), 1_005, &AcceptSignature,
        );
        assert!(matches!(decision, OwnerDispatchDecision::Rejected {
            reason: OwnerDispatchRejectionReason::OfferNotReserved, ..
        }));
    }

    #[test]
    fn expiration_is_exclusive_at_the_expiry_timestamp() {
        let mut ledger = OwnerOfferReservationLedger::default();
        let o = reserve_default(&mut ledger);
        let mut replay = DispatchReplayLedger::default();
        let decision = authorize_owner_dispatch(
            &request("dispatch-expiry-boundary", "offer-1", 1, 1_000, 1_010, 990),
            &o, &policy(), &ledger, &mut replay, readiness(990), 1_010, &AcceptSignature,
        );
        assert!(matches!(decision, OwnerDispatchDecision::Rejected {
            reason: OwnerDispatchRejectionReason::DispatchExpired, ..
        }));
    }

    #[test]
    fn replay_ledger_capacity_fails_closed_without_accepting_a_new_offer_sequence() {
        let mut ledger = OwnerOfferReservationLedger::default();
        let o = reserve_default(&mut ledger);
        let mut replay = DispatchReplayLedger::default();
        for index in 0..MAX_REPLAY_ENTRIES {
            replay.seen_key_nonces.insert((
                "retained-market-key".to_string(),
                format!("retained-nonce-{index}"),
            ));
        }
        let decision = authorize_owner_dispatch(
            &request("dispatch-at-capacity", "offer-1", 1, 1_000, 1_060, 990),
            &o, &policy(), &ledger, &mut replay, readiness(990), 1_005, &AcceptSignature,
        );
        assert!(matches!(decision, OwnerDispatchDecision::Rejected {
            reason: OwnerDispatchRejectionReason::ReplayLedgerCapacityExceeded, ..
        }));
        assert!(!replay.per_offer.contains_key("offer-1"));
    }

    #[test]
    fn oversized_signature_and_identity_are_rejected_at_ingress() {
        let oversized_signature = OwnerDispatchRequest::new(
            "dispatch-large-signature", "offer-1", "reserve-product", "site-1", "pcc-1",
            GridFlexDirection::ReduceGridImport, 40.0, 600, 5.0, 10.0,
            1_000, 1_060, 1_010, 1, "nonce-large", "market-key-1",
            vec![0_u8; MAX_SIGNATURE_BYTES + 1],
        );
        assert!(matches!(
            oversized_signature,
            Err(OwnerFlexibilityError::InvalidDispatchMessage)
        ));
        assert!(matches!(
            OwnerDispatchRequest::new(
                "x".repeat(MAX_ID_BYTES + 1), "offer-1", "reserve-product", "site-1", "pcc-1",
                GridFlexDirection::ReduceGridImport, 40.0, 600, 5.0, 10.0,
                1_000, 1_060, 1_010, 1, "nonce-large-id", "market-key-1",
                b"test-signature".to_vec(),
            ),
            Err(OwnerFlexibilityError::IdentityTooLong { .. })
        ));
    }

    #[test]
    fn serde_policy_wire_cannot_bypass_constructor_allowlist_validation() {
        let wire = OwnerFlexibilityPolicyWire {
            policy_id: "policy".to_string(),
            policy_version: "v1".to_string(),
            site_id: "site".to_string(),
            pcc_boundary_id: "pcc".to_string(),
            portfolio_id: "portfolio".to_string(),
            allowed_product_ids: BTreeSet::new(),
            authorized_signing_key_ids: ["market-key".to_string()].into_iter().collect(),
            maximum_concurrent_reserved_kw: 100.0,
            maximum_reduce_import_reserved_kw: 80.0,
            maximum_increase_import_reserved_kw: 60.0,
            maximum_response_latency_seconds: 30,
            maximum_sustain_seconds: 3_600,
            maximum_recovery_power_kw: 20.0,
            maximum_recovery_energy_kwh: 50.0,
            maximum_dispatch_ttl_seconds: 120,
            allowed_clock_skew_seconds: 2,
            maximum_telemetry_age_seconds: 30,
        };
        assert!(matches!(
            OwnerFlexibilityPolicy::try_from(wire),
            Err(OwnerFlexibilityError::EmptyPolicyAllowlist)
        ));
    }

    #[test]
    fn serde_offer_wire_cannot_bypass_offer_time_and_receipt_validation() {
        let wire = OwnerFlexibilityOfferWire {
            offer_id: "offer".to_string(),
            portfolio_id: "portfolio".to_string(),
            site_id: "site".to_string(),
            pcc_boundary_id: "pcc".to_string(),
            product_id: "product".to_string(),
            direction: GridFlexDirection::ReduceGridImport,
            maximum_power_kw: 10.0,
            maximum_response_latency_seconds: 10,
            maximum_sustain_seconds: 60,
            maximum_recovery_power_kw: 2.0,
            maximum_recovery_energy_kwh: 3.0,
            valid_from_unix_seconds: 100,
            valid_until_unix_seconds: 90,
            qualification_receipt_ids: vec!["receipt".to_string()],
        };
        assert!(matches!(
            OwnerFlexibilityOffer::try_from(wire),
            Err(OwnerFlexibilityError::InvalidOfferEnvelope)
        ));
    }

    #[test]
    fn serde_dispatch_wire_cannot_bypass_request_identity_validation() {
        let wire = OwnerDispatchRequestWire {
            dispatch_id: String::new(),
            offer_id: "offer".to_string(),
            product_id: "product".to_string(),
            site_id: "site".to_string(),
            pcc_boundary_id: "pcc".to_string(),
            direction: GridFlexDirection::ReduceGridImport,
            requested_power_kw: 1.0,
            requested_sustain_seconds: 10,
            declared_recovery_power_kw: 0.0,
            declared_recovery_energy_kwh: 0.0,
            issued_at_unix_seconds: 100,
            expires_at_unix_seconds: 110,
            response_deadline_unix_seconds: 105,
            sequence_number: 1,
            nonce: "nonce".to_string(),
            signing_key_id: "key".to_string(),
            signature: vec![1],
        };
        assert!(matches!(
            OwnerDispatchRequest::try_from(wire),
            Err(OwnerFlexibilityError::MissingIdentity { name: "dispatch ID" })
        ));
    }

    #[test]
    fn restored_reservation_ledger_revalidates_capacity_and_policy_invariants() {
        let mut ledger = OwnerOfferReservationLedger::default();
        let first = offer("offer-a", "reserve-product", GridFlexDirection::ReduceGridImport, 60.0, 1_000, 2_000);
        ledger.reserve_offer(&first, &policy()).expect("reserve first");
        let mut second = ledger.reservations[0].clone();
        second.offer_id = "offer-b".to_string();
        second.reserved_power_kw = 30.0;
        second.valid_from_unix_seconds = 1_500;
        second.valid_until_unix_seconds = 2_500;
        let restored = OwnerOfferReservationLedger::try_from(OwnerOfferReservationLedgerWire {
            reservations: vec![ledger.reservations[0].clone(), second],
        });
        assert!(matches!(
            restored,
            Err(OwnerFlexibilityError::SharedCapacityOvercommitted { .. })
        ));
    }

    #[test]
    fn restored_replay_ledger_rejects_inconsistent_nonce_dispatch_counts() {
        let wire = DispatchReplayLedgerWire {
            per_offer: [(
                "offer-1".to_string(),
                DispatchReplayState { highest_sequence_number: 1 },
            )].into_iter().collect(),
            seen_key_nonces: [("market-key-1".to_string(), "nonce-1".to_string())]
                .into_iter().collect(),
            seen_dispatch_ids: BTreeSet::new(),
        };
        assert!(matches!(
            DispatchReplayLedger::try_from(wire),
            Err(OwnerFlexibilityError::InvalidPersistedReplayState)
        ));
    }

    #[test]
    fn invalid_message_times_and_nan_values_are_rejected_at_construction() {
        assert!(matches!(
            OwnerDispatchRequest::new(
                "d", "o", "p", "s", "b", GridFlexDirection::ReduceGridImport,
                f64::NAN, 10, 0.0, 0.0, 100, 90, 100, 1, "n", "key", vec![1],
            ),
            Err(OwnerFlexibilityError::InvalidQuantity { .. })
        ));
        assert!(matches!(
            OwnerDispatchRequest::new(
                "d", "o", "p", "s", "b", GridFlexDirection::ReduceGridImport,
                1.0, 10, 0.0, 0.0, 100, 90, 100, 1, "n", "key", vec![1],
            ),
            Err(OwnerFlexibilityError::InvalidDispatchMessage)
        ));
    }
}
