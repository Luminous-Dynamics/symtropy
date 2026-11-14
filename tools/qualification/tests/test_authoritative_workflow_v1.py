from __future__ import annotations

from pathlib import Path
import unittest

WORKFLOW = Path(__file__).resolve().parents[3] / ".github" / "workflows" / "qual-001b-authoritative-verifier-v1.yml"

class AuthoritativeWorkflowV1Tests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.text = WORKFLOW.read_text(encoding="utf-8")

    def test_authority_uses_repository_dispatch(self) -> None:
        self.assertIn("repository_dispatch:", self.text)
        self.assertIn("types: [qual-001b-authority-v1]", self.text)

    def test_authority_has_no_workflow_dispatch_trigger(self) -> None:
        self.assertNotIn("workflow_dispatch:", self.text)

    def test_verifier_checkout_is_release_bound(self) -> None:
        self.assertIn("ref: ${{ steps.release.outputs.verifier_commit_sha }}", self.text)
        self.assertNotIn("ref: ${{ github.sha }}", self.text.split("Checkout exact approved verifier commit", 1)[-1])

    def test_authority_recomputes_commit_and_tree(self) -> None:
        self.assertIn('git -C "$VERIFIER_DIR" rev-parse HEAD', self.text)
        self.assertIn('git -C "$VERIFIER_DIR" rev-parse HEAD^{tree}', self.text)
        self.assertIn('test "$actual_commit" = "$EXPECTED_COMMIT"', self.text)
        self.assertIn('test "$actual_tree" = "$EXPECTED_TREE"', self.text)

    def test_release_record_path_is_fixed(self) -> None:
        self.assertIn("RELEASE_RECORD_PATH: tools/qualification/verifier-release-v1.json", self.text)

    def test_verifier_commit_is_not_derived_from_event_sha(self) -> None:
        self.assertNotIn("VERIFIER_COMMIT: ${{ github.sha }}", self.text)
        self.assertIn("VERIFIER_COMMIT: ${{ steps.release.outputs.verifier_commit_sha }}", self.text)
 
    def test_dispatch_payload_is_retained_exactly(self) -> None:
        self.assertIn("Retain exact validated dispatch payload", self.text)
        self.assertIn("cp --preserve=mode,timestamps /tmp/qual-001b-dispatch.json", self.text)
        self.assertIn('"$EVIDENCE_DIR/authority-dispatch-payload.json"', self.text)
        self.assertIn("cmp --silent /tmp/qual-001b-dispatch.json", self.text)

    def test_dispatch_evidence_recording_is_fail_closed(self) -> None:
        self.assertIn(
            "if: steps.dispatch.outcome == 'success' && steps.release.outcome == 'success'",
            self.text,
        )
        self.assertNotIn("name: Record dispatch identity\n        if: always()", self.text)

    def test_dispatch_evidence_contains_contract_binding(self) -> None:
        self.assertIn('"schema_version": 1', self.text)
        self.assertIn('"contract_commit_sha": os.environ["CONTRACT_COMMIT_SHA"]', self.text)
        self.assertIn('"contract_path": os.environ["CONTRACT_PATH"]', self.text)
        self.assertIn('"contract_sha256": os.environ["CONTRACT_SHA256"]', self.text)

    def test_contract_identity_is_retained_and_bound(self) -> None:
        self.assertIn("Record exact contract checkout identity", self.text)
        self.assertIn('git -C "$CONTRACT_DIR" rev-parse HEAD', self.text)
        self.assertIn('git -C "$CONTRACT_DIR" rev-parse HEAD^{tree}', self.text)
        self.assertIn('git -C "$CONTRACT_DIR" rev-parse "HEAD:$CONTRACT_PATH"', self.text)
        self.assertIn('"contract_commit_sha": os.environ["CONTRACT_COMMIT_SHA"]', self.text)
        self.assertIn('"contract_tree_sha": os.environ["CONTRACT_TREE_SHA"]', self.text)
        self.assertIn('"contract_blob_sha": os.environ["CONTRACT_BLOB_SHA"]', self.text)

    def test_complete_envelope_and_recursive_root_are_replayed(self) -> None:
        self.assertIn("Independently replay complete authority evidence envelope", self.text)
        self.assertIn("authority_evidence_envelope_v1.py", self.text)
        self.assertIn("Materialize recursive authority evidence root", self.text)
        self.assertIn("authority_evidence_root_v1.py", self.text)
        self.assertIn("Independently replay recursive authority evidence root", self.text)

    def test_recursive_evidence_steps_are_fail_safe(self) -> None:
        guard = "if: always() && steps.dispatch.outcome == 'success' && steps.release.outcome == 'success' && steps.contract.outcome == 'success'"
        self.assertGreaterEqual(self.text.count(guard), 5)
        self.assertIn("if: always()\n        uses: actions/upload-artifact@", self.text)
        self.assertIn("if-no-files-found: error", self.text)

    def test_retained_dispatch_evidence_is_replayed(self) -> None:
        self.assertIn("Independently verify retained dispatch evidence", self.text)
        self.assertIn("authority_dispatch_evidence_verify_v1.py", self.text)
        self.assertIn("--payload", self.text)
        self.assertIn("authority-dispatch-payload.json", self.text)

if __name__ == "__main__":
    unittest.main()