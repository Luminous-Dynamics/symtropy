# Johannesburg Water Service Cascade T0 — protocol v0.1

**Status:** Synthetic conformance design pack; not a data acquisition, forecasting or field qualification result.  
**Date:** 17 November 2026  
**Parent:** Symtropy [#5799 JW-WATER-CASCADE-001](https://github.com/Luminous-Dynamics/symtropy/issues/5799)

## 1. Research change

The water-service problem must be represented as an evidence chain rather than a single "water available" variable:

`catchment observations → utility-reported asset state → point-of-use service`

These layers are related, but they are not interchangeable:

- Rainfall is not runoff or reservoir inflow.
- Gauge stage is not discharge without a station- and period-valid rating relation.
- Catchment state is not treated-water production.
- A utility asset status-board report is not household tap pressure, flow or delivered volume.
- A notice's expected completion is not verified restoration.
- A publisher-derived end to a reported outage is not proof that a customer's service was physically restored.

The benchmark therefore evaluates *whether a cross-layer inference is admissible*, as well as whether a prediction is useful. A correct abstention is a result, not a model failure.

## 2. Source and evidence boundary

The metadata-only source manifest is separate from the synthetic input and expected-output oracle.

| Population | Candidate source | What it can support | Current state |
|---|---|---|---|
| Utility-reported supply | [Watersearch API](https://watersearch.co.za/api), compiling Johannesburg Water boards/notices | Publisher-reported asset status, method-versioned outage/notice derivations | Live documentation/examples were observed in the parent source-admission work; immutable bulk bytes and checksums are not admitted |
| Hydrometric / catchment | [DWS Hydrological Services](https://www.dws.gov.za/hydrology/) and separately admitted station records | Station/parameter/time-bounded hydrometric observations | Candidate sources only; no station/file is admitted by this pack |
| Meteorological | [SAWS Climate Services](https://testing.weathersa.co.za/home/aboutclimateatsaws) | Station/parameter/time-bounded weather observations under source terms | Candidate only; no file is admitted |
| Point of use | Future independently metered test-cell or consented site | Local pressure, flow and delivered-volume observations at an identified boundary | No physical installation or point-of-use dataset exists in this pack |

Watersearch documentation states that it is not operated by Johannesburg Water or the City and that its compilation licence is provisional pending public release. Revalidate the licence field and `X-Licence` header for every acquired release. Do not treat the publisher's present release identifier as a frozen dataset version.

**All IDs, dates, statuses, episodes and instrument values in the fixture are synthetic.** The manifest contains no raw bytes or checksums. No Watersearch, JW, DWS, SAWS or household records are copied into the test corpus.

## 3. Scope and ownership

This pack owns the water-specific cross-layer join and inference contract only. It composes with:

- [#3894](https://github.com/Luminous-Dynamics/symtropy/issues/3894): generic dependency graph and infrastructure cascade evaluator.
- [#5350](https://github.com/Luminous-Dynamics/symtropy/issues/5350): Highveld source and catchment admission.
- [#5792](https://github.com/Luminous-Dynamics/symtropy/pull/5792): Watersearch reported-service source candidate and licence/evidence-ceiling note.
- [#5338](https://github.com/Luminous-Dynamics/symtropy/issues/5338): catchment-conditioned hydrology baseline ladder.
- [#5364](https://github.com/Luminous-Dynamics/symtropy/issues/5364): use-qualified service continuity.
- [#5425](https://github.com/Luminous-Dynamics/symtropy/issues/5425): lifecycle cost per verified household service.
- Mycelix #5431/#6112: original source provenance, event lineage and reported-versus-observed service evidence.
- Mycelix #6034/#6029/#6048/#6054/#6057: metrology, measurand, uncertainty, timebase and channel applicability.
- Mycelix #6100: intended-use water-quality evidence.

It does not create a new hydraulic solver, hydrology model, event registry, asset registry, water ledger, tariff engine, instrument passport, household outcome authority or service controller.

## 4. T0 input and output contract

The input fixture, independent oracle, source manifest and schemas are separate files. Each case binds:

- one or more supply IDs and SAST valid dates;
- the publisher-reported daily status and bypass flag;
- scheduled-closure state where relevant;
- the publisher's outage method ID and derived episode fields where relevant;
- notice completion and the first later fair/off-bypass valid date;
- hydrometric parameter, station identity, time basis and rating-curve reference;
- point-of-use boundary, local pressure/flow and evidence class;
- typed dependency evidence (or explicit unknown);
- the requested claim ceiling.

The oracle requires a single exact disposition for each case. It also preserves the per-supply status vector, publisher episode/censoring information, notice-join result, timebase disposition, causal-claim ceiling and point-of-use evidence class.

## 5. Frozen conformance cases

| ID | Challenge | Required invariant |
|---|---|---|
| C01 | `no_report` day | Never convert missing board data into fair/normal service |
| C02 | One reported low-supply day | Reported asset state does not become tap-level truth |
| C03 | Two supplies linked to one suburb | Preserve per-supply vector; do not collapse to a suburb/tap label |
| C04 | Warned plus bypass | Keep bypass distinct from fair/off-bypass |
| C05 | No-water during scheduled closure | Do not count it as an unplanned event by default |
| C06 | Four no-board days under C41 | Accept a single publisher-derived bridged episode |
| C07 | Five no-board days under C41 | Reject a single merged episode over the declared bridge cap |
| C08 | No reported end | Preserve right-censoring and duration lower bound |
| C09 | Notice completion vs first later fair/off-bypass board | Compute a day-bounded reported return-to-status delay interval |
| C10 | Notice and board reference different supplies | Reject the asset join |
| C11 | Timezone/time basis absent | Do not perform an exact temporal join |
| C12 | Gauge stage without rating relation | Do not call stage discharge |
| C13 | Temporal co-occurrence but unknown dependency | Association only; no causal claim |
| C14 | Local fixture pressure/flow exists | Keep the measurement at its local boundary and synthetic evidence class |
| C15 | Synthetic input requests field qualification | Reject claim-ceiling promotion |

C41 cases are *method-conformance checks against a supplied publisher-derived episode*. This benchmark does not independently reproduce Watersearch's outage algorithm or replace its rule version. An acquired release must retain the exact C41/method revision and raw report lineage.

## 6. Temporal join rules

1. Preserve source-native valid time separately from publication/ingestion time.
2. A SAST daily board row represents a day-level interval, not an exact event timestamp.
3. Exact timestamp comparisons require a declared timezone/offset and compatible time semantics.
4. If the time basis is missing, inconsistent or unknown, return an indeterminate join; never guess UTC or SAST.
5. A same-day daily join is coarse alignment. It can support a stratified association test but not event ordering or causal attribution.
6. A point-of-use sample/window joins only to its own declared boundary and observation window. It does not inherit the upstream supply's status as ground truth.

Notice delay is an interval when the first return evidence is daily:

- Let the first fair/off-bypass supply-day be the half-open SAST interval `[00:00 on valid_date, 00:00 on the next date)`.
- Subtract the offset-aware announced completion instant from both interval bounds.
- Report `[lower_hours, upper_hours)`; do not invent an exact board observation time.

It measures *reported return-to-status*. It is not physical repair completion, pressure restoration or customer-level service recovery.

## 7. Dependency and causal rules

Every upstream-to-downstream edge must be typed as one of:

- directly measured physical dependency;
- operator-documented operational dependency;
- topology-derived but unverified dependency;
- statistical association only;
- unknown / not admitted.

A time-aligned edge is not automatically a causal edge. Causal claims require a predeclared graph, confounder inventory, intervention or defensible quasi-experimental design, negative controls and out-of-sample replication. Candidate confounders include power interruptions, planned works, pump/valve operations, treatment constraints, transfers, demand peaks, communication failures and unobserved asset states.

## 8. Benchmark ladder and metrics

Keep populations separate; do not create a pooled leaderboard from different evidence classes or data cadences.

**Reported-status forecast baselines**
- persistence of the last valid report, retaining `no_report`;
- seasonal/day-of-week status climatology fitted on training data only;
- notice-only reference for scheduled work;
- supply/asset-type-conditioned prevalence baseline;
- any documented operational forecast, where use and data rights permit.

**Point-of-use transfer baselines**
- local persistence and local demand/power covariates;
- upstream reported-status features added as a separate ablation;
- a model using both layers only after local instrument data are independently admitted.

**Metrics**
- Brier score and Brier Skill Score for declared binary events;
- multiclass Brier/log score for consistently defined status classes;
- CRPS/CRPSS or censoring-aware survival scores for duration distributions;
- calibration/reliability, sharpness and uncertainty intervals;
- useful warning lead, false warnings per asset-month and missed severe-event rate;
- data coverage and `no_report` rate;
- added predictive value over local-only baselines;
- decision value and cost per independently verified service-loss event avoided, only when real intervention/outcome evidence exists.

Negative skill must remain visible. If a reference score is zero, the corresponding skill score is undefined—not infinite. Do not tune on a public evaluation year and call it blind.

**Splits**
- chronological future holdout;
- complete outage/event holdout;
- held-out supply asset;
- held-out JW system where sample size allows;
- separate point-of-use site/fixture holdout.

Never split adjacent days from the same outage episode randomly across training and evaluation. Refit preprocessing and calibration only on the training partition.

## 9. Acceptance and falsification

For this T0 pack:

- exactly 15 unique case IDs in the input and oracle;
- exact input/oracle case-set equality;
- all cases remain `synthetic_only`;
- unknown/missing status is never recoded as fair/normal;
- multi-supply outputs preserve supply identity;
- C41 bridge cap is not exceeded;
- censored duration remains a lower bound;
- notice return-to-status joins require exact supply identity, an offset-aware notice time and a day-bounded return interval;
- unknown time basis produces an indeterminate join;
- stage without rating-curve evidence does not become discharge;
- unknown dependency never becomes causation;
- local point-of-use evidence remains boundary-specific;
- field qualification is rejected for synthetic inputs.

The read-only Rust evaluator is implemented in `tools/water-service-t0/src/bin/jw-water-cascade-t0.rs` on a branch stacked over the existing `water-service-t0` package. It compares all 15 computed case outputs against this separate oracle and emits a deterministic receipt binding input, oracle, both schemas, source manifest, root Cargo.lock, package manifest, implementation bytes and exact checkout revision.

### Exact-head software validation — 17 November 2026

GitHub Actions run [#1227 / 44094878255](https://github.com/Luminous-Dynamics/symtropy/actions/runs/44094878255) completed successfully for PR source head `1fb8bc38cf5a4f927df2216f40178f9b2acbfdc7`. The workflow evaluated the merged checkout revision `7ddf297f9de2ba968159b3e794c13c1399650a89`, which merges that source head into the declared parent branch.

- `cargo fmt --check`: PASS.
- `cargo test --locked --package water-service-t0`: PASS; 22 unit tests total (8 library + 7 new cascade binary + 7 existing binary).
- `cargo clippy --locked --package water-service-t0 --all-targets -- -D warnings`: PASS.
- Existing water-service T0 evaluator: 23/23 expected outputs matched, zero mismatches.
- Johannesburg cascade T0 evaluator: 15/15 expected outputs matched, zero mismatches.
- Exact-revision receipt gate: PASS; evidence artifact [14333142561](https://github.com/Luminous-Dynamics/symtropy/actions/runs/44094878255/artifacts/14333142561) uploaded.

The receipts deliberately retain `evidence_class: synthetic_only` and `qualification_status: NotRun`. This confirms software behavior against frozen synthetic contracts only. It does not admit live datasets or qualify a hydraulic network, sensor, household service, causal dependency, field forecast, repair outcome or water saving. No action/control output is permitted.

## 10. Nonclaims

This design pack does not establish:

- a live Johannesburg Water supply map or customer outage;
- a verified outage start/end or tap restoration;
- a causal connection between rainfall, river flow, utility assets and household service;
- real water savings, predictive skill, warning lead time or model superiority;
- a permitted Watersearch bulk data release;
- a DWS/SAWS dataset acquisition, calibration or rating-curve validity;
- potable or non-potable water quality;
- regulatory compliance, field installation or control authority.

The first evidence-raising step after T0 is rights-cleared, release-pinned source acquisition and cross-checking against original JW report/notice records. Point-of-use claims require independently metered, consented test-cell/site evidence.
