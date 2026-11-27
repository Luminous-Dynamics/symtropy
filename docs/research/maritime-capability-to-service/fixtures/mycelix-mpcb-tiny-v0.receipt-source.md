# Frozen owner receipt source metadata

This file contains the deterministic receipt emitted by Mycelix's dedicated **Maritime Productive Capability Reference** workflow, run #5 (GitHub Actions run ID 45946710709).

- Owner repository: https://github.com/Luminous-Dynamics/mycelix
- Source branch: `research/maritime-productive-capability-v0`
- Source code commit: `877716f9be9cbdf56f9c4b8be67349b9bca97a43`
- Source fixture: `docs/research/maritime-productive-capability/fixtures/maritime-capability-tiny-v0.tsv`
- Workflow: https://github.com/Luminous-Dynamics/mycelix/actions/runs/45946710709
- Receipt extraction: deterministic receipt lines captured from the workflow job log's receipt-emission step; this is a frozen transcript snapshot, not a downloaded signed artifact.
- Source claim ceiling: synthetic resource-capacity and Pareto qualification only.

The receipt states that the two-work-package all-regional plan is infeasible because plate demand is 4 t against 3 t capacity and welding demand is 16 h against 12 h capacity. Its NDT use is 2 jobs against 2 jobs capacity. The three other sourcing portfolios are feasible and Pareto-undominated under the source fixture's declared objective directions.

MCSC consumes the owner's **status and plan metrics**. It does not rerun Mycelix's portfolio enumerator or resource-feasibility logic. A follow-on production adapter must bind the receipt to an exact owner commit/digest, freshness and rights/authorization evidence; this snapshot alone is not evidence that any real yard has the represented capability.

- Frozen receipt file SHA-256: `d3a43974c810c73d74a89748add79d21135458c63e60a226446395fe26ae371e`
- Frozen receipt UTF-8 byte length: 1,675
- Non-empty receipt record count: 23
- GitHub blob SHA for the frozen TSV: `63016145a83a0aadfb3f15a48affeaaca6129bc6`

These identify the exact checked-in transcript bytes used by the composition integration test. The SHA-256 was computed over the UTF-8 text including its terminal newline.
