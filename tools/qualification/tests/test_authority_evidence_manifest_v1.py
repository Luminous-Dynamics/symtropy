from __future__ import annotations
import hashlib,tempfile,unittest
from pathlib import Path
from tools.qualification import authority_evidence_manifest_v1 as manifest
class ManifestTests(unittest.TestCase):
 def _set(self,root):
  for name,_ in manifest.MEMBERS: (root/name).write_bytes((name+"\n").encode())
 def _materialize(self,root):
  value=manifest.build(root); path=root/"authority-evidence-manifest-v1.json"; path.write_bytes(manifest.canonical(value)+b"\n"); return path,value
 def test_round_trip(self):
  with tempfile.TemporaryDirectory() as td:
   root=Path(td); self._set(root); path,value=self._materialize(root); loaded,digest=manifest.load(path.read_bytes()); manifest.verify(loaded,root); self.assertEqual(value["root_sha256"],loaded["root_sha256"]); self.assertEqual(digest,hashlib.sha256(path.read_bytes()).hexdigest())
 def test_tampered_member(self):
  with tempfile.TemporaryDirectory() as td:
   root=Path(td); self._set(root); path,_=self._materialize(root); (root/"verifier-release-v1.json").write_bytes(b"tampered"); loaded,_=manifest.load(path.read_bytes())
   with self.assertRaises(manifest.ManifestValidationError): manifest.verify(loaded,root)
 def test_digest_substitution(self):
  with tempfile.TemporaryDirectory() as td:
   root=Path(td); self._set(root); path,value=self._materialize(root); value["members"][0]["sha256"]="0"*64; value["root_sha256"]=hashlib.sha256(manifest.canonical({"schema_id":value["schema_id"],"schema_version":value["schema_version"],"members":value["members"]})).hexdigest(); path.write_bytes(manifest.canonical(value)+b"\n"); loaded,_=manifest.load(path.read_bytes())
   with self.assertRaises(manifest.ManifestValidationError): manifest.verify(loaded,root)
 def test_extra_member(self):
  with tempfile.TemporaryDirectory() as td:
   root=Path(td); self._set(root); path,_=self._materialize(root); (root/"unexpected.json").write_bytes(b"x"); loaded,_=manifest.load(path.read_bytes())
   with self.assertRaises(manifest.ManifestValidationError): manifest.verify(loaded,root)
 def test_reordered_members(self):
  with tempfile.TemporaryDirectory() as td:
   root=Path(td); self._set(root); path,value=self._materialize(root); value["members"]=list(reversed(value["members"])); value["root_sha256"]=hashlib.sha256(manifest.canonical({"schema_id":value["schema_id"],"schema_version":value["schema_version"],"members":value["members"]})).hexdigest(); path.write_bytes(manifest.canonical(value)+b"\n")
   with self.assertRaises(manifest.ManifestValidationError): manifest.load(path.read_bytes())
if __name__=="__main__": unittest.main()
