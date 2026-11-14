from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected one exact anchor, found {count}")
    return text.replace(old, new, 1)


path = Path("crates/domains/symtropy-fabrication/src/executable_plan.rs")
text = path.read_text()

text = replace_once(
    text,
    """            if expected_capabilities != actual_capabilities {\n                return Err(ExecutablePlanError::CapabilityBindingMismatch {\n                    step_id: binding.step_id.clone(),\n                    expected: expected_capabilities,\n                    actual: actual_capabilities,\n                });\n            }\n        }\n\n        for step in plan.steps() {\n""",
    """            if expected_capabilities != actual_capabilities {
                return Err(ExecutablePlanError::CapabilityBindingMismatch {
                    step_id: binding.step_id.clone(),
                    expected: expected_capabilities,
                    actual: actual_capabilities,
                });
            }

            // F5 deliberately projects a rich satisfied admission into the F4
            // compatibility token `available_value = 1`. Once F10 and F4 agree
            // that these capability IDs are F5 need identities, any other F4
            // threshold would create a second scalar capability semantics that
            // the rich F5 theorem can never prove.
            for requirement in binding.process_spec.required_capabilities() {
                if requirement.minimum_value != 1 {
                    return Err(ExecutablePlanError::NonBinaryF5BootstrapRequirement {
                        step_id: binding.step_id.clone(),
                        need_id: CapabilityNeedId::new(requirement.capability_id.clone()),
                        minimum_value: requirement.minimum_value,
                    });
                }
            }
        }

        for step in plan.steps() {
""",
    "binary F4/F5 bridge check",
)

text = replace_once(
    text,
    """    CapabilityBindingMismatch {\n        step_id: PlanStepId,\n        expected: Vec<StableId>,\n        actual: Vec<StableId>,\n    },\n    MissingCapabilityNeedBinding(CapabilityNeedId),\n""",
    """    CapabilityBindingMismatch {
        step_id: PlanStepId,
        expected: Vec<StableId>,
        actual: Vec<StableId>,
    },
    NonBinaryF5BootstrapRequirement {
        step_id: PlanStepId,
        need_id: CapabilityNeedId,
        minimum_value: u64,
    },
    MissingCapabilityNeedBinding(CapabilityNeedId),
""",
    "binary bridge error variant",
)

text = replace_once(
    text,
    """            Self::CapabilityBindingMismatch {\n                step_id,\n                expected,\n                actual,\n            } => write!(\n                formatter,\n                \"plan step {step_id} capability bindings differ from exact F4 bootstrap requirements: expected {expected:?}, got {actual:?}\"\n            ),\n            Self::MissingCapabilityNeedBinding(need_id) => write!(\n""",
    """            Self::CapabilityBindingMismatch {
                step_id,
                expected,
                actual,
            } => write!(
                formatter,
                "plan step {step_id} capability bindings differ from exact F4 bootstrap requirements: expected {expected:?}, got {actual:?}"
            ),
            Self::NonBinaryF5BootstrapRequirement {
                step_id,
                need_id,
                minimum_value,
            } => write!(
                formatter,
                "plan step {step_id} maps F5 need {need_id} to non-binary F4 threshold {minimum_value}; exact F5 bootstrap requires 1"
            ),
            Self::MissingCapabilityNeedBinding(need_id) => write!(
""",
    "binary bridge error display",
)

anchor = """    #[test]\n    fn capability_bearing_plan_requires_exact_f5_need_binding() {\n"""
test = r'''    #[test]
    fn f5_backed_process_requirement_must_remain_binary_at_f4_bridge() {
        let mut process_bindings = bindings();
        process_bindings[0].process_spec = ProcessSpec::new(
            process_id("clean"),
            1,
            ProcessKind::Clean,
            vec![CapabilityRequirement {
                capability_id: capability_id("clean").stable_id().clone(),
                minimum_value: 2,
            }],
            vec![WorkpieceLifecycle::Available],
        )
        .unwrap()
        .snapshot();

        let result = ExecutableFabricationPlan::new_with_capability_needs(
            plan(),
            process_bindings,
            capability_needs(),
        );
        assert!(matches!(
            result,
            Err(ExecutablePlanError::NonBinaryF5BootstrapRequirement {
                need_id,
                minimum_value: 2,
                ..
            }) if need_id == capability_id("clean")
        ));
    }

'''
text = replace_once(text, anchor, test + anchor, "binary bridge regression")
path.write_text(text)
