# Civilizational Research Baseline Audit

**Date:** 2026-11-02  
**Purpose:** Establish the actual repository baseline for Civilizational Systems Science and CivBench.  
**Status:** Source-review notes; no build, test, CI, or scientific validation pass is claimed.

## Executive finding

Do not create a parallel civilization simulator, scenario schema, evidence ledger, or planetary atlas. The organization already has substantial work in all four projects and a parent multiworld simulation repository. The next gains come from composing that work, correcting validation weaknesses, and measuring whether mechanisms actually change independently meaningful outcomes.

The benchmark specification in CivBench PR #3318 is intended to be the scientific task/evaluation layer. The existing Humanity Atlas contracts remain the shared scenario/run artifact layer.

## 1. Existing work to compose

### Symtropy

The current main branch already contains game and domain foundations for settlement, resources, world state, physical mechanisms, and a feature-gated Mycelix path. The city-scale Mycelix Village app's CityScalePlugin is explicitly an intentional no-op; the related ecology stack is not wired to that plugin or GamePhase on the inspected main head.

Relevant open draft work inspected on 2026-11-02:

- [PR #369 — civilizational authority contract](https://github.com/Luminous-Dynamics/symtropy/pull/369): separates world truth, observation, belief, claims, institutional records, authority, control, ownership, and legitimacy.
- [PR #378 — causal resource and shipment flows](https://github.com/Luminous-Dynamics/symtropy/pull/378): inventory, production, shipment, and arrival semantics; prevents free material creation and early destination credit.
- [PR #391 — deterministic Three Worlds integration lab](https://github.com/Luminous-Dynamics/symtropy/pull/391): composes institutions, succession claims, shipment, treaty disputes, identity, project evidence, and conflict into a headless scenario with a causal event-chain verifier.
- [PR #399 — astrobiology authority and uncertainty contract](https://github.com/Luminous-Dynamics/symtropy/pull/399): separates planet descriptors, planetary state, forcing, habitability, life, evolutionary and observational authorities; explicitly rejects a universal alien-civilization scalar.
- [PR #1571 — Humanity Atlas scenario contract](https://github.com/Luminous-Dynamics/symtropy/pull/1571): Draft 2020-12 scenario/run schemas, artifact digests, evidence-gated classifications, model-credibility procedure, failure-injection tests, and an authored real-artifact qualification harness.

These PRs were open drafts at audit time. Their descriptions explicitly distinguish authored tests from exact-head builds and real-artifact qualification. They must not be counted as merged or executable passes.

### Multiworld simulator in the parent repository

The simulator already exists at [Luminous-Dynamics/luminous-dynamics/mycelix-multiworld-sim](https://github.com/Luminous-Dynamics/luminous-dynamics/tree/main/mycelix-multiworld-sim). Its source module index includes demographics, population, biosphere, cascade, economy, education, governance, knowledge, disasters, interworld, stoichiometry, empirical parameters, historical validation, stochastic replay, and reporting. It is the natural existing engine to audit and compose, not replace.

Repository documents describe a monthly-tick, multi-world Rust model, deterministic xoshiro256++ replay, CSV/JSONL outputs, ablation and sensitivity binaries, and hundreds of tests. These are repository claims, not independently re-run results in this audit. The architecture document calls the engine approximately 48K LOC while FINDINGS.md calls it approximately 45K LOC; future claims must bind to exact commit/tree and a reproducible inventory.

The architecture document also says Sol-Atlas currently visualizes a static timeline derived from simulator constants and has no runtime integration. The current integration work is the open draft chain below; do not treat the older static milestone list as a calibrated forecast.

### Producer/run/consumer integration

- [Luminous Dynamics PR #3760](https://github.com/Luminous-Dynamics/luminous-dynamics/pull/3760) adds a model-qualified technology-tree producer from the active simulator, resolved configuration/parameters, graph validation and a source-specific digest.
- [Symtropy PR #1571](https://github.com/Luminous-Dynamics/symtropy/pull/1571) supplies the generic Humanity Atlas scenario/run envelope and qualification harness.
- [Sol-Atlas PR #79](https://github.com/Luminous-Dynamics/sol-atlas-leptos/pull/79) supplies a typed run-envelope and source-artifact consumer.

All were open drafts at audit time. The integration descriptions explicitly say Rust compilation/tests and real simulator-artifact round trips have not yet been executed in the editing environment. PR #1571's contract smoke tests were run with Python and synthetic fixtures, but that is not a real simulator qualification or empirical calibration.

### Mycelix

Mycelix already has DKG claim structures and Knowledge-cluster inference records, plus an RFC for a shared evidence plane:

- [Mycelix issue #5451](https://github.com/Luminous-Dynamics/mycelix/issues/5451): repo-portable evidence plane, content integrity, transformation lineage, benchmark fairness, independent verification, and claim ceilings.
- [Mycelix issue #3022](https://github.com/Luminous-Dynamics/mycelix/issues/3022): canonical statement identity and provenance-family semantics, including mirrored sources and temporal uncertainty.
- The CivOS Chronicle, Forecast, Loom and related hApps are primarily design/catalog documentation; the Mycelix README labels the overall ecosystem pre-alpha and distinguishes unit-tested modules from multi-agent integration and deployed UIs.

CSS-specific historical-source work should adapt these existing contracts to archaeological contexts, oral-history transmission, inscriptions, translations, contested chronology, source-family dependence, and community stewardship—not invent a second generic evidence plane.

### Symthaea

Relevant existing Symthaea research includes:
- [#5774 CIV-BOOT-000](https://github.com/Luminous-Dynamics/symthaea/issues/5774) and [#6287 CIV-BOOT-005](https://github.com/Luminous-Dynamics/symthaea/issues/6287): evidence-bounded capability closure and bootstrap-to-federation/interplanetary progression.
- [#6688 Human Fallibility & Ontological Humility](https://github.com/Luminous-Dynamics/symthaea/issues/6688): perceptual, inferential, cultural and collective epistemic failure modes.
- [#7046 institutional evolution](https://github.com/Luminous-Dynamics/symthaea/issues/7046), [#7188 socioeconomic interoperability](https://github.com/Luminous-Dynamics/symthaea/issues/7188), [#7193 stock-flow accounting](https://github.com/Luminous-Dynamics/symthaea/issues/7193), [#7201 metric gaming/Goodhart](https://github.com/Luminous-Dynamics/symthaea/issues/7201), and [#7205 simulation-to-authority firewall](https://github.com/Luminous-Dynamics/symthaea/issues/7205).
- [#8252 cross-tradition transfer](https://github.com/Luminous-Dynamics/symthaea/issues/8252): source-grounded narrative graphs and held-out traditions, including protection against similarity being mistaken for historical contact.

The new social-learning research track should benchmark specific transmission mechanisms and their contribution beyond these existing workstreams, not create a separate cognitive architecture.

## 2. Existing null results are valuable evidence

[PHASE2_FINDINGS.md](https://github.com/Luminous-Dynamics/luminous-dynamics/blob/main/mycelix-multiworld-sim/PHASE2_FINDINGS.md) records a controlled attempt to test whether a governance-defense package changed civilizational viability under simulated adversarial pressure.

The report states that the result was null: approximately -0.004 ± 0.008 arithmetic CVS delta, and -0.007 ± 0.007 with a geometric aggregation; the interventions changed adversary/governance-internal state, but did not propagate to the five inputs used by the civilization viability score. The write-up diagnoses two disconnected state spaces.

This is not proof that governance defenses are useless. It is evidence that the tested outcome metric was causally insensitive to the intervention pathways. The correct next step is to model and test explicit mechanisms from governance changes to services, access, production, distribution, welfare, ecological stocks, or other declared outcomes—and preserve null results when the causal path is absent.

The benchmark must therefore require:
- an explicit causal dependency graph;
- intervention-to-outcome path tests;
- outcome vectors disaggregated from any composite index;
- mediation/sensitivity analysis;
- placebo and negative controls;
- checks for metrics that do not respond to the mechanism they purport to measure.

A composite CVS may remain a descriptive dashboard component, but cannot be the sole outcome used to validate all governance mechanisms.

## 3. Concrete validation defects to address first

The current main-branch [mycelix-multiworld-sim/src/validation.rs](https://github.com/Luminous-Dynamics/luminous-dynamics/blob/main/mycelix-multiworld-sim/src/validation.rs) was inspected.

### Hard-coded observations

The module embeds 12 global data points for 1970–2024 (population, temperature anomaly, fertility and CO2 emissions). It names UN, World Bank, NASA GISS and Global Carbon Project in comments, but this code path does not bind a run to versioned source snapshots, extraction transformations, content digests or provenance receipts.

### Zero-baseline scoring defect

The APE calculation uses a fallback that assigns 0.0 error whenever the observed value is zero or near zero. Thus a non-zero prediction against a zero observed baseline—such as the first temperature anomaly point—can be recorded as zero error. This needs a regression test and a metric redesign before the report is used as evidence of historical fit.

### Invalid cross-metric aggregation

The report averages metric-specific MAPE values into one “overall MAPE”, despite comparing different variables and units. MAPE is unstable or undefined near zero and can conceal errors for sign-changing series. A single pooled MAPE is not a defensible global score.

Use a metric-specific score vector instead: absolute or squared errors for temperature in stated units, suitable relative errors for positive-scale variables, and proper probabilistic scores (for example Brier/log score or CRPS) when distributions are available. Compare with persistence, historical mean/climatology, linear trend and appropriate time-series baselines. Report calibration and interval coverage, not only point error.

This issue is tracked in [parent repository issue #3816](https://github.com/Luminous-Dynamics/luminous-dynamics/issues/3816).

### Empirical parameter lineage

The current empirical-parameter module includes useful candidate references and physical parameters but mostly expresses source attribution in comments. Source version, dataset snapshot, retrieval date, extraction method, uncertainty interval, applicable regime, and transformation lineage should be machine-readable.

One apparent inconsistency requires source-level reconciliation: the solar-proton-event extreme rate is calculated as 8 / 41 = 0.195 events/year while an adjacent comment says it matches a 0.7%/year estimate. The event definition and estimate may differ; this should be resolved from the original source, not normalized by assumption.

## 4. Revised work allocation

### Symthaea
Test cognition, social learning, cultural transmission, institutional memory and collective sensemaking as bounded mechanisms. Require simple non-cognitive baselines, held-out networks/contexts, ablations, uncertainty and a causal path to an external outcome. Consciousness measures are not rights, legitimacy or authority metrics.

### Symtropy
Reuse the parent multiworld simulator and current domain crates. Compose the open Three Worlds lab, resource-flow core and Humanity Atlas run contract. Add the independent water kernel as a test oracle for stock-flow correctness only; do not fork production state ownership or create a second civilization engine.

### Mycelix
Reuse issue #5451 and canonical identity/provenance work. Add historical-source adapters and domain-specific representations for uncertain chronology, archaeological context, oral-history lineage, source families, community stewardship and plural interpretations.

### Sol-Atlas
Reuse the existing Humanity/AI Bootstrap Atlas work (#17/#19) and Humanity Atlas consumer (#79). Add evidence-aware historical/contemporary geospatial data adapters, time intervals, location uncertainty, boundary uncertainty, coverage gaps and explicit missingness.

### Shared
CivBench supplies the task protocols, locked splits, baselines, metrics, uncertainty and failure analysis. Humanity Atlas supplies versioned scenario/run artifacts, identity, provenance and receipts. These are complementary layers.

## 5. Competitive positioning

Do not try to replace mature domain platforms:
- use historical sources and datasets such as Seshat and D-PLACE rather than rebuilding their curation from scratch;
- use contemporary data sources such as WVS, OWID, UN SDG indicators and Global Carbon Budget with explicit coverage and release versions;
- use established agent-based environments (Mesa, GAMA, NetLogo) as comparison baselines and interchange targets where helpful;
- use Earth-system and climate intercomparison infrastructures (ISIMIP, IAMC scenario databases) rather than inventing a climate model;
- use NASA's Exoplanet Archive and astronomy archives as observational infrastructure, not as evidence for extraterrestrial intelligence.

The distinctive contribution should be cross-domain composition with:
1. source and claim provenance that survives every transformation;
2. physical stock-flow closure coupled to social/institutional mechanisms;
3. temporal hindcasts and geographic/cultural transfer;
4. causal intervention tests and negative controls;
5. calibrated uncertainty and explicit applicability domains;
6. multi-dimensional outcomes and power/distribution analysis;
7. exact-run reproducibility and independent re-execution.

A claim of superiority over SOTA requires a named task, strongest suitable baselines, matched data and budgets, a frozen holdout, proper metrics, uncertainty, and independent reproduction. No universal leaderboard is appropriate.

## 6. Audit limitations

This audit is based on repository source and draft PR descriptions fetched on 2026-11-02. It does not execute Rust/Cargo tests, validate external dataset citations, run the current simulator, or independently reproduce historical or civilizational findings. The open drafts and their test descriptions are not equivalent to merged code or passing exact-head checks.

The current editing environment did not provide rustc/cargo. No Rust test or build pass is claimed for the water reference kernel or existing draft integration work.
