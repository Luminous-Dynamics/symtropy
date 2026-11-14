#!/usr/bin/env python3
from pathlib import Path

path = Path("crates/apps/symtropy-firstlight/src/patch_conduit_exact_plan.rs")
text = path.read_text()
if "patch_conduit_executable_identity_v1_goldens" in text:
    raise SystemExit("Patch Conduit executable identity golden test already present")

marker = "\n}\n"
position = text.rfind(marker)
if position == -1:
    raise SystemExit("final tests module closing brace not found")

golden = r'''

    #[test]
    fn patch_conduit_executable_identity_v1_goldens() {
        let expected = [
            (
                "assembly:patch-conduit:degraded-hose",
                "fabrication-plan:firstlight:patch-conduit:degraded-bypass",
                2_u64,
                "aa6902076be2cc4c61f637b774873ea8eb1106b1a45d4af2b6d6355ef19aadf2",
            ),
            (
                "assembly:patch-conduit:emergency-bypass",
                "fabrication-plan:firstlight:patch-conduit:serviceable-bypass",
                2_u64,
                "57db32de10648d084aefee6b8c99635b0fadba1cf246ed6201dd3653ac72b477",
            ),
            (
                "assembly:patch-conduit:salvaged-sleeve",
                "fabrication-plan:firstlight:patch-conduit:salvaged-sleeve",
                2_u64,
                "cdf91f4e7ffc0bea27f2505fdf6a2ed8f2f8919fe8d29091d3197caed2a35f75",
            ),
            (
                "assembly:patch-conduit:standard-banded",
                "fabrication-plan:firstlight:patch-conduit:standard-banded",
                2_u64,
                "682d12ee72c108ee4552f934d1035a8f9bec9f19deedc7206aecedf90a2ed38d",
            ),
        ];

        let scenario = PatchConduitScenario::canonical().unwrap();
        let exact = ExactPatchConduitProfile::compile(&scenario).unwrap();
        assert_eq!(exact.approaches().len(), expected.len());

        for (subject, plan_id, revision, digest) in expected {
            let approach = exact
                .approaches()
                .iter()
                .find(|approach| approach.subject.id.as_str() == subject)
                .unwrap_or_else(|| panic!("missing canonical Patch Conduit approach {subject}"));

            assert_eq!(approach.plan.plan().id.stable_id().as_str(), plan_id);
            assert_eq!(approach.plan.plan().revision, revision);
            assert_eq!(approach.plan.content_digest().to_hex(), digest);

            let exact_ref = approach.plan.content_ref();
            exact_ref.validate_against(&approach.plan).unwrap();
        }
    }
'''

path.write_text(text[:position] + golden + text[position:])
