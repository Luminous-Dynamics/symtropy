from pathlib import Path
import re


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected one exact anchor, found {count}")
    return text.replace(old, new, 1)


def sub_once(text: str, pattern: str, replacement: str, label: str, flags: int = 0) -> str:
    updated, count = re.subn(pattern, replacement, text, count=1, flags=flags)
    if count != 1:
        raise SystemExit(f"{label}: regex match count={count}")
    return updated


process = Path("crates/domains/symtropy-fabrication/src/process.rs")
text = process.read_text()

text = replace_once(
    text,
    "use serde::{Deserialize, Serialize};\n",
    "use serde::{Deserialize, Deserializer, Serialize};\n",
    "process serde import",
)

text = replace_once(
    text,
    "#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]\npub struct ProcessSpecSnapshot {\n",
    "#[derive(Debug, Clone, PartialEq, Eq, Serialize)]\npub struct ProcessSpecSnapshot {\n",
    "snapshot derive",
)

snapshot_marker = """}\n\nimpl ProcessSpecSnapshot {\n    pub fn required_capabilities(&self) -> &[CapabilityRequirement] {\n"""
snapshot_replacement = """}\n\n#[derive(Deserialize)]
struct ProcessSpecSnapshotWire {
    id: ProcessSpecId,
    revision: u64,
    kind: ProcessKind,
    required_capabilities: Vec<CapabilityRequirement>,
    allowed_workpiece_states: Vec<WorkpieceLifecycle>,
}

/// Representation-level canonicality failures for exact F4 process snapshots.
///
/// A reusable `ProcessSpec` may be normalized through `ProcessSpec::new`, but a
/// historical `ProcessSpecSnapshot` must already be in this exact canonical
/// representation before it can regain authority after persistence.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProcessSpecSnapshotError {
    NoAllowedWorkpieceStates,
    DuplicateRequiredCapability(StableId),
    NonCanonicalCapabilityOrder,
    DuplicateAllowedWorkpieceState(WorkpieceLifecycle),
    NonCanonicalLifecycleOrder,
}

impl fmt::Display for ProcessSpecSnapshotError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoAllowedWorkpieceStates => write!(
                formatter,
                "canonical process snapshot requires at least one allowed workpiece state"
            ),
            Self::DuplicateRequiredCapability(capability_id) => write!(
                formatter,
                "canonical process snapshot repeats required capability {capability_id}"
            ),
            Self::NonCanonicalCapabilityOrder => write!(
                formatter,
                "canonical process snapshot capability requirements are not constructor-normalized"
            ),
            Self::DuplicateAllowedWorkpieceState(state) => write!(
                formatter,
                "canonical process snapshot repeats allowed workpiece state {state:?}"
            ),
            Self::NonCanonicalLifecycleOrder => write!(
                formatter,
                "canonical process snapshot allowed workpiece states are not constructor-normalized"
            ),
        }
    }
}

impl Error for ProcessSpecSnapshotError {}

impl<'de> Deserialize<'de> for ProcessSpecSnapshot {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = ProcessSpecSnapshotWire::deserialize(deserializer)?;
        let snapshot = Self {
            id: wire.id,
            revision: wire.revision,
            kind: wire.kind,
            required_capabilities: wire.required_capabilities,
            allowed_workpiece_states: wire.allowed_workpiece_states,
        };
        snapshot
            .validate_canonical()
            .map_err(serde::de::Error::custom)?;
        Ok(snapshot)
    }
}

impl ProcessSpecSnapshot {
    /// Proves that this exact representation is the unique constructor-normalized
    /// form used for structural equality at F4/F10 authority boundaries.
    pub fn validate_canonical(&self) -> Result<(), ProcessSpecSnapshotError> {
        for (index, requirement) in self.required_capabilities.iter().enumerate() {
            if self.required_capabilities[..index]
                .iter()
                .any(|existing| existing.capability_id == requirement.capability_id)
            {
                return Err(ProcessSpecSnapshotError::DuplicateRequiredCapability(
                    requirement.capability_id.clone(),
                ));
            }
        }
        for pair in self.required_capabilities.windows(2) {
            if (&pair[0].capability_id, pair[0].minimum_value)
                >= (&pair[1].capability_id, pair[1].minimum_value)
            {
                return Err(ProcessSpecSnapshotError::NonCanonicalCapabilityOrder);
            }
        }

        if self.allowed_workpiece_states.is_empty() {
            return Err(ProcessSpecSnapshotError::NoAllowedWorkpieceStates);
        }
        for (index, state) in self.allowed_workpiece_states.iter().enumerate() {
            if self.allowed_workpiece_states[..index]
                .iter()
                .any(|existing| existing == state)
            {
                return Err(ProcessSpecSnapshotError::DuplicateAllowedWorkpieceState(*state));
            }
        }
        for pair in self.allowed_workpiece_states.windows(2) {
            if lifecycle_rank(pair[0]) >= lifecycle_rank(pair[1]) {
                return Err(ProcessSpecSnapshotError::NonCanonicalLifecycleOrder);
            }
        }
        Ok(())
    }

    pub fn required_capabilities(&self) -> &[CapabilityRequirement] {
"""
text = replace_once(text, snapshot_marker, snapshot_replacement, "snapshot authority block")

text = replace_once(
    text,
    "#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]\npub struct ProcessSpec {\n",
    "#[derive(Debug, Clone, PartialEq, Eq, Serialize)]\npub struct ProcessSpec {\n",
    "spec derive",
)

spec_marker = """}\n\nimpl ProcessSpec {\n    pub fn new(\n"""
spec_replacement = """}

#[derive(Deserialize)]
struct ProcessSpecWire {
    id: ProcessSpecId,
    revision: u64,
    kind: ProcessKind,
    required_capabilities: Vec<CapabilityRequirement>,
    allowed_workpiece_states: Vec<WorkpieceLifecycle>,
}

impl<'de> Deserialize<'de> for ProcessSpec {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let wire = ProcessSpecWire::deserialize(deserializer)?;
        Self::new(
            wire.id,
            wire.revision,
            wire.kind,
            wire.required_capabilities,
            wire.allowed_workpiece_states,
        )
        .map_err(serde::de::Error::custom)
    }
}

impl ProcessSpec {
    pub fn new(
"""
text = replace_once(text, spec_marker, spec_replacement, "spec wire block")

text = replace_once(
    text,
    """    /// Returns the canonical semantic contract captured by plans/executions.\n    pub fn snapshot(&self) -> ProcessSpecSnapshot {\n""",
    """    /// Revalidates a live spec before an authority-bearing consumer uses it.
    /// This catches public field mutation without silently normalizing an object
    /// that is already being presented as live execution knowledge.
    pub fn validate_canonical(&self) -> Result<(), ProcessSpecSnapshotError> {
        self.snapshot().validate_canonical()
    }

    /// Returns the structural process semantics. Authority-bearing consumers
    /// must call `validate_canonical` before accepting a snapshot produced from
    /// a publicly mutable `ProcessSpec`.
    pub fn snapshot(&self) -> ProcessSpecSnapshot {
""",
    "spec live validator",
)

text = replace_once(
    text,
    """    ) -> Result<Self, ProcessError> {\n        if let Err(failures) = spec.evaluate_preconditions(workpieces, capabilities) {\n""",
    """    ) -> Result<Self, ProcessError> {
        spec.validate_canonical()
            .map_err(ProcessError::NonCanonicalSpec)?;
        if let Err(failures) = spec.evaluate_preconditions(workpieces, capabilities) {
""",
    "execution live spec validation",
)

text = replace_once(
    text,
    """    DuplicateAllowedWorkpieceState(WorkpieceLifecycle),\n    Preconditions(Vec<ProcessPreconditionFailure>),\n""",
    """    DuplicateAllowedWorkpieceState(WorkpieceLifecycle),
    NonCanonicalSpec(ProcessSpecSnapshotError),
    Preconditions(Vec<ProcessPreconditionFailure>),
""",
    "process error variant",
)

text = replace_once(
    text,
    """            Self::DuplicateAllowedWorkpieceState(lifecycle) => write!(\n                formatter,\n                "process repeats allowed workpiece lifecycle {lifecycle:?}"\n            ),\n            Self::Preconditions(failures) => {\n""",
    """            Self::DuplicateAllowedWorkpieceState(lifecycle) => write!(
                formatter,
                "process repeats allowed workpiece lifecycle {lifecycle:?}"
            ),
            Self::NonCanonicalSpec(error) => {
                write!(formatter, "process spec is not canonical: {error}")
            }
            Self::Preconditions(failures) => {
""",
    "process error display",
)

process_test_anchor = """    #[test]\n    fn same_id_revision_with_changed_semantics_has_distinct_snapshot() {\n"""
process_tests = r'''    #[test]
    fn reusable_process_spec_wire_normalizes_input_order() {
        let canonical = ProcessSpec::new(
            ProcessSpecId::new(id("process-spec:wire-normalization")),
            2,
            ProcessKind::PressureTest,
            vec![
                requirement("capability:a", 1),
                requirement("capability:b", 2),
            ],
            vec![WorkpieceLifecycle::Available, WorkpieceLifecycle::Installed],
        )
        .unwrap();
        let mut value = serde_json::to_value(&canonical).unwrap();
        value["required_capabilities"].as_array_mut().unwrap().reverse();
        value["allowed_workpiece_states"].as_array_mut().unwrap().reverse();

        let restored: ProcessSpec = serde_json::from_value(value).unwrap();
        assert_eq!(restored, canonical);
        assert_eq!(restored.snapshot(), canonical.snapshot());
    }

    #[test]
    fn reusable_process_spec_wire_rejects_duplicate_capability_identity() {
        let mut value = serde_json::to_value(weld_spec()).unwrap();
        let mut duplicate = value["required_capabilities"][0].clone();
        duplicate["minimum_value"] = 801.into();
        value["required_capabilities"]
            .as_array_mut()
            .unwrap()
            .push(duplicate);
        assert!(serde_json::from_value::<ProcessSpec>(value).is_err());
    }

    #[test]
    fn reusable_process_spec_wire_rejects_empty_or_duplicate_allowed_states() {
        let mut empty = serde_json::to_value(weld_spec()).unwrap();
        empty["allowed_workpiece_states"] = serde_json::Value::Array(Vec::new());
        assert!(serde_json::from_value::<ProcessSpec>(empty).is_err());

        let mut duplicate = serde_json::to_value(weld_spec()).unwrap();
        duplicate["allowed_workpiece_states"] =
            serde_json::json!(["available", "available"]);
        assert!(serde_json::from_value::<ProcessSpec>(duplicate).is_err());
    }

    #[test]
    fn exact_snapshot_wire_rejects_noncanonical_capability_order() {
        let spec = ProcessSpec::new(
            ProcessSpecId::new(id("process-spec:snapshot-cap-order")),
            1,
            ProcessKind::Inspect,
            vec![
                requirement("capability:a", 1),
                requirement("capability:b", 1),
            ],
            vec![WorkpieceLifecycle::Available],
        )
        .unwrap();
        let mut value = serde_json::to_value(spec.snapshot()).unwrap();
        value["required_capabilities"].as_array_mut().unwrap().reverse();
        assert!(serde_json::from_value::<ProcessSpecSnapshot>(value).is_err());
    }

    #[test]
    fn exact_snapshot_wire_rejects_duplicate_capability_even_with_different_thresholds() {
        let mut value = serde_json::to_value(weld_spec().snapshot()).unwrap();
        let mut duplicate = value["required_capabilities"][0].clone();
        duplicate["minimum_value"] = 999.into();
        value["required_capabilities"]
            .as_array_mut()
            .unwrap()
            .push(duplicate);
        assert!(serde_json::from_value::<ProcessSpecSnapshot>(value).is_err());
    }

    #[test]
    fn exact_snapshot_wire_rejects_empty_duplicate_or_reordered_states() {
        let snapshot = weld_spec().snapshot();

        let mut empty = serde_json::to_value(&snapshot).unwrap();
        empty["allowed_workpiece_states"] = serde_json::Value::Array(Vec::new());
        assert!(serde_json::from_value::<ProcessSpecSnapshot>(empty).is_err());

        let mut duplicate = serde_json::to_value(&snapshot).unwrap();
        duplicate["allowed_workpiece_states"] =
            serde_json::json!(["available", "available"]);
        assert!(serde_json::from_value::<ProcessSpecSnapshot>(duplicate).is_err());

        let mut reordered = serde_json::to_value(&snapshot).unwrap();
        reordered["allowed_workpiece_states"] =
            serde_json::json!(["installed", "available"]);
        assert!(serde_json::from_value::<ProcessSpecSnapshot>(reordered).is_err());
    }

    #[test]
    fn execution_rejects_publicly_mutated_noncanonical_spec() {
        let workpiece = workpiece("workpiece:mutated-spec", 1);
        let mut spec = weld_spec();
        spec.allowed_workpiece_states.reverse();
        let result = ProcessExecution::begin(
            ProcessExecutionId::new(id("process-execution:mutated-spec")),
            &spec,
            &[&workpiece],
            &[welder()],
        );
        assert!(matches!(
            result,
            Err(ProcessError::NonCanonicalSpec(
                ProcessSpecSnapshotError::NonCanonicalLifecycleOrder
            ))
        ));
    }

'''
text = replace_once(text, process_test_anchor, process_tests + process_test_anchor, "process canonicality tests")
process.write_text(text)


evidence = Path("crates/domains/symtropy-fabrication/src/process_evidence.rs")
text = evidence.read_text()
text = replace_once(
    text,
    """    ProcessExecutionState, ProcessInputSnapshot, ProcessKind, ProcessSpecId, ProcessSpecSnapshot,\n    WorkpieceId, WorkpieceLifecycle, validate_matter_bindings,\n""",
    """    ProcessExecutionState, ProcessInputSnapshot, ProcessKind, ProcessSpecId, ProcessSpecSnapshot,
    ProcessSpecSnapshotError, WorkpieceId, WorkpieceLifecycle, validate_matter_bindings,
""",
    "process evidence imports",
)
text = replace_once(
    text,
    "    validate_spec_snapshot(&evidence.spec_snapshot)?;\n",
    """    evidence
        .spec_snapshot
        .validate_canonical()
        .map_err(map_spec_snapshot_error)?;
""",
    "process evidence snapshot validation call",
)
text = sub_once(
    text,
    r"fn validate_spec_snapshot\(\n    snapshot: &ProcessSpecSnapshot,\n\) -> Result<\(\), ProcessEvidenceValidationError> \{.*?\n\}\n\nfn validate_input_matter_bindings",
    """fn map_spec_snapshot_error(
    error: ProcessSpecSnapshotError,
) -> ProcessEvidenceValidationError {
    match error {
        ProcessSpecSnapshotError::NoAllowedWorkpieceStates => {
            ProcessEvidenceValidationError::SnapshotAllowedStateRequired
        }
        ProcessSpecSnapshotError::DuplicateRequiredCapability(capability_id) => {
            ProcessEvidenceValidationError::DuplicateSnapshotRequiredCapability(capability_id)
        }
        ProcessSpecSnapshotError::NonCanonicalCapabilityOrder => {
            ProcessEvidenceValidationError::SnapshotCapabilityOrder
        }
        ProcessSpecSnapshotError::DuplicateAllowedWorkpieceState(state) => {
            ProcessEvidenceValidationError::DuplicateSnapshotAllowedWorkpieceState(state)
        }
        ProcessSpecSnapshotError::NonCanonicalLifecycleOrder => {
            ProcessEvidenceValidationError::SnapshotLifecycleOrder
        }
    }
}

fn validate_input_matter_bindings""",
    "remove duplicated snapshot validator",
    flags=re.S,
)
text = sub_once(
    text,
    r"\nconst fn lifecycle_rank\(lifecycle: WorkpieceLifecycle\) -> u8 \{.*?\n\}\n\n#\[derive\(Debug, Clone, PartialEq, Eq\)\]",
    "\n#[derive(Debug, Clone, PartialEq, Eq)]",
    "remove duplicated evidence lifecycle rank",
    flags=re.S,
)
evidence.write_text(text)


executable = Path("crates/domains/symtropy-fabrication/src/executable_plan.rs")
text = executable.read_text()
text = replace_once(
    text,
    "use crate::{FabricationPlan, PlanStepId, ProcessSpecId, ProcessSpecSnapshot};\n",
    """use crate::{
    FabricationPlan, PlanStepId, ProcessSpecId, ProcessSpecSnapshot, ProcessSpecSnapshotError,
};
""",
    "executable imports",
)
text = replace_once(
    text,
    """        for binding in &process_bindings {\n            let step = plan\n""",
    """        for binding in &process_bindings {
            binding
                .process_spec
                .validate_canonical()
                .map_err(|error| ExecutablePlanError::NonCanonicalProcessSpec {
                    step_id: binding.step_id.clone(),
                    error,
                })?;
            let step = plan
""",
    "executable live snapshot validation",
)
text = replace_once(
    text,
    """    UnknownStepBinding(PlanStepId),\n    ProcessIdentityMismatch {\n""",
    """    UnknownStepBinding(PlanStepId),
    NonCanonicalProcessSpec {
        step_id: PlanStepId,
        error: ProcessSpecSnapshotError,
    },
    ProcessIdentityMismatch {
""",
    "executable error variant",
)
text = replace_once(
    text,
    """            Self::UnknownStepBinding(step_id) => {\n                write!(\n                    formatter,\n                    "executable process binding references unknown step {step_id}"\n                )\n            }\n            Self::ProcessIdentityMismatch {\n""",
    """            Self::UnknownStepBinding(step_id) => {
                write!(
                    formatter,
                    "executable process binding references unknown step {step_id}"
                )
            }
            Self::NonCanonicalProcessSpec { step_id, error } => write!(
                formatter,
                "executable plan step {step_id} carries a noncanonical F4 process snapshot: {error}"
            ),
            Self::ProcessIdentityMismatch {
""",
    "executable error display",
)

exec_test_anchor = """    #[test]\n    fn deserialization_revalidates_executable_plan_contract() {\n"""
exec_tests = r'''    #[test]
    fn constructor_rejects_snapshot_from_publicly_mutated_process_spec() {
        let mut process = ProcessSpec::new(
            process_id("clean"),
            1,
            ProcessKind::Clean,
            vec![CapabilityRequirement {
                capability_id: capability_id("clean").stable_id().clone(),
                minimum_value: 1,
            }],
            vec![WorkpieceLifecycle::Available, WorkpieceLifecycle::Installed],
        )
        .unwrap();
        process.allowed_workpiece_states.reverse();

        let mut values = bindings();
        values[0].process_spec = process.snapshot();
        let result = ExecutableFabricationPlan::new(plan(), values);
        assert!(matches!(
            result,
            Err(ExecutablePlanError::NonCanonicalProcessSpec {
                error: ProcessSpecSnapshotError::NonCanonicalLifecycleOrder,
                ..
            })
        ));
    }

    #[test]
    fn deserialization_rejects_nested_noncanonical_process_snapshot() {
        let executable = ExecutableFabricationPlan::new(plan(), bindings()).unwrap();
        let mut value = serde_json::to_value(&executable).unwrap();
        value["process_bindings"][0]["process_spec"]["allowed_workpiece_states"] =
            serde_json::json!(["installed", "available"]);
        assert!(serde_json::from_value::<ExecutableFabricationPlan>(value).is_err());
    }

'''
text = replace_once(text, exec_test_anchor, exec_tests + exec_test_anchor, "executable canonicality tests")
executable.write_text(text)
