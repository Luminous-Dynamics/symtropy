//! Deterministic structural validator for normalized construction work graphs.
//!
//! This crate does not parse IFC, author/approve construction methods, authenticate
//! authorities, establish robot capability, approve safety, issue motion commands,
//! measure physical work, or accept construction output. It checks graph structure and
//! declared source/method/transformation lineage only.

pub mod compiler;

use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskGraph {
    pub graph_id: String,
    pub project_id: String,
    pub site_id: String,
    pub work_package_id: String,
    pub model_digest: String,
    pub design_revision: String,
    pub target_frame_id: String,
    pub site_epoch: String,
    pub method_template_id: String,
    pub method_revision_digest: String,
    pub method_approval_ref: String,
    pub compiler_version: String,
    pub rule_set_digest: String,
    pub nodes: Vec<TaskNode>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskNode {
    pub task_id: String,
    pub task_family: String,
    pub source_element_ids: Vec<String>,
    pub method_step_id: String,
    pub transformation_rule_id: String,
    pub transformation_rule_digest: String,
    pub functional_unit_class: String,
    pub dependencies: Vec<String>,
    pub parameters: Vec<ParameterBinding>,
    pub required_capabilities: Vec<String>,
    pub required_resources: Vec<String>,
    pub preconditions: Vec<String>,
    pub postconditions: Vec<String>,
    pub required_evidence: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParameterBinding {
    pub name: String,
    pub source_property: String,
    pub value: String,
    pub unit: String,
    pub source_ref: String,
    pub source_unit: Option<String>,
    pub conversion_evidence_ref: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphDiagnostic {
    pub code: &'static str,
    pub node_id: Option<String>,
    pub detail: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidationReport {
    /// Stable lexicographically tie-broken topological order.
    pub topological_order: Vec<String>,
    /// Sorted diagnostics for deterministic report serialization by callers.
    pub diagnostics: Vec<GraphDiagnostic>,
}

impl ValidationReport {
    pub fn is_valid(&self) -> bool {
        self.diagnostics.is_empty()
    }
}

impl TaskGraph {
    pub fn validate(&self) -> ValidationReport {
        let mut diagnostics = Vec::new();

        self.validate_header(&mut diagnostics);
        if self.nodes.is_empty() {
            diagnostics.push(diagnostic(
                "empty_task_graph",
                None,
                "graph must contain at least one task node",
            ));
        }

        let mut nodes_by_id: BTreeMap<&str, &TaskNode> = BTreeMap::new();
        for node in &self.nodes {
            if node.task_id.trim().is_empty() {
                diagnostics.push(diagnostic(
                    "empty_task_id",
                    None,
                    "task node has an empty identity",
                ));
                continue;
            }

            if nodes_by_id.insert(node.task_id.as_str(), node).is_some() {
                diagnostics.push(diagnostic(
                    "duplicate_task_id",
                    Some(&node.task_id),
                    "task IDs must be unique within a graph",
                ));
            }

            validate_node(node, &mut diagnostics);
        }

        let topological_order = if nodes_by_id.len() == self.nodes.len() {
            validate_dependencies(&nodes_by_id, &mut diagnostics)
        } else {
            Vec::new()
        };

        diagnostics.sort_by(|left, right| {
            (
                left.code,
                left.node_id.as_deref().unwrap_or(""),
                left.detail.as_str(),
            )
                .cmp(&(
                    right.code,
                    right.node_id.as_deref().unwrap_or(""),
                    right.detail.as_str(),
                ))
        });

        ValidationReport {
            topological_order,
            diagnostics,
        }
    }

    fn validate_header(&self, diagnostics: &mut Vec<GraphDiagnostic>) {
        for (name, value) in [
            ("graph_id", self.graph_id.as_str()),
            ("project_id", self.project_id.as_str()),
            ("site_id", self.site_id.as_str()),
            ("work_package_id", self.work_package_id.as_str()),
            ("model_digest", self.model_digest.as_str()),
            ("design_revision", self.design_revision.as_str()),
            ("target_frame_id", self.target_frame_id.as_str()),
            ("site_epoch", self.site_epoch.as_str()),
            ("method_template_id", self.method_template_id.as_str()),
            (
                "method_revision_digest",
                self.method_revision_digest.as_str(),
            ),
            ("method_approval_ref", self.method_approval_ref.as_str()),
            ("compiler_version", self.compiler_version.as_str()),
            ("rule_set_digest", self.rule_set_digest.as_str()),
        ] {
            if value.trim().is_empty() {
                diagnostics.push(diagnostic("missing_graph_header_field", None, name));
            }
        }
    }
}

fn validate_node(node: &TaskNode, diagnostics: &mut Vec<GraphDiagnostic>) {
    let node_id = Some(node.task_id.as_str());

    for (name, value) in [
        ("task_family", node.task_family.as_str()),
        ("method_step_id", node.method_step_id.as_str()),
        (
            "transformation_rule_id",
            node.transformation_rule_id.as_str(),
        ),
        (
            "transformation_rule_digest",
            node.transformation_rule_digest.as_str(),
        ),
        ("functional_unit_class", node.functional_unit_class.as_str()),
    ] {
        if value.trim().is_empty() {
            diagnostics.push(diagnostic("missing_task_derivation_field", node_id, name));
        }
    }

    validate_string_list(
        "source_element_ids",
        &node.source_element_ids,
        true,
        node_id,
        diagnostics,
    );
    validate_string_list(
        "required_capabilities",
        &node.required_capabilities,
        true,
        node_id,
        diagnostics,
    );
    validate_string_list(
        "preconditions",
        &node.preconditions,
        true,
        node_id,
        diagnostics,
    );
    validate_string_list(
        "postconditions",
        &node.postconditions,
        true,
        node_id,
        diagnostics,
    );
    validate_string_list(
        "required_evidence",
        &node.required_evidence,
        true,
        node_id,
        diagnostics,
    );
    validate_string_list(
        "dependencies",
        &node.dependencies,
        false,
        node_id,
        diagnostics,
    );

    let mut parameter_names = BTreeSet::new();
    for parameter in &node.parameters {
        if parameter.name.trim().is_empty()
            || parameter.source_property.trim().is_empty()
            || parameter.value.trim().is_empty()
            || parameter.unit.trim().is_empty()
            || parameter.source_ref.trim().is_empty()
        {
            diagnostics.push(diagnostic(
                "incomplete_parameter_binding",
                node_id,
                "parameter requires name, source property, value, unit and source reference",
            ));
        }

        if !parameter_names.insert(parameter.name.as_str()) {
            diagnostics.push(diagnostic(
                "duplicate_parameter_name",
                node_id,
                parameter.name.as_str(),
            ));
        }

        if let Some(source_unit) = &parameter.source_unit {
            if source_unit.trim().is_empty() {
                diagnostics.push(diagnostic(
                    "empty_source_unit",
                    node_id,
                    parameter.name.as_str(),
                ));
            } else if source_unit != &parameter.unit
                && parameter
                    .conversion_evidence_ref
                    .as_deref()
                    .is_none_or(str::is_empty)
            {
                diagnostics.push(diagnostic(
                    "unit_conversion_without_evidence",
                    node_id,
                    parameter.name.as_str(),
                ));
            }
        } else if parameter.conversion_evidence_ref.is_some() {
            diagnostics.push(diagnostic(
                "conversion_evidence_without_source_unit",
                node_id,
                parameter.name.as_str(),
            ));
        }
    }
}

fn validate_string_list(
    field: &'static str,
    values: &[String],
    required: bool,
    node_id: Option<&str>,
    diagnostics: &mut Vec<GraphDiagnostic>,
) {
    if required && values.is_empty() {
        diagnostics.push(diagnostic("empty_required_task_list", node_id, field));
    }

    let mut seen = BTreeSet::new();
    for value in values {
        if value.trim().is_empty() {
            diagnostics.push(diagnostic("empty_list_item", node_id, field));
        } else if !seen.insert(value.as_str()) {
            diagnostics.push(diagnostic("duplicate_list_item", node_id, field));
        }
    }
}

fn validate_dependencies(
    nodes_by_id: &BTreeMap<&str, &TaskNode>,
    diagnostics: &mut Vec<GraphDiagnostic>,
) -> Vec<String> {
    let mut indegree: BTreeMap<&str, usize> =
        nodes_by_id.keys().map(|node_id| (*node_id, 0)).collect();
    let mut outgoing: BTreeMap<&str, BTreeSet<&str>> = nodes_by_id
        .keys()
        .map(|node_id| (*node_id, BTreeSet::new()))
        .collect();

    for (node_id, node) in nodes_by_id {
        let mut seen_dependencies = BTreeSet::new();
        for dependency in &node.dependencies {
            if !seen_dependencies.insert(dependency.as_str()) {
                continue;
            }

            if dependency.as_str() == *node_id {
                diagnostics.push(diagnostic("self_dependency", Some(node_id), dependency));
                continue;
            }

            if !nodes_by_id.contains_key(dependency.as_str()) {
                diagnostics.push(diagnostic("unknown_dependency", Some(node_id), dependency));
                continue;
            }

            *indegree
                .get_mut(node_id)
                .expect("all node IDs are initialized") += 1;
            outgoing
                .get_mut(dependency.as_str())
                .expect("known dependency has an outgoing edge set")
                .insert(node_id);
        }
    }

    let mut ready: BTreeSet<&str> = indegree
        .iter()
        .filter_map(|(node_id, degree)| (*degree == 0).then_some(*node_id))
        .collect();
    let mut order = Vec::with_capacity(nodes_by_id.len());

    while let Some(node_id) = ready.pop_first() {
        order.push(node_id.to_owned());

        if let Some(children) = outgoing.get(node_id) {
            for child in children {
                let degree = indegree
                    .get_mut(child)
                    .expect("outgoing child exists in indegree map");
                *degree -= 1;
                if *degree == 0 {
                    ready.insert(child);
                }
            }
        }
    }

    if order.len() != nodes_by_id.len() {
        diagnostics.push(diagnostic(
            "dependency_cycle",
            None,
            "task graph contains a dependency cycle",
        ));
        Vec::new()
    } else {
        order
    }
}

fn diagnostic(
    code: &'static str,
    node_id: Option<&str>,
    detail: impl Into<String>,
) -> GraphDiagnostic {
    GraphDiagnostic {
        code,
        node_id: node_id.map(str::to_owned),
        detail: detail.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn graph(nodes: Vec<TaskNode>) -> TaskGraph {
        TaskGraph {
            graph_id: "graph-1".to_owned(),
            project_id: "project-1".to_owned(),
            site_id: "site-1".to_owned(),
            work_package_id: "wp-1".to_owned(),
            model_digest: "sha256:model-1".to_owned(),
            design_revision: "design-rev-1".to_owned(),
            target_frame_id: "project-grid-1".to_owned(),
            site_epoch: "site-epoch-1".to_owned(),
            method_template_id: "method-install-1".to_owned(),
            method_revision_digest: "sha256:method-1".to_owned(),
            method_approval_ref: "approval-method-1".to_owned(),
            compiler_version: "compiler-0.1.0".to_owned(),
            rule_set_digest: "sha256:rules-1".to_owned(),
            nodes,
        }
    }

    fn node(id: &str, dependencies: &[&str]) -> TaskNode {
        TaskNode {
            task_id: id.to_owned(),
            task_family: "piling".to_owned(),
            source_element_ids: vec![format!("element-{id}")],
            method_step_id: "install".to_owned(),
            transformation_rule_id: "rule-element-install".to_owned(),
            transformation_rule_digest: "sha256:rule-1".to_owned(),
            functional_unit_class: "installed-component".to_owned(),
            dependencies: dependencies
                .iter()
                .map(|value| (*value).to_owned())
                .collect(),
            parameters: vec![ParameterBinding {
                name: "embedment_depth".to_owned(),
                source_property: "embedment_depth".to_owned(),
                value: "1200".to_owned(),
                unit: "mm".to_owned(),
                source_ref: "design.property.embedment_depth".to_owned(),
                source_unit: None,
                conversion_evidence_ref: None,
            }],
            required_capabilities: vec!["rotary-installation".to_owned()],
            required_resources: vec!["material-staging".to_owned()],
            preconditions: vec!["work-zone-released".to_owned()],
            postconditions: vec!["component-installed".to_owned()],
            required_evidence: vec!["independent-as-built-inspection".to_owned()],
        }
    }

    #[test]
    fn valid_dag_produces_deterministic_topological_order() {
        let report = graph(vec![
            node("task-c", &["task-a", "task-b"]),
            node("task-b", &[]),
            node("task-a", &[]),
        ])
        .validate();

        assert!(report.is_valid());
        assert_eq!(
            report.topological_order,
            vec![
                "task-a".to_owned(),
                "task-b".to_owned(),
                "task-c".to_owned()
            ]
        );
    }

    #[test]
    fn input_vector_permutation_does_not_change_topological_order() {
        let first = graph(vec![
            node("task-c", &["task-a", "task-b"]),
            node("task-b", &[]),
            node("task-a", &[]),
        ])
        .validate();
        let second = graph(vec![
            node("task-a", &[]),
            node("task-c", &["task-b", "task-a"]),
            node("task-b", &[]),
        ])
        .validate();

        assert_eq!(first.topological_order, second.topological_order);
    }

    #[test]
    fn independent_parallel_nodes_use_stable_lexicographic_tiebreak() {
        let report = graph(vec![node("z-task", &[]), node("a-task", &[])]).validate();

        assert_eq!(
            report.topological_order,
            vec!["a-task".to_owned(), "z-task".to_owned()]
        );
    }

    #[test]
    fn empty_graph_is_invalid() {
        let report = graph(Vec::new()).validate();

        assert!(!report.is_valid());
        assert!(
            report
                .diagnostics
                .iter()
                .any(|item| item.code == "empty_task_graph")
        );
    }

    #[test]
    fn missing_graph_approval_and_revision_fields_are_reported() {
        let mut input = graph(vec![node("task-a", &[])]);
        input.method_approval_ref.clear();
        input.design_revision.clear();

        let report = input.validate();

        assert!(
            report
                .diagnostics
                .iter()
                .any(|item| item.code == "missing_graph_header_field")
        );
    }

    #[test]
    fn duplicate_task_ids_are_rejected() {
        let report = graph(vec![node("task-a", &[]), node("task-a", &[])]).validate();

        assert!(
            report
                .diagnostics
                .iter()
                .any(|item| item.code == "duplicate_task_id")
        );
        assert!(report.topological_order.is_empty());
    }

    #[test]
    fn missing_source_element_lineage_is_rejected() {
        let mut task = node("task-a", &[]);
        task.source_element_ids.clear();

        let report = graph(vec![task]).validate();

        assert!(
            report
                .diagnostics
                .iter()
                .any(|item| item.code == "empty_required_task_list")
        );
    }

    #[test]
    fn missing_method_step_or_transformation_digest_is_rejected() {
        let mut task = node("task-a", &[]);
        task.method_step_id.clear();
        task.transformation_rule_digest.clear();

        let report = graph(vec![task]).validate();

        assert!(
            report
                .diagnostics
                .iter()
                .any(|item| item.code == "missing_task_derivation_field")
        );
    }

    #[test]
    fn unknown_dependency_is_rejected() {
        let report = graph(vec![node("task-a", &["task-missing"])]).validate();

        assert!(
            report
                .diagnostics
                .iter()
                .any(|item| item.code == "unknown_dependency")
        );
    }

    #[test]
    fn self_dependency_is_rejected() {
        let report = graph(vec![node("task-a", &["task-a"])]).validate();

        assert!(
            report
                .diagnostics
                .iter()
                .any(|item| item.code == "self_dependency")
        );
    }

    #[test]
    fn multi_node_dependency_cycle_is_rejected() {
        let report = graph(vec![
            node("task-a", &["task-c"]),
            node("task-b", &["task-a"]),
            node("task-c", &["task-b"]),
        ])
        .validate();

        assert!(
            report
                .diagnostics
                .iter()
                .any(|item| item.code == "dependency_cycle")
        );
    }

    #[test]
    fn duplicate_dependency_edges_are_rejected() {
        let report = graph(vec![
            node("task-a", &[]),
            node("task-b", &["task-a", "task-a"]),
        ])
        .validate();

        assert!(
            report
                .diagnostics
                .iter()
                .any(|item| item.code == "duplicate_list_item")
        );
    }

    #[test]
    fn missing_acceptance_evidence_is_rejected() {
        let mut task = node("task-a", &[]);
        task.required_evidence.clear();

        let report = graph(vec![task]).validate();

        assert!(
            report
                .diagnostics
                .iter()
                .any(|item| item.code == "empty_required_task_list")
        );
    }

    #[test]
    fn missing_precondition_or_postcondition_is_rejected() {
        let mut task = node("task-a", &[]);
        task.preconditions.clear();
        task.postconditions.clear();

        let report = graph(vec![task]).validate();

        assert_eq!(
            report
                .diagnostics
                .iter()
                .filter(|item| item.code == "empty_required_task_list")
                .count(),
            2
        );
    }

    #[test]
    fn duplicate_parameter_names_are_rejected() {
        let mut task = node("task-a", &[]);
        task.parameters.push(task.parameters[0].clone());

        let report = graph(vec![task]).validate();

        assert!(
            report
                .diagnostics
                .iter()
                .any(|item| item.code == "duplicate_parameter_name")
        );
    }

    #[test]
    fn unit_conversion_requires_evidence() {
        let mut task = node("task-a", &[]);
        task.parameters[0].source_unit = Some("m".to_owned());
        task.parameters[0].unit = "mm".to_owned();

        let report = graph(vec![task]).validate();

        assert!(
            report
                .diagnostics
                .iter()
                .any(|item| item.code == "unit_conversion_without_evidence")
        );
    }

    #[test]
    fn unit_conversion_with_explicit_evidence_is_valid() {
        let mut task = node("task-a", &[]);
        task.parameters[0].source_unit = Some("m".to_owned());
        task.parameters[0].unit = "mm".to_owned();
        task.parameters[0].conversion_evidence_ref = Some("conversion-record-1".to_owned());

        assert!(graph(vec![task]).validate().is_valid());
    }

    #[test]
    fn conversion_evidence_without_source_unit_is_rejected() {
        let mut task = node("task-a", &[]);
        task.parameters[0].conversion_evidence_ref = Some("conversion-record-1".to_owned());

        let report = graph(vec![task]).validate();

        assert!(
            report
                .diagnostics
                .iter()
                .any(|item| item.code == "conversion_evidence_without_source_unit")
        );
    }

    #[test]
    fn source_element_identity_is_unique_within_a_task() {
        let mut task = node("task-a", &[]);
        task.source_element_ids
            .push(task.source_element_ids[0].clone());

        let report = graph(vec![task]).validate();

        assert!(
            report
                .diagnostics
                .iter()
                .any(|item| item.code == "duplicate_list_item")
        );
    }


    #[test]
    fn task_family_is_a_required_output_semantic() {
        let mut task = node("task-a", &[]);
        task.task_family.clear();
        let report = graph(vec![task]).validate();
        assert!(
            report
                .diagnostics
                .iter()
                .any(|item| item.code == "missing_task_derivation_field")
        );
    }

    #[test]
    fn source_property_name_cannot_be_dropped_from_parameter_lineage() {
        let mut task = node("task-a", &[]);
        task.parameters[0].source_property.clear();
        let report = graph(vec![task]).validate();
        assert!(
            report
                .diagnostics
                .iter()
                .any(|item| item.code == "incomplete_parameter_binding")
        );
    }

    #[test]
    fn diagnostic_order_is_deterministic() {
        let mut task = node("task-a", &[]);
        task.method_step_id.clear();
        task.transformation_rule_id.clear();
        task.required_evidence.clear();

        let first = graph(vec![task.clone()]).validate();
        let second = graph(vec![task]).validate();

        assert_eq!(first.diagnostics, second.diagnostics);
    }

    #[test]
    fn missing_capability_is_not_inferred_from_task_family() {
        let mut task = node("task-a", &[]);
        task.required_capabilities.clear();

        let report = graph(vec![task]).validate();

        assert!(
            report
                .diagnostics
                .iter()
                .any(|item| item.code == "empty_required_task_list")
        );
    }

    #[test]
    fn duplicate_method_step_across_distinct_elements_is_allowed() {
        let mut first = node("task-a", &[]);
        first.method_step_id = "install".to_owned();

        let mut second = node("task-b", &["task-a"]);
        second.method_step_id = "install".to_owned();

        assert!(graph(vec![first, second]).validate().is_valid());
    }
}
