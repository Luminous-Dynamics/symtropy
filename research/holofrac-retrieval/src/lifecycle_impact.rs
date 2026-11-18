//! Lifecycle mutation to exact evidence-impact qualification.
//!
//! This module composes the append-only component lifecycle fixture with the
//! existing exact dependency-closure engine. It does not infer dependencies
//! from HDC similarity and it never upgrades historical evidence to current.

use crate::impact::{ArtifactKind, EvidenceDependencyGraph, ImpactError, ImpactReport};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LifecycleMutation {
    ComponentCondition,
    ConnectionInterface,
    MaterialPassport,
    FunctionalUse,
}

impl LifecycleMutation {
    pub const ALL: [Self; 4] = [
        Self::ComponentCondition,
        Self::ConnectionInterface,
        Self::MaterialPassport,
        Self::FunctionalUse,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ComponentCondition => "component_condition",
            Self::ConnectionInterface => "connection_interface",
            Self::MaterialPassport => "material_passport",
            Self::FunctionalUse => "functional_use",
        }
    }

    const fn changed_artifact_id(self) -> &'static str {
        match self {
            Self::ComponentCondition => "beam-001-condition-r0",
            Self::ConnectionInterface => "beam-001-interface-r0",
            Self::MaterialPassport => "beam-001-passport-r0",
            Self::FunctionalUse => "beam-001-use-class-r0",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LifecycleEvidenceImpactReport {
    pub mutation: LifecycleMutation,
    pub changed_artifact_id: String,
    pub stale_evidence_ids: Vec<String>,
    pub retained_evidence_ids: Vec<String>,
    pub conservative_full_invalidation: bool,
    pub full_rebuild_evidence_count: usize,
    pub revalidation_evidence_count: usize,
    pub evidence_revalidation_avoided: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LifecycleEvidenceImpactError {
    pub source: ImpactError,
}

impl std::fmt::Display for LifecycleEvidenceImpactError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "lifecycle evidence impact error: {}", self.source)
    }
}

impl std::error::Error for LifecycleEvidenceImpactError {}

impl From<ImpactError> for LifecycleEvidenceImpactError {
    fn from(source: ImpactError) -> Self {
        Self { source }
    }
}

/// Build a fixed, finite current-evidence inventory for one reused component.
///
/// This fixture declares the dependency inventory complete only when requested.
/// The thermal/daylight evidence uses a separate building-envelope/weather
/// path so component-local mutations can prove selective retention.
pub fn lifecycle_evidence_graph(
    declare_complete: bool,
) -> Result<EvidenceDependencyGraph, ImpactError> {
    let mut graph = EvidenceDependencyGraph::new();

    for (id, kind) in [
        ("beam-001-condition-r0", ArtifactKind::DesignParameter),
        ("beam-001-interface-r0", ArtifactKind::Interface),
        ("beam-001-passport-r0", ArtifactKind::DesignParameter),
        ("beam-001-use-class-r0", ArtifactKind::Requirement),
        ("floor-use-load-requirement-r0", ArtifactKind::Requirement),
        ("floor-assembly-geometry-r0", ArtifactKind::Geometry),
        ("structural-solver-input-r0", ArtifactKind::SolverInput),
        ("structural-solver-run-r0", ArtifactKind::SolverRun),
        (
            "evidence-structural-capacity-r0",
            ArtifactKind::EvidenceReceipt,
        ),
        ("fire-solver-input-r0", ArtifactKind::SolverInput),
        ("fire-solver-run-r0", ArtifactKind::SolverRun),
        ("evidence-fire-rating-r0", ArtifactKind::EvidenceReceipt),
        (
            "recoverability-assessment-input-r0",
            ArtifactKind::SolverInput,
        ),
        (
            "recoverability-assessment-run-r0",
            ArtifactKind::SolverRun,
        ),
        (
            "evidence-recoverability-r0",
            ArtifactKind::EvidenceReceipt,
        ),
        (
            "adaptation-assessment-input-r0",
            ArtifactKind::SolverInput,
        ),
        (
            "adaptation-assessment-run-r0",
            ArtifactKind::SolverRun,
        ),
        (
            "evidence-functional-adaptation-r0",
            ArtifactKind::EvidenceReceipt,
        ),
        ("lifecycle-readiness-input-r0", ArtifactKind::SolverInput),
        ("lifecycle-readiness-run-r0", ArtifactKind::SolverRun),
        (
            "evidence-lifecycle-readiness-r0",
            ArtifactKind::EvidenceReceipt,
        ),
        ("site-weather-r0", ArtifactKind::DesignParameter),
        ("building-envelope-geometry-r0", ArtifactKind::Geometry),
        ("thermal-solver-input-r0", ArtifactKind::SolverInput),
        ("thermal-solver-run-r0", ArtifactKind::SolverRun),
        (
            "evidence-thermal-performance-r0",
            ArtifactKind::EvidenceReceipt,
        ),
        ("daylight-solver-input-r0", ArtifactKind::SolverInput),
        ("daylight-solver-run-r0", ArtifactKind::SolverRun),
        (
            "evidence-daylight-performance-r0",
            ArtifactKind::EvidenceReceipt,
        ),
    ] {
        graph.add_artifact(id, kind)?;
    }

    // Current structural evidence depends on exact component condition,
    // interface, passport/material, use class and declared floor-load requirement.
    for dependency in [
        "beam-001-condition-r0",
        "beam-001-interface-r0",
        "beam-001-passport-r0",
        "beam-001-use-class-r0",
        "floor-use-load-requirement-r0",
    ] {
        graph.add_dependency("floor-assembly-geometry-r0", dependency);
    }
    graph.add_dependency(
        "structural-solver-input-r0",
        "floor-assembly-geometry-r0",
    );
    graph.add_dependency(
        "structural-solver-run-r0",
        "structural-solver-input-r0",
    );
    graph.add_dependency(
        "evidence-structural-capacity-r0",
        "structural-solver-run-r0",
    );

    // Fire-rating evidence depends on the material passport, not on the
    // component's current functional use or condition-only inspection.
    graph.add_dependency("fire-solver-input-r0", "beam-001-passport-r0");
    graph.add_dependency("fire-solver-run-r0", "fire-solver-input-r0");
    graph.add_dependency("evidence-fire-rating-r0", "fire-solver-run-r0");

    // Reuse/recoverability and lifecycle-readiness claims depend on the
    // component's material, current condition, interface and functional class.
    for dependency in [
        "beam-001-condition-r0",
        "beam-001-interface-r0",
        "beam-001-passport-r0",
        "beam-001-use-class-r0",
    ] {
        graph.add_dependency("recoverability-assessment-input-r0", dependency);
        graph.add_dependency("lifecycle-readiness-input-r0", dependency);
    }
    graph.add_dependency(
        "recoverability-assessment-run-r0",
        "recoverability-assessment-input-r0",
    );
    graph.add_dependency(
        "evidence-recoverability-r0",
        "recoverability-assessment-run-r0",
    );
    graph.add_dependency(
        "lifecycle-readiness-run-r0",
        "lifecycle-readiness-input-r0",
    );
    graph.add_dependency(
        "evidence-lifecycle-readiness-r0",
        "lifecycle-readiness-run-r0",
    );

    // Functional-adaptation evidence depends on the assembly and functional use.
    graph.add_dependency(
        "adaptation-assessment-input-r0",
        "floor-assembly-geometry-r0",
    );
    graph.add_dependency(
        "adaptation-assessment-input-r0",
        "beam-001-use-class-r0",
    );
    graph.add_dependency(
        "adaptation-assessment-run-r0",
        "adaptation-assessment-input-r0",
    );
    graph.add_dependency(
        "evidence-functional-adaptation-r0",
        "adaptation-assessment-run-r0",
    );

    // Independent building services: a beam condition/use/interface change
    // cannot stale evidence whose declared source set is envelope + weather.
    graph.add_dependency(
        "thermal-solver-input-r0",
        "building-envelope-geometry-r0",
    );
    graph.add_dependency("thermal-solver-input-r0", "site-weather-r0");
    graph.add_dependency("thermal-solver-run-r0", "thermal-solver-input-r0");
    graph.add_dependency(
        "evidence-thermal-performance-r0",
        "thermal-solver-run-r0",
    );
    graph.add_dependency(
        "daylight-solver-input-r0",
        "building-envelope-geometry-r0",
    );
    graph.add_dependency("daylight-solver-input-r0", "site-weather-r0");
    graph.add_dependency("daylight-solver-run-r0", "daylight-solver-input-r0");
    graph.add_dependency(
        "evidence-daylight-performance-r0",
        "daylight-solver-run-r0",
    );

    if declare_complete {
        graph.declare_complete();
    }

    Ok(graph)
}

pub fn evaluate_lifecycle_mutation(
    mutation: LifecycleMutation,
    declare_complete: bool,
) -> Result<LifecycleEvidenceImpactReport, LifecycleEvidenceImpactError> {
    let graph = lifecycle_evidence_graph(declare_complete)?;
    let changed_artifact_id = mutation.changed_artifact_id();
    let report: ImpactReport =
        graph.impact_closure(&[changed_artifact_id.to_owned()])?;

    Ok(LifecycleEvidenceImpactReport {
        mutation,
        changed_artifact_id: changed_artifact_id.to_owned(),
        stale_evidence_ids: report.stale_evidence_ids,
        retained_evidence_ids: report.unaffected_evidence_ids,
        conservative_full_invalidation: report.conservative_full_invalidation,
        full_rebuild_evidence_count: report.full_rebuild_evidence_count,
        revalidation_evidence_count: report.revalidation_evidence_count,
        evidence_revalidation_avoided: report.evidence_revalidation_avoided,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn as_set(values: &[String]) -> BTreeSet<String> {
        values.iter().cloned().collect()
    }

    #[test]
    fn condition_change_invalidates_dependent_current_claims_only() {
        let report =
            evaluate_lifecycle_mutation(LifecycleMutation::ComponentCondition, true).unwrap();

        assert_eq!(
            as_set(&report.stale_evidence_ids),
            [
                "evidence-structural-capacity-r0",
                "evidence-recoverability-r0",
                "evidence-functional-adaptation-r0",
                "evidence-lifecycle-readiness-r0",
            ]
            .into_iter()
            .map(str::to_owned)
            .collect()
        );
        assert_eq!(
            as_set(&report.retained_evidence_ids),
            [
                "evidence-fire-rating-r0",
                "evidence-thermal-performance-r0",
                "evidence-daylight-performance-r0",
            ]
            .into_iter()
            .map(str::to_owned)
            .collect()
        );
        assert_eq!(report.evidence_revalidation_avoided, 3);
        assert!(!report.conservative_full_invalidation);
    }

    #[test]
    fn interface_change_invalidates_structure_recovery_and_adaptation() {
        let report =
            evaluate_lifecycle_mutation(LifecycleMutation::ConnectionInterface, true).unwrap();

        assert!(report.stale_evidence_ids.contains(
            &"evidence-structural-capacity-r0".to_owned()
        ));
        assert!(report.stale_evidence_ids.contains(
            &"evidence-recoverability-r0".to_owned()
        ));
        assert!(report.stale_evidence_ids.contains(
            &"evidence-functional-adaptation-r0".to_owned()
        ));
        assert!(report.stale_evidence_ids.contains(
            &"evidence-lifecycle-readiness-r0".to_owned()
        ));
        assert!(report.retained_evidence_ids.contains(
            &"evidence-fire-rating-r0".to_owned()
        ));
        assert!(report.retained_evidence_ids.contains(
            &"evidence-thermal-performance-r0".to_owned()
        ));
    }

    #[test]
    fn material_passport_change_invalidates_all_material_dependent_claims() {
        let report =
            evaluate_lifecycle_mutation(LifecycleMutation::MaterialPassport, true).unwrap();

        assert_eq!(
            as_set(&report.stale_evidence_ids),
            [
                "evidence-structural-capacity-r0",
                "evidence-fire-rating-r0",
                "evidence-recoverability-r0",
                "evidence-functional-adaptation-r0",
                "evidence-lifecycle-readiness-r0",
            ]
            .into_iter()
            .map(str::to_owned)
            .collect()
        );
        assert_eq!(
            as_set(&report.retained_evidence_ids),
            [
                "evidence-thermal-performance-r0",
                "evidence-daylight-performance-r0",
            ]
            .into_iter()
            .map(str::to_owned)
            .collect()
        );
    }

    #[test]
    fn functional_use_change_does_not_stale_fire_or_envelope_evidence() {
        let report =
            evaluate_lifecycle_mutation(LifecycleMutation::FunctionalUse, true).unwrap();

        assert!(report.stale_evidence_ids.contains(
            &"evidence-structural-capacity-r0".to_owned()
        ));
        assert!(report.stale_evidence_ids.contains(
            &"evidence-functional-adaptation-r0".to_owned()
        ));
        assert!(report.stale_evidence_ids.contains(
            &"evidence-lifecycle-readiness-r0".to_owned()
        ));
        assert!(report.retained_evidence_ids.contains(
            &"evidence-fire-rating-r0".to_owned()
        ));
        assert!(report.retained_evidence_ids.contains(
            &"evidence-thermal-performance-r0".to_owned()
        ));
        assert!(report.retained_evidence_ids.contains(
            &"evidence-daylight-performance-r0".to_owned()
        ));
    }

    #[test]
    fn incomplete_component_inventory_retains_no_current_evidence() {
        let report =
            evaluate_lifecycle_mutation(LifecycleMutation::ComponentCondition, false).unwrap();

        assert!(report.conservative_full_invalidation);
        assert_eq!(
            report.stale_evidence_ids.len(),
            report.full_rebuild_evidence_count
        );
        assert!(report.retained_evidence_ids.is_empty());
        assert_eq!(report.evidence_revalidation_avoided, 0);
    }

    #[test]
    fn every_mutation_matches_the_frozen_independent_evidence_oracle() {
        let expected = [
            (
                LifecycleMutation::ComponentCondition,
                vec![
                    "evidence-structural-capacity-r0",
                    "evidence-recoverability-r0",
                    "evidence-functional-adaptation-r0",
                    "evidence-lifecycle-readiness-r0",
                ],
            ),
            (
                LifecycleMutation::ConnectionInterface,
                vec![
                    "evidence-structural-capacity-r0",
                    "evidence-recoverability-r0",
                    "evidence-functional-adaptation-r0",
                    "evidence-lifecycle-readiness-r0",
                ],
            ),
            (
                LifecycleMutation::MaterialPassport,
                vec![
                    "evidence-structural-capacity-r0",
                    "evidence-fire-rating-r0",
                    "evidence-recoverability-r0",
                    "evidence-functional-adaptation-r0",
                    "evidence-lifecycle-readiness-r0",
                ],
            ),
            (
                LifecycleMutation::FunctionalUse,
                vec![
                    "evidence-structural-capacity-r0",
                    "evidence-recoverability-r0",
                    "evidence-functional-adaptation-r0",
                    "evidence-lifecycle-readiness-r0",
                ],
            ),
        ];

        for (mutation, expected_stale) in expected {
            let report = evaluate_lifecycle_mutation(mutation, true).unwrap();
            assert_eq!(
                as_set(&report.stale_evidence_ids),
                expected_stale
                    .into_iter()
                    .map(str::to_owned)
                    .collect(),
                "stale evidence mismatch for {}",
                mutation.as_str()
            );
        }
    }
}
