# Shipyard lift-envelope benchmark protocol V0.1

This protocol separates the lift preflight verifier from downstream crane-selection, rigging-design, trajectory-planning and operational decision systems. It is a research contract, not a claim of superiority.

## 1. The benchmark has four distinct layers

### A. Evidence and preflight correctness

The verifier consumes a source-bound lift-plan input and produces one of:

- `ReadyForHumanReview`: the declared references and bounded arithmetic are internally consistent under the supported profile;
- `Rejected`: a supplied, applicable numeric limit is exceeded;
- `Indeterminate`: a required identity, source, subject, revision, clock, validity interval, bound or supported semantic is missing or inconsistent.

The hard release target is **zero false-ready results** on the frozen negative-control corpus. Missing, stale, revoked, superseded, wrong-subject, wrong-revision, wrong-clock, wrong-chart-cell and under-bounded cases must never become ReadyForHumanReview.

Measure:
- per-case oracle agreement and diagnostic-code agreement;
- false-ready count (hard gate; target 0);
- false-reject and indeterminate counts, separated by cause;
- percentage of cases in the declared supported domain;
- evidence identity/revision/clock invalidation detection;
- crane chart cell mismatch detection;
- source-record conflict detection;
- checked arithmetic/rounding boundary agreement;
- canonical input/result byte determinism under permutation;
- p50/p95/p99 verifier latency and peak memory on a named runner.

Do not optimise the indeterminate rate by guessing missing bounds, assuming equal load sharing, interpolating crane charts or weakening evidence requirements.

### B. Arithmetic and independent exact oracles

Use a second implementation that does not call production arithmetic helpers.

For small integer domains, exhaustively compare:
1. upper-load conversion and dynamic-factor application;
2. ceiling division for every declared numerator/denominator combination;
3. per-leg upper tension calculation;
4. exact limit-boundary behavior (equal-to-limit versus one-unit-over);
5. share-bound coverage (sum below 1,000,000 ppm is indeterminate; sum at/above it is allowed);
6. checked overflow and zero-denominator abstention;
7. input/diagnostic ordering and canonical serialization.

Use rational integer arithmetic as the reference. A floating-point spreadsheet or another copy of the same helper is not an independent oracle.

### C. Downstream hoist sequence and crane-path planning

The current preflight is not a hoisting optimiser. The direct 2026 planning comparator is Qi et al., *Application of Digital Twin-Based Improved A*HNSA Algorithm to Ship Block Hoisting Tasks* (Journal of Shanghai Jiao Tong University, 2026, DOI 10.16183/j.cnki.jsjtu.2025.353). It jointly optimises hoisting sequence and gantry-crane path planning on real Shanghai shipyard scheduling data. The paper reports reductions in total transport distance of 3.51%, 5.33% and 6.79%, and reductions in total scheduling time of 3.82%, 5.35% and 7.07%, across its small, medium and large cases.

These figures are literature anchors only. They are not reproduced by this repository and do not compare directly to the present preflight verifier.

A future fair head-to-head requires:
- the same licensed/source-authorized block geometry and mass/CG inputs;
- identical crane model, chart cells, configuration, rail/gantry constraints, obstacles and initial positions;
- identical pick/drop sequence constraints and handling intervals;
- identical motion, clearance and occupancy semantics;
- identical environment/disruption sequence and compute/time budget;
- per-run raw outputs, seeds, hardware, method revision, configuration and exact digests.

Report separately:
- total hoist distance and travel time;
- crane setup/makespan and idle time;
- number of re-handles and repositionings;
- hard collision/clearance and capacity violations;
- disruption recovery time and schedule churn;
- energy use only where measured or from a separately qualified model;
- feasibility coverage and all timeout/incomplete outcomes.

Do not merge those metrics into a weighted headline score that can hide a hard constraint violation.

### D. Commercial lift-planning and field workflow comparison

Commercial systems already offer substantial lift-planning functions. A1A's 3D Lift Plan product pages describe 3D jobsite modelling, crane selection, capacity monitoring while boom/jib/location/radius change, ground-bearing calculations and rigging configurations with sling-angle/tension calculations.

This prototype does not claim to replace those capabilities. The intended differentiation is a vendor-neutral, machine-readable **evidence and configuration preflight** that binds the exact block revision, source-owned mass/CG/pick-point records, rigging component revision, chart cell, weather observation, operating-limit profile, motion report, occupancy projection, site epoch and validity clock. The system should complement, interoperate with and independently audit the existing engineering workflow.

A commercial-product comparison should therefore use a capability matrix, not invented numeric accuracy claims:

| Capability | 3D Lift Plan public feature description | This V0.1 |
|---|---|---|
| 3D lift/jobsite visualisation | Described | No |
| Crane selection/search | Described | No |
| Live/configuration-dependent capacity monitoring | Described | Exact supplied chart-cell consistency only |
| Ground/outrigger bearing | Described | Unsupported |
| Rigging arrangement design | Described | Unsupported; consumes externally supplied bounds |
| Sling-angle/tension calculation | Described | Narrow upper-bound arithmetic on supplied angle/share limits |
| Evidence subject/revision/clock binding | Not established by public feature description | Explicit contract |
| Motion/occupancy report identity binding | Not established by public feature description | Explicit references; does not recompute path |
| Canonical input and result bytes | Not established by public feature description | Explicit serialization |
| Independent field qualification | Vendor-specific; requires separate evidence | Not established |

Sources:
- https://www.3dliftplan.com/info/Features.aspx
- https://www.3dliftplan.com/Info/Apps.aspx

### E. Standards and applicability

ISO 12480-1:2024 defines a safe system of work involving task planning, crane/equipment selection, erection/dismantling, operation/maintenance and operator/slinger/signaller competence. Its published scope excludes cranes mounted on water-borne vessels, except land-based cranes temporarily affixed to vessels. This V0 is therefore limited to a declared land-based crane profile. Floating/shipboard crane use requires a separate applicability and engineering basis.

ASME applicability must be selected by exact crane arrangement and jurisdiction. B30.2-2022 covers specified top-running bridge and trolley overhead/gantry arrangements; B30.17-2026 covers overhead/monorail systems with underhung trolley or bridge and includes gantry systems in its scope; B30.5-2025 is specific to mobile and locomotive crane families. The V0.1 contract does not yet encode the detailed trolley/bridge subtype needed to resolve the correct volume. A source-owned applicability profile and responsible review are required; no standards conformity is claimed.

Sources:
- https://www.iso.org/standard/83637.html
- https://www.asme.org/codes-standards/find-codes-standards/b30-5-mobile-locomotive-cranes

### Machine-readable contract

The canonical plan and result JSON schemas are:
- `schemas/lift-plan-v0.1.schema.json`
- `schemas/preflight-report-v0.1.schema.json`

The frozen 34-vector suite is `fixtures/conformance-vectors-v0.1.json`, with the envelope defined by `schemas/conformance-vectors-v0.1.schema.json`. Each vector is mapped to a named Rust test; CI checks unique IDs/test names and test-name presence. Evaluation vectors pin expected disposition and required diagnostics; arithmetic/metamorphic/serialization vectors pin invariants rather than a lift disposition.

The vector suite is entirely synthetic and tests the declared software contract only. JSON Schema shape validation does not verify source signatures, OEM chart authenticity, physical measurements or engineering approval.

## 2. Frozen adversarial strata

The preflight corpus should keep distinct strata for:
- source identity and ownership;
- subject ID/revision mismatch;
- document supersession/revocation and validity-window gaps;
- clock-domain mismatch and site epoch changes;
- mass/CG/pick-point evidence gaps;
- dynamic-factor profile changes;
- rigging-component identity/revision and WLL mismatch;
- missing/invalid leg share or angle lower bounds;
- under-covering versus conservatively over-covering load-share maxima;
- exact chart-cell mismatch, wrong quadrant/radius/configuration, missing chart row;
- weather observation stale versus operating-limit profile stale;
- weather observation incorrectly used as the operating limit;
- gust limit exceeded;
- motion report/payload/block/revision mismatch;
- occupancy projection not covering the lift interval;
- source identity collisions and conflicting metadata;
- integer overflow, boundary rounding and one-unit capacity violations;
- input order permutation and canonical byte comparison.

For every scenario, pin exact expected disposition and diagnostic codes. Include mutation controls that deliberately remove the clock match, substitute the wrong chart cell, round demand downward, assume equal sharing, drop one evidence subject, reuse stale evidence or replace the operating limit with a weather reading. The tests must detect each mutation.

## 3. Statistical and reproducibility protocol

The V0.1 verifier is deterministic; do not use repeated stochastic runs to make a deterministic correctness claim. For any future stochastic planner comparison:
- pre-register scenario families, objectives, stop conditions and compute budgets;
- pair methods on identical inputs and disturbance tapes;
- report every run, timeout, failure and incomplete outcome;
- report median, dispersion and paired confidence intervals;
- hold out scenario families and yard configurations;
- bind exact code, toolchain, chart/profile revision, input bytes, source permissions and output digests.

Synthetic tests establish only the declared model contract. Historical replay, shadow operation and sustained yard KPIs are separate evidence tiers. No productivity, safety or cost-saving claim is warranted until rights-reviewed site data and independently checked field outcomes exist.

## 4. Release gates

- **G0 — input contract:** required subject IDs, revisions, units, validity intervals and validity clock are present.
- **G1 — arithmetic:** independent rational oracle agrees on the exhaustive bounded domain.
- **G2 — adversarial evidence:** all frozen negative controls reject or abstain as expected; zero false-ready.
- **G3 — reproducibility:** canonical input and report bytes are deterministic; source or configuration changes change the corresponding input bytes.
- **G4 — integration:** exact motion-envelope and occupancy records are bound; no margin is recomputed or expanded twice.
- **G5 — engineering review:** responsible lift/rigging/crane engineers review model applicability and source-owner semantics.
- **G6 — field evidence:** only a separate, authorised, rights-reviewed programme can establish shadow/field performance. This crate never authorizes a lift.

## 5. Current measured status

No independent CI result, physical lift data, OEM chart dataset or site-calibrated benchmark has yet been admitted for this V0.1 PR. The code has authored unit tests, but exact-head Rust formatting, compilation, tests and Clippy remain pending until the pinned workflow reports success. No SOTA, field-performance, safety or standards-conformance result is claimed.
