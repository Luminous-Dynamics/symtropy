# Net additionality under substitution uncertainty and circular rebound — v0.1

**Status:** synthetic research benchmark; not a production sustainability claim.

## Research question

Can an intervention that maximizes nominal recovered content be reliably treated as the best environmental decision? This benchmark makes the counterfactual explicit: recovered material only displaces virgin material to the extent that it is recoverable, meets quality requirements, and actually substitutes for virgin demand. It then asks whether any per-unit advantage survives additional demand and changes in embodied/process burdens.

The experiment follows the measurement discipline described by ISO 59020: define a functional unit and system scope, keep data traceable, state assumptions, and avoid treating a circularity indicator as a complete environmental-impact assessment. ISO is developing a second edition of ISO 59020; the cited work item is still a working draft, not a replacement published standard:
- https://www.iso.org/standard/80650.html
- https://www.iso.org/standard/94166.html

Research on circular-economy rebound effects identifies pathways by which efficiency/circularity gains may be offset by changed demand: Castro et al. (2022), DOI https://doi.org/10.1016/j.jclepro.2022.131136. Recent work on digital product passports likewise highlights gaps between traceability attributes and data needed for representative lifecycle inventories: Kleijn et al. (2026), DOI https://doi.org/10.1007/s44498-026-00144-7.

## Scope and accounting

The frozen manifest describes 100 synthetic service units under one accounting horizon. The same virgin-material requirement applies to every option. The input includes a declared embodied-GHG factor, action process burden, fixed-point recovery yield, a claimed substitution ceiling, rebound sensitivity, and cost. Three scenarios define probability weights, a market substitution cap, rebound response, and virgin/process GHG multipliers.

All material quantities are integer grams; costs are integer cents; rates are basis points (10,000 bp = 100%). The fixture intentionally uses small bounded values so the complete option × scenario space can be enumerated.

For option a and scenario s:

1. **Yield-adjusted recovery** = floor(gross recovered mass × recovery yield / 10,000). The displayed gross-circularity score is capped at the virgin-material requirement per service unit, so excess recovered mass cannot inflate the ranking.
2. **Effective substitution** = min(option claim, scenario substitution cap).
3. **Virgin material displaced per service unit** = min(virgin requirement, floor(yield-adjusted recovery × effective substitution / 10,000)).
4. **Remaining virgin burden** = remaining virgin grams × embodied GHG per gram × scenario virgin multiplier, rounded upward.
5. **Rebound-adjusted demand** = base demand plus the ceiling-rounded additional demand from option rebound sensitivity × scenario rebound multiplier.
6. **Total synthetic burden** = rebound-adjusted demand × (virgin burden per unit + scenario-scaled process burden per unit).

Benefits are not credited when evidence is unknown, revoked, incomplete, claim-mismatched, or out of scope. Every evidence digest is recomputed from the exact UTF-8 payload. Unknown data are not interpreted as zero environmental impact. Invalid source digests reject the whole manifest.

Conservative rounding is used for recovered material and displacement (floor, avoiding over-credit) and for burdens/additional demand (ceiling, avoiding under-count). Expected metrics use basis-point probability weighting and half-up rounding.

## Decision rules compared

- **Gross-circularity policy:** chooses the evidence-eligible option with the highest yield-adjusted recovered mass per service unit.
- **Expected-impact policy:** chooses the option with the lowest probability-weighted total synthetic GHG burden over the declared scenarios.
- **Minimax-regret policy:** chooses the option with the smallest worst-case burden gap to the scenario-specific hindsight oracle.
- **Hindsight oracle:** for each scenario, identifies the eligible option with the lowest burden in that scenario. It is a comparison bound, not deployable intelligence; real decisions cannot know the realized scenario in advance.
- **Expected GHG/cost frontier:** retains eligible options not dominated on both expected GHG burden and expected cost. Material loops are diagnostic rather than added as a third reward, so the benchmark does not automatically favor additional throughput.

Tie-breaks are deterministic by lexicographic option ID. Input option/scenario order must not alter decisions. The benchmark rejects probability weights that do not sum to exactly 10,000 bp and bounds the exhaustive search space.

## Frozen fixture: expected outcomes

The reference fixture is deliberately adversarial:
- Gross-circularity winner: **high-loop**.
- Expected-impact winner under the declared probabilities: **high-loop**.
- Minimax-regret winner: **verified-reuse**.
- Per-scenario oracle: **high-loop** for stable-supply, **baseline-virgin** for weak-displacement, and **verified-reuse** for carbon-stress.
- Expected GHG/cost frontier: **high-loop** and **verified-reuse**.
- A nominally attractive **unverified-loop** candidate is excluded without a score.

Expected total synthetic GHG burden: high-loop 40,776 gCO₂e; verified-reuse 41,361 gCO₂e; remanufacture 43,148 gCO₂e; virgin baseline 53,520 gCO₂e. Expected burdens are scenario probability-weighted, then rounded to the nearest gram. The robust policy differs from the expected policy because weak-displacement scenario regret is large for high-loop. This does not mean the robust policy is always superior; it illustrates which declared risk preference changes the decision.

A scenario mutation that removes all substitution credit from the weak-displacement state should reverse the expected-impact selection from high-loop to verified-reuse. This is a falsifiable ranking-sensitivity test.

## Benchmark ladder and proposed quantitative targets

These are engineering targets to adopt as the benchmark matures, not ISO requirements and not results already demonstrated by this synthetic fixture.

### Phase A — deterministic contract

- **100% independent parity** for every eligible candidate × scenario outcome.
- **0 unearned benefit credits:** invalid/unknown/partial/out-of-scope evidence cannot receive a decision score; displacement cannot exceed yield-adjusted recovery or the functional unit's virgin-material requirement.
- **100% probability reconciliation** to exactly 10,000 bp; no silent overflow/wraparound.
- **100% deterministic receipts** for an identical manifest and source revision; option/scenario input ordering must not affect policy decisions.
- Expected-impact selection must be the exact minimum of declared expected burden; minimax regret must be the exact minimum over the same declared option set. Both must be no worse than any candidate under their own objective.

### Phase B — adversarial model validation

- Run at least **10,000 seeded generated manifests** across boundary rates (0/10,000 bp), changing scenario probabilities, hostile/missing evidence, reordered records, and near-overflow values.
- **Zero crashes, wrapped arithmetic, eligibility leaks, or oracle/objective invariant violations** across accepted and rejected inputs.
- On every accepted generated case, cross-check against a separately implemented integer reference or exhaustive finite oracle.
- Report normalized regret alongside absolute units when scenarios use different scale; publish the full scenario set and seed rather than just a leaderboard.

### Phase C — measured-data promotion

- Keep synthetic, measured, estimated, and externally sourced inputs distinguishable, each with scope, timestamp, provenance, and uncertainty.
- For any future declared **90% predictive interval**, target at least **90% empirical coverage** on held-out assets/sites, with absolute calibration gap no greater than **5 percentage points** across materially different asset classes. These criteria require an interval model and are not yet implemented in v0.1.
- Do not claim net environmental benefit unless the measured, system-boundary-consistent counterfactual indicates an improvement and uncertainty does not obscure the direction. Publish cases where circular actions are worse as well as better.
- Evaluate at least **1,000 held-out scenario draws** before representing a policy as robust beyond the declared finite fixture; report median and 95th-percentile normalized regret.

### Phase D — performance and independent reproduction

- Proposed throughput target: process **4,096 candidate × scenario cells in under 250 ms median and 1 s p95** on a pinned, documented reference runner. This is a target to measure, not a claim about current performance.
- Require an independent reproducer to verify source digests, rerun the exact manifest, reproduce the receipt, and confirm the interpretation limits before any public real-world claim.

## Qualification and promotion gates

- Recomputed source digests, strict JSON schema, unique identifiers, valid bounds, and exact scenario-probability reconciliation.
- Independent direct-equation reference replay for every eligible option × scenario pair.
- Tests for unknown/partial evidence, corrupted digests, altered scenario weights, order invariance, ranking reversals, and Pareto dominance.
- Exact-head rustfmt, unit tests, and receipt emission must pass before qualification; queued means unqualified.
- Synthetic benchmark success cannot be reported as measured lifecycle improvement. Promotion needs verified primary inventory data, a traceable counterfactual, representative temporal/spatial boundaries, measured service delivery and rebound assumptions, and uncertainty calibrated on held-out observations.
