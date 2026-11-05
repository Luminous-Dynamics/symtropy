//! Deterministic instantiation of approved construction method templates.
//!
//! Inputs are normalized design facts and an externally approved method template.
//! This module does not parse IFC, approve a method, calculate geometry, infer robot
//! capability, authenticate approvals, establish safety or authorize execution.

use std::collections::{BTreeMap, BTreeSet};

use crate::{ParameterBinding, TaskGraph, TaskNode};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DesignManifest {
    pub project_id: String,
    pub site_id: String,
    pub work_package_id: String,
    pub model_digest: String,
    pub design_revision: String,
    pub target_frame_id: String,
    pub site_epoch: String,
    pub elements: Vec<DesignElement>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DesignElement {
    pub element_id: String,
    pub task_family: String,
    pub properties: Vec<DesignProperty>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DesignProperty {
    pub name: String,
    pub value: String,
    pub unit: String,
    pub source_ref: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MethodTemplate {
    pub template_id: String,
    pub revision_digest: String,
    /// Opaque reference to the external project/contract authority's approval.
    pub approval_ref: String,
    pub supported_task_families: Vec<String>,
    pub steps: Vec<MethodStep>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MethodStep {
    pub step_id: String,
    pub functional_unit_class: String,
    pub transformation_rule_id: String,
    pub transformation_rule_digest: String,
    /// Step IDs that must be completed for the same design element first.
    pub dependencies: Vec<String>,
    pub parameters: Vec<ParameterMapping>,
    pub required_capabilities: Vec<String>,
    pub required_resources: Vec<String>,
    pub preconditions: Vec<String>,
    pub postconditions: Vec<String>,
    pub required_evidence: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParameterMapping {
    pub target_name: String,
    pub source_property: String,
    pub target_unit: String,
    /// For a unit change, the conversion is computed/approved externally and referenced
    /// here. This compiler never performs an implicit conversion or invents a value.
    pub conversion: Option<ApprovedConversion>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ApprovedConversion {
    pub converted_value: String,
    pub evidence_ref: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CompileVerdict {
    CompiledForReview,
    RejectedUnsupported,
    RejectedInconsistent,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompileDiagnostic {
    pub code: &'static str,
    pub element_id: Option<String>,
    pub step_id: Option<String>,
    pub node_id: Option<String>,
    pub detail: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompileResult {
    pub verdict: CompileVerdict,
    pub graph: Option<TaskGraph>,
    pub topological_order: Vec<String>,
    pub diagnostics: Vec<CompileDiagnostic>,
}

impl CompileResult {
    pub fn is_compiled_for_review(&self) -> bool {
        self.verdict == CompileVerdict::CompiledForReview && self.graph.is_some()
    }
}

/// Instantiate every method step against every supported design element.
///
/// The same element's method-step dependencies become task-graph edges. Element and step
/// ordering in the input vectors is non-semantic; generated nodes and parameter mappings
/// are sorted deterministically before graph validation.
pub fn compile_work_graph(
    manifest: &DesignManifest,
    method: &MethodTemplate,
    compiler_version: &str,
    rule_set_digest: &str,
) -> CompileResult {
    let mut diagnostics = Vec::new();

    for (field, value) in [
        ("project_id", manifest.project_id.as_str()),
        ("site_id", manifest.site_id.as_str()),
        ("work_package_id", manifest.work_package_id.as_str()),
        ("model_digest", manifest.model_digest.as_str()),
        ("design_revision", manifest.design_revision.as_str()),
        ("target_frame_id", manifest.target_frame_id.as_str()),
        ("site_epoch", manifest.site_epoch.as_str()),
        ("method_template_id", method.template_id.as_str()),
        ("method_revision_digest", method.revision_digest.as_str()),
        ("method_approval_ref", method.approval_ref.as_str()),
        ("compiler_version", compiler_version),
        ("rule_set_digest", rule_set_digest),
    ] {
        if value.trim().is_empty() {
            diagnostics.push(compile_diagnostic(
                "missing_compile_header",
                None,
                None,
                field,
            ));
        }
    }

    if method.steps.is_empty() {
        diagnostics.push(compile_diagnostic(
            "empty_method_template",
            None,
            None,
            "approved method template has no steps",
        ));
    }
    if method.supported_task_families.is_empty() {
        diagnostics.push(compile_diagnostic(
            "empty_method_applicability",
            None,
            None,
            "method template declares no supported task families",
        ));
    }

    let mut step_by_id: BTreeMap<&str, &MethodStep> = BTreeMap::new();
    for step in &method.steps {
        if step.step_id.trim().is_empty() {
            diagnostics.push(compile_diagnostic(
                "empty_method_step_id",
                None,
                None,
                "method step has an empty identity",
            ));
            continue;
        }

        if step_by_id.insert(step.step_id.as_str(), step).is_some() {
            diagnostics.push(compile_diagnostic(
                "duplicate_method_step_id",
                None,
                Some(step.step_id.as_str()),
                "method step IDs must be unique within a template",
            ));
        }
    }

    for step in &method.steps {
        for dependency in &step.dependencies {
            if !step_by_id.contains_key(dependency.as_str()) {
                diagnostics.push(compile_diagnostic(
                    "unknown_method_step_dependency",
                    None,
                    Some(step.step_id.as_str()),
                    dependency.as_str(),
                ));
            }
        }
    }

    let mut elements: Vec<&DesignElement> = manifest.elements.iter().collect();
    elements.sort_by(|left, right| left.element_id.cmp(&right.element_id));

    let mut element_ids = BTreeSet::new();
    let mut nodes = Vec::new();
    let mut unsupported = false;

    for element in &elements {
        if element.element_id.trim().is_empty() {
            diagnostics.push(compile_diagnostic(
                "empty_design_element_id",
                None,
                None,
                "design element has an empty identity",
            ));
            continue;
        }
        if !element_ids.insert(element.element_id.as_str()) {
            diagnostics.push(compile_diagnostic(
                "duplicate_design_element_id",
                Some(element.element_id.as_str()),
                None,
                "design element IDs must be unique within the manifest",
            ));
            continue;
        }

        if element.task_family.trim().is_empty() {
            diagnostics.push(compile_diagnostic(
                "missing_task_family",
                Some(element.element_id.as_str()),
                None,
                "design element has no declared task family",
            ));
            continue;
        }

        if !method
            .supported_task_families
            .iter()
            .any(|family| family == &element.task_family)
        {
            unsupported = true;
            diagnostics.push(compile_diagnostic(
                "unsupported_task_family",
                Some(element.element_id.as_str()),
                None,
                element.task_family.as_str(),
            ));
            continue;
        }

        let mut steps: Vec<&MethodStep> = method.steps.iter().collect();
        steps.sort_by(|left, right| left.step_id.cmp(&right.step_id));

        for step in steps {
            let mut parameter_bindings = Vec::new();
            let mut parameter_names = BTreeSet::new();
            let mut step_has_error = false;

            for mapping in &step.parameters {
                if mapping.target_name.trim().is_empty()
                    || mapping.source_property.trim().is_empty()
                    || mapping.target_unit.trim().is_empty()
                {
                    diagnostics.push(compile_diagnostic(
                        "incomplete_parameter_mapping",
                        Some(element.element_id.as_str()),
                        Some(step.step_id.as_str()),
                        "target name, source property and target unit are required",
                    ));
                    step_has_error = true;
                    continue;
                }

                if !parameter_names.insert(mapping.target_name.as_str()) {
                    diagnostics.push(compile_diagnostic(
                        "duplicate_parameter_mapping",
                        Some(element.element_id.as_str()),
                        Some(step.step_id.as_str()),
                        mapping.target_name.as_str(),
                    ));
                    step_has_error = true;
                    continue;
                }

                let matching_properties: Vec<&DesignProperty> = element
                    .properties
                    .iter()
                    .filter(|property| property.name == mapping.source_property)
                    .collect();

                let property = match matching_properties.as_slice() {
                    [] => {
                        diagnostics.push(compile_diagnostic(
                            "missing_design_property",
                            Some(element.element_id.as_str()),
                            Some(step.step_id.as_str()),
                            mapping.source_property.as_str(),
                        ));
                        step_has_error = true;
                        continue;
                    }
                    [property] => *property,
                    _ => {
                        diagnostics.push(compile_diagnostic(
                            "ambiguous_design_property",
                            Some(element.element_id.as_str()),
                            Some(step.step_id.as_str()),
                            mapping.source_property.as_str(),
                        ));
                        step_has_error = true;
                        continue;
                    }
                };

                if property.value.trim().is_empty()
                    || property.unit.trim().is_empty()
                    || property.source_ref.trim().is_empty()
                {
                    diagnostics.push(compile_diagnostic(
                        "incomplete_design_property",
                        Some(element.element_id.as_str()),
                        Some(step.step_id.as_str()),
                        mapping.source_property.as_str(),
                    ));
                    step_has_error = true;
                    continue;
                }

                let (value, conversion_evidence_ref) = if property.unit == mapping.target_unit {
                    if mapping.conversion.is_some() {
                        diagnostics.push(compile_diagnostic(
                            "unnecessary_unit_conversion",
                            Some(element.element_id.as_str()),
                            Some(step.step_id.as_str()),
                            mapping.target_name.as_str(),
                        ));
                        step_has_error = true;
                        continue;
                    }
                    (property.value.clone(), None)
                } else {
                    match &mapping.conversion {
                        Some(conversion)
                            if !conversion.converted_value.trim().is_empty()
                                && !conversion.evidence_ref.trim().is_empty() =>
                        {
                            (
                                conversion.converted_value.clone(),
                                Some(conversion.evidence_ref.clone()),
                            )
                        }
                        Some(_) => {
                            diagnostics.push(compile_diagnostic(
                                "incomplete_unit_conversion_receipt",
                                Some(element.element_id.as_str()),
                                Some(step.step_id.as_str()),
                                mapping.target_name.as_str(),
                            ));
                            step_has_error = true;
                            continue;
                        }
                        None => {
                            diagnostics.push(compile_diagnostic(
                                "unit_conversion_receipt_required",
                                Some(element.element_id.as_str()),
                                Some(step.step_id.as_str()),
                                mapping.target_name.as_str(),
                            ));
                            step_has_error = true;
                            continue;
                        }
                    }
                };

                parameter_bindings.push(ParameterBinding {
                    name: mapping.target_name.clone(),
                    source_property: mapping.source_property.clone(),
                    value,
                    unit: mapping.target_unit.clone(),
                    source_ref: property.source_ref.clone(),
                    source_unit: Some(property.unit.clone()),
                    conversion_evidence_ref,
                });
            }

            if step_has_error {
                continue;
            }

            parameter_bindings.sort_by(|left, right| left.name.cmp(&right.name));
            let mut dependencies: Vec<String> = step
                .dependencies
                .iter()
                .map(|dependency| task_id(&element.element_id, dependency))
                .collect();
            dependencies.sort();

            let mut required_capabilities = step.required_capabilities.clone();
            required_capabilities.sort();
            let mut required_resources = step.required_resources.clone();
            required_resources.sort();
            let mut preconditions = step.preconditions.clone();
            preconditions.sort();
            let mut postconditions = step.postconditions.clone();
            postconditions.sort();
            let mut required_evidence = step.required_evidence.clone();
            required_evidence.sort();

            nodes.push(TaskNode {
                task_id: task_id(&element.element_id, &step.step_id),
                task_family: element.task_family.clone(),
                source_element_ids: vec![element.element_id.clone()],
                method_step_id: step.step_id.clone(),
                transformation_rule_id: step.transformation_rule_id.clone(),
                transformation_rule_digest: step.transformation_rule_digest.clone(),
                functional_unit_class: step.functional_unit_class.clone(),
                dependencies,
                parameters: parameter_bindings,
                required_capabilities,
                required_resources,
                preconditions,
                postconditions,
                required_evidence,
            });
        }
    }

    if !diagnostics.is_empty() {
        diagnostics.sort_by(compile_diagnostic_order);
        return CompileResult {
            verdict: if unsupported {
                CompileVerdict::RejectedUnsupported
            } else {
                CompileVerdict::RejectedInconsistent
            },
            graph: None,
            topological_order: Vec::new(),
            diagnostics,
        };
    }

    nodes.sort_by(|left, right| left.task_id.cmp(&right.task_id));
    let graph = TaskGraph {
        graph_id: graph_identity(manifest, method, compiler_version, rule_set_digest),
        project_id: manifest.project_id.clone(),
        site_id: manifest.site_id.clone(),
        work_package_id: manifest.work_package_id.clone(),
        model_digest: manifest.model_digest.clone(),
        design_revision: manifest.design_revision.clone(),
        target_frame_id: manifest.target_frame_id.clone(),
        site_epoch: manifest.site_epoch.clone(),
        method_template_id: method.template_id.clone(),
        method_revision_digest: method.revision_digest.clone(),
        method_approval_ref: method.approval_ref.clone(),
        compiler_version: compiler_version.to_owned(),
        rule_set_digest: rule_set_digest.to_owned(),
        nodes,
    };

    let report = graph.validate();
    if !report.is_valid() {
        let diagnostics = report
            .diagnostics
            .into_iter()
            .map(|item| CompileDiagnostic {
                code: item.code,
                element_id: None,
                step_id: None,
                node_id: item.node_id,
                detail: item.detail,
            })
            .collect();

        return CompileResult {
            verdict: CompileVerdict::RejectedInconsistent,
            graph: None,
            topological_order: Vec::new(),
            diagnostics,
        };
    }

    CompileResult {
        verdict: CompileVerdict::CompiledForReview,
        graph: Some(graph),
        topological_order: report.topological_order,
        diagnostics: Vec::new(),
    }
}

/// Length-prefix fields so different identifier pairs cannot collide through delimiters.
/// This is a deterministic identity label, not a cryptographic digest.
pub fn task_id(element_id: &str, step_id: &str) -> String {
    format!(
        "task:e{}:{}:s{}:{}",
        element_id.len(),
        element_id,
        step_id.len(),
        step_id
    )
}

/// Combines declared source/compiler identities into a deterministic graph label. It is
/// not a content hash or tamper-evident receipt; authoritative digests remain upstream.
fn graph_identity(
    manifest: &DesignManifest,
    method: &MethodTemplate,
    compiler_version: &str,
    rule_set_digest: &str,
) -> String {
    let fields = [
        manifest.project_id.as_str(),
        manifest.site_id.as_str(),
        manifest.work_package_id.as_str(),
        manifest.model_digest.as_str(),
        manifest.design_revision.as_str(),
        manifest.target_frame_id.as_str(),
        manifest.site_epoch.as_str(),
        method.template_id.as_str(),
        method.revision_digest.as_str(),
        compiler_version,
        rule_set_digest,
    ];
    let mut output = String::from("graph:");
    for field in fields {
        output.push_str(&field.len().to_string());
        output.push(':');
        output.push_str(field);
        output.push(':');
    }
    output
}

fn compile_diagnostic(
    code: &'static str,
    element_id: Option<&str>,
    step_id: Option<&str>,
    detail: impl Into<String>,
) -> CompileDiagnostic {
    CompileDiagnostic {
        code,
        element_id: element_id.map(str::to_owned),
        step_id: step_id.map(str::to_owned),
        node_id: None,
        detail: detail.into(),
    }
}

fn compile_diagnostic_order(
    left: &CompileDiagnostic,
    right: &CompileDiagnostic,
) -> std::cmp::Ordering {
    (
        left.code,
        left.element_id.as_deref().unwrap_or(""),
        left.step_id.as_deref().unwrap_or(""),
        left.node_id.as_deref().unwrap_or(""),
        left.detail.as_str(),
    )
        .cmp(&(
            right.code,
            right.element_id.as_deref().unwrap_or(""),
            right.step_id.as_deref().unwrap_or(""),
            right.node_id.as_deref().unwrap_or(""),
            right.detail.as_str(),
        ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest(elements: Vec<DesignElement>) -> DesignManifest {
        DesignManifest {
            project_id: "project-1".to_owned(),
            site_id: "site-1".to_owned(),
            work_package_id: "wp-1".to_owned(),
            model_digest: "sha256:model-1".to_owned(),
            design_revision: "design-rev-1".to_owned(),
            target_frame_id: "project-grid-1".to_owned(),
            site_epoch: "site-epoch-1".to_owned(),
            elements,
        }
    }

    fn property(name: &str, value: &str, unit: &str) -> DesignProperty {
        DesignProperty {
            name: name.to_owned(),
            value: value.to_owned(),
            unit: unit.to_owned(),
            source_ref: format!("ifc.element.property.{name}"),
        }
    }

    fn element(id: &str) -> DesignElement {
        DesignElement {
            element_id: id.to_owned(),
            task_family: "piling".to_owned(),
            properties: vec![property("embedment_depth", "1.2", "m")],
        }
    }

    fn step(id: &str, dependencies: &[&str]) -> MethodStep {
        MethodStep {
            step_id: id.to_owned(),
            functional_unit_class: format!("pile-{id}"),
            transformation_rule_id: format!("rule-{id}"),
            transformation_rule_digest: format!("sha256:rule-{id}"),
            dependencies: dependencies
                .iter()
                .map(|dependency| (*dependency).to_owned())
                .collect(),
            parameters: vec![ParameterMapping {
                target_name: "embedment_depth".to_owned(),
                source_property: "embedment_depth".to_owned(),
                target_unit: "m".to_owned(),
                conversion: None,
            }],
            required_capabilities: vec!["rotary-piling".to_owned()],
            required_resources: vec!["pile-basket".to_owned()],
            preconditions: vec!["work-zone-released".to_owned()],
            postconditions: vec![format!("{id}-complete")],
            required_evidence: vec!["independent-pile-inspection".to_owned()],
        }
    }

    fn method(steps: Vec<MethodStep>) -> MethodTemplate {
        MethodTemplate {
            template_id: "solar-pile-installation".to_owned(),
            revision_digest: "sha256:method-rev-1".to_owned(),
            approval_ref: "project-method-approval-1".to_owned(),
            supported_task_families: vec!["piling".to_owned()],
            steps,
        }
    }

    fn compile(manifest: &DesignManifest, method: &MethodTemplate) -> CompileResult {
        compile_work_graph(manifest, method, "compiler-0.1.0", "sha256:rules-1")
    }

    #[test]
    fn one_element_one_step_compiles_with_exact_lineage() {
        let result = compile(
            &manifest(vec![element("pile-1")]),
            &method(vec![step("install", &[])]),
        );
        assert_eq!(result.verdict, CompileVerdict::CompiledForReview);
        let graph = result.graph.expect("compiled graph");
        assert_eq!(graph.nodes.len(), 1);
        assert_eq!(graph.nodes[0].source_element_ids, vec!["pile-1"]);
        assert_eq!(graph.nodes[0].method_step_id, "install");
        assert_eq!(
            graph.nodes[0].transformation_rule_digest,
            "sha256:rule-install"
        );
    }

    #[test]
    fn task_family_and_resource_obligations_are_preserved() {
        let result = compile(
            &manifest(vec![element("pile-1")]),
            &method(vec![step("install", &[])]),
        );
        let graph = result.graph.expect("compiled graph");
        assert_eq!(graph.nodes[0].task_family, "piling");
        assert_eq!(graph.nodes[0].required_resources, vec!["pile-basket"]);
        assert_eq!(
            graph.nodes[0].parameters[0].source_property,
            "embedment_depth"
        );
    }

    #[test]
    fn multistep_template_becomes_ordered_same_element_dag() {
        let result = compile(
            &manifest(vec![element("pile-1")]),
            &method(vec![step("install", &["prepare"]), step("prepare", &[])]),
        );
        assert_eq!(result.verdict, CompileVerdict::CompiledForReview);
        assert_eq!(
            result.topological_order,
            vec![task_id("pile-1", "prepare"), task_id("pile-1", "install")]
        );
        let graph = result.graph.expect("compiled graph");
        let install = graph
            .nodes
            .iter()
            .find(|node| node.method_step_id == "install")
            .expect("install node");
        assert_eq!(install.dependencies, vec![task_id("pile-1", "prepare")]);
    }

    #[test]
    fn template_instantiation_preserves_distinct_element_identity() {
        let result = compile(
            &manifest(vec![element("pile-2"), element("pile-1")]),
            &method(vec![step("install", &[])]),
        );
        let graph = result.graph.expect("compiled graph");
        assert_eq!(graph.nodes.len(), 2);
        assert_ne!(graph.nodes[0].task_id, graph.nodes[1].task_id);
        assert_eq!(graph.nodes[0].task_id, task_id("pile-1", "install"));
        assert_eq!(graph.nodes[1].task_id, task_id("pile-2", "install"));
    }

    #[test]
    fn input_order_permutations_produce_identical_graph_and_topology() {
        let first = compile(
            &manifest(vec![element("pile-2"), element("pile-1")]),
            &method(vec![step("install", &["prepare"]), step("prepare", &[])]),
        );
        let second = compile(
            &manifest(vec![element("pile-1"), element("pile-2")]),
            &method(vec![step("prepare", &[]), step("install", &["prepare"])]),
        );
        assert_eq!(first, second);
    }

    #[test]
    fn missing_required_property_is_rejected_not_defaulted() {
        let mut input = element("pile-1");
        input.properties.clear();
        let result = compile(&manifest(vec![input]), &method(vec![step("install", &[])]));
        assert_eq!(result.verdict, CompileVerdict::RejectedInconsistent);
        assert!(result.graph.is_none());
        assert!(
            result
                .diagnostics
                .iter()
                .any(|item| item.code == "missing_design_property")
        );
    }

    #[test]
    fn unsupported_family_is_typed_rejection() {
        let mut input = element("wall-1");
        input.task_family = "masonry".to_owned();
        let result = compile(&manifest(vec![input]), &method(vec![step("install", &[])]));
        assert_eq!(result.verdict, CompileVerdict::RejectedUnsupported);
        assert!(
            result
                .diagnostics
                .iter()
                .any(|item| item.code == "unsupported_task_family")
        );
    }

    #[test]
    fn unit_change_requires_an_external_conversion_receipt() {
        let mut input = element("pile-1");
        input.properties = vec![property("embedment_depth", "1200", "mm")];
        let result = compile(&manifest(vec![input]), &method(vec![step("install", &[])]));
        assert_eq!(result.verdict, CompileVerdict::RejectedInconsistent);
        assert!(
            result
                .diagnostics
                .iter()
                .any(|item| item.code == "unit_conversion_receipt_required")
        );
    }

    #[test]
    fn explicit_unit_conversion_receipt_binds_the_converted_value() {
        let mut input = element("pile-1");
        input.properties = vec![property("embedment_depth", "1200", "mm")];
        let mut install = step("install", &[]);
        install.parameters[0].target_unit = "m".to_owned();
        install.parameters[0].conversion = Some(ApprovedConversion {
            converted_value: "1.2".to_owned(),
            evidence_ref: "conversion-receipt-1".to_owned(),
        });

        let result = compile(&manifest(vec![input]), &method(vec![install]));
        assert_eq!(result.verdict, CompileVerdict::CompiledForReview);
        let binding = &result.graph.expect("compiled graph").nodes[0].parameters[0];
        assert_eq!(binding.value, "1.2");
        assert_eq!(binding.source_unit.as_deref(), Some("mm"));
        assert_eq!(
            binding.conversion_evidence_ref.as_deref(),
            Some("conversion-receipt-1")
        );
    }

    #[test]
    fn missing_method_approval_reference_blocks_compilation() {
        let mut template = method(vec![step("install", &[])]);
        template.approval_ref.clear();
        let result = compile(&manifest(vec![element("pile-1")]), &template);
        assert_eq!(result.verdict, CompileVerdict::RejectedInconsistent);
        assert!(
            result
                .diagnostics
                .iter()
                .any(|item| item.code == "missing_compile_header")
        );
    }

    #[test]
    fn duplicate_design_element_ids_are_rejected() {
        let result = compile(
            &manifest(vec![element("pile-1"), element("pile-1")]),
            &method(vec![step("install", &[])]),
        );
        assert_eq!(result.verdict, CompileVerdict::RejectedInconsistent);
        assert!(
            result
                .diagnostics
                .iter()
                .any(|item| item.code == "duplicate_design_element_id")
        );
    }

    #[test]
    fn duplicate_method_step_ids_are_rejected() {
        let result = compile(
            &manifest(vec![element("pile-1")]),
            &method(vec![step("install", &[]), step("install", &[])]),
        );
        assert_eq!(result.verdict, CompileVerdict::RejectedInconsistent);
        assert!(
            result
                .diagnostics
                .iter()
                .any(|item| item.code == "duplicate_method_step_id")
        );
    }

    #[test]
    fn missing_method_step_dependency_is_rejected() {
        let result = compile(
            &manifest(vec![element("pile-1")]),
            &method(vec![step("install", &["missing-step"])]),
        );
        assert_eq!(result.verdict, CompileVerdict::RejectedInconsistent);
        assert!(
            result
                .diagnostics
                .iter()
                .any(|item| item.code == "unknown_method_step_dependency")
        );
    }

    #[test]
    fn cyclic_method_steps_are_rejected_by_graph_validator() {
        let result = compile(
            &manifest(vec![element("pile-1")]),
            &method(vec![
                step("install", &["prepare"]),
                step("prepare", &["install"]),
            ]),
        );
        assert_eq!(result.verdict, CompileVerdict::RejectedInconsistent);
        assert!(
            result
                .diagnostics
                .iter()
                .any(|item| item.code == "dependency_cycle")
        );
    }

    #[test]
    fn duplicated_source_property_is_ambiguous_not_first_match_wins() {
        let mut input = element("pile-1");
        input
            .properties
            .push(property("embedment_depth", "2.0", "m"));
        let result = compile(&manifest(vec![input]), &method(vec![step("install", &[])]));
        assert_eq!(result.verdict, CompileVerdict::RejectedInconsistent);
        assert!(
            result
                .diagnostics
                .iter()
                .any(|item| item.code == "ambiguous_design_property")
        );
    }

    #[test]
    fn duplicate_method_parameter_mapping_is_rejected() {
        let mut install = step("install", &[]);
        install.parameters.push(install.parameters[0].clone());
        let result = compile(&manifest(vec![element("pile-1")]), &method(vec![install]));
        assert_eq!(result.verdict, CompileVerdict::RejectedInconsistent);
        assert!(
            result
                .diagnostics
                .iter()
                .any(|item| item.code == "duplicate_parameter_mapping")
        );
    }

    #[test]
    fn source_unit_without_source_reference_is_rejected() {
        let mut input = element("pile-1");
        input.properties[0].source_ref.clear();
        let result = compile(&manifest(vec![input]), &method(vec![step("install", &[])]));
        assert_eq!(result.verdict, CompileVerdict::RejectedInconsistent);
        assert!(
            result
                .diagnostics
                .iter()
                .any(|item| item.code == "incomplete_design_property")
        );
    }

    #[test]
    fn empty_manifest_and_empty_template_do_not_emit_empty_success() {
        let result = compile(&manifest(Vec::new()), &method(Vec::new()));
        assert_eq!(result.verdict, CompileVerdict::RejectedInconsistent);
        assert!(result.graph.is_none());
    }

    #[test]
    fn site_epoch_changes_graph_identity() {
        let first = compile(
            &manifest(vec![element("pile-1")]),
            &method(vec![step("install", &[])]),
        );
        let mut second_manifest = manifest(vec![element("pile-1")]);
        second_manifest.site_epoch = "site-epoch-2".to_owned();
        let second = compile(&second_manifest, &method(vec![step("install", &[])]));
        assert_ne!(
            first.graph.expect("first graph").graph_id,
            second.graph.expect("second graph").graph_id
        );
    }

    #[test]
    fn target_frame_changes_graph_identity() {
        let first = compile(
            &manifest(vec![element("pile-1")]),
            &method(vec![step("install", &[])]),
        );
        let mut second_manifest = manifest(vec![element("pile-1")]);
        second_manifest.target_frame_id = "local-frame-2".to_owned();
        let second = compile(&second_manifest, &method(vec![step("install", &[])]));
        assert_ne!(
            first.graph.expect("first graph").graph_id,
            second.graph.expect("second graph").graph_id
        );
    }

    #[test]
    fn task_ids_are_unambiguous_for_delimiter_like_identifiers() {
        assert_ne!(task_id("a:b", "c"), task_id("a", "b:c"));
    }
}
