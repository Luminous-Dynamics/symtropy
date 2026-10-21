# EV factory epistemic parameter sensitivity v0.1

**Benchmark:** `EV-FACTORY-EPISTEMIC-001`  
**Issue:** [#2455](https://github.com/Luminous-Dynamics/symtropy/issues/2455)  
**Implementation:** `tools/ev-passenger-cost-frontier/src/bin/ev_factory_parameter_sensitivity.rs`  
**Status:** synthetic controlled sensitivity screen; not a calibrated factory model

## Paradigm shift: more random seeds do not validate the inputs

A simulator can estimate stochastic variability precisely under the wrong cycle-time, yield, supplier or reliability assumptions. We therefore separate:

- **Aleatory uncertainty:** event-level variability across the same declared 20 seeds while holding the catalog fixed.
- **Input/model uncertainty:** response changes between explicit low, nominal and high settings of a declared input factor.
- **Structural uncertainty:** omitted or incorrectly modeled mechanisms, which this screen does not quantify.
- **Measurement uncertainty:** error in any future PLC/MES/quality observation, which also remains outside this synthetic fixture.

This separation follows recent manufacturing simulation research that explicitly handles aleatory and epistemic uncertainties plus product variance (Frey et al., *Journal of Manufacturing Systems*, 2025, https://doi.org/10.1016/j.jmsy.2025.06.007). NIST's October 2026 guidance likewise emphasizes risk-based credibility, VVUQ and the connection between simulation uncertainty and metrology practice (https://doi.org/10.6028/NIST.AMS.100-85); its manufacturing digital-twin workshop report prioritizes interoperability, VVUQ, cybersecurity and workforce readiness (https://doi.org/10.6028/NIST.IR.8620).

## Design

The current implementation is a one-factor-at-a-time screening plan with 13 distinct parameter settings: nominal plus low/high endpoint values for six factor groups.

| Factor | Low / high perturbation from nominal | Changed paths |
|---|---|---|
| Supplier inter-arrival | 0.75× / 1.25× | Mean supplier inter-arrival |
| Station cycle-time scale | 0.85× / 1.15× | Mean cycle time for every station |
| Yield envelope | −0.005 / +0.005 absolute fraction | Station good-yield values |
| Changeover scale | 0.5× / 2.0× | Changeover seconds for every station |
| MTBF scale | 0.5× / 2.0× | Mean time between failures for every station |
| MTTR scale | 0.5× / 2.0× | Mean time to repair for every station |

These are deliberately declared **synthetic engineering perturbations**, not confidence limits, empirical priors, likely plant values, or a sampled distribution over factories. The levels remain separate outputs and are never pooled as if they were additional stochastic replications. Some factors change a family of station fields together; the exact JSON paths appear in the output.

All 13 cases retain the same ordered 20-seed list. For each parameter setting, the tool validates the simulator invariants, checks per-seed baseline-to-intervention arithmetic, verifies that the simulation report contains the SHA-256 of its exact source catalog, and calculates seed-level baseline throughput and per-station intervention gain.

The sensitivity evaluator additionally masks only the factor's declared JSON-pointer paths and proves the low/high catalogs are otherwise identical to the nominal catalog. It rejects accidental co-variation, wrong seed association, missing seed/scenario pairs, stale/tampered catalogs, failed simulation invariants or non-synthetic evidence.

## What is reported

For every factor, low/nominal/high:
- Baseline good-units/hour distribution and the station added-server gain distribution.
- All 10 pairwise intervention comparisons among the five station candidates, based on per-seed effect-of-effects: `gain_A(seed) − gain_B(seed)`.
- Point-estimate ranking and robust winner for each parameter setting. A robust level winner must beat every alternative by the predeclared practical margin of 0.05 good units/hour under its paired 95% interval.
- The paired high-minus-low response interval for baseline output and each station's paired intervention gain.
- Ranking switches, exact catalog/report hashes, and a heuristic priority order for which input parameter may be worth measuring more accurately.

**The priority order is a screening heuristic, not formal value-of-information.** It is not weighted by real-world likelihood, measurement cost, or the consequences of a wrong decision. Formal value-of-information would require independently justified priors/ranges, measurement costs, and decision-loss assumptions.

## Run

The CI workflow generates the catalogs from the public versioned base fixture and invokes the Rust tool:

```sh
cargo fmt --manifest-path tools/ev-passenger-cost-frontier/Cargo.toml -- --check
cargo test --manifest-path tools/ev-passenger-cost-frontier/Cargo.toml
```

It then emits 13 deterministic simulation reports, the sensitivity report and a SHA-256 manifest. Re-running the same inputs must produce byte-identical sensitivity JSON.

## How to interpret the benchmark

1. **Seed intervals answer only the question "how much stochastic variation does this model generate at these fixed parameters?"** They do not include uncertainty about those parameters.
2. **Low/high endpoint changes are sensitivity contrasts, not probabilities.** Do not average their values into a made-up probability that a real factory will behave that way.
3. **A winner switch is a signal that the decision depends on that input factor.** A sensible engineering response is to prioritize a permitted measurement or controlled experiment for that factor—not to choose whichever endpoint favors a preferred investment.
4. **No unique robust winner is an acceptable result.** It means this declared envelope does not justify one intervention across every tested setting. It is preferable to a falsely certain capital recommendation.
5. **One-at-a-time screens miss interactions.** Once this validates, a second phase should use an explicitly designed fractional-factorial or space-filling experiment (with interaction terms and held-out settings) for factors that dominate ranking sensitivity.
6. **The screen does not quantify structural uncertainty.** The simulator still approximates failures at the completion boundary and does not fully model common-cause disruptions, planned maintenance, labor calendars, correlated supply-chain outages, due-date policies or energy/inventory economics.

## Proposed performance and credibility gates

| Gate | Initial acceptance |
|---|---|
| Trace/catalog linkage | 100% of generated reports verify the source catalog SHA-256 |
| Parameter isolation | 100% of declared OAT profiles differ only at the documented JSON-pointer paths |
| Paired design | Exact same ordered seeds for all 13 settings; complete per-seed baseline/intervention pairs |
| Negative controls | Wrong catalog digest, co-varied parameter, mismatched seed, duplicate/missing outcome are rejected |
| Robustness reporting | Publish per-factor endpoints and rankings; never treat endpoint settings as IID draws |
| Decision gate | Robust winner only if the same station clears the margin against all alternatives at all declared endpoints and nominal |
| Field validation | Requires rights-cleared measured trace data and independent temporal holdouts; not available here |
| Production release | Hard false in software; separate safety, quality, local compliance and site approvals remain required |

These are project acceptance targets, not universal industry standards and not yet achieved field metrics.

## Claim ceiling

Every output is model-conditional and synthetic. No plant data has been collected, no manufacturing operator/vendor has been contacted, and no quote has been requested. The tool does not establish commercial superiority, predict actual plant output, authorize purchases, or grant site/product/process release.
