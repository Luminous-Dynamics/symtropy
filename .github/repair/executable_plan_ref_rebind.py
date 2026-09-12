#!/usr/bin/env python3
from pathlib import Path


def replace_once(text: str, old: str, new: str, label: str) -> str:
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected one exact anchor, found {count}")
    return text.replace(old, new, 1)


identity_path = Path("crates/domains/symtropy-fabrication/src/executable_plan_identity.rs")
text = identity_path.read_text()

text = replace_once(
    text,
    "/// Deserialization restores only evidence-shaped data. Call\n"
    "/// [`Self::validate_against`] with the complete strong executable plan before a\n"
    "/// consequential boundary treats this reference as exact.\n",
    "/// Deserialization restores only evidence-shaped data. Call [`Self::rebind`]\n"
    "/// with the complete strong executable plan to obtain a validated reference\n"
    "/// before a consequential boundary treats this reference as exact.\n",
    "raw ref authority docs",
)

struct_anchor = '''pub struct ExecutableFabricationPlanRef {
    plan_id: FabricationPlanId,
    plan_revision: u64,
    content_digest: ExecutablePlanDigest,
}

'''
validated_type = '''pub struct ExecutableFabricationPlanRef {
    plan_id: FabricationPlanId,
    plan_revision: u64,
    content_digest: ExecutablePlanDigest,
}

/// Proof that one raw executable-plan reference has been rebound to the exact
/// complete strong plan whose canonical content identity it names.
///
/// This type is intentionally not serializable/deserializable and its fields
/// are private. Persistence restores only [`ExecutableFabricationPlanRef`]; a
/// caller must perform [`ExecutableFabricationPlanRef::rebind`] again after
/// restore before regaining this authority state.
#[derive(Debug, Clone, Copy)]
pub struct ValidatedExecutableFabricationPlanRef<'a> {
    reference: &'a ExecutableFabricationPlanRef,
    plan: &'a ExecutableFabricationPlan,
}

impl<'a> ValidatedExecutableFabricationPlanRef<'a> {
    pub const fn reference(&self) -> &'a ExecutableFabricationPlanRef {
        self.reference
    }

    pub const fn plan(&self) -> &'a ExecutableFabricationPlan {
        self.plan
    }
}

'''
text = replace_once(text, struct_anchor, validated_type, "validated ref type")

old_rebind = '''    pub fn validate_against(
        &self,
        plan: &ExecutableFabricationPlan,
    ) -> Result<(), ExecutablePlanIdentityError> {
        if self.plan_id != plan.plan().id || self.plan_revision != plan.plan().revision {
            return Err(ExecutablePlanIdentityError::PlanIdentityMismatch {
                expected_id: self.plan_id.clone(),
                expected_revision: self.plan_revision,
                actual_id: plan.plan().id.clone(),
                actual_revision: plan.plan().revision,
            });
        }
        let actual = plan.content_digest();
        if self.content_digest != actual {
            return Err(ExecutablePlanIdentityError::ContentDigestMismatch {
                expected: self.content_digest,
                actual,
            });
        }
        Ok(())
    }
'''
new_rebind = '''    pub fn rebind<'a>(
        &'a self,
        plan: &'a ExecutableFabricationPlan,
    ) -> Result<ValidatedExecutableFabricationPlanRef<'a>, ExecutablePlanIdentityError> {
        if self.plan_id != plan.plan().id || self.plan_revision != plan.plan().revision {
            return Err(ExecutablePlanIdentityError::PlanIdentityMismatch {
                expected_id: self.plan_id.clone(),
                expected_revision: self.plan_revision,
                actual_id: plan.plan().id.clone(),
                actual_revision: plan.plan().revision,
            });
        }
        let actual = plan.content_digest();
        if self.content_digest != actual {
            return Err(ExecutablePlanIdentityError::ContentDigestMismatch {
                expected: self.content_digest,
                actual,
            });
        }
        Ok(ValidatedExecutableFabricationPlanRef {
            reference: self,
            plan,
        })
    }
'''
text = replace_once(text, old_rebind, new_rebind, "validated rebind API")

text = replace_once(
    text,
    "        restored_ref.validate_against(&restored).unwrap();\n",
    "        let validated = restored_ref.rebind(&restored).unwrap();\n"
    "        assert_eq!(validated.reference(), &restored_ref);\n"
    "        assert_eq!(validated.plan(), &restored);\n",
    "round-trip rebind regression",
)
text = replace_once(
    text,
    "            reference.validate_against(&changed),\n",
    "            reference.rebind(&changed),\n",
    "changed-content rebind regression",
)

identity_test_anchor = '''    #[test]
    fn compact_ref_rejects_same_lineage_changed_exact_content() {
'''
identity_test = '''    #[test]
    fn compact_ref_rebind_rejects_different_plan_identity_before_digest() {
        let original = golden_executable(false);
        let different = golden_executable_variant(
            false,
            "fabrication-plan:digest-other",
            7,
            ProcessKind::Cut,
            "mode:cnc",
            "workpiece:b",
            "evidence:dimensional",
        );
        let reference = original.content_ref();
        assert!(matches!(
            reference.rebind(&different),
            Err(ExecutablePlanIdentityError::PlanIdentityMismatch { .. })
        ));
    }

'''
text = replace_once(
    text,
    identity_test_anchor,
    identity_test + identity_test_anchor,
    "plan identity hostile regression",
)
identity_path.write_text(text)

firstlight_path = Path("crates/apps/symtropy-firstlight/src/patch_conduit_exact_plan.rs")
firstlight = firstlight_path.read_text()
firstlight = replace_once(
    firstlight,
    "            exact_ref.validate_against(&approach.plan).unwrap();\n",
    "            let validated = exact_ref.rebind(&approach.plan).unwrap();\n"
    "            assert_eq!(validated.reference(), &exact_ref);\n"
    "            assert_eq!(validated.plan(), &approach.plan);\n",
    "Patch Conduit validated rebind",
)
firstlight_path.write_text(firstlight)
