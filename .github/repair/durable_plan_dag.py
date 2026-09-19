from pathlib import Path
import re


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected one exact anchor, found {count}")
    return text.replace(old, new, 1)


plan = Path("crates/domains/symtropy-fabrication/src/plan.rs")
text = plan.read_text()

text = replace_once(
    text,
    "use serde::{Deserialize, Serialize};\n",
    "use serde::{Deserialize, Deserializer, Serialize};\n",
    "serde import",
)

text = replace_once(
    text,
    "#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]\npub struct PlanStep {\n",
    "#[derive(Debug, Clone, PartialEq, Eq, Serialize)]\npub struct PlanStep {\n",
    "PlanStep derive",
)

step_anchor = """}\n\nimpl PlanStep {\n    pub fn new(\n"""
step_block = """}

#[derive(Deserialize)]
struct PlanStepWire {
    id: PlanStepId,
    process_spec_id: ProcessSpecId,
    process_spec_revision: u64,
    workpieces: Vec<WorkpieceId>,
    capability_needs: Vec<CapabilityNeedId>,
    expected_evidence_kinds: Vec<StableId>,
}

impl<'de> Deserialize<'de> for PlanStep {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = PlanStepWire::deserialize(deserializer)?;
        Self::new(
            wire.id,
            wire.process_spec_id,
            wire.process_spec_revision,
            wire.workpieces,
            wire.capability_needs,
            wire.expected_evidence_kinds,
        )
        .map_err(serde::de::Error::custom)
    }
}

impl PlanStep {
    pub fn new(
"""
text = replace_once(text, step_anchor, step_block, "PlanStep wire block")

text = replace_once(
    text,
    "#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]\npub struct PlanDependency {\n",
    "#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize)]\npub struct PlanDependency {\n",
    "PlanDependency derive",
)

dep_anchor = """}\n\nimpl PlanDependency {\n    pub fn new(prerequisite: PlanStepId, dependent: PlanStepId) -> Result<Self, PlanError> {\n"""
dep_block = """}

#[derive(Deserialize)]
struct PlanDependencyWire {
    prerequisite: PlanStepId,
    dependent: PlanStepId,
}

impl<'de> Deserialize<'de> for PlanDependency {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = PlanDependencyWire::deserialize(deserializer)?;
        Self::new(wire.prerequisite, wire.dependent).map_err(serde::de::Error::custom)
    }
}

impl PlanDependency {
    pub fn new(prerequisite: PlanStepId, dependent: PlanStepId) -> Result<Self, PlanError> {
"""
text = replace_once(text, dep_anchor, dep_block, "PlanDependency wire block")

text = replace_once(
    text,
    """        Ok(Self {\n            prerequisite,\n            dependent,\n        })\n    }\n}\n\n/// Immutable reusable plan definition. Runtime progress belongs to a later\n""",
    """        Ok(Self {
            prerequisite,
            dependent,
        })
    }

    /// Replays the primitive dependency theorem for a publicly mutable value
    /// before it is admitted into a plan graph.
    pub fn validate_current(&self) -> Result<(), PlanError> {
        if self.prerequisite == self.dependent {
            return Err(PlanError::SelfDependency(self.prerequisite.clone()));
        }
        Ok(())
    }
}

/// Immutable reusable plan definition. Runtime progress belongs to a later
""",
    "PlanDependency validator",
)

text = replace_once(
    text,
    "#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]\npub struct FabricationPlan {\n",
    "#[derive(Debug, Clone, PartialEq, Eq, Serialize)]\npub struct FabricationPlan {\n",
    "FabricationPlan derive",
)

plan_anchor = """}\n\nimpl FabricationPlan {\n    pub fn new(\n"""
plan_block = """}

#[derive(Deserialize)]
struct FabricationPlanWire {
    id: FabricationPlanId,
    revision: u64,
    steps: Vec<PlanStep>,
    dependencies: Vec<PlanDependency>,
}

impl<'de> Deserialize<'de> for FabricationPlan {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = FabricationPlanWire::deserialize(deserializer)?;
        Self::new(wire.id, wire.revision, wire.steps, wire.dependencies)
            .map_err(serde::de::Error::custom)
    }
}

impl FabricationPlan {
    pub fn new(
"""
text = replace_once(text, plan_anchor, plan_block, "FabricationPlan wire block")

text = replace_once(
    text,
    """        dependencies.sort();\n        for pair in dependencies.windows(2) {\n""",
    """        for dependency in &dependencies {
            dependency.validate_current()?;
        }
        dependencies.sort();
        for pair in dependencies.windows(2) {
""",
    "dependency primitive replay",
)

test_anchor = """    #[test]\n    fn plan_serialization_contains_no_runtime_or_authority_claims() {\n"""
tests = r'''    #[test]
    fn plan_step_wire_normalizes_set_like_order() {
        let step = PlanStep::new(
            PlanStepId::new(id("step:wire-order")),
            ProcessSpecId::new(id("process-spec:wire-order")),
            4,
            vec![
                WorkpieceId::new(id("workpiece:a")),
                WorkpieceId::new(id("workpiece:b")),
            ],
            vec![
                CapabilityNeedId::new(id("capability-need:a")),
                CapabilityNeedId::new(id("capability-need:b")),
            ],
            vec![id("evidence-kind:a"), id("evidence-kind:b")],
        )
        .unwrap();
        let mut value = serde_json::to_value(&step).unwrap();
        value["workpieces"].as_array_mut().unwrap().reverse();
        value["capability_needs"].as_array_mut().unwrap().reverse();
        value["expected_evidence_kinds"]
            .as_array_mut()
            .unwrap()
            .reverse();

        let restored: PlanStep = serde_json::from_value(value).unwrap();
        assert_eq!(restored, step);
    }

    #[test]
    fn plan_step_wire_rejects_duplicate_or_missing_set_members() {
        let step = PlanStep::new(
            PlanStepId::new(id("step:wire-invalid")),
            ProcessSpecId::new(id("process-spec:wire-invalid")),
            1,
            vec![WorkpieceId::new(id("workpiece:a"))],
            vec![CapabilityNeedId::new(id("capability-need:a"))],
            vec![id("evidence-kind:a")],
        )
        .unwrap();

        let mut duplicate_workpiece = serde_json::to_value(&step).unwrap();
        let duplicate = duplicate_workpiece["workpieces"][0].clone();
        duplicate_workpiece["workpieces"]
            .as_array_mut()
            .unwrap()
            .push(duplicate);
        assert!(serde_json::from_value::<PlanStep>(duplicate_workpiece).is_err());

        let mut duplicate_capability = serde_json::to_value(&step).unwrap();
        let duplicate = duplicate_capability["capability_needs"][0].clone();
        duplicate_capability["capability_needs"]
            .as_array_mut()
            .unwrap()
            .push(duplicate);
        assert!(serde_json::from_value::<PlanStep>(duplicate_capability).is_err());

        let mut duplicate_evidence = serde_json::to_value(&step).unwrap();
        let duplicate = duplicate_evidence["expected_evidence_kinds"][0].clone();
        duplicate_evidence["expected_evidence_kinds"]
            .as_array_mut()
            .unwrap()
            .push(duplicate);
        assert!(serde_json::from_value::<PlanStep>(duplicate_evidence).is_err());

        let mut empty_workpieces = serde_json::to_value(&step).unwrap();
        empty_workpieces["workpieces"] = serde_json::Value::Array(Vec::new());
        assert!(serde_json::from_value::<PlanStep>(empty_workpieces).is_err());
    }

    #[test]
    fn plan_step_canonical_round_trip_succeeds() {
        let original = step("round-trip", "workpiece:round-trip");
        let encoded = serde_json::to_vec(&original).unwrap();
        let restored: PlanStep = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(restored, original);
    }

    #[test]
    fn dependency_wire_and_public_mutation_reject_self_edges() {
        let original = dependency("clean", "inspect");
        let mut value = serde_json::to_value(&original).unwrap();
        value["dependent"] = value["prerequisite"].clone();
        assert!(serde_json::from_value::<PlanDependency>(value).is_err());

        let mut mutated = original;
        mutated.dependent = mutated.prerequisite.clone();
        let result = FabricationPlan::new(
            FabricationPlanId::new(id("fabrication-plan:mutated-dependency")),
            1,
            vec![step("clean", "workpiece:clean")],
            vec![mutated],
        );
        assert!(matches!(result, Err(PlanError::SelfDependency(_))));
    }

    #[test]
    fn plan_wire_normalizes_valid_step_and_dependency_order() {
        let original = patch_plan(
            vec![
                step("clean", "workpiece:clean"),
                step("seal", "workpiece:seal"),
                step("inspect", "workpiece:inspect"),
            ],
            vec![dependency("clean", "seal"), dependency("seal", "inspect")],
        );
        let expected_order = original.topological_order();
        let mut value = serde_json::to_value(&original).unwrap();
        value["steps"].as_array_mut().unwrap().reverse();
        value["dependencies"].as_array_mut().unwrap().reverse();

        let restored: FabricationPlan = serde_json::from_value(value).unwrap();
        assert_eq!(restored, original);
        assert_eq!(restored.topological_order(), expected_order);
    }

    #[test]
    fn plan_wire_rejects_duplicate_step_and_dependency() {
        let original = patch_plan(
            vec![
                step("clean", "workpiece:clean"),
                step("inspect", "workpiece:inspect"),
            ],
            vec![dependency("clean", "inspect")],
        );

        let mut duplicate_step = serde_json::to_value(&original).unwrap();
        let duplicate = duplicate_step["steps"][0].clone();
        duplicate_step["steps"].as_array_mut().unwrap().push(duplicate);
        assert!(serde_json::from_value::<FabricationPlan>(duplicate_step).is_err());

        let mut duplicate_dependency = serde_json::to_value(&original).unwrap();
        let duplicate = duplicate_dependency["dependencies"][0].clone();
        duplicate_dependency["dependencies"]
            .as_array_mut()
            .unwrap()
            .push(duplicate);
        assert!(serde_json::from_value::<FabricationPlan>(duplicate_dependency).is_err());
    }

    #[test]
    fn plan_wire_rejects_unknown_dependency_endpoints() {
        let original = patch_plan(
            vec![
                step("clean", "workpiece:clean"),
                step("inspect", "workpiece:inspect"),
            ],
            vec![dependency("clean", "inspect")],
        );

        let mut unknown_prerequisite = serde_json::to_value(&original).unwrap();
        unknown_prerequisite["dependencies"][0]["prerequisite"] =
            serde_json::Value::String("step:missing-prerequisite".into());
        assert!(serde_json::from_value::<FabricationPlan>(unknown_prerequisite).is_err());

        let mut unknown_dependent = serde_json::to_value(&original).unwrap();
        unknown_dependent["dependencies"][0]["dependent"] =
            serde_json::Value::String("step:missing-dependent".into());
        assert!(serde_json::from_value::<FabricationPlan>(unknown_dependent).is_err());
    }

    #[test]
    fn plan_wire_rejects_simple_and_multi_node_cycles() {
        let simple = patch_plan(
            vec![
                step("a", "workpiece:a"),
                step("b", "workpiece:b"),
            ],
            vec![dependency("a", "b")],
        );
        let mut simple_value = serde_json::to_value(&simple).unwrap();
        simple_value["dependencies"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({
                "prerequisite": "step:b",
                "dependent": "step:a"
            }));
        assert!(serde_json::from_value::<FabricationPlan>(simple_value).is_err());

        let multi = patch_plan(
            vec![
                step("a", "workpiece:a"),
                step("b", "workpiece:b"),
                step("c", "workpiece:c"),
            ],
            vec![dependency("a", "b"), dependency("b", "c")],
        );
        let mut multi_value = serde_json::to_value(&multi).unwrap();
        multi_value["dependencies"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({
                "prerequisite": "step:c",
                "dependent": "step:a"
            }));
        assert!(serde_json::from_value::<FabricationPlan>(multi_value).is_err());
    }

    #[test]
    fn fabrication_plan_canonical_round_trip_succeeds() {
        let original = patch_plan(
            vec![
                step("clean", "workpiece:clean"),
                step("inspect", "workpiece:inspect"),
            ],
            vec![dependency("clean", "inspect")],
        );
        let expected_order = original.topological_order();
        let encoded = serde_json::to_vec(&original).unwrap();
        let restored: FabricationPlan = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(restored, original);
        assert_eq!(restored.topological_order(), expected_order);
    }

'''
text = replace_once(text, test_anchor, tests + test_anchor, "plan durable tests")
plan.write_text(text)


executable = Path("crates/domains/symtropy-fabrication/src/executable_plan.rs")
text = executable.read_text()
exec_anchor = """    #[test]\n    fn deserialization_revalidates_executable_plan_contract() {\n"""
exec_tests = r'''    #[test]
    fn deserialization_rejects_nested_cyclic_f10_plan() {
        let executable = ExecutableFabricationPlan::new(plan(), bindings()).unwrap();
        let mut value = serde_json::to_value(&executable).unwrap();
        value["plan"]["dependencies"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({
                "prerequisite": "step:inspect",
                "dependent": "step:clean"
            }));
        assert!(serde_json::from_value::<ExecutableFabricationPlan>(value).is_err());
    }

    #[test]
    fn deserialization_rejects_nested_f10_unknown_dependency_step() {
        let executable = ExecutableFabricationPlan::new(plan(), bindings()).unwrap();
        let mut value = serde_json::to_value(&executable).unwrap();
        value["plan"]["dependencies"][0]["prerequisite"] =
            serde_json::Value::String("step:unknown".into());
        assert!(serde_json::from_value::<ExecutableFabricationPlan>(value).is_err());
    }

'''
text = replace_once(text, exec_anchor, exec_tests + exec_anchor, "executable nested F10 tests")
executable.write_text(text)
