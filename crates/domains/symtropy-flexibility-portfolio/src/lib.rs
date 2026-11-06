// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![forbid(unsafe_code)]

//! Empirical joint-delivery evaluation for flexible resource portfolios.
//!
//! Unlike a correlation estimator, this crate evaluates complete dispatch-event
//! outcomes against a portfolio commitment. Each event contains one outcome for
//! every enrolled asset and one observation for every declared failure domain.
//! Missing telemetry is an error, never a zero or presumed-success observation.
//!
//! Repeated dispatch episodes may share one independent incident-group ID. The
//! group's worst event margin determines group pass/fail, so several dispatches
//! during one heatwave are not presented as independent reliability trials.
//!
//! Failure-domain membership and observed domain state are descriptive evidence.
//! This crate does not infer causality, forecast future failure probabilities,
//! certify a balancing product, perform nodal deliverability analysis or compute
//! ELCC. It assumes per-asset delivered lower bounds were produced by upstream
//! qualified response and metrology evaluators.

use core::fmt;
use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

const BASIS_POINTS: f64 = 10_000.0;
const EPSILON: f64 = 1.0e-9;

/// Portfolio and source identity. Digests are supplied by the upstream evidence
/// owner and are not cryptographically authenticated by this crate.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PortfolioIdentity {
    portfolio_id: String,
    product_contract_id: String,
    asset_manifest_digest: String,
    failure_domain_manifest_digest: String,
}

impl PortfolioIdentity {
    pub fn new(
        portfolio_id: impl Into<String>,
        product_contract_id: impl Into<String>,
        asset_manifest_digest: impl Into<String>,
        failure_domain_manifest_digest: impl Into<String>,
    ) -> Result<Self, PortfolioResilienceError> {
        let identity = Self {
            portfolio_id: portfolio_id.into(),
            product_contract_id: product_contract_id.into(),
            asset_manifest_digest: asset_manifest_digest.into(),
            failure_domain_manifest_digest: failure_domain_manifest_digest.into(),
        };
        for (field, value) in [
            ("portfolio ID", identity.portfolio_id.as_str()),
            ("product contract ID", identity.product_contract_id.as_str()),
            (
                "asset manifest digest",
                identity.asset_manifest_digest.as_str(),
            ),
            (
                "failure-domain manifest digest",
                identity.failure_domain_manifest_digest.as_str(),
            ),
        ] {
            require_id(field, value)?;
        }
        Ok(identity)
    }
}

/// Product-specific portfolio success criteria. Rates are declared in basis
/// points; they are policy inputs, not universal grid-product rules.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PortfolioContract {
    product_contract_id: String,
    minimum_event_count: usize,
    minimum_independent_incident_groups: usize,
    minimum_event_pass_fraction_basis_points: u16,
    minimum_incident_group_pass_fraction_basis_points: u16,
    shortfall_tail_alpha_basis_points: u16,
}

impl PortfolioContract {
    pub fn new(
        product_contract_id: impl Into<String>,
        minimum_event_count: usize,
        minimum_independent_incident_groups: usize,
        minimum_event_pass_fraction_basis_points: u16,
        minimum_incident_group_pass_fraction_basis_points: u16,
        shortfall_tail_alpha_basis_points: u16,
    ) -> Result<Self, PortfolioResilienceError> {
        let product_contract_id = product_contract_id.into();
        require_id("product contract ID", &product_contract_id)?;
        if minimum_event_count == 0 || minimum_independent_incident_groups == 0 {
            return Err(PortfolioResilienceError::InvalidMinimumCount);
        }
        for (field, value) in [
            (
                "minimum event pass fraction",
                minimum_event_pass_fraction_basis_points,
            ),
            (
                "minimum incident-group pass fraction",
                minimum_incident_group_pass_fraction_basis_points,
            ),
            ("shortfall tail alpha", shortfall_tail_alpha_basis_points),
        ] {
            if value == 0 || value > 10_000 {
                return Err(PortfolioResilienceError::InvalidBasisPoints { field, value });
            }
        }
        Ok(Self {
            product_contract_id,
            minimum_event_count,
            minimum_independent_incident_groups,
            minimum_event_pass_fraction_basis_points,
            minimum_incident_group_pass_fraction_basis_points,
            shortfall_tail_alpha_basis_points,
        })
    }
}

/// A declared shared dependency or common stress domain. The kind is an
/// extensible source-defined taxonomy such as utility, weather, telecom,
/// control-plane, cloud, cooling, water, fuel, storage-control or software.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct FailureDomain {
    domain_id: String,
    domain_kind: String,
}

impl FailureDomain {
    pub fn new(
        domain_id: impl Into<String>,
        domain_kind: impl Into<String>,
    ) -> Result<Self, PortfolioResilienceError> {
        let domain = Self {
            domain_id: domain_id.into(),
            domain_kind: domain_kind.into(),
        };
        require_id("failure-domain ID", &domain.domain_id)?;
        require_id("failure-domain kind", &domain.domain_kind)?;
        Ok(domain)
    }
}

/// One enrolled resource and the capacity it offers under the pinned product.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PortfolioAsset {
    asset_id: String,
    site_id: String,
    pcc_id: String,
    measurement_boundary_id: String,
    offered_capacity_kw: f64,
    failure_domain_ids: Vec<String>,
    asset_evidence_id: String,
}

impl PortfolioAsset {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        asset_id: impl Into<String>,
        site_id: impl Into<String>,
        pcc_id: impl Into<String>,
        measurement_boundary_id: impl Into<String>,
        offered_capacity_kw: f64,
        failure_domain_ids: Vec<String>,
        asset_evidence_id: impl Into<String>,
    ) -> Result<Self, PortfolioResilienceError> {
        let asset = Self {
            asset_id: asset_id.into(),
            site_id: site_id.into(),
            pcc_id: pcc_id.into(),
            measurement_boundary_id: measurement_boundary_id.into(),
            offered_capacity_kw,
            failure_domain_ids,
            asset_evidence_id: asset_evidence_id.into(),
        };
        for (field, value) in [
            ("asset ID", asset.asset_id.as_str()),
            ("site ID", asset.site_id.as_str()),
            ("PCC ID", asset.pcc_id.as_str()),
            (
                "measurement boundary ID",
                asset.measurement_boundary_id.as_str(),
            ),
            ("asset evidence ID", asset.asset_evidence_id.as_str()),
        ] {
            require_id(field, value)?;
        }
        if !offered_capacity_kw.is_finite() || offered_capacity_kw <= 0.0 {
            return Err(PortfolioResilienceError::InvalidQuantity {
                field: "offered asset capacity",
                value: offered_capacity_kw,
            });
        }
        if asset.failure_domain_ids.is_empty() {
            return Err(PortfolioResilienceError::AssetHasNoFailureDomain {
                asset_id: asset.asset_id.clone(),
            });
        }
        let unique: BTreeSet<&str> = asset
            .failure_domain_ids
            .iter()
            .map(String::as_str)
            .collect();
        if unique.len() != asset.failure_domain_ids.len() {
            return Err(PortfolioResilienceError::DuplicateAssetFailureDomain {
                asset_id: asset.asset_id.clone(),
            });
        }
        for domain_id in &asset.failure_domain_ids {
            require_id("asset failure-domain ID", domain_id)?;
        }
        Ok(asset)
    }
}

/// Frozen portfolio catalogue used by every event in the panel.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PortfolioDefinition {
    identity: PortfolioIdentity,
    contract: PortfolioContract,
    assets: Vec<PortfolioAsset>,
    failure_domains: Vec<FailureDomain>,
    total_offered_capacity_kw: f64,
    largest_failure_domain_capacity_kw: f64,
    largest_failure_domain_share_basis_points: f64,
}

impl PortfolioDefinition {
    pub fn new(
        identity: PortfolioIdentity,
        contract: PortfolioContract,
        assets: Vec<PortfolioAsset>,
        failure_domains: Vec<FailureDomain>,
    ) -> Result<Self, PortfolioResilienceError> {
        if identity.product_contract_id != contract.product_contract_id {
            return Err(PortfolioResilienceError::ProductContractMismatch);
        }
        if assets.is_empty() || failure_domains.is_empty() {
            return Err(PortfolioResilienceError::EmptyPortfolioCatalogue);
        }
        let mut asset_ids = BTreeSet::new();
        let mut site_pcc_pairs = BTreeSet::new();
        for asset in &assets {
            if !asset_ids.insert(asset.asset_id.as_str()) {
                return Err(PortfolioResilienceError::DuplicateAsset {
                    asset_id: asset.asset_id.clone(),
                });
            }
            if !site_pcc_pairs.insert((asset.site_id.as_str(), asset.pcc_id.as_str())) {
                return Err(PortfolioResilienceError::DuplicateSitePcc {
                    site_id: asset.site_id.clone(),
                    pcc_id: asset.pcc_id.clone(),
                });
            }
        }
        let mut domains_by_id = BTreeMap::new();
        for domain in &failure_domains {
            if domains_by_id
                .insert(domain.domain_id.as_str(), domain)
                .is_some()
            {
                return Err(PortfolioResilienceError::DuplicateFailureDomain {
                    domain_id: domain.domain_id.clone(),
                });
            }
        }
        for asset in &assets {
            for domain_id in &asset.failure_domain_ids {
                if !domains_by_id.contains_key(domain_id.as_str()) {
                    return Err(PortfolioResilienceError::UnknownFailureDomain {
                        domain_id: domain_id.clone(),
                    });
                }
            }
        }

        let total_offered_capacity_kw = assets
            .iter()
            .map(|asset| asset.offered_capacity_kw)
            .sum::<f64>();
        if !total_offered_capacity_kw.is_finite() || total_offered_capacity_kw <= 0.0 {
            return Err(PortfolioResilienceError::NumericalOverflow(
                "total offered capacity",
            ));
        }

        // Domain memberships may overlap. Each domain's member capacity is
        // measured independently; only the maximum single-domain share is
        // reported. Domain shares must never be summed into total capacity.
        let largest_failure_domain_capacity_kw = failure_domains
            .iter()
            .map(|domain| {
                assets
                    .iter()
                    .filter(|asset| asset.failure_domain_ids.contains(&domain.domain_id))
                    .map(|asset| asset.offered_capacity_kw)
                    .sum::<f64>()
            })
            .fold(0.0, f64::max);
        let largest_failure_domain_share_basis_points =
            largest_failure_domain_capacity_kw / total_offered_capacity_kw * BASIS_POINTS;

        Ok(Self {
            identity,
            contract,
            assets,
            failure_domains,
            total_offered_capacity_kw,
            largest_failure_domain_capacity_kw,
            largest_failure_domain_share_basis_points,
        })
    }

    pub fn identity(&self) -> &PortfolioIdentity {
        &self.identity
    }

    pub fn assets(&self) -> &[PortfolioAsset] {
        &self.assets
    }

    pub fn total_offered_capacity_kw(&self) -> f64 {
        self.total_offered_capacity_kw
    }

    pub fn largest_failure_domain_capacity_kw(&self) -> f64 {
        self.largest_failure_domain_capacity_kw
    }

    pub fn largest_failure_domain_share_basis_points(&self) -> f64 {
        self.largest_failure_domain_share_basis_points
    }
}

/// Asset availability is an observed status, not a substitute for telemetry.
/// Unknown is explicit and contributes zero certified delivered lower-bound kW.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum AssetAvailability {
    Available,
    Unavailable,
    Unknown,
}

/// Observed lower-bound delivery for one enrolled asset in one event.
/// This crate assumes the value is derived from a separately qualified PCC
/// response/metrology receipt and does not recompute that measurement.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AssetDeliveryOutcome {
    asset_id: String,
    delivered_lower_bound_kw: f64,
    availability: AssetAvailability,
    response_receipt_id: String,
    source_digest: String,
}

impl AssetDeliveryOutcome {
    pub fn delivered_lower_bound_kw(&self) -> f64 {
        self.delivered_lower_bound_kw
    }

    pub const fn availability(&self) -> AssetAvailability {
        self.availability
    }

    pub fn new(
        asset_id: impl Into<String>,
        delivered_lower_bound_kw: f64,
        availability: AssetAvailability,
        response_receipt_id: impl Into<String>,
        source_digest: impl Into<String>,
    ) -> Result<Self, PortfolioResilienceError> {
        let outcome = Self {
            asset_id: asset_id.into(),
            delivered_lower_bound_kw,
            availability,
            response_receipt_id: response_receipt_id.into(),
            source_digest: source_digest.into(),
        };
        require_id("outcome asset ID", &outcome.asset_id)?;
        require_id("response receipt ID", &outcome.response_receipt_id)?;
        require_id("asset outcome source digest", &outcome.source_digest)?;
        if !delivered_lower_bound_kw.is_finite() || delivered_lower_bound_kw < 0.0 {
            return Err(PortfolioResilienceError::InvalidQuantity {
                field: "delivered lower-bound capacity",
                value: delivered_lower_bound_kw,
            });
        }
        if availability != AssetAvailability::Available && delivered_lower_bound_kw > 0.0 {
            return Err(PortfolioResilienceError::AvailabilityDeliveryMismatch {
                asset_id: outcome.asset_id.clone(),
                availability,
                delivered_lower_bound_kw,
            });
        }
        Ok(outcome)
    }
}

/// Observed state of a declared shared failure domain during an event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum FailureDomainState {
    Normal,
    Degraded,
    Failed,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FailureDomainObservation {
    domain_id: String,
    state: FailureDomainState,
    evidence_id: String,
    source_digest: String,
}

impl FailureDomainObservation {
    pub fn new(
        domain_id: impl Into<String>,
        state: FailureDomainState,
        evidence_id: impl Into<String>,
        source_digest: impl Into<String>,
    ) -> Result<Self, PortfolioResilienceError> {
        let observation = Self {
            domain_id: domain_id.into(),
            state,
            evidence_id: evidence_id.into(),
            source_digest: source_digest.into(),
        };
        require_id("observed failure-domain ID", &observation.domain_id)?;
        require_id("failure-domain evidence ID", &observation.evidence_id)?;
        require_id("failure-domain source digest", &observation.source_digest)?;
        Ok(observation)
    }
}

/// One event with an explicit independent stress-group identifier.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PortfolioStressEvent {
    event_id: String,
    incident_group_id: String,
    scenario_id: String,
    start_ns: u64,
    end_ns: u64,
    required_delivery_kw: f64,
    evidence_id: String,
    source_digest: String,
}

impl PortfolioStressEvent {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        event_id: impl Into<String>,
        incident_group_id: impl Into<String>,
        scenario_id: impl Into<String>,
        start_ns: u64,
        end_ns: u64,
        required_delivery_kw: f64,
        evidence_id: impl Into<String>,
        source_digest: impl Into<String>,
    ) -> Result<Self, PortfolioResilienceError> {
        let event = Self {
            event_id: event_id.into(),
            incident_group_id: incident_group_id.into(),
            scenario_id: scenario_id.into(),
            start_ns,
            end_ns,
            required_delivery_kw,
            evidence_id: evidence_id.into(),
            source_digest: source_digest.into(),
        };
        for (field, value) in [
            ("event ID", event.event_id.as_str()),
            ("incident group ID", event.incident_group_id.as_str()),
            ("scenario ID", event.scenario_id.as_str()),
            ("event evidence ID", event.evidence_id.as_str()),
            ("event source digest", event.source_digest.as_str()),
        ] {
            require_id(field, value)?;
        }
        if start_ns >= end_ns {
            return Err(PortfolioResilienceError::InvalidEventWindow { start_ns, end_ns });
        }
        if !required_delivery_kw.is_finite() || required_delivery_kw < 0.0 {
            return Err(PortfolioResilienceError::InvalidQuantity {
                field: "required portfolio delivery",
                value: required_delivery_kw,
            });
        }
        Ok(event)
    }
}

/// Complete panel row: all catalogue assets and all declared failure domains
/// must have exactly one observation for this event.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PortfolioEventObservation {
    pub event: PortfolioStressEvent,
    pub asset_outcomes: Vec<AssetDeliveryOutcome>,
    pub failure_domain_observations: Vec<FailureDomainObservation>,
}

impl PortfolioEventObservation {
    pub fn new(
        event: PortfolioStressEvent,
        asset_outcomes: Vec<AssetDeliveryOutcome>,
        failure_domain_observations: Vec<FailureDomainObservation>,
    ) -> Self {
        Self {
            event,
            asset_outcomes,
            failure_domain_observations,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct AssetOutcomeReceipt {
    pub asset_id: String,
    pub site_id: String,
    pub pcc_id: String,
    pub offered_capacity_kw: f64,
    pub delivered_lower_bound_kw: f64,
    pub availability: AssetAvailability,
    pub response_receipt_id: String,
    pub source_digest: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DomainObservationReceipt {
    pub domain_id: String,
    pub state: FailureDomainState,
    pub evidence_id: String,
    pub source_digest: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PortfolioEventReceipt {
    pub event_id: String,
    pub incident_group_id: String,
    pub scenario_id: String,
    pub start_ns: u64,
    pub end_ns: u64,
    pub offered_capacity_kw: f64,
    pub delivered_lower_bound_kw: f64,
    pub required_delivery_kw: f64,
    pub delivered_to_offered_fraction: f64,
    pub margin_kw: f64,
    pub shortfall_kw: f64,
    pub passed: bool,
    pub asset_outcomes: Vec<AssetOutcomeReceipt>,
    pub failure_domain_observations: Vec<DomainObservationReceipt>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct IncidentGroupReceipt {
    pub incident_group_id: String,
    pub scenario_id: String,
    pub event_count: usize,
    pub worst_margin_kw: f64,
    pub maximum_shortfall_kw: f64,
    pub passed: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ScenarioReceipt {
    pub scenario_id: String,
    pub independent_incident_group_count: usize,
    pub passed_incident_group_count: usize,
    pub incident_group_pass_fraction_basis_points: f64,
    pub event_count: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FailureDomainReceipt {
    pub domain_id: String,
    pub domain_kind: String,
    pub normal_event_count: usize,
    pub degraded_event_count: usize,
    pub failed_event_count: usize,
    pub unknown_event_count: usize,
    pub portfolio_passes_when_normal: usize,
    pub portfolio_passes_when_degraded: usize,
    pub portfolio_passes_when_failed: usize,
    pub portfolio_passes_when_unknown: usize,
    pub member_offered_capacity_kw: f64,
    /// Sum across observed events; it is not installed capacity.
    pub member_delivered_lower_bound_sum_kw: f64,
    pub member_delivery_to_offer_fraction: f64,
    /// Domain memberships can overlap; this capacity must not be summed
    /// across domain receipts.
    pub membership_overlaps_other_domains: bool,
}

/// Descriptive panel result. It reports observed event and incident-group
/// frequencies; it does not infer future probabilities or causal failure risk.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PortfolioResilienceReport {
    pub identity: PortfolioIdentity,
    pub product_contract_id: String,
    pub asset_count: usize,
    pub failure_domain_count: usize,
    pub event_count: usize,
    pub independent_incident_group_count: usize,
    pub total_offered_capacity_kw: f64,
    pub event_pass_fraction_basis_points: f64,
    pub incident_group_pass_fraction_basis_points: f64,
    pub passed_event_count: usize,
    pub passed_incident_group_count: usize,
    pub worst_incident_group_margin_kw: f64,
    pub maximum_event_shortfall_kw: f64,
    /// Mean of the worst alpha fraction of incident-group shortfalls.
    /// This is descriptive empirical tail loss, not a predictive CVaR estimate.
    pub empirical_upper_tail_mean_incident_shortfall_kw: f64,
    pub shortfall_tail_alpha_basis_points: u16,
    pub largest_failure_domain_capacity_kw: f64,
    pub largest_failure_domain_share_basis_points: f64,
    pub event_threshold_passed: bool,
    pub incident_group_threshold_passed: bool,
    pub complete_panel: bool,
    pub events: Vec<PortfolioEventReceipt>,
    pub incident_groups: Vec<IncidentGroupReceipt>,
    pub scenarios: Vec<ScenarioReceipt>,
    pub failure_domains: Vec<FailureDomainReceipt>,
    pub claim_boundary: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum PortfolioResilienceError {
    MissingIdentity {
        field: &'static str,
    },
    InvalidQuantity {
        field: &'static str,
        value: f64,
    },
    InvalidBasisPoints {
        field: &'static str,
        value: u16,
    },
    InvalidMinimumCount,
    EmptyPortfolioCatalogue,
    ProductContractMismatch,
    DuplicateAsset {
        asset_id: String,
    },
    DuplicateSitePcc {
        site_id: String,
        pcc_id: String,
    },
    DuplicateFailureDomain {
        domain_id: String,
    },
    UnknownFailureDomain {
        domain_id: String,
    },
    AssetHasNoFailureDomain {
        asset_id: String,
    },
    DuplicateAssetFailureDomain {
        asset_id: String,
    },
    DuplicateEvent {
        event_id: String,
    },
    DuplicateAssetOutcome {
        event_id: String,
        asset_id: String,
    },
    MissingAssetOutcome {
        event_id: String,
        asset_id: String,
    },
    UnknownAssetOutcome {
        event_id: String,
        asset_id: String,
    },
    DeliveryExceedsOffer {
        event_id: String,
        asset_id: String,
        delivered_kw: f64,
        offered_kw: f64,
    },
    AvailabilityDeliveryMismatch {
        asset_id: String,
        availability: AssetAvailability,
        delivered_lower_bound_kw: f64,
    },
    DuplicateDomainObservation {
        event_id: String,
        domain_id: String,
    },
    MissingDomainObservation {
        event_id: String,
        domain_id: String,
    },
    UnknownDomainObservation {
        event_id: String,
        domain_id: String,
    },
    DuplicateIncidentGroupScenario {
        incident_group_id: String,
    },
    InvalidEventWindow {
        start_ns: u64,
        end_ns: u64,
    },
    EmptyEventPanel,
    InsufficientEventCount {
        observed: usize,
        required: usize,
    },
    InsufficientIncidentGroups {
        observed: usize,
        required: usize,
    },
    NumericalOverflow(&'static str),
}

impl fmt::Display for PortfolioResilienceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingIdentity { field } => write!(f, "{field} must not be empty"),
            Self::InvalidQuantity { field, value } => {
                write!(f, "{field} must be finite and in range; got {value}")
            }
            Self::InvalidBasisPoints { field, value } => {
                write!(f, "{field} must be in 1..=10000 basis points; got {value}")
            }
            Self::InvalidMinimumCount => write!(
                f,
                "event and independent incident-group minima must be positive"
            ),
            Self::EmptyPortfolioCatalogue => write!(
                f,
                "portfolio requires at least one asset and one failure domain"
            ),
            Self::ProductContractMismatch => {
                write!(f, "portfolio identity and contract product IDs differ")
            }
            Self::DuplicateAsset { asset_id } => write!(f, "duplicate portfolio asset {asset_id}"),
            Self::DuplicateSitePcc { site_id, pcc_id } => {
                write!(f, "duplicate site/PCC pair {site_id}/{pcc_id}")
            }
            Self::DuplicateFailureDomain { domain_id } => {
                write!(f, "duplicate failure domain {domain_id}")
            }
            Self::UnknownFailureDomain { domain_id } => {
                write!(f, "asset references unknown failure domain {domain_id}")
            }
            Self::AssetHasNoFailureDomain { asset_id } => {
                write!(f, "asset {asset_id} has no declared failure domain")
            }
            Self::DuplicateAssetFailureDomain { asset_id } => {
                write!(f, "asset {asset_id} repeats a failure-domain membership")
            }
            Self::DuplicateEvent { event_id } => write!(f, "duplicate event ID {event_id}"),
            Self::DuplicateAssetOutcome { event_id, asset_id } => {
                write!(f, "event {event_id} repeats asset outcome {asset_id}")
            }
            Self::MissingAssetOutcome { event_id, asset_id } => write!(
                f,
                "event {event_id} omits enrolled asset outcome {asset_id}"
            ),
            Self::UnknownAssetOutcome { event_id, asset_id } => {
                write!(f, "event {event_id} includes unenrolled asset {asset_id}")
            }
            Self::DeliveryExceedsOffer {
                event_id,
                asset_id,
                delivered_kw,
                offered_kw,
            } => write!(
                f,
                "event {event_id} asset {asset_id} delivers lower bound {delivered_kw} kW above offer {offered_kw} kW"
            ),
            Self::AvailabilityDeliveryMismatch {
                asset_id,
                availability,
                delivered_lower_bound_kw,
            } => write!(
                f,
                "asset {asset_id} is {availability:?} but reports nonzero delivered lower bound {delivered_lower_bound_kw} kW"
            ),
            Self::DuplicateDomainObservation {
                event_id,
                domain_id,
            } => write!(
                f,
                "event {event_id} repeats failure-domain observation {domain_id}"
            ),
            Self::MissingDomainObservation {
                event_id,
                domain_id,
            } => write!(f, "event {event_id} omits failure-domain state {domain_id}"),
            Self::UnknownDomainObservation {
                event_id,
                domain_id,
            } => write!(
                f,
                "event {event_id} reports undeclared failure domain {domain_id}"
            ),
            Self::DuplicateIncidentGroupScenario { incident_group_id } => write!(
                f,
                "incident group {incident_group_id} spans inconsistent scenario IDs"
            ),
            Self::InvalidEventWindow { start_ns, end_ns } => write!(
                f,
                "event window must satisfy start < end; got {start_ns}..{end_ns}"
            ),
            Self::EmptyEventPanel => write!(f, "portfolio evaluation requires at least one event"),
            Self::InsufficientEventCount { observed, required } => write!(
                f,
                "observed {observed} events; contract requires {required}"
            ),
            Self::InsufficientIncidentGroups { observed, required } => write!(
                f,
                "observed {observed} independent incident groups; contract requires {required}"
            ),
            Self::NumericalOverflow(field) => write!(f, "numerical overflow computing {field}"),
        }
    }
}

impl std::error::Error for PortfolioResilienceError {}

pub fn evaluate_portfolio_resilience(
    portfolio: &PortfolioDefinition,
    observations: &[PortfolioEventObservation],
) -> Result<PortfolioResilienceReport, PortfolioResilienceError> {
    if observations.is_empty() {
        return Err(PortfolioResilienceError::EmptyEventPanel);
    }

    let asset_by_id: BTreeMap<&str, &PortfolioAsset> = portfolio
        .assets
        .iter()
        .map(|asset| (asset.asset_id.as_str(), asset))
        .collect();
    let domain_by_id: BTreeMap<&str, &FailureDomain> = portfolio
        .failure_domains
        .iter()
        .map(|domain| (domain.domain_id.as_str(), domain))
        .collect();

    let mut event_ids = BTreeSet::new();
    let mut group_scenarios: BTreeMap<&str, &str> = BTreeMap::new();
    let mut event_receipts = Vec::with_capacity(observations.len());
    let mut group_work: BTreeMap<&str, Vec<(f64, f64)>> = BTreeMap::new();
    let mut domain_work: BTreeMap<&str, DomainAccumulator> = BTreeMap::new();

    for observation in observations {
        let event = &observation.event;
        if !event_ids.insert(event.event_id.as_str()) {
            return Err(PortfolioResilienceError::DuplicateEvent {
                event_id: event.event_id.clone(),
            });
        }
        if let Some(previous_scenario) =
            group_scenarios.insert(event.incident_group_id.as_str(), event.scenario_id.as_str())
        {
            if previous_scenario != event.scenario_id {
                return Err(PortfolioResilienceError::DuplicateIncidentGroupScenario {
                    incident_group_id: event.incident_group_id.clone(),
                });
            }
        }

        let mut outcome_by_asset: BTreeMap<&str, &AssetDeliveryOutcome> = BTreeMap::new();
        for outcome in &observation.asset_outcomes {
            if !asset_by_id.contains_key(outcome.asset_id.as_str()) {
                return Err(PortfolioResilienceError::UnknownAssetOutcome {
                    event_id: event.event_id.clone(),
                    asset_id: outcome.asset_id.clone(),
                });
            }
            if outcome_by_asset
                .insert(outcome.asset_id.as_str(), outcome)
                .is_some()
            {
                return Err(PortfolioResilienceError::DuplicateAssetOutcome {
                    event_id: event.event_id.clone(),
                    asset_id: outcome.asset_id.clone(),
                });
            }
        }
        if outcome_by_asset.len() != portfolio.assets.len() {
            for asset in &portfolio.assets {
                if !outcome_by_asset.contains_key(asset.asset_id.as_str()) {
                    return Err(PortfolioResilienceError::MissingAssetOutcome {
                        event_id: event.event_id.clone(),
                        asset_id: asset.asset_id.clone(),
                    });
                }
            }
        }

        let mut domain_observation_by_id: BTreeMap<&str, &FailureDomainObservation> =
            BTreeMap::new();
        for domain_observation in &observation.failure_domain_observations {
            if !domain_by_id.contains_key(domain_observation.domain_id.as_str()) {
                return Err(PortfolioResilienceError::UnknownDomainObservation {
                    event_id: event.event_id.clone(),
                    domain_id: domain_observation.domain_id.clone(),
                });
            }
            if domain_observation_by_id
                .insert(domain_observation.domain_id.as_str(), domain_observation)
                .is_some()
            {
                return Err(PortfolioResilienceError::DuplicateDomainObservation {
                    event_id: event.event_id.clone(),
                    domain_id: domain_observation.domain_id.clone(),
                });
            }
        }
        if domain_observation_by_id.len() != portfolio.failure_domains.len() {
            for domain in &portfolio.failure_domains {
                if !domain_observation_by_id.contains_key(domain.domain_id.as_str()) {
                    return Err(PortfolioResilienceError::MissingDomainObservation {
                        event_id: event.event_id.clone(),
                        domain_id: domain.domain_id.clone(),
                    });
                }
            }
        }

        let mut delivered_lower_bound_kw = 0.0;
        for asset in &portfolio.assets {
            let outcome = outcome_by_asset
                .get(asset.asset_id.as_str())
                .expect("complete catalogue check precedes aggregation");
            if outcome.delivered_lower_bound_kw > asset.offered_capacity_kw {
                return Err(PortfolioResilienceError::DeliveryExceedsOffer {
                    event_id: event.event_id.clone(),
                    asset_id: asset.asset_id.clone(),
                    delivered_kw: outcome.delivered_lower_bound_kw,
                    offered_kw: asset.offered_capacity_kw,
                });
            }
            delivered_lower_bound_kw += outcome.delivered_lower_bound_kw;
        }
        let margin_kw = delivered_lower_bound_kw - event.required_delivery_kw;
        let shortfall_kw = (-margin_kw).max(0.0);
        let delivered_to_offered_fraction =
            delivered_lower_bound_kw / portfolio.total_offered_capacity_kw;
        if !delivered_lower_bound_kw.is_finite()
            || !margin_kw.is_finite()
            || !shortfall_kw.is_finite()
            || !delivered_to_offered_fraction.is_finite()
        {
            return Err(PortfolioResilienceError::NumericalOverflow("event totals"));
        }
        let passed = margin_kw + EPSILON >= 0.0;
        let mut asset_receipts = Vec::with_capacity(portfolio.assets.len());
        for asset in &portfolio.assets {
            let outcome = outcome_by_asset
                .get(asset.asset_id.as_str())
                .expect("complete catalogue check precedes receipt construction");
            asset_receipts.push(AssetOutcomeReceipt {
                asset_id: asset.asset_id.clone(),
                site_id: asset.site_id.clone(),
                pcc_id: asset.pcc_id.clone(),
                offered_capacity_kw: asset.offered_capacity_kw,
                delivered_lower_bound_kw: outcome.delivered_lower_bound_kw,
                availability: outcome.availability,
                response_receipt_id: outcome.response_receipt_id.clone(),
                source_digest: outcome.source_digest.clone(),
            });
        }
        asset_receipts.sort_by(|a, b| a.asset_id.cmp(&b.asset_id));
        let mut domain_receipts = Vec::with_capacity(portfolio.failure_domains.len());
        for domain in &portfolio.failure_domains {
            let observed = domain_observation_by_id
                .get(domain.domain_id.as_str())
                .expect("complete domain check precedes receipt construction");
            domain_receipts.push(DomainObservationReceipt {
                domain_id: domain.domain_id.clone(),
                state: observed.state,
                evidence_id: observed.evidence_id.clone(),
                source_digest: observed.source_digest.clone(),
            });
        }
        domain_receipts.sort_by(|a, b| a.domain_id.cmp(&b.domain_id));
        event_receipts.push(PortfolioEventReceipt {
            event_id: event.event_id.clone(),
            incident_group_id: event.incident_group_id.clone(),
            scenario_id: event.scenario_id.clone(),
            start_ns: event.start_ns,
            end_ns: event.end_ns,
            offered_capacity_kw: portfolio.total_offered_capacity_kw,
            delivered_lower_bound_kw,
            required_delivery_kw: event.required_delivery_kw,
            delivered_to_offered_fraction,
            margin_kw,
            shortfall_kw,
            passed,
            asset_outcomes: asset_receipts,
            failure_domain_observations: domain_receipts,
        });
        group_work
            .entry(event.incident_group_id.as_str())
            .or_default()
            .push((margin_kw, shortfall_kw));

        for domain in &portfolio.failure_domains {
            let state = domain_observation_by_id
                .get(domain.domain_id.as_str())
                .expect("complete domain check precedes aggregation")
                .state;
            let member_assets: Vec<&PortfolioAsset> = portfolio
                .assets
                .iter()
                .filter(|asset| asset.failure_domain_ids.contains(&domain.domain_id))
                .collect();
            let member_offered_kw = member_assets
                .iter()
                .map(|asset| asset.offered_capacity_kw)
                .sum::<f64>();
            let member_delivered_kw = member_assets
                .iter()
                .map(|asset| outcome_by_asset[asset.asset_id.as_str()].delivered_lower_bound_kw)
                .sum::<f64>();
            let accumulator = domain_work.entry(domain.domain_id.as_str()).or_default();
            accumulator.observe(state, passed, member_offered_kw, member_delivered_kw);
            if !accumulator.member_offered_kw_sum.is_finite()
                || !accumulator.member_delivered_kw_sum.is_finite()
            {
                return Err(PortfolioResilienceError::NumericalOverflow(
                    "failure-domain member totals",
                ));
            }
        }
    }

    if event_receipts.len() < portfolio.contract.minimum_event_count {
        return Err(PortfolioResilienceError::InsufficientEventCount {
            observed: event_receipts.len(),
            required: portfolio.contract.minimum_event_count,
        });
    }
    if group_work.len() < portfolio.contract.minimum_independent_incident_groups {
        return Err(PortfolioResilienceError::InsufficientIncidentGroups {
            observed: group_work.len(),
            required: portfolio.contract.minimum_independent_incident_groups,
        });
    }

    let mut group_receipts = Vec::with_capacity(group_work.len());
    let mut group_shortfalls = Vec::with_capacity(group_work.len());
    for (incident_group_id, values) in group_work {
        let worst_margin_kw = values
            .iter()
            .map(|value| value.0)
            .fold(f64::INFINITY, f64::min);
        let maximum_shortfall_kw = values.iter().map(|value| value.1).fold(0.0, f64::max);
        let passed = values.iter().all(|value| value.0 + EPSILON >= 0.0);
        group_shortfalls.push(maximum_shortfall_kw);
        group_receipts.push(IncidentGroupReceipt {
            incident_group_id: incident_group_id.to_owned(),
            scenario_id: (*group_scenarios
                .get(incident_group_id)
                .expect("group scenario inserted above"))
            .to_owned(),
            event_count: values.len(),
            worst_margin_kw,
            maximum_shortfall_kw,
            passed,
        });
    }
    group_receipts.sort_by(|a, b| a.incident_group_id.cmp(&b.incident_group_id));
    event_receipts.sort_by(|a, b| {
        a.start_ns
            .cmp(&b.start_ns)
            .then_with(|| a.event_id.cmp(&b.event_id))
    });

    let event_passed = event_receipts.iter().filter(|event| event.passed).count();
    let group_passed = group_receipts.iter().filter(|group| group.passed).count();
    let event_pass_fraction_basis_points =
        event_passed as f64 / event_receipts.len() as f64 * BASIS_POINTS;
    let incident_group_pass_fraction_basis_points =
        group_passed as f64 / group_receipts.len() as f64 * BASIS_POINTS;
    let worst_incident_group_margin_kw = group_receipts
        .iter()
        .map(|group| group.worst_margin_kw)
        .fold(f64::INFINITY, f64::min);
    let maximum_event_shortfall_kw = event_receipts
        .iter()
        .map(|event| event.shortfall_kw)
        .fold(0.0, f64::max);

    group_shortfalls.sort_by(|a, b| b.total_cmp(a));
    let tail_count = (group_shortfalls.len()
        * portfolio.contract.shortfall_tail_alpha_basis_points as usize)
        .div_ceil(10_000)
        .max(1)
        .min(group_shortfalls.len());
    let empirical_upper_tail_mean_incident_shortfall_kw =
        group_shortfalls.iter().take(tail_count).sum::<f64>() / tail_count as f64;
    if !empirical_upper_tail_mean_incident_shortfall_kw.is_finite() {
        return Err(PortfolioResilienceError::NumericalOverflow(
            "incident shortfall tail mean",
        ));
    }

    let mut scenarios_work: BTreeMap<&str, Vec<(&str, bool, usize)>> = BTreeMap::new();
    for group in &group_receipts {
        scenarios_work
            .entry(group.scenario_id.as_str())
            .or_default()
            .push((
                group.incident_group_id.as_str(),
                group.passed,
                group.event_count,
            ));
    }
    let mut scenarios = Vec::with_capacity(scenarios_work.len());
    for (scenario_id, groups) in scenarios_work {
        let passed_count = groups.iter().filter(|group| group.1).count();
        let event_count = groups.iter().map(|group| group.2).sum();
        scenarios.push(ScenarioReceipt {
            scenario_id: scenario_id.to_owned(),
            independent_incident_group_count: groups.len(),
            passed_incident_group_count: passed_count,
            incident_group_pass_fraction_basis_points: passed_count as f64 / groups.len() as f64
                * BASIS_POINTS,
            event_count,
        });
    }
    scenarios.sort_by(|a, b| a.scenario_id.cmp(&b.scenario_id));

    let mut failure_domains = Vec::with_capacity(portfolio.failure_domains.len());
    for domain in &portfolio.failure_domains {
        let accumulator = domain_work
            .get(domain.domain_id.as_str())
            .expect("each domain is observed for every event");
        let member_delivery_to_offer_fraction = if accumulator.member_offered_kw_sum > 0.0 {
            accumulator.member_delivered_kw_sum / accumulator.member_offered_kw_sum
        } else {
            0.0
        };
        failure_domains.push(FailureDomainReceipt {
            domain_id: domain.domain_id.clone(),
            domain_kind: domain.domain_kind.clone(),
            normal_event_count: accumulator.normal_count,
            degraded_event_count: accumulator.degraded_count,
            failed_event_count: accumulator.failed_count,
            unknown_event_count: accumulator.unknown_count,
            portfolio_passes_when_normal: accumulator.normal_pass_count,
            portfolio_passes_when_degraded: accumulator.degraded_pass_count,
            portfolio_passes_when_failed: accumulator.failed_pass_count,
            portfolio_passes_when_unknown: accumulator.unknown_pass_count,
            member_offered_capacity_kw: accumulator.member_offered_kw_sum
                / observations.len() as f64,
            member_delivered_lower_bound_sum_kw: accumulator.member_delivered_kw_sum,
            member_delivery_to_offer_fraction: member_delivery_to_offer_fraction,
            membership_overlaps_other_domains: portfolio.assets.iter().any(|asset| {
                asset.failure_domain_ids.contains(&domain.domain_id)
                    && asset.failure_domain_ids.len() > 1
            }),
        });
    }
    failure_domains.sort_by(|a, b| a.domain_id.cmp(&b.domain_id));

    let event_threshold_passed = event_pass_fraction_basis_points
        >= portfolio.contract.minimum_event_pass_fraction_basis_points as f64;
    let incident_group_threshold_passed = incident_group_pass_fraction_basis_points
        >= portfolio
            .contract
            .minimum_incident_group_pass_fraction_basis_points as f64;
    Ok(PortfolioResilienceReport {
        identity: portfolio.identity.clone(),
        product_contract_id: portfolio.contract.product_contract_id.clone(),
        asset_count: portfolio.assets.len(),
        failure_domain_count: portfolio.failure_domains.len(),
        event_count: observations.len(),
        independent_incident_group_count: group_receipts.len(),
        total_offered_capacity_kw: portfolio.total_offered_capacity_kw,
        event_pass_fraction_basis_points,
        incident_group_pass_fraction_basis_points,
        passed_event_count: event_passed,
        passed_incident_group_count: group_passed,
        worst_incident_group_margin_kw,
        maximum_event_shortfall_kw,
        empirical_upper_tail_mean_incident_shortfall_kw,
        shortfall_tail_alpha_basis_points: portfolio.contract.shortfall_tail_alpha_basis_points,
        largest_failure_domain_capacity_kw: portfolio.largest_failure_domain_capacity_kw,
        largest_failure_domain_share_basis_points: portfolio.largest_failure_domain_share_basis_points,
        event_threshold_passed,
        incident_group_threshold_passed,
        complete_panel: true,
        events: event_receipts,
        incident_groups: group_receipts,
        scenarios,
        failure_domains,
        claim_boundary: "descriptive empirical joint delivery over the supplied complete event panel; failure-domain states are not causal attribution; no future reliability probability, nodal deliverability, accreditation or ELCC is asserted".to_owned(),
    })
}

#[derive(Default)]
struct DomainAccumulator {
    normal_count: usize,
    degraded_count: usize,
    failed_count: usize,
    unknown_count: usize,
    normal_pass_count: usize,
    degraded_pass_count: usize,
    failed_pass_count: usize,
    unknown_pass_count: usize,
    member_offered_kw_sum: f64,
    member_delivered_kw_sum: f64,
}

impl DomainAccumulator {
    fn observe(
        &mut self,
        state: FailureDomainState,
        portfolio_passed: bool,
        member_offered_kw: f64,
        member_delivered_kw: f64,
    ) {
        match state {
            FailureDomainState::Normal => {
                self.normal_count += 1;
                self.normal_pass_count += usize::from(portfolio_passed);
            }
            FailureDomainState::Degraded => {
                self.degraded_count += 1;
                self.degraded_pass_count += usize::from(portfolio_passed);
            }
            FailureDomainState::Failed => {
                self.failed_count += 1;
                self.failed_pass_count += usize::from(portfolio_passed);
            }
            FailureDomainState::Unknown => {
                self.unknown_count += 1;
                self.unknown_pass_count += usize::from(portfolio_passed);
            }
        }
        self.member_offered_kw_sum += member_offered_kw;
        self.member_delivered_kw_sum += member_delivered_kw;
    }
}

fn require_id(field: &'static str, value: &str) -> Result<(), PortfolioResilienceError> {
    if value.trim().is_empty() {
        Err(PortfolioResilienceError::MissingIdentity { field })
    } else {
        Ok(())
    }
}
