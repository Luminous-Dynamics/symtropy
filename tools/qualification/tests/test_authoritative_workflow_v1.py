from __future__ import annotations

from pathlib import Path
import unittest

WORKFLOW = Path(__file__).resolve().parents[2] / ".github" / "workflows" / "qual-001b-authoritative-verifier-v1.yml"

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

if __name__ == "__main__":
    unittest.main()