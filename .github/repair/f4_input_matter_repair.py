from pathlib import Path
import re


def sub_once(text: str, pattern: str, replacement: str, label: str, flags: int = 0) -> str:
    updated, count = re.subn(pattern, replacement, text, count=1, flags=flags)
    if count != 1:
        raise SystemExit(f"{label} match count={count}")
    return updated


def insert_once(text: str, marker: str, addition: str, label: str) -> str:
    count = text.count(marker)
    if count != 1:
        raise SystemExit(f"{label} marker count={count}")
    return text.replace(marker, addition + marker, 1)


identity = Path("crates/domains/symtropy-fabrication/src/identity.rs")
text = identity.read_text()

text = sub_once(
    text,
    r"""    \) -> Result<Self, FabricationError> \{\n        let binding_digest = binding_digest\.into\(\);\n        if binding_digest\.is_empty\(\) \|\| binding_digest\.len\(\) > 256 \{\n            return Err\(FabricationError::InvalidMatterDigest\(binding_digest\)\);\n        \}\n        Ok\(Self \{\n            authority_id,\n            allocation_id,\n            revision,\n            binding_digest,\n        \}\)\n    \}\n""",
    """    ) -> Result<Self, FabricationError> {
        let binding = Self {
            authority_id,
            allocation_id,
            revision,
            binding_digest: binding_digest.into(),
        };
        binding.validate_current()?;
        Ok(binding)
    }

    /// Revalidates the current-schema binding after deserialization or public
    /// field mutation. This does not interpret the digest; it only enforces the
    /// same structural boundary as `MatterBinding::new`.
    pub fn validate_current(&self) -> Result<(), MatterIntegrityError> {
        if self.binding_digest.is_empty() || self.binding_digest.len() > 256 {
            return Err(MatterIntegrityError::InvalidDigest {
                authority_id: self.authority_id.clone(),
                allocation_id: self.allocation_id.clone(),
                digest: self.binding_digest.clone(),
            });
        }
        Ok(())
    }
""",
    "matter binding constructor",
)

if text.count("        validate_bindings(&matter_bindings)?;\n") != 1:
    raise SystemExit("workpiece constructor validator anchor mismatch")
text = text.replace(
    "        validate_bindings(&matter_bindings)?;\n",
    "        validate_matter_bindings(&matter_bindings)?;\n",
    1,
)

text = insert_once(
    text,
    "    /// Returns the exact binding for an authority-scoped matter allocation.\n",
    """    /// Revalidates current-schema matter identity after persistence or public
    /// mutation. Schema-version interpretation itself is deliberately separate.
    pub fn validate_matter_integrity(&self) -> Result<(), MatterIntegrityError> {
        validate_matter_bindings(&self.matter_bindings)
    }

""",
    "workpiece validation method",
)

text = sub_once(
    text,
    r"""fn validate_bindings\(bindings: &\[MatterBinding\]\) -> Result<\(\), FabricationError> \{.*?\n\}\n\nconst fn valid_transition""",
    """/// Replays current-schema matter-binding invariants for an existing value.
/// This is the single structural validator shared by constructors and authority
/// boundaries; it intentionally does not decide schema migration semantics.
pub fn validate_matter_bindings(
    bindings: &[MatterBinding],
) -> Result<(), MatterIntegrityError> {
    if bindings.is_empty() {
        return Err(MatterIntegrityError::BindingRequired);
    }
    for (index, binding) in bindings.iter().enumerate() {
        binding.validate_current()?;
        if bindings[..index].iter().any(|existing| {
            existing.authority_id == binding.authority_id
                && existing.allocation_id == binding.allocation_id
        }) {
            return Err(MatterIntegrityError::DuplicateAllocation {
                authority_id: binding.authority_id.clone(),
                allocation_id: binding.allocation_id.clone(),
            });
        }
    }
    Ok(())
}

const fn valid_transition""",
    "identity binding validator",
    flags=re.S,
)

text = insert_once(
    text,
    "/// Foundation errors are deterministic contract violations, not random\n/// workmanship outcomes.\n",
    """/// Current-schema structural matter-integrity failure. This retains
/// authority/allocation context so higher authority layers can explain which
/// physical identity failed revalidation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MatterIntegrityError {
    BindingRequired,
    InvalidDigest {
        authority_id: StableId,
        allocation_id: StableId,
        digest: String,
    },
    DuplicateAllocation {
        authority_id: StableId,
        allocation_id: StableId,
    },
}

impl fmt::Display for MatterIntegrityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BindingRequired => write!(formatter, "at least one matter binding is required"),
            Self::InvalidDigest {
                authority_id,
                allocation_id,
                digest,
            } => write!(
                formatter,
                "matter binding {authority_id}/{allocation_id} digest must contain 1..=256 bytes, got {}",
                digest.len()
            ),
            Self::DuplicateAllocation {
                authority_id,
                allocation_id,
            } => write!(
                formatter,
                "matter allocation {authority_id}/{allocation_id} is bound more than once"
            ),
        }
    }
}

impl Error for MatterIntegrityError {}

impl From<MatterIntegrityError> for FabricationError {
    fn from(error: MatterIntegrityError) -> Self {
        match error {
            MatterIntegrityError::BindingRequired => Self::MatterBindingRequired,
            MatterIntegrityError::InvalidDigest { digest, .. } => Self::InvalidMatterDigest(digest),
            MatterIntegrityError::DuplicateAllocation {
                authority_id,
                allocation_id,
            } => Self::DuplicateMatterAllocation {
                authority_id,
                allocation_id,
            },
        }
    }
}

""",
    "matter integrity error",
)

text = insert_once(
    text,
    "    #[test]\n    fn one_authority_scoped_allocation_cannot_be_bound_twice() {\n",
    """    #[test]
    fn current_schema_matter_integrity_revalidates_public_mutation() {
        let mut workpiece = Workpiece::new(
            WorkpieceId::new(id("workpiece:mutated")),
            vec![binding("allocation:mutated", 1)],
        )
        .unwrap();
        workpiece.matter_bindings[0].binding_digest.clear();

        assert!(matches!(
            workpiece.validate_matter_integrity(),
            Err(MatterIntegrityError::InvalidDigest { .. })
        ));
    }

""",
    "identity regression",
)
identity.write_text(text)


process = Path("crates/domains/symtropy-fabrication/src/process.rs")
text = process.read_text()

if text.count("use crate::{MatterBinding, Workpiece, WorkpieceId, WorkpieceLifecycle};\n") != 1:
    raise SystemExit("process identity import anchor mismatch")
text = text.replace(
    "use crate::{MatterBinding, Workpiece, WorkpieceId, WorkpieceLifecycle};\n",
    "use crate::{MatterBinding, MatterIntegrityError, Workpiece, WorkpieceId, WorkpieceLifecycle};\n",
    1,
)

text = sub_once(
    text,
    r"""            if !self\.allowed_workpiece_states\.contains\(&workpiece\.lifecycle\) \{\n                failures\.push\(ProcessPreconditionFailure::WorkpieceState \{\n                    workpiece_id: workpiece\.id\.clone\(\),\n                    actual: workpiece\.lifecycle,\n                \}\);\n            \}\n        \}\n\n        for requirement""",
    """            if !self.allowed_workpiece_states.contains(&workpiece.lifecycle) {
                failures.push(ProcessPreconditionFailure::WorkpieceState {
                    workpiece_id: workpiece.id.clone(),
                    actual: workpiece.lifecycle,
                });
            }
            if let Err(error) = workpiece.validate_matter_integrity() {
                failures.push(match error {
                    MatterIntegrityError::BindingRequired => {
                        ProcessPreconditionFailure::WorkpieceMatterRequired {
                            workpiece_id: workpiece.id.clone(),
                        }
                    }
                    MatterIntegrityError::InvalidDigest {
                        authority_id,
                        allocation_id,
                        digest,
                    } => ProcessPreconditionFailure::InvalidWorkpieceMatterDigest {
                        workpiece_id: workpiece.id.clone(),
                        authority_id,
                        allocation_id,
                        digest,
                    },
                    MatterIntegrityError::DuplicateAllocation {
                        authority_id,
                        allocation_id,
                    } => ProcessPreconditionFailure::DuplicateWorkpieceMatterAllocation {
                        workpiece_id: workpiece.id.clone(),
                        authority_id,
                        allocation_id,
                    },
                });
            }

            for binding in &workpiece.matter_bindings {
                for previous_workpiece in &workpieces[..index] {
                    if previous_workpiece.id != workpiece.id
                        && previous_workpiece.matter_bindings.iter().any(|existing| {
                            existing.authority_id == binding.authority_id
                                && existing.allocation_id == binding.allocation_id
                        })
                    {
                        failures.push(
                            ProcessPreconditionFailure::CrossWorkpieceMatterAllocationAlias {
                                authority_id: binding.authority_id.clone(),
                                allocation_id: binding.allocation_id.clone(),
                                first_workpiece_id: previous_workpiece.id.clone(),
                                second_workpiece_id: workpiece.id.clone(),
                            },
                        );
                        break;
                    }
                }
            }
        }

        for requirement""",
    "process admission loop",
)

text = sub_once(
    text,
    r"""    WorkpieceState \{\n        workpiece_id: WorkpieceId,\n        actual: WorkpieceLifecycle,\n    \},\n    Capability \{""",
    """    WorkpieceState {
        workpiece_id: WorkpieceId,
        actual: WorkpieceLifecycle,
    },
    WorkpieceMatterRequired {
        workpiece_id: WorkpieceId,
    },
    InvalidWorkpieceMatterDigest {
        workpiece_id: WorkpieceId,
        authority_id: StableId,
        allocation_id: StableId,
        digest: String,
    },
    DuplicateWorkpieceMatterAllocation {
        workpiece_id: WorkpieceId,
        authority_id: StableId,
        allocation_id: StableId,
    },
    CrossWorkpieceMatterAllocationAlias {
        authority_id: StableId,
        allocation_id: StableId,
        first_workpiece_id: WorkpieceId,
        second_workpiece_id: WorkpieceId,
    },
    Capability {""",
    "process precondition variants",
)

text = insert_once(
    text,
    "    #[test]\n    fn abort_is_valid_and_cannot_later_be_completed() {\n",
    """    #[test]
    fn restored_workpiece_without_matter_is_rejected_before_execution() {
        let mut input = workpiece("workpiece:empty-matter", 1);
        input.matter_bindings.clear();
        let result = ProcessExecution::begin(
            ProcessExecutionId::new(id("process-execution:empty-matter")),
            &weld_spec(),
            &[&input],
            &[welder()],
        );
        assert!(matches!(
            result,
            Err(ProcessError::Preconditions(failures))
                if failures.iter().any(|failure| matches!(
                    failure,
                    ProcessPreconditionFailure::WorkpieceMatterRequired { .. }
                ))
        ));
    }

    #[test]
    fn restored_invalid_input_matter_digest_is_rejected_before_execution() {
        let mut input = workpiece("workpiece:bad-digest", 1);
        input.matter_bindings[0].binding_digest.clear();
        let result = ProcessExecution::begin(
            ProcessExecutionId::new(id("process-execution:bad-digest")),
            &weld_spec(),
            &[&input],
            &[welder()],
        );
        assert!(matches!(
            result,
            Err(ProcessError::Preconditions(failures))
                if failures.iter().any(|failure| matches!(
                    failure,
                    ProcessPreconditionFailure::InvalidWorkpieceMatterDigest { .. }
                ))
        ));
    }

    #[test]
    fn restored_duplicate_allocation_inside_one_workpiece_is_rejected() {
        let mut input = workpiece("workpiece:duplicate-matter", 1);
        input.matter_bindings.push(input.matter_bindings[0].clone());
        let result = ProcessExecution::begin(
            ProcessExecutionId::new(id("process-execution:duplicate-matter")),
            &weld_spec(),
            &[&input],
            &[welder()],
        );
        assert!(matches!(
            result,
            Err(ProcessError::Preconditions(failures))
                if failures.iter().any(|failure| matches!(
                    failure,
                    ProcessPreconditionFailure::DuplicateWorkpieceMatterAllocation { .. }
                ))
        ));
    }

    #[test]
    fn distinct_workpieces_cannot_alias_same_authority_scoped_allocation() {
        let first = workpiece("workpiece:alias-a", 1);
        let mut second = workpiece("workpiece:alias-b", 1);
        second.matter_bindings = first.matter_bindings.clone();
        let result = ProcessExecution::begin(
            ProcessExecutionId::new(id("process-execution:cross-alias")),
            &weld_spec(),
            &[&first, &second],
            &[welder()],
        );
        assert!(matches!(
            result,
            Err(ProcessError::Preconditions(failures))
                if failures.iter().any(|failure| matches!(
                    failure,
                    ProcessPreconditionFailure::CrossWorkpieceMatterAllocationAlias {
                        first_workpiece_id,
                        second_workpiece_id,
                        ..
                    } if first_workpiece_id == &first.id && second_workpiece_id == &second.id
                ))
        ));
    }

    #[test]
    fn same_local_input_allocation_id_from_distinct_authorities_is_valid() {
        let mut first = workpiece("workpiece:authority-a", 1);
        first.matter_bindings = vec![MatterBinding::new(
            id("matter:authority-a"),
            id("allocation:shared-input"),
            1,
            "digest:a:shared-input:1",
        )
        .unwrap()];
        let mut second = workpiece("workpiece:authority-b", 1);
        second.matter_bindings = vec![MatterBinding::new(
            id("matter:authority-b"),
            id("allocation:shared-input"),
            1,
            "digest:b:shared-input:1",
        )
        .unwrap()];
        let result = ProcessExecution::begin(
            ProcessExecutionId::new(id("process-execution:multi-authority-input")),
            &weld_spec(),
            &[&first, &second],
            &[welder()],
        );
        assert!(result.is_ok());
    }

    #[test]
    fn distinct_multi_input_allocations_are_valid() {
        let first = workpiece("workpiece:multi-a", 1);
        let second = workpiece("workpiece:multi-b", 1);
        let result = ProcessExecution::begin(
            ProcessExecutionId::new(id("process-execution:distinct-multi")),
            &weld_spec(),
            &[&first, &second],
            &[welder()],
        );
        assert!(result.is_ok());
    }

""",
    "process regressions",
)
process.write_text(text)


evidence = Path("crates/domains/symtropy-fabrication/src/process_evidence.rs")
text = evidence.read_text()

text = sub_once(
    text,
    r"""use crate::\{\n    CapabilityEvidence, MatterBinding, ProcessEvidence, ProcessExecutionId, ProcessExecutionState,\n    ProcessInputSnapshot, ProcessKind, ProcessSpecId, ProcessSpecSnapshot, WorkpieceId,\n    WorkpieceLifecycle,\n\};""",
    """use crate::{
    CapabilityEvidence, MatterBinding, MatterIntegrityError, ProcessEvidence, ProcessExecutionId,
    ProcessExecutionState, ProcessInputSnapshot, ProcessKind, ProcessSpecId, ProcessSpecSnapshot,
    WorkpieceId, WorkpieceLifecycle, validate_matter_bindings,
};""",
    "process evidence imports",
)

text = sub_once(
    text,
    r"""        validate_input_matter_bindings\(&input\.matter_bindings\)\?;\n    \}\n\n    for requirement""",
    """        validate_input_matter_bindings(&input.matter_bindings)?;
        for previous_input in &evidence.inputs[..index] {
            for binding in &input.matter_bindings {
                if previous_input.matter_bindings.iter().any(|existing| {
                    existing.authority_id == binding.authority_id
                        && existing.allocation_id == binding.allocation_id
                }) {
                    return Err(ProcessEvidenceValidationError::CrossInputMatterAllocationAlias {
                        authority_id: binding.authority_id.clone(),
                        allocation_id: binding.allocation_id.clone(),
                        first_workpiece_id: previous_input.workpiece_id.clone(),
                        second_workpiece_id: input.workpiece_id.clone(),
                    });
                }
            }
        }
    }

    for requirement""",
    "wire cross-input validation",
)

text = sub_once(
    text,
    r"""fn validate_input_matter_bindings\(.*?\n\}\n\nfn validate_resulting_matter_bindings\(.*?\n\}\n\nfn validate_matter_digests\(.*?\n\}\n""",
    """fn validate_input_matter_bindings(
    bindings: &[MatterBinding],
) -> Result<(), ProcessEvidenceValidationError> {
    validate_matter_bindings(bindings).map_err(|error| match error {
        MatterIntegrityError::BindingRequired => ProcessEvidenceValidationError::InputMatterRequired,
        MatterIntegrityError::InvalidDigest {
            authority_id,
            allocation_id,
            digest,
        } => ProcessEvidenceValidationError::InvalidMatterDigest {
            authority_id,
            allocation_id,
            digest,
        },
        MatterIntegrityError::DuplicateAllocation {
            authority_id,
            allocation_id,
        } => ProcessEvidenceValidationError::DuplicateInputMatterAllocation {
            authority_id,
            allocation_id,
        },
    })
}

fn validate_resulting_matter_bindings(
    bindings: &[MatterBinding],
) -> Result<(), ProcessEvidenceValidationError> {
    validate_matter_bindings(bindings).map_err(|error| match error {
        MatterIntegrityError::BindingRequired => ProcessEvidenceValidationError::ResultingMatterRequired,
        MatterIntegrityError::InvalidDigest {
            authority_id,
            allocation_id,
            digest,
        } => ProcessEvidenceValidationError::InvalidMatterDigest {
            authority_id,
            allocation_id,
            digest,
        },
        MatterIntegrityError::DuplicateAllocation {
            authority_id,
            allocation_id,
        } => ProcessEvidenceValidationError::DuplicateResultingMatterAllocation {
            authority_id,
            allocation_id,
        },
    })
}
""",
    "wire matter validator centralization",
    flags=re.S,
)

text = sub_once(
    text,
    r"""    DuplicateInputMatterAllocation \{\n        authority_id: StableId,\n        allocation_id: StableId,\n    \},\n    DuplicateResultingMatterAllocation \{""",
    """    DuplicateInputMatterAllocation {
        authority_id: StableId,
        allocation_id: StableId,
    },
    CrossInputMatterAllocationAlias {
        authority_id: StableId,
        allocation_id: StableId,
        first_workpiece_id: WorkpieceId,
        second_workpiece_id: WorkpieceId,
    },
    DuplicateResultingMatterAllocation {""",
    "wire alias error variant",
)

text = sub_once(
    text,
    r"""            Self::DuplicateInputMatterAllocation \{\n                authority_id,\n                allocation_id,\n            \} => write!\(\n                formatter,\n                "process evidence repeats input matter allocation \{authority_id\}/\{allocation_id\}"\n            \),\n            Self::DuplicateResultingMatterAllocation \{""",
    """            Self::DuplicateInputMatterAllocation {
                authority_id,
                allocation_id,
            } => write!(
                formatter,
                "process evidence repeats input matter allocation {authority_id}/{allocation_id}"
            ),
            Self::CrossInputMatterAllocationAlias {
                authority_id,
                allocation_id,
                first_workpiece_id,
                second_workpiece_id,
            } => write!(
                formatter,
                "process evidence aliases input matter allocation {authority_id}/{allocation_id} across workpieces {first_workpiece_id} and {second_workpiece_id}"
            ),
            Self::DuplicateResultingMatterAllocation {""",
    "wire alias error display",
)

text = insert_once(
    text,
    "    #[test]\n    fn nested_matter_constructor_invariants_are_revalidated() {\n",
    """    #[test]
    fn wire_cannot_alias_one_matter_allocation_across_distinct_inputs() {
        let mut value = serde_json::to_value(evidence()).unwrap();
        let mut second = value["inputs"][0].clone();
        second["workpiece_id"] = serde_json::Value::String("workpiece:second".into());
        value["inputs"].as_array_mut().unwrap().push(second);
        assert!(serde_json::from_value::<ValidatedProcessEvidence>(value).is_err());
    }

    #[test]
    fn wire_allows_same_local_allocation_id_under_distinct_authorities() {
        let mut value = serde_json::to_value(evidence()).unwrap();
        let mut second = value["inputs"][0].clone();
        second["workpiece_id"] = serde_json::Value::String("workpiece:second-authority".into());
        second["matter_bindings"][0]["authority_id"] =
            serde_json::Value::String("matter-authority:other".into());
        second["matter_bindings"][0]["binding_digest"] =
            serde_json::Value::String("digest:other-authority".into());
        value["inputs"].as_array_mut().unwrap().push(second);
        assert!(serde_json::from_value::<ValidatedProcessEvidence>(value).is_ok());
    }

""",
    "wire regressions",
)
evidence.write_text(text)
