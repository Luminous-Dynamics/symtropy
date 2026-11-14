//! Tiny exhaustive oracle for AeroWeft infrastructure common-cause fixtures.
//!
//! This reference oracle evaluates a finite portfolio of synthetic recovery
//! sites. It distinguishes landing availability from a complete repeat-service
//! chain that additionally requires energy and ground-transfer resources.
//!
//! It is not a site registry, investment optimizer, electrical solver, generic
//! reliability engine, aircraft model, or operational aviation system.
//! Failure sets are bounded adversarial sets; no probabilities are assigned.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::collections::BTreeSet;

/// Canonical synthetic failure domains in the frozen tiny portfolio.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FailureDomain {
    /// Shared coastal weather region affecting both north sites.
    WxCoastal,
    /// Shared upstream power feeder for both north chargers.
    PowerNorth,
    /// Shared bridge/access link for both north sites.
    BridgeNorth,
    /// Separate inland weather region for the south site.
    WxInland,
    /// Separate power feeder for the south charger.
    PowerSouth,
    /// Separate ground-access road for the south site.
    RoadSouth,
}

/// All domains in the frozen fixture in canonical order.
pub const ALL_FAILURE_DOMAINS: [FailureDomain; 6] = [
    FailureDomain::WxCoastal,
    FailureDomain::PowerNorth,
    FailureDomain::BridgeNorth,
    FailureDomain::WxInland,
    FailureDomain::PowerSouth,
    FailureDomain::RoadSouth,
];

/// A synthetic recovery site and its complete-chain dependencies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecoverySite {
    /// Stable fixture site identifier.
    pub site_id: &'static str,
    /// Whether the site record and status are positively admitted in this fixture.
    pub operational_status_qualified: bool,
    /// Failure domains that remove the declared landing/recovery capability.
    pub landing_failure_domains: &'static [FailureDomain],
    /// Failure domains that remove required energy service for the next sortie.
    pub energy_failure_domains: &'static [FailureDomain],
    /// Failure domains that remove required ground transfer.
    pub ground_transfer_failure_domains: &'static [FailureDomain],
}

/// A finite portfolio of candidate recovery sites.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecoveryPortfolio {
    /// Stable portfolio identifier.
    pub portfolio_id: &'static str,
    /// Selected sites in this portfolio.
    pub sites: &'static [RecoverySite],
}

/// Result error for an invalid finite oracle input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OracleError {
    /// The failure-domain universe contains duplicate identifiers.
    DuplicateFailureDomain,
    /// The finite universe exceeds the implementation's explicit mask bound.
    TooManyFailureDomains,
}

const WX_COASTAL: [FailureDomain; 1] = [FailureDomain::WxCoastal];
const POWER_NORTH: [FailureDomain; 1] = [FailureDomain::PowerNorth];
const BRIDGE_NORTH: [FailureDomain; 1] = [FailureDomain::BridgeNorth];
const WX_INLAND: [FailureDomain; 1] = [FailureDomain::WxInland];
const POWER_SOUTH: [FailureDomain; 1] = [FailureDomain::PowerSouth];
const ROAD_SOUTH: [FailureDomain; 1] = [FailureDomain::RoadSouth];

/// Synthetic north site A.
pub const SITE_NORTH_A: RecoverySite = RecoverySite {
    site_id: "site-north-a",
    operational_status_qualified: true,
    landing_failure_domains: &WX_COASTAL,
    energy_failure_domains: &POWER_NORTH,
    ground_transfer_failure_domains: &BRIDGE_NORTH,
};

/// Synthetic north site B, sharing all three failure domains with site A.
pub const SITE_NORTH_B: RecoverySite = RecoverySite {
    site_id: "site-north-b",
    operational_status_qualified: true,
    landing_failure_domains: &WX_COASTAL,
    energy_failure_domains: &POWER_NORTH,
    ground_transfer_failure_domains: &BRIDGE_NORTH,
};

/// Synthetic south site C with distinct declared failure domains.
pub const SITE_SOUTH_C: RecoverySite = RecoverySite {
    site_id: "site-south-c",
    operational_status_qualified: true,
    landing_failure_domains: &WX_INLAND,
    energy_failure_domains: &POWER_SOUTH,
    ground_transfer_failure_domains: &ROAD_SOUTH,
};

/// Synthetic geometry-only site that must never count as a recovery resource.
pub const SITE_UNKNOWN_STATUS: RecoverySite = RecoverySite {
    site_id: "site-unknown-status",
    operational_status_qualified: false,
    landing_failure_domains: &[],
    energy_failure_domains: &[],
    ground_transfer_failure_domains: &[],
};

const NORTH_SITES: [RecoverySite; 2] = [SITE_NORTH_A, SITE_NORTH_B];
const DIVERSE_SITES: [RecoverySite; 2] = [SITE_NORTH_A, SITE_SOUTH_C];
const NORTH_AB_PLUS_C_SITES: [RecoverySite; 3] =
    [SITE_NORTH_A, SITE_NORTH_B, SITE_SOUTH_C];
const UNKNOWN_ONLY: [RecoverySite; 1] = [SITE_UNKNOWN_STATUS];

/// Two nominally separate north sites sharing weather, power and access.
pub const PORTFOLIO_NORTH_AB: RecoveryPortfolio = RecoveryPortfolio {
    portfolio_id: "nominal-coverage-a-plus-b",
    sites: &NORTH_SITES,
};

/// Two sites with distinct declared weather, power and access domains.
pub const PORTFOLIO_DIVERSE_AC: RecoveryPortfolio = RecoveryPortfolio {
    portfolio_id: "recovery-diverse-a-plus-c",
    sites: &DIVERSE_SITES,
};

/// Portfolio containing only a geometry-only unknown-status site.
pub const PORTFOLIO_NORTH_AB_PLUS_C: RecoveryPortfolio = RecoveryPortfolio {
    portfolio_id: "north-a-b-plus-south-c",
    sites: &NORTH_AB_PLUS_C_SITES,
};

pub const PORTFOLIO_UNKNOWN_ONLY: RecoveryPortfolio = RecoveryPortfolio {
    portfolio_id: "unknown-status-only",
    sites: &UNKNOWN_ONLY,
};

fn is_failed(
    domains: &[FailureDomain],
    failed: &BTreeSet<FailureDomain>,
) -> bool {
    domains.iter().any(|domain| failed.contains(domain))
}

/// Whether a site can still provide its declared landing/recovery function.
///
/// Energy and ground transfer are deliberately not part of this predicate.
pub fn site_landing_available(
    site: &RecoverySite,
    failed: &BTreeSet<FailureDomain>,
) -> bool {
    site.operational_status_qualified
        && !is_failed(site.landing_failure_domains, failed)
}

/// Whether a site still provides a complete repeat-service chain.
///
/// The chain requires a qualified landing site, energy for the next sortie,
/// and ground transfer. A power failure alone does not invalidate the landing
/// surface; it invalidates this complete-chain claim when energy is required.
pub fn site_complete_chain_available(
    site: &RecoverySite,
    failed: &BTreeSet<FailureDomain>,
) -> bool {
    site_landing_available(site, failed)
        && !is_failed(site.energy_failure_domains, failed)
        && !is_failed(site.ground_transfer_failure_domains, failed)
}

/// Whether any site in a portfolio retains a complete service chain.
pub fn portfolio_has_complete_chain(
    portfolio: &RecoveryPortfolio,
    failed: &BTreeSet<FailureDomain>,
) -> bool {
    portfolio
        .sites
        .iter()
        .any(|site| site_complete_chain_available(site, failed))
}

/// Enumerate every inclusion-minimal failure set that removes all complete chains.
///
/// The universe is finite and explicit. A failure set is minimal only if it
/// disables the portfolio and removing any one member restores at least one
/// complete chain. Under the monotone site-dependency contract, this immediate
/// predecessor test is sufficient to establish inclusion minimality.
pub fn minimal_failure_sets(
    portfolio: &RecoveryPortfolio,
    universe: &[FailureDomain],
) -> Result<Vec<Vec<FailureDomain>>, OracleError> {
    let mut seen = BTreeSet::new();
    for domain in universe {
        if !seen.insert(*domain) {
            return Err(OracleError::DuplicateFailureDomain);
        }
    }

    // Keep the exhaustive reference bounded to at most 2^16 assignments.
    if universe.len() > 16 {
        return Err(OracleError::TooManyFailureDomains);
    }

    let assignment_count = 1usize << universe.len();
    let mut result = Vec::new();

    for mask in 0..assignment_count {
        let failed: BTreeSet<FailureDomain> = universe
            .iter()
            .enumerate()
            .filter_map(|(index, domain)| {
                ((mask & (1usize << index)) != 0).then_some(*domain)
            })
            .collect();

        if portfolio_has_complete_chain(portfolio, &failed) {
            continue;
        }

        let mut minimal = true;
        for domain in &failed {
            let mut predecessor = failed.clone();
            predecessor.remove(domain);
            if !portfolio_has_complete_chain(portfolio, &predecessor) {
                minimal = false;
                break;
            }
        }

        if minimal {
            result.push(failed.into_iter().collect());
        }
    }

    result.sort();
    Ok(result)
}

/// Construct a typed failure set for a declared scenario.
pub fn failure_set(domains: &[FailureDomain]) -> BTreeSet<FailureDomain> {
    domains.iter().copied().collect()
}
