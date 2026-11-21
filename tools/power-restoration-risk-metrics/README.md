# Power restoration risk metrics v0.1

**Owner:** Symtropy #6533  
**Benchmark protocol:** #6448  
**Programme:** Power Fabric #6320 / #5241

## Purpose

A restoration leaderboard must not reward a policy for dropping difficult episodes, hiding timeouts, or blending incompatible physical scopes. This crate computes descriptive tail-risk and safety-adjudication metrics over a frozen cohort. It is planner-neutral and does not qualify the physical truth of input results.

## Cohort matching

A batch represents one policy in one cohort. Cohort identity binds benchmark suite/revision, dataset digest, physical qualification scope, objective/unit, horizon, topology split/holdout policy, hardware profile, wall-clock budget and maximum decision steps. A batch with a mismatched episode cohort or policy build/configuration is rejected. Compare policies by creating separate batches with identical cohort identity.

A balanced-AC FLARE-style cohort, a three-phase feeder cohort, a protection-qualified cohort and an HIL cohort must be different leaderboards. A single score across those scopes is invalid.

## Incomplete episodes

All attempted episodes remain in the denominator:
- completed;
- incomplete;
- timed out;
- abstained;
- malformed;
- infrastructure failed.

Strict-complete refuses any unscored episode. Conservative-worst-case imputes the configured regret bound (normally 1.0) and reports the count. Non-completed episodes are never assigned zero regret. A completed episode must carry a score in [0,1]; non-completed episodes must not carry one.

## Tail statistics

- Mean and median use all scored or explicitly imputed episodes.
- P95 is nearest-rank: sorted value at rank ceil(0.95 n), one-indexed.
- CVaR95 is the exact upper 5% tail mean. If 5% of n is fractional, the boundary observation receives fractional weight. It is not the average of every sample greater than or equal to P95.
- Maximum and scored coverage are explicit.

For small n, the exact worst-tail statistic may equal the maximum because the 5% tail mass is less than one full observation. This is mathematically intentional; larger matched cohorts provide more stable tail estimates.

## Independent safety adjudication

Every action record requires a separate adjudicator identity/profile, independence assertion, evidence artifact URI and SHA-256. Unknown is a distinct label and is excluded from known-safe/known-unsafe denominators, not counted as safe.

Separate quantities:
- unsafe proposal rate: unsafe / known-adjudicated actions;
- guard rejection rate: rejected / all action rows;
- guard precision: unsafe rejected / known rejected;
- guard recall: unsafe rejected / all known unsafe;
- unsafe executed-transition rate: unsafe executed / known executed;
- false-safe rate: unsafe executed / all known unsafe.

The evaluator reports Wilson 95% intervals for every rate. With zero observed failures and nonzero n, the upper bound is nonzero; zero failures in a finite sample is not proof of zero risk.

The evaluator checks declared adjudicator independence and provenance fields but does not verify signatures or source bytes. That remains the job of the end-to-end claim verifier. It also does not decide whether an action is physically unsafe: it consumes the independent adjudication result.

## Operational dimensions

Episode elapsed latency and per-action decision latency, tokens, API cost, solver calls/time, observation requests and peak memory are summarized independently. Distribution reports include observed count and sum as well as mean/median/P95/CVaR95/max, so total resource costs can be compared directly. Missing operational data remains missing and is never imputed. Do not collapse quality, tail risk, safety, runtime and cost into one weighted scalar by default.

## Run

    cargo fmt --manifest-path tools/power-restoration-risk-metrics/Cargo.toml --package power-restoration-risk-metrics -- --check
    cargo clippy --locked --manifest-path tools/power-restoration-risk-metrics/Cargo.toml --package power-restoration-risk-metrics --all-targets -- -D warnings
    cargo test --locked --manifest-path tools/power-restoration-risk-metrics/Cargo.toml --package power-restoration-risk-metrics --all-targets

Input schema: fixtures/risk-metrics-input-v0.1.schema.json

## Tests

The authored tests cover nearest-rank P95, fractional CVaR tail, strict and conservative episode accounting, cohort mismatch, duplicate IDs, policy mismatch, evidence/adjudicator requirements, unknown classification, guard precision/recall, unsafe executed action rate, Wilson intervals and operational missingness.

Tests are authored; they are not passed until exact-head Rust CI executes.

## Claim boundary

This crate computes statistics from declared scores and independently supplied adjudication labels. It does not independently validate AC/three-phase/protection feasibility, physical solver correctness, the guard implementation, evidence bytes or field safety. A valid result is a statistical report, not an electrical safety certificate.
