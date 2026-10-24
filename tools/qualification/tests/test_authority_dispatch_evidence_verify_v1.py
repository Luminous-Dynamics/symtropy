from __future__ import annotations
import hashlib, json, tempfile, unittest
from pathlib import Path
from tools.qualification import authority_dispatch_evidence_verify_v1 as verify

PAYLOAD = {"schema_id":"luminous.authority-dispatch.v1","schema_version":1,"contract_commit_sha":"3"*40,"contract_path":"tools/qualification/contract.json","contract_sha256":"4"*64}

def raw_payload():
    return json.dumps(PAYLOAD, sort_keys=True, separators=(",", ":")).encode()
def evidence_for(raw):
    return {"schema_id":"luminous.authority-dispatch-evidence.v1","schema_version":1,"dispatch_schema_id":"luminous.authority-dispatch.v1","dispatch_sha256":hashlib.sha256(raw).hexdigest(),"release_sha256":"2"*64,"contract_commit_sha":PAYLOAD["contract_commit_sha"],"contract_path":PAYLOAD["contract_path"],"contract_sha256":PAYLOAD["contract_sha256"]}

class AuthorityDispatchEvidenceVerifyV1Tests(unittest.TestCase):
    def test_exact_payload_binding(self):
        with tempfile.TemporaryDirectory() as d:
            root=Path(d); raw=raw_payload(); (root/"payload.json").write_bytes(raw); (root/"evidence.json").write_text(json.dumps(evidence_for(raw)),encoding="utf-8")
            self.assertEqual(verify.main(["--payload",str(root/"payload.json"),"--evidence",str(root/"evidence.json")]),0)
    def test_tampered_payload_is_rejected(self):
        with tempfile.TemporaryDirectory() as d:
            root=Path(d); raw=raw_payload(); (root/"payload.json").write_bytes(raw); ev=evidence_for(raw); tampered=dict(PAYLOAD); tampered["contract_path"]="other.json"; (root/"payload.json").write_bytes(json.dumps(tampered,sort_keys=True,separators=(",",":")).encode()); (root/"evidence.json").write_text(json.dumps(ev),encoding="utf-8")
            self.assertEqual(verify.main(["--payload",str(root/"payload.json"),"--evidence",str(root/"evidence.json")]),2)
    def test_tampered_dispatch_digest_is_rejected(self):
        with tempfile.TemporaryDirectory() as d:
            root=Path(d); raw=raw_payload(); (root/"payload.json").write_bytes(raw); ev=evidence_for(raw); ev["dispatch_sha256"]="9"*64; (root/"evidence.json").write_text(json.dumps(ev),encoding="utf-8")
            self.assertEqual(verify.main(["--payload",str(root/"payload.json"),"--evidence",str(root/"evidence.json")]),2)

if __name__ == "__main__": unittest.main()