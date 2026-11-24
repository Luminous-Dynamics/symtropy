#![forbid(unsafe_code)]

//! SAFRB v0.7: evidence-qualified physical capacity and access envelope.
//!
//! This evaluator consumes resource-capacity claims and an externally qualified
//! authority profile. It calculates a read-only upper bound for a proposed
//! flow. It does not grant access, reserve capacity, commit a provider, or
//! authorize physical execution.

use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Unit {
    Litres,
    WattHours,
    Tonnes,
    CubicMetres,
    Slots,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EvidenceClass {
    Measured,
    QualifiedAuthority,
    Scenario,
    Reported,
    Derived,
    Forecast,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Evidence {
    pub source_id: String,
    pub source_uri: String,
    pub evidence_class: EvidenceClass,
    /// Reference-shape check only; this crate does not hash source bytes.
    pub raw_sha256: Option<String>,
    /// Receipt identifier produced by an upstream authority verifier.
    pub authority_receipt_id: Option<String>,
}

impl Evidence {
    pub fn validate(&self) -> Result<(), String> {
        if self.source_id.trim().is_empty() || self.source_uri.trim().is_empty() {
            return Err("evidence source_id and source_uri must not be empty".into());
        }
        if let Some(digest) = &self.raw_sha256
            && (digest.len() != 64 || !digest.bytes().all(|byte| byte.is_ascii_hexdigit()))
        {
            return Err("raw_sha256 must be exactly 64 hexadecimal characters".into());
        }
        if let Some(receipt) = &self.authority_receipt_id
            && receipt.trim().is_empty()
        {
            return Err("authority_receipt_id must not be empty when present".into());
        }
        Ok(())
    }

    fn is_explicit_scenario(&self) -> bool {
        self.evidence_class == EvidenceClass::Scenario
            && self.source_uri.starts_with("urn:safrb:fixture:")
    }

    fn qualifies_physical_quantity(&self) -> bool {
        match self.evidence_class {
            EvidenceClass::Measured => self.raw_sha256.as_ref().is_some_and(|digest| {
                digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
            }),
            EvidenceClass::Scenario => self.is_explicit_scenario(),
            EvidenceClass::QualifiedAuthority
            | EvidenceClass::Reported
            | EvidenceClass::Derived
            | EvidenceClass::Forecast => false,
        }
    }

    fn qualifies_authority_claim(&self) -> bool {
        match self.evidence_class {
            EvidenceClass::QualifiedAuthority => {
                self.raw_sha256.as_ref().is_some_and(|digest| {
                    digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit())
                }) && self
                    .authority_receipt_id
                    .as_ref()
                    .is_some_and(|receipt| !receipt.trim().is_empty())
            }
            EvidenceClass::Scenario => self.is_explicit_scenario(),
            EvidenceClass::Measured
            | EvidenceClass::Reported
            | EvidenceClass::Derived
            | EvidenceClass::Forecast => false,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Quantity {
    pub amount: u128,
    pub unit: Unit,
}

impl Quantity {
    fn validate_positive(self, label: &str) -> Result<(), String> {
        if self.amount == 0 {
            return Err(format!("{label} must be positive"));
        }
        Ok(())
    }

    fn require_unit(self, expected: Unit, label: &str) -> Result<(), String> {
        if self.unit != expected {
            return Err(format!("{label} unit does not match the resource envelope"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LimitValue {
    Absolute(Quantity),
    /// Share of an explicitly identified denominator. Percentage-only claims
    /// without a known denominator cannot be converted to physical capacity.
    Share {
        basis_points: u16,
        denominator_id: String,
        denominator: Quantity,
        denominator_evidence: Evidence,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ClaimKind {
    InstalledCapacity,
    OperableCapacity,
    AccessEntitlement,
    ProviderOffer,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapacityClaim {
    pub claim_id: String,
    pub resource_id: String,
    pub period_id: String,
    pub kind: ClaimKind,
    pub limit: LimitValue,
    pub evidence: Evidence,
}

impl CapacityClaim {
    fn resolve(
        &self,
        resource_id: &str,
        period_id: &str,
        expected_unit: Unit,
    ) -> Result<u128, String> {
        if self.claim_id.trim().is_empty() {
            return Err("capacity claim_id must not be empty".into());
        }
        if self.resource_id != resource_id {
            return Err(format!(
                "claim {} references a different resource",
                self.claim_id
            ));
        }
        if self.period_id != period_id {
            return Err(format!(
                "claim {} references a different period",
                self.claim_id
            ));
        }
        self.evidence.validate()?;

        match self.kind {
            ClaimKind::InstalledCapacity | ClaimKind::OperableCapacity => {
                if !self.evidence.qualifies_physical_quantity() {
                    return Err(format!(
                        "physical capacity claim {} lacks qualified measurement or fixture evidence",
                        self.claim_id
                    ));
                }
                let LimitValue::Absolute(quantity) = &self.limit else {
                    return Err(format!(
                        "physical capacity claim {} must use an absolute quantity",
                        self.claim_id
                    ));
                };
                (*quantity).validate_positive(&self.claim_id)?;
                (*quantity).require_unit(expected_unit, &self.claim_id)?;
                Ok(quantity.amount)
            }
            ClaimKind::AccessEntitlement | ClaimKind::ProviderOffer => {
                if !self.evidence.qualifies_authority_claim() {
                    return Err(format!(
                        "access claim {} lacks an externally qualified authority receipt",
                        self.claim_id
                    ));
                }
                match &self.limit {
                    LimitValue::Absolute(quantity) => {
                        quantity.validate_positive(&self.claim_id)?;
                        quantity.require_unit(expected_unit, &self.claim_id)?;
                        Ok(quantity.amount)
                    }
                    LimitValue::Share {
                        basis_points,
                        denominator_id,
                        denominator,
                        denominator_evidence,
                    } => {
                        if *basis_points == 0 || *basis_points > 10_000 {
                            return Err(format!(
                                "share claim {} must be in 1..=10000 basis points",
                                self.claim_id
                            ));
                        }
                        if denominator_id.trim().is_empty() {
                            return Err(format!(
                                "share claim {} has no denominator identity",
                                self.claim_id
                            ));
                        }
                        denominator.validate_positive("share denominator")?;
                        denominator.require_unit(expected_unit, "share denominator")?;
                        denominator_evidence.validate()?;
                        if !denominator_evidence.qualifies_physical_quantity() {
                            return Err(format!(
                                "share claim {} has an unqualified denominator",
                                self.claim_id
                            ));
                        }
                        denominator
                            .amount
                            .checked_mul(u128::from(*basis_points))
                            .ok_or_else(|| "share conversion overflowed u128".to_string())
                            .map(|numerator| numerator / 10_000)
                    }
                }
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AllocationState {
    Reserved,
    Committed,
    Delivered,
    Cancelled,
    Expired,
}

impl AllocationState {
    fn consumes_capacity(self) -> bool {
        matches!(self, Self::Reserved | Self::Committed | Self::Delivered)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExistingAllocation {
    pub allocation_id: String,
    pub resource_id: String,
    pub period_id: String,
    pub quantity: Quantity,
    pub state: AllocationState,
    pub evidence: Evidence,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CandidateRequest {
    pub request_id: String,
    pub resource_id: String,
    pub period_id: String,
    pub quantity: Quantity,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapacityEnvelope {
    pub scenario_id: String,
    pub resource_id: String,
    pub period_id: String,
    pub unit: Unit,
    pub installed: CapacityClaim,
    pub operable: CapacityClaim,
    pub entitlement: CapacityClaim,
    pub provider_offer: CapacityClaim,
    pub existing_allocations: Vec<ExistingAllocation>,
    pub candidate: CandidateRequest,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Disposition {
    FitsWithinEvidenceBoundedCapacity,
    ExceedsEvidenceBoundedCapacity,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CapacityAssessment {
    pub scenario_id: String,
    pub resource_id: String,
    pub period_id: String,
    pub unit: Unit,
    pub installed_capacity: u128,
    pub operable_capacity: u128,
    pub entitlement_limit: u128,
    pub provider_offer_limit: u128,
    pub physical_limit: u128,
    pub access_limited_capacity: u128,
    pub existing_capacity_consumed: u128,
    pub residual_schedulable_capacity: u128,
    pub candidate_requested: u128,
    pub candidate_excess: u128,
    pub disposition: Disposition,
    /// Identifies inputs for audit; this is not an authorization receipt.
    pub source_claim_ids: Vec<String>,
    pub counted_allocation_ids: Vec<String>,
}

fn checked_add(target: &mut u128, amount: u128, label: &str) -> Result<(), String> {
    *target = target
        .checked_add(amount)
        .ok_or_else(|| format!("{label} overflowed u128"))?;
    Ok(())
}

impl CapacityEnvelope {
    pub fn evaluate(&self) -> Result<CapacityAssessment, String> {
        if self.scenario_id.trim().is_empty()
            || self.resource_id.trim().is_empty()
            || self.period_id.trim().is_empty()
        {
            return Err("scenario, resource and period identifiers must not be empty".into());
        }

        let installed_capacity =
            self.installed
                .resolve(&self.resource_id, &self.period_id, self.unit)?;
        let operable_capacity =
            self.operable
                .resolve(&self.resource_id, &self.period_id, self.unit)?;
        let entitlement_limit =
            self.entitlement
                .resolve(&self.resource_id, &self.period_id, self.unit)?;
        let provider_offer_limit =
            self.provider_offer
                .resolve(&self.resource_id, &self.period_id, self.unit)?;

        if operable_capacity > installed_capacity {
            return Err("operable capacity exceeds installed capacity".into());
        }

        if self.candidate.request_id.trim().is_empty()
            || self.candidate.resource_id != self.resource_id
            || self.candidate.period_id != self.period_id
        {
            return Err("candidate request identity does not match the capacity envelope".into());
        }
        self.candidate
            .quantity
            .validate_positive("candidate request")?;
        self.candidate
            .quantity
            .require_unit(self.unit, "candidate request")?;

        let physical_limit = installed_capacity.min(operable_capacity);
        let access_limited_capacity = physical_limit
            .min(entitlement_limit)
            .min(provider_offer_limit);

        let mut allocation_ids = BTreeSet::new();
        let mut existing_capacity_consumed = 0_u128;
        let mut counted_allocation_ids = Vec::new();

        for allocation in &self.existing_allocations {
            if allocation.allocation_id.trim().is_empty()
                || allocation.resource_id != self.resource_id
                || allocation.period_id != self.period_id
            {
                return Err("existing allocation identity does not match the envelope".into());
            }
            if !allocation_ids.insert(allocation.allocation_id.as_str()) {
                return Err(format!(
                    "duplicate allocation_id: {}",
                    allocation.allocation_id
                ));
            }
            allocation
                .quantity
                .validate_positive("existing allocation")?;
            allocation
                .quantity
                .require_unit(self.unit, "existing allocation")?;
            allocation.evidence.validate()?;

            if allocation.state.consumes_capacity() {
                if !allocation.evidence.qualifies_authority_claim() {
                    return Err(format!(
                        "capacity-consuming allocation {} lacks qualified authority evidence",
                        allocation.allocation_id
                    ));
                }
                checked_add(
                    &mut existing_capacity_consumed,
                    allocation.quantity.amount,
                    "existing capacity consumed",
                )?;
                counted_allocation_ids.push(allocation.allocation_id.clone());
            }
        }

        if existing_capacity_consumed > access_limited_capacity {
            return Err("existing allocations exceed evidence-bounded accessible capacity".into());
        }
        let residual_schedulable_capacity = access_limited_capacity - existing_capacity_consumed;
        let candidate_requested = self.candidate.quantity.amount;
        let candidate_excess = candidate_requested.saturating_sub(residual_schedulable_capacity);
        let disposition = if candidate_excess == 0 {
            Disposition::FitsWithinEvidenceBoundedCapacity
        } else {
            Disposition::ExceedsEvidenceBoundedCapacity
        };

        Ok(CapacityAssessment {
            scenario_id: self.scenario_id.clone(),
            resource_id: self.resource_id.clone(),
            period_id: self.period_id.clone(),
            unit: self.unit,
            installed_capacity,
            operable_capacity,
            entitlement_limit,
            provider_offer_limit,
            physical_limit,
            access_limited_capacity,
            existing_capacity_consumed,
            residual_schedulable_capacity,
            candidate_requested,
            candidate_excess,
            disposition,
            source_claim_ids: vec![
                self.installed.claim_id.clone(),
                self.operable.claim_id.clone(),
                self.entitlement.claim_id.clone(),
                self.provider_offer.claim_id.clone(),
            ],
            counted_allocation_ids,
        })
    }
}

fn fixture_evidence(source_id: &str) -> Evidence {
    Evidence {
        source_id: source_id.into(),
        source_uri: format!("urn:safrb:fixture:{source_id}"),
        evidence_class: EvidenceClass::Scenario,
        raw_sha256: None,
        authority_receipt_id: None,
    }
}

fn fixture_claim(
    claim_id: &str,
    resource_id: &str,
    period_id: &str,
    kind: ClaimKind,
    limit: LimitValue,
) -> CapacityClaim {
    CapacityClaim {
        claim_id: claim_id.into(),
        resource_id: resource_id.into(),
        period_id: period_id.into(),
        kind,
        limit,
        evidence: fixture_evidence(claim_id),
    }
}

pub fn base_case() -> CapacityEnvelope {
    let resource_id = "ivp-terminal-a";
    let period_id = "2026-q4-window-001";
    let unit = Unit::Litres;
    let denominator_evidence = fixture_evidence("terminal-operator-capacity-basis");

    CapacityEnvelope {
        scenario_id: "synthetic-capacity-rights-versus-delivery".into(),
        resource_id: resource_id.into(),
        period_id: period_id.into(),
        unit,
        installed: fixture_claim(
            "installed-capacity",
            resource_id,
            period_id,
            ClaimKind::InstalledCapacity,
            LimitValue::Absolute(Quantity {
                amount: 1_000,
                unit,
            }),
        ),
        operable: fixture_claim(
            "operable-capacity",
            resource_id,
            period_id,
            ClaimKind::OperableCapacity,
            LimitValue::Absolute(Quantity { amount: 800, unit }),
        ),
        entitlement: fixture_claim(
            "access-entitlement",
            resource_id,
            period_id,
            ClaimKind::AccessEntitlement,
            LimitValue::Share {
                basis_points: 8_750,
                denominator_id: "terminal-allocation-basis-v1".into(),
                denominator: Quantity { amount: 800, unit },
                denominator_evidence: denominator_evidence.clone(),
            },
        ),
        provider_offer: fixture_claim(
            "provider-offer",
            resource_id,
            period_id,
            ClaimKind::ProviderOffer,
            LimitValue::Absolute(Quantity { amount: 750, unit }),
        ),
        existing_allocations: vec![
            ExistingAllocation {
                allocation_id: "reservation-001".into(),
                resource_id: resource_id.into(),
                period_id: period_id.into(),
                quantity: Quantity { amount: 200, unit },
                state: AllocationState::Reserved,
                evidence: fixture_evidence("reservation-001"),
            },
            ExistingAllocation {
                allocation_id: "commitment-002".into(),
                resource_id: resource_id.into(),
                period_id: period_id.into(),
                quantity: Quantity { amount: 100, unit },
                state: AllocationState::Committed,
                evidence: fixture_evidence("commitment-002"),
            },
            ExistingAllocation {
                allocation_id: "delivery-003".into(),
                resource_id: resource_id.into(),
                period_id: period_id.into(),
                quantity: Quantity { amount: 50, unit },
                state: AllocationState::Delivered,
                evidence: fixture_evidence("delivery-003"),
            },
            ExistingAllocation {
                allocation_id: "cancelled-004".into(),
                resource_id: resource_id.into(),
                period_id: period_id.into(),
                quantity: Quantity { amount: 75, unit },
                state: AllocationState::Cancelled,
                evidence: fixture_evidence("cancelled-004"),
            },
        ],
        candidate: CandidateRequest {
            request_id: "candidate-005".into(),
            resource_id: resource_id.into(),
            period_id: period_id.into(),
            quantity: Quantity { amount: 300, unit },
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn computes_physical_accessible_and_residual_capacity_separately() {
        let result = base_case().evaluate().unwrap();
        assert_eq!(result.installed_capacity, 1_000);
        assert_eq!(result.operable_capacity, 800);
        assert_eq!(result.physical_limit, 800);
        assert_eq!(result.entitlement_limit, 700);
        assert_eq!(result.provider_offer_limit, 750);
        assert_eq!(result.access_limited_capacity, 700);
        assert_eq!(result.existing_capacity_consumed, 350);
        assert_eq!(result.residual_schedulable_capacity, 350);
        assert_eq!(result.candidate_excess, 0);
        assert_eq!(
            result.disposition,
            Disposition::FitsWithinEvidenceBoundedCapacity
        );
        assert_eq!(
            result.counted_allocation_ids,
            vec!["reservation-001", "commitment-002", "delivery-003"]
        );
    }

    #[test]
    fn distinct_share_denominators_must_be_resolved_before_comparing_percentages() {
        let mut envelope = base_case();
        envelope.entitlement.limit = LimitValue::Share {
            basis_points: 1_000,
            denominator_id: "total-precinct-capacity".into(),
            denominator: Quantity {
                amount: 10_000,
                unit: Unit::Litres,
            },
            denominator_evidence: fixture_evidence("total-precinct-capacity"),
        };
        envelope.provider_offer.limit = LimitValue::Share {
            basis_points: 1_300,
            denominator_id: "operator-terminal-capacity".into(),
            denominator: Quantity {
                amount: 2_000,
                unit: Unit::Litres,
            },
            denominator_evidence: fixture_evidence("operator-terminal-capacity"),
        };
        envelope.existing_allocations.clear();
        let result = envelope.evaluate().unwrap();
        assert_eq!(result.entitlement_limit, 1_000);
        assert_eq!(result.provider_offer_limit, 260);
        assert_eq!(result.access_limited_capacity, 260);
    }

    #[test]
    fn percentage_without_a_resolved_denominator_cannot_be_evaluated() {
        let mut envelope = base_case();
        envelope.entitlement.limit = LimitValue::Share {
            basis_points: 1_000,
            denominator_id: "unknown-total-ivp-capacity".into(),
            denominator: Quantity {
                amount: 0,
                unit: Unit::Litres,
            },
            denominator_evidence: fixture_evidence("unknown-denominator"),
        };
        assert!(envelope.evaluate().is_err());
    }

    #[test]
    fn candidate_above_residual_capacity_is_reported_not_capped_or_authorized() {
        let mut envelope = base_case();
        envelope.candidate.quantity.amount = 500;
        let result = envelope.evaluate().unwrap();
        assert_eq!(result.residual_schedulable_capacity, 350);
        assert_eq!(result.candidate_excess, 150);
        assert_eq!(
            result.disposition,
            Disposition::ExceedsEvidenceBoundedCapacity
        );
    }

    #[test]
    fn provider_offer_is_a_separate_cap_from_entitlement() {
        let mut envelope = base_case();
        envelope.provider_offer.limit = LimitValue::Absolute(Quantity {
            amount: 400,
            unit: Unit::Litres,
        });
        let result = envelope.evaluate().unwrap();
        assert_eq!(result.entitlement_limit, 700);
        assert_eq!(result.access_limited_capacity, 400);
        assert_eq!(result.residual_schedulable_capacity, 50);
    }

    #[test]
    fn installed_capacity_does_not_override_a_lower_operable_capacity() {
        let mut envelope = base_case();
        envelope.operable.limit = LimitValue::Absolute(Quantity {
            amount: 500,
            unit: Unit::Litres,
        });
        let result = envelope.evaluate().unwrap();
        assert_eq!(result.physical_limit, 500);
        assert_eq!(result.residual_schedulable_capacity, 150);
    }

    #[test]
    fn unqualified_reported_access_claim_is_rejected() {
        let mut envelope = base_case();
        envelope.entitlement.evidence.evidence_class = EvidenceClass::Reported;
        assert!(envelope.evaluate().is_err());
    }

    #[test]
    fn absolute_access_claim_requires_external_authority_receipt_or_fixture() {
        let mut envelope = base_case();
        envelope.provider_offer.evidence = Evidence {
            source_id: "provider-offer".into(),
            source_uri: "https://provider.example/offer".into(),
            evidence_class: EvidenceClass::QualifiedAuthority,
            raw_sha256: Some("a".repeat(64)),
            authority_receipt_id: None,
        };
        assert!(envelope.evaluate().is_err());
    }

    #[test]
    fn external_authority_receipt_shape_is_required() {
        let mut envelope = base_case();
        envelope.entitlement.evidence = Evidence {
            source_id: "mycelix-verified-entitlement".into(),
            source_uri: "urn:mycelix:authority:receipt:123".into(),
            evidence_class: EvidenceClass::QualifiedAuthority,
            raw_sha256: Some("b".repeat(64)),
            authority_receipt_id: Some("receipt-123".into()),
        };
        let result = envelope.evaluate().unwrap();
        assert_eq!(result.entitlement_limit, 700);
    }

    #[test]
    fn cancelled_and_expired_allocations_do_not_consume_capacity() {
        let mut envelope = base_case();
        envelope.existing_allocations.retain(|allocation| {
            matches!(
                allocation.state,
                AllocationState::Cancelled | AllocationState::Expired
            )
        });
        let result = envelope.evaluate().unwrap();
        assert_eq!(result.existing_capacity_consumed, 0);
        assert_eq!(result.residual_schedulable_capacity, 700);
    }

    #[test]
    fn duplicate_allocation_ids_fail_closed_even_across_states() {
        let mut envelope = base_case();
        let duplicate = ExistingAllocation {
            allocation_id: "reservation-001".into(),
            resource_id: envelope.resource_id.clone(),
            period_id: envelope.period_id.clone(),
            quantity: Quantity {
                amount: 10,
                unit: Unit::Litres,
            },
            state: AllocationState::Committed,
            evidence: fixture_evidence("duplicate-allocation"),
        };
        envelope.existing_allocations.push(duplicate);
        assert!(envelope.evaluate().is_err());
    }

    #[test]
    fn existing_uses_above_accessible_capacity_are_rejected() {
        let mut envelope = base_case();
        envelope.existing_allocations.clear();
        envelope.existing_allocations.push(ExistingAllocation {
            allocation_id: "overbooked".into(),
            resource_id: envelope.resource_id.clone(),
            period_id: envelope.period_id.clone(),
            quantity: Quantity {
                amount: 701,
                unit: Unit::Litres,
            },
            state: AllocationState::Committed,
            evidence: fixture_evidence("overbooked"),
        });
        assert!(envelope.evaluate().is_err());
    }

    #[test]
    fn unit_and_period_mismatches_fail_closed() {
        let mut envelope = base_case();
        envelope.provider_offer.limit = LimitValue::Absolute(Quantity {
            amount: 100,
            unit: Unit::WattHours,
        });
        assert!(envelope.evaluate().is_err());

        let mut envelope = base_case();
        envelope.candidate.period_id = "different-window".into();
        assert!(envelope.evaluate().is_err());
    }

    #[test]
    fn operable_capacity_cannot_exceed_installed_capacity() {
        let mut envelope = base_case();
        envelope.operable.limit = LimitValue::Absolute(Quantity {
            amount: 1_001,
            unit: Unit::Litres,
        });
        assert!(envelope.evaluate().is_err());
    }

    #[test]
    fn share_rounding_is_conservative() {
        let mut envelope = base_case();
        envelope.entitlement.limit = LimitValue::Share {
            basis_points: 3_333,
            denominator_id: "small-capacity-base".into(),
            denominator: Quantity {
                amount: 101,
                unit: Unit::Litres,
            },
            denominator_evidence: fixture_evidence("small-capacity-base"),
        };
        envelope.existing_allocations.clear();
        let result = envelope.evaluate().unwrap();
        assert_eq!(result.entitlement_limit, 33);
    }
}
