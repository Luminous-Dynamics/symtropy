# AV Decision-Sufficient Observation Benchmark v0

**Status:** synthetic-first benchmark contract and paired fixtures; no policy is integrated or qualified.  
**Issue:** [#6222](https://github.com/Luminous-Dynamics/symtropy/issues/6222)  
**Parent:** [#6183 AV observation world](https://github.com/Luminous-Dynamics/symtropy/issues/6183)  
**Consumer:** [Symthaea #12837 AV-STREAM-001](https://github.com/Luminous-Dynamics/symthaea/issues/12837)

## Competitive benchmark comparison

See [COMPETITIVE_LANDSCAPE.md](COMPETITIVE_LANDSCAPE.md) for the November 2026 comparison set (HERMES, StreamScout, OASIS, LiveProBench, SPOT-Bench, OmniInteract, IPIBench, P3IBench, DCASE and LOCATA), and [TOURNAMENT_PROTOCOL.md](TOURNAMENT_PROTOCOL.md) for the preregistered policy arms, hard gates, metrics, statistics and promotion sequence.

## The benchmark question

Can an audiovisual agent request the next observation that changes its decision, rather than simply requesting the observation with the greatest expected entropy reduction?

The benchmark distinguishes three non-interchangeable criteria:

- **EIG:** expected reduction in uncertainty over a declared hypothesis distribution.
- **EVSI:** expected reduction in optimal terminal decision loss after observing, under the supplied decision/loss model.
- **Probability-free discrimination:** worst-case residual ambiguity over a declared finite hypothesis set when calibrated probabilities are not defensible.

This package is a scenario adapter and a known-answer fixture corpus. It does **not** implement another EIG/EVSI/active-sensing planner. Consume the existing owners:
- Symthaea #7946 for spatial-acoustic EIG sensing.
- Symtropy #6045 for risk-constrained EVSI.
- Symtropy #5774 for probability-free measurement selection.
- Symthaea #3934 VIS-006 / #12837 A5 for active visual and audiovisual observation.
- Symthaea #9075 / #9177 for as-known retrieval and proof-aware sufficient context.
- Symtropy #6183 / #6201 for hidden-truth observation/replay boundaries.

## Fixture separation

- `fixtures/agent/agent-inputs-v1.json` contains the task, currently admitted evidence, public candidate hypotheses, supplied belief/observation models, decision loss, request catalogue, and budgets.
- `fixtures/evaluator-only/evaluator-oracle-v1.json` contains the realized hidden world, actual observation outcomes, and source-dependence truth.
- `fixtures/expected/known-answer-scorecard-v1.json` contains independently authored expected dispositions and arithmetic checks.

The directories are a packaging convention, **not a security boundary**. A competitive run must copy the agent fixture to an agent-only root and the oracle to a separate evaluator-only root/process. Do not mount both into the policy process. This repository package itself does not prove OS isolation.

## High-EIG / zero-EVSI known answer

Episode `av-dso-001` supplies four equally likely hypotheses:
- H1 and H2 are hazard states; H3 and H4 are benign states.
- The best terminal action is either `warn` or `stay_silent`.
- False warning costs 40; missing a hazard costs 100.

Two candidate observations are available:

1. `subtype_detail` perfectly distinguishes H1 from H2 and H3 from H4. It yields `ln(2) = 0.693147...` nats of EIG, but leaves hazard probability at 0.5 in either branch. The best terminal action remains `warn`; EVSI is exactly 0.
2. `hazard_cue` has P(alarm | hazard)=0.8 and P(alarm | benign)=0.2. Its EIG is approximately 0.192745 nats—lower than `subtype_detail`. Yet it changes the terminal action between alarm and quiet branches. Current expected loss is 20; expected post-observation loss is 14; EVSI is 6 utility units. With a declared cost of 2, net decision value is +4.

This is the core counterexample: **the most informative observation is not the most decision-useful observation.**

The numbers are synthetic and serve only as an arithmetic known answer. They are not calibrated real-world probabilities or welfare valuations.

## Additional integrity/decision cases

The paired corpus also includes:
- robust stop while hypothesis entropy remains nonzero;
- an informative result that arrives after the response deadline;
- a high-value request excluded by a hard latency/energy budget;
- a privacy/permission-ineligible request that must never be selected;
- two sensor views with one common capture root (not independent corroboration);
- uncalibrated priors plus an incomplete observation partition (EIG/EVSI must remain unquantified);
- late historical evidence that cannot be retroactively treated as known at an earlier decision time.

## Required tournament arms

Compare on the same frozen episodes, evidence, task losses and request budgets:
1. no further request / current best decision;
2. fixed-rate acquisition;
3. seeded random request;
4. recency-only retrieval;
5. surprise/entropy trigger;
6. EIG-per-cost (#7946);
7. risk-constrained EVSI (#6045);
8. probability-free minimax partitioning (#5774), only when its closed-set assumptions hold;
9. OASIS-style event retrieval, StreamScout-style stop/escalate, and HERMES-style streaming memory where matched implementations and budgets are available;
10. clairvoyant evaluator ceiling, diagnostic only and never a competitive policy.

A weak strawman is not a valid comparison. Keep the best practical modular audiovisual stack as the baseline.

## Scorecard

Report independently:
- terminal decision accuracy and realized regret;
- current versus post-observation Bayes risk;
- EIG and realized entropy change, only on calibrated probability tracks;
- EVSI and net EVSI after explicitly declared costs;
- worst-case residual set size/diameter on probability-free tracks;
- evidence sufficiency, unsupported claims, abstention and stop decisions;
- deadline misses, age-of-information, request-to-arrival and source-to-decision latency;
- bytes/tokens, compute, energy (measured versus synthetic), privacy footprint and interruption burden;
- common-cause false-independence rate and prohibited request rate;
- p50/p95/p99 for acquisition-to-decision latency.

Do not compress these into a single weighted “intelligence score.” A forbidden sensor request, hidden-truth leak, false independence claim or missed hard deadline is a separate failure.

## Promotion gates

Proposed, not achieved:
- 100% agreement with finite known-answer dispositions and arithmetic.
- 0 hidden-world/future-observation leakage.
- 0 request outside declared permission, capability or hard budgets.
- 0 fabricated probability or incomplete outcome partition treated as complete.
- 0 common-cause paths treated as independent without qualified evidence.
- 100% correct stopping on robust-stop fixtures.
- deterministic replay for identical fixture/policy revisions.
- at least one preregistered family where EVSI improves paired decision regret or all-in acquisition cost versus EIG/entropy-only without weakening hard gates.

## Executable fixture audit

The isolated Rust package performs a finite known-answer audit; it does not select a request.

Run from the repository root:

```sh
cargo test --locked --manifest-path research/av-decision-sufficient-observation-v0/Cargo.toml

cargo run --locked --manifest-path research/av-decision-sufficient-observation-v0/Cargo.toml -- \
  research/av-decision-sufficient-observation-v0/fixtures/agent/agent-inputs-v1.json \
  research/av-decision-sufficient-observation-v0/fixtures/evaluator-only/evaluator-oracle-v1.json \
  research/av-decision-sufficient-observation-v0/fixtures/expected/known-answer-scorecard-v1.json
```

The audit independently recomputes Shannon EIG and finite Bayes decision risk/EVSI for the first paired counterexample, checks all seven public/oracle/expected episode IDs, checks that realized outcomes are consistent with the declared model except for the explicit open-world negative, and verifies deadline, permission, budget and dependency-group metadata.

The CLI takes three explicit paths. It does not compile evaluator truth into the executable or pass oracle values to an agent. CI runs the audit in the evaluator context. Unit tests use `include_str!` for the fixtures and therefore must not be treated as an agent runtime or as evidence of process isolation.

## Next integration boundary

The next implementation step is a thin request/response adapter from Symthaea's A5 policy to Symtropy's scenario executor. Requests remain advisory. Symtropy checks capability, consent, privacy, deadline and budget, then produces an observation or explicit rejection receipt. No new generic planner, memory engine, renderer, sensor driver, clock service, or authority system belongs here.

## Claim ceiling

This synthetic corpus can qualify the declared finite decision semantics only. It cannot establish calibrated probabilities, real-world utility, camera/microphone performance, privacy-law compliance, safe physical movement, human interaction quality, or SOTA superiority.
