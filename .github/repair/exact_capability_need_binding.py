from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected one exact anchor, found {count}")
    return text.replace(old, new, 1)


# ---------------------------------------------------------------------------
# F5 reusable need canonicalization + exact immutable need snapshots.
# ---------------------------------------------------------------------------
capability = Path("crates/domains/symtropy-fabrication/src/capability.rs")
text = capability.read_text()

text = replace_once(
    text,
    "use serde::{Deserialize, Serialize};\n",
    "use serde::{Deserialize, Deserializer, Serialize};\n",
    "capability serde import",
)

axis_need_tail = """        Ok(Self {\n            axis_id,\n            lower,\n            upper,\n            max_resolution,\n        })\n    }\n}\n\n/// Current operating envelope of a concrete tool, machine, operator, fixture,\n"""
axis_need_replacement = """        Ok(Self {
            axis_id,
            lower,
            upper,
            max_resolution,
        })
    }

    /// Replays the primitive range/resolution theorem for a value that may have
    /// been publicly mutated before it reaches a consequential boundary.
    pub fn validate_current(&self) -> Result<(), CapabilityError> {
        if self.lower > self.upper {
            return Err(CapabilityError::InvalidAxisNeed {
                axis_id: self.axis_id.clone(),
                lower: self.lower,
                upper: self.upper,
            });
        }
        if self.max_resolution == Some(0) {
            return Err(CapabilityError::ZeroMaximumResolution(
                self.axis_id.clone(),
            ));
        }
        Ok(())
    }
}

/// Current operating envelope of a concrete tool, machine, operator, fixture,
"""
text = replace_once(text, axis_need_tail, axis_need_replacement, "axis-need validator")

text = replace_once(
    text,
    "#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]\npub struct CapabilityNeed {\n",
    "#[derive(Debug, Clone, PartialEq, Eq, Serialize)]\npub struct CapabilityNeed {\n",
    "CapabilityNeed derive",
)

need_impl_anchor = """}\n\nimpl CapabilityNeed {\n    pub fn new(\n"""
need_wire_block = """}

#[derive(Deserialize)]
struct CapabilityNeedWire {
    id: CapabilityNeedId,
    capability_id: StableId,
    required_mode_id: Option<StableId>,
    axes: Vec<CapabilityAxisNeed>,
    required_conditions: Vec<StableId>,
}

impl<'de> Deserialize<'de> for CapabilityNeed {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = CapabilityNeedWire::deserialize(deserializer)?;
        Self::new(
            wire.id,
            wire.capability_id,
            wire.required_mode_id,
            wire.axes,
            wire.required_conditions,
        )
        .map_err(serde::de::Error::custom)
    }
}

impl CapabilityNeed {
    pub fn new(
"""
text = replace_once(text, need_impl_anchor, need_wire_block, "CapabilityNeed wire block")

old_need_constructor = """        id: CapabilityNeedId,\n        capability_id: StableId,\n        required_mode_id: Option<StableId>,\n        axes: Vec<CapabilityAxisNeed>,\n        required_conditions: Vec<StableId>,\n    ) -> Result<Self, CapabilityError> {\n        reject_duplicate_axis_needs(&axes)?;\n        reject_duplicate_ids(\n            &required_conditions,\n            CapabilityError::DuplicateRequiredCondition,\n        )?;\n        Ok(Self {\n            id,\n            capability_id,\n            required_mode_id,\n            axes,\n            required_conditions,\n        })\n    }\n\n    /// Evaluates containment and categorical predicates without inventing a\n"""
new_need_constructor = """        id: CapabilityNeedId,
        capability_id: StableId,
        required_mode_id: Option<StableId>,
        mut axes: Vec<CapabilityAxisNeed>,
        mut required_conditions: Vec<StableId>,
    ) -> Result<Self, CapabilityError> {
        for axis in &axes {
            axis.validate_current()?;
        }
        axes.sort_by(|left, right| left.axis_id.cmp(&right.axis_id));
        reject_duplicate_axis_needs(&axes)?;

        required_conditions.sort();
        reject_duplicate_ids(
            &required_conditions,
            CapabilityError::DuplicateRequiredCondition,
        )?;

        Ok(Self {
            id,
            capability_id,
            required_mode_id,
            axes,
            required_conditions,
        })
    }

    /// Captures exact canonical requirement semantics for an executable plan.
    /// Reusable knowledge may normalize benign ordering here; malformed public
    /// mutation still fails closed.
    pub fn snapshot(&self) -> Result<CapabilityNeedSnapshot, CapabilityError> {
        let canonical = Self::new(
            self.id.clone(),
            self.capability_id.clone(),
            self.required_mode_id.clone(),
            self.axes.clone(),
            self.required_conditions.clone(),
        )?;
        Ok(CapabilityNeedSnapshot::from_canonical(canonical))
    }

    /// Evaluates containment and categorical predicates without inventing a
"""
text = replace_once(text, old_need_constructor, new_need_constructor, "CapabilityNeed canonical constructor")

snapshot_anchor = """}\n\n#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]\n#[serde(rename_all = \"snake_case\")]\npub enum CapabilityOutcome {\n"""
snapshot_block = """}

/// Exact canonical F5 requirement semantics captured by executable authority.
///
/// Unlike reusable [`CapabilityNeed`] knowledge, this historical/executable
/// value never silently normalizes wire representation. It is immutable outside
/// F5 and rejects noncanonical ordering on restore.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CapabilityNeedSnapshot {
    id: CapabilityNeedId,
    capability_id: StableId,
    required_mode_id: Option<StableId>,
    axes: Vec<CapabilityAxisNeed>,
    required_conditions: Vec<StableId>,
}

#[derive(Deserialize)]
struct CapabilityNeedSnapshotWire {
    id: CapabilityNeedId,
    capability_id: StableId,
    required_mode_id: Option<StableId>,
    axes: Vec<CapabilityAxisNeed>,
    required_conditions: Vec<StableId>,
}

impl<'de> Deserialize<'de> for CapabilityNeedSnapshot {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = CapabilityNeedSnapshotWire::deserialize(deserializer)?;
        let value = Self {
            id: wire.id,
            capability_id: wire.capability_id,
            required_mode_id: wire.required_mode_id,
            axes: wire.axes,
            required_conditions: wire.required_conditions,
        };
        value
            .validate_canonical()
            .map_err(serde::de::Error::custom)?;
        Ok(value)
    }
}

impl CapabilityNeedSnapshot {
    fn from_canonical(need: CapabilityNeed) -> Self {
        Self {
            id: need.id,
            capability_id: need.capability_id,
            required_mode_id: need.required_mode_id,
            axes: need.axes,
            required_conditions: need.required_conditions,
        }
    }

    pub fn id(&self) -> &CapabilityNeedId {
        &self.id
    }

    pub fn capability_id(&self) -> &StableId {
        &self.capability_id
    }

    pub fn required_mode_id(&self) -> Option<&StableId> {
        self.required_mode_id.as_ref()
    }

    pub fn axes(&self) -> &[CapabilityAxisNeed] {
        &self.axes
    }

    pub fn required_conditions(&self) -> &[StableId] {
        &self.required_conditions
    }

    pub fn validate_canonical(&self) -> Result<(), CapabilityError> {
        for axis in &self.axes {
            axis.validate_current()?;
        }
        for pair in self.axes.windows(2) {
            match pair[0].axis_id.cmp(&pair[1].axis_id) {
                std::cmp::Ordering::Less => {}
                std::cmp::Ordering::Equal => {
                    return Err(CapabilityError::DuplicateAxisNeed(
                        pair[0].axis_id.clone(),
                    ));
                }
                std::cmp::Ordering::Greater => {
                    return Err(CapabilityError::NonCanonicalNeedAxisOrder);
                }
            }
        }
        for pair in self.required_conditions.windows(2) {
            match pair[0].cmp(&pair[1]) {
                std::cmp::Ordering::Less => {}
                std::cmp::Ordering::Equal => {
                    return Err(CapabilityError::DuplicateRequiredCondition(
                        pair[0].clone(),
                    ));
                }
                std::cmp::Ordering::Greater => {
                    return Err(CapabilityError::NonCanonicalRequiredConditionOrder);
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityOutcome {
"""
text = replace_once(text, snapshot_anchor, snapshot_block, "CapabilityNeedSnapshot block")

text = replace_once(
    text,
    """    DuplicateAxisNeed(StableId),\n    DuplicateCondition(StableId),\n    DuplicateRequiredCondition(StableId),\n    UnsatisfiedAdmission(CapabilityAdmissionId),\n""",
    """    DuplicateAxisNeed(StableId),
    NonCanonicalNeedAxisOrder,
    DuplicateCondition(StableId),
    DuplicateRequiredCondition(StableId),
    NonCanonicalRequiredConditionOrder,
    UnsatisfiedAdmission(CapabilityAdmissionId),
""",
    "CapabilityError variants",
)

text = replace_once(
    text,
    """            Self::DuplicateAxisNeed(axis_id) => {\n                write!(formatter, \"capability need repeats axis {axis_id}\")\n            }\n            Self::DuplicateCondition(condition) => {\n""",
    """            Self::DuplicateAxisNeed(axis_id) => {
                write!(formatter, "capability need repeats axis {axis_id}")
            }
            Self::NonCanonicalNeedAxisOrder => write!(
                formatter,
                "exact capability-need snapshot axes are not in canonical axis-id order"
            ),
            Self::DuplicateCondition(condition) => {
""",
    "axis-order display",
)

text = replace_once(
    text,
    """            Self::DuplicateRequiredCondition(condition) => {\n                write!(formatter, \"capability need repeats condition {condition}\")\n            }\n            Self::UnsatisfiedAdmission(id) => {\n""",
    """            Self::DuplicateRequiredCondition(condition) => {
                write!(formatter, "capability need repeats condition {condition}")
            }
            Self::NonCanonicalRequiredConditionOrder => write!(
                formatter,
                "exact capability-need snapshot conditions are not in canonical stable-id order"
            ),
            Self::UnsatisfiedAdmission(id) => {
""",
    "condition-order display",
)

capability_test_anchor = """    #[test]\n    fn serialized_envelope_has_no_level_quality_or_score_field() {\n"""
capability_tests = r'''    #[test]
    fn reusable_need_wire_normalizes_set_like_order() {
        let expected = welding_need();
        let mut value = serde_json::to_value(&expected).unwrap();
        value["axes"].as_array_mut().unwrap().reverse();
        value["required_conditions"].as_array_mut().unwrap().reverse();

        let restored: CapabilityNeed = serde_json::from_value(value).unwrap();
        assert_eq!(restored, expected);
    }

    #[test]
    fn exact_need_snapshot_rejects_noncanonical_wire_order() {
        let snapshot = welding_need().snapshot().unwrap();
        let mut axis_order = serde_json::to_value(&snapshot).unwrap();
        axis_order["axes"].as_array_mut().unwrap().reverse();
        assert!(serde_json::from_value::<CapabilityNeedSnapshot>(axis_order).is_err());

        let mut condition_order = serde_json::to_value(&snapshot).unwrap();
        condition_order["required_conditions"]
            .as_array_mut()
            .unwrap()
            .reverse();
        assert!(serde_json::from_value::<CapabilityNeedSnapshot>(condition_order).is_err());
    }

    #[test]
    fn exact_need_snapshot_revalidates_publicly_mutated_axis_semantics() {
        let mut need = welding_need();
        need.axes[0].max_resolution = Some(0);
        assert!(matches!(
            need.snapshot(),
            Err(CapabilityError::ZeroMaximumResolution(_))
        ));
    }

    #[test]
    fn same_need_id_with_changed_rich_semantics_has_distinct_snapshot() {
        let original = welding_need().snapshot().unwrap();
        let mut altered = welding_need();
        altered
            .required_conditions
            .retain(|condition| condition != &id("condition:shielding-active"));
        let altered = altered.snapshot().unwrap();

        assert_eq!(original.id(), altered.id());
        assert_ne!(original, altered);
    }

    #[test]
    fn exact_need_snapshot_round_trip_preserves_canonical_semantics() {
        let snapshot = welding_need().snapshot().unwrap();
        let encoded = serde_json::to_vec(&snapshot).unwrap();
        let restored: CapabilityNeedSnapshot = serde_json::from_slice(&encoded).unwrap();
        assert_eq!(restored, snapshot);
    }

'''
text = replace_once(
    text,
    capability_test_anchor,
    capability_tests + capability_test_anchor,
    "capability snapshot tests",
)
capability.write_text(text)


# ---------------------------------------------------------------------------
# Exact executable plan owns complete F5 requirement snapshots.
# ---------------------------------------------------------------------------
executable = Path("crates/domains/symtropy-fabrication/src/executable_plan.rs")
text = executable.read_text()

text = replace_once(
    text,
    "use std::{error::Error, fmt};\n",
    "use std::{collections::BTreeSet, error::Error, fmt};\n",
    "executable std import",
)

text = replace_once(
    text,
    """use crate::{\n    FabricationPlan, PlanStepId, ProcessSpecId, ProcessSpecSnapshot, ProcessSpecSnapshotError,\n};\n""",
    """use crate::{
    CapabilityError, CapabilityNeedId, CapabilityNeedSnapshot, FabricationPlan, PlanStepId,
    ProcessSpecId, ProcessSpecSnapshot, ProcessSpecSnapshotError,
};
""",
    "executable crate imports",
)

text = replace_once(
    text,
    """pub struct ExecutableFabricationPlan {\n    plan: FabricationPlan,\n    process_bindings: Vec<ExactPlanProcessBinding>,\n}\n\nimpl ExecutableFabricationPlan {\n    pub fn new(\n        plan: FabricationPlan,\n        mut process_bindings: Vec<ExactPlanProcessBinding>,\n    ) -> Result<Self, ExecutablePlanError> {\n""",
    """pub struct ExecutableFabricationPlan {
    plan: FabricationPlan,
    process_bindings: Vec<ExactPlanProcessBinding>,
    capability_needs: Vec<CapabilityNeedSnapshot>,
}

impl ExecutableFabricationPlan {
    /// Convenience constructor for executable plans that require no F5
    /// capability needs. Capability-bearing plans fail closed here and must use
    /// [`Self::new_with_capability_needs`].
    pub fn new(
        plan: FabricationPlan,
        process_bindings: Vec<ExactPlanProcessBinding>,
    ) -> Result<Self, ExecutablePlanError> {
        Self::new_with_capability_needs(plan, process_bindings, Vec::new())
    }

    pub fn new_with_capability_needs(
        plan: FabricationPlan,
        mut process_bindings: Vec<ExactPlanProcessBinding>,
        mut capability_needs: Vec<CapabilityNeedSnapshot>,
    ) -> Result<Self, ExecutablePlanError> {
""",
    "executable constructor header",
)

old_ok = """        Ok(Self {\n            plan,\n            process_bindings,\n        })\n    }\n\n    pub fn plan(&self) -> &FabricationPlan {\n"""
new_ok = """        capability_needs.sort_by(|left, right| left.id().cmp(right.id()));
        for need in &capability_needs {
            need.validate_canonical().map_err(|error| {
                ExecutablePlanError::NonCanonicalCapabilityNeed {
                    need_id: need.id().clone(),
                    error,
                }
            })?;
        }
        for pair in capability_needs.windows(2) {
            if pair[0].id() == pair[1].id() {
                return Err(ExecutablePlanError::DuplicateCapabilityNeedBinding(
                    pair[0].id().clone(),
                ));
            }
        }

        let expected_capability_needs = plan
            .steps()
            .iter()
            .flat_map(|step| step.capability_needs().iter().cloned())
            .collect::<BTreeSet<_>>();
        for need_id in &expected_capability_needs {
            if capability_needs
                .binary_search_by(|need| need.id().cmp(need_id))
                .is_err()
            {
                return Err(ExecutablePlanError::MissingCapabilityNeedBinding(
                    need_id.clone(),
                ));
            }
        }
        for need in &capability_needs {
            if !expected_capability_needs.contains(need.id()) {
                return Err(ExecutablePlanError::UnexpectedCapabilityNeedBinding(
                    need.id().clone(),
                ));
            }
        }

        Ok(Self {
            plan,
            process_bindings,
            capability_needs,
        })
    }

    pub fn plan(&self) -> &FabricationPlan {
"""
text = replace_once(text, old_ok, new_ok, "executable capability coverage")

accessor_anchor = """    pub fn process_bindings(&self) -> &[ExactPlanProcessBinding] {\n        &self.process_bindings\n    }\n\n    pub fn binding(&self, step_id: &PlanStepId) -> Option<&ExactPlanProcessBinding> {\n"""
accessor_replacement = """    pub fn process_bindings(&self) -> &[ExactPlanProcessBinding] {
        &self.process_bindings
    }

    pub fn capability_needs(&self) -> &[CapabilityNeedSnapshot] {
        &self.capability_needs
    }

    pub fn capability_need(&self, id: &CapabilityNeedId) -> Option<&CapabilityNeedSnapshot> {
        self.capability_needs
            .binary_search_by(|need| need.id().cmp(id))
            .ok()
            .map(|index| &self.capability_needs[index])
    }

    pub fn binding(&self, step_id: &PlanStepId) -> Option<&ExactPlanProcessBinding> {
"""
text = replace_once(text, accessor_anchor, accessor_replacement, "capability accessors")

text = replace_once(
    text,
    """struct ExecutableFabricationPlanWire {\n    plan: FabricationPlan,\n    process_bindings: Vec<ExactPlanProcessBinding>,\n}\n""",
    """struct ExecutableFabricationPlanWire {
    plan: FabricationPlan,
    process_bindings: Vec<ExactPlanProcessBinding>,
    capability_needs: Vec<CapabilityNeedSnapshot>,
}
""",
    "executable wire fields",
)

text = replace_once(
    text,
    """        let wire = ExecutableFabricationPlanWire::deserialize(deserializer)?;\n        Self::new(wire.plan, wire.process_bindings).map_err(serde::de::Error::custom)\n""",
    """        let wire = ExecutableFabricationPlanWire::deserialize(deserializer)?;
        Self::new_with_capability_needs(wire.plan, wire.process_bindings, wire.capability_needs)
            .map_err(serde::de::Error::custom)
""",
    "executable wire reconstruction",
)

text = replace_once(
    text,
    """    CapabilityBindingMismatch {\n        step_id: PlanStepId,\n        expected: Vec<StableId>,\n        actual: Vec<StableId>,\n    },\n}\n""",
    """    CapabilityBindingMismatch {
        step_id: PlanStepId,
        expected: Vec<StableId>,
        actual: Vec<StableId>,
    },
    MissingCapabilityNeedBinding(CapabilityNeedId),
    DuplicateCapabilityNeedBinding(CapabilityNeedId),
    UnexpectedCapabilityNeedBinding(CapabilityNeedId),
    NonCanonicalCapabilityNeed {
        need_id: CapabilityNeedId,
        error: CapabilityError,
    },
}
""",
    "executable error variants",
)

text = replace_once(
    text,
    """            Self::CapabilityBindingMismatch {\n                step_id,\n                expected,\n                actual,\n            } => write!(\n                formatter,\n                \"plan step {step_id} capability bindings differ from exact F4 bootstrap requirements: expected {expected:?}, got {actual:?}\"\n            ),\n        }\n""",
    """            Self::CapabilityBindingMismatch {
                step_id,
                expected,
                actual,
            } => write!(
                formatter,
                "plan step {step_id} capability bindings differ from exact F4 bootstrap requirements: expected {expected:?}, got {actual:?}"
            ),
            Self::MissingCapabilityNeedBinding(need_id) => write!(
                formatter,
                "executable plan lacks exact F5 semantics for required capability need {need_id}"
            ),
            Self::DuplicateCapabilityNeedBinding(need_id) => write!(
                formatter,
                "executable plan repeats exact F5 semantics for capability need {need_id}"
            ),
            Self::UnexpectedCapabilityNeedBinding(need_id) => write!(
                formatter,
                "executable plan carries unreferenced exact F5 capability need {need_id}"
            ),
            Self::NonCanonicalCapabilityNeed { need_id, error } => write!(
                formatter,
                "executable plan capability need {need_id} is not canonical: {error}"
            ),
        }
""",
    "executable error display",
)

text = replace_once(
    text,
    """impl Error for ExecutablePlanError {}\n""",
    """impl Error for ExecutablePlanError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::NonCanonicalProcessSpec { error, .. } => Some(error),
            Self::NonCanonicalCapabilityNeed { error, .. } => Some(error),
            _ => None,
        }
    }
}
""",
    "executable error source",
)

text = replace_once(
    text,
    """        CapabilityNeedId, CapabilityRequirement, FabricationPlanId, PlanDependency, PlanStep,\n        ProcessKind, ProcessSpec, WorkpieceId, WorkpieceLifecycle,\n""",
    """        CapabilityNeed, CapabilityNeedId, CapabilityNeedSnapshot, CapabilityRequirement,
        FabricationPlanId, PlanDependency, PlanStep, ProcessKind, ProcessSpec, WorkpieceId,
        WorkpieceLifecycle,
""",
    "executable test imports",
)

bindings_tail = """    fn bindings() -> Vec<ExactPlanProcessBinding> {\n        vec![\n            ExactPlanProcessBinding::new(\n                step_id(\"clean\"),\n                spec(\n                    \"clean\",\n                    ProcessKind::Clean,\n                    Some(\"clean\"),\n                    vec![WorkpieceLifecycle::Available],\n                ),\n            ),\n            ExactPlanProcessBinding::new(\n                step_id(\"inspect\"),\n                spec(\n                    \"inspect\",\n                    ProcessKind::Inspect,\n                    None,\n                    vec![WorkpieceLifecycle::Available],\n                ),\n            ),\n        ]\n    }\n\n"""
bindings_replacement = bindings_tail + """    fn capability_needs() -> Vec<CapabilityNeedSnapshot> {
        vec![CapabilityNeed::new(
            capability_id("clean"),
            id("capability:clean"),
            None,
            Vec::new(),
            vec![id("condition:a"), id("condition:b")],
        )
        .unwrap()
        .snapshot()
        .unwrap()]
    }

    fn executable_with(
        process_bindings: Vec<ExactPlanProcessBinding>,
    ) -> Result<ExecutableFabricationPlan, ExecutablePlanError> {
        ExecutableFabricationPlan::new_with_capability_needs(
            plan(),
            process_bindings,
            capability_needs(),
        )
    }

"""
text = replace_once(text, bindings_tail, bindings_replacement, "executable capability test helpers")

# Every pre-existing executable-plan test uses the common capability-bearing
# plan helper. Route it through the exact F5 binding helper; dedicated #605 tests
# below still exercise the zero-F5 convenience constructor explicitly.
text = text.replace(
    "ExecutableFabricationPlan::new(plan(), incomplete)",
    "executable_with(incomplete)",
)
text = text.replace(
    "ExecutableFabricationPlan::new(plan(), duplicated)",
    "executable_with(duplicated)",
)
text = text.replace(
    "ExecutableFabricationPlan::new(plan(), values)",
    "executable_with(values)",
)
text = text.replace(
    "ExecutableFabricationPlan::new(plan(), bindings()).unwrap()",
    "executable_with(bindings()).unwrap()",
)
text = text.replace(
    "ExecutableFabricationPlan::new(plan(), altered).unwrap()",
    "executable_with(altered).unwrap()",
)

exec_test_anchor = """    #[test]\n    fn constructor_rejects_snapshot_from_publicly_mutated_process_spec() {\n"""
exec_tests = r'''    #[test]
    fn capability_bearing_plan_requires_exact_f5_need_binding() {
        let result = ExecutableFabricationPlan::new(plan(), bindings());
        assert!(matches!(
            result,
            Err(ExecutablePlanError::MissingCapabilityNeedBinding(need_id))
                if need_id == capability_id("clean")
        ));
    }

    #[test]
    fn exact_f5_need_bindings_reject_duplicates_and_extras() {
        let mut duplicated = capability_needs();
        duplicated.push(duplicated[0].clone());
        assert!(matches!(
            ExecutableFabricationPlan::new_with_capability_needs(
                plan(),
                bindings(),
                duplicated,
            ),
            Err(ExecutablePlanError::DuplicateCapabilityNeedBinding(need_id))
                if need_id == capability_id("clean")
        ));

        let mut extra = capability_needs();
        extra.push(
            CapabilityNeed::new(
                capability_id("extra"),
                id("capability:extra"),
                None,
                Vec::new(),
                Vec::new(),
            )
            .unwrap()
            .snapshot()
            .unwrap(),
        );
        assert!(matches!(
            ExecutableFabricationPlan::new_with_capability_needs(plan(), bindings(), extra),
            Err(ExecutablePlanError::UnexpectedCapabilityNeedBinding(need_id))
                if need_id == capability_id("extra")
        ));
    }

    #[test]
    fn same_plan_id_revision_with_changed_f5_semantics_is_structurally_distinct() {
        let left = executable_with(bindings()).unwrap();
        let altered_need = CapabilityNeed::new(
            capability_id("clean"),
            id("capability:clean"),
            Some(id("mode:changed")),
            Vec::new(),
            vec![id("condition:a"), id("condition:b")],
        )
        .unwrap()
        .snapshot()
        .unwrap();
        let right = ExecutableFabricationPlan::new_with_capability_needs(
            plan(),
            bindings(),
            vec![altered_need],
        )
        .unwrap();

        assert_eq!(left.plan().id, right.plan().id);
        assert_eq!(left.plan().revision, right.plan().revision);
        assert_ne!(left, right);
    }

    #[test]
    fn exact_f5_need_wire_rejects_noncanonical_historical_order() {
        let executable = executable_with(bindings()).unwrap();
        let mut value = serde_json::to_value(&executable).unwrap();
        value["capability_needs"][0]["required_conditions"]
            .as_array_mut()
            .unwrap()
            .reverse();
        assert!(serde_json::from_value::<ExecutableFabricationPlan>(value).is_err());
    }

    #[test]
    fn exact_f5_need_binding_round_trip_is_queryable_by_need_id() {
        let executable = executable_with(bindings()).unwrap();
        let encoded = serde_json::to_vec(&executable).unwrap();
        let restored: ExecutableFabricationPlan = serde_json::from_slice(&encoded).unwrap();
        let need = restored.capability_need(&capability_id("clean")).unwrap();

        assert_eq!(restored, executable);
        assert_eq!(need.capability_id(), &id("capability:clean"));
        assert_eq!(
            need.required_conditions(),
            &[id("condition:a"), id("condition:b")]
        );
    }

'''
text = replace_once(text, exec_test_anchor, exec_tests + exec_test_anchor, "executable F5 tests")
executable.write_text(text)


# ---------------------------------------------------------------------------
# Firstlight exact compiler captures exact F5 requirements from its real catalog.
# ---------------------------------------------------------------------------
exact_plan = Path("crates/apps/symtropy-firstlight/src/patch_conduit_exact_plan.rs")
text = exact_plan.read_text()

text = replace_once(
    text,
    "use std::{error::Error, fmt};\n",
    "use std::{collections::BTreeSet, error::Error, fmt};\n",
    "Firstlight exact-plan std import",
)

text = replace_once(
    text,
    """use symtropy_fabrication::{\n    ExactPlanProcessBinding, ExecutableFabricationPlan, ExecutablePlanError, FunctionalSubject,\n};\n""",
    """use symtropy_fabrication::{
    CapabilityError, CapabilityNeedId, ExactPlanProcessBinding, ExecutableFabricationPlan,
    ExecutablePlanError, FunctionalSubject,
};
""",
    "Firstlight exact-plan fabrication imports",
)

old_compile = """            let exact_plan = ExecutableFabricationPlan::new(source.plan.clone(), bindings)?;\n            for temporary_work in &source.temporary_works {\n"""
new_compile = """            let capability_need_ids = source
                .plan
                .steps()
                .iter()
                .flat_map(|step| step.capability_needs().iter().cloned())
                .collect::<BTreeSet<_>>();
            let mut capability_needs = Vec::with_capacity(capability_need_ids.len());
            for need_id in capability_need_ids {
                let need = compiled
                    .catalog
                    .capability_need(&need_id)
                    .ok_or_else(|| ExactPatchConduitError::MissingCapabilityNeed(need_id.clone()))?;
                capability_needs.push(need.snapshot().map_err(|error| {
                    ExactPatchConduitError::CapabilityNeedSnapshot {
                        need_id: need_id.clone(),
                        error,
                    }
                })?);
            }

            let exact_plan = ExecutableFabricationPlan::new_with_capability_needs(
                source.plan.clone(),
                bindings,
                capability_needs,
            )?;
            for temporary_work in &source.temporary_works {
"""
text = replace_once(text, old_compile, new_compile, "Firstlight exact F5 compilation")

text = replace_once(
    text,
    """    MissingProcess(symtropy_fabrication::PlanStepId),\n    ExecutablePlan(ExecutablePlanError),\n    TemporaryWork(String),\n""",
    """    MissingProcess(symtropy_fabrication::PlanStepId),
    MissingCapabilityNeed(CapabilityNeedId),
    CapabilityNeedSnapshot {
        need_id: CapabilityNeedId,
        error: CapabilityError,
    },
    ExecutablePlan(ExecutablePlanError),
    TemporaryWork(String),
""",
    "Firstlight exact-plan errors",
)

text = replace_once(
    text,
    """            Self::MissingProcess(step) => {\n                write!(\n                    formatter,\n                    \"Patch Conduit exact plan lacks a process for step {step}\"\n                )\n            }\n            Self::ExecutablePlan(error) => fmt::Display::fmt(error, formatter),\n""",
    """            Self::MissingProcess(step) => {
                write!(
                    formatter,
                    "Patch Conduit exact plan lacks a process for step {step}"
                )
            }
            Self::MissingCapabilityNeed(need_id) => write!(
                formatter,
                "Patch Conduit exact plan lacks catalog semantics for capability need {need_id}"
            ),
            Self::CapabilityNeedSnapshot { need_id, error } => write!(
                formatter,
                "Patch Conduit capability need {need_id} could not become exact executable semantics: {error}"
            ),
            Self::ExecutablePlan(error) => fmt::Display::fmt(error, formatter),
""",
    "Firstlight exact-plan display",
)

text = replace_once(
    text,
    """        match self {\n            Self::ExecutablePlan(error) => Some(error),\n            Self::Profile(_) | Self::MissingProcess(_) | Self::TemporaryWork(_) => None,\n        }\n""",
    """        match self {
            Self::CapabilityNeedSnapshot { error, .. } => Some(error),
            Self::ExecutablePlan(error) => Some(error),
            Self::Profile(_)
            | Self::MissingProcess(_)
            | Self::MissingCapabilityNeed(_)
            | Self::TemporaryWork(_) => None,
        }
""",
    "Firstlight exact-plan source",
)

firstlight_test_anchor = """    #[test]\n    fn pressure_test_exact_binding_retains_in_situ_installed_admission() {\n"""
firstlight_tests = r'''    #[test]
    fn real_patch_conduit_exact_plan_binds_every_f5_need_semantically() {
        let scenario = PatchConduitScenario::canonical().unwrap();
        let compiled = PatchConduitExecutionProfile::compile(&scenario).unwrap();
        let exact = ExactPatchConduitProfile::from_compiled(&compiled).unwrap();

        for approach in exact.approaches() {
            for step in approach.plan.plan().steps() {
                for need_id in step.capability_needs() {
                    let expected = compiled
                        .catalog
                        .capability_need(need_id)
                        .unwrap()
                        .snapshot()
                        .unwrap();
                    assert_eq!(approach.plan.capability_need(need_id), Some(&expected));
                }
            }
        }
    }

'''
text = replace_once(
    text,
    firstlight_test_anchor,
    firstlight_tests + firstlight_test_anchor,
    "Firstlight exact F5 test",
)
exact_plan.write_text(text)
