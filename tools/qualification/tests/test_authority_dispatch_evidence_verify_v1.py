from __future__ import annotations
import hashlib, json, tempfile, unittest
from pathlib import Path
from tools.qualification import authority_dispatch_evidence_verify_v1 as verify
from tools.qualification import verifier_release_v1 as release

CONTRACT={
    "schema_id":"luminous.qualification-contract.v1",
    "contract_id":"qual-001b-fixture",
    "contract_version":1,
    "subject_repository":"Luminous-Dynamics/symtropy",
    "subject_head_sha":"5"*40,
    "subject_tree_sha":"6"*40,
    "manifest_path":"tools/qualification/manifest.json",
    "manifest_sha256":"7"*64,
    "suite_profile_path":"tools/qualification/profile.json",
    "suite_profile_sha256":"8"*64,
}
def raw_contract(): return json.dumps(CONTRACT,sort_keys=True,separators=(",",":")).encode()
CONTRACT_SHA256=hashlib.sha256(raw_contract()).hexdigest()
PAYLOAD={"schema_id":"luminous.authority-dispatch.v1","schema_version":1,"contract_commit_sha":"3"*40,"contract_path":"tools/qualification/contract.json","contract_sha256":CONTRACT_SHA256}
RELEASE={"schema_id":release.SCHEMA_ID,"release_id":"qual-001b-verifier-v1-r2","release_version":1,"verifier_repository":"Luminous-Dynamics/symtropy","verifier_commit_sha":"1"*40,"verifier_tree_sha":"2"*40,"verifier_profile_id":"qual-001b-authoritative-verifier-v1","verifier_profile_sha256":"3"*64,"contract_schema_id":"luminous.qualification-contract.v1","contract_schema_version":1,"suite_id":"rust-workspace-package-v1","suite_revision":"rust-workspace-package-v1/1","toolchain_id":"rust-1.96.0"}
def raw_payload(): return json.dumps(PAYLOAD,sort_keys=True,separators=(",",":")).encode()
def raw_release(): return json.dumps(RELEASE,sort_keys=True,separators=(",",":")).encode()
def evidence_for(raw,rr=None):
    if rr is None: rr=raw_release()
    return {"schema_id":"luminous.authority-dispatch-evidence.v1","schema_version":1,"dispatch_schema_id":"luminous.authority-dispatch.v1","dispatch_sha256":hashlib.sha256(raw).hexdigest(),"release_sha256":hashlib.sha256(rr).hexdigest(),"contract_commit_sha":PAYLOAD["contract_commit_sha"],"contract_path":PAYLOAD["contract_path"],"contract_sha256":PAYLOAD["contract_sha256"]}

class AuthorityDispatchEvidenceVerifyV1Tests(unittest.TestCase):
    def run_verify(self,root):
        return verify.main(["--payload",str(root/"payload.json"),"--evidence",str(root/"evidence.json"),"--release",str(root/"release.json"),"--contract",str(root/"contract.json")])
    def write_valid(self,root):
        raw=raw_payload(); rr=raw_release()
        (root/"payload.json").write_bytes(raw); (root/"release.json").write_bytes(rr); (root/"contract.json").write_bytes(raw_contract())
        (root/"evidence.json").write_text(json.dumps(evidence_for(raw,rr)),encoding="utf-8")
    def test_exact_binding(self):
        with tempfile.TemporaryDirectory() as d:
            root=Path(d); self.write_valid(root); self.assertEqual(self.run_verify(root),0)
    def test_tampered_payload(self):
        with tempfile.TemporaryDirectory() as d:
            root=Path(d); self.write_valid(root); bad=dict(PAYLOAD); bad["contract_path"]="other.json"; (root/"payload.json").write_bytes(json.dumps(bad,sort_keys=True,separators=(",",":")).encode()); self.assertEqual(self.run_verify(root),2)
    def test_tampered_dispatch_digest(self):
        with tempfile.TemporaryDirectory() as d:
            root=Path(d); self.write_valid(root); ev=evidence_for(raw_payload()); ev["dispatch_sha256"]="9"*64; (root/"evidence.json").write_text(json.dumps(ev),encoding="utf-8"); self.assertEqual(self.run_verify(root),2)
    def test_tampered_release_bytes(self):
        with tempfile.TemporaryDirectory() as d:
            root=Path(d); self.write_valid(root); (root/"release.json").write_bytes(raw_release()+b" "); self.assertEqual(self.run_verify(root),2)
    def test_release_digest_substitution(self):
        with tempfile.TemporaryDirectory() as d:
            root=Path(d); self.write_valid(root); ev=evidence_for(raw_payload()); ev["release_sha256"]="9"*64; (root/"evidence.json").write_text(json.dumps(ev),encoding="utf-8"); self.assertEqual(self.run_verify(root),2)
    def test_tampered_contract_bytes(self):
        with tempfile.TemporaryDirectory() as d:
            root=Path(d); self.write_valid(root); (root/"contract.json").write_bytes(raw_contract()+b" "); self.assertEqual(self.run_verify(root),2)
    def test_contract_digest_substitution(self):
        with tempfile.TemporaryDirectory() as d:
            root=Path(d); self.write_valid(root); ev=evidence_for(raw_payload()); ev["contract_sha256"]="9"*64; (root/"evidence.json").write_text(json.dumps(ev),encoding="utf-8"); self.assertEqual(self.run_verify(root),2)
    def test_invalid_contract_rejected(self):
        with tempfile.TemporaryDirectory() as d:
            root=Path(d); self.write_valid(root); bad=dict(CONTRACT); del bad["subject_head_sha"]; (root/"contract.json").write_text(json.dumps(bad),encoding="utf-8"); self.assertEqual(self.run_verify(root),2)

if __name__=="__main__": unittest.main()
