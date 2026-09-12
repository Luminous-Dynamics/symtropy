// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Deterministic traces of industrial capability-watch availability over time.
//!
//! A first-failure scalar is insufficient for regenerative experiments because a
//! capability can later recover after inventory, recycling, or upstream support is
//! restored. These traces retain availability transitions without adding any
//! domain-specific MANTA or Genome semantics to Symtropy.

use crate::industrial_watch::IndustrialCapabilityWatchReport;
use std::collections::{BTreeMap, BTreeSet};

/// One meaningful watch-state transition observed at a completed tick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndustrialCapabilityWatchTransition {
    pub tick: u64,
    pub available: bool,
    pub unavailable_required_capability_ids: Vec<String>,
}

/// Compact transition trace for one generic capability watch.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndustrialCapabilityWatchTrace {
    pub watch_id: String,
    /// First completed tick on which this watch was unavailable.
    pub first_unavailable_tick: Option<u64>,
    /// Most recent completed tick incorporated into the trace.
    pub last_observed_tick: Option<u64>,
    /// First observation plus subsequent changes in availability/missing-capability set.
    pub transitions: Vec<IndustrialCapabilityWatchTransition>,
}

impl IndustrialCapabilityWatchTrace {
    pub fn new(watch_id: impl Into<String>) -> Self {
        Self {
            watch_id: watch_id.into(),
            first_unavailable_tick: None,
            last_observed_tick: None,
            transitions: Vec::new(),
        }
    }

    /// Record one completed-tick watch assessment.
    pub fn observe(
        &mut self,
        report: &IndustrialCapabilityWatchReport,
    ) -> Result<(), IndustrialCapabilityWatchTraceError> {
        if report.watch_id != self.watch_id {
            return Err(IndustrialCapabilityWatchTraceError::WatchIdMismatch {
                expected: self.watch_id.clone(),
                observed: report.watch_id.clone(),
            });
        }
        if self
            .last_observed_tick
            .is_some_and(|last| report.tick <= last)
        {
            return Err(IndustrialCapabilityWatchTraceError::NonMonotonicTick {
                watch_id: self.watch_id.clone(),
                previous_tick: self.last_observed_tick.unwrap(),
                observed_tick: report.tick,
            });
        }

        if !report.available && self.first_unavailable_tick.is_none() {
            self.first_unavailable_tick = Some(report.tick);
        }

        let should_record = self.transitions.last().is_none_or(|previous| {
            previous.available != report.available
                || previous.unavailable_required_capability_ids
                    != report.unavailable_required_capability_ids
        });
        if should_record {
            self.transitions.push(IndustrialCapabilityWatchTransition {
                tick: report.tick,
                available: report.available,
                unavailable_required_capability_ids: report
                    .unavailable_required_capability_ids
                    .clone(),
            });
        }
        self.last_observed_tick = Some(report.tick);
        Ok(())
    }

    pub fn currently_available(&self) -> Option<bool> {
        self.transitions.last().map(|transition| transition.available)
    }

    /// First tick after the first failure where the watch returned to available.
    pub fn first_recovery_tick(&self) -> Option<u64> {
        let first_failure = self.first_unavailable_tick?;
        self.transitions
            .iter()
            .find(|transition| transition.tick > first_failure && transition.available)
            .map(|transition| transition.tick)
    }
}

/// Errors while recording deterministic watch traces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IndustrialCapabilityWatchTraceError {
    WatchIdMismatch {
        expected: String,
        observed: String,
    },
    NonMonotonicTick {
        watch_id: String,
        previous_tick: u64,
        observed_tick: u64,
    },
    DuplicateTraceWatchId { watch_id: String },
    DuplicateReportWatchId { watch_id: String },
    WatchCoverageMismatch,
}

/// Atomically incorporate one same-tick report set into an exact trace set.
///
/// Every trace must receive exactly one report and vice versa. Errors leave the
/// caller's traces unchanged.
pub fn record_industrial_capability_watch_reports(
    traces: &mut [IndustrialCapabilityWatchTrace],
    reports: &[IndustrialCapabilityWatchReport],
) -> Result<(), IndustrialCapabilityWatchTraceError> {
    if traces.len() != reports.len() {
        return Err(IndustrialCapabilityWatchTraceError::WatchCoverageMismatch);
    }

    let mut trace_ids = BTreeSet::new();
    for trace in traces.iter() {
        if !trace_ids.insert(trace.watch_id.as_str()) {
            return Err(IndustrialCapabilityWatchTraceError::DuplicateTraceWatchId {
                watch_id: trace.watch_id.clone(),
            });
        }
    }
    let mut report_by_id = BTreeMap::new();
    for report in reports {
        if report_by_id.insert(report.watch_id.as_str(), report).is_some() {
            return Err(IndustrialCapabilityWatchTraceError::DuplicateReportWatchId {
                watch_id: report.watch_id.clone(),
            });
        }
    }
    if trace_ids != report_by_id.keys().copied().collect() {
        return Err(IndustrialCapabilityWatchTraceError::WatchCoverageMismatch);
    }

    let mut candidate = traces.to_vec();
    for trace in &mut candidate {
        trace.observe(report_by_id[trace.watch_id.as_str()])?;
    }
    traces.clone_from_slice(&candidate);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report(
        watch_id: &str,
        tick: u64,
        available: bool,
        unavailable: &[&str],
    ) -> IndustrialCapabilityWatchReport {
        IndustrialCapabilityWatchReport {
            watch_id: watch_id.into(),
            tick,
            available,
            unavailable_required_capability_ids: unavailable
                .iter()
                .map(|value| (*value).into())
                .collect(),
        }
    }

    #[test]
    fn trace_preserves_first_failure_and_later_recovery_without_duplicate_states() {
        let mut trace = IndustrialCapabilityWatchTrace::new("successor_qualification");
        trace
            .observe(&report("successor_qualification", 1, true, &[]))
            .unwrap();
        trace
            .observe(&report(
                "successor_qualification",
                2,
                false,
                &["metrology"],
            ))
            .unwrap();
        trace
            .observe(&report(
                "successor_qualification",
                3,
                false,
                &["metrology"],
            ))
            .unwrap();
        trace
            .observe(&report("successor_qualification", 4, true, &[]))
            .unwrap();

        assert_eq!(trace.first_unavailable_tick, Some(2));
        assert_eq!(trace.first_recovery_tick(), Some(4));
        assert_eq!(trace.currently_available(), Some(true));
        assert_eq!(trace.last_observed_tick, Some(4));
        assert_eq!(trace.transitions.len(), 3);
        assert_eq!(trace.transitions[0].tick, 1);
        assert_eq!(trace.transitions[1].tick, 2);
        assert_eq!(trace.transitions[2].tick, 4);
    }

    #[test]
    fn changed_missing_capability_set_is_retained_even_if_still_unavailable() {
        let mut trace = IndustrialCapabilityWatchTrace::new("construction");
        trace
            .observe(&report("construction", 5, false, &["forge-tooling"]))
            .unwrap();
        trace
            .observe(&report(
                "construction",
                6,
                false,
                &["forge-tooling", "metrology"],
            ))
            .unwrap();
        assert_eq!(trace.transitions.len(), 2);
        assert_eq!(
            trace.transitions[1].unavailable_required_capability_ids,
            vec!["forge-tooling", "metrology"]
        );
    }

    #[test]
    fn nonmonotonic_observation_fails_without_mutating_trace() {
        let mut trace = IndustrialCapabilityWatchTrace::new("operation");
        trace.observe(&report("operation", 7, true, &[])).unwrap();
        let before = trace.clone();
        assert!(trace.observe(&report("operation", 7, false, &["spares"])).is_err());
        assert_eq!(trace, before);
    }

    #[test]
    fn batch_recording_is_exact_coverage_and_transactional() {
        let mut traces = vec![
            IndustrialCapabilityWatchTrace::new("operation"),
            IndustrialCapabilityWatchTrace::new("qualification"),
        ];
        record_industrial_capability_watch_reports(
            &mut traces,
            &[
                report("qualification", 1, true, &[]),
                report("operation", 1, true, &[]),
            ],
        )
        .unwrap();
        assert_eq!(traces[0].last_observed_tick, Some(1));
        assert_eq!(traces[1].last_observed_tick, Some(1));

        let before = traces.clone();
        let error = record_industrial_capability_watch_reports(
            &mut traces,
            &[
                report("operation", 2, true, &[]),
                report("unknown", 2, true, &[]),
            ],
        );
        assert_eq!(
            error,
            Err(IndustrialCapabilityWatchTraceError::WatchCoverageMismatch)
        );
        assert_eq!(traces, before);
    }
}
