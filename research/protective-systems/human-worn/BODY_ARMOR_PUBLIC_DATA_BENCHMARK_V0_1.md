# Body-Armour Public-Data Validation Benchmark v0.1

**Status:** benchmark specification; real-data ingestion and model runs are pending  
**Date:** 2026-11-15  
**Parent:** BODY-ARMOR-001 #5377 / PROTECTIVE-SYSTEMS-001 #5246  
**Data registry:** `research/protective-systems/fixtures/body-armor-public-datasets-v0.1.json`

## 1. Why this benchmark exists

The first body-armour work increment established a synthetic B0 record verifier: it checks whether a proposed evidence record has the right scope, revision, standard references and authority. That is necessary, but it does not test whether a statistical model predicts measured experimental outcomes.

B1 adds a deliberately narrow, reproducible statistical track using the public NIST V50 archive. Its purpose is to check data ingestion, reproduce published statistical baselines, and develop uncertainty/calibration practices on historical experimental data.

**B1 is not a ballistic qualification method.** It does not certify a product, establish NIJ conformity, predict human injury, validate a finite-element solver, or authorise a physical test.

## 2. Dataset selection and rights

### Primary dataset: NIST M32149

- Persistent identifier: https://doi.org/10.18434/M32149
- Landing page: https://catalog.data.gov/dataset/ballistic-test-results-for-several-different-soft-body-armor-systems
- Archive: `V50.zip`
- Published SHA-256: `ba8ee6071849d5883af434d1bda46727f695ef64d446fdbf8f26a0a653f4d66b`
- Declared contents: six CSV files, one per armour system, with velocity in m/s and a binary result (1 = perforated; 0 = stopped), plus a README.
- Dataset last updated: 2011-02-14.
- Associated method reference: NISTIR 7760, https://doi.org/10.6028/NIST.IR.7760.

The Data.gov record points to the NIST data licence. Preserve NIST attribution and the as-is/no-warranty notice. Any transformed or modified data must retain a notice stating the date and nature of the modification. The licence page discusses possible foreign copyright as well as the rights granted under its terms; do not replace the source licence with an assumed CC licence.

**Age and scope warning:** the NIST study is situated in the NIJ 0101.06 era. It is appropriate for historical statistical-method reproduction, not for claims of current NIJ 0101.07 conformity or current product performance.

### Excluded discovery candidate: Zenodo 23313293

The October 28, 2026 Zenodo record describes a 95-shot, ten-architecture carbon/epoxy laminate experiment with 92 high-speed videos, material/specimen metadata and image measurements. It is not body armour and not a torso-worn product. The record lists its licence as “to be confirmed”; Zenodo's terms also restrict access to non-military purposes and make reuse subject to the item-specific licence. It is therefore recorded for discovery only and excluded from ingestion, training, evaluation and redistribution until the licence and permitted-use scope are explicitly resolved. No dataset files have been downloaded.

## 3. Standards context and boundary

As of this specification date:

- NIJ 0101.07 is the current NIJ ballistic body-armour test/performance standard and its official page lists three addenda. The addenda form part of the standard.
- NIJ 0123.00 specifies protection levels and associated test threats; it does not itself provide product performance requirements or a test method. Its official page lists two addenda.
- NIJ's Compliant Products List is the reference for product models that have completed the relevant NIJ conformity process. NIJ indicates it will maintain the 0101.06 list through at least the end of 2029 during transition.

B1 must not be mistaken for any of these conformity mechanisms. A NIST V50 curve, a fitted model, or an in-silico prediction cannot be labelled “NIJ compliant”.

## 4. Frozen benchmark tasks

### B1-A — source integrity and reproduction

1. Retrieve the archive from the canonical NIST PDR record.
2. Verify the complete archive against the repository-published SHA-256 before extraction.
3. Preserve the original archive unchanged. Compute and record a SHA-256 for every extracted file.
4. Parse the six CSV files without silently coercing, dropping or reordering rows. Record row counts, min/max velocity, unique outcome labels, missing values, duplicate rows and parser version.
5. Reproduce the NISTIR 7760 logistic, probit and complementary log-log analyses using the same per-system observations and documented assumptions.
6. Compare estimates, uncertainty intervals and model outputs with the report. Record every discrepancy; do not tune an implementation merely to hide a difference.

Acceptance is exact source identity plus a reproducible analysis trail. The NIST paper is the reference for reproducing its analysis, not proof that any model is suitable for present-day product decisions.

### B1-B — honest predictive evaluation

Separate model fit from prediction. A model fitted to all observations may be used to reproduce the paper, but its in-sample fit is not an independent predictive score.

Before fitting any new candidate:

1. Inspect and document the data structure and any available shot ordering.
2. Freeze a deterministic, velocity-blocked validation design within each armour-system file. Hold out contiguous velocity regions rather than relying solely on random row splits.
3. Ensure the split generator does not use outcomes to select an easier holdout. Publish the split manifest and its digest.
4. Where sample size or outcome balance makes a blocked fold uninformative, mark the fold **not estimable**. Do not relax the split after seeing a candidate's results.
5. Keep the six armour systems separate in reporting.

The source CSV schema exposes velocity and binary outcome, but no material/process covariates sufficient to establish transfer to a new architecture. Therefore this dataset cannot support a credible “generalises to unseen armour architectures” result. Cross-system transfer is out of scope until a separately rights-cleared source supplies relevant covariates and independent test data.

## 5. Baselines and candidate models

The mandatory matched baselines are:

1. Logistic response model.
2. Probit response model.
3. Complementary log-log response model.

These are the three published NISTIR 7760 model families. Implement and verify them before adding new methods.

Candidate extensions may include monotone nonparametric calibration and Bayesian uncertainty estimates, but only when they use the same allowed inputs and the same frozen folds. A Symthaea-derived candidate must compete on the identical data, target, compute budget and split. A more complex HDC/CfC or other learned representation is not a meaningful advantage on this two-column, small-sample dataset merely because it is architecturally novel.

## 6. Scorecard

| Dimension | Required reporting | Why it matters |
|---|---|---|
| Probabilistic prediction | Log loss and Brier score, per system and aggregate | Scores the full probability, not only a hard class |
| Calibration | Calibration intercept/slope and reliability plot; ECE only as a secondary diagnostic | Detects confident but wrong probabilities |
| Ballistic-limit summary | V50 estimate, uncertainty interval and method | Makes the familiar summary explicit without treating it as the whole distribution |
| Uncertainty | Predictive interval coverage and interval width at declared nominal levels | Penalises under-reporting of uncertainty |
| Robustness | Results by system and held-out velocity region; sensitivity to fit assumptions | Reveals dependence on a convenient subset |
| Provenance | Source/archive/file hashes, code revision, environment, split digest and run configuration | Makes every score reproducible |
| Abstention | Explicit “not estimable” outcomes when coverage or support is inadequate | Prevents unsupported claims from being filled in by extrapolation |

Accuracy and AUC may be included as secondary descriptive metrics. Neither is sufficient to establish calibrated perforation probabilities or safe protection.

## 7. Proposed research targets beyond the baselines

These are **prospective targets**, not achieved results or acceptance claims for the current six-system dataset:

- Reproduce all three published NISTIR 7760 baselines and explain any numeric discrepancy.
- On a genuinely independent, pre-registered holdout, seek at least a 10% relative improvement in log loss over the strongest matched baseline, with a paired uncertainty interval that excludes no improvement. Require replication on a second independent dataset before describing the result as general.
- Keep nominal 90% predictive-interval coverage close to 90% on independent data, reporting interval width and sample uncertainty alongside coverage.
- Demonstrate calibration under a held-out velocity region, not merely calibration on the same observations used to fit the model.
- Publish negative controls: shuffled labels, deliberately shifted units, corrupted source digest, missing outcome labels and split leakage. The pipeline must fail closed rather than quietly emit a performance score.

No numeric “SOTA” claim should be made from this single historical dataset. The 2026 systematic review of 3D-woven ballistic modelling reports that broad, independently validated prediction across architectures and impact regimes remains an open problem; reported accuracy can be calibration-dependent. Our research opportunity is to make independent validation and uncertainty visible, not to claim that an unvalidated digital twin has solved impact mechanics.

## 8. How this compares with industry

The industry comparison must distinguish three different activities:

1. **Product conformity:** standards-based physical tests, approved laboratories, manufacturing controls, follow-up inspections and a compliant-products listing.
2. **Research and design:** material/process experiments, modelling and optimisation used to form candidates and explain mechanisms.
3. **Evidence infrastructure:** traceable configuration, data provenance, uncertainty, test-to-model discrepancy, change impact and lifecycle records.

NIJ's current conformity programme occupies the first category. B1 occupies a small part of the third and provides a historical statistical baseline for the second. It is not an alternative certification route.

The potential advantage is not “Symthaea beats NIJ” or “simulation replaces the lab”. It is a tighter evidence loop: requirements → candidate hypothesis → model and uncertainty → independently measured result → discrepancy analysis → controlled revision → configuration-bound lifecycle evidence. A candidate design must still pass the applicable external physical qualification.

## 9. System responsibilities

- **Symthaea:** advisory material/process hypotheses, feature reasoning and uncertainty estimates. It does not authorise physical tests or certify its own outputs.
- **Symtropy:** deterministic dataset/benchmark execution, split control, geometry and system-level modelling only within validated scope. Its current rigid-body core is not a ballistic composite-impact solver.
- **Mycelix:** provenance and configuration-linked evidence records for later sample, lot, test, inspection and repair lineage.
- **Independent laboratories and standards bodies:** retain authority for physical test methods, conformity assessment and certification.

## 10. Current execution state

- Dataset identity, metadata, source schema, NISTIR 7760 reference and repository-published archive digest have been recorded.
- The archive has **not yet been downloaded or checksum-verified in the working environment**; baseline reproduction has therefore **not** been run.
- The 2026 Zenodo CFRP record has not been downloaded or ingested because its item-specific licence is unconfirmed and its repository terms require permitted-use review.
- No physical tests, wearer studies, prototypes, procurement, injury claims, product certification or SOTA superiority are claimed.

The next executable gate is a successful archive fetch followed by digest verification. If the hash does not match, stop; do not parse or train on the artifact.
