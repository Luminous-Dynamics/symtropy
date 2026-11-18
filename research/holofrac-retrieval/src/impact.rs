//! Exact dependency closure for engineering-evidence invalidation.
//!
//! This graph is intentionally separate from the spatial graph. An edge
//! (consumer, dependency) means the consumer is derived from the dependency.
//! A changed source invalidates the source and every transitive consumer.
//! HDC similarity is never consulted by this module.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::time::Instant;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ArtifactKind {
    DesignParameter,
    Requirement,
    Geometry,
    Interface,
    SolverInput,
    SolverRun,
    EvidenceReceipt,
}

impl ArtifactKind {
    fn is_evidence(self) -> bool {
        self == Self::EvidenceReceipt
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ImpactError {
    EmptyArtifactId,
    InvalidArtifactId,
    DuplicateArtifact(String),
    UnknownDependencyEndpoint {
        consumer: String,
        dependency: String,
    },
    SelfDependency(String),
    DuplicateDependency {
        consumer: String,
        dependency: String,
    },
    DependencyCycle,
    EmptyChangeSet,
    UnknownChangedArtifact(String),
}

impl std::fmt::Display for ImpactError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyArtifactId => f.write_str("artifact ID must not be empty"),
            Self::InvalidArtifactId => {
                f.write_str("artifact ID must not contain control characters")
            }
            Self::DuplicateArtifact(id) => write!(f, "duplicate artifact ID: {id}"),
            Self::UnknownDependencyEndpoint {
                consumer,
                dependency,
            } => write!(
                f,
                "dependency edge references unknown endpoint: {consumer} -> {dependency}"
            ),
            Self::SelfDependency(id) => write!(f, "artifact cannot depend on itself: {id}"),
            Self::DuplicateDependency {
                consumer,
                dependency,
            } => {
                write!(f, "duplicate dependency edge: {consumer} -> {dependency}")
            }
            Self::DependencyCycle => f.write_str("evidence dependency graph contains a cycle"),
            Self::EmptyChangeSet => {
                f.write_str("change-impact query requires at least one changed artifact")
            }
            Self::UnknownChangedArtifact(id) => {
                write!(
                    f,
                    "changed artifact is not present in dependency graph: {id}"
                )
            }
        }
    }
}

impl std::error::Error for ImpactError {}

#[derive(Clone, Debug, Default)]
pub struct EvidenceDependencyGraph {
    artifacts: BTreeMap<String, ArtifactKind>,
    /// Each pair is (consumer, dependency).
    dependencies: Vec<(String, String)>,
    declared_complete: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImpactReport {
    pub changed_artifact_ids: Vec<String>,
    pub affected_artifact_ids: Vec<String>,
    pub stale_evidence_ids: Vec<String>,
    /// Evidence that may remain current only when the dependency graph is complete.
    pub unaffected_evidence_ids: Vec<String>,
    pub full_rebuild_evidence_count: usize,
    pub revalidation_evidence_count: usize,
    pub evidence_revalidation_avoided: usize,
    pub conservative_full_invalidation: bool,
}

impl EvidenceDependencyGraph {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_artifact(
        &mut self,
        id: impl Into<String>,
        kind: ArtifactKind,
    ) -> Result<(), ImpactError> {
        let id = id.into();
        if id.trim().is_empty() {
            return Err(ImpactError::EmptyArtifactId);
        }
        if id.chars().any(char::is_control) {
            return Err(ImpactError::InvalidArtifactId);
        }
        if self.artifacts.contains_key(&id) {
            return Err(ImpactError::DuplicateArtifact(id));
        }
        self.artifacts.insert(id, kind);
        Ok(())
    }

    /// Record that consumer was derived from dependency.
    ///
    /// Endpoints are validated when calculating impact closure, so callers
    /// may ingest artifacts and edges from separate source streams.
    pub fn add_dependency(&mut self, consumer: impl Into<String>, dependency: impl Into<String>) {
        self.dependencies.push((consumer.into(), dependency.into()));
    }

    /// Declare that every dependency relevant to the scoped evidence set has
    /// been enumerated. Without this declaration, every evidence receipt is
    /// invalidated conservatively and none is retained as current.
    pub fn declare_complete(&mut self) {
        self.declared_complete = true;
    }

    pub fn artifact_count(&self) -> usize {
        self.artifacts.len()
    }

    pub fn dependency_count(&self) -> usize {
        self.dependencies.len()
    }

    pub fn evidence_count(&self) -> usize {
        self.artifacts
            .values()
            .filter(|kind| kind.is_evidence())
            .count()
    }

    fn validated_dependents(&self) -> Result<BTreeMap<String, BTreeSet<String>>, ImpactError> {
        let mut dependents: BTreeMap<String, BTreeSet<String>> = self
            .artifacts
            .keys()
            .cloned()
            .map(|id| (id, BTreeSet::new()))
            .collect();
        let mut indegree: BTreeMap<String, usize> =
            self.artifacts.keys().cloned().map(|id| (id, 0)).collect();
        let mut seen_edges = BTreeSet::new();

        for (consumer, dependency) in &self.dependencies {
            if !self.artifacts.contains_key(consumer) || !self.artifacts.contains_key(dependency) {
                return Err(ImpactError::UnknownDependencyEndpoint {
                    consumer: consumer.clone(),
                    dependency: dependency.clone(),
                });
            }
            if consumer == dependency {
                return Err(ImpactError::SelfDependency(consumer.clone()));
            }
            if !seen_edges.insert((consumer.clone(), dependency.clone())) {
                return Err(ImpactError::DuplicateDependency {
                    consumer: consumer.clone(),
                    dependency: dependency.clone(),
                });
            }

            // Reverse consumer -> dependency for invalidation traversal:
            // a dependency points to every consumer that uses it.
            dependents
                .get_mut(dependency)
                .expect("validated dependency endpoint")
                .insert(consumer.clone());
            *indegree
                .get_mut(consumer)
                .expect("validated consumer endpoint") += 1;
        }

        // Kahn's algorithm verifies acyclicity in O(V + E).
        let mut ready: VecDeque<String> = indegree
            .iter()
            .filter(|(_, degree)| **degree == 0)
            .map(|(id, _)| id.clone())
            .collect();
        let mut visited = 0_usize;
        while let Some(id) = ready.pop_front() {
            visited += 1;
            if let Some(consumers) = dependents.get(&id) {
                for consumer in consumers {
                    let degree = indegree
                        .get_mut(consumer)
                        .expect("consumer exists in indegree map");
                    *degree -= 1;
                    if *degree == 0 {
                        ready.push_back(consumer.clone());
                    }
                }
            }
        }
        if visited != self.artifacts.len() {
            return Err(ImpactError::DependencyCycle);
        }
        Ok(dependents)
    }

    /// Calculate exact transitive impact. Unknown changed IDs, unknown edge
    /// endpoints and cycles return errors. If dependency completeness has not
    /// been declared, all scoped evidence is marked stale conservatively.
    pub fn impact_closure(&self, changed_ids: &[String]) -> Result<ImpactReport, ImpactError> {
        if changed_ids.is_empty() {
            return Err(ImpactError::EmptyChangeSet);
        }
        let changed: BTreeSet<String> = changed_ids.iter().cloned().collect();
        for id in &changed {
            if !self.artifacts.contains_key(id) {
                return Err(ImpactError::UnknownChangedArtifact(id.clone()));
            }
        }

        let dependents = self.validated_dependents()?;
        let conservative_full_invalidation = !self.declared_complete;
        let affected: BTreeSet<String> = if conservative_full_invalidation {
            self.artifacts.keys().cloned().collect()
        } else {
            let mut affected = changed.clone();
            let mut queue: VecDeque<String> = changed.iter().cloned().collect();
            while let Some(dependency) = queue.pop_front() {
                if let Some(consumers) = dependents.get(&dependency) {
                    for consumer in consumers {
                        if affected.insert(consumer.clone()) {
                            queue.push_back(consumer.clone());
                        }
                    }
                }
            }
            affected
        };

        let all_evidence: BTreeSet<String> = self
            .artifacts
            .iter()
            .filter(|(_, kind)| kind.is_evidence())
            .map(|(id, _)| id.clone())
            .collect();
        let stale_evidence: BTreeSet<String> =
            all_evidence.intersection(&affected).cloned().collect();
        let unaffected_evidence: BTreeSet<String> = if conservative_full_invalidation {
            BTreeSet::new()
        } else {
            all_evidence.difference(&stale_evidence).cloned().collect()
        };
        let full_rebuild_evidence_count = all_evidence.len();
        let revalidation_evidence_count = stale_evidence.len();

        Ok(ImpactReport {
            changed_artifact_ids: changed.into_iter().collect(),
            affected_artifact_ids: affected.into_iter().collect(),
            stale_evidence_ids: stale_evidence.into_iter().collect(),
            unaffected_evidence_ids: unaffected_evidence.into_iter().collect(),
            full_rebuild_evidence_count,
            revalidation_evidence_count,
            evidence_revalidation_avoided: full_rebuild_evidence_count
                .saturating_sub(revalidation_evidence_count),
            conservative_full_invalidation,
        })
    }
}

#[derive(Clone, Debug)]
pub struct ImpactBenchmarkReport {
    pub artifact_count: usize,
    pub dependency_count: usize,
    pub evidence_count: usize,
    pub changed_source_count: usize,
    pub affected_artifact_count: usize,
    pub stale_evidence_count: usize,
    pub retained_evidence_count: usize,
    pub graph_build_micros: f64,
    pub closure_micros: f64,
    pub evidence_revalidation_avoided: usize,
}

/// Generate independent linear dependency chains. Each chain represents one
/// mini-project, so one source mutation leaves other chains' evidence intact.
pub fn run_impact_benchmark(
    artifact_count: usize,
    chain_length: usize,
    changed_chain_count: usize,
) -> Result<ImpactBenchmarkReport, ImpactError> {
    if artifact_count == 0 || chain_length < 2 {
        return Err(ImpactError::EmptyChangeSet);
    }
    let graph_started = Instant::now();
    let mut graph = EvidenceDependencyGraph::new();
    for index in 0..artifact_count {
        let position_in_chain = index % chain_length;
        let kind = match position_in_chain {
            0 => ArtifactKind::DesignParameter,
            1 => ArtifactKind::Geometry,
            2 => ArtifactKind::Interface,
            3 => ArtifactKind::SolverInput,
            4 => ArtifactKind::SolverRun,
            _ if position_in_chain % 5 == 0 => ArtifactKind::EvidenceReceipt,
            _ => ArtifactKind::SolverInput,
        };
        graph.add_artifact(format!("artifact-{index:010}"), kind)?;
        if position_in_chain > 0 {
            graph.add_dependency(
                format!("artifact-{index:010}"),
                format!("artifact-{:010}", index - 1),
            );
        }
    }
    let graph_build_micros = graph_started.elapsed().as_secs_f64() * 1_000_000.0;
    graph.declare_complete();

    let chain_count = artifact_count.div_ceil(chain_length);
    let changed_count = changed_chain_count.min(chain_count).max(1);
    let changed_ids: Vec<String> = (0..changed_count)
        .map(|chain| format!("artifact-{:010}", chain * chain_length))
        .collect();

    let closure_started = Instant::now();
    let report = graph.impact_closure(&changed_ids)?;
    let closure_micros = closure_started.elapsed().as_secs_f64() * 1_000_000.0;

    Ok(ImpactBenchmarkReport {
        artifact_count,
        dependency_count: graph.dependency_count(),
        evidence_count: graph.evidence_count(),
        changed_source_count: changed_ids.len(),
        affected_artifact_count: report.affected_artifact_ids.len(),
        stale_evidence_count: report.stale_evidence_ids.len(),
        retained_evidence_count: report.unaffected_evidence_ids.len(),
        graph_build_micros,
        closure_micros,
        evidence_revalidation_avoided: report.evidence_revalidation_avoided,
    })
}

#[derive(Clone, Debug)]
pub struct SharedCauseImpactBenchmarkReport {
    pub project_count: usize,
    pub artifact_count: usize,
    pub dependency_count: usize,
    pub evidence_count: usize,
    pub changed_source_count: usize,
    pub shared_cause_changed: bool,
    pub affected_artifact_count: usize,
    pub stale_evidence_count: usize,
    pub retained_evidence_count: usize,
    pub graph_build_micros: f64,
    pub closure_micros: f64,
    pub evidence_revalidation_avoided: usize,
}

/// Benchmark shared-cause invalidation and local selective invalidation over
/// identical project DAGs. Each project's geometry depends on one shared site
/// input and one project-local parameter.
pub fn run_shared_cause_impact_benchmark(
    project_count: usize,
    shared_cause_changed: bool,
) -> Result<SharedCauseImpactBenchmarkReport, ImpactError> {
    if project_count == 0 {
        return Err(ImpactError::EmptyChangeSet);
    }

    let graph_started = Instant::now();
    let mut graph = EvidenceDependencyGraph::new();
    graph.add_artifact("shared-site-input", ArtifactKind::DesignParameter)?;
    for project in 0..project_count {
        let prefix = format!("project-{project:08}");
        let parameter = format!("{prefix}-parameter");
        let geometry = format!("{prefix}-geometry");
        let solver_input = format!("{prefix}-solver-input");
        let solver_run = format!("{prefix}-solver-run");
        let evidence = format!("{prefix}-evidence");

        graph.add_artifact(parameter.clone(), ArtifactKind::DesignParameter)?;
        graph.add_artifact(geometry.clone(), ArtifactKind::Geometry)?;
        graph.add_artifact(solver_input.clone(), ArtifactKind::SolverInput)?;
        graph.add_artifact(solver_run.clone(), ArtifactKind::SolverRun)?;
        graph.add_artifact(evidence.clone(), ArtifactKind::EvidenceReceipt)?;

        graph.add_dependency(geometry.clone(), "shared-site-input");
        graph.add_dependency(geometry.clone(), parameter);
        graph.add_dependency(solver_input.clone(), geometry);
        graph.add_dependency(solver_run.clone(), solver_input);
        graph.add_dependency(evidence.clone(), solver_run);
    }
    let graph_build_micros = graph_started.elapsed().as_secs_f64() * 1_000_000.0;
    graph.declare_complete();

    let changed_ids = if shared_cause_changed {
        vec!["shared-site-input".to_owned()]
    } else {
        vec!["project-00000000-parameter".to_owned()]
    };
    let closure_started = Instant::now();
    let report = graph.impact_closure(&changed_ids)?;
    let closure_micros = closure_started.elapsed().as_secs_f64() * 1_000_000.0;

    Ok(SharedCauseImpactBenchmarkReport {
        project_count,
        artifact_count: graph.artifact_count(),
        dependency_count: graph.dependency_count(),
        evidence_count: graph.evidence_count(),
        changed_source_count: changed_ids.len(),
        shared_cause_changed,
        affected_artifact_count: report.affected_artifact_ids.len(),
        stale_evidence_count: report.stale_evidence_ids.len(),
        retained_evidence_count: report.unaffected_evidence_ids.len(),
        graph_build_micros,
        closure_micros,
        evidence_revalidation_avoided: report.evidence_revalidation_avoided,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(complete: bool) -> EvidenceDependencyGraph {
        let mut graph = EvidenceDependencyGraph::new();
        for (id, kind) in [
            ("parameter-a", ArtifactKind::DesignParameter),
            ("geometry-a", ArtifactKind::Geometry),
            ("solver-input-a", ArtifactKind::SolverInput),
            ("solver-run-a", ArtifactKind::SolverRun),
            ("evidence-a", ArtifactKind::EvidenceReceipt),
            ("parameter-b", ArtifactKind::DesignParameter),
            ("geometry-b", ArtifactKind::Geometry),
            ("evidence-b", ArtifactKind::EvidenceReceipt),
        ] {
            graph.add_artifact(id, kind).unwrap();
        }
        graph.add_dependency("geometry-a", "parameter-a");
        graph.add_dependency("solver-input-a", "geometry-a");
        graph.add_dependency("solver-run-a", "solver-input-a");
        graph.add_dependency("evidence-a", "solver-run-a");
        graph.add_dependency("geometry-b", "parameter-b");
        graph.add_dependency("evidence-b", "geometry-b");
        if complete {
            graph.declare_complete();
        }
        graph
    }

    #[test]
    fn one_source_change_invalidates_only_transitive_dependent_evidence() {
        let graph = fixture(true);
        let report = graph.impact_closure(&["parameter-a".to_owned()]).unwrap();
        assert_eq!(
            report.affected_artifact_ids,
            vec![
                "evidence-a",
                "geometry-a",
                "parameter-a",
                "solver-input-a",
                "solver-run-a"
            ]
        );
        assert_eq!(report.stale_evidence_ids, vec!["evidence-a"]);
        assert_eq!(report.unaffected_evidence_ids, vec!["evidence-b"]);
        assert_eq!(report.evidence_revalidation_avoided, 1);
        assert!(!report.conservative_full_invalidation);
    }

    #[test]
    fn multi_source_change_uses_union_of_dependency_closures() {
        let graph = fixture(true);
        let report = graph
            .impact_closure(&["parameter-a".to_owned(), "parameter-b".to_owned()])
            .unwrap();
        assert_eq!(report.stale_evidence_ids, vec!["evidence-a", "evidence-b"]);
        assert!(report.unaffected_evidence_ids.is_empty());
    }

    #[test]
    fn incomplete_dependency_graph_invalidates_all_evidence_conservatively() {
        let graph = fixture(false);
        let report = graph.impact_closure(&["parameter-a".to_owned()]).unwrap();
        assert_eq!(report.stale_evidence_ids, vec!["evidence-a", "evidence-b"]);
        assert!(report.unaffected_evidence_ids.is_empty());
        assert!(report.conservative_full_invalidation);
    }

    #[test]
    fn unknown_change_ids_fail_closed() {
        let graph = fixture(true);
        assert_eq!(
            graph.impact_closure(&["missing".to_owned()]).unwrap_err(),
            ImpactError::UnknownChangedArtifact("missing".to_owned())
        );
    }

    #[test]
    fn unknown_dependency_endpoints_fail_closed() {
        let mut graph = fixture(true);
        graph.add_dependency("evidence-a", "missing");
        assert!(matches!(
            graph.impact_closure(&["parameter-a".to_owned()]),
            Err(ImpactError::UnknownDependencyEndpoint { .. })
        ));
    }

    #[test]
    fn cycles_and_self_dependencies_fail_closed() {
        let mut cycle = EvidenceDependencyGraph::new();
        cycle.add_artifact("a", ArtifactKind::Geometry).unwrap();
        cycle.add_artifact("b", ArtifactKind::SolverInput).unwrap();
        cycle.add_dependency("a", "b");
        cycle.add_dependency("b", "a");
        cycle.declare_complete();
        assert_eq!(
            cycle.impact_closure(&["a".to_owned()]).unwrap_err(),
            ImpactError::DependencyCycle
        );

        let mut self_edge = EvidenceDependencyGraph::new();
        self_edge.add_artifact("a", ArtifactKind::Geometry).unwrap();
        self_edge.add_dependency("a", "a");
        self_edge.declare_complete();
        assert_eq!(
            self_edge.impact_closure(&["a".to_owned()]).unwrap_err(),
            ImpactError::SelfDependency("a".to_owned())
        );
    }

    #[test]
    fn shared_common_cause_invalidates_every_dependent_evidence_receipt() {
        let mut graph = EvidenceDependencyGraph::new();
        for (id, kind) in [
            ("weather-input", ArtifactKind::DesignParameter),
            ("room-a", ArtifactKind::Geometry),
            ("room-b", ArtifactKind::Geometry),
            ("solver-a", ArtifactKind::SolverRun),
            ("solver-b", ArtifactKind::SolverRun),
            ("evidence-a", ArtifactKind::EvidenceReceipt),
            ("evidence-b", ArtifactKind::EvidenceReceipt),
        ] {
            graph.add_artifact(id, kind).unwrap();
        }
        graph.add_dependency("room-a", "weather-input");
        graph.add_dependency("room-b", "weather-input");
        graph.add_dependency("solver-a", "room-a");
        graph.add_dependency("solver-b", "room-b");
        graph.add_dependency("evidence-a", "solver-a");
        graph.add_dependency("evidence-b", "solver-b");
        graph.declare_complete();

        let report = graph.impact_closure(&["weather-input".to_owned()]).unwrap();
        assert_eq!(
            report.stale_evidence_ids,
            vec!["evidence-a".to_owned(), "evidence-b".to_owned()]
        );
        assert!(report.unaffected_evidence_ids.is_empty());
    }

    #[test]
    fn duplicate_edges_and_empty_change_sets_fail_closed() {
        let mut graph = fixture(true);
        graph.add_dependency("geometry-a", "parameter-a");
        assert!(matches!(
            graph.impact_closure(&["parameter-a".to_owned()]),
            Err(ImpactError::DuplicateDependency { .. })
        ));

        let graph = fixture(true);
        assert_eq!(
            graph.impact_closure(&[]).unwrap_err(),
            ImpactError::EmptyChangeSet
        );
    }

    #[test]
    fn deterministic_generated_dags_match_independent_oracle_across_change_sets() {
        for seed in 1_u64..=24 {
            let mut graph = EvidenceDependencyGraph::new();
            let node_count = 48_usize;
            for index in 0..node_count {
                let kind = if index % 7 == 0 {
                    ArtifactKind::EvidenceReceipt
                } else if index % 5 == 0 {
                    ArtifactKind::SolverRun
                } else {
                    ArtifactKind::Geometry
                };
                graph
                    .add_artifact(format!("node-{index:03}"), kind)
                    .unwrap();
            }

            // Deterministic xorshift schedule. Every edge points from a higher
            // index to a lower index, guaranteeing a DAG before validation.
            let mut random = seed;
            for consumer in 1..node_count {
                for dependency in 0..consumer {
                    random ^= random << 13;
                    random ^= random >> 7;
                    random ^= random << 17;
                    if random % 11 == 0 {
                        graph.add_dependency(
                            format!("node-{consumer:03}"),
                            format!("node-{dependency:03}"),
                        );
                    }
                }
            }
            graph.declare_complete();

            let changed = vec![
                format!("node-{:03}", seed as usize % node_count),
                format!("node-{:03}", (seed as usize * 7) % node_count),
            ];
            let report = graph.impact_closure(&changed).unwrap();
            let changed_set: BTreeSet<_> = changed.iter().cloned().collect();

            // Independent per-candidate backward walk over the original edge
            // list, not the production reverse-adjacency structure.
            let mut oracle_affected = BTreeSet::new();
            for candidate in graph.artifacts.keys() {
                let mut pending = vec![candidate.clone()];
                let mut visited = BTreeSet::new();
                while let Some(current) = pending.pop() {
                    if !visited.insert(current.clone()) {
                        continue;
                    }
                    if changed_set.contains(&current) {
                        oracle_affected.insert(candidate.clone());
                        break;
                    }
                    for (consumer, dependency) in &graph.dependencies {
                        if consumer == &current {
                            pending.push(dependency.clone());
                        }
                    }
                }
            }
            assert_eq!(
                report
                    .affected_artifact_ids
                    .into_iter()
                    .collect::<BTreeSet<_>>(),
                oracle_affected,
                "mismatch for deterministic DAG seed {seed}"
            );
        }
    }

    #[test]
    fn exact_closure_matches_independent_brute_force_oracle() {
        let graph = fixture(true);
        let changed = ["parameter-a".to_owned()];
        let report = graph.impact_closure(&changed).unwrap();

        let mut oracle_affected = BTreeSet::new();
        for candidate in graph.artifacts.keys() {
            let mut pending = vec![candidate.clone()];
            let mut visited = BTreeSet::new();
            while let Some(current) = pending.pop() {
                if !visited.insert(current.clone()) {
                    continue;
                }
                if changed.contains(&current) {
                    oracle_affected.insert(candidate.clone());
                    break;
                }
                for (consumer, dependency) in &graph.dependencies {
                    if consumer == &current {
                        pending.push(dependency.clone());
                    }
                }
            }
        }
        assert_eq!(
            report
                .affected_artifact_ids
                .into_iter()
                .collect::<BTreeSet<_>>(),
            oracle_affected
        );
    }

    #[test]
    fn shared_cause_change_invalidates_all_projects_but_local_change_is_selective() {
        let shared = run_shared_cause_impact_benchmark(32, true).unwrap();
        let local = run_shared_cause_impact_benchmark(32, false).unwrap();
        assert_eq!(shared.stale_evidence_count, 32);
        assert_eq!(shared.retained_evidence_count, 0);
        assert_eq!(local.stale_evidence_count, 1);
        assert_eq!(local.retained_evidence_count, 31);
        assert_eq!(local.evidence_revalidation_avoided, 31);
    }

    #[test]
    fn deterministic_capacity_fixture_reports_selective_revalidation() {
        let report = run_impact_benchmark(1_000, 20, 1).unwrap();
        assert_eq!(report.artifact_count, 1_000);
        assert!(report.dependency_count > 900);
        assert!(report.stale_evidence_count < report.evidence_count);
        assert!(report.retained_evidence_count > 0);
        assert!(report.evidence_revalidation_avoided > 0);
    }
}
