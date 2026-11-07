// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

#![forbid(unsafe_code)]

//! Premise-trace evaluation over frozen multi-agent outcome evidence.
//!
//! This crate evaluates explicitly declared normative premises. It does not
//! derive morality from thermodynamics, choose an authoritative framework,
//! simulate human preferences, or emit a universal welfare score.
//!
//! Resource stocks and conservation receipts are inputs from the existing
//! resource-ledger/evidence boundary. This crate does not recalculate them.

use core::fmt;
use std::collections::BTreeSet;

const TOLERANCE: f64 = 1e-9;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ResourceKind {
    Water,
    Energy,
    Food,
}

impl ResourceKind {
    pub const ALL: [Self; 3] = [Self::Water, Self::Energy, Self::Food];

    pub const fn name(self) -> &'static str {
        match self {
            Self::Water => "water",
            Self::Energy => "energy",
            Self::Food => "food",
        }
    }

    pub const fn unit(self) -> &'static str {
        match self {
            Self::Water => "m3",
            Self::Energy => "kWh",
            Self::Food => "kg",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResourceState {
    pub resource: ResourceKind,
    pub opening_stock: f64,
    pub closing_stock: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LedgerReceipt {
    pub receipt_id: String,
    pub resource: ResourceKind,
    pub unit: String,
    /// Opaque source-provided digest; this crate does not verify its signature.
    pub source_digest: String,
    pub source_verified: bool,
    pub conservation_residual: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GroupOutcome {
    pub group_id: String,
    pub water_coverage: Option<f64>,
    pub energy_coverage: Option<f64>,
    pub food_coverage: Option<f64>,
    /// Share of declared total burden assigned to this group, when measured.
    pub burden_share: Option<f64>,
    /// Whether a meaningful, non-coercive exit route was recorded as available.
    pub exit_available: Option<bool>,
}

impl GroupOutcome {
    pub fn coverage(&self, resource: ResourceKind) -> Option<f64> {
        match resource {
            ResourceKind::Water => self.water_coverage,
            ResourceKind::Energy => self.energy_coverage,
            ResourceKind::Food => self.food_coverage,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TransferOutcome {
    pub transfer_id: String,
    pub recipient_group_id: String,
    pub resource: ResourceKind,
    pub amount: f64,
    /// None means consent was not recorded; it is not equivalent to consent.
    pub recipient_consent: Option<bool>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OutcomeSnapshot {
    pub snapshot_id: String,
    pub resources: Vec<ResourceState>,
    pub ledger_receipts: Vec<LedgerReceipt>,
    pub groups: Vec<GroupOutcome>,
    pub transfers: Vec<TransferOutcome>,
}

impl OutcomeSnapshot {
    pub fn validate(&self) -> Result<(), AuditError> {
        require_id("snapshot_id", &self.snapshot_id)?;
        if self.groups.len() < 2 {
            return Err(AuditError("at least two groups are required".into()));
        }

        let mut resources = BTreeSet::new();
        for state in &self.resources {
            if !state.opening_stock.is_finite()
                || !state.closing_stock.is_finite()
                || state.opening_stock < 0.0
                || state.closing_stock < 0.0
            {
                return Err(AuditError(format!(
                    "{} stock must be finite and non-negative",
                    state.resource.name()
                )));
            }
            if !resources.insert(state.resource) {
                return Err(AuditError(format!(
                    "duplicate {} resource state",
                    state.resource.name()
                )));
            }
        }

        let mut receipt_ids = BTreeSet::new();
        for receipt in &self.ledger_receipts {
            require_id("receipt_id", &receipt.receipt_id)?;
            require_id("source_digest", &receipt.source_digest)?;
            if !receipt_ids.insert(receipt.receipt_id.as_str()) {
                return Err(AuditError(format!(
                    "duplicate receipt ID {}",
                    receipt.receipt_id
                )));
            }
            if receipt.unit != receipt.resource.unit() {
                return Err(AuditError(format!(
                    "{} receipt unit must be {}, got {}",
                    receipt.resource.name(),
                    receipt.resource.unit(),
                    receipt.unit
                )));
            }
            if !receipt.conservation_residual.is_finite() || receipt.conservation_residual < 0.0 {
                return Err(AuditError(format!(
                    "{} conservation residual must be finite and non-negative",
                    receipt.resource.name()
                )));
            }
        }

        let mut group_ids = BTreeSet::new();
        let mut all_burdens_present = true;
        let mut total_burden = 0.0;
        for group in &self.groups {
            require_id("group_id", &group.group_id)?;
            if !group_ids.insert(group.group_id.as_str()) {
                return Err(AuditError(format!("duplicate group ID {}", group.group_id)));
            }
            for (name, value) in [
                ("water_coverage", group.water_coverage),
                ("energy_coverage", group.energy_coverage),
                ("food_coverage", group.food_coverage),
                ("burden_share", group.burden_share),
            ] {
                if let Some(value) = value {
                    if !value.is_finite() || !(0.0..=1.0).contains(&value) {
                        return Err(AuditError(format!(
                            "{}.{name} must be finite and in [0, 1]",
                            group.group_id
                        )));
                    }
                } else if name == "burden_share" {
                    all_burdens_present = false;
                }
            }
            if let Some(burden) = group.burden_share {
                total_burden += burden;
            }
        }
        if all_burdens_present && (total_burden - 1.0).abs() > TOLERANCE {
            return Err(AuditError(format!(
                "complete burden shares must sum to one, got {total_burden}"
            )));
        }

        let mut transfer_ids = BTreeSet::new();
        for transfer in &self.transfers {
            require_id("transfer_id", &transfer.transfer_id)?;
            if !transfer_ids.insert(transfer.transfer_id.as_str()) {
                return Err(AuditError(format!(
                    "duplicate transfer ID {}",
                    transfer.transfer_id
                )));
            }
            if !group_ids.contains(transfer.recipient_group_id.as_str()) {
                return Err(AuditError(format!(
                    "transfer {} references unknown recipient group {}",
                    transfer.transfer_id, transfer.recipient_group_id
                )));
            }
            if !transfer.amount.is_finite() || transfer.amount < 0.0 {
                return Err(AuditError(format!(
                    "transfer {} amount must be finite and non-negative",
                    transfer.transfer_id
                )));
            }
        }

        Ok(())
    }
}

fn require_id(field: &str, value: &str) -> Result<(), AuditError> {
    if value.trim().is_empty() {
        Err(AuditError(format!("{field} must not be empty")))
    } else {
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum PremiseRule {
    /// Aggregate-consequence premise: the unweighted mean must meet the floor.
    MeanServiceFloor {
        resource: ResourceKind,
        minimum: f64,
    },
    /// Equal-consideration premise: every declared group must meet the floor.
    EveryGroupServiceFloor {
        resource: ResourceKind,
        minimum: f64,
    },
    MaximumServiceGap {
        resource: ResourceKind,
        maximum: f64,
    },
    BurdenShareCap {
        maximum: f64,
    },
    RequireConsentForTransfers,
    RequireExitForEveryGroup,
    ResourceClosingFloor {
        resource: ResourceKind,
        minimum_stock: f64,
    },
    NoNetResourceDepletion {
        resource: ResourceKind,
    },
    /// Requires a source-verified, unit-correct receipt within the stated tolerance.
    LedgerConservationQualified {
        resources: Vec<ResourceKind>,
        maximum_residual: f64,
    },
}

impl PremiseRule {
    fn validate(&self) -> Result<(), AuditError> {
        let valid_fraction = |value: f64| value.is_finite() && (0.0..=1.0).contains(&value);
        match self {
            Self::MeanServiceFloor { minimum, .. }
            | Self::EveryGroupServiceFloor { minimum, .. } => {
                if !valid_fraction(*minimum) {
                    return Err(AuditError("service floor must be in [0, 1]".into()));
                }
            }
            Self::MaximumServiceGap { maximum, .. } => {
                if !valid_fraction(*maximum) {
                    return Err(AuditError("maximum service gap must be in [0, 1]".into()));
                }
            }
            Self::BurdenShareCap { maximum } => {
                if !valid_fraction(*maximum) {
                    return Err(AuditError("burden cap must be in [0, 1]".into()));
                }
            }
            Self::ResourceClosingFloor { minimum_stock, .. } => {
                if !minimum_stock.is_finite() || *minimum_stock < 0.0 {
                    return Err(AuditError(
                        "resource closing floor must be finite and non-negative".into(),
                    ));
                }
            }
            Self::LedgerConservationQualified {
                resources,
                maximum_residual,
            } => {
                if resources.is_empty() {
                    return Err(AuditError(
                        "ledger qualification requires at least one resource".into(),
                    ));
                }
                let mut unique = BTreeSet::new();
                if resources.iter().any(|resource| !unique.insert(*resource)) {
                    return Err(AuditError(
                        "ledger qualification resource list contains duplicates".into(),
                    ));
                }
                if !maximum_residual.is_finite() || *maximum_residual < 0.0 {
                    return Err(AuditError(
                        "maximum conservation residual must be finite and non-negative".into(),
                    ));
                }
            }
            Self::RequireConsentForTransfers
            | Self::RequireExitForEveryGroup
            | Self::NoNetResourceDepletion { .. } => {}
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Premise {
    pub premise_id: String,
    pub rule: PremiseRule,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PremiseProfile {
    pub profile_id: String,
    pub premises: Vec<Premise>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FindingStatus {
    Supported,
    Contradicted,
    Undetermined,
}

impl FindingStatus {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Supported => "supported",
            Self::Contradicted => "contradicted",
            Self::Undetermined => "undetermined",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PremiseFinding {
    pub premise_id: String,
    pub status: FindingStatus,
    pub evidence_refs: Vec<String>,
    pub explanation: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProfileFinding {
    pub profile_id: String,
    pub status: FindingStatus,
    pub premises: Vec<PremiseFinding>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AuditReport {
    pub snapshot_id: String,
    /// Profiles are sorted by ID; each premise vector is also sorted by premise ID.
    pub profiles: Vec<ProfileFinding>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AuditError(pub String);

impl fmt::Display for AuditError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for AuditError {}

pub fn evaluate_profiles(
    snapshot: &OutcomeSnapshot,
    profiles: &[PremiseProfile],
) -> Result<AuditReport, AuditError> {
    snapshot.validate()?;
    if profiles.is_empty() {
        return Err(AuditError(
            "at least one premise profile is required".into(),
        ));
    }

    let mut profile_ids = BTreeSet::new();
    let mut profile_results = Vec::with_capacity(profiles.len());
    for profile in profiles {
        require_id("profile_id", &profile.profile_id)?;
        if !profile_ids.insert(profile.profile_id.as_str()) {
            return Err(AuditError(format!(
                "duplicate profile ID {}",
                profile.profile_id
            )));
        }
        if profile.premises.is_empty() {
            return Err(AuditError(format!(
                "profile {} must contain at least one premise",
                profile.profile_id
            )));
        }

        let mut premise_ids = BTreeSet::new();
        let mut findings = Vec::with_capacity(profile.premises.len());
        for premise in &profile.premises {
            require_id("premise_id", &premise.premise_id)?;
            if !premise_ids.insert(premise.premise_id.as_str()) {
                return Err(AuditError(format!(
                    "profile {} has duplicate premise {}",
                    profile.profile_id, premise.premise_id
                )));
            }
            premise.rule.validate()?;
            findings.push(evaluate_premise(snapshot, premise));
        }
        findings.sort_by(|left, right| left.premise_id.cmp(&right.premise_id));
        let status = combine_status(findings.iter().map(|finding| finding.status));
        profile_results.push(ProfileFinding {
            profile_id: profile.profile_id.clone(),
            status,
            premises: findings,
        });
    }

    profile_results.sort_by(|left, right| left.profile_id.cmp(&right.profile_id));
    Ok(AuditReport {
        snapshot_id: snapshot.snapshot_id.clone(),
        profiles: profile_results,
    })
}

fn combine_status(statuses: impl Iterator<Item = FindingStatus>) -> FindingStatus {
    let mut has_unknown = false;
    for status in statuses {
        match status {
            FindingStatus::Contradicted => return FindingStatus::Contradicted,
            FindingStatus::Undetermined => has_unknown = true,
            FindingStatus::Supported => {}
        }
    }
    if has_unknown {
        FindingStatus::Undetermined
    } else {
        FindingStatus::Supported
    }
}

fn finding(
    premise: &Premise,
    status: FindingStatus,
    refs: Vec<String>,
    explanation: impl Into<String>,
) -> PremiseFinding {
    let mut refs = refs;
    refs.sort();
    refs.dedup();
    PremiseFinding {
        premise_id: premise.premise_id.clone(),
        status,
        evidence_refs: refs,
        explanation: explanation.into(),
    }
}

fn evaluate_premise(snapshot: &OutcomeSnapshot, premise: &Premise) -> PremiseFinding {
    match &premise.rule {
        PremiseRule::MeanServiceFloor { resource, minimum } => {
            let values = snapshot
                .groups
                .iter()
                .map(|group| group.coverage(*resource))
                .collect::<Vec<_>>();
            if values.iter().any(Option::is_none) {
                return finding(
                    premise,
                    FindingStatus::Undetermined,
                    group_refs(snapshot),
                    format!(
                        "mean {} service cannot be established because one or more groups are unmeasured",
                        resource.name()
                    ),
                );
            }
            let mean = values.iter().map(|value| value.unwrap()).sum::<f64>() / values.len() as f64;
            let status = compare_lower(mean, *minimum);
            finding(
                premise,
                status,
                group_refs(snapshot),
                format!(
                    "unweighted mean {} service is {mean:.6}; declared minimum is {minimum:.6}",
                    resource.name()
                ),
            )
        }
        PremiseRule::EveryGroupServiceFloor { resource, minimum } => {
            let mut missing = Vec::new();
            let mut failures = Vec::new();
            for group in &snapshot.groups {
                match group.coverage(*resource) {
                    Some(value) if value + TOLERANCE < *minimum => {
                        failures.push(format!("{}={value:.6}", group.group_id));
                    }
                    Some(_) => {}
                    None => missing.push(group.group_id.clone()),
                }
            }
            if !failures.is_empty() {
                finding(
                    premise,
                    FindingStatus::Contradicted,
                    group_refs(snapshot),
                    format!(
                        "{} group(s) are below the {} service floor {:.6}: {}",
                        failures.len(),
                        resource.name(),
                        minimum,
                        failures.join("; ")
                    ),
                )
            } else if !missing.is_empty() {
                finding(
                    premise,
                    FindingStatus::Undetermined,
                    group_refs(snapshot),
                    format!(
                        "{} service is unmeasured for groups: {}",
                        resource.name(),
                        missing.join(", ")
                    ),
                )
            } else {
                finding(
                    premise,
                    FindingStatus::Supported,
                    group_refs(snapshot),
                    format!(
                        "every group meets the {} service floor {:.6}",
                        resource.name(),
                        minimum
                    ),
                )
            }
        }
        PremiseRule::MaximumServiceGap { resource, maximum } => {
            let values = snapshot
                .groups
                .iter()
                .map(|group| group.coverage(*resource))
                .collect::<Vec<_>>();
            if values.iter().any(Option::is_none) {
                return finding(
                    premise,
                    FindingStatus::Undetermined,
                    group_refs(snapshot),
                    format!(
                        "{} service gap is unknown because one or more groups are unmeasured",
                        resource.name()
                    ),
                );
            }
            let min = values
                .iter()
                .map(|value| value.unwrap())
                .fold(1.0, f64::min);
            let max = values
                .iter()
                .map(|value| value.unwrap())
                .fold(0.0, f64::max);
            let gap = max - min;
            finding(
                premise,
                compare_upper(gap, *maximum),
                group_refs(snapshot),
                format!(
                    "{} service gap is {gap:.6}; declared maximum is {maximum:.6}",
                    resource.name()
                ),
            )
        }
        PremiseRule::BurdenShareCap { maximum } => {
            let mut missing = Vec::new();
            let mut over_cap = Vec::new();
            for group in &snapshot.groups {
                match group.burden_share {
                    Some(value) if value > *maximum + TOLERANCE => {
                        over_cap.push(format!("{}={value:.6}", group.group_id));
                    }
                    Some(_) => {}
                    None => missing.push(group.group_id.clone()),
                }
            }
            if !over_cap.is_empty() {
                finding(
                    premise,
                    FindingStatus::Contradicted,
                    group_refs(snapshot),
                    format!(
                        "burden cap {:.6} is exceeded by {}",
                        maximum,
                        over_cap.join("; ")
                    ),
                )
            } else if !missing.is_empty() {
                finding(
                    premise,
                    FindingStatus::Undetermined,
                    group_refs(snapshot),
                    format!("burden share is unmeasured for {}", missing.join(", ")),
                )
            } else {
                finding(
                    premise,
                    FindingStatus::Supported,
                    group_refs(snapshot),
                    format!("all measured group burdens are at or below {maximum:.6}"),
                )
            }
        }
        PremiseRule::RequireConsentForTransfers => {
            let denied = snapshot
                .transfers
                .iter()
                .filter(|transfer| transfer.recipient_consent == Some(false))
                .map(|transfer| transfer.transfer_id.clone())
                .collect::<Vec<_>>();
            let missing = snapshot
                .transfers
                .iter()
                .filter(|transfer| transfer.recipient_consent.is_none())
                .map(|transfer| transfer.transfer_id.clone())
                .collect::<Vec<_>>();
            if !denied.is_empty() {
                finding(
                    premise,
                    FindingStatus::Contradicted,
                    denied,
                    "one or more transfers have recorded recipient non-consent",
                )
            } else if !missing.is_empty() {
                finding(
                    premise,
                    FindingStatus::Undetermined,
                    missing,
                    "consent is not recorded for one or more transfers",
                )
            } else {
                finding(
                    premise,
                    FindingStatus::Supported,
                    snapshot
                        .transfers
                        .iter()
                        .map(|transfer| transfer.transfer_id.clone())
                        .collect(),
                    "all recorded transfers have recipient consent",
                )
            }
        }
        PremiseRule::RequireExitForEveryGroup => {
            let unavailable = snapshot
                .groups
                .iter()
                .filter(|group| group.exit_available == Some(false))
                .map(|group| group.group_id.clone())
                .collect::<Vec<_>>();
            let missing = snapshot
                .groups
                .iter()
                .filter(|group| group.exit_available.is_none())
                .map(|group| group.group_id.clone())
                .collect::<Vec<_>>();
            if !unavailable.is_empty() {
                finding(
                    premise,
                    FindingStatus::Contradicted,
                    unavailable,
                    "one or more groups have no recorded exit route",
                )
            } else if !missing.is_empty() {
                finding(
                    premise,
                    FindingStatus::Undetermined,
                    missing,
                    "exit availability is not recorded for one or more groups",
                )
            } else {
                finding(
                    premise,
                    FindingStatus::Supported,
                    group_refs(snapshot),
                    "every group has a recorded exit route",
                )
            }
        }
        PremiseRule::ResourceClosingFloor {
            resource,
            minimum_stock,
        } => {
            let Some(state) = resource_state(snapshot, *resource) else {
                return finding(
                    premise,
                    FindingStatus::Undetermined,
                    Vec::new(),
                    format!("{} closing stock is not recorded", resource.name()),
                );
            };
            finding(
                premise,
                compare_lower(state.closing_stock, *minimum_stock),
                vec![resource_ref(*resource)],
                format!(
                    "{} closing stock is {:.6} {}; declared floor is {:.6} {}",
                    resource.name(),
                    state.closing_stock,
                    resource.unit(),
                    minimum_stock,
                    resource.unit()
                ),
            )
        }
        PremiseRule::NoNetResourceDepletion { resource } => {
            let Some(state) = resource_state(snapshot, *resource) else {
                return finding(
                    premise,
                    FindingStatus::Undetermined,
                    Vec::new(),
                    format!("{} opening/closing stock is not recorded", resource.name()),
                );
            };
            finding(
                premise,
                compare_lower(state.closing_stock, state.opening_stock),
                vec![resource_ref(*resource)],
                format!(
                    "{} stock changed from {:.6} to {:.6} {}",
                    resource.name(),
                    state.opening_stock,
                    state.closing_stock,
                    resource.unit()
                ),
            )
        }
        PremiseRule::LedgerConservationQualified {
            resources,
            maximum_residual,
        } => evaluate_ledger_qualification(snapshot, premise, resources, *maximum_residual),
    }
}

fn evaluate_ledger_qualification(
    snapshot: &OutcomeSnapshot,
    premise: &Premise,
    resources: &[ResourceKind],
    maximum_residual: f64,
) -> PremiseFinding {
    let mut refs = Vec::new();
    let mut missing = Vec::new();
    let mut unverified = Vec::new();
    let mut violations = Vec::new();

    for resource in resources {
        if resource_state(snapshot, *resource).is_none() {
            missing.push(format!("{} resource state", resource.name()));
        }
        let receipts = snapshot
            .ledger_receipts
            .iter()
            .filter(|receipt| receipt.resource == *resource)
            .collect::<Vec<_>>();
        if receipts.is_empty() {
            missing.push(format!("{} ledger receipt", resource.name()));
            continue;
        }
        if receipts.len() != 1 {
            violations.push(format!(
                "{} has {} ledger receipts; exactly one required by this v0 contract",
                resource.name(),
                receipts.len()
            ));
            continue;
        }
        let receipt = receipts[0];
        refs.push(receipt.receipt_id.clone());
        if !receipt.source_verified {
            unverified.push(receipt.receipt_id.clone());
        }
        if receipt.unit != resource.unit() {
            violations.push(format!(
                "{} ledger unit mismatch: expected {}, got {}",
                resource.name(),
                resource.unit(),
                receipt.unit
            ));
        }
        if receipt.conservation_residual > maximum_residual {
            violations.push(format!(
                "{} residual {:.6} exceeds {:.6}",
                resource.name(),
                receipt.conservation_residual,
                maximum_residual
            ));
        }
    }

    if !violations.is_empty() {
        finding(
            premise,
            FindingStatus::Contradicted,
            refs,
            violations.join("; "),
        )
    } else if !missing.is_empty() || !unverified.is_empty() {
        finding(
            premise,
            FindingStatus::Undetermined,
            refs,
            format!(
                "missing evidence: [{}]; unverified receipts: [{}]",
                missing.join(", "),
                unverified.join(", ")
            ),
        )
    } else {
        finding(
            premise,
            FindingStatus::Supported,
            refs,
            format!(
                "all requested resource ledgers have source-verified receipts and residuals <= {:.6}",
                maximum_residual
            ),
        )
    }
}

fn compare_lower(value: f64, minimum: f64) -> FindingStatus {
    if value + TOLERANCE < minimum {
        FindingStatus::Contradicted
    } else {
        FindingStatus::Supported
    }
}

fn compare_upper(value: f64, maximum: f64) -> FindingStatus {
    if value > maximum + TOLERANCE {
        FindingStatus::Contradicted
    } else {
        FindingStatus::Supported
    }
}

fn resource_state(snapshot: &OutcomeSnapshot, resource: ResourceKind) -> Option<&ResourceState> {
    snapshot
        .resources
        .iter()
        .find(|state| state.resource == resource)
}

fn group_refs(snapshot: &OutcomeSnapshot) -> Vec<String> {
    snapshot
        .groups
        .iter()
        .map(|group| group.group_id.clone())
        .collect()
}

fn resource_ref(resource: ResourceKind) -> String {
    format!("resource:{}", resource.name())
}

pub fn reference_profiles() -> Vec<PremiseProfile> {
    let all_resources = ResourceKind::ALL.to_vec();
    vec![
        PremiseProfile {
            profile_id: "aggregate_consequence".into(),
            premises: vec![
                Premise {
                    premise_id: "mean_water_service".into(),
                    rule: PremiseRule::MeanServiceFloor {
                        resource: ResourceKind::Water,
                        minimum: 0.94,
                    },
                },
                Premise {
                    premise_id: "mean_energy_service".into(),
                    rule: PremiseRule::MeanServiceFloor {
                        resource: ResourceKind::Energy,
                        minimum: 0.94,
                    },
                },
            ],
        },
        PremiseProfile {
            profile_id: "equal_consideration".into(),
            premises: vec![
                Premise {
                    premise_id: "every_group_water_floor".into(),
                    rule: PremiseRule::EveryGroupServiceFloor {
                        resource: ResourceKind::Water,
                        minimum: 0.90,
                    },
                },
                Premise {
                    premise_id: "every_group_energy_floor".into(),
                    rule: PremiseRule::EveryGroupServiceFloor {
                        resource: ResourceKind::Energy,
                        minimum: 0.90,
                    },
                },
                Premise {
                    premise_id: "water_gap_cap".into(),
                    rule: PremiseRule::MaximumServiceGap {
                        resource: ResourceKind::Water,
                        maximum: 0.05,
                    },
                },
            ],
        },
        PremiseProfile {
            profile_id: "rights_constrained".into(),
            premises: vec![
                Premise {
                    premise_id: "transfer_consent".into(),
                    rule: PremiseRule::RequireConsentForTransfers,
                },
                Premise {
                    premise_id: "group_exit".into(),
                    rule: PremiseRule::RequireExitForEveryGroup,
                },
            ],
        },
        PremiseProfile {
            profile_id: "intergenerational_stewardship".into(),
            premises: vec![
                Premise {
                    premise_id: "water_reserve".into(),
                    rule: PremiseRule::ResourceClosingFloor {
                        resource: ResourceKind::Water,
                        minimum_stock: 70.0,
                    },
                },
                Premise {
                    premise_id: "food_not_depleted".into(),
                    rule: PremiseRule::NoNetResourceDepletion {
                        resource: ResourceKind::Food,
                    },
                },
                Premise {
                    premise_id: "ledger_physical_accounting".into(),
                    rule: PremiseRule::LedgerConservationQualified {
                        resources: all_resources,
                        maximum_residual: TOLERANCE,
                    },
                },
            ],
        },
    ]
}

pub fn reference_snapshot() -> OutcomeSnapshot {
    OutcomeSnapshot {
        snapshot_id: "synthetic_two_community_reference".into(),
        resources: vec![
            ResourceState {
                resource: ResourceKind::Water,
                opening_stock: 100.0,
                closing_stock: 75.0,
            },
            ResourceState {
                resource: ResourceKind::Energy,
                opening_stock: 200.0,
                closing_stock: 210.0,
            },
            ResourceState {
                resource: ResourceKind::Food,
                opening_stock: 80.0,
                closing_stock: 75.0,
            },
        ],
        ledger_receipts: ResourceKind::ALL
            .into_iter()
            .map(|resource| LedgerReceipt {
                receipt_id: format!("synthetic_{}_receipt", resource.name()),
                resource,
                unit: resource.unit().into(),
                source_digest: format!("synthetic:{}", resource.name()),
                source_verified: true,
                conservation_residual: 0.0,
            })
            .collect(),
        groups: vec![
            GroupOutcome {
                group_id: "community_a".into(),
                water_coverage: Some(0.99),
                energy_coverage: Some(0.99),
                food_coverage: Some(0.96),
                burden_share: Some(0.80),
                exit_available: Some(true),
            },
            GroupOutcome {
                group_id: "community_b".into(),
                water_coverage: Some(0.91),
                energy_coverage: Some(0.92),
                food_coverage: Some(0.94),
                burden_share: Some(0.20),
                exit_available: Some(false),
            },
        ],
        transfers: vec![TransferOutcome {
            transfer_id: "synthetic_transfer_001".into(),
            recipient_group_id: "community_b".into(),
            resource: ResourceKind::Energy,
            amount: 10.0,
            recipient_consent: Some(false),
        }],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_snapshot_can_support_one_premise_profile_and_contradict_another() {
        let report = evaluate_profiles(&reference_snapshot(), &reference_profiles()).unwrap();
        let status = report
            .profiles
            .iter()
            .map(|profile| (profile.profile_id.as_str(), profile.status))
            .collect::<std::collections::BTreeMap<_, _>>();
        assert_eq!(status["aggregate_consequence"], FindingStatus::Supported);
        assert_eq!(status["equal_consideration"], FindingStatus::Contradicted);
        assert_eq!(status["rights_constrained"], FindingStatus::Contradicted);
        assert_eq!(
            status["intergenerational_stewardship"],
            FindingStatus::Contradicted
        );
    }

    #[test]
    fn missing_consent_is_undetermined_not_consent() {
        let mut snapshot = reference_snapshot();
        snapshot.transfers[0].recipient_consent = None;
        snapshot.groups[1].exit_available = Some(true);
        let report = evaluate_profiles(&snapshot, &reference_profiles()).unwrap();
        let rights = report
            .profiles
            .iter()
            .find(|profile| profile.profile_id == "rights_constrained")
            .unwrap();
        assert_eq!(rights.status, FindingStatus::Undetermined);
    }

    #[test]
    fn missing_or_unverified_ledger_evidence_is_undetermined() {
        let mut missing = reference_snapshot();
        missing.ledger_receipts.pop();
        // Remove the independent food-depletion contradiction so the missing
        // ledger receipt is the only unresolved stewardship condition.
        missing.resources[2].closing_stock = 80.0;
        let report = evaluate_profiles(&missing, &reference_profiles()).unwrap();
        let stewardship = report
            .profiles
            .iter()
            .find(|profile| profile.profile_id == "intergenerational_stewardship")
            .unwrap();
        assert_eq!(stewardship.status, FindingStatus::Undetermined);

        let mut unverified = reference_snapshot();
        unverified.ledger_receipts[0].source_verified = false;
        unverified.resources[2].closing_stock = 80.0;
        let report = evaluate_profiles(&unverified, &reference_profiles()).unwrap();
        let stewardship = report
            .profiles
            .iter()
            .find(|profile| profile.profile_id == "intergenerational_stewardship")
            .unwrap();
        assert_eq!(stewardship.status, FindingStatus::Undetermined);
    }

    #[test]
    fn known_conservation_violation_contradicts_ledger_premise() {
        let mut snapshot = reference_snapshot();
        snapshot.ledger_receipts[0].conservation_residual = 0.1;
        let report = evaluate_profiles(&snapshot, &reference_profiles()).unwrap();
        let stewardship = report
            .profiles
            .iter()
            .find(|profile| profile.profile_id == "intergenerational_stewardship")
            .unwrap();
        assert_eq!(stewardship.status, FindingStatus::Contradicted);
    }

    #[test]
    fn profile_and_premise_order_do_not_change_the_report() {
        let snapshot = reference_snapshot();
        let profiles = reference_profiles();
        let first = evaluate_profiles(&snapshot, &profiles).unwrap();
        let mut reversed = profiles;
        reversed.reverse();
        for profile in &mut reversed {
            profile.premises.reverse();
        }
        let second = evaluate_profiles(&snapshot, &reversed).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn incomplete_service_observations_remain_undetermined() {
        let mut snapshot = reference_snapshot();
        snapshot.groups[1].water_coverage = None;
        let report = evaluate_profiles(&snapshot, &reference_profiles()).unwrap();
        let equal = report
            .profiles
            .iter()
            .find(|profile| profile.profile_id == "equal_consideration")
            .unwrap();
        let water = equal
            .premises
            .iter()
            .find(|premise| premise.premise_id == "every_group_water_floor")
            .unwrap();
        assert_eq!(water.status, FindingStatus::Undetermined);
    }

    #[test]
    fn malformed_units_stocks_and_resource_sets_fail_closed() {
        let mut invalid_unit = reference_snapshot();
        invalid_unit.ledger_receipts[0].unit = "litres".into();
        assert!(evaluate_profiles(&invalid_unit, &reference_profiles()).is_err());

        let mut invalid_stock = reference_snapshot();
        invalid_stock.resources[0].closing_stock = f64::NAN;
        assert!(evaluate_profiles(&invalid_stock, &reference_profiles()).is_err());

        let mut invalid_rule = reference_profiles();
        invalid_rule[0].premises[0].rule = PremiseRule::LedgerConservationQualified {
            resources: vec![],
            maximum_residual: 0.0,
        };
        assert!(evaluate_profiles(&reference_snapshot(), &invalid_rule).is_err());
    }

    #[test]
    fn no_universal_score_or_ranking_is_emitted() {
        let report = evaluate_profiles(&reference_snapshot(), &reference_profiles()).unwrap();
        for profile in report.profiles {
            assert!(matches!(
                profile.status,
                FindingStatus::Supported
                    | FindingStatus::Contradicted
                    | FindingStatus::Undetermined
            ));
            assert!(!profile.premises.is_empty());
        }
    }
}
