# Symtropy Mobility Energy Policy Tournament

A matched, fail-closed outcome evaluator for mobility-first charging and grid-flexibility policies. It consumes policy run summaries; it does not implement or optimize the policies.

## Core comparison rule

Every policy must be replayed on exactly the same exogenous scenario/replication pairs, random seeds, independent clusters, evidence class, compute environment and compute budget. Each policy keeps its own configuration digest. A missing, duplicated, invalid or mismatched run rejects the comparison; timeouts, failures and abstentions stay in the run denominator. Each record must contain the complete realized horizon outcome, including any safe-fallback behaviour after timeout or abstention.

Every online run also carries explicit per-decision receipts: decision point, decision timestamp, information-set digest, observation digest and action digest. The evaluator independently groups receipts by policy + decision point + information-set digest and verifies that every occurrence has the same time, observation and action commitments. A conflict is charged to every run participating in that shared information set as an evidence-integrity hard-gate failure; different revealed information-set digests may legitimately select different actions. Missing or duplicate within-run decision receipts are rejected. Each run also carries information-trace/audit digests and future-information/nonanticipativity counters. A clean online audit reports `true`, an online leak/conflicting action mapping reports `false`, and a clairvoyant offline upper-bound track reports `null`/not-applicable. These digests and receipt semantics are source-supplied: the evaluator does not authenticate the audit producer or prove that the supplied information-set digest faithfully includes every observation. The existing #7133/#7181 scenario-tree oracle remains the information-set construction/optimization owner; this crate checks cross-run policy action consistency and does not replace that oracle.

## Two separate score planes

**Hard assurance:** mission reserve, chronological recovery, PCC/network limits, authorization, double commitment, evidence integrity, cyber integrity, false-safe admission and false abstention. These are not weighted into an objective and cannot be compensated for by lower cost or more grid response.

**Performance:** on-time and completed mobility service, wait, requested/instructed/signed-metered grid response, maximum DC deliverability, site imports, renewable curtailment, battery throughput, operating/degradation cost, emissions proxy and decision latency.

The signed PCC response is retained. A negative value means increased import, not zero response. The evaluator also records when positive metered reduction exceeds the declared DC network-deliverable cap. It never clips such a conflict.

## Paired inference and tail risk

Candidate-minus-reference deltas are paired by scenario ID and replication ID. Repeated observations are first averaged within the declared independent cluster (for example site-day or independent incident block). The evaluator then reports a deterministic cluster-bootstrap percentile interval with an explicit random seed, resample count, confidence level and independent-cluster count.

No interval is emitted when the declared minimum number of independent clusters is not met. These are resampling intervals over the supplied labelled clusters, not distribution-free guarantees and not a claim that correlated intervals are independent.

Per-policy output also reports empirical upper-tail CVaR95 and maximum reserve/recovery shortfall, as well as scenario-family and independent-cluster strata. These are descriptive summaries over the supplied benchmark runs, not guarantees about unseen events.

The bootstrap has an explicit 100,000,000 sample-operation work budget over independent clusters × resamples × candidate-policy comparisons × six metrics. Work is not charged when the independent-cluster threshold is too small to produce intervals. Excessive workloads are rejected before sampling.

## Run

    cargo run -p symtropy-mobility-policy-tournament --bin mobility_policy_tournament -- crates/domains/symtropy-mobility-policy-tournament/fixtures/mobility_policy_tournament_v0_1.json

Tests and lint:

    cargo test -p symtropy-mobility-policy-tournament --locked
    cargo clippy -p symtropy-mobility-policy-tournament --all-targets --locked -- -D warnings

## Frozen synthetic fixture

The fixture contains three declared policies and four paired site-day clusters (12 run summaries): mobility-only fixed-cap, rolling-horizon MPC and a shielded learned policy. All share the same scenario digests, input digests, random seeds, compute environment and per-run compute budget.

The fixture is only a contract known-answer test. It is not a simulation of a real depot, not a native reproduction of any paper, and not physical field data.

## Relationship to exact risk and nonanticipative oracles

The information-set semantics should reuse [#7133 / PR #7181](https://github.com/Luminous-Dynamics/symtropy/issues/7133) rather than treating a declared online label as proof. That oracle is mobility-service scoped; it is not yet a grid-energy scenario-tree oracle. The per-run information audit digests/counters in this scorecard are source-reported evidence, not independently authenticated proof.

The evaluator's empirical CVaR95 is an unweighted sample-tail summary. Probability-weighted finite-leaf CVaR and total-variation ambiguity belong to [#7196 / PR #7201](https://github.com/Luminous-Dynamics/symtropy/issues/7196) and the probability-evidence owner (#6460). Keep those metrics distinct when integrating reports.

## Source ownership and report hand-off

Run summaries should be produced by the same frozen replay runner and composed from the owning evidence contracts: reserve/recovery (#6803/#6809), lease/journal (#6829/#6837), network deliverability (#7313), online risk monitoring (#7290), dispatch tracking (#3649), PCC metrology (#3772), and baseline/rebound reconciliation (#3807/#7321). This evaluator does not recalculate those results or promote a model-level pass to market/dispatch authority.

## Claim ceiling

The scorecard compares the supplied finite corpus only. It does not authenticate source artifacts, establish causal benefits, certify a physical resource, qualify a market product, authorize export/dispatch, establish AC feasibility or compute ELCC. Paper-reported and vendor-reported performance must remain distinct from independently reproduced, measured-replay and physical-trial evidence.
