#!/usr/bin/env python3
# Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Apply PB-01 #439 H1/H3 to the primary proposal path.

The transformer is deliberately exact-string/fail-closed. If the source no
longer matches the reviewed preimage, it refuses to guess.
"""

from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
LIB = ROOT / "crates/domains/symtropy-player-building/src/lib.rs"
TEST = ROOT / "crates/domains/symtropy-player-building/tests/h1_h3_primary_path.rs"


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected exactly one match, found {count}")
    return text.replace(old, new, 1)


s = LIB.read_text()

# Intent target set is first-class authored scope.
s = replace_once(
    s,
    "    parent_refs: Vec<ConstructionIntentRef>,\n    elements: Vec<IntentElement>,\n    source_refs: Vec<ExactAuthorityRef>,",
    "    parent_refs: Vec<ConstructionIntentRef>,\n    elements: Vec<IntentElement>,\n    target_refs: Vec<ExactAuthorityRef>,\n    source_refs: Vec<ExactAuthorityRef>,",
    "intent target field",
)

s = replace_once(
    s,
    "impl ConstructionIntentManifest {\n    #[allow(clippy::too_many_arguments)]\n    pub fn new(\n        intent_id: StableId,\n        revision: u64,\n        proposer_id: StableId,\n        site_id: Option<StableId>,\n        authoring_frame_ref: ExactAuthorityRef,\n        mut parent_refs: Vec<ConstructionIntentRef>,\n        mut elements: Vec<IntentElement>,\n        mut source_refs: Vec<ExactAuthorityRef>,\n    ) -> Result<Self, ProposalError> {",
    "impl ConstructionIntentManifest {\n    #[allow(clippy::too_many_arguments)]\n    pub fn new(\n        intent_id: StableId,\n        revision: u64,\n        proposer_id: StableId,\n        site_id: Option<StableId>,\n        authoring_frame_ref: ExactAuthorityRef,\n        parent_refs: Vec<ConstructionIntentRef>,\n        elements: Vec<IntentElement>,\n        source_refs: Vec<ExactAuthorityRef>,\n    ) -> Result<Self, ProposalError> {\n        Self::new_with_targets(\n            intent_id,\n            revision,\n            proposer_id,\n            site_id,\n            authoring_frame_ref,\n            parent_refs,\n            elements,\n            Vec::new(),\n            source_refs,\n        )\n    }\n\n    #[allow(clippy::too_many_arguments)]\n    pub fn new_with_targets(\n        intent_id: StableId,\n        revision: u64,\n        proposer_id: StableId,\n        site_id: Option<StableId>,\n        authoring_frame_ref: ExactAuthorityRef,\n        mut parent_refs: Vec<ConstructionIntentRef>,\n        mut elements: Vec<IntentElement>,\n        mut target_refs: Vec<ExactAuthorityRef>,\n        mut source_refs: Vec<ExactAuthorityRef>,\n    ) -> Result<Self, ProposalError> {",
    "intent constructor split",
)

s = replace_once(
    s,
    "        validate_bounded_len(\"intent.elements\", elements.len(), MAX_INTENT_ELEMENTS)?;\n        validate_bounded_len(\"intent.source_refs\", source_refs.len(), MAX_EXACT_REFS)?;\n        parent_refs.sort_by(compare_intent_ref_identity);\n        elements.sort_by(|left, right| left.element_id.cmp(&right.element_id));\n        source_refs.sort_by(compare_exact_ref_identity);",
    "        validate_bounded_len(\"intent.elements\", elements.len(), MAX_INTENT_ELEMENTS)?;\n        validate_bounded_len(\"intent.target_refs\", target_refs.len(), MAX_EXACT_REFS)?;\n        validate_bounded_len(\"intent.source_refs\", source_refs.len(), MAX_EXACT_REFS)?;\n        parent_refs.sort_by(compare_intent_ref_identity);\n        elements.sort_by(|left, right| left.element_id.cmp(&right.element_id));\n        target_refs.sort_by(compare_exact_ref_identity);\n        source_refs.sort_by(compare_exact_ref_identity);",
    "intent constructor canonicalization",
)

s = replace_once(
    s,
    "            parent_refs,\n            elements,\n            source_refs,",
    "            parent_refs,\n            elements,\n            target_refs,\n            source_refs,",
    "intent constructor fields",
)

s = replace_once(
    s,
    "    pub fn elements(&self) -> &[IntentElement] {\n        &self.elements\n    }\n\n    pub fn source_refs(&self) -> &[ExactAuthorityRef] {",
    "    pub fn elements(&self) -> &[IntentElement] {\n        &self.elements\n    }\n\n    pub fn target_refs(&self) -> &[ExactAuthorityRef] {\n        &self.target_refs\n    }\n\n    pub fn source_refs(&self) -> &[ExactAuthorityRef] {",
    "intent target accessor",
)

s = replace_once(
    s,
    "        validate_bounded_len(\"intent.elements\", self.elements.len(), MAX_INTENT_ELEMENTS)?;\n        if self.elements.is_empty() {\n            return Err(ProposalError::IntentElementsRequired);\n        }",
    "        validate_bounded_len(\"intent.elements\", self.elements.len(), MAX_INTENT_ELEMENTS)?;",
    "remove creation-only intent invariant",
)

s = replace_once(
    s,
    "        validate_bounded_len(\"intent.source_refs\", self.source_refs.len(), MAX_EXACT_REFS)?;\n        validate_exact_ref_slice(\"intent.source_refs\", &self.source_refs)",
    "        validate_bounded_len(\"intent.target_refs\", self.target_refs.len(), MAX_EXACT_REFS)?;\n        validate_exact_ref_slice(\"intent.target_refs\", &self.target_refs)?;\n        if self.elements.is_empty() && self.target_refs.is_empty() {\n            return Err(ProposalError::IntentSubjectsRequired);\n        }\n\n        validate_bounded_len(\"intent.source_refs\", self.source_refs.len(), MAX_EXACT_REFS)?;\n        validate_exact_ref_slice(\"intent.source_refs\", &self.source_refs)",
    "intent target validation",
)

# Parent identity includes digest, enabling explicit same-revision fork merges.
s = replace_once(
    s,
    "    (&left.intent_id, left.revision).cmp(&(&right.intent_id, right.revision))",
    "    (&left.intent_id, left.revision, &left.content_digest).cmp(&(\n        &right.intent_id,\n        right.revision,\n        &right.content_digest,\n    ))",
    "content-addressed parent ordering",
)

# Plan captures exact target scope from the intent.
s = replace_once(
    s,
    "    intent_element_ids: Vec<StableId>,\n    /// Exact compiler/planner implementation/profile identity.",
    "    intent_element_ids: Vec<StableId>,\n    intent_target_refs: Vec<ExactAuthorityRef>,\n    /// Exact compiler/planner implementation/profile identity.",
    "plan target snapshot field",
)

s = replace_once(
    s,
    "        let manifest = Self {\n            schema_version: PLAYER_BUILDING_PROPOSAL_SCHEMA_VERSION,\n            plan_id,\n            revision,\n            intent_ref: intent.exact_ref(),\n            intent_element_ids,\n            compiler_ref,",
    "        let intent_target_refs = intent.manifest().target_refs().to_vec();\n        let manifest = Self {\n            schema_version: PLAYER_BUILDING_PROPOSAL_SCHEMA_VERSION,\n            plan_id,\n            revision,\n            intent_ref: intent.exact_ref(),\n            intent_element_ids,\n            intent_target_refs,\n            compiler_ref,",
    "plan target snapshot construction",
)

s = replace_once(
    s,
    "    pub fn intent_element_ids(&self) -> &[StableId] {\n        &self.intent_element_ids\n    }\n\n    pub fn operations(&self) -> &[PlannedOperation] {",
    "    pub fn intent_element_ids(&self) -> &[StableId] {\n        &self.intent_element_ids\n    }\n\n    pub fn intent_target_refs(&self) -> &[ExactAuthorityRef] {\n        &self.intent_target_refs\n    }\n\n    pub fn operations(&self) -> &[PlannedOperation] {",
    "plan target accessor",
)

s = replace_once(
    s,
    "        if self.intent_element_ids.is_empty() {\n            return Err(ProposalError::IntentElementsRequired);\n        }\n        validate_bounded_len(",
    "        validate_bounded_len(",
    "remove plan creation-only invariant",
)

needle = "        validate_bounded_len(\"plan.operations\", self.operations.len(), MAX_PLAN_OPERATIONS)?;"
s = replace_once(
    s,
    needle,
    "        validate_bounded_len(\"plan.intent_target_refs\", self.intent_target_refs.len(), MAX_EXACT_REFS)?;\n        validate_exact_ref_slice(\"plan.intent_target_refs\", &self.intent_target_refs)?;\n        if self.intent_element_ids.is_empty() && self.intent_target_refs.is_empty() {\n            return Err(ProposalError::IntentSubjectsRequired);\n        }\n\n" + needle,
    "plan target validation",
)

# Actions modifying existing state must remain inside the exact target closure.
s = replace_once(
    s,
    "    JoinElements {\n        first_element_id: StableId,\n        second_element_id: StableId,\n        connection_kind: StableId,\n    },\n    ModifyExactSubject {",
    "    JoinElements {\n        first_element_id: StableId,\n        second_element_id: StableId,\n        connection_kind: StableId,\n    },\n    JoinElementToExactSubject {\n        element_id: StableId,\n        target: ExactAuthorityRef,\n        connection_kind: StableId,\n    },\n    ModifyExactSubject {",
    "new-to-existing typed action",
)

s = replace_once(
    s,
    "impl ConstructionAction {\n    fn validate(&self, intent_element_ids: &BTreeSet<StableId>) -> Result<(), ProposalError> {",
    "impl ConstructionAction {\n    fn validate(\n        &self,\n        intent_element_ids: &BTreeSet<StableId>,\n        intent_target_refs: &[ExactAuthorityRef],\n    ) -> Result<(), ProposalError> {",
    "action target-aware validation signature",
)

s = replace_once(
    s,
    "            Self::ModifyExactSubject {\n                target,",
    "            Self::JoinElementToExactSubject {\n                element_id,\n                target,\n                connection_kind,\n            } => {\n                validate_stable_id(element_id)?;\n                target.validate()?;\n                validate_stable_id(connection_kind)?;\n                ensure_intent_element(intent_element_ids, element_id)?;\n                ensure_intent_target(intent_target_refs, target)\n            }\n            Self::ModifyExactSubject {\n                target,",
    "new-to-existing validation",
)

s = replace_once(
    s,
    "                target.validate()?;\n                validate_stable_id(modification_kind)?;\n                if let Some(payload_ref) = payload_ref {",
    "                target.validate()?;\n                validate_stable_id(modification_kind)?;\n                ensure_intent_target(intent_target_refs, target)?;\n                if let Some(payload_ref) = payload_ref {",
    "modify target closure",
)

s = replace_once(
    s,
    "                target.validate()?;\n                validate_stable_id(removal_kind)\n            }",
    "                target.validate()?;\n                validate_stable_id(removal_kind)?;\n                ensure_intent_target(intent_target_refs, target)\n            }",
    "remove target closure",
)

s = replace_once(
    s,
    "            operation.action.validate(&element_ids)?;",
    "            operation\n                .action\n                .validate(&element_ids, &self.intent_target_refs)?;",
    "plan passes target scope",
)

s = replace_once(
    s,
    "                }\n            }\n        }\n\n        topological_order(&self.operations).map(|_| ())",
    "                }\n                ConstructionAction::JoinElementToExactSubject { element_id, .. } => {\n                    let realization = realization_by_element.get(element_id).ok_or_else(|| {\n                        ProposalError::JoinElementNotRealized(element_id.clone())\n                    })?;\n                    if operation.depends_on().binary_search(realization).is_err() {\n                        return Err(ProposalError::JoinMissingRealizationDependency {\n                            join_operation_id: operation.operation_id.clone(),\n                            realization_operation_id: realization.clone(),\n                        });\n                    }\n                }\n                _ => {}\n            }\n        }\n\n        topological_order(&self.operations).map(|_| ())",
    "new-to-existing realization dependency",
)

s = replace_once(
    s,
    "        if self.manifest.intent_element_ids != expected_ids {\n            return Err(ProposalError::IntentElementSetMismatch);\n        }\n        Ok(())",
    "        if self.manifest.intent_element_ids != expected_ids {\n            return Err(ProposalError::IntentElementSetMismatch);\n        }\n        if self.manifest.intent_target_refs != intent.manifest().target_refs() {\n            return Err(ProposalError::IntentTargetSetMismatch);\n        }\n        Ok(())",
    "plan replay target snapshot",
)

s = replace_once(
    s,
    "fn topological_order(operations: &[PlannedOperation]) -> Result<Vec<StableId>, ProposalError> {",
    "fn ensure_intent_target(\n    intent_target_refs: &[ExactAuthorityRef],\n    target: &ExactAuthorityRef,\n) -> Result<(), ProposalError> {\n    if intent_target_refs.iter().any(|candidate| candidate == target) {\n        Ok(())\n    } else {\n        Err(ProposalError::UnknownIntentTarget(target.clone()))\n    }\n}\n\nfn topological_order(operations: &[PlannedOperation]) -> Result<Vec<StableId>, ProposalError> {",
    "target closure helper",
)

s = replace_once(
    s,
    "    IntentElementsRequired,\n    DuplicateElement(StableId),",
    "    IntentElementsRequired,\n    IntentSubjectsRequired,\n    DuplicateElement(StableId),",
    "intent subject error",
)

s = replace_once(
    s,
    "    UnknownIntentElement(StableId),\n    DuplicateRealization(StableId),",
    "    UnknownIntentElement(StableId),\n    UnknownIntentTarget(ExactAuthorityRef),\n    DuplicateRealization(StableId),",
    "unknown target error",
)

s = replace_once(
    s,
    "    IntentElementSetMismatch,\n    DuplicateFinding(StableId),",
    "    IntentElementSetMismatch,\n    IntentTargetSetMismatch,\n    DuplicateFinding(StableId),",
    "target snapshot mismatch error",
)

s = replace_once(
    s,
    "            Self::IntentElementsRequired => {\n                write!(formatter, \"construction intent requires at least one authored element\")\n            }",
    "            Self::IntentElementsRequired => {\n                write!(formatter, \"construction intent requires at least one authored element\")\n            }\n            Self::IntentSubjectsRequired => write!(\n                formatter,\n                \"construction intent requires an authored element or exact existing target\"\n            ),",
    "intent subject display",
)

s = replace_once(
    s,
    "            Self::UnknownIntentElement(id) => {\n                write!(formatter, \"planned action references unknown intent element {id}\")\n            }",
    "            Self::UnknownIntentElement(id) => {\n                write!(formatter, \"planned action references unknown intent element {id}\")\n            }\n            Self::UnknownIntentTarget(target) => write!(\n                formatter,\n                \"planned action references undeclared exact intent target {}/{}@{}\",\n                target.authority_id, target.subject_id, target.revision\n            ),",
    "unknown target display",
)

s = replace_once(
    s,
    "            Self::IntentElementSetMismatch => {\n                write!(formatter, \"plan intent-element snapshot differs from supplied intent\")\n            }",
    "            Self::IntentElementSetMismatch => {\n                write!(formatter, \"plan intent-element snapshot differs from supplied intent\")\n            }\n            Self::IntentTargetSetMismatch => {\n                write!(formatter, \"plan intent-target snapshot differs from supplied intent\")\n            }",
    "target mismatch display",
)

LIB.write_text(s)

TEST.parent.mkdir(parents=True, exist_ok=True)
TEST.write_text(r'''// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use symtropy_game_state::StableId;
use symtropy_player_building::*;

fn id(value: &str) -> StableId { StableId::parse(value).unwrap() }
fn digest(value: &str) -> ProposalDigest { ProposalDigest::new(id("sha256"), value).unwrap() }
fn exact(authority: &str, subject: &str, revision: u64, value: &str) -> ExactAuthorityRef {
    ExactAuthorityRef::new(id(authority), id(subject), revision, digest(value)).unwrap()
}
fn frame() -> ExactAuthorityRef { exact("authority:frame", "frame:local", 1, "frame-a") }
fn compiler() -> ExactAuthorityRef { exact("authority:planner", "planner:pb01", 1, "compiler-a") }
fn target(value: &str) -> ExactAuthorityRef {
    exact("authority:construction", "structure:shelter", 7, value)
}
fn material() -> MaterialIntent { MaterialIntent::new(id("material:timber"), Vec::new()).unwrap() }
fn element() -> IntentElement {
    IntentElement::new(
        id("element:new"), id("role:frame"), AuthoringPose::IDENTITY,
        GeometryIntent::Beam { length_um: 1_000_000, cross_section_um: [10_000, 10_000] },
        material(),
    ).unwrap()
}
fn context() -> PlanningContext { PlanningContext::new(id("context:test"), 1, Vec::new()).unwrap() }

#[test]
fn repair_only_intent_is_valid_but_empty_intent_is_not() {
    let repair = ConstructionIntentManifest::new_with_targets(
        id("intent:repair"), 1, id("actor:player"), None, frame(),
        Vec::new(), Vec::new(), vec![target("state-a")], Vec::new(),
    );
    assert!(repair.is_ok());

    let empty = ConstructionIntentManifest::new_with_targets(
        id("intent:empty"), 1, id("actor:player"), None, frame(),
        Vec::new(), Vec::new(), Vec::new(), Vec::new(),
    );
    assert!(matches!(empty, Err(ProposalError::IntentSubjectsRequired)));
}

#[test]
fn planner_cannot_widen_exact_target_scope() {
    let intent = ConstructionIntent::seal(ConstructionIntentManifest::new_with_targets(
        id("intent:repair"), 1, id("actor:player"), None, frame(),
        Vec::new(), Vec::new(), vec![target("state-a")], Vec::new(),
    ).unwrap()).unwrap();

    let op = PlannedOperation::new(
        id("op:repair"), Vec::new(), ConstructionAction::ModifyExactSubject {
            target: target("state-b"), modification_kind: id("modify:repair"), payload_ref: None,
        },
    ).unwrap();
    let result = ConstructionPlanManifest::new(
        id("plan:repair"), 1, &intent, compiler(), context(), vec![op],
    );
    assert!(matches!(result, Err(ProposalError::UnknownIntentTarget(_))));
}

#[test]
fn new_to_existing_join_requires_declared_target_and_realization_dependency() {
    let existing = target("state-a");
    let intent = ConstructionIntent::seal(ConstructionIntentManifest::new_with_targets(
        id("intent:addition"), 1, id("actor:player"), None, frame(),
        Vec::new(), vec![element()], vec![existing.clone()], Vec::new(),
    ).unwrap()).unwrap();
    let realize = PlannedOperation::new(
        id("op:realize"), Vec::new(), ConstructionAction::RealizeElement { element_id: id("element:new") },
    ).unwrap();
    let join = PlannedOperation::new(
        id("op:join"), vec![id("op:realize")], ConstructionAction::JoinElementToExactSubject {
            element_id: id("element:new"), target: existing, connection_kind: id("join:fastener"),
        },
    ).unwrap();
    assert!(ConstructionPlanManifest::new(
        id("plan:addition"), 1, &intent, compiler(), context(), vec![join, realize],
    ).is_ok());
}

#[test]
fn two_content_distinct_same_revision_parents_can_merge() {
    let a = ConstructionIntentRef {
        intent_id: id("intent:fork"), revision: 2, content_digest: digest("fork-a"),
    };
    let b = ConstructionIntentRef {
        intent_id: id("intent:fork"), revision: 2, content_digest: digest("fork-b"),
    };
    let left = ConstructionIntent::seal(ConstructionIntentManifest::new(
        id("intent:fork"), 3, id("actor:player"), None, frame(),
        vec![a.clone(), b.clone()], vec![element()], Vec::new(),
    ).unwrap()).unwrap();
    let right = ConstructionIntent::seal(ConstructionIntentManifest::new(
        id("intent:fork"), 3, id("actor:player"), None, frame(),
        vec![b, a], vec![element()], Vec::new(),
    ).unwrap()).unwrap();
    assert_eq!(left.exact_ref(), right.exact_ref());
}
''')

print(f"updated {LIB.relative_to(ROOT)}")
print(f"wrote {TEST.relative_to(ROOT)}")
