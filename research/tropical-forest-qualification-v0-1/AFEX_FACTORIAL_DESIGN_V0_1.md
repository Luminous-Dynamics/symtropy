# AFEX factorial intervention design gate v0.1

**Issue:** [#5408 — AFEX-FOREST-INTERVENTION-001](https://github.com/Luminous-Dynamics/symtropy/issues/5408)  
**Parent:** [#5173 — tropical forest ecological qualification](https://github.com/Luminous-Dynamics/symtropy/issues/5173)  
**Composes:** #5340 process trajectories; #5349 structural flux identifiability; #5360 practical identifiability; #5198 model-set adequacy; #5223 decision value  
**Date:** 15 November 2026  
**Status:** synthetic allocation gate; external archive bytes not ingested

## 1. Why AFEX is the next empirical pilot

The official EIDC candidate is *Tree census and diameter increment in fertilised plots in the Central Amazon, 2017–2020*, DOI [10.5285/c2587e20-ba4a-4444-8ce9-ccdec15b0aa3](https://catalogue.ceh.ac.uk/documents/c2587e20-ba4a-4444-8ce9-ccdec15b0aa3). Its record specifies annual inventories in 2017–2019 (May in 2018/2019; June–November in 2017), trees with DBH >10 cm measured at 1.3 m unless buttresses/deformities require a different height, recruitment as new individuals reaching 10 cm DBH, and recorded mortality. Dendrometer bands were installed on 1,299 trees; circumference-window distance (mm) was read with a digital caliper from April 2018 through January 2020 on an irregular schedule. The instrument is humidity-sensitive and readings were not collected during rain. The EIDC record states Open Government Licence and requires dataset citation/attribution.

The companion AFEX experiment describes a randomized complete-block, fully factorial manipulation of nitrogen (N), phosphorus (P), and cations (C), with eight treatment combinations and four blocks/replicates. This yields a candidate design of 32 plot-level assignment units. This expected allocation must be confirmed against the released files and protocol before any treatment estimate is admitted.

AFEX is valuable because it adds controlled perturbation to the forest process-trajectory question. Instead of only asking whether a model matches observed growth and mortality, we can test whether it predicts the changes associated with a defined nutrient intervention, including interactions and response timing.

## 2. The critical statistical distinction

**Treatment assignment unit:** plot.  
**Observation unit:** tree.  
**Repeated measurement unit:** tree × measurement date.

The reported 1,299 dendrometer trees are nested measurements; they are not 1,299 independently randomized treatment replicates. The plot count and block-by-treatment assignment determine treatment replication. Treating every tree or every repeated circumference reading as independent treatment evidence would be pseudoreplication and could severely understate uncertainty.

The first gate therefore checks:
- four block identities;
- all eight combinations of N/P/C in every block;
- one unique plot per block × treatment cell;
- 32 unique plot identities;
- explicit plot-level treatment assignment and nested tree-measurement semantics;
- a false value for both tree-as-treatment-replicate and repeated-reading-as-treatment-replicate.

For the balanced complete factorial, the treatment cell-mean space has rank 8. Adding an intercept and three independent block contrasts gives a fixed-effect design rank of 11. With 32 plot units, the corresponding residual degrees of freedom are 21. These are design-matrix facts for the declared balanced synthetic fixture, not an estimate of ecological effect or power.

## 3. Treatment contrasts

Encode the three factors as binary indicators:
- N: nitrogen addition;
- P: phosphorus addition;
- C: cation addition.

The full factorial contrast set is:
- main effects: N, P, C;
- two-way interactions: NP, NC, PC;
- three-way interaction: NPC.

The analysis must retain the factorial interactions. A nutrient response may be conditional on another nutrient; fitting only three additive main effects can hide the experimental mechanism.

The primary estimand should be the intention-to-treat effect of plot assignment. Actual application, adherence, timing and dose are additional evidence fields and should not silently replace randomized assignment.

## 4. Process-resolved outcomes and measurement semantics

Potential outcomes include:
- repeated stem circumference increment from dendrometer bands;
- annual DBH / stem-state measurements under the inventory protocol;
- mortality and recruitment, including the source's DBH threshold and recruit definition;
- litterfall and nutrient cycling where compatible companion observations are admitted;
- canopy/LAI observations where their dates and spatial support align.

Do not equate raw circumference-window distance (mm) with circumference increment, DBH increment or biomass increment without reconstructing the instrument reading convention, band geometry, calibration and any allometric transformation. The EIDC data describe window distance, not a ready-made annual biomass-growth series. Preserve exact observation timestamps and interval durations. Rain-related uncollected readings, instrument resets, lost bands, tree death and other missingness must be distinguished; a missing reading is never numeric zero.

Growth, recruitment and mortality must remain separate response channels. A treatment may affect one before another, and total biomass alone can conceal compensating responses. Compose their downstream qualification with #5340, #5349 and #5360 instead of implementing another flux engine.

## 5. Required empirical admission sequence

1. Retrieve the official EIDC archive in a network-enabled environment.
2. Verify the exact archive bytes against provider checksums where supplied; compute SHA-256 and retain a source receipt.
3. Inspect all files, their metadata, licence/attribution and any access conditions.
4. Reconcile block, plot and treatment IDs with the experiment protocol. Confirm whether the real file set contains exactly one plot for each block × treatment cell.
5. Verify persistent tree IDs and tree-to-plot links across every dendrometer date and census.
6. Verify census dates, DBH threshold, recruitment entry rule, mortality state transitions, units and any plot exclusions.
7. Verify actual fertilization doses, dates, treatment compliance and whether the reported four blocks are the appropriate independent assignment units.
8. Freeze plot-level splits and the pre-analysis plan before inspecting treatment effects. Never split trees or repeated observations from the same plot across train/test.
9. Keep a raw-to-derived lineage for each growth, mortality, recruitment and carbon variable.

No archive bytes have been downloaded or ingested in the current runtime. This document and its fixture are not a substitute for the file-level gates.

## 6. Matched causal and predictive tournament

At minimum compare:
1. block-adjusted factorial contrasts with all main and interaction effects;
2. a repeated-measure model with tree nested in plot and plot nested in block, only where identities and sample sizes justify it;
3. a plot-level randomization-inference baseline respecting the actual randomization scheme;
4. a mechanistic vegetation-demography baseline when model inputs/outputs can be reconciled;
5. Symtropy process-resolved forecasts and model-set adequacy diagnostics.

Freeze identical plot/time support, treatment assignment, forecast issue time, covariates, outcome definitions and compute budgets. Use randomization/design-aware uncertainty at the plot level. Trees and time points can improve within-plot measurement precision but do not create more independent treatment allocations.

Report treatment-effect bias and intervals separately for each factor and interaction; growth, recruitment and mortality errors; stock/pool budget residuals; predictive CRPS/WIS/Brier where distributions exist; worst-block and worst-treatment performance; calibration and abstention coverage; runtime, memory and data/field cost. Do not bury a failed process or invalid design in a composite score.

## 7. What would count as progress beyond SOTA?

The first success is not a leaderboard claim. It is a reproducible demonstration that Symtropy:
- correctly recovers the known factorial assignment and uncertainty unit;
- exposes when tree-level or repeated-measure pseudoreplication inflates apparent evidence;
- distinguishes additive from interaction responses on held-out plots;
- detects a model that matches total biomass but misses treatment-specific process responses;
- selects measurements that improve process discrimination or a stated decision;
- remains calibrated under leave-plot/block/time splits.

Only after the data and design have passed admission should we set numerical empirical improvement thresholds. They should be preregistered using measurement error, sample-size/power analysis and paired baselines—not chosen after seeing the outcomes.

## 8. Local execution

From the repository root:

```bash
cargo run --manifest-path research/tropical-forest-qualification-v0-1/validator/Cargo.toml -- \
  research/tropical-forest-qualification-v0-1/cases/afex-factorial-design-synthetic-v0.1.json

cargo test --manifest-path research/tropical-forest-qualification-v0-1/validator/Cargo.toml

cargo fmt --manifest-path research/tropical-forest-qualification-v0-1/validator/Cargo.toml -- --check

cargo clippy --manifest-path research/tropical-forest-qualification-v0-1/validator/Cargo.toml --all-targets -- -D warnings
```

These Rust commands are the required next local gates; they have not been executed in the current runtime because `cargo`, `rustc` and `rustfmt` are unavailable here. The independent JavaScript arithmetic/control check is a separate reference, not a substitute for compilation or Rust tests.

## 9. Claim ceiling

A pass of `cases/afex-factorial-design-synthetic-v0.1.json` establishes only the arithmetic completeness of a synthetic 2×2×2 randomized complete-block allocation and the anti-pseudoreplication contract. It does not establish that actual AFEX records match this allocation, any nutrient treatment effect, causal mechanism, growth or mortality calibration, forecast superiority, or transfer to other tropical forests.

Empirical state: `EXTERNAL_DATA_NOT_INGESTED`; `EMPIRICAL_TREATMENT_EFFECTS_NOT_ESTABLISHED`.
