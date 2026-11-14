from __future__ import annotations
import json, unittest
from tools.qualification import authority_dispatch_v1 as dispatch
BASE={"schema_id":dispatch.SCHEMA_ID,"schema_version":1,"contract_commit_sha":"1"*40,"contract_path":"tools/qualification/contract.json","contract_sha256":"2"*64}
class AuthorityDispatchV1Tests(unittest.TestCase):
    def test_valid(self): self.assertEqual(dispatch.validate(dict(BASE)),BASE)
    def test_unknown_field_rejected(self):
        bad=dict(BASE); bad["verifier_ref"]="refs/heads/main"
        with self.assertRaisesRegex(ValueError,"exactly"): dispatch.validate(bad)
    def test_missing_field_rejected(self):
        bad=dict(BASE); del bad["contract_sha256"]
        with self.assertRaisesRegex(ValueError,"exactly"): dispatch.validate(bad)
    def test_attacker_verifier_identity_is_not_accepted(self):
        for key in ("verifier_ref","verifier_commit_sha","verifier_tree_sha","verifier_repository","suite_id","toolchain_id"):
            bad=dict(BASE); bad[key]="attacker-controlled"
            with self.assertRaises(ValueError): dispatch.validate(bad)
    def test_path_traversal_rejected(self):
        for value in ("../contract.json","a/../contract.json","/etc/passwd","\\evil\\contract.json",""):
            bad=dict(BASE); bad["contract_path"]=value
            with self.assertRaises(ValueError): dispatch.validate(bad)
    def test_invalid_ids_rejected(self):
        for key,value in (("contract_commit_sha","a"*39),("contract_sha256","b"*63)):
            bad=dict(BASE); bad[key]=value
            with self.assertRaises(ValueError): dispatch.validate(bad)
    def test_duplicate_key_rejected(self):
        raw=json.dumps(BASE,separators=(",",":")).replace('"schema_version":1,','"schema_version":1,"schema_version":2,').encode()
        with self.assertRaisesRegex(ValueError,"duplicate"): dispatch.load_bytes(raw)
    def test_digest_is_exact_payload_bytes(self):
        raw=json.dumps(BASE,sort_keys=True,separators=(",",":")).encode()
        value,digest=dispatch.load_bytes(raw)
        import hashlib
        self.assertEqual(digest,hashlib.sha256(raw).hexdigest()); self.assertEqual(value,BASE)
    def test_oversized_payload_rejected(self):
        with self.assertRaisesRegex(ValueError,"exceeds"): dispatch.load_bytes(b" "*(dispatch.MAX_BYTES+1))
if __name__=="__main__": unittest.main()
