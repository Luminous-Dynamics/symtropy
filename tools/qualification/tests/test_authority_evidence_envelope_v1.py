from __future__ import annotations
import hashlib,json,tempfile,unittest
from pathlib import Path
from tools.qualification import authority_evidence_envelope_v1 as verify
from tools.qualification import verifier_release_v1 as release
CONTRACT={"schema_id":"luminous.qualification-contract.v1","contract_id":"qual-001b-fixture","contract_version":1,"subject_repository":"Luminous-Dynamics/symtropy","subject_head_sha":"5"*40,"subject_tree_sha":"6"*40,"manifest_path":"tools/qualification/manifest.json","manifest_sha256":"7"*64,"suite_profile_path":"tools/qualification/profile.json","suite_profile_sha256":"8"*64}
RELEASE={"schema_id":release.SCHEMA_ID,"release_id":"qual-001b-verifier-v1-r2","release_version":1,"verifier_repository":"Luminous-Dynamics/symtropy","verifier_commit_sha":"1"*40,"verifier_tree_sha":"2"*40,"verifier_profile_id":"qual-001b-authoritative-verifier-v1","verifier_profile_sha256":"3"*64,"contract_schema_id":"luminous.qualification-contract.v1","contract_schema_version":1,"suite_id":"rust-workspace-package-v1","suite_revision":"rust-workspace-package-v1/1","toolchain_id":"rust-1.96.0"}
def raw(v): return json.dumps(v,sort_keys=True,separators=(",",":")).encode()
def write(root,result="TheoremExecutedPass"):
    cr=raw(CONTRACT); rr=raw(RELEASE)
    payload={"schema_id":"luminous.authority-dispatch.v1","schema_version":1,"contract_commit_sha":"3"*40,"contract_path":"tools/qualification/contract.json","contract_sha256":hashlib.sha256(cr).hexdigest()}
    pr=raw(payload)
    de={"schema_id":"luminous.authority-dispatch-evidence.v1","schema_version":1,"dispatch_schema_id":"luminous.authority-dispatch.v1","dispatch_sha256":hashlib.sha256(pr).hexdigest(),"release_sha256":hashlib.sha256(rr).hexdigest(),"contract_commit_sha":payload["contract_commit_sha"],"contract_path":payload["contract_path"],"contract_sha256":payload["contract_sha256"]}
    blob_sha=hashlib.sha1(b"blob "+str(len(cr)).encode()+b"\0"+cr).hexdigest()
    checkout={"schema_id":"luminous.qualification-contract-checkout-identity.v1","schema_version":1,"contract_commit_sha":payload["contract_commit_sha"],"contract_tree_sha":"a"*40,"contract_blob_sha":blob_sha}
    ex={"schema_id":"luminous.qualification-execution-evidence.v1","verifier_commit_sha":RELEASE["verifier_commit_sha"],"verifier_tree_sha":RELEASE["verifier_tree_sha"],"contract_commit_sha":payload["contract_commit_sha"],"contract_tree_sha":"a"*40,"contract_sha256":hashlib.sha256(cr).hexdigest(),"contract_id":CONTRACT["contract_id"],"subject_repository":CONTRACT["subject_repository"],"subject_head_sha":CONTRACT["subject_head_sha"],"subject_tree_sha":CONTRACT["subject_tree_sha"],"manifest_sha256":CONTRACT["manifest_sha256"],"manifest_profile_id":"profile","suite_profile_sha256":CONTRACT["suite_profile_sha256"],"suite_id":RELEASE["suite_id"],"suite_revision":RELEASE["suite_revision"],"toolchain_id":RELEASE["toolchain_id"],"expanded_step_ids":[],"executed_step_ids":[],"steps":{},"first_failing_step_id":None,"first_failing_step_result":None,"final_result":result}
    (root/"payload.json").write_bytes(pr); (root/"dispatch-evidence.json").write_text(json.dumps(de)); (root/"release.json").write_bytes(rr); (root/"contract.json").write_bytes(cr); (root/"contract-checkout.json").write_text(json.dumps(checkout)); (root/"execution.json").write_text(json.dumps(ex))
class T(unittest.TestCase):
    def runv(self,r): return verify.main(["--payload",str(r/"payload.json"),"--dispatch-evidence",str(r/"dispatch-evidence.json"),"--release",str(r/"release.json"),"--contract",str(r/"contract.json"),"--execution-evidence",str(r/"execution.json"),"--contract-checkout-identity",str(r/"contract-checkout.json")])
    def test_complete_pass(self):
        with tempfile.TemporaryDirectory() as d: r=Path(d); write(r); self.assertEqual(self.runv(r),0)
    def test_tampered_contract(self):
        with tempfile.TemporaryDirectory() as d: r=Path(d); write(r); (r/"contract.json").write_bytes(raw(CONTRACT)+b" "); self.assertEqual(self.runv(r),2)
    def test_tampered_execution_identity(self):
        with tempfile.TemporaryDirectory() as d: r=Path(d); write(r); e=json.loads((r/"execution.json").read_text()); e["subject_head_sha"]="9"*40; (r/"execution.json").write_text(json.dumps(e)); self.assertEqual(self.runv(r),2)
    def test_checkout_identity_requires_blob_field(self):
        with tempfile.TemporaryDirectory() as d:
            r=Path(d); write(r); c=json.loads((r/"contract-checkout.json").read_text()); del c["contract_blob_sha"]; (r/"contract-checkout.json").write_text(json.dumps(c)); self.assertEqual(self.runv(r),2)

    def test_checkout_identity_rejects_unknown_field(self):
        with tempfile.TemporaryDirectory() as d:
            r=Path(d); write(r); c=json.loads((r/"contract-checkout.json").read_text()); c["unexpected"]="x"; (r/"contract-checkout.json").write_text(json.dumps(c)); self.assertEqual(self.runv(r),2)

    def test_checkout_tree_must_match_execution_identity(self):
        with tempfile.TemporaryDirectory() as d:
            r=Path(d); write(r)
            e=json.loads((r/"execution.json").read_text())
            e["contract_tree_sha"]="b"*40
            (r/"execution.json").write_text(json.dumps(e))
            self.assertEqual(self.runv(r),2)

    def test_tampered_contract_blob_identity_rejected(self):
        with tempfile.TemporaryDirectory() as d:
            r=Path(d); write(r); c=json.loads((r/"contract-checkout.json").read_text()); c["contract_blob_sha"]="0"*40; (r/"contract-checkout.json").write_text(json.dumps(c)); self.assertEqual(self.runv(r),2)
    def test_failed_execution_is_replayable(self):
        with tempfile.TemporaryDirectory() as d: r=Path(d); write(r,"TheoremExecutedFail"); self.assertEqual(self.runv(r),0)
if __name__=="__main__": unittest.main()
