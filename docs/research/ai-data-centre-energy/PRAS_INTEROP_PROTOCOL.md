# PRAS Interoperability and Known-Truth Protocol

**Protocol:** GRID-PRAS-INTEROP-001  
**Version:** 0.1  
**Parent research issue:** #3687  
**Status:** interchange contract and deterministic fixture specification  
**External reference:** NLR PRAS 0.9.0

## 1. Purpose and division of responsibility

Use the National Laboratory of the Rockies (NLR) Probabilistic Resource Adequacy Suite (PRAS) as the independent probabilistic, multi-region bulk-system adequacy comparator. Symtropy should not reproduce PRAS's sequential Monte Carlo engine as a second production adequacy engine.

The division is:

- **Symtropy:** prove the workload/service contract, physical resource trajectories, signed PCC import response, rebound/recovery, owner authorization, provenance and condition-specific evidence.
- **PRAS:** evaluate a frozen system model with chronological unit outages, energy-limited resources and interregional transfer constraints; calculate system risk and capacity credit under its documented operating model.
- **Independent operator/model review:** establish whether regional abstractions, topology, data and results are representative of a real grid. A PRAS result is not automatic NTCSA/TSO acceptance.

This protocol defines a matched interchange and validation process. It does not claim a current Symtropy-to-PRAS adapter exists or that any PRAS run has been executed.

## 2. Version and artifact pinning

Every run must pin and record:

- PRAS package version (initial target: `0.9.0`);
- Julia runtime version and platform/architecture;
- complete Julia `Manifest.toml` digest and project environment;
- PRAS SystemModel HDF5 artifact digest;
- `pras_dataversion` attribute read from the actual HDF5 artifact;
- original source model, weather-year, outage and network-data identifiers/digests;
- scenario/load-profile manifest and all transformation parameters;
- simulation specification, sample count, seed, threading setting and result specifications;
- source revision, executable/script digest and output digests.

Do not assume that the PRAS package version and the HDF5 `pras_dataversion` are the same string. Read the file's format version and check compatibility using the pinned PRAS reader. Never hand-author a `.pras` HDF5 file from an incomplete schema transcription.

PRAS documents the SystemModel as HDF5. Its mandatory root attributes include `pras_dataversion`, `start_timestamp`, `timestep_count`, `timestep_length`, `timestep_unit`, `power_unit` and `energy_unit`. Store the timezone offset in the timestamp and preserve the declared power/energy units.

## 3. Chronology, units and array orientation

The interchange manifest uses the explicit canonical convention:

- timestamps are UTC instants with an explicit ISO-8601 offset;
- timestep duration and units are explicit and constant within one SystemModel;
- power and energy units are explicit; the initial qualification fixture uses MW/MWh;
- regional load profiles are keyed by stable region identity;
- manifest arrays are time-major: one value per consecutive timestamp;
- adapter code must explicitly map time-major manifest data to the PRAS in-memory region/time orientation;
- all resource capacity and transition-probability series must have the same time extent as the region load series;
- no implicit resampling, timezone conversion, interpolation, row truncation or missing-value fill is permitted.

The official `.pras` HDF5 format stores chronologically increasing timestep rows in the file representation. PRAS's Julia data structures expose resource collections and their time series through Julia arrays. Adapter tests must explicitly verify round-trip orientation using a non-symmetric multi-region/multi-time fixture; square matrices or constant profiles are not sufficient to detect transposition.

A model is invalid for comparison when timestamps, interval duration, timezone/calendar semantics, region identity/order, units or horizon differ. For calendar-day metrics, compute day membership from the preserved timestamps and timezone rule rather than a caller-supplied ordinal alone.

## 4. Map the verified flexibility at the PCC boundary

The preferred V0.1 bridge for a measured or independently verified Symtropy trajectory is to apply its **complete signed net PCC import reduction** to the corresponding PRAS regional demand profile.

For interval (t), region (r):

\[
L^{flex}_{r,t} = L^{base}_{r,t} - R_{r,t}
\]

where:

- (L^{base}_{r,t}) is the frozen reference regional load in MW;
- (R_{r,t}) is the paired Symtropy net PCC import reduction in MW;
- positive (R) reduces grid demand;
- negative (R) is rebound/recovery and increases grid demand.

Requirements:

1. PCC-to-region mapping must be explicit, versioned and supported by a topology/source record. Do not infer a substation from a city, province or national aggregate.
2. Preserve every timestep, including post-event rebound and recovery.
3. Reject non-finite values, out-of-horizon values, negative resulting regional load, unit mismatch and changed timebase.
4. Keep baseline and flexible SystemModels identical in all supply, storage, outage, interface, line, weather and non-candidate load assumptions. Only the qualified candidate trajectory may differ.
5. Preserve the original and transformed load arrays and record a digest for each.
6. Never add the same flexibility again as a PRAS DemandResponse, storage resource, generator or second load adjustment.

This profile method does not claim PRAS has modeled the internal data-centre scheduler. It feeds the externally verified net PCC trajectory to the independent system model.

## 5. Native PRAS demand-response mapping is a separate, lossy comparator

PRAS DemandResponses represent simplified load borrowing/payback with borrow capacity, payback capacity, borrowed-energy capacity, borrowed-energy interest, allowable payback period and availability transitions.

This is useful as a generic demand-response comparator, but it does not natively preserve per-job identity, useful-work obligations, model/request semantics, latency SLOs, migration destination capacity or contractual deadlines.

Use native PRAS DemandResponses only for a separately labelled sensitivity that intentionally tests that abstraction. Document the borrow/payback translation and the information lost. Do not combine that resource with a load profile that already contains the same measured flexibility. PRAS itself accounts unrepaid borrowed demand as shortfall; demand-response-attributable shortfall is included in system shortfall and must not be added to it again.

## 6. Metric semantics crosswalk

Use PRAS `ShortfallSamples` where sample-level event-day and energy-tail metrics are needed. Request only the sample detail needed for the study because sample-level result objects can be memory intensive.

| Quantity | PRAS 0.9 meaning | Symtropy/NTCSA handling |
|---|---|---|
| LOLE | Expected number of simulation event-periods containing shortfall | Do not label as contiguous event count; preserve timestep resolution |
| LOLD | Expected number of calendar event-days containing at least one shortfall period | Preserve timestamp/calendar boundaries |
| EUE | Expected unserved energy across the study horizon | Compare in matching MWh and horizon |
| Energy CVaR | Upper-tail severity of sample-level horizon EUE | Preserve alpha, sample population, tail mass and standard error |
| LOLH | Not the same field as PRAS LOLE | Derive elapsed shortage duration from shortfall samples and timestep lengths |
| NTCSA LOLE | Contiguous shortage event count | Derive separately from timestamp-ordered samples; close an event only after adequacy is restored |
| NTCSA CVaR | Shortfall-power magnitude in MW in the specified tail population | Not PRAS energy CVaR; preserve its observation unit and percentile convention |

PRAS LOLE counts event-periods; it is not an assumed hourly count. Two 15-minute shortfall intervals are two event-periods but only 0.5 hours. LOLD is not obtained by dividing LOLE by 24. One shortfall period can make a day an event-day, and event periods can be clustered or dispersed.

PRAS energy CVaR is calculated from the sample-level total EUE distribution. It is not an interval power CVaR, nor a confidence interval on mean EUE. The PRAS result's Monte Carlo standard error must be reported separately from tail severity.

For a multi-weather-year study, preserve weather-year weights and sample-level outcomes. A weighted average of per-year CVaRs is generally not the CVaR of the pooled mixture. To report pooled tail risk, construct the explicitly weighted mixture of per-sample horizon outcomes and apply the preregistered tail-mass convention to that mixture.

## 7. Matched simulation and capacity credit

For each frozen weather-year/system case:

- run the reference and flexible models with identical simulation horizon, timestep, units, supply/resource identities, outage transition parameters and scenario data;
- use matched PRAS Monte Carlo seeds and sample counts;
- keep resource identities/order and outage model consistent so common random numbers can reduce comparison noise;
- record all seed/sample settings and the returned Monte Carlo errors.

For capacity credit, use PRAS's native EUE- or LOLE-based `ELCC`/EFC method where its assumptions match the study question. Preserve the returned lower and upper bounds and any hypothesis-test/convergence warnings; do not report only the midpoint.

PRAS documentation defines ELCC as additional load added in every timestep while maintaining the reference system's adequacy level. The region(s) to which that constant load is added are a modelling choice and must be reported. This is not automatically equivalent to a new site-specific peak-only load or a nodal interconnection rating.

Do not assume PRAS's built-in capacity-credit method calculates a CVaR-based ELCC. The initial documented method supports EUE or LOLE risk metrics. Keep EUE/LOLE capacity credit separate from CVaR tail-risk comparison unless a validated method is explicitly added.

## 8. Multi-region and network boundary

PRAS regions are copper-sheet aggregates: individual electrical buses and within-region transfer limits are not represented. Interregional interfaces and their lines can represent transfer limits and outages between regions.

Therefore:

- a PRAS multi-region result is stronger than a single-zone copperplate result for interregional deliverability;
- it is not a full bus-level AC/DC power-flow, voltage, protection or distribution-substation model;
- no local data-centre capacity value may be inferred without a defensible PCC-to-region mapping and appropriate finer-grained network validation;
- report the regional model boundary and unresolved within-region constraints.

Start with the published RTS-GMLC PRAS model for adapter smoke tests, then a small deterministic multi-region known-truth fixture, then appropriately sourced and mapped regional scenarios. A reconstructed South African PRAS representation must be labelled a reconstruction, not the official NTCSA model.

## 9. Weather-year and Monte Carlo experiment design

PRAS sequential Monte Carlo samples chronological unit outage state transitions and tracks energy-limited resource state. It does not automatically make an arbitrary deterministic load profile a calibrated weather/load probability distribution.

Use one of these explicit experiment designs:

1. separate SystemModels/runs per source weather year, with weather-year probabilities retained in a separate ensemble manifest; or
2. a single explicitly concatenated multi-year chronology only when cross-year state carryover and output annualization semantics are intended and verified.

For each year, align demand, renewable/weather profiles, thermal derating, storage availability and flexibility availability to the same chronology. Preserve correlations; do not independently shuffle marginal time series.

Report per-year metrics and spread, pooled results only under a declared weighting convention, independent-seed convergence, Monte Carlo standard error, tail effective sample size and sensitivity to sample count. If a rare-event tail cannot be estimated reliably, say so.

## 10. Deterministic known-truth gates

The frozen fixture corpus in `benchmarks/ai-data-centre-energy/v1/pras-interop-fixtures.json` includes:

- hourly shortage metrics (3 event-periods, 3 shortage hours, 1 event-day, 40 MWh EUE);
- 15-minute shortage semantics (2 event-periods, 0.5 shortage hours, 1 event-day, 5 MWh EUE);
- a midnight-crossing shortage (2 event-periods, 2 shortage hours, 2 event-days, 20 MWh EUE);
- a paired signed workload trajectory (10 MWh baseline EUE, 0 MWh flexible EUE, 5 MW EUE-based ELCC under the declared deterministic system);
- a rebound trap (30 MWh baseline EUE versus 50 MWh flexible EUE);
- a two-region transfer-limited location case showing that identical relief has different value at different regions.

The fixture values are synthetic arithmetic controls. They are not a PRAS execution result, an NTCSA reproduction or field evidence.

## 11. Acceptance gates

Proposed engineering gates, not standards or achieved results:

- 100% agreement with all deterministic known-truth metrics within declared numeric tolerances.
- Exact round-trip model/timebase/unit/region mapping, including non-symmetric time-region matrices.
- Matched baseline/flexible models differ only by the pinned candidate intervention.
- No lost rebound, altered chronology or double-counted flexibility.
- PRAS LOLE, LOLD, EUE and energy-CVaR labels match their documented semantics.
- Pooled weather-year tail risk uses explicit mixture weights, not an average of per-year CVaRs.
- Common-seed pairing and convergence diagnostics are retained in each result receipt.
- No claim of within-region nodal deliverability from a PRAS copper-sheet region.
- Independent reproduction from pinned model, environment, seeds and data digests.

## 12. Implementation sequence

- **V0.1:** this protocol and deterministic mapping fixture; validate schema, arithmetic and metric definitions.
- **V0.2:** Julia-only optional harness pinned to PRAS 0.9.0; load/save native `.pras`, run deterministic known-truth and RTS-GMLC smoke cases, emit machine-readable result receipt.
- **V0.3:** paired signed-load-profile adapter and common-seed ELCC/EUE/LOLE comparison.
- **V0.4:** multi-region interface/line known-truth case and location-sensitive portfolio sweeps.
- **V0.5:** multi-weather-year Monte Carlo convergence, weighted tail metrics and sensitivity.
- **V0.6:** South African reconstructed regional case after NTCSA source-table, topology, resource and assumption reconciliation.
- **V1.0:** independent external reproduction and documented discrepancy report.

Julia remains an optional benchmark/interoperability runtime. It is not a production Symtropy dependency. No Julia/PRAS execution, Rust compilation or test pass is claimed by this protocol document.
