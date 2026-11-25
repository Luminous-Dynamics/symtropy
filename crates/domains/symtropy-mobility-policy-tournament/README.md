# Symtropy Mobility Energy Policy Tournament

A matched, fail-closed outcome evaluator for mobility-first charging and grid-flexibility policies. It consumes policy run summaries; it does not implement or optimize the policies.

## Core comparison rule

Every policy must be replayed on exactly the same exogenous scenario/replication pairs, random seeds, independent clusters, evidence class, compute environment and compute budget. Each policy keeps its own configuration digest. A missing, duplicated, invalid or mismatched run rejects the comparison; timeouts, failures and abstentions stay in the run denominator. Each record must still contain the full realized service/energy outcome for that horizon, including any deterministic safe-fallback outcome after a timeout or abstention.

## Two separate score planes

**Hard assurance:** mission reserve, chronological recovery, PCC/network limits, authorization, double commitment, evidence integrity, cyber integrity, false-safe admission and false abstention. These are not weighted into an objective and cannot be compensated for by lower cost or more grid response.

**Performance:** on-time and completed mobility service, wait, requested/instructed/signed-metered grid response, maximum DC deliverability, site imports, renewable curtailment, battery throughput, operating/degradation cost, emissions proxy and decision latency.

The signed PCC response is retained. A negative value means increased import, not zero response. The evaluator also records when positive metered reduction exceeds the declared DC network-deliverable cap. It never clips such a conflict.

## Paired inference

Candidate-minus-reference deltas are paired by scenario ID and replication ID. Repeated observations are first averaged within the declared independent cluster (for example site-day or independent incident block). The evaluator then reports a deterministic cluster-bootstrap percentile interval with an explicit random seed, resample count, confidence level and independent-cluster count.

No interval is emitted when the declared minimum number of independent clusters is not met. These are resampling intervals over the supplied labelled clusters, not distribution-free guarantees and not a claim that correlated intervals are independent. Per-policy reports also expose empirical upper-tail CVaR95 for mission-reserve and recovery shortfall, maximum shortfalls, and outcome summaries by scenario family and independent cluster. These tail summaries are descriptive; they are not extrapolated to unseen events.

## Run

    cargo run -p symtropy-mobility-policy-tournament --bin mobility_policy_tournament -- crates/domains/symtropy-mobility-policy-tournament/fixtures/mobility_policy_tournament_v0_1.json

Tests and lint:

    cargo test -p symtropy-mobility-policy-tournament --locked
    cargo clippy -p symtropy-mobility-policy-tournament --all-targets --locked -- -D warnings

## Frozen synthetic fixture

The fixture contains three declared policies and four paired site-day clusters (12 run summaries): mobility-only fixed-cap, rolling-horizon MPC and a shielded learned policy. All share the same scenario digests, input digests, random seeds, compute environment and per-run compute budget.

The fixture is only a contract known-answer test. It is not a simulation of a real depot, not a native reproduction of any paper, and not physical field data.

## Source ownership and report hand-off

Run summaries should be produced by the same frozen replay runner and composed from the owning evidence contracts: reserve/recovery (#6803/#6809), lease/journal (#6829/#6837), network deliverability (#7313), online risk monitoring (#7290), dispatch tracking (#3649), PCC metrology (#3772), and baseline/rebound reconciliation (#3807/#7321). This evaluator does not recalculate those results or promote a model-level pass to market/dispatch authority.

## Claim ceiling

The scorecard compares the supplied finite corpus only. It does not authenticate source artifacts, establish causal benefits, certify a physical resource, qualify a market product, authorize export/dispatch, establish AC feasibility or compute ELCC. Paper-reported and vendor-reported performance must remain distinct from independently reproduced, measured-replay and physical-trial evidence.
