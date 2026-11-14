from __future__ import annotations
import hashlib,json,unittest
from tools.qualification import verifier_release_v1 as release
BASE={"schema_id":release.SCHEMA_ID,"release_id":"qual-001b-verifier-v1-r2","release_version":1,"verifier_repository":"Luminous-Dynamics/symtropy","verifier_commit_sha":"1"*40,"verifier_tree_sha":"2"*40,"verifier_profile_id":"qual-001b-authoritative-verifier-v1","verifier_profile_sha256":"3"*64,"contract_schema_id":"luminous.qualification-contract.v1","contract_schema_version":1,"suite_id":"rust-workspace-package-v1","suite_revision":"rust-workspace-package-v1/1","toolchain_id":"rust-1.96.0"}
class VerifierReleaseV1Tests(unittest.TestCase):
    def test_valid_record(self): self.assertIs(release.validate_record(dict(BASE)),BASE)
    def test_exact_field_set_rejects_executable_surface(self):
        bad=dict(BASE); bad["command"]="cargo test"
        with self.assertRaisesRegex(ValueError,"unknown fields"): release.validate_record(bad)
    def test_wrong_commit_rejected(self):
        bad=dict(BASE); bad["verifier_commit_sha"]="a"*39
        with self.assertRaisesRegex(ValueError,"verifier_commit_sha"): release.validate_record(bad)
    def test_wrong_tree_rejected(self):
        with self.assertRaisesRegex(ValueError,"tree"): release.validate_checkout_identity(head_sha=BASE["verifier_commit_sha"],tree_sha="9"*40,record=dict(BASE))
    def test_wrong_commit_rejected_at_checkout(self):
        with self.assertRaisesRegex(ValueError,"HEAD"): release.validate_checkout_identity(head_sha="9"*40,tree_sha=BASE["verifier_tree_sha"],record=dict(BASE))
    def test_wrong_repository_rejected(self):
        bad=dict(BASE); bad["verifier_repository"]="attacker/example"
        with self.assertRaisesRegex(ValueError,"verifier_repository"): release.validate_record(bad)
    def test_wrong_suite_rejected(self):
        bad=dict(BASE); bad["suite_id"]="arbitrary-suite"
        with self.assertRaisesRegex(ValueError,"suite_id"): release.validate_record(bad)
    def test_wrong_toolchain_rejected(self):
        bad=dict(BASE); bad["toolchain_id"]="rust-nightly"
        with self.assertRaisesRegex(ValueError,"toolchain_id"): release.validate_record(bad)
    def test_duplicate_json_key_rejected(self):
        raw=json.dumps(BASE,separators=(",",":")).replace('"release_version":1,','"release_version":1,"release_version":2,').encode()
        with self.assertRaisesRegex(ValueError,"duplicate"): release.load_record_bytes(raw)
    def test_digest_is_exact_record_bytes(self):
        raw=json.dumps(BASE,sort_keys=True,separators=(",",":")).encode(); value,digest=release.load_record_bytes(raw)
        self.assertEqual(value,BASE); self.assertEqual(digest,hashlib.sha256(raw).hexdigest())
    def test_only_identity_data_is_allowed(self):
        for key in ("command","workflow","workflow_ref","shell","cargo_args","ref"):
            bad=dict(BASE); bad[key]="anything"
            with self.assertRaises(ValueError): release.validate_record(bad)
if __name__=="__main__": unittest.main()
