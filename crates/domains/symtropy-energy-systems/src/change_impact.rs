// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Dependency-aware invalidation for Power Fabric derived artifacts.
//! This is a deterministic planning kernel, not an electrical control system.

use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InputFingerprint { input_id: String, digest: String }

impl InputFingerprint {
    pub fn new(input_id: impl Into<String>, digest: impl Into<String>) -> Result<Self, ChangeImpactError> {
        let input_id = input_id.into();
        let digest = digest.into();
        require_non_empty("input_id", &input_id)?;
        require_non_empty("input_digest", &digest)?;
        Ok(Self { input_id, digest })
    }
    pub fn input_id(&self) -> &str { &self.input_id }
    pub fn digest(&self) -> &str { &self.digest }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DerivedArtifact {
    artifact_id: String,
    output_digest: String,
    compiler_version: String,
    profile_version: String,
    dependency_ids: Vec<String>,
    dependency_inventory_complete: bool,
}

impl DerivedArtifact {
    pub fn new(
        artifact_id: impl Into<String>,
        output_digest: impl Into<String>,
        compiler_version: impl Into<String>,
        profile_version: impl Into<String>,
        mut dependency_ids: Vec<String>,
        dependency_inventory_complete: bool,
    ) -> Result<Self, ChangeImpactError> {
        let artifact_id = artifact_id.into();
        let output_digest = output_digest.into();
        let compiler_version = compiler_version.into();
        let profile_version = profile_version.into();
        require_non_empty("artifact_id", &artifact_id)?;
        require_non_empty("output_digest", &output_digest)?;
        require_non_empty("compiler_version", &compiler_version)?;
        require_non_empty("profile_version", &profile_version)?;
        for id in &dependency_ids { require_non_empty("dependency_id", id)?; }
        dependency_ids.sort();
        if let Some(pair) = dependency_ids.windows(2).find(|pair| pair[0] == pair[1]) {
            return Err(ChangeImpactError::DuplicateDependency {
                artifact_id,
                dependency_id: pair[0].clone(),
            });
        }
        Ok(Self { artifact_id, output_digest, compiler_version, profile_version, dependency_ids, dependency_inventory_complete })
    }
    pub fn artifact_id(&self) -> &str { &self.artifact_id }
    pub fn output_digest(&self) -> &str { &self.output_digest }
    pub fn dependency_ids(&self) -> &[String] { &self.dependency_ids }
    pub const fn dependency_inventory_complete(&self) -> bool { self.dependency_inventory_complete }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependencyGraph {
    inputs: BTreeMap<String, InputFingerprint>,
    artifacts: BTreeMap<String, DerivedArtifact>,
    reverse_dependencies: BTreeMap<String, BTreeSet<String>>,
}

impl DependencyGraph {
    pub fn new(inputs: Vec<InputFingerprint>, artifacts: Vec<DerivedArtifact>) -> Result<Self, ChangeImpactError> {
        let mut input_map = BTreeMap::new();
        for input in inputs {
            let id = input.input_id.clone();
            if input_map.insert(id.clone(), input).is_some() {
                return Err(ChangeImpactError::DuplicateInput { input_id: id });
            }
        }
        let mut artifact_map = BTreeMap::new();
        for artifact in artifacts {
            let id = artifact.artifact_id.clone();
            if artifact_map.insert(id.clone(), artifact).is_some() {
                return Err(ChangeImpactError::DuplicateArtifact { artifact_id: id });
            }
        }
        for id in input_map.keys() {
            if artifact_map.contains_key(id) {
                return Err(ChangeImpactError::InputArtifactIdentityCollision { node_id: id.clone() });
            }
        }

        let mut reverse: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
        for artifact in artifact_map.values() {
            for dependency_id in &artifact.dependency_ids {
                if !input_map.contains_key(dependency_id) && !artifact_map.contains_key(dependency_id) {
                    return Err(ChangeImpactError::UnknownDependency {
                        artifact_id: artifact.artifact_id.clone(),
                        dependency_id: dependency_id.clone(),
                    });
                }
                if dependency_id == &artifact.artifact_id {
                    return Err(ChangeImpactError::SelfDependency { artifact_id: artifact.artifact_id.clone() });
                }
                reverse.entry(dependency_id.clone()).or_default().insert(artifact.artifact_id.clone());
            }
        }
        validate_acyclic(&artifact_map)?;
        Ok(Self { inputs: input_map, artifacts: artifact_map, reverse_dependencies: reverse })
    }

    pub fn inputs(&self) -> &BTreeMap<String, InputFingerprint> { &self.inputs }
    pub fn artifacts(&self) -> &BTreeMap<String, DerivedArtifact> { &self.artifacts }

    pub fn assess(
        &self,
        changes: Vec<InputChange>,
        current_compiler_version: impl Into<String>,
        current_profile_version: impl Into<String>,
    ) -> Result<InvalidationReport, ChangeImpactError> {
        let current_compiler_version = current_compiler_version.into();
        let current_profile_version = current_profile_version.into();
        require_non_empty("current_compiler_version", &current_compiler_version)?;
        require_non_empty("current_profile_version", &current_profile_version)?;

        let mut changed_inputs = BTreeMap::new();
        for change in changes {
            let Some(input) = self.inputs.get(&change.input_id) else {
                return Err(ChangeImpactError::UnknownChangedInput { input_id: change.input_id });
            };
            if change.expected_old_digest != input.digest {
                return Err(ChangeImpactError::StaleExpectedInputDigest {
                    input_id: change.input_id, expected: change.expected_old_digest, actual: input.digest.clone()
                });
            }
            if let Some(new_digest) = &change.new_digest { require_non_empty("new_input_digest", new_digest)?; }
            let changed = change.new_digest.as_deref() != Some(input.digest.as_str());
            if changed && changed_inputs.insert(change.input_id.clone(), change.clone()).is_some() {
                return Err(ChangeImpactError::DuplicateChangedInput { input_id: change.input_id });
            }
        }

        let mut reasons: BTreeMap<String, BTreeSet<InvalidationReason>> = BTreeMap::new();
        let mut queue = BTreeSet::new();

        for artifact in self.artifacts.values() {
            if artifact.compiler_version != current_compiler_version {
                add_reason(&mut reasons, &mut queue, &artifact.artifact_id, InvalidationReason::CompilerVersionChanged);
            }
            if artifact.profile_version != current_profile_version {
                add_reason(&mut reasons, &mut queue, &artifact.artifact_id, InvalidationReason::ProfileVersionChanged);
            }
            if !artifact.dependency_inventory_complete {
                add_reason(&mut reasons, &mut queue, &artifact.artifact_id, InvalidationReason::DependencyInventoryIncomplete);
            }
        }
        for input_id in changed_inputs.keys() { queue.insert(input_id.clone()); }

        let mut processed = BTreeSet::new();
        while let Some(dependency_id) = queue.pop_first() {
            if !processed.insert(dependency_id.clone()) { continue; }
            if let Some(dependents) = self.reverse_dependencies.get(&dependency_id) {
                for artifact_id in dependents {
                    let reason = if self.inputs.contains_key(&dependency_id) {
                        InvalidationReason::DirectDependencyChanged
                    } else {
                        InvalidationReason::TransitiveDependencyChanged
                    };
                    add_reason(&mut reasons, &mut queue, artifact_id, reason);
                }
            }
        }

        let invalidated: BTreeMap<String, Vec<InvalidationReason>> = reasons.into_iter()
            .map(|(id, values)| (id, values.into_iter().collect())).collect();
        let retained = self.artifacts.iter()
            .filter(|(id, _)| !invalidated.contains_key(*id))
            .map(|(id, artifact)| (id.clone(), artifact.output_digest.clone()))
            .collect::<BTreeMap<_, _>>();
        let receipts = changed_inputs.into_iter().map(|(id, change)| {
            (id, InputChangeReceipt { expected_old_digest: change.expected_old_digest, new_digest: change.new_digest })
        }).collect::<BTreeMap<_, _>>();
        let reconciliation_required = receipts.values().any(|receipt| receipt.new_digest.is_none());

        Ok(InvalidationReport {
            compiler_version: current_compiler_version,
            profile_version: current_profile_version,
            changed_inputs: receipts,
            invalidated,
            retained,
            reconciliation_required,
        })
    }
}

fn add_reason(
    reasons: &mut BTreeMap<String, BTreeSet<InvalidationReason>>,
    queue: &mut BTreeSet<String>,
    artifact_id: &str,
    reason: InvalidationReason,
) {
    reasons.entry(artifact_id.to_owned()).or_default().insert(reason);
    queue.insert(artifact_id.to_owned());
}

fn validate_acyclic(artifacts: &BTreeMap<String, DerivedArtifact>) -> Result<(), ChangeImpactError> {
    let mut indegree: BTreeMap<String, usize> = artifacts.keys().map(|id| (id.clone(), 0)).collect();
    let mut dependents: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    for artifact in artifacts.values() {
        for dependency_id in &artifact.dependency_ids {
            if artifacts.contains_key(dependency_id) {
                *indegree.get_mut(&artifact.artifact_id).expect("artifact key exists") += 1;
                dependents.entry(dependency_id.clone()).or_default().insert(artifact.artifact_id.clone());
            }
        }
    }
    let mut ready: BTreeSet<String> = indegree.iter().filter(|(_, degree)| **degree == 0).map(|(id, _)| id.clone()).collect();
    let mut visited = 0usize;
    while let Some(id) = ready.pop_first() {
        visited += 1;
        if let Some(children) = dependents.get(&id) {
            for child in children {
                let degree = indegree.get_mut(child).expect("dependent exists");
                *degree -= 1;
                if *degree == 0 { ready.insert(child.clone()); }
            }
        }
    }
    if visited != artifacts.len() {
        return Err(ChangeImpactError::DependencyCycle {
            artifact_ids: indegree.into_iter().filter(|(_, degree)| *degree > 0).map(|(id, _)| id).collect()
        });
    }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputChange { input_id: String, expected_old_digest: String, new_digest: Option<String> }
impl InputChange {
    pub fn new(input_id: impl Into<String>, expected_old_digest: impl Into<String>, new_digest: Option<String>) -> Result<Self, ChangeImpactError> {
        let input_id = input_id.into();
        let expected_old_digest = expected_old_digest.into();
        require_non_empty("changed_input_id", &input_id)?;
        require_non_empty("expected_old_digest", &expected_old_digest)?;
        if let Some(digest) = &new_digest { require_non_empty("new_input_digest", digest)?; }
        Ok(Self { input_id, expected_old_digest, new_digest })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InvalidationReason {
    DirectDependencyChanged,
    TransitiveDependencyChanged,
    DependencyInventoryIncomplete,
    CompilerVersionChanged,
    ProfileVersionChanged,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InputChangeReceipt { expected_old_digest: String, new_digest: Option<String> }

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InvalidationReport {
    compiler_version: String,
    profile_version: String,
    changed_inputs: BTreeMap<String, InputChangeReceipt>,
    invalidated: BTreeMap<String, Vec<InvalidationReason>>,
    retained: BTreeMap<String, String>,
    reconciliation_required: bool,
}
impl InvalidationReport {
    pub fn compiler_version(&self) -> &str { &self.compiler_version }
    pub fn profile_version(&self) -> &str { &self.profile_version }
    pub fn changed_inputs(&self) -> &BTreeMap<String, InputChangeReceipt> { &self.changed_inputs }
    pub fn invalidated(&self) -> &BTreeMap<String, Vec<InvalidationReason>> { &self.invalidated }
    pub fn retained(&self) -> &BTreeMap<String, String> { &self.retained }
    pub const fn reconciliation_required(&self) -> bool { self.reconciliation_required }
}

fn require_non_empty(field: &'static str, value: &str) -> Result<(), ChangeImpactError> {
    if value.trim().is_empty() { return Err(ChangeImpactError::EmptyField { field }); }
    Ok(())
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChangeImpactError {
    EmptyField { field: &'static str },
    DuplicateInput { input_id: String },
    DuplicateArtifact { artifact_id: String },
    DuplicateDependency { artifact_id: String, dependency_id: String },
    InputArtifactIdentityCollision { node_id: String },
    UnknownDependency { artifact_id: String, dependency_id: String },
    SelfDependency { artifact_id: String },
    DependencyCycle { artifact_ids: Vec<String> },
    UnknownChangedInput { input_id: String },
    StaleExpectedInputDigest { input_id: String, expected: String, actual: String },
    DuplicateChangedInput { input_id: String },
}
impl core::fmt::Display for ChangeImpactError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result { write!(f, "{self:?}") }
}
impl std::error::Error for ChangeImpactError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn input(id: &str) -> InputFingerprint { InputFingerprint::new(id, format!("sha256:old:{id}")).unwrap() }
    fn artifact(id: &str, dependencies: &[&str]) -> DerivedArtifact {
        DerivedArtifact::new(id, format!("sha256:output:{id}"), "compiler:1", "profile:1",
            dependencies.iter().map(|value| (*value).to_owned()).collect(), true).unwrap()
    }
    fn graph() -> DependencyGraph {
        DependencyGraph::new(
            vec![input("input:topology"), input("input:catalog"), input("input:carbon")],
            vec![
                artifact("artifact:compatibility", &["input:catalog", "input:topology"]),
                artifact("artifact:source-map", &["artifact:compatibility"]),
                artifact("artifact:resident-view", &["artifact:source-map"]),
                artifact("artifact:carbon-report", &["input:carbon"]),
            ],
        ).unwrap()
    }
    fn change(id: &str) -> InputChange {
        InputChange::new(id, format!("sha256:old:{id}"), Some(format!("sha256:new:{id}"))).unwrap()
    }

    #[test]
    fn direct_and_transitive_dependents_are_invalidated_but_unrelated_is_retained() {
        let report = graph().assess(vec![change("input:catalog")], "compiler:1", "profile:1").unwrap();
        assert!(report.invalidated().contains_key("artifact:compatibility"));
        assert!(report.invalidated().contains_key("artifact:source-map"));
        assert!(report.invalidated().contains_key("artifact:resident-view"));
        assert!(!report.invalidated().contains_key("artifact:carbon-report"));
        assert!(report.retained().contains_key("artifact:carbon-report"));
    }
    #[test]
    fn retained_artifact_keeps_its_output_digest() {
        let report = graph().assess(vec![change("input:topology")], "compiler:1", "profile:1").unwrap();
        assert_eq!(report.retained()["artifact:carbon-report"], "sha256:output:artifact:carbon-report");
    }
    #[test]
    fn unknown_new_digest_invalidates_and_requires_reconciliation() {
        let unknown = InputChange::new("input:catalog", "sha256:old:input:catalog", None).unwrap();
        let report = graph().assess(vec![unknown], "compiler:1", "profile:1").unwrap();
        assert!(report.reconciliation_required());
        assert!(report.invalidated().contains_key("artifact:compatibility"));
    }
    #[test]
    fn unchanged_digest_is_a_noop() {
        let same = InputChange::new("input:catalog", "sha256:old:input:catalog", Some("sha256:old:input:catalog".into())).unwrap();
        let report = graph().assess(vec![same], "compiler:1", "profile:1").unwrap();
        assert!(report.invalidated().is_empty());
        assert_eq!(report.retained().len(), 4);
    }
    #[test]
    fn stale_expected_digest_fails_closed() {
        let stale = InputChange::new("input:catalog", "sha256:wrong", Some("sha256:new".into())).unwrap();
        assert!(matches!(graph().assess(vec![stale], "compiler:1", "profile:1"), Err(ChangeImpactError::StaleExpectedInputDigest { .. })));
    }
    #[test]
    fn unknown_changed_input_is_rejected() {
        let unknown = InputChange::new("input:unknown", "old", Some("new".into())).unwrap();
        assert!(matches!(graph().assess(vec![unknown], "compiler:1", "profile:1"), Err(ChangeImpactError::UnknownChangedInput { .. })));
    }
    #[test]
    fn duplicate_changed_input_is_rejected() {
        assert!(matches!(graph().assess(vec![change("input:catalog"), change("input:catalog")], "compiler:1", "profile:1"), Err(ChangeImpactError::DuplicateChangedInput { .. })));
    }
    #[test]
    fn duplicate_input_and_artifact_ids_are_rejected() {
        assert!(matches!(DependencyGraph::new(vec![input("same"), input("same")], Vec::new()), Err(ChangeImpactError::DuplicateInput { .. })));
        assert!(matches!(DependencyGraph::new(vec![input("in")], vec![artifact("same", &[]), artifact("same", &[])]), Err(ChangeImpactError::DuplicateArtifact { .. })));
    }
    #[test]
    fn duplicate_dependency_is_rejected() {
        let duplicate = DerivedArtifact::new("a", "out", "c", "p", vec!["x".into(), "x".into()], true);
        assert!(matches!(duplicate, Err(ChangeImpactError::DuplicateDependency { .. })));
    }
    #[test]
    fn unknown_dependency_is_rejected() {
        let invalid = artifact("artifact:a", &["input:missing"]);
        assert!(matches!(DependencyGraph::new(Vec::new(), vec![invalid]), Err(ChangeImpactError::UnknownDependency { .. })));
    }
    #[test]
    fn dependency_cycle_is_rejected() {
        let a = artifact("artifact:a", &["artifact:b"]);
        let b = artifact("artifact:b", &["artifact:a"]);
        assert!(matches!(DependencyGraph::new(Vec::new(), vec![a, b]), Err(ChangeImpactError::DependencyCycle { .. })));
    }
    #[test]
    fn incomplete_dependency_inventory_invalidates_even_without_change() {
        let incomplete = DerivedArtifact::new("artifact:emergency", "out", "compiler:1", "profile:1",
            vec!["input:topology".into()], false).unwrap();
        let graph = DependencyGraph::new(vec![input("input:topology")], vec![incomplete]).unwrap();
        let report = graph.assess(Vec::new(), "compiler:1", "profile:1").unwrap();
        assert!(report.invalidated()["artifact:emergency"].contains(&InvalidationReason::DependencyInventoryIncomplete));
    }
    #[test]
    fn compiler_or_profile_change_invalidates_artifacts() {
        let report = graph().assess(Vec::new(), "compiler:2", "profile:1").unwrap();
        assert_eq!(report.invalidated().len(), 4);
        assert!(report.invalidated()["artifact:compatibility"].contains(&InvalidationReason::CompilerVersionChanged));
    }
    #[test]
    fn stale_upstream_artifact_invalidates_transitive_dependents() {
        let artifacts = vec![
            DerivedArtifact::new("artifact:a", "out-a", "compiler:0", "profile:1", vec!["input:source".into()], true).unwrap(),
            DerivedArtifact::new("artifact:b", "out-b", "compiler:1", "profile:1", vec!["artifact:a".into()], true).unwrap(),
        ];
        let graph = DependencyGraph::new(vec![input("input:source")], artifacts).unwrap();
        let report = graph.assess(Vec::new(), "compiler:1", "profile:1").unwrap();
        assert!(report.invalidated().contains_key("artifact:a"));
        assert!(report.invalidated().contains_key("artifact:b"));
        assert!(report.invalidated()["artifact:b"].contains(&InvalidationReason::TransitiveDependencyChanged));
    }
    #[test]
    fn input_and_artifact_identity_collision_is_rejected() {
        assert!(matches!(DependencyGraph::new(vec![input("same")], vec![artifact("same", &[])]), Err(ChangeImpactError::InputArtifactIdentityCollision { .. })));
    }
}
