# Federated Service Calibration and Privacy Benchmark v0.1

**Parent calibration programme:** [#7240 — SERVICE-PREDICTION-CALIBRATION-001](https://github.com/Luminous-Dynamics/symtropy/issues/7240)  
**Federated child:** [#7242 — FEDERATED-SERVICE-CALIBRATION-001](https://github.com/Luminous-Dynamics/symtropy/issues/7242)  
**Branch:** \`research/federated-service-calibration-v0-1\`  
**Status:** protocol design and benchmark specification; no secure aggregation or differential privacy implementation is claimed.

## 1. Why federation is useful—and dangerous

Mobility depots may have small local samples, but very different operating domains: chemistry, pack generation, age/SOH, charger/inverter, climate, altitude, payload, duty cycle and site-grid behavior.

Pooling raw battery, customer and aircraft telemetry is not a suitable default. It creates privacy, commercial confidentiality and data-governance risk. Yet pooling every depot's calibration scores into one global distribution creates a different risk: the global interval may look calibrated while one site or chemistry is catastrophically undercovered.

The design must therefore preserve two distinct outputs:
1. **Fleet-level aggregate learning signal**, which is available only when the cohort and privacy thresholds are satisfied.
2. **Site-local validity decision**, which remains owned by the site and can abstain regardless of the global aggregate.

A global summary never overrides local undercoverage.

## 2. Research baseline and transfer boundary

Shi et al. (UAI 2026, PMLR 337:6232–6260) propose Federated Conformal Prediction for Joint Shift. Their method builds privacy-preserving global sketches of client nonconformity distributions and performs local importance-weighted calibration against that summary. The paper reports a finite-sample approximation term depending on sketch size and calibration count, and improved efficiency on its heterogeneous benchmarks. [PMLR paper](https://proceedings.mlr.press/v337/shi26a.html).

This is the closest methodological comparator, but its guarantee depends on its stated distribution-shift, weighting and data assumptions. A mobility deployment with temporal dependence, correlated packs, non-IID chemistry and changing site permissions must reproduce those assumptions rather than inherit the paper's theoretical claim by analogy.

A second comparator is Huang, Ma and Michailidis (UAI 2026), who use online martingale PAC-Bayes certificates and a predict-then-update protocol for time-series forecasting under dependence and shift. [PMLR paper](https://proceedings.mlr.press/v337/huang26b.html). The federated design should compare privacy-preserving distribution sketches with explicit temporal-risk certificates, not assume one solves the other's problem.

## 3. Protocol architecture

### 3.1 Local site auditor

Each site runs #7240 locally. Raw telemetry, individual sample identifiers, pack serials, event receipts and calibration residuals remain in the local trust boundary.

A local release candidate contains only:
- protocol and metric-definition version;
- aggregation epoch and forecast model/version;
- coarse, approved stratum taxonomy;
- nominal coverage level and measurement boundary;
- clipped/bounded nonconformity histogram or quantile sketch;
- sample count and covered count;
- interval-width/score aggregates;
- local shift/undercoverage/insufficient-support state;
- site membership credential and signature over the summary;
- privacy parameters, contribution limit and release-budget reference.

No raw residual, customer ID, pack serial, flight ID or event receipt is included in the global summary.

### 3.2 Compatibility gate

The coordinator must refuse to merge summaries unless they match exactly on:
- model ID/version and forecast horizon;
- target metric and physical measurement boundary;
- units and normalization;
- nominal coverage;
- quantile-score definition and bin edges/sketch algorithm;
- stratum taxonomy and cohort policy;
- observation/forecast time cut-off;
- protocol version and privacy mechanism.

Do not merge pack-terminal Wh with charger-input Wh. Do not merge chemistry/charger/site strata merely because their metric name is the same.

### 3.3 Bounded contribution and secure aggregation

Each authenticated site contributes at most one bounded summary per stratum/metric/epoch. Per-site sample counts and histogram mass are clipped to declared maximum contributions before aggregation.

Secure aggregation should ensure the coordinator receives the aggregate sum, not each site's cleartext histogram. Use a threshold protocol with an explicit collusion threshold and availability/recovery behavior. Membership credentials are consumed once per epoch; old or revoked membership cannot replay a contribution.

A histogram is not automatically private. Sparse bins, repeated releases and differencing may reveal one site's data even when raw samples are withheld.

### 3.4 Differential privacy is a separate claim

If the protocol is called differentially private, define:
- adjacency: one sample, one pack-cycle, one customer or one site;
- contribution clipping and maximum per-site/per-epoch samples;
- Laplace or Gaussian mechanism and sensitivity;
- \(\epsilon,\delta\) for each release and cumulative accountant over epochs;
- composition across overlapping strata/metrics;
- distributed noise or trusted setup so the coordinator does not see the exact aggregate before noise;
- utility degradation in quantiles, coverage and interval width;
- membership/participation leakage boundaries.

Secure aggregation is not differential privacy. A noisy histogram without a documented adjacency, sensitivity and privacy accountant is not a DP result. v0.1 intentionally makes no DP claim.

## 4. Local and global decisions

The coordinator returns a signed fleet summary containing:
- aggregate counts and noisy/sketched score distribution;
- approximation error bound or empirical sketch error;
- number of participating sites and minimum cohort result;
- model/metric/stratum compatibility digest;
- privacy budget consumption and secure-aggregation epoch receipt;
- aggregate shift/undercoverage summary.

Each site independently decides whether that summary is applicable to its own local stratum. Site-local outcomes remain:
- supported by local holdout;
- undercoverage alarm;
- insufficient samples;
- configuration/stratum mismatch;
- privacy/cohort suppressed;
- abstain / unsupported.

A fleet aggregate can help a site choose an initial correction or allocate calibration effort, but it cannot convert an unsupported local stratum into a supported one.

## 5. Threat model

Assume:
- curious coordinator;
- fewer than threshold-colluding sites;
- malicious site submits fabricated/extreme score counts;
- compromised or revoked membership credential;
- repeated-query/differencing attacker;
- sparse strata and small local fleets;
- poisoning to widen intervals or hide undercoverage;
- non-IID site populations and correlated common-cause events;
- network partition and replayed epochs;
- privacy-accountant bypass or stale protocol version.

Required mitigations:
- membership authentication and key rotation/revocation;
- per-site contribution caps and score-range clipping;
- secure aggregation with declared threshold and dropout semantics;
- minimum participating-site and minimum cohort thresholds;
- query/release budgets and privacy composition accounting;
- signed protocol/metric taxonomy;
- stale/replay/cross-epoch rejection;
- robust per-site outlier diagnostics without revealing raw scores;
- no global-only field qualification.

## 6. v0.1 executable acceptance contract

Start with a deterministic bounded aggregation evaluator over already prepared site summaries. It must implement semantics only—not claim secure aggregation or DP:
- validate each local summary's schema, membership ID, epoch, model/config/metric/stratum compatibility and signature-verification status supplied by an independent verifier;
- reject more than one contribution from a site in the same epoch/stratum/metric;
- enforce per-site maximum sample count and per-bin contribution;
- enforce minimum participating sites and minimum total sample count before returning an aggregate;
- merge fixed-bin counts deterministically with checked arithmetic;
- compute aggregate quantile/rank and compare with an independently generated centralized histogram;
- mark sparse bins/strata as suppressed;
- report local undercoverage alarms independently of fleet aggregate;
- enforce per-site release budget and repeated-query suppression in the supplied ledger snapshot;
- emit canonical input/output digests and a reason-coded abstention;
- never modify site-local model or calibration state;
- no global model update or operational authority.

A later version must replace the declarative membership/signature status with a real credential verifier and secure-aggregation protocol before any privacy guarantee is stated.

## 7. Negative controls

- one site contributes twice in the same epoch;
- site exceeds maximum contribution count or bin sensitivity;
- fewer than minimum sites; no aggregate is published;
- sparse stratum is published after only one site's contribution;
- two aggregates differ only by one site's contribution and reveal it;
- query budget exceeded;
- old epoch replay or revoked member replays a contribution;
- model version, metric units, measurement boundary, nominal coverage or sketch-bin mismatch;
- input order changes the merged output digest;
- checked count overflow;
- a global nominal aggregate masks a local hot-shift undercoverage alarm;
- missing local site audit is silently treated as a good result;
- coordinator sees exact individual histograms despite secure aggregation being claimed;
- privacy accountant omits composition across epochs or metrics;
- DP flag set without adjacency, sensitivity, mechanism and epsilon/delta.

## 8. Benchmarks and target ladder

Targets, not achievements:
- 100% deterministic merge against centralized exact histogram reference for bounded synthetic inputs;
- zero duplicate or replayed site/epoch contributions;
- zero publication below minimum participating-site/cohort thresholds;
- local site undercoverage never suppressed by fleet-level averages;
- zero incompatible boundary/model/stratum merges;
- publish sketch approximation error versus exact pooled quantile;
- publish coverage/interval-width change and per-site worst-stratum coverage;
- publish bytes per epoch, coordinator merge p50/p95/p99 and missing-site tolerance;
- publish poisoning rejection/robustness and privacy/utility trade-off;
- if DP is implemented, independently verify the accountant, cumulative \(\epsilon,\delta\), and empirical privacy/utility properties under the declared adjacency.

## 9. Ownership and non-claims

- #7240 owns site-local chronological forecast/outcome calibration and deployment-shift audits.
- #7242 owns federated summaries, secure aggregation and the privacy protocol.
- #6921/#7074/#7129/#7133 own service request decisions, event-derived receipts and information constraints.
- #6674/#6700/#6705 own physical pack identity/state/custody/charger qualification.
- #6803/#6809/#6829/#6837 remain hard flexibility, recovery, capability lease and promise journal gates.

This v0.1 specification is not a privacy guarantee, differential privacy implementation, secure aggregation implementation or a statistical guarantee under arbitrary non-IID shift. It defines a bounded testable path toward those capabilities without exporting raw mobility telemetry.

---

## 8. Executable v0.1 bounded reducer

The protocol now has a deterministic Rust reducer and CLI:

- \`tools/ev-passenger-cost-frontier/src/federated_calibration.rs\`
- \`tools/ev-passenger-cost-frontier/src/bin/federated_calibration_screen.rs\`
- \`tools/ev-passenger-cost-frontier/schemas/federated_calibration_v0_1.schema.json\`
- \`tools/ev-passenger-cost-frontier/fixtures/federated_calibration_v0_1.json\`

This first executable implementation consumes already-prepared site summaries. It does **not** authenticate credentials, verify signatures, hide individual contributions from the coordinator, perform secure aggregation, add DP noise or commit the release ledger.

### 8.1 Reducer contract

Every summary must match the exact compatibility key: model ID/version, forecast horizon, metric kind, measurement boundary, unit, stratum, nominal coverage, score definition and fixed histogram edges. It rejects mismatched boundaries (for example pack-terminal Wh against charger-input Wh), model versions, strata, bins, or epochs.

The reducer also enforces:
- a registered roster and one contribution per site per epoch;
- one unique local evidence digest per site summary;
- per-site calibration sample and per-bin count caps;
- exact histogram mass = declared calibration sample count;
- holdout covered count no greater than holdout count and bounded width sums;
- bounded local audit age and exact epoch match;
- release-ledger presence for each participating site;
- rejection of exhausted per-epoch release budgets and repeated query-scope digests;
- deterministic merge and canonical input digest independent of input order;
- minimum participating-site and minimum total calibration-sample thresholds.

When a cohort is too small, the report withholds participant counts and aggregate distribution values. It does not publish a seemingly harmless histogram with a sparse cohort.

The quantile uses the bounded histogram and reports the selected bin's upper edge. This is an approximation at the declared bin resolution, not an exact residual quantile. A non-finite conformal order-statistic rank is rejected rather than silently clipped into a valid-looking rank.

### 8.2 Local authority is not pooled

The reducer reports how many sites declare:
- locally supported;
- undercoverage alarm;
- insufficient local support;
- unsupported stratum or unknown status.

The fleet distribution cannot turn a local undercoverage alarm or unsupported stratum into local support. The output permanently sets site-local qualification, fleet field qualification and operational authority to false. The local status is a site-supplied audit result; this reducer does not independently verify that result.

### 8.3 Frozen synthetic known answer

The fixture contains three sites with one compatible metric/stratum:

| Measure | Expected |
|---|---:|
| Participating sites | 3 |
| Total calibration samples | 30 |
| Merged score-bin counts | [6, 9, 15] |
| 90% order-statistic rank | 28 |
| Quantile upper bin edge | 40 |
| Holdout samples | 30 |
| Holdout covered | 24 |
| Pooled holdout empirical coverage | 800‰ |
| Holdout interval-width sum | 300 |
| Mean holdout interval width | 10 |
| Reported local undercoverage alarms | 1 |
| Reported local insufficient-support sites | 1 |

All three contributions and their local evidence digests are synthetic placeholders. The fixture tests deterministic arithmetic and policy semantics only.

### 8.4 Privacy and security boundary

The current reducer processes summaries in cleartext. It is not a secure aggregation protocol and does not protect a site from a curious coordinator. Its cohort threshold is only a release-suppression rule; it is not a privacy guarantee. The local evidence digest is an opaque reference, not a cryptographic signature or verified credential.

Secure aggregation requires a separately specified protocol, member authentication, threshold/collusion semantics, dropout handling, replay protection and an adversarial implementation review. Differential privacy additionally requires a declared adjacency relation, clipping, sensitivity, mechanism, per-release and cumulative \`epsilon/delta\` accounting, and utility measurements. These must not be inferred from the reducer's functional tests.

The output marks:
- \`membership_cryptographically_verified=false\`;
- \`secure_aggregation_performed=false\`;
- \`differential_privacy_applied=false\`;
- \`release_budget_consumed=false\`;
- all local, fleet and operational qualification flags false.

The coordinator's ledger snapshot is only a preflight view. A production release must use an atomic compare-and-swap or consume-once operation at publication time; this reducer does not reserve or consume a release token.

## 9. Updated comparator and benchmark plan

The federated method should be evaluated against:
1. **Centralized pooled residuals** as an exact reference when raw data are permitted in a benchmark environment.
2. **Fixed-bin deterministic aggregation** as implemented here, with error reported against the centralized empirical quantile.
3. **Quantile sketches** (for example KLL or t-digest variants) with a declared approximation metric and bounded memory.
4. **Federated Conformal Prediction for Joint Shift**, Shi et al., UAI 2026 / PMLR 337:6232–6260, whose approach shares privacy-preserving score sketches and performs local importance-weighted calibration under its stated distribution-shift assumptions.
5. **Online certificate-driven calibration** for dependent time series, Huang et al., UAI 2026 / PMLR 337:2244–2273, as a different way to reason about temporal dependence and distribution shift.

These methods address different assumptions. Federated score sketches do not automatically solve concept shift, non-IID temporal dependence, poisoned membership or absent target-domain evidence. A fleet result remains an input to a local decision, not a local certificate.

### 9.1 Targets (research goals, not achievements)

| Benchmark | Target |
|---|---|
| Merge vs independent centralized histogram | 100% exact bin-count agreement |
| Input permutation | 100% identical canonical digest and output |
| Duplicate site/epoch contribution | 0 accepted |
| Compatibility mismatch | 0 merged |
| Per-site/per-bin cap overflow | 0 accepted |
| Stale or future summary | 0 accepted |
| Cohort below configured minimum | 100% aggregate suppression |
| Local undercoverage alarm masked by fleet aggregate | 0 cases |
| Release-budget replay | 0 repeated query releases |
| Quantile approximation | Publish absolute and normalized error vs exact pooled rank |
| Resource cost | Publish bytes, memory, merge p50/p95/p99 and site count |
| Poisoning robustness | Report worst-site influence and bounded-contribution impact |
| DP utility (future work) | Publish epsilon/delta, accounting horizon and coverage/width degradation |

A useful federation must improve calibration sample efficiency or aggregate quantile efficiency without making the worst-supported site less safe. Reject-all and publish-everything are both failures: the former destroys utility, the latter may disclose sensitive site information.

### 9.2 Negative controls required before v0.2

- same site contributes twice under different IDs in an epoch;
- site summary appears with a stale or future audit cutoff;
- summary carries a different bin edge, unit, score definition or nominal level;
- cross-boundary summaries are merged;
- local undercoverage is hidden by high fleet coverage;
- one site contributes at or above a prohibited count cap;
- query-scope digest is replayed after publication;
- release ledger is exhausted or has conflicting duplicate rows;
- cohort threshold is bypassed by including registered-but-nonparticipating sites in the count;
- histogram mass differs from the declared sample count;
- quantile rank is non-finite at small sample sizes;
- input order changes merged output or digest;
- a synthetic summary is labelled source-authenticated, privacy-preserving or field-qualified.

The next security increment must implement authenticated membership and atomic release-ledger consumption. The next privacy increment must implement and independently audit secure aggregation and/or DP accounting. Neither can be claimed from this reducer.
