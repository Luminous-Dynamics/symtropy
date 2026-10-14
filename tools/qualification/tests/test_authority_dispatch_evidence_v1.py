from __future__ import annotations
import unittest
from tools.qualification import authority_dispatch_evidence_v1 as evidence

BASE = {
    "schema_id": evidence.SCHEMA_ID,
    "schema_version": 1,
    "dispatch_schema_id": "luminous.authority-dispatch.v1",
    "dispatch_sha256": "1" * 64,
    "release_sha256": "2" * 64,
    "contract_commit_sha": "3" * 40,
    "contract_path": "tools/qualification/contract.json",
    "contract_sha256": "4" * 64,
}

class AuthorityDispatchEvidenceV1Tests(unittest.TestCase):
    def test_valid(self):
        self.assertEqual(evidence.validate(dict(BASE)), BASE)

    def test_unknown_field_rejected(self):
        bad = dict(BASE); bad["verifier_commit_sha"] = "5" * 40
        with self.assertRaisesRegex(ValueError, "exactly"):
            evidence.validate(bad)

    def test_missing_field_rejected(self):
        bad = dict(BASE); del bad["release_sha256"]
        with self.assertRaisesRegex(ValueError, "exactly"):
            evidence.validate(bad)

    def test_empty_digest_rejected(self):
        for key in ("dispatch_sha256", "release_sha256", "contract_sha256"):
            bad = dict(BASE); bad[key] = ""
            with self.assertRaises(ValueError):
                evidence.validate(bad)

    def test_invalid_ids_rejected(self):
        for key, value in (
            ("dispatch_sha256", "a" * 63),
            ("release_sha256", "b" * 65),
            ("contract_commit_sha", "c" * 39),
            ("contract_sha256", "d" * 63),
        ):
            bad = dict(BASE); bad[key] = value
            with self.assertRaises(ValueError):
                evidence.validate(bad)

    def test_path_traversal_rejected(self):
        for value in ("../contract.json", "a/../contract.json", "/etc/passwd", "\\evil\\contract.json"):
            bad = dict(BASE); bad["contract_path"] = value
            with self.assertRaises(ValueError):
                evidence.validate(bad)

    def test_verifier_identity_is_not_an_evidence_field(self):
        for key in ("verifier_ref", "verifier_commit_sha", "verifier_tree_sha", "suite_id", "toolchain_id"):
            bad = dict(BASE); bad[key] = "attacker-controlled"
            with self.assertRaises(ValueError):
                evidence.validate(bad)

if __name__ == "__main__":
    unittest.main()
