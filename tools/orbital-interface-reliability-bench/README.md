# Orbital Interface Reliability Claim Benchmark v0.1

**Status:** synthetic reference implementation; exact-head validation pending.  
**Owner:** [ORBITAL-INTERFACE-RELIABILITY-001 #6548](https://github.com/Luminous-Dynamics/symtropy/issues/6548).  
**Claim ceiling:** synthetic independent-mission binomial semantics only.

## Why this exists

A campaign can cover its test obligations and perform every planned cycle without having sufficient independent statistical evidence to claim a target mission reliability at a stated confidence level.

The v0.1 profile is deliberately narrow: one declared, representative mission outcome per independently selected article under a simple-random single-lot model. It computes an exact one-sided Clopper-Pearson lower bound on the success probability.

For (n) trials, (f) failures and confidence (C), v0.1 solves for the upper failure probability (q_U) satisfying:

`P(X \le f \mid X \sim Binomial(n,q_U)) = 1-C`

and reports `R_lower = 1 - q_U`. With zero failures this reduces to `R_lower = (1-C)^{1/n}`.

The minimum zero-failure sample size is:

`n = ceil(log(1 - C) / log(R_target))`

Known answers at 90% confidence:
- 90% mission reliability: 22 independent article trials;
- 95%: 45 independent article trials;
- 99%: 230 independent article trials.

These counts are consequences of the independent Bernoulli mission-outcome model. They are not recommendations for the physical qualification campaign and do not mean repeated cycles of a single article are independent trials.

## What the evaluator will and will not admit

The evaluator requires exact population, lot, configuration, mission profile, failure definition, pre-registration, sampling-frame reference and dependence-assessment reference. Each trial must identify one article and one declared sampling cluster.

- One article is one experimental unit in v0.1.
- Byte-identical replay of one trial ID is DuplicateNoOp.
- A second distinct trial ID for the same article is pseudoreplication (Indeterminate) or Disputed if outcomes/bytes conflict.
- A repeated sampling-cluster ID is Indeterminate.
- Restricted outcomes count as failures against full mission success.
- Aborted, withdrawn, invalid, right-censored, missing-evidence or intervened-during-mission trials make the v0.1 claim Indeterminate.
- Population/configuration/mission/failure-definition/lot mismatch is Rejected.
- Convenience sampling, multi-lot pooling, recurrent-event evidence, wear-out or right-censoring is outside the v0.1 model.

A digest's shape, declared evidence flags and review references are input fields, not authenticated measurements or proof of independence. V0.1 does not validate a real sampling frame, calibration, producer identity or dependence assessment. It counts distinct declared articles/clusters only under the declared assumption.

## Why cycle count is a different baseline

The CLI and report include a deliberately naive cycle-as-independent-trials baseline, calculated on the same mathematical method. It is not admissible for the mission reliability result. It helps expose false assurance: e.g. 22 successful mating cycles on one physical article may look like 22 independent successes, but the correct experimental unit remains one article and its one mission outcome.

The v0.1 corpus also includes naive observed success fraction, to show that 100% observed success is not a high-confidence reliability demonstration at small sample sizes.

## Industry and standards comparators

- **MIL-HDBK-781 Rev B** is listed by DLA as active (29 July 2026) and provides reliability test methods, plans and environments for development, qualification and production. It is a plan-selection baseline, not replaced by this profile: https://quicksearch.dla.mil/qsDocDetails.aspx?ident_number=54097
- **IEC 61124:2023** specifies compliance test plans for failure rate/intensity, MTTF and MTBF. Its default model assumes independent, identically exponentially distributed operating times / constant failure rate. This does not automatically describe wear-out, seal deterioration, fatigue or recurrent mission cycles: https://webstore.iec.ch/en/publication/65838
- **NASA-STD-5017B** separately requires life testing of all mechanism functions, including backup/redundant functions, and service-life cycle requirements. Life-cycle coverage is not itself a population confidence bound: https://standards.nasa.gov/standard/NASA/NASA-STD-5017
- **NASA reliability-demonstration guidance** discusses incorporating design and process assurance and fault-detection probability instead of relying on simple binomial sample counts alone. That does not license unsupported prior assumptions: https://ntrs.nasa.gov/archive/nasa/casi.ntrs.nasa.gov/20100035126.pdf

## Explicitly separate future statistical profiles

Do not apply the binomial mission-success result to:
- constant-failure-rate/exponential MTBF testing (MIL-HDBK-781 / IEC 61124);
- repairable-system recurrent events;
- Weibull or other wear-out/survival models;
- accelerated life tests;
- condition/degradation monitoring;
- Bayesian or reliability-growth models;
- fleet availability or maintainability.

Each requires its own estimand, exposure, censoring, independence assumptions, model-validation evidence and oracle.

## Frozen corpus and commands

The v0.1 corpus contains 36 synthetic cases covering exact sample-size boundaries, failures-allowed confidence bounds, duplicate trial IDs, repeat article serials, shared declared clusters, replacement after failure, withdrawn/aborted/invalid/censored outcomes, configuration/lot/mission mismatch, convenience sampling, restricted outcomes, cycle pseudoreplication and model boundaries.

```bash
cargo fmt --manifest-path tools/orbital-interface-reliability-bench/Cargo.toml -- --check
cargo test --manifest-path tools/orbital-interface-reliability-bench/Cargo.toml --all-targets
cargo clippy --manifest-path tools/orbital-interface-reliability-bench/Cargo.toml --all-targets -- -D warnings
cargo run --manifest-path tools/orbital-interface-reliability-bench/Cargo.toml --bin orbital-interface-reliability -- \
  tools/orbital-interface-reliability-bench/fixtures/v0.1/reliability-cases.json \
  --source-commit <40-lowercase-hex-commit>
node tools/orbital-interface-reliability-bench/tests/reference_oracle.mjs \
  tools/orbital-interface-reliability-bench/fixtures/v0.1/reliability-cases.json
```

## Composition and boundaries

Composes with:
- #6540 campaign coverage;
- #6484/#6485 lifecycle qualification;
- #6515/#6516 exact event replay;
- #6256/#6303 physical test evidence;
- #5062 metrology uncertainty;
- #6519 signed authority evidence.

This evaluator does not produce a reliability physics model or operational approval. A passing synthetic corpus does not demonstrate real hardware reliability, test population representativeness, independent sampling, flight readiness or mission assurance.
