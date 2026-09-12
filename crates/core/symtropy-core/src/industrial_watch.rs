// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Generic capability watches for industrial-ecology tick evidence.
//!
//! Watches deliberately know nothing about maritime genomes or lineage semantics.
//! A caller may classify capability sets as operation, construction, qualification,
//! or any other role while Symtropy only reports whether the named simulated
//! capabilities are currently available.

use crate::industrial_ecology::{IndustrialCapability, IndustrialTickReport};
use std::collections::BTreeSet;

const MAX_ID_LEN: usize = 256;
const MAX_WATCH_CAPABILITIES: usize = 4096;

/// A validated set of simulated capabilities that must all remain available.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndustrialCapabilityWatch {
    pub watch_id: String,
    pub required_capability_ids: BTreeSet<String>,
}

impl IndustrialCapabilityWatch {
    /// Build a watch against the exact capability definitions used to configure an ecology.
    pub fn new(
        watch_id: impl Into<String>,
        required_capability_ids: BTreeSet<String>,
        known_capabilities: &[IndustrialCapability],
    ) -> Result<Self, IndustrialCapabilityWatchError> {
        let watch_id = watch_id.into();
        validate_id(&watch_id)?;
        if required_capability_ids.is_empty() {
            return Err(IndustrialCapabilityWatchError::NoRequiredCapabilities);
        }
        if required_capability_ids.len() > MAX_WATCH_CAPABILITIES {
            return Err(IndustrialCapabilityWatchError::TooManyRequiredCapabilities);
        }

        let known: BTreeSet<&str> = known_capabilities
            .iter()
            .map(|capability| capability.capability_id.as_str())
            .collect();
        for capability_id in &required_capability_ids {
            validate_id(capability_id)?;
            if !known.contains(capability_id.as_str()) {
                return Err(IndustrialCapabilityWatchError::UnknownCapability {
                    capability_id: capability_id.clone(),
                });
            }
        }

        Ok(Self {
            watch_id,
            required_capability_ids,
        })
    }

    /// Assess this watch against one completed industrial tick.
    pub fn assess(&self, report: &IndustrialTickReport) -> IndustrialCapabilityWatchReport {
        let unavailable: BTreeSet<&str> = report
            .unavailable_capability_ids
            .iter()
            .map(String::as_str)
            .collect();
        let unavailable_required_capability_ids = self
            .required_capability_ids
            .iter()
            .filter(|capability_id| unavailable.contains(capability_id.as_str()))
            .cloned()
            .collect::<Vec<_>>();

        IndustrialCapabilityWatchReport {
            watch_id: self.watch_id.clone(),
            tick: report.tick,
            available: unavailable_required_capability_ids.is_empty(),
            unavailable_required_capability_ids,
        }
    }
}

/// Availability of one validated watch at one completed tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndustrialCapabilityWatchReport {
    pub watch_id: String,
    pub tick: u64,
    pub available: bool,
    pub unavailable_required_capability_ids: Vec<String>,
}

/// Assess multiple watches against the same tick in caller-supplied order.
pub fn assess_industrial_capability_watches(
    report: &IndustrialTickReport,
    watches: &[IndustrialCapabilityWatch],
) -> Vec<IndustrialCapabilityWatchReport> {
    watches.iter().map(|watch| watch.assess(report)).collect()
}

/// Structural watch-construction errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IndustrialCapabilityWatchError {
    InvalidIdentifier,
    NoRequiredCapabilities,
    TooManyRequiredCapabilities,
    UnknownCapability { capability_id: String },
}

fn validate_id(value: &str) -> Result<(), IndustrialCapabilityWatchError> {
    if value.is_empty()
        || value.len() > MAX_ID_LEN
        || value.trim() != value
        || value.chars().any(char::is_control)
    {
        Err(IndustrialCapabilityWatchError::InvalidIdentifier)
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn capabilities() -> Vec<IndustrialCapability> {
        vec![
            IndustrialCapability {
                capability_id: "operation".into(),
                essential: true,
                dependency_ids: BTreeSet::from(["spares".into()]),
            },
            IndustrialCapability {
                capability_id: "successor-construction".into(),
                essential: true,
                dependency_ids: BTreeSet::from(["forge".into()]),
            },
            IndustrialCapability {
                capability_id: "successor-qualification".into(),
                essential: true,
                dependency_ids: BTreeSet::from(["metrology".into()]),
            },
        ]
    }

    #[test]
    fn unknown_watch_capability_fails_closed() {
        let result = IndustrialCapabilityWatch::new(
            "qualification",
            BTreeSet::from(["unknown".into()]),
            &capabilities(),
        );
        assert_eq!(
            result,
            Err(IndustrialCapabilityWatchError::UnknownCapability {
                capability_id: "unknown".into(),
            })
        );
    }

    #[test]
    fn one_tick_can_distinguish_operation_construction_and_qualification() {
        let known = capabilities();
        let watches = vec![
            IndustrialCapabilityWatch::new(
                "operation",
                BTreeSet::from(["operation".into()]),
                &known,
            )
            .unwrap(),
            IndustrialCapabilityWatch::new(
                "construction",
                BTreeSet::from(["successor-construction".into()]),
                &known,
            )
            .unwrap(),
            IndustrialCapabilityWatch::new(
                "qualification",
                BTreeSet::from(["successor-qualification".into()]),
                &known,
            )
            .unwrap(),
        ];
        let tick = IndustrialTickReport {
            tick: 30,
            shortages: Vec::new(),
            blocked_flows: Vec::new(),
            unavailable_capability_ids: vec!["successor-qualification".into()],
            essential_capabilities_available: false,
        };

        let reports = assess_industrial_capability_watches(&tick, &watches);
        assert_eq!(reports[0].watch_id, "operation");
        assert!(reports[0].available);
        assert_eq!(reports[1].watch_id, "construction");
        assert!(reports[1].available);
        assert_eq!(reports[2].watch_id, "qualification");
        assert!(!reports[2].available);
        assert_eq!(
            reports[2].unavailable_required_capability_ids,
            vec!["successor-qualification"]
        );
    }
}
