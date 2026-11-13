# SUF Research and Product Programme Map v0.3

**Programme:** Subterranean Utility Fabric (SUF)  
**Parent:** Symtropy #4910 — observability-first infrastructure verification  
**Snapshot:** 2026-11-13  
**Branch:** `research/subterranean-utility-fabric-v0-1`  
**Status:** public research and executable reference components; not a deployed utility product

## 1. Programme thesis

Turn existing and future infrastructure sensing into an evidence-qualified, owner-authorized observability and investigation service.

The system must be able to say:
- what is actually observed;
- what is degraded, blind, missing or unknown;
- which event class and minimum magnitude are qualified;
- where the observation is on the fibre and, separately, on the surveyed route;
- what independent truth supports the event and the no-event denominators;
- how uncertainty and calibration validity propagate;
- what follow-up measurement is valuable;
- what remains an owner/surveyor/engineer decision.

The objective is not a bigger alarm dashboard or another interrogator. The initial product complements existing interrogators and utility asset systems.

## 2. Programme architecture

`Owner rights + source identity
→ immutable vendor-native artifact
→ typed source-observation envelope
→ fibre/topology/revision binding
→ calibrated fibre chainage
→ surveyed route station and CRS
→ event-conditioned observability and uncertainty
→ value-aware investigation proposal
→ owner/human acceptance
→ independently reviewable evidence bundle`

Each arrow is a separate qualification boundary. A downstream transform cannot promote an unsupported upstream claim.

## 3. Workstream map

| ID | Workstream | Owner / issue | Current output | Evidence state | Next useful increment |
|---|---|---|---|---|---|
| W00 | Programme architecture and claim policy | Symtropy #4910 / PR #4911 | SUF observability-first direction and inter-project ownership links | Research design and public work | Preserve programme map and exact-head evidence |
| W01 | DAS event observability | Symtropy #4972 | Event-conditioned metric definitions, SOTA baselines and independent-truth protocol | Research protocol; Rust coverage evaluator | Obtain a rights-cleared event/no-event sample and bind physical-event IDs |
| W02 | Active/opportunistic calibration | Symtropy #5002 | 22-case campaign; adaptive measurement and datum-identifiability protocol | Research protocol; EVSI/EIG arithmetic executable | Reproduce trajectory/geolocalization baseline before building adaptive policy |
| W03 | Data lifecycle and retention | Symtropy #5003 | 24-case campaign for compression, buffer, source integrity and replay | Research protocol | Implement sample-only immutable artifact ingestion and replay |
| W04 | Vendor-neutral source interchange | Symtropy #5004 | JSON Schema, raw/derived examples, 16 negative controls; Rust semantic validator | Structural fixtures and 14 Rust tests | Full JSON Schema 2020-12 validation and one authorized vendor sample |
| W05 | Observability Evidence Gateway MVP | Symtropy #5006 | Read-only MVP specification | Product architecture only | Offline vertical slice with one export + one approved GIS/fibre inventory |
| W06 | Signal preservation / denoising | Symtropy #5007 | 20-case campaign, SVD-NLM/wavelet/physics-informed baselines | Research protocol; no Luminous denoiser | Reproduce one baseline on its own authorized task/split |
| W07 | Observability-by-design | Symtropy #5008 | 28-case campaign, 21 metrics; existing/spare/shared/dedicated/point/hybrid/conventional comparisons | Design protocol only | Apply to one non-live/renewal corridor case with competent engineering review |
| W08 | Public corpus rights/readiness | Symtropy #5023 | Four-corpus metadata/rights index and schema | Metadata audit; no bulk downloads | Resolve licence and obtain a small sample only where allowed |
| W09 | Terrestrial utility field corpus | Symtropy #5025 | Safe tiered collection protocol, independent truth and split requirements | Proposed acquisition protocol; no site/partner secured | Identify a non-live terrestrial testbed and independent survey partner |
| W10 | Spatial evidence interoperability | Sol-Atlas #755 / PR #757 | MUDDI/SensorThings profile, authorization/query boundaries | Profile proposal, not conformance | Add implementation-level conformance tests on an actual service |
| W11 | Owner authority, custody and acceptance | Mycelix existing workstreams | Existing authorization/provenance infrastructure to compose | Separate owner/ledger qualification | Bind gateway receipts to owner-issued authority without self-authorizing assets or journals |

## 4. Executable engineering already present

Standalone Rust crate: `tools/suf-fibre-calibration/`

- **Calibration kernel:** bounded piecewise-linear optical-range → physical fibre chainage → route station → projected position; exact endpoint/direction/revision/time binding; method and artifact digests; no asset attribution.
- **Route-station orientation:** station origin and increasing direction bind to physical endpoints; reverse B-to-A interrogation has explicit tests.
- **Source envelope:** raw-vs-derived lineage, SHA-256 shape, axis/unit constraints, chainage calibration reference and sensing-modality boundaries.
- **Decision value:** expected information gain and expected value of sample information are separate; net value is suppressed for incomparable cost/loss units.
- **Coverage:** complete route-length state partition; unique physical-event IDs and false-alarm episode IDs; separate independent truth gates; POD Wilson interval and false alarms per observable fibre-km-day; blind exposure remains explicit.
- **Synthetic numeric benchmark:** 10,000 seeded optical-range inputs against independent bent-route oracle equations, with boundary and monotonicity checks.
- **Sample-size planning:** idealized zero-miss one-sided binomial upper-bound calculation for a single independent event stratum.

The tests establish arithmetic/contract behavior only. They do not establish DAS measurement accuracy, event detection performance, real fibre-to-asset geometry or field utility safety.

## 5. Industry and research comparison

Maintain separate, task-matched baselines for:

1. interrogator acquisition and signal quality;
2. vehicle trajectory segmentation;
3. trajectory association and topological continuity;
4. relative fibre-path reconstruction;
5. absolute route geolocation;
6. event classification and probability of detection;
7. fibre-relative event chainage;
8. geographic localization and uncertainty coverage;
9. owner-authorized asset attribution;
10. data lifecycle, replay and retention;
11. investigation decision value and lifecycle economics.

Published trajectory F1, image mIoU, SNR, PSNR/SSIM, interrogator range, channel pitch and one successful field demonstration are different performance measures. No single "DAS SOTA" number is defensible across them.

Useful task-specific 2026 comparators are recorded in:
- `DAS_SOTA_AND_GENERALIZATION_RESEARCH_UPDATE_V0_1.md`
- `DAS_SOTA_TRAJECTORY_AND_NOISE_ADDENDUM_V0_2.md`
- `DAS_INDUSTRY_AND_FIELD_BASELINES_V0_3.md`
- `DAS_COMMERCIAL_SPECIFICATIONS_MATCHED_PARAMETER_UPDATE_V0_2.md`

All published figures remain paper/vendor-reported until independently reproduced.

## 6. Paradigm shifts to pursue

### 6.1 Observability service levels, not nominal sensor reach
Procure/qualify event-class × magnitude × route × profile × time cells. Publish POD, false alarms, blind length/time, localization error, uncertainty, calibration freshness and data rights.

### 6.2 Calibratability and repairability by design
Plan surveyed ties, endpoints, access nodes, as-built revision capture and recalibration after splices/reroutes. No datum tie means no absolute geographic claim.

### 6.3 Active evidence acquisition
Select surveys/tests/inspections by decision value, not information gain alone. If costs and losses cannot be put on a defensible common scale, show Pareto options and do not rank net value.

### 6.4 Preservation-first signal processing
Denoising/compression must preserve critical events, timing, chainage, uncertainty and replay. PSNR/SSIM/SNR are supplementary, not acceptance.

### 6.5 Evidence portability
A utility should be able to independently replay the source, transformations, calibrations and claims without trusting the model vendor's dashboard.

### 6.6 Rights and authority as technical inputs
No fibre access, data acquisition, exact geometry disclosure, work order or asset mutation without the relevant owner authority. Software never creates operational control authority.

## 7. Immediate execution sequence

**P0 — qualify the reference kernel:** resolve the exact-head three-platform test/Clippy/rustfmt result for all five Rust files and the 10,000-point integration test. Preserve failures until corrected; no weakening of gates.

**P1 — offline product slice:** create a replayable source bundle, validate the envelope, import one approved route inventory, apply calibration, produce explicit observability states and export an evidence bundle. No live stream or SCADA connection is required.

**P2 — public research reproduction:** verify rights first, then sample-read one corpus suited to a specific task. Reproduce one paper baseline faithfully; keep marine/campus data separate from terrestrial utility claims.

**P3 — non-live terrestrial testbed:** find a surveyable decommissioned/mock corridor, record fibre/utility offset and independent event/no-event truth, then evaluate one event class and coupling profile.

**P4 — read-only live passive pilot:** only after written utility/telecom ownership rights, cybersecurity review, non-interference plan, change control, and qualified independent survey/truth arrangements.

**P5 — observability-by-design:** apply the design benchmark to new/renewed corridors after brownfield evidence chain is demonstrated.

## 8. Acceptance and claim discipline

The programme does not call a result "SOTA", "field validated", "qualified", "safe", "commercial" or "independent" unless the relevant population, denominators, source identity, truth process, confidence interval, rights, review and exact commit are available.

Keep separate states for:
- implemented;
- structurally checked;
- unit/integration tested;
- exact-head CI passed;
- synthetic;
- public research;
- laboratory;
- field;
- operational;
- independently surveyed;
- owner accepted.

A green CI result is not physical qualification. A source digest is not source authenticity. A calibration is not survey truth. A geographic coordinate is not asset attribution. An alarm is not an engineering diagnosis. An evidence receipt is not proof that physical work was correct.

## 9. Claim ceiling

This programme map documents public research and executable reference components. It does not assert a production gateway, vendor adapter, utility partner, live pilot, field result, cost saving, commercial deployment, engineering approval or autonomous operational authority.
