# RoboCasa-Mem behavior-conditioned evaluation contract rehearsal

This is a dependency-free Rust **synthetic contract fixture** for one semantic property described in RoboCasa-Mem (arXiv:2611.06475): when multiple execution paths are valid, a later evaluator constraint can be derived from the agent's own earlier interaction rather than from a fixed target.

It does not implement RoboCasa-Mem, reproduce its task evaluator, or run the RoboCasa simulator.

## Contract under test

- The policy receives only a typed `PolicyInput` (episode, instruction, current observation reference and robot-state reference).
- The evaluator separately receives an ordered stream of evaluator-observed `InteractionEvent` values.
- The evaluator derives an `EvaluatorDerivedConstraint` from the latest qualifying prior interaction.
- A later evaluator-observed interaction is checked against that derived constraint.
- Two episodes with identical policy-visible inputs may correctly have different evaluator-private expected entities because their earlier valid choices differed.
- Missing prior interactions fail closed; there is no fixed-target fallback.
- Duplicate/replayed IDs, mixed episodes and reordered/duplicate sequence numbers are rejected rather than silently repaired.
- A separate synthetic process-memory oracle preserves paused/open obligations across interleaved task threads; submission remains blocked until all obligations are satisfied.
- Illegal task transitions, cross-task satisfaction, duplicate openings, late resumption after completion and reordered/replayed events fail closed.

The synthetic example models a prior choice of cup A or cup B followed by a later choice that must remain consistent with the earlier interaction. This is an intentionally minimal semantic fixture, not a copy of any RoboCasa-Mem task.

## Why behavior-conditioned evaluation matters

A fixed-target evaluation can reward a memorized destination even when the task permits multiple valid choices. A behavior-conditioned evaluator can assess history-consistent execution while leaving the valid path open.

The paper describes nine mobile-manipulation tasks spanning:
- Environment memory: spatial, attribute and relational information.
- Process memory: single-task progress and multi-task scheduling.

It reports task success (SR) and stage completion (SCR) separately. The synthetic contract tests none of the native task policies, scene assets, simulator, benchmark scores, or physical robot.

## Run

```bash
cargo fmt --manifest-path experiments/robocasa-mem-bce-contract/Cargo.toml -- --check
cargo test --locked --manifest-path experiments/robocasa-mem-bce-contract/Cargo.toml
cargo clippy --locked --manifest-path experiments/robocasa-mem-bce-contract/Cargo.toml --all-targets -- -D warnings
```

## Nonclaims

Passing these tests means only that this small Rust fixture implements the declared synthetic evaluator semantics. It does not qualify the upstream RoboCasa-Mem release, prove that the upstream evaluator is implemented correctly, demonstrate policy memory capability, establish any benchmark improvement, or establish physical truth/safety.


## Result-import and qualification envelope

The additive Rust module `result_envelope.rs` defines a per-episode result contract for later native benchmark adapters. It deliberately keeps these statements independent:

- terminal task outcome (Succeeded / Failed / Unresolved);
- each preregistered stage result (Succeeded / Failed / Unresolved / NotApplicable);
- Behavior-Conditioned Evaluation decision receipts;
- policy-visible inputs versus evaluator-private fields;
- policy/checkpoint/deployment/training-data identity;
- assistance track (policy-only, agent, tool, human, mixed, unknown);
- run disposition (completed, policy abort, safety abort, operator abort, infrastructure indeterminate, invalid protocol);
- evidence class (synthetic, simulator, HIL, supervised physical, autonomous physical, independently reviewed physical).

The importer requires exactly one ordered stage record for every stage in the frozen task roster. Missing, duplicate, extra or reordered stages reject rather than silently changing the denominator. Unresolved applicable stages remain in the denominator; NotApplicable is retained separately. Terminal success is never inferred from stage completion, and successful stages are not erased when the terminal task later fails.

The protocol envelope pins the task manifest, evaluator, dataset manifest, dataset-rights manifest, observation schema and action schema digests, plus an exact 40-hex source commit. Dataset-rights status and training-overlap disclosure are explicit. Unknown rights or unknown training overlap block a qualified comparison while preserving the raw measurements; known overlap remains disclosed rather than being relabeled as no overlap. The task record can declare whether BCE receipts are required; missing required receipts block qualification. BCE receipts keep evaluator-private decision evidence separate from the policy input digest.

Assistance disclosure is not a cosmetic label: a PolicyOnly record cannot contain tools, agent wrappers, oracle channels or human interventions; a ToolAssisted/HumanAssisted/AgentAssisted label must have the corresponding disclosed evidence. Unknown or contradictory assistance blocks comparison rather than being silently treated as autonomous.

The paired-control check requires identical protocol, task/scene/seed/split/stage roster, embodiment, action executor, environment, resource budget, evidence class and policy-visible observation manifest. Only the declared policy variant/checkpoint/assistance may differ. This is an identity check, not a statistical significance test. The per-arm episode IDs may differ: pairing is by the frozen task instance, scene identity, seed, split, expected stage roster and shared control manifests—not by reusing the same run ID.

This module is a typed Rust validation contract only. It does not yet parse the upstream RoboCasa-Mem HDF5/Python export, run policies, compute the official macro aggregation, authenticate evidence digests, or verify that the upstream task/evaluator/dataset release has been pinned. See [VIS-MEM-029 #4414](https://github.com/Luminous-Dynamics/symtropy/issues/4414).


## Current release qualification gate

The Rust module `release_gate.rs` adds a separate preflight contract for native RoboCasa-Mem evaluation. It does not run the upstream simulator or policy. It prevents evidence from one source/wheel/runtime from being silently reused to qualify a different candidate.

The gate requires:

- exact equality between the checked-out source revision and the release manifest's declared revision;
- independent verification that the source-file manifest corresponds to the checked-out source, plus pinned release/suite/input-lock/simulator-source manifests and preserved suite bytes;
- exact task, suite and resolved official input counts;
- separate dataset and asset rights statuses, plus digest-addressed rights manifests;
- asset archive identity, asset-manifest task coverage, an independent task-coverage verification receipt, exact task count, and a clean candidate installation;
- installed-wheel acceptance for the **current wheel digest and current runtime digest**, with every expected official input passed in a clean environment;
- a pinned expected policy-family manifest and exact expected checkpoint digest for every (family, task) suite in the 27-suite matrix; every tuple must have a checkpoint-loaded/inference smoke using the exact candidate runtime, with inference requests, no runtime errors and complete episode artifacts;
- explicit training-data overlap disclosure; unknown overlap blocks qualified comparative evaluation, while known overlap remains disclosed rather than being relabeled as no overlap.

A historical wheel pass does not qualify a changed wheel or runtime. A smoke of one (policy family, task) tuple does not unlock another tuple, even within the same family. A model inference smoke is not required to complete the benchmark task; task success and benchmark performance are measured later by the native evaluator. These stages remain distinct.

The upstream candidate inspected at source revision `2c7d498e589d1a5a62b5b30c9d786513fa86bfa2` declares candidate source revision `b891bdf35fbe7b8390ea6cc663a870fb9ea7820c`, `release_ready=false`, and `suite_bytes_preserved=false`. The declared source revision and inspected source revision therefore need reconciliation. The release-candidate manifest contains a separate asset-verification receipt that reports coverage for all 450 official inputs, but the referenced distributed asset manifest itself still says `nine_task_coverage_verified=false` and describes an archive inventory rather than a verified minimal nine-task dependency closure. Both sources say clean installation of the current candidate is unverified. The historical 450-case installed-wheel pass used a previous wheel/runtime; metadata states `runtime_files_match_accepted_wheel=false`. The gate requires both the asset-manifest assertion and the independent receipt, plus clean candidate installation and current wheel/runtime acceptance. It includes regression fixtures for these contradictory and historical-evidence states.

Use the typed API from an adapter which has already verified and hashed the evidence artifacts:

~~~rust
use symtropy_robocasa_mem_bce_contract::release_gate::{
    qualify_release, RoboCasaMemReleaseEvidence,
};

let report = qualify_release(&release_evidence)?;
if !report.native_evaluation_ready {
    // Preserve and report every blocker; do not launch native benchmark runs.
}
~~~

The crate validates digest shape and exact equality; it does not hash or authenticate files, establish the truth of an external receipt, adjudicate licensing, or verify the runtime itself. Those responsibilities belong to the acquisition/verification adapter and independent review. The full pinned-source audit and release-gate design are documented in [ROBOCASA_MEM_RELEASE_QUALIFICATION_V0_1.md](../../docs/research/ROBOCASA_MEM_RELEASE_QUALIFICATION_V0_1.md). The synthetic release-gate tests establish only blocker logic, not that the upstream candidate is qualified.

Run the complete nested contract suite (including release-gate regression cases):

~~~sh
cargo fmt --manifest-path experiments/robocasa-mem-bce-contract/Cargo.toml -- --check
cargo test --locked --manifest-path experiments/robocasa-mem-bce-contract/Cargo.toml
cargo clippy --locked --manifest-path experiments/robocasa-mem-bce-contract/Cargo.toml --all-targets -- -D warnings
~~~
