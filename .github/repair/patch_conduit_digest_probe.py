#!/usr/bin/env python3
from pathlib import Path

path = Path("crates/apps/symtropy-firstlight/src/patch_conduit_exact_plan.rs")
text = path.read_text()
marker = "\n}\n"
position = text.rfind(marker)
if position == -1:
    raise SystemExit("final tests module closing brace not found")
probe = r'''

    #[test]
    fn patch_conduit_digest_probe() {
        let scenario = PatchConduitScenario::canonical().unwrap();
        let exact = ExactPatchConduitProfile::compile(&scenario).unwrap();
        assert!(!exact.approaches().is_empty());

        for approach in exact.approaches() {
            println!(
                "PATCH_CONDUIT_DIGEST subject={} plan={} revision={} digest={}",
                approach.subject.id.as_str(),
                approach.plan.plan().id.stable_id().as_str(),
                approach.plan.plan().revision,
                approach.plan.content_digest().to_hex(),
            );
        }
    }
'''
path.write_text(text[:position] + probe + text[position:])
