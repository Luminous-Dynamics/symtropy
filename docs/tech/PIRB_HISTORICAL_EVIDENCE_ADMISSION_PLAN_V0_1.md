---
title: PIRB Historical Evidence Admission Plan
version: 0.1
status: research plan; no dataset admitted
authority: MeasurementOnly
---

# PIRB historical evidence admission plan v0.1

## Decision

Keep the current replay fixture synthetic. Do not represent it as a historical evaluation. The next data experiment should use a public archived forecast/outcome pairing only after the data steward records the permitted use, release vintage, outcome definition, observation unit, temporal availability, and an acceptable spatial aggregation. Public data publication does not automatically make every analysis or derived use harmless.

## Candidates and what they can establish

### Candidate A — UCDP

Official download center: https://ucdp.uu.se/downloads/

The Uppsala Conflict Data Program (UCDP) publishes its current datasets under CC BY 4.0, with reuse and redistribution allowed subject to citing the publications listed for the chosen dataset. Its GED v26.1 captures organized-violence events at individual-day and village-level granularity; the UCDP Candidate Events series has monthly releases and states a global lag of no more than one month. Select the actual dataset and codebook before defining the target.

Fit:
- Candidate outcome source and data-quality/coverage study.
- A label-lag sensitivity experiment using historical release vintages if the historical snapshots needed for the analysis are available.
- Aggregated event-outcome characterization at a predeclared geography/time scale, after a conflict-sensitivity and re-identification review.

Not fit by itself:
- Alert precision/recall, lead-time estimates, or response-completion evaluation. An event record is not a record of a warning being issued, a warning being received, a response dispatched, or a response completed.
- Claims about service delivery, consent, local authority, peacebuilding intervention effects, or prevention.

Safety note: GED may contain village-level event geocoding. Do not publish or operationally surface fine-grained locations of vulnerable people or sites merely because the raw data are licensed for reuse. Coarsening has to be designed for the context, not selected only to satisfy a numeric cell-size threshold.

Required citation: UCDP Dataset Download Center and the specific selected dataset's codebook/publication. Verify the release version, historical-vintage availability, fields, target definition, and geography before building an importer.

### Candidate B — VIEWS archived forecasts

Official data access: https://viewsforecasting.org/data/
FAQ and partitioning: https://viewsforecasting.org/faq/
Model directory: https://viewsforecasting.org/models/

VIEWS publishes monthly conflict forecasts, forecast codebooks, APIs, and replication material. Its data page currently identifies a release named `fatalities003_2026_08_t01`, forecasting September 2026 through August 2029 from inputs through August 2026. Its dashboard offers forecast/outcome context from 2022 onward, and its FAQ documents how training, calibration, predictor updating, and forecasting periods are separated.

Potential fit:
- Retrospective scoring of an already-issued archived forecast against subsequently observed outcomes.
- Horizon-specific discrimination/calibration work **only** if the archive supplies genuine probabilities, the target definition matches, the forecast origin/cutoff is recoverable, and outcomes have matured.
- Baseline benchmarking of temporal leakage controls and coverage by country/grid-month, using the model's declared unit, target, and horizon.

Limitations:
- VIEWS forecasts are not actual local warning-system alerts and do not encode alert acknowledgement, human review, available responder resources, dispatch, or verified service delivery. They therefore cannot validate the warning-to-response chain.
- Do not score the latest forward-looking forecast as if it were historical truth. Use an archived forecast vintage and join only outcomes that occur after that forecast's input cutoff. Later revisions to outcome data must not be allowed to leak back into decision-time features.
- This harness currently accepts binary recorded alerts, not probabilities. A separate, reviewed VIEWS adapter is required to preserve probability, horizon, unit, model/release ID, input cutoff, observed target, and uncertainty; do not silently threshold probabilities. Any threshold should be selected on a development/calibration window and frozen before test evaluation.
- The public data-access page establishes availability, not by itself every license/redistribution right for every file and dependency. Confirm the exact dataset's codebook, attribution, and reuse terms before downloading, running, or publishing derived artifacts.

### Candidate C — ACLED

Official terms: https://acleddata.com/eula and https://acleddata.com/contentusage

Do not use ACLED as the initial public benchmark source without a written, project-specific review and permission where needed. Its EULA and Content Usage Terms restrict some benchmarking, AI/ML testing/development, substitute-product, access, redistribution, and targeting use cases. The applicability depends on the exact license and contemplated use; the safe default is to exclude it until the rights question has been resolved by the data owner and counsel.

### Candidate D — locally governed warning-to-response records

This is the only candidate class that can directly test a real warning-to-response chain, if it includes valid timestamped records for alerts, accountable owners, acknowledgement, feasibility/resources, dispatch, completion, independent verification, outcomes, and known missingness.

Admission prerequisites:
- Documented local authority and data-controller permission for this specific evaluation purpose.
- Locally reviewed protocol, legal/ethics/protection review where appropriate, community participation and dissent mechanisms, and a complaint/withdrawal path that are not delegated to the model.
- Clear separation of warning issued, warning received, response dispatched, service completed, independently verified completion, observed outcome, and any causal estimate.
- Data minimization, retention/deletion rules, access controls, and a threat model for revealing identities, vulnerable locations, or sensitive testimony.
- No raw row-level records in a public repository or public CI artifact. Run the evaluator inside the authorized environment; publish only a reviewed summary or protected aggregate.
- Predefined outcomes, horizons, false-alert cost, subgroup reporting and suppression policy, uncertainty, missing-data treatment, and credible counterfactual where a causal claim is sought.

## Admission checklist

Before any importer can mark a dataset eligible, record:

1. Dataset identifier, owner, immutable release/version, retrieval date, codebook and citation.
2. Legal/contractual permission for the specific use, publication and derived artifacts.
3. Unit of analysis and intended interpretation; geography and time granularity.
4. Prediction/decision origin and exact feature-availability cutoff, including revision/vintage history.
5. Outcome definition, ascertainment delay, censoring/maturation rule, and known coverage/missingness.
6. Geography crosswalk and coarsening/privacy review; prohibited fields and minimum-safe publication rule.
7. Mapping from original schema into PIRB schema, including discarded fields and why.
8. Independent expected-count/metric oracle, negative controls for leakage, and dataset-specific acceptance criteria.
9. Claim ceiling: descriptive forecast performance, alert-to-response process performance, observed outcome change, and causal impact are separate classes of claim.

Unanswered permission, chronology, outcome, or disclosure questions mean `NOT_ADMITTED`, not a permissive default.

## Staged plan

1. Continue synthetic negative controls and validate the tool's receipt on its current exact head.
2. Build a separate probability-forecast adapter for an archived VIEWS vintage only after checking release schema/terms, and freeze the selected outcome, horizon, spatial unit and threshold policy before test scoring.
3. Use UCDP as a separately versioned outcome/coverage input with citation and a release-vintage rule; don't assume every candidate event is final or use labels that postdate the forecast cutoff as features.
4. Treat this as a forecasting performance study only. The response chain remains unevaluated until authorized historical alert/response records are admitted.
5. Seek independent evaluation and locally governed field protocols before any impact or deployment-safety claim.

## Explicit non-claims

A public dataset's license does not prove its observations are true, complete, representative, suitable for a target, harmless to publish, or available at the time an operator would have acted. A passing schema or replay receipt is not consent, authority, response effectiveness, peace impact, causal attribution, or deployment authorization.
