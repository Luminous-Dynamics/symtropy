# AERB Evaluator v0.1

**Evaluator-side only.** This package validates and scores a candidate submission against a separately permissioned evaluator world pack. The hidden world pack must never be mounted, returned to or made accessible to the candidate model process. This package does not create a security sandbox; deployments must enforce separation outside the process.

## Inputs

1. Candidate-visible episode snapshot.
2. Candidate submission containing model/profile identity, possible-world support, a non-actuating disposition and optional probe request.
3. Evaluator-only world pack containing hidden truth, the world catalog, probe outcome supports, decision-loss rows and a frozen hard-risk ceiling.

## What v0.1 validates and reports

- Compact sorted-JSON SHA-256 binding of the exact candidate-visible snapshot.
- Episode identity, decision-cut/ledger-frontier equality, post-cut leakage, duplicate observation identities/sequences and observation quality counts.
- Command/scope/profile digest syntax, clock-domain metadata and permitted probe menu identity.
- Evaluator world-catalog uniqueness, hidden-truth membership, complete decision-loss rows and full per-probe outcome-support coverage.
- Candidate support-set membership, hidden-truth coverage and mandatory open-world-residual retention.
- Requested-probe permission, acquisition budget and decision-deadline admission.
- Selected disposition worst-case loss over the candidate support and admission under the evaluator-frozen hard-risk ceiling.
- Selected disposition loss at evaluator truth, emitted only as a post-run score; hidden truth identity is never emitted.

The evaluator does not implement or approximate EVSI/minimax probe ranking. Use Symthaea's existing prior-free robust evaluator for that calculation. This package does not execute probes or robot commands.

## Deterministic challenge corpus

The `aerb_generate` binary builds a repeatable process-model corpus while keeping candidate-visible and evaluator-only artifacts in disjoint output directories. It uses a seed-shuffled cycle through the eight world IDs, covering each world exactly once per full eight-case block. This is a balanced challenge schedule, not a probability distribution or an estimate of real-world event frequencies.

Example:

```bash
cargo run --manifest-path research/mobility-rrvr-aerb-v0.1/evaluator/Cargo.toml --locked --bin aerb_generate -- 42 128 /tmp/aerb-candidate /tmp/aerb-evaluator
```

The candidate directory receives only episode and submission files plus a candidate manifest. The seed and hidden truth mapping are not written to candidate-visible artifacts; they appear only in the evaluator-side manifest. The evaluator directory receives world packs and the separate truth manifest. The generator rejects output directories that are equal or nested. Do not mount the evaluator directory into the candidate process.


## Run

From the repository root:

```bash
cargo fmt --manifest-path research/mobility-rrvr-aerb-v0.1/evaluator/Cargo.toml -- --check
cargo test --manifest-path research/mobility-rrvr-aerb-v0.1/evaluator/Cargo.toml --locked
cargo clippy --manifest-path research/mobility-rrvr-aerb-v0.1/evaluator/Cargo.toml --locked --all-targets -- -D warnings

cargo run --manifest-path research/mobility-rrvr-aerb-v0.1/evaluator/Cargo.toml --locked --bin aerb_evaluate -- \
  research/mobility-rrvr-aerb-v0.1/fixtures/candidate_episode_synthetic_v0_1.json \
  research/mobility-rrvr-aerb-v0.1/fixtures/candidate_submission_synthetic_v0_1.json \
  research/mobility-rrvr-aerb-v0.1/fixtures/evaluator_only/world_pack_synthetic_v0_1.json
```

The public fixture contains synthetic truth for known-answer tests. The generator can create additional balanced stress cases, but neither it nor the public fixture is a blind evaluation set or demonstrates benchmark performance.

## Claim ceiling

Software/process-model conformance only. No physical effect truth, causal identification, calibrated probability, simulator/HIL/physical qualification, robot safety certification, RoboRecover reproduction or SOTA superiority is established. The public hard-risk threshold is illustrative; a real campaign must justify its threshold and losses prospectively.