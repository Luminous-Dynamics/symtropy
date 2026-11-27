# Receiver-service owner receipt source metadata

This is the exact TSV receipt extracted from the uploaded artifact of the canonical Symtropy receiver-service owner.

- Owner issue: https://github.com/Luminous-Dynamics/symtropy/issues/7554
- Owner PR: https://github.com/Luminous-Dynamics/symtropy/pull/7557
- Source branch: `research/maritime-receiver-service-v0`
- Source code commit: `a7208c541a1a567bbca807c1417636e3c16bd036`
- Source fixture: `research/maritime-receiver-service/fixtures/outcomes.tsv`
- Dedicated workflow: [run #23](https://github.com/Luminous-Dynamics/symtropy/actions/runs/45776192970), exact-head success.
- Artifact: `maritime-receiver-service-receipt`, ID `15095902422`.
- Artifact ZIP digest: `sha256:efd8213ad3fc966c1b52170734e1498f8bc9e9e7e719e57ff697c055188458a4`.
- Extracted file: `receiver-service-receipt.tsv`.
- Extracted file SHA-256: `5024d111b281f71a5bd9e97adbf9013a99975004510506506c22f06c4facb4ae`.
- Extracted file size: 1,363 bytes; 24 lines (including header).
- The receipt came from a hosted upload artifact, not from a manually transcribed result.

The receipt summarizes ten synthetic cases and eleven (cohort, unit) summaries. It is owner-produced output from the #7554 receiver-service evaluator. MCSC consumes the typed summary states; it must not reimplement their lower/upper service-bound or protected-floor decisions.

The receipt itself is still a synthetic arithmetic/state-contract result. It does not authenticate real receiver evidence or establish cargo custody; compose #6631/#6662 and Mycelix authority/custody evidence for any physical claim.
