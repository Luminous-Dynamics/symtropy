## ErgoStream conformal calibration transport protocol v0.2

**Scope:** transport uncertainty calibration between source and target domains using an independently qualified structural/behavioural map.  
**Current executable baseline:** \`calibration.rs\` split-conformal binary transfer, owner-assigned stratum, independent-unit outcomes.  
**Connected owners:** Symtropy #7173 / #7193 / #7041 · Symthaea #11171 / #5476 / #5478.

### Research thesis

A valid structural correspondence may permit more than predictor transfer. It may identify which *uncertainty calibration information* can be carried from a labeled source domain into a paired but unlabeled target domain.

This is a new claim, not an automatic consequence of graph isomorphism:

> Given a frozen predictor and score function, a qualified source-to-target structural map, paired source/target observations, and sufficient support overlap, can source calibration produce target-domain prediction sets with better coverage–efficiency trade-offs than ordinary source-only split conformal?

The claim must be falsifiable. A map can preserve observable structure yet fail to preserve the conditional outcome law or nonconformity-score distribution. Mechanism-change controls must therefore invalidate transported calibration even when observed structures are isomorphic.

### Why ordinary split conformal is not enough

The current v0.1 implementation provides a finite-sample marginal coverage baseline only under exchangeability between calibration and future claims within one admitted stratum. It:
- fits on a frozen calibration cohort;
- requires one evaluator-defined independent unit per score;
- rejects duplicate independent-unit IDs;
- returns a full two-label set and no decision when the owner-assigned target stratum differs from the calibration stratum;
- does not guarantee conditional coverage or robustness to arbitrary covariate/concept/mechanism shift.

The domain support registry and independent-block threshold remain owned by #7041. The split-conformal module consumes its owner-approved stratum; it does not infer or authenticate the domain fingerprint.

### State-of-the-art methods to reproduce

These methods address different assumptions and must remain separate baselines:

1. **Ordinary split conformal.** Exchangeable calibration and target claims within a declared cohort.
2. **Domain-Shift-Aware CP (DS-CP, ICML 2026).** Reweights calibration examples by test-prompt proximity for language-model domain shift. Reproduce on its native LLM setup before adapting its distance/reweighting method to a relational/physical domain. [Paper](https://proceedings.mlr.press/v306/lin26t.html).
3. **Transported Conformal Calibration (TCC, ICML 2026).** Uses unlabeled paired source/target observations to transport labeled source calibration, then corrects residual mismatch with label-free diagnostics (TCC-KS) or weighting (weighted-TCC). This is especially relevant to changed sensors/modalities/graph formats with paired source/target observations. [Paper](https://proceedings.mlr.press/v306/doula26a.html).
4. **KMM-CP (UAI 2026).** Uses kernel mean matching under covariate shift and restricts correction to regions with adequate support overlap. The authors report more than 50% coverage-gap reduction on their molecular-property shift benchmarks, but emphasize the instability of importance weighting when overlap is poor. [Paper](https://proceedings.mlr.press/v337/laghuvarapu26a.html).
5. **Abstain-on-support-failure.** If the target lies outside a predeclared support envelope or the map fails qualification, return the full prediction set / no transfer decision.
6. **Oracle target calibration.** Uses labeled target calibration only as an upper-bound control; never available to the learner in the transport track.

### Transport artifact

A proposed transport receipt must bind:
- exact source and target episode/cohort IDs;
- source and target evidence-prefix digests;
- source/target domain-fingerprint digests and owner-approved stratum IDs;
- exact transfer-claim type and canonical claim-scope digest;
- structural/behavioural correspondence artifact and its independent witness receipt;
- predictor, preprocessing and nonconformity-score-function digests;
- calibration cohort digest and independent-unit manifest;
- paired-observation cohort digest and pairing rule;
- transport method/version, fitted weights or diagnostic output digest;
- support-overlap statistic, effective sample size and maximum weight;
- target calibration disposition: admitted, limited, or rejected;
- abstention and revocation reason where applicable.

Every mutable or learned artifact in the transport path must be frozen before target evaluation outcomes are released. A digest proves identity only after the owner has acquired and verified the bytes; it does not authenticate the truth of the observations.

### Receipt identity and predictor immutability

A transported calibrator is bound to the full owner-issued calibration identity, not just a stratum ID. At prediction time the owner verifies and supplies the same receipt ID, claim type/scope, predictor, preprocessing, score function, calibration cohort, sample-ID set, split manifest, evidence prefix and independent-unit kind. Any mismatch yields the full two-label prediction set and no decision. Signature/custody/revocation verification occurs before this Rust identity reaches the evaluator; digest-shape checking is not signature verification.

### Mathematical assumptions — declare per method

**Ordinary split conformal** requires exchangeability of independent calibration units and future prediction units within the admitted stratum, plus a fixed predictor/preprocessor/nonconformity score chosen independently of calibration outcomes.

**Weighted conformal / covariate shift** requires a valid covariate-shift statement (same conditional outcome law given the transported features), target/source support overlap, and known or suitably estimated density ratios. If weights are estimated, clipped, truncated or regularized, record that choice and do not claim the exact known-weight finite-sample guarantee without a matching theorem.

**Paired transport** requires a declared source-target pairing mechanism and a transport relation that preserves the relevant score/label semantics. Paired unlabeled data can diagnose and correct selected forms of mismatch; it does not reveal arbitrary target concept shift.

**Structural correspondence** establishes only the declared finite relation. It does not prove equality of source/target \(P(Y\mid X)\), invariance of nonconformity-score distributions, common support, sensor measurement equivalence, independent calibration blocks, or valid density ratios. The transport protocol must test these assumptions independently.

### Paired-world Transfer Atlas

Create a factorial campaign with source/target pairs and evaluator-hidden labels/outcomes. Keep the model, score function and source calibration cohort fixed inside each comparison.

| World family | Change | Expected transport disposition |
|---|---|---|
| T0 | Exact same domain / exchangeable resampling | Admit ordinary split conformal |
| T1 | Identifier relabeling / serialization only | Admit only if pairing/map and score semantics validate |
| T2 | Sensor or modality representation change with paired observations | Compare TCC transport and mismatch correction |
| T3 | Covariate shift with known density ratios and adequate support | Compare weighted conformal and KMM-style weighting |
| T4 | Covariate shift with weak/no support overlap | Reject or restrict to overlap region; quantify lost coverage |
| T5 | Isomorphic visible structure but changed hidden mechanism / \(P(Y\mid X)\) | Reject transported calibration; false admission is a critical error |
| T6 | Relation/action semantics changed | Structural map rejected or downgraded; no inherited calibration |
| T7 | Predictor, preprocessing or score function changed after calibration | Receipt invalid; recalibration required |
| T8 | Temporal or campaign dependence within nominal stratum | Require independent blocks or separately qualified block-conformal method |
| T9 | Missing/conflicting/stale domain fingerprint | UNKNOWN/UNAVAILABLE; no finite set or transfer assertion |

Include a degree-matched graph negative and the existing isomorphic-observation/different-mechanism Atlas control. The same visible graph must not cause TCC/KMM calibration to be admitted without independent evidence that the relevant score/outcome semantics transport.

### Adaptive observations and delayed calibration feedback

The current `fit_split_conformal_binary_transfer` is an **offline split** baseline. It does not cover calibration under an adaptive measurement policy, optional stopping, non-mature labels, or sequential updates.

Compose rather than duplicate:
- Symtropy [#6887](https://github.com/Luminous-Dynamics/symtropy/issues/6887) owns anytime-valid surviving-world / decision sets under adaptive observations, e-process assumptions and optional-stopping controls.
- Symthaea [#13280](https://github.com/Luminous-Dynamics/symthaea/issues/13280) owns online conformal under delayed metrology feedback and process drift, including label maturation, revision/supersession, clock alignment, regime changes and update policy.
- Symtropy [#6943](https://github.com/Luminous-Dynamics/symtropy/issues/6943) owns trajectory dependence and defensible independence-group contracts.
- Symtropy [#5285](https://github.com/Luminous-Dynamics/symtropy/issues/5285) owns conservative block-maximum conformal and transfer-failure controls.

Rules for ErgoStream:
1. Do not treat adaptively selected probes as exchangeable merely because sample IDs differ.
2. Do not update calibration from a target result until its outcome is owner-validated and mature.
3. Do not double-count a revised/superseded measurement.
4. Preserve the policy/action receipt and exact public history used to choose each probe.
5. When no defensible block-independence or sequential validity contract applies, mark the coverage guarantee unavailable and abstain rather than inventing an effective sample size.
6. Never label the static v0 split-conformal result as anytime-valid or online.

A future adaptive calibration track should compare fixed-window split conformal, campaign/block maximum, EnbPI/AcMCP where their assumptions fit, drift-triggered online calibration, and #6887's time-uniform e-process decision sets. Report sustained local undercoverage and longest miss runs, not only signed long-run average coverage.

## Baseline tournament

Run each method on identical source/target pairs, matched data budgets and the same frozen predictor:
- source-only split conformal;
- owner-stratified split conformal;
- DS-CP-style proximity weighting;
- TCC-KS-style paired mismatch correction;
- weighted-TCC;
- KMM-CP / support-restricted weighting;
- structural-map-gated transport;
- random/naive pairing control;
- oracle target-calibrated predictor as a non-deployable upper bound;
- always-abstain and fixed-threshold deferral controls.

Do not tune weights, support thresholds, map thresholds, calibration bins or deferral operating points on confirmatory target labels.

### Adaptive-coverage and delayed-feedback comparators

The transfer tournament must not collapse static, adaptive and time-series calibration into one method family:

- **Adaptive Coverage Policies (AISTATS 2026):** example-dependent coverage policy trained through a leave-one-out / e-value procedure. Benchmark whether difficulty-aware coverage allocation improves prediction-set efficiency at fixed validity requirements. This does not alone handle arbitrary concept shift. https://proceedings.mlr.press/v300/gauthier26a.html
- **Delayed-feedback ACI (2026):** label delays are explicit, and theoretical/empirical behavior depends on the delay-to-residual-memory relationship. Test fixed, variable, censored and out-of-order labels; update only after an owner-validated outcome matures. https://arxiv.org/abs/2609.07251
- **Dependence-aware realized-coverage inference (2026):** use functional-dependence bounds and block-based standard errors to quantify uncertainty in *observed* coverage; keep that interval distinct from the conformal prediction-set coverage statement. Include long-memory sensitivity. https://arxiv.org/abs/2609.33868
- **Drift-detection online conformal (2026):** evaluate abrupt/smooth shift update policies and training-conditional cumulative regret under the paper's independent-stream conditions; do not claim transfer to dependent campaigns without further assumptions. https://arxiv.org/abs/2602.16537
- **Anytime-valid adaptive observation:** reuse Symtropy #6887 for time-uniform decision sets/e-processes when the learner actively chooses the next probe. A static split-conformal quantile is not valid merely because the adaptively selected observations have unique IDs.

Report separate tracks for static exchangeable calibration, independent-block calibration, adaptive sampling/optional stopping, delayed label feedback, and online distribution drift. A combined system must use the appropriate owner-approved guarantee for each track; it must not relabel a static split-conformal result as online or anytime-valid.

### Metrics

Publish a vector, not one blended leaderboard score:
- marginal target coverage and nominal coverage gap;
- worst admitted-stratum coverage gap;
- prediction-set size and singleton decision coverage;
- false-transfer / false-mechanism assertion rate;
- support-gate false admission and false rejection;
- target-label-free mismatch diagnostic AUROC / precision-recall where an evaluator label exists;
- effective sample size and maximum density ratio;
- fraction of target data outside support;
- risk–coverage curve and tie-aware AURC, always alongside decision coverage;
- Brier, log loss and ECE at preregistered bin counts;
- calibration sample efficiency in independent units;
- paired-observation cost, target-label cost, wall time, compute and memory;
- recovery after a controlled shift and recalibration burden.

Use cluster/block bootstrap at the actual independent unit for uncertainty intervals. Do not use frame/window counts as independent trials. Report every shift axis separately and keep pooled results secondary.

### Prospective internal gates (not achieved results)

1. 100% correct admission/rejection on finite known-answer domain-fingerprint and stratum-boundary controls.
2. Zero high-confidence false mechanism-transfer assertions on the isomorphic-observation/different-mechanism control.
3. On representation-only transfer, achieve nominal target marginal coverage within preregistered confidence bounds while improving set size or singleton decision coverage over source-only split conformal.
4. On covariate shift with adequate overlap, match or improve the best reproduced weighted/TCC comparator under equal source data and paired-observation budgets.
5. On weak/no overlap and concept shift, fail closed or explicitly localize the admitted subset; no universal coverage claim.
6. Report independent-unit count, effective sample size, maximum weight and all dropped/unsupported cases.
7. No result may hide worst-stratum undercoverage behind pooled coverage.

The exact risk ceilings, confidence bounds, sample size and multiplicity plan must be preregistered before the sealed cohort.

### Architecture composition

- **#7041** owns source-backed domain fingerprints, stratum registry and minimum independent-block support.
- **#7193** owns paired worlds, false-analogy controls and Transfer Atlas scoring.
- **#7173** owns finite correspondence witnesses, evidence-bound claims, calibration metrics and the within-stratum split-conformal reference.
- **#5990** owns hidden episode custody and outcome release.
- **#11171** owns the production research-policy adapter and frozen forecast path.
- **#5476/#5478/#8721** own prospective metareasoning/value-of-computation and action-selection ablations.

Do not add a second domain registry, episode runner, density-ratio framework, model DSL, scheduler or evidence store.

### Claim ceiling

A successful finite transport benchmark establishes only the tested map, paired-data regime, support assumptions, predictor and claim family. It does not establish arbitrary cross-domain transfer, causal mechanism identity, real-world sensor equivalence, temporal/online validity, physical safety or universal calibration.
