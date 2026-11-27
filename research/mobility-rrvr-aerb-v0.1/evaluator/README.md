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

## Candidate counterfactual-policy contract v0.2

The isolated module `src/v0_2.rs` validates the candidate-visible AERB v0.2 envelope without changing the v0.1 evaluator's semantics.

It checks:
- schema / episode identity and the existing v0.1 sorted-JSON candidate snapshot digest;
- digest syntax for DecisionNeed, decision contract, probe catalogue, model and observer profile;
- a mandatory `block_new_effect` actuation guard;
- non-empty, unique possible-history support with `open_world_residual` retained;
- permitted candidate probe models and complete outcome-support coverage of the candidate's initial support;
- ordinary-probe open-world retention and the special closed-plus-unresolved support rule for `reconcile_effect_and_scope`;
- an acyclic, bounded policy tree with one parent per non-root node, no unreachable nodes, and branch edges matching exactly the outcomes reachable under the current candidate support;
- cumulative acquisition cost and result delay along every policy path;
- rejection of a `recommend_fresh_attempt` terminal while the candidate's own support still contains any history other than `not_dispatched` / `dispatched_no_effect`.

The module denies unknown JSON fields, so candidate submissions cannot smuggle evaluator-only truth, outcome partitions, likelihoods or loss rows into this candidate-facing object. A schema / type check is not a process-security boundary; blind evaluation still requires separately permissioned candidate and evaluator environments.

Run the existing focused commands:

~~~bash
cargo fmt --manifest-path research/mobility-rrvr-aerb-v0.1/evaluator/Cargo.toml -- --check
cargo test --manifest-path research/mobility-rrvr-aerb-v0.1/evaluator/Cargo.toml --locked
cargo clippy --manifest-path research/mobility-rrvr-aerb-v0.1/evaluator/Cargo.toml --locked --all-targets -- -D warnings
~~~

The v0.2 tests are authored, but must not be described as passing until an exact-head run reports the formatter, Rust tests and strict Clippy complete successfully. This module checks the candidate contract and policy graph; it does not compute minimax / EVSI, certify the source or observation, execute a probe, or authorize robot motion.
