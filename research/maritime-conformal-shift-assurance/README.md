# Maritime conformal shift and missingness assurance v0.1

A standalone, dependency-free Rust benchmark for **joint predictive coverage under temporal drift and outcome-dependent missingness** in sequential maritime scheduling.

This is a follow-on to the multi-vessel scarce-reserve recovery PR #7188. It does not duplicate the berth optimizer, the active-sensing engine or a general conformal library.

## Why this benchmark exists

Makhado, Sejeso and Paepae (2026), *Conformal prediction-based sequential scheduling for container terminal operations under uncertainty*, evaluates 241 vessel calls at Durban Container Terminal from April 2024 through March 2025. It uses a chronological 70/15/15 train/calibration/test split and 90% split-conformal prediction intervals for arrival-delay and handling-time predictions.

Paper:
- DOI: https://doi.org/10.1016/j.orp.2026.100409
- University of Johannesburg repository: https://ujcontent.uj.ac.za/esploro/outputs/journalArticle/Conformal-prediction-based-sequential-scheduling-for-container/9963662507691
- Earlier manuscript: https://ssrn.com/abstract=6631198

The published rolling-window table reports mean arrival/handling coverage of 89.8%/84.8% (87.3% overall average), with 2025 Q1 coverage falling to 79.7% and 71.9%. The study itself attributes the deterioration as likely distributional shift. Its nominal conformal schedule is 5.1% worse than deterministic BAP at 50 vessels, and its third stage is an aggregate makespan estimate rather than a bay-level QCSP. The 241-call operational dataset is available on request, not as an openly downloadable data release.

The new benchmark isolates two failure modes that ordinary split-conformal point metrics can hide:

1. **Marginal-versus-joint coverage.** Separate 90% arrival and 90% handling intervals do not imply 90% simultaneous coverage for the arrival+handling vector. A joint maximum-normalized-residual score calibrates a single band for both targets.
2. **Temporal drift with MNAR observation.** The distribution changes across stationary, drift and burst windows. High-residual outcomes are more likely to be missing. A deployed calibrator cannot update from outcomes it never observed; the evaluator retains those hidden truths only for latent-coverage adjudication.

This is a deterministic synthetic stress fixture, not a reproduction of the DCT dataset and not a fitted model of actual DCT missingness.

## Methods compared

| Method | Calibration | Update policy |
|---|---|---|
| FIXED_MARGINAL_SPLIT | Separate arrival and handling split-conformal quantiles | Frozen calibration |
| FIXED_JOINT_SPLIT | One maximum normalized residual score | Frozen calibration |
| ROLLING_JOINT | Joint score in a rolling window | Update on observed outcomes only |
| ADAPTIVE_JOINT_ACI | Rolling joint score with adaptive miscoverage level | Update on observed outcomes only |

The joint score is defined as the maximum of normalized arrival and handling residuals:
- arrival residual divided by 5 hours;
- handling residual divided by 20 hours.

The joint conformal quantile is the finite-sample order statistic k = ceil((n + 1) × (1 − alpha)). The same quantile scales both arrival and handling residual bounds, so the joint event is tested directly. This is still a marginal-in-time finite-sample guarantee under exchangeability; it is not conditional coverage for every vessel class, season, berth or weather regime.

The ACI update changes the future miscoverage level after an observed miss/success. If the outcome is missing, no update is made. This deliberately exposes a central limitation: adaptive recalibration cannot learn from outcomes systematically hidden by an MNAR observation process unless a separate evidence source or a defensible missingness model exists.

## Frozen synthetic experiment

- 160 pre-shift calibration residual pairs.
- 360 chronological test pairs in three 120-sample regimes: STATIONARY, TEMPORAL_DRIFT and BURST_TAIL.
- Deterministic SplitMix64 seed.
- The observation channel hides high normalized residuals much more often than ordinary residuals.
- The policy receives only observed/missing status and observed residuals. It never receives evaluator-only hidden residuals.
- The evaluator retains all generated outcomes to compute latent joint coverage and the observed-coverage optimism gap.

The generated numbers are not estimates of DCT performance. Their purpose is falsification of an unjustified coverage claim when time dependence and selected missingness are present.

## Industry comparators and fair comparison protocol

The industry landscape already includes capable local optimization systems. The programme should not compare a synthetic interval-calibration crate directly against a vendor TOS; it should compare each decision layer on matched inputs.

| Comparator | Operational scope | Evidence available | Our fair comparison |
|---|---|---|---|
| **Kaleris N4 / Advanced Optimization** | Terminal planning/control across vessel, yard, rail and gate; vendor modules include RTG, terminal-truck and yard optimization | Kaleris/YILPORT Gebze reported a 21% increase in RTG moves per hour over three months with its existing 31-RTG fleet. This is a vendor/operator-reported deployment result, not an independently controlled causal estimate. https://www.porttechnology.org/news/kaleris-n4-platform-delivers-21-increase-in-rtg-productivity-at-yilport-gebze/ | Compare RTG move productivity only after matching yard topology, job mix, shift, fleet, congestion and labor rules. Keep evidence calibration as a separate layer. |
| **Portchain Quay** | Dedicated berth planning, crane/gang allocation and vessel tracking; KCT announced expanded deployment in 2026 | Product/deployment materials describe berth, crane and gang planning with scenario support. https://portchain.com/portchain-quay/ | Compare berth/crane feasibility, schedule changes, recovery latency and operator acceptance on the same frozen snapshots. |
| **Tideworks Mainsail 10 + Spinnaker** | Cloud TOS, vessel/yard/gate operation and planning-system integration; POMTOC deployment reported in March 2026 | Vendor announcement describes production go-live and API integration framework. https://tideworks.com/pomtoc-accelerates-terminal-modernization-to-support-regional-trade-growth-with-tideworks-cloud-tos/ | Treat as an integration/execution baseline, not a published CP optimizer result. Compare event semantics, API lifecycle, replay, change audit and fail-closed behavior with permissioned test fixtures. |
| **INFORM terminal optimization** | Modular AI-supported yard, vehicle, crane and rail optimization | Vendor describes optimization modules and TOS integration. https://www.inform-software.com/industries/ports-terminals | Compare each module's own KPI on matched workloads; do not infer evidence independence from multiple modules sharing a TOS or telemetry source. |

### Required matched baseline matrix

For the DCT reproduction (#7221), compare:
- deterministic BAP / QCAP reference from the paper;
- XGBoost point forecast + deterministic schedule;
- separate Q90/quantile regression intervals;
- fixed marginal split conformal;
- fixed joint split conformal;
- rolling/block conformal;
- adaptive conformal inference;
- the paper's SAH-50 scenario averaging baseline;
- a small-instance exact integrated BAP/QCAP oracle where tractable.

For terminal execution comparisons, keep Kaleris/Portchain/Tideworks/INFORM product scope separate and record version, configuration, interface contract, resource model and input availability. Vendor-reported improvements are useful leads, not independent SOTA rankings.

### Beyond predictive coverage

A wider interval can trivially improve empirical coverage while making schedules worse. Every method must therefore be compared on a joint scorecard:
- joint arrival/handling coverage and interval width;
- schedule feasibility and optimality gap where an exact bound exists;
- vessel waiting, berth idle time, makespan and quay-crane productivity;
- recovery latency and replan frequency;
- missed receiver deadlines and cargo displacement;
- reserve/energy consumption and emissions;
- false-safe rate: a plan labelled feasible that violates a hard constraint after adjudication;
- abstention/unknown rate and cost of additional observations;
- subgroup and temporal-slice coverage under vessel class, berth segment, season and operating regime.

Use matched calibration sample counts and chronological evaluation windows. Report paired seed-level results, uncertainty intervals and timeouts. Never compare a method's 50-vessel result against a competitor's 30-vessel optimum or treat a timeout as a solved objective.

## Required metrics

Each method/window row emits:
- total, observed and missing sample counts;
- observed joint coverage;
- evaluator-only latent joint coverage;
- observed arrival and handling marginal coverage;
- mean arrival and handling interval widths;
- final adaptive alpha (where applicable);
- an explicit claim status.

Claim status is deliberately fail-closed:
- NOT_CERTIFIED_MISSINGNESS whenever the evaluation window contains missing outcomes;
- OBSERVED_UNDER_TARGET when complete observed outcomes fall below 90%;
- OBSERVED_AT_OR_ABOVE_TARGET only when outcomes are complete and observed joint coverage reaches 90%.

This small benchmark does not implement a confidence sequence or provide a probabilistic guarantee under arbitrary drift. A real deployment needs independent outcome adjudication, a calibrated observation/missingness process, temporal shift monitoring, and pre-registered operating thresholds.

## Reproduction

From repository root:

    cargo fmt --manifest-path research/maritime-conformal-shift-assurance/Cargo.toml --all -- --check
    cargo clippy --locked --manifest-path research/maritime-conformal-shift-assurance/Cargo.toml --all-targets -- -D warnings
    cargo check --locked --manifest-path research/maritime-conformal-shift-assurance/Cargo.toml --all-targets
    cargo test --locked --manifest-path research/maritime-conformal-shift-assurance/Cargo.toml --all-targets
    cargo run --quiet --locked --manifest-path research/maritime-conformal-shift-assurance/Cargo.toml --bin conformal-shift-audit

## Next research gates

1. Request the DCT 241-call dataset, code and split definitions from the authors under explicit data-use terms.
2. Reproduce the exact 70/15/15 chronological split and every published rolling window before changing methods.
3. Add rolling, block, weighted and adaptive conformal baselines with matched interval widths and prequential ordering.
4. Add joint vessel-level and schedule-level simultaneous coverage; separately measure berth-feasibility and receiver-deadline violations.
5. Stratify by vessel size, berth segment, season, weather, congestion and handling regime without leaking future outcomes.
6. Add independent observation/ground-truth adjudication for missing outcomes; compare observed and latent coverage under MCAR, MAR and MNAR.
7. Couple calibrated intervals to multi-vessel scarce-reserve recovery, recourse cost, participant displacement and physical capacity constraints.
8. Compare deterministic MIP, Q90, XGBoost point prediction, fixed split CP, joint CP, rolling CP, ACI and the published SAH-50 protocol on identical inputs.
9. Integrate berth and crane stages with decomposition diagnostics or exact small-instance bounds; do not claim to solve QCSP until bay-level task sequencing is modelled.

## Claim ceiling

Synthetic software research only. This crate does not reproduce the published DCT results, certify 90% field coverage, infer DCT's hidden missingness process, validate a real port schedule, command a TOS, authorize resource allocation or establish cargo delivery.
