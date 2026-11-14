# Maritime Monte Carlo randomization inference v0.1

**Owner:** [MARITIME-RANDOMIZATION-INFERENCE-001 #5115](https://github.com/Luminous-Dynamics/symtropy/issues/5115)  
**Parent estimator:** [MARITIME-MAINTENANCE-EFFECT-001 #5108 / PR #5111](https://github.com/Luminous-Dynamics/symtropy/pull/5111)  
**Status:** source, schemas, protocol, test manifest and exact-head workflow authored; hosted qualification pending  
**Evidence class:** deterministic synthetic sampler tests; no external maintenance trial has been re-randomized or analyzed.

## Why this is separate from the estimator

PR #5111 implements a blocked-randomized finite-population ATE and a two-sided exact Fisher randomization test. Exact enumeration is deliberately bounded to 16 independent units. The exact implementation remains the reference for small assignment spaces.

This module is only a scale-up of the **randomization reference distribution**. It does not alter the ATE estimator, observational identification assumptions, or the measurement-uncertainty model.

## Schemas and hosted benchmark evidence

- `monte-carlo-randomization-config.schema.json` is a closed Draft 2020-12 configuration schema. It accepts the complete unsigned 64-bit seed range and the frozen draw-count bounds.
- `monte-carlo-randomization-result.schema.json` is a closed result schema, including zero allocation violations, no exact-enumeration claim and false authorization fields.
- `monte-carlo-randomization-result.example.json` is an explicitly synthetic example validated against the result schema in the exact-head workflow.
- The workflow checks the exact `u64::MAX` value (`18446744073709551615`) in both config and result schemas. This guards against schema generators that round the largest integer through IEEE-754 floating point.
- The exact-head workflow executes the fixed-budget throughput harness, records the commit, runner architecture, Rust compiler metadata and CPU information, then uploads the JSONL profile for 14 days. Each JSONL row binds the algorithm, seed, draw count, observed statistic, exceedance count, raw and plus-one p-values, Monte Carlo uncertainty and replay digests.
- The v0.1 throughput harness does not measure peak memory. It reports elapsed microseconds, draws/second and unit-draws/second only; memory remains a separately instrumented benchmark task.

## Algorithm contract

Rust module: `src/monte_carlo_randomization.rs`.

- **Assignment space:** all assignments satisfying the frozen treatment/control counts in each declared stratum.
- **Sampler:** partial Fisher–Yates within each stratum. Bounded integer selection uses rejection, not biased modulo reduction. A separate verifier recounts every generated stratum before the statistic or assignment digest is updated; a malformed block fails closed.
- **PRNG:** SplitMix64 with frozen constants and a golden-vector test. It is a deterministic simulation PRNG, not a cryptographic generator.
- **Randomization:** each sampled assignment is uniform over the admissible fixed-count block assignment space. Draws are with replacement.
- **Statistic:** absolute value of the same stratum-size-weighted difference in observed means used by the exact reference.
- **Tail rule:** count sampled statistics at least as extreme as the observed statistic, with the same (10^{-12}) tie tolerance.
- **Finite-sample correction:** (p_{MC}=(b+1)/(B+1)), where (b) is the exceedance count and (B) is the fixed draw budget.
- **Monte Carlo uncertainty:** report the raw tail estimate, its Monte Carlo standard error and a 95% Wilson score interval. These describe uncertainty in the estimated tail probability, not uncertainty in the ATE.
- **Budget:** 100–2,000,000 draws, at most 10,000 units, and at most 100,000,000 unit-draw operations. The budget is checked before sampling.

The plus-one correction is not cosmetic. Phipson and Smyth showed why randomly drawn permutation p-values should not be reported as zero and derived the appropriate correction for sampled permutation tests. The module uses that correction and keeps the uncorrected tail estimate as a separate field:
- Phipson & Smyth (2010), [Permutation P-values Should Never Be Zero](https://doi.org/10.2202/1544-6115.1585).
- [Exact testing with random permutations](https://pmc.ncbi.nlm.nih.gov/articles/PMC6405018/) provides further formal treatment of Monte Carlo permutation-test validity and assumptions.

SplitMix was described by Steele, Lea and Flood as a fast splittable pseudorandom generator; this implementation freezes a simple SplitMix64 stream for deterministic replay. That citation does not imply cryptographic quality or seed fairness:
- Steele, Lea & Flood (2014), [Fast Splittable Pseudorandom Number Generators](https://gee.cs.oswego.edu/dl/papers/oopsla14.pdf).

## Replay and provenance

The result binds:
- the frozen trial assignment-plan digest;
- a canonical randomization-frame digest over the full serialized trial, after sorting observations by unit ID and allocations by stratum ID (therefore binding the full source, calibration, timing, quality and uncertainty contract);
- the algorithm ID/version, unsigned 64-bit seed and fixed draw count;
- a digest over every sampled assignment in canonical unit-ID order;
- exceedance count, plus-one p-value, raw tail estimate, Monte Carlo standard error and Wilson interval.

Reordering observation rows or allocation declarations must not change the randomization-frame digest, sampled-assignment digest or p-value. The PRNG golden-vector test freezes the state transition; a test changes the seed and verifies a different sampled-assignment digest.

The deterministic seed makes a run reproducible; it does **not** prove that an externally selected seed was unbiased, that an assignment plan was executed, or that the observed trial was actually randomized.

## Falsification and benchmark suite

The manifest contains sixteen scenarios:
1. byte-identical replay for identical seed and inputs;
2. different seed changes the sampled-assignment digest;
3. plus-one correction never returns zero;
4. Monte Carlo interval checked against exact small-sample Fisher inference;
5. support for 17 units beyond the exact enumeration boundary;
6. invalid draw counts rejected;
7. SplitMix64 golden-vector integrity;
8. row-order invariance;
9. total unit-draw work budget enforced before sampling;
10. uniform subset frequencies within a fixed-count stratum;
11. zero blocked-allocation constraint violations;
12. sampling/instrument/ATE uncertainty and authorization channels remain distinct;
13. a valid 10,001-unit trial is rejected before sampling;
14. the fixed-budget Rust throughput profile emits replay-bound metrics without a machine-independent speed threshold;
15. an independent recount rejects an invalid sampled block before statistic evaluation;
16. zero exceedances still produce the plus-one minimum p-value.

The Rust suite currently contains sixteen tests. The `cargo bench --bench monte_carlo_randomization` harness sweeps 16, 32, 64, 128, 256, 1,000 and 10,000 units with 1,000 fixed draws and emits JSONL timing/throughput/replay records. The exact-head workflow runs the harness and uploads the JSONL output together with Rust and runner/CPU metadata. The JSONL records include algorithm ID, seed, draw budget, observed statistic, exceedance count, raw and corrected p-values, Monte Carlo uncertainty, frame digest and sampled-assignment digest. It includes a 30,000-draw differential example against the exact 12-unit/three-stratum reference. The Wilson interval must contain the exact tail probability in that frozen seed case. This is a deterministic regression test, not a claim of 95% global coverage; global coverage needs many independently generated trials. The v0.1 harness measures elapsed time, draws/second and unit-draws/second; it does not yet measure peak memory.

## SOTA comparison and next benchmarks

Compare on the same frozen trial tapes:
- exact Fisher enumeration for every assignment space that can be fully enumerated;
- Monte Carlo fixed budgets of 100, 1,000, 10,000, 100,000 and 1,000,000 draws;
- alternative validated PRNGs/samplers only after a versioned protocol and seed contract exists;
- a rejected modulo-reduction baseline to quantify bias in bounded integer generation;
- a rejected unseeded/runtime-random baseline to expose replay loss.

Report:
- absolute Monte Carlo p-value error against exact enumeration;
- empirical Wilson-interval coverage of exact tail probabilities;
- p-value bias and Type-I error under the sharp null;
- power under fixed alternatives;
- distribution of Monte Carlo standard errors and interval widths;
- uniform assignment-frequency deviations in small enumerable blocks;
- digest replay and row-order-invariance rate;
- measured elapsed time, draws/second, unit-draws/second and total-work behaviour at 16, 32, 64, 128, 256, 1,000 and 10,000 units;
- peak memory only when separately instrumented with a platform-specific, per-size process measurement.

**Aspirational targets (not achieved):**
- 100% byte-identical replay for the same algorithm, seed, draw budget and trial frame;
- zero block-allocation violations and zero p=0 outputs;
- at least 99% empirical coverage by the nominal 95% Wilson interval across a large held-out exact-reference suite, with binomial uncertainty reported;
- median absolute p-value error decreases with increasing fixed draw budgets;
- no post-hoc stopping, hidden seed changes or PRNG swaps;
- a million-draw run at 1,000 units remains below the frozen resource budget.

Do not claim Monte Carlo is better than exact enumeration. Exact is the standard when the assignment space is tractable; Monte Carlo extends the same reference distribution when it is not.

## Industry, standards and research comparison

This module is a statistical inference primitive, not a replacement for industrial anomaly detection or OEM service expertise.

**Wärtsilä Expert Insight** is a strong marine comparator for the complete detect → investigate → intervene service loop. In Wärtsilä's published Aurora Spirit case, the service flagged abnormal turbocharger-inlet lube-oil pressure. Experts considered sensor error or an obstruction, requested onboard checks, then Technical Services confirmed a turbocharger problem and recommended taking the engine offline as a precaution. Inspection identified broken compressor-side bearing location pins; a service engineer completed the repair during the voyage, in time for the vessel's dynamic-positioning work. This illustrates why diagnosis, independent verification and operational response must be evaluated as separate stages. The public case does not publish a controlled randomization distribution or Monte Carlo inference benchmark for repair effects:
- [Wärtsilä Aurora Spirit case](https://www.wartsila.com/insights/case-study/aurora-spirit)
- [Wärtsilä Expert Insight](https://www.wartsila.com/marine/services/lifecycle-agreements/expert-insight-service)

**ABB's 2026 Digital Powertrain portfolio** now describes two service models: expert-led Predictive Intelligence, combining ABB experts with analytics and continuous “what to fix and when” guidance; and self-service Digital Powertrain Insights, providing real-time asset-health insights and analytics-driven alerts for in-house teams. This matters for comparison: the benchmark should compare against both human-in-the-loop expert services and self-service fleet intelligence, not a generic dashboard:
- [ABB Ability Digital Powertrain portfolio (2026)](https://library.e.abb.com/public/ad6138cb14d5433d85a2e4fe43932009/4MWA000447_ABB_Ability_Digital_Powertrain_Drives_Enabling%20Hardware_RevA_Nov2026.pdf)
- [ABB Digital Powertrain Insights](https://www.abb.com/global/en/areas/motion/services/asset-health-and-monitoring/abb-ability-digital-powertrain-insights)

The P0 statistical differentiator is deliberately narrower: reproducible sampling from a frozen blocked-randomization assignment space, exact-small-sample differential testing, fixed-budget Monte Carlo uncertainty and canonical replay evidence. Public product pages do not publish comparable randomization-inference benchmarks. This is a limitation of public comparative data, not evidence that providers lack internal statistical methods.

Relevant standards are guidance, not claims of conformance:
- **ISO 13379-1:2025**, edition 2, provides general guidelines for condition-monitoring data interpretation and diagnostic techniques: https://www.iso.org/standard/88027.html
- **ISO 13381-1:2025**, edition 3, provides guidelines and requirements for machinery prognostics processes: https://www.iso.org/standard/88029.html

These standards are complementary to the maintenance-effect and diagnostic evidence architecture; neither defines this Monte Carlo algorithm nor certifies the implementation.

Research methods address different layers and should not be ranked on one number:
- **Najafi & Taghipour (2026)** model stochastic imperfect-repair effects and regime-dependent feasibility inside a semi-Markov inspection/maintenance policy. This is a downstream decision-policy comparator, not a randomization p-value benchmark: https://doi.org/10.1016/j.cie.2026.112109
- **Nieminen (2026)** describes a physics-grounded front-door estimator for virtual hydraulic pump repair from sensor logs, with mediator-overlap checks and bootstrap uncertainty. It is an observational identification route on a controlled hydraulic test rig, not a substitute for randomized assignment or Monte Carlo randomization inference: https://doi.org/10.1016/j.eswa.2026.133024
- **Phipson & Smyth (2010)** motivate plus-one correction for sampled permutation p-values: https://doi.org/10.2202/1544-6115.1585
- **Steele, Lea & Flood (2014)** describe fast splittable pseudorandom generators; this supports deterministic simulation, not cryptographic seed fairness: https://gee.cs.oswego.edu/dl/papers/oopsla14.pdf

The fair comparison ladder is:
1. Reproduce exact Fisher inference wherever full enumeration is tractable.
2. Compare Monte Carlo estimators at identical draw budgets and frozen assignment frames.
3. Test uniformity, p-value error, Wilson coverage and Type-I calibration on many known-truth randomized trials.
4. Separately reproduce observational front-door / target-trial methods on their own data and assumptions.
5. Evaluate downstream policy value only through #4545, with action feasibility and cost assumptions exposed.

Do not claim Monte Carlo is better than exact inference. Exact enumeration is the reference when it is tractable; Monte Carlo extends that reference distribution when it is not.

## Claim ceiling

A pass establishes only a reproducible fixed-budget approximation of the declared blocked randomization distribution for the exact PRNG, seed, draw count and trial frame. It does not prove the external trial was randomized correctly, seed unbiasedness, causal assumptions, intervention effectiveness on a real engine, safe remaining life, seaworthiness, class approval, maintenance authorization or permission to operate.
