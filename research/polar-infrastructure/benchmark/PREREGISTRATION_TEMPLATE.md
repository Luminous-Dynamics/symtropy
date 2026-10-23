# Polar resilience benchmark preregistration template

**Purpose:** freeze the comparison contract before tuning, scoring, or inspecting candidate outcomes. This is a research template; completing it does not constitute safety approval or standards conformity.

Copy this file into a study-specific directory and fill every mandatory field before the first candidate run. Use `UNKNOWN` where an input is not known and explain the consequence; do not silently invent values.

## 1. Study identity

- Study ID:
- Owner / accountable engineering authority:
- Independent reviewer:
- Date/time frozen (UTC):
- Protocol commit SHA:
- Code repository + exact candidate commit SHA:
- Evidence class at study start: `S0` / `S1` / `S2` / `S3` / `S4`
- Intended decision this evidence may inform:
- Decisions this study explicitly may not authorize:

## 2. Site and service contract

- Context: Arctic land community / Antarctic station / mobile ice platform / other specified site.
- Site description and authorized geography resolution:
- Applicable local governance, land/sea and environmental constraints:
- Design envelope and source for each variable (temperature, wind, icing, snow, precipitation, sea/ground conditions, salinity, access and resupply):
- Service contracts: identifier, unit, owner-approved minimum service fraction, criticality and time resolution.
- Responsible person/group for approving each safety/service limit:
- Why each floor is physically and operationally meaningful:
- Unmodeled services or dependencies:
- Community/Indigenous authorization and data governance status, where applicable:

**Hard rule:** service limits are defined and approved before scoring. Do not tune a floor to make the candidate pass. If an owner-approved limit is unavailable, mark the corresponding gate unevaluated; do not manufacture a safety threshold.

## 3. Data, model and provenance

For every source/model/artifact, record:
- Canonical source URL/DOI and publisher/steward:
- Retrieval time, access/license/permission basis:
- Raw bytes SHA-256 and byte count, or explicit `not_acquired`:
- Spatial/temporal coverage, units, resolution, quality flags and known gaps:
- Parser/normalizer code commit and configuration digest:
- Simulation model version, parameters, calibration data and validation split:
- Source data used to calibrate the model versus data reserved for evaluation:
- Known uncertainty and conflicts:
- Whether inputs are observed, reanalysis, forecast, synthetic, inferred or model outputs.

A URL is not an acquisition receipt. A file hash establishes byte identity only. Reanalysis is not direct station measurement; aggregate data do not become point observations without a defensible mapping.

## 4. Scenario split and leakage control

Enumerate every scenario before running candidates.

| Field | Required entry |
|---|---|
| Scenario ID | Stable unique identifier |
| Family | nominal / component fault / correlated failure / weather shift / logistics / sensor or comms / restart / maintenance / other |
| Geography and envelope | Explicit and sourced |
| Construction method | observed episode / calibrated simulation / synthetic negative control |
| Role | calibration / tuning / validation / locked holdout / stress-only |
| Failure trigger | Testable and precise |
| Required inputs | Units and provenance |
| Expected detectable failure | What the harness must fail on |
| Owner-approved hard gates | Named IDs; no generic invented thresholds |
| Exclusions | Predeclared reason, never post-hoc |
| Probability weight | Only where independently justified; otherwise `none` |

- How are temporally adjacent or spatially correlated episodes kept from leaking across splits?
- Who controls the locked holdout?
- How are previously observed failures handled in subsequent versions?
- How is scenario coverage reported, including families not yet represented?

**No-probability rule:** when the frequency of a hazard cannot be defended, compare a declared unweighted stress set and report worst-case results. Do not describe the count or unweighted average of scenarios as event frequency or expected risk.

## 5. Comparison arms and fair resource budgets

Minimum arms as applicable:
1. Current/installed or documented incumbent policy.
2. Simple transparent rule-based baseline.
3. Conventional optimizer (LP/MILP/MPC) with model assumptions disclosed.
4. Robust/stochastic method only where the uncertainty model is justified.
5. Candidate controller/design.
6. Ablations isolating individual components.
7. Known-defective negative controls.

For every arm, freeze:
- Method family, public implementation/version and exact configuration.
- Allowed tuning data, search space, number of trials and wall-clock budget.
- CPU/GPU/device, precision, parallelism, memory and energy measurement boundary.
- Random seed policy and number of replicates.
- Failure/retry policy and reporting rule.
- Engineering labour, maintenance and spares assumptions.
- Any baseline-specific adaptations and why they are fair.

Do not compare a heavily tuned candidate with a default baseline or silently remove a failed run.

## 6. Primary endpoints, hard gates and uncertainty

- Primary endpoint(s), with exact formula and unit:
- Secondary endpoints:
- Hard-gate vector and authoritative source for each limit:
- Statistical/uncertainty method and rationale:
- What constitutes a meaningful improvement and how was this threshold chosen?
- Missing/invalid values and failed-run treatment:
- Confidence/credible interval and sensitivity plan:
- Tail statistic, confidence parameter and reason for using it:
- Pareto objectives and dominance tolerance:
- No-go / stop condition:
- Who may approve protocol deviations?

Report at minimum:
- per-scenario service fraction, service shortfall, gate outcome and breach duration;
- worst-case and unweighted stress-set summary when probabilities are unavailable;
- expected/tail metrics only with justified scenario weights;
- fuel/energy, resupply burden, recovery time, repair time/spares, lifecycle cost/emissions and compute cost where supported;
- calibration error, source coverage, parameter uncertainty and all gate failures.

Never replace an engineering gate with a weighted average. Pareto non-dominance is not admissibility or safety approval.

## 7. Predeclared acceptance and falsification

- Exact acceptance rule:
- Conditions that falsify the central hypothesis:
- Scenario/control designed to expose overclaiming:
- Which result would make the candidate clearly worse?
- Minimum number and type of independent holdout episodes:
- What evidence level is required before any field recommendation:
- External review owner and competence boundary:
- What claims remain prohibited even after the benchmark passes:

## 8. Reproducibility manifest

At release, archive a machine-readable run manifest with:
- protocol, code, model, configuration and data artifact SHA-256 digests;
- exact commit SHAs and toolchain versions;
- scenario IDs, split assignment, input/output byte digests and execution environment;
- command line, seeds, resource budgets, run starts/ends, retries and exit codes;
- raw outputs, calculated metrics, hard-gate vector, negative-control outcomes and known limitations;
- signed reviewer receipt only if signatures are genuinely implemented and verified.

A queued, skipped, missing, or failed run is not a pass. A passing software benchmark does not upgrade synthetic data into physical evidence or authorize commissioning.
