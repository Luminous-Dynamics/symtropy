import hashlib
import json
import os
import subprocess
import tempfile
import unittest
from pathlib import Path

HERE = Path(__file__).resolve().parents[1]
SCRIPT = HERE / "verify_signed_execution_receipt.sh"
POLICY = HERE.parent / "fixtures" / "execution-attestation-verification-policy-2026.json"
RECEIPT = HERE.parent / "fixtures" / "controller-execution-receipt-template-2026.json"
PAIRED = HERE.parent / "fixtures" / "paired-comparison-v0.4.json"


class SignedExecutionReceiptWrapperTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.policy_sha = hashlib.sha256(POLICY.read_bytes()).hexdigest()

    def run_wrapper(self, policy_bytes=None, bundle_obj=None, trusted_sha=None, cosign_version="v3.1.4",
                    cosign_exit=0):
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            policy_path = root / "policy.json"
            receipt_path = root / "receipt.json"
            paired_path = root / "paired.json"
            bundle_path = root / "bundle.json"
            log_path = root / "cosign-arguments.log"
            stub_dir = root / "bin"
            stub_dir.mkdir()

            policy_path.write_bytes(policy_bytes if policy_bytes is not None else POLICY.read_bytes())
            receipt_path.write_bytes(RECEIPT.read_bytes())
            paired_path.write_bytes(PAIRED.read_bytes())
            bundle = bundle_obj if bundle_obj is not None else {
                "mediaType": "application/vnd.dev.sigstore.bundle.v0.3+json",
                "synthetic_test_only": True
            }
            bundle_path.write_text(json.dumps(bundle))

            cosign_stub = stub_dir / "cosign"
            cosign_stub.write_text(
                "#!/usr/bin/env bash\n"
                "set -euo pipefail\n"
                "if [[ \"$1\" == \"version\" ]]; then\n"
                f"  echo 'GitVersion: {cosign_version}'\n"
                "  exit 0\n"
                "fi\n"
                "printf '%s\\n' \"$*\" > \"$FAKE_COSIGN_LOG\"\n"
                "if [[ \"$1\" == \"verify-blob\" ]]; then exit \"$FAKE_COSIGN_EXIT\"; fi\n"
                "exit 90\n"
            )
            os.chmod(cosign_stub, 0o755)
            env = os.environ.copy()
            env["PATH"] = str(stub_dir) + os.pathsep + env.get("PATH", "")
            env["FAKE_COSIGN_LOG"] = str(log_path)
            env["FAKE_COSIGN_EXIT"] = str(cosign_exit)
            env.pop("SYMTROPY_TRUSTED_POLICY_SHA256", None)
            if trusted_sha is not None:
                env["SYMTROPY_TRUSTED_POLICY_SHA256"] = trusted_sha
            proc = subprocess.run(
                ["bash", str(SCRIPT), str(receipt_path), str(bundle_path), str(paired_path), str(policy_path)],
                text=True, capture_output=True, env=env, check=False
            )
            arguments = log_path.read_text() if log_path.exists() else None
            return proc, arguments

    def test_requires_out_of_band_policy_digest(self):
        proc, arguments = self.run_wrapper()
        self.assertNotEqual(proc.returncode, 0)
        self.assertIn("out-of-band trusted configuration", proc.stderr)
        self.assertIsNone(arguments)

    def test_rejects_policy_digest_mismatch(self):
        proc, arguments = self.run_wrapper(trusted_sha="0" * 64)
        self.assertNotEqual(proc.returncode, 0)
        self.assertIn("policy digest does not match", proc.stderr)
        self.assertIsNone(arguments)

    def test_rejects_legacy_or_nonstandard_bundle(self):
        bundle = {"mediaType": "application/vnd.dev.cosign.simplesigning.v1+json"}
        proc, arguments = self.run_wrapper(trusted_sha=self.policy_sha, bundle_obj=bundle)
        self.assertNotEqual(proc.returncode, 0)
        self.assertIn("legacy/nonstandard bundle formats are rejected", proc.stderr)
        self.assertIsNone(arguments)

    def test_rejects_unreviewed_cosign_version(self):
        proc, arguments = self.run_wrapper(trusted_sha=self.policy_sha, cosign_version="v3.1.3")
        self.assertNotEqual(proc.returncode, 0)
        self.assertIn("exactly v3.1.4", proc.stderr)
        self.assertIsNone(arguments)

    def test_rejects_signature_verification_failure(self):
        proc, arguments = self.run_wrapper(trusted_sha=self.policy_sha, cosign_exit=1)
        self.assertNotEqual(proc.returncode, 0)
        self.assertIsNotNone(arguments)

    def test_invokes_cosign_with_exact_identity_and_issuer(self):
        proc, arguments = self.run_wrapper(trusted_sha=self.policy_sha)
        self.assertEqual(proc.returncode, 0, proc.stderr)
        self.assertIn("--certificate-identity https://github.com/Luminous-Dynamics/symtropy/.github/workflows/controller-execution-attestation.yml@refs/heads/main", arguments)
        self.assertIn("--certificate-oidc-issuer https://token.actions.githubusercontent.com", arguments)
        self.assertIn("--bundle", arguments)
        self.assertIn("scoring_authorized=false", proc.stdout)
        self.assertIn("physical validity not inferred", proc.stdout)

    def test_rejects_receipt_promoted_to_scoring_before_signature_check(self):
        receipt = json.loads(RECEIPT.read_text())
        receipt["scoring_eligible"] = True
        with tempfile.TemporaryDirectory() as td:
            root = Path(td)
            policy_path, receipt_path = root / "policy.json", root / "receipt.json"
            paired_path, bundle_path = root / "paired.json", root / "bundle.json"
            policy_path.write_bytes(POLICY.read_bytes())
            receipt_path.write_text(json.dumps(receipt))
            paired_path.write_bytes(PAIRED.read_bytes())
            bundle_path.write_text(json.dumps({"mediaType": "application/vnd.dev.sigstore.bundle.v0.3+json"}))
            env = os.environ.copy()
            env["SYMTROPY_TRUSTED_POLICY_SHA256"] = self.policy_sha
            proc = subprocess.run(
                ["bash", str(SCRIPT), str(receipt_path), str(bundle_path), str(paired_path), str(policy_path)],
                text=True, capture_output=True, env=env, check=False
            )
        self.assertNotEqual(proc.returncode, 0)
        self.assertIn("scoring remains disabled", proc.stderr)

    def test_rejects_policy_with_scoring_enabled(self):
        policy = json.loads(POLICY.read_text())
        policy["scoring_authorized"] = True
        proc, arguments = self.run_wrapper(
            policy_bytes=json.dumps(policy).encode(),
            trusted_sha=hashlib.sha256(json.dumps(policy).encode()).hexdigest()
        )
        self.assertNotEqual(proc.returncode, 0)
        self.assertIsNone(arguments)


if __name__ == "__main__":
    unittest.main()
