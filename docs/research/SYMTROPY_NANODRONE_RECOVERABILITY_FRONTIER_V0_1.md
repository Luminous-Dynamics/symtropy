# Symtropy × Symthaea: intervention-conditioned recoverability frontier v0.1

**Status:** Research evaluator draft. Exact-head Rust build and tests are not yet verified. The checked-in fixture is synthetic only; this document reports no physical-flight or control-performance result.

## Why this is a separate benchmark

The paired warning evaluator asks whether a warning ranks impending severe error. The failure-aware rollout evaluator asks how accurately a learned dynamics model predicts complete registered trajectories. Neither tells us whether a particular response can still recover the mission from a given state.

This evaluator makes recoverability conditional on the **state × intervention × remaining budget**, rather than treating it as an intrinsic scalar property of a state. It compares the latest preregistered checkpoint at which each intervention's estimated success probability satisfies a declared lower-confidence-bound criterion.

Research motivation:
- RESCUE, submitted 9 October 2026, proposes restoring simulator/policy checkpoints and comparing continuation with response branches; it reports that useful interventions depend jointly on the state and action: https://arxiv.org/abs/2610.13516
- NanoBench's July 2026 contamination audit motivates complete-recording splits and uncertainty over both recordings and fresh model-training seeds: https://arxiv.org/abs/2607.18482
- Nano-Drone and NanoBench remain separate benchmark protocols and must never be combined into a pooled score: https://doi.org/10.1016/j.conengprac.2026.106871 and https://arxiv.org/abs/2603.09908

This evaluator does **not** claim to reproduce RESCUE's posterior criterion or results. It uses a transparent one-sided 95% Wilson lower confidence bound across branch-seed outcomes at each fixed checkpoint and training seed.

## Input contracts

### Checkpoint manifest

CSV columns:

    family_id,flight_id,rollout_start_id,regime,checkpoint_time_s,remaining_budget_s

Each row registers one exact checkpoint of a trunk trajectory. A family is the independent generalization cluster; multiple flights, rollout starts, checkpoints and seeds within a family are not independent families. Checkpoint time and remaining budget must be positive and finite. For each flight/start, checkpoint_time_s + remaining_budget_s must preserve one fixed terminal deadline across the registered checkpoints. Every flight/start must register at least two checkpoints.

### Counterfactual branch CSV

CSV columns:

    family_id,flight_id,rollout_start_id,regime,checkpoint_time_s,remaining_budget_s,branch_elapsed_s,training_seed,intervention_id,branch_seed,state_snapshot_sha256,exogenous_tape_sha256,restore_verified,status,mission_completed,boundary_violation_count,tracking_rmse_m,energy_wh,deadline_misses

The intervention_id must contain continue and at least one response. Every registered checkpoint × training seed × intervention × branch seed combination must be present exactly once.

The evaluator fails closed on:
- a training/evaluation family overlap;
- duplicate or missing grid cells, duplicate keys, unregistered seeds/interventions, or regime disagreement;
- failed state-restore verification;
- non-hex snapshot/tape identifiers;
- any mismatch in the snapshot digest across branches forked from the same checkpoint/training-seed state;
- any mismatch in the exogenous disturbance-tape digest across intervention arms for the same checkpoint/training-seed/branch-seed;
- any mismatch between the branch's remaining budget and its checkpoint manifest, any checkpoint schedule that changes the terminal deadline, or any branch elapsed time that exceeds its remaining budget;
- reuse of one tape digest as if it were multiple distinct branch seeds;
- non-finite or negative supplied metrics, unsupported statuses, or an ok status missing tracking/energy values.

For real experiments, snapshot and disturbance identifiers must be SHA-256 of canonical serialized bytes. The synthetic fixture's digest-shaped values are intentionally only consistency tokens and must not be treated as provenance.

## Success criterion

A branch is recoverable only if all conditions hold:
- elapsed time is finite, non-negative and within the registered remaining task budget;
- status is ok;
- mission completion is true;
- no safety-boundary violation is recorded;
- no hard deadline miss is recorded;
- tracking RMSE does not exceed the preregistered --max-tracking-rmse-m.

Failures remain in the branch denominator as unsuccessful outcomes. The threshold is a task-specific success definition, not a universal drone-safety definition. Energy is reported as a descriptive outcome and is not silently mixed into this binary recovery criterion.

## Estimator

At each fixed checkpoint, training seed and intervention, estimate the branch-seed success fraction. The checkpoint qualifies only if its one-sided 95% Wilson lower confidence bound is at least the registered --recoverability-threshold.

For every flight/start/training-seed/intervention unit, the recoverability horizon is the latest registered checkpoint that qualifies. A horizon of 0 means no registered checkpoint met the threshold; it is not a claim that the physical recovery time is literally zero.

Every checkpoint carries an explicit remaining budget. For a fixed flight/start, the declared terminal deadline is invariant across checkpoint times; branches report their elapsed time from the restored checkpoint and may not exceed the remaining budget. This prevents a branch from qualifying solely because it was scored under a more generous stopping deadline.

The report separates:
- checkpoint-level success and lower bounds;
- failure/violation/deadline rates and observed tracking/energy summaries;
- each unit's latest recoverable checkpoint;
- mean recoverability horizons and paired horizon deltas against continue.

The 95% interval for mean horizon delta uses paired crossed resampling of independent family clusters and training seeds, with the same draws across intervention and continuation. Family clusters receive equal weight in the aggregate. This interval is **conditional on the registered branch-seed panel**; branch-seed identities are not themselves re-sampled into that horizon-delta interval. The pointwise Wilson bounds account for branch-seed sampling when classifying each checkpoint, but they do not propagate all branch-seed uncertainty into the horizon-difference interval. Larger studies should examine a predeclared multi-stage/cluster-bootstrap extension near the recoverability boundary.

The code must not infer full warning-policy benefit from a positive horizon delta. An intervention can extend recoverability while still costing too much energy/time, increasing workload, or failing to improve full-episode mission outcomes.

## Run

The example lives in integration/symthaea-physics-loop. With the real integration checkout configured:

    cargo test --manifest-path integration/symthaea-physics-loop/Cargo.toml --example nanodrone_recoverability_frontier
    cargo check --manifest-path integration/symthaea-physics-loop/Cargo.toml --example nanodrone_recoverability_frontier

CLI usage:

    nanodrone_recoverability_frontier \
      --input branches.csv \
      --checkpoints checkpoints.csv \
      --train-families train_families.txt \
      --dataset-receipt-sha256 HEX64 \
      --model-manifest-sha256 HEX64 \
      --source-commit HEX40 \
      --training-seeds 101,102,103 \
      --branch-seeds 11,12,13,14 \
      --interventions continue,robust_fallback,controlled_hover \
      --max-tracking-rmse-m 0.5 \
      --recoverability-threshold 0.5 \
      --bootstrap 2000 --seed 17 \
      --evidence-class simulation \
      --output recoverability-frontier.json

The smoke test is wired behind STFR_CHECK_NANODRONE_RECOVERABILITY=1 in scripts/test-real-symthaea-in-symtropy.sh. It validates the output schema and expected ordering on the synthetic fixture. Passing that fixture would demonstrate evaluator plumbing only, not simulated dynamics, physical recoverability or closed-loop benefit.

## Research gates and comparative targets

These are proposed research targets, not established industry standards:
- 100% of preregistered branch-grid cells accounted for; missing or duplicated branches fail closed.
- 100% of restore-mismatch, unpaired disturbance-tape, duplicate-tape, training/evaluation split, and invalid metric controls are rejected.
- Compare continue, simple deterministic fallback, bounded attitude/rate mode, hover/landing only where the simulator supports them, and a random response schedule with matched intervention budget.
- Report recoverability by regime, checkpoint time, training seed and intervention; do not pool vehicles or regimes without a declared estimand.
- Report branch count, success count, lower bound, boundary violations, deadline misses, energy and latency. Retain null/unavailable measurements as such.
- Near the threshold boundary, predeclare an adaptive replication policy (for example, increase branch repeats only for ambiguous checkpoint/intervention pairs) and evaluate its type-I behavior and compute cost on a separate development panel before confirmatory use.
- Once a response improves recoverability, test the **frozen warning × response policy** on complete episodes against both nominal/no-intervention and rate-matched random-trigger policies. Prediction scores and recoverability alone do not establish policy value.

## Evidence ceiling and follow-on

This tool consumes precomputed branches. It does not capture snapshots, run the simulator, train models, or prove snapshot fidelity. A matching hash confirms identity of the supplied identifier bytes, not that the simulator's hidden state was completely captured.

A result is limited to the declared simulator, success rule, interventions, checkpoints, registered seeds and scenario families. Physical-flight safety requires separate independent HIL/flight qualification. Full-policy utility remains tracked in issue #2812, with the parent protocol in PR #2813.
