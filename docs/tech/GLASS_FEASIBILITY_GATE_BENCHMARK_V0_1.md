# GLASS-FEASIBILITY-GATE-001 — hierarchical feasibility and negative-region benchmark v0.1

**Status:** proposed evaluation contract. No model, dataset, or material outcome has been validated by this document.

**Parent:** [GLASS-SILICON-001 #2715](https://github.com/Luminous-Dynamics/symtropy/issues/2715), [decision-value protocol](./GLASS_SILICON_DECISION_VALUE_BENCHMARK_V0_1.md), [implementation PR #2716](https://github.com/Luminous-Dynamics/symtropy/pull/2716). Related work: [Symthaea MAT-GLASS-SCALE-001 #8480](https://github.com/Luminous-Dynamics/symthaea/issues/8480), [MAT-GLASS-MULTI-001 #8481](https://github.com/Luminous-Dynamics/symthaea/issues/8481), [GLASS-SILICON-DRIFT-001 #2740](https://github.com/Luminous-Dynamics/symtropy/issues/2740), and [GLASS-SILICON-CAUSAL-001 #2747](https://github.com/Luminous-Dynamics/symtropy/issues/2747).

## 1. Paradigm shift: model the no-go region as carefully as the target

Do not train a formulation recommender only on successful glass samples and then interpret high predicted properties as a synthesizable product. A composition can be mathematically valid yet fail to form glass; it can form glass yet be unprocessable in the intended furnace; it can be processable but miss one mandatory product property; it can satisfy properties but fail yield, energy, contamination, regulatory or cost constraints.

Treat candidate generation as a sequence of **explicit, evidence-bearing gates**, not a single opaque feasibility score. A failed or unattempted candidate must never disappear from the denominator.

There are two distinct 2026 glass-forming-region (GFR) baselines that should not be collapsed into one:

- **Composition-only, cross-family GFR prediction (March 2026):** Ji et al. use XGBoost and SHAP to predict glass-forming regions using composition alone. The paper reports F1-score above 0.89 and reports validation across oxides, oxyfluorides and metallic glasses. This is a useful transferability comparator, but the task definitions, family-specific sample counts, split design and reported F1 must be reproduced before comparing it numerically with the later model. It is a GFR classification result, not a process-feasibility or accepted-container-glass result ([Ji et al., *Journal of the American Ceramic Society*, 2026, DOI 10.1111/jace.70616](https://doi.org/10.1111/jace.70616); the paper lists [GFRprediction-XGBoost](https://github.com/jiyao-phys/GFRprediction-XGBoost)). A source-tree inspection on 2026-10-24 found its observed main head `82ba2b8e06efaf583721f6e2cc2bf895d4234cb8` contains only `.gitattributes` and three prediction-probability workbooks (oxide, oxyfluoride, metal)—no README, LICENSE, or training source files. GitHub reports `license: null`. This repository therefore does not by itself provide a reproducible training pipeline; the paper says additional data are available on request. Keep data/model admission blocked until the authors clarify terms, provide the required source artifacts, and exact bytes/rights are reviewed.

- **Negative-region-informed oxide GFR prediction (July 2026):** GFRI-ML includes 44,753 oxide-glass samples from 2,771 ternary systems and deliberately includes non-glass-forming examples. It reports 90.6% cross-validation accuracy for composition-only prediction and 94.6% for a model using 51 physical-chemical descriptors, with additional independent held-out experimental systems. These are paper-reported figures for that paper's task and splits—not a result for container-furnace production or a universal threshold for this benchmark [Ji et al., *Journal of the American Ceramic Society*, 2026, DOI 10.1111/jace.70980](https://doi.org/10.1111/jace.70980); the paper lists [GFR-ML](https://github.com/jiyao-phys/GFR-ML). A source-tree inspection on 2026-10-24 pinned main at `efc35b5a8598b77a4a4beba7ba6fd821fa15421a`; it contains README, data, example, out and src directories, but GitHub reports `license: null` and no root LICENSE file. The presence of dataset/code directories is not a reuse grant. Keep ingestion blocked pending rights clarification; file-level digests must be computed from lawfully acquired bytes, not confused with Git tree/blob object IDs. The observed repository root includes `data/composition_dataset_all.xlsx` (7,417,722 bytes) and `data/property_dataset_all.xlsx` (31,142,141 bytes), but these sizes and Git blob IDs do not supply local SHA-256 digests or usage terms.

For both papers, reproduce the original task where data terms permit, then test the stricter system/family-held-out protocol in this benchmark. Do not infer the March paper's cross-family transfer claim from the July paper's oxide-only metrics, or vice versa. A public repository listing is not by itself evidence that every file, checkpoint or label has a compatible reuse license; apply the artifact-admission gate before ingestion.

A separate strong baseline for joint properties is the 2026 PNNL low-activity-waste-glass work, which experimentally validated ML property models, incorporated measurements into updated models, and reported higher waste loading with a lower formulation failure rate. This supports evaluating *joint constraints and real failure outcomes*, not transferring its numerical results to container glass ([Lu et al., *Journal of Non-Crystalline Solids*, 2026, DOI 10.1016/j.jnoncrysol.2026.123990](https://doi.org/10.1016/j.jnoncrysol.2026.123990); [PNNL summary](https://www.pnnl.gov/publications/low-activity-waste-glass-optimization-property-models-machine-learning-part-2)).

## 2. Keep labels separate

For each candidate, preserve distinct status/label fields and their provenance:

1. **Input/chemistry validity:** formula parses; units and composition basis are declared; non-negativity and sum/normalization constraints pass; components are in the declared chemistry and data domain.
2. **Glass formation:** measured glass-forming/non-forming result under the documented synthesis and quench conditions. Keep true negative observations separate from unattempted, unknown or censored candidates.
3. **Process-window feasibility:** declared temperature/viscosity window, liquidus/crystallization risks, volatility/redox, contamination, refractory compatibility and equipment constraints are all evaluated for the target process boundary. Unknowns remain unknown.
4. **Joint product qualification:** every required property falls within its declared specification under the agreed measurement/uncertainty rule. A high score on one property cannot compensate for a failure of another required property.
5. **Manufacturing/economic feasibility:** accepted yield, energy per accepted product, throughput, scrap/rework, input availability, operating and safety constraints, and cost/environmental boundaries are separately declared.
6. **Evidence status:** measured, literature-reported, computed, simulated, synthetic, failed, unattempted, stale, censored or unresolved. Each record points to exact source/version, measurement protocol, conditions, units, instrument/calibration context where applicable, and file-level digest/rights evidence.

A literature record that is absent is not a negative. A failed synthesis is not an ordinary missing value. A glass that forms at a laboratory quench rate is not automatically feasible at an industrial cooling rate. Where labels conflict because methods or process conditions differ, retain provenance and model conditional outcomes rather than flattening them.

## 3. Evaluation lanes and split contracts

Do not pool different materials/application domains into one score.

- **Lane A — broad glass-forming regions:** reproduce the March composition-only XGBoost/SHAP baseline on its stated cross-family GFR definitions where licensed artifacts and labels permit; separately reproduce July GFRI-ML on its oxide/ternary negative-region task. Do not merge their splits or report one paper's metric as the other's result. Add system/family-held-out evaluation for both when labels and grouping metadata support it.
- **Lane B — property-constrained glass formulation:** compare joint feasibility for explicitly declared properties. Use PNNL/waste-glass benchmarks only within their stated chemistry, waste composition, property definitions, and authorized data scope.
- **Lane C — sustainable container glass:** use product- and process-specific composition, cullet/feedstock, manufacturing window, accepted yield, energy and quality evidence. The broad formation classifier is only one upstream gate.
- **Lane D — simulation-to-experiment calibration:** reproduce the SCALE workflow where lawful artifacts and compatible task definitions are admitted; preserve its target/property and campaign splits. SCALE uses >42,000 melt–quench MD simulations and 55 robotic glass syntheses and reports density bias up to 5% for some series; those observations make discrepancy calibration a critical baseline, not a universal accuracy promise ([Arendt et al., npj Computational Materials, 2026, DOI 10.1038/s41524-026-02249-w](https://doi.org/10.1038/s41524-026-02249-w)).

For each lane, predeclare at least three views:
- random split only as a diagnostic comparison with the literature;
- **system/family-held-out split** (e.g. leave ternary systems or composition families out);
- source/batch/time/experiment-held-out split when metadata support it.

Replicates, near-duplicate compositions, derivatives of the same parent sample, related simulation trajectories and rows from the same experimental campaign must remain grouped across folds. Feature selection, imputation, scaling, descriptor weighting, calibration and thresholds are fit only on training/validation partitions. Publish split identifiers and a frozen test-set digest. Report the source's original split separately from the benchmark's stricter external split; do not claim a reproduced paper score when the protocol differs.

## 4. Baselines to compare, without weakening their assumptions

Use the same permitted candidate pool, information available at decision time, target definitions and cumulative budget within each lane.

- Frozen majority / simple composition-rule control and logistic or calibrated gradient-boosted composition baseline.
- Reproduced **GFRI-ML composition-only** and **property-descriptor** methods, where the exact source version, license, preprocessing and model assumptions can be verified.
- For property prediction, domain-appropriate source baselines such as Gaussian-process property models with uncertainty, conventional mixture/property models where applicable, and independent physics/engineering constraints.
- For sequential selection, random and stratified/space-filling DOE; uncertainty-only selection; simulation–experiment discrepancy selection with declared minimum-distance rules; Expected Improvement/constrained BO; cost-aware multi-fidelity selection; and the admitted SCALE protocol where compatible.
- For multi-property tasks, compare independent per-property gates against joint feasibility modeling and a conservative intersection rule. Track the dependency assumptions; multiplying per-property probabilities is invalid unless the required independence/calibration assumptions are justified.
- Compare the full evidence-aware adaptive loop with (a) incumbent process/physics method, (b) incumbent model/twin, and (c) incumbent plus a simple selector. The small finite-pool oracle remains evaluator-only.
- LLM-based proposal or selection methods are comparable only when candidate mapping/retrieval, human review, inference, retries and failed proposals are charged and the same chemistry/action domain is available to all arms.

Mark a baseline **not comparable** if the published inputs, split, labels, target or budget cannot be faithfully aligned. Do not use cross-domain scores as glass-process evidence.

## 5. Metrics that expose unsafe optimism

Report every property and gate independently before any composite summary.

**Classification / ranking**
- per-gate precision, recall, specificity, balanced accuracy, PR-AUC and ROC-AUC (ROC-AUC is not sufficient under class imbalance);
- confusion matrices and thresholds on the locked blind set;
- false-positive rate on truly non-forming / infeasible candidates, plus false-negative rate;
- precision among the top K proposals and the number of independently verified feasible candidates found per fixed budget.

**Calibration and abstention**
- Brier score, reliability diagrams and declared calibration-error metric with binning convention;
- joint all-constraints feasibility calibration, not only calibration of individual properties;
- selective risk versus coverage and unsupported/out-of-domain abstention;
- interval coverage/width per property and subgroup; no aggregate result may hide a property with systematic bias.

**Closed-loop outcome**
- budget to first independently confirmed feasible candidate, verified feasible hits per total charged budget, and regret versus the qualified feasible frontier where one exists;
- number/fraction of proposed candidates that fail each mandatory gate, failed synthesis, retries, invalid measurements, duplicate proposals and unresolved decisions;
- number of measurements that change a decision or resolve a named uncertainty, not only acquisition-score improvements;
- utility and cost frontiers, wall-clock, solver/LLM compute and reviewer time, with failed jobs included.

## 6. Initial acceptance targets (proposed, not achieved)

- Reject **100%** of frozen invalid-formula/unit, missing-required-property, unknown-as-zero, failed-as-success, source/leakage and impossible-constraint controls; accept valid controls to prevent reject-all systems from passing.
- On admitted grouped held-outs, report the false-feasible rate with a binomial uncertainty interval. Do not state a guaranteed ≤5% false-feasible gate until there are enough independent, representative negative examples to support its predeclared upper confidence bound.
- Report joint-feasibility precision and verified-feasible-hit rate at predeclared budgets; the selector must beat or statistically tie strong baselines to claim parity, and must beat them on a predeclared primary outcome with uncertainty to claim superiority.
- Require empirical calibration checks and selective-risk/coverage curves; fix bins, confidence levels and subgroup analyses before opening test labels.
- Any candidate that lacks required evidence or falls outside validated support must abstain or return UNRESOLVED, not inherit a pass from high model confidence.
- Promotion from retrospective to prospective computation, then to physical experiments, requires a new frozen protocol and new evidence; better retrospective metrics alone do not authorize physical or industrial claims.

## 7. Data governance and reproducible packet

For every dataset, model and descriptor pipeline, pin the exact source revision, file list, licenses/rights, local SHA-256 digests, preprocessing manifest, split IDs, feature code/version, random seeds, hardware/software, dependency lock, and result digest. Keep MD5 only as legacy checksum corroboration, never as secure identity/authentication. Do not admit public archives into the benchmark until file identity, permitted reuse and hashes are reviewed.

The result packet must distinguish published paper values, local reproduction, retrospective held-out evidence, prospective compute and physical experimental outcome. The benchmark does not grant plant-control authority and does not qualify product safety by itself.

## 8. When can we claim to beat the state of the art?

A score claim requires reproducing the relevant published baseline on the same task and reporting both its original split and the stricter held-out split. A closed-loop efficiency claim requires paired, multi-seed campaigns on the same candidate pool with identical initial information and fully charged budgets. A manufacturing advantage requires independently qualified, product-boundary-matched physical outcomes; it cannot be inferred from glass-formation accuracy, synthetic regret, literature comparisons or simulator-only counterfactuals.

The differentiator should be a **feasibility-first, auditable loop**: preserve known failures and censored outcomes, predict joint feasibility with calibrated uncertainty, choose measurements that can change a real decision, enforce resource/quality constraints, record provenance and abstain when evidence is inadequate. That is a falsifiable research claim, not a claim of current superiority.


## 9. Executable synthetic contract status — 2026-10-24

The standalone Rust tool now exposes `audit-feasibility <corpus.json>`. Its synthetic fixture, `tools/glass-silicon-contract-audit/fixtures/glass-feasibility-gate.synthetic.json`, covers six known answers: one FEASIBLE control, confirmed non-forming and mandatory-property failures (INFEASIBLE), and unknown measurement, out-of-domain and stale process evidence (UNRESOLVED). Eight Rust tests mutate the fixture to verify that unsupported PASS, unknown-as-zero payloads, missing gates, unrecognized statuses, overridden dispositions and untraceable FAIL evidence fail closed.

The 54-test implementation, including the staged-feasibility controls and feasibility-gated adaptive campaign, passed its focused workflow at `784d02cc0be13519e467da455b3236cad779643a` ([run](https://github.com/Luminous-Dynamics/symtropy/actions/runs/40041754485)). The current branch adds a grouped-split manifest auditor (nine tests) and a statistical false-feasible bound evaluator (eight tests), bringing the defined suite to **71 tests**. The exact-head focused audit and general CI must pass before the expanded 71-test suite is qualified; earlier 54-test results do not qualify the added code. The grouped-split tests cover group overlap, identity/schema errors, empty splits, label-feature injection, and evidence-class preservation. The risk-bound tests cover the 600-negative threshold, underpowered samples, nonzero errors and malformed protocol inputs. These checks do not establish material validity, source-data admission, or manufacturing performance. The authoring environment has no local Rust toolchain.

This executable is still a contract auditor, not a glass-forming predictor, a furnace-process model, product certification, or a physical controller. No external dataset is admitted by the synthetic fixture. The GFRI-ML and PNNL studies are comparison baselines only until exact artifacts, data terms/rights, preprocessing and splits are independently reviewed.


## 10. Gate-conditioned adaptive campaign — synthetic control

The fixture `tools/glass-silicon-contract-audit/fixtures/adaptive-decision-pool-feasibility-gated.synthetic.json` combines the staged-feasibility idea with the existing bounded adaptive decision-tree oracle. It contains three possible worlds: a confirmed non-former and two glass-forming worlds with different feasible downstream routes.

- The cheap screen costs 1 synthetic unit / 2 seconds and separates the non-forming world from the two forming worlds.
- The downstream route-characterization step costs 3 synthetic units / 10 seconds. Either measurement by itself leaves worst-case regret at 10 synthetic units; the one-step selector's gross regret reduction is 0.
- The adaptive policy screens first, stops with the safe `reject` action on the non-forming branch, and only characterizes the route on the forming branch. It reaches synthetic terminal regret 0, with maximum path cost 4 units, duration 12 seconds and two measurements.

No scenario probabilities are assigned, so the result is **not an expected-cost estimate** and yields no industrial savings claim. It tests whether a policy conditions expensive follow-up work on a cheap feasibility signal while remaining correct on every declared branch. The focused workflow runs both the one-step comparator and adaptive tree and asserts the full branch structure. Its regression is test 54; the grouped-split auditor adds nine tests for a current total of 63, pending exact-head CI.

## 11. Executable grouped-split leakage contract

A grouped holdout is only meaningful if the assignments preserve the declared independence unit. The CLI command `audit-grouped-split <manifest.json>` checks a frozen assignment-only manifest before model fitting. The manifest contains `record_id`, `split`, and `group_keys` only; outcome labels and predictor features are forbidden in this assignment manifest. Declare each applicable grouping field explicitly (for example, composition family and synthesis/measurement campaign), with every record assigned to exactly one of `train`, `validation`, or `test`.

The auditor fails on repeated record IDs, missing/extra group keys, empty group values, unknown split names, empty train/validation/test partitions, or any declared group key occurring in more than one split. Multiple records may share the same family/campaign **inside the same split**. The CLI also emits a **SHA-256 fingerprint of the raw manifest bytes**; CI compares it to an independent `sha256sum` of the fixture so the receipt identifies the exact assignment file. The digest is a content identifier, not an authenticity proof, source-rights decision, or independent data-admission receipt. This split contract catches one common leakage source but does not prove downstream features, embeddings, preprocessing, model selection, or the actual training pipeline are leakage-free; it is a preflight, not a substitute for full pipeline review.

The checked-in `grouped-split-manifest.synthetic.json` is a synthetic control with 7 records (3 train, 2 validation, 2 test), grouped by both `composition_family_id` and `synthesis_campaign_id`. Nine tests cover the valid manifest, preservation of a declared evidence class, family and campaign overlap, duplicate IDs, missing group keys, forged outcome-label fields, missing validation coverage, and non-string grouping-field names. No external data are loaded by this audit; real datasets still need data rights, exact artifact digests, split provenance, and independent audit of every label/feature transformation.

## 12. GFR artifact-admission preflight — 2026-10-24

A metadata-only review of the linked source repositories found that the GFR baselines are **not yet admitted for reproduction or redistribution**:

- [GFR-ML repository](https://github.com/jiyao-phys/GFR-ML): GitHub API repository metadata reports `license=null`; the root listing contains `README.md`, `data/`, `example/`, `out/` and `src/`, but no root `LICENSE` file. The readme describes datasets and results, but the public listing alone does not establish rights to reuse or redistribute each data/model artifact.
- [GFRprediction-XGBoost repository](https://github.com/jiyao-phys/GFRprediction-XGBoost): GitHub API metadata also reports `license=null`; the root listing contains `.gitattributes` and three prediction workbooks for metal, oxide and oxyfluoride systems, with no root README/LICENSE or complete training-source package in the listing.

This review is not legal advice and does not conclude that use is prohibited; it means the benchmark does not have a documented reuse basis yet. Keep both sources at **reference-only / data-admission blocked** until the authors or repository metadata clarify permitted use, exact versions are frozen, every acquired artifact is hashed, and the expected preprocessing/splits can be reproduced. The benchmark must not download, ingest or redistribute these files merely because the repository is public. No GFR data/model files have been admitted or used in this benchmark.

## 13. Statistical gate for false-feasible risk

A point estimate of zero false-feasible predictions is not evidence that the true risk is negligible. The Rust command `audit-false-feasible-bound <summary.json>` calculates a one-sided Hoeffding upper confidence bound over a caller-declared set of independent, independently adjudicated negative units:

`upper = min(1, observed_rate + sqrt(ln(1 / (1 - confidence)) / (2 * n)))`

where `n` is the number of independent negative units and `observed_rate = false_feasible_count / n`. The declared target must be pre-registered, the test set frozen, and evidence references provided for the independence unit and negative labels. The tool rejects malformed counts, absent references, non-preregistered protocols, and unfrozen test sets; it returns `UNRESOLVED` when the upper bound exceeds the target.

For a 5% target at one-sided 95% confidence, 600 independent negative units with zero false-feasible outcomes yield a Hoeffding upper bound of approximately 4.996%, just under the target; 599 zero-error units are still above 5%, and a single false-feasible error among 600 also misses the gate. The bound is intentionally conservative and uses the declared *independent group count*, not correlated raw rows. It is not a guarantee under arbitrary distribution shift: the independence/representativeness and label-adjudication references still need independent review.

The current fixtures are synthetic arithmetic controls only. A `PASS` means the bound is met **if** the input counts, independent-unit definition, negative labels and frozen test protocol are valid; the command cannot verify those assertions or certify the model. Do not report an industrial false-feasible rate from these fixtures.

## 14. Statistical gate for false-feasible risk

A zero-error test set does not imply zero true false-feasible risk. The Rust command `audit-false-feasible-bound <summary.json>` calculates a one-sided Hoeffding upper confidence bound for the rate of infeasible candidates falsely declared feasible:

`upper = min(1, observed_rate + sqrt(ln(1 / (1 - confidence)) / (2 * n)))`

Here (n) is the declared count of independent, independently adjudicated negative groups and `observed_rate = false_feasible_count / n`. A result cannot pass if the count fields are malformed, references for the independent-unit definition or negative-label adjudication are missing, the protocol was not pre-registered, or the test set was not frozen. A valid summary whose upper bound exceeds the target returns `UNRESOLVED`, not a pass.

For a 5% target at one-sided 95% confidence, **600 independent negative groups with zero false-feasible errors** give an upper bound of approximately 4.996%. The 599-group/zero-error control remains UNRESOLVED; one false-feasible error among 600 also misses the target. This is intentionally conservative and distribution-free. Count independent family/campaign units, not correlated rows. The independent-unit, representativeness and label-quality references are caller declarations that require review; the command cannot independently verify them or guarantee performance under distribution shift.

The positive and underpowered fixtures are synthetic arithmetic controls. A PASS means only that the reported bound meets the declared target under the supplied assumptions; it is not a claim that a real glass model has this false-feasible rate.
