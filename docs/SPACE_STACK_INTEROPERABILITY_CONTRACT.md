# Space Stack Interoperability Contract

**Status:** engineering proposal and current code-audit findings; not flight qualification, a safety certification, or a claim that the listed interfaces are already implemented across all repositories.  
**Date:** 13 October 2026  
**Scope:** Symthaea, Mycelix Space, Sol Atlas, Symtropy.

## Goal

Make space-related calculations and autonomous operations composable without silently confusing coordinate frames, times, units, uncertainty models, environment versions, or decision authority. The shared product is a versioned evidence contract and an end-to-end replayable reference campaign, not a single universal physics engine.

## Responsibilities

| Project | Owns | Must not claim by itself |
|---|---|---|
| **Mycelix Space** | Earth-orbit object tracking, SGP4/TLE workflow, conjunction screening, traffic coordination, standardized orbital-message adapters | That a TLE-age heuristic is calibrated covariance, or that Earth-orbit SGP4 is a cislunar transfer propagator |
| **Sol Atlas** | Provenance-bound planetary/lunar data, ROI/candidate geometry identity, illumination/terrain/communications layers, infrastructure configuration | That a source URL is a verified raster, a navigation grid is a candidate perimeter, or an unknown metric is zero |
| **Symtropy** | Rigid-body/contact/robot models, resource-bearing physical simulations, deterministic replay and test scenarios | That a successful simulation is a validated flight model or that consciousness/Φ is a flight-safety case |
| **Symthaea** | Planning, model selection, anomaly detection, estimation/control assistance, experiment orchestration | That a proposed trajectory or controller output can bypass deterministic authority and safety checks |
| **Mycelix evidence services** | Identity, signed observations, provenance receipts, approvals, supersession and coordination across participants | That distributed consensus substitutes for local hard real-time control or independently validated physics |

## Shared records

Initial shared contracts should be versioned, language-neutral schemas with generated or schema-validated Rust/TypeScript representations.

### 1. State estimate

Required fields:

- `schema_version`, stable `object_id`, `central_body_id`, origin and axes/frame identifier.
- Epoch plus explicit time scale/encoding; UTC alone is not a sufficient label for all orbit-dynamics work.
- Position and velocity with unit identifiers and state ordering.
- Source kind: observed, orbit-determined, propagated, fused, empirical estimate, or unknown.
- Covariance/uncertainty representation, units, frame, provenance and model-validity window. An absent covariance must stay absent/unknown, never be silently replaced with a calibrated-looking number.
- Input IDs/digests, model/ephemeris version, software revision and transformation history.

**Invariant:** a state vector and its covariance must describe the same body, epoch and frame, or a verified transform must be applied. Three `f64` values do not establish frame compatibility.

### 2. Spatial data asset

Required fields:

- Publisher/product ID, exact URI, retrieval timestamp, byte count and SHA-256 of local bytes.
- Retrieval outcome and content type; a checksum establishes byte identity, not publisher authenticity.
- Coordinate reference, datum/body-fixed convention, axes, resolution, units, pixel registration, nodata/scale/offset and valid spatial/temporal extent.
- ROI/polygon identity and digest; explicit distinction between candidate boundary, navigation grid, raster footprint and derived mask.
- Verification states separating source-reported, downloaded/hash-captured, label-parsed, coordinate-validated, semantically accepted, and calculation-ready.

### 3. Plan / action proposal

Required fields:

- Proposed command/trajectory/task, target asset and time window.
- Preconditions, hard constraints, reserve floors, allowed authority and expiry.
- Input/environment evidence digests and the planner/model version.
- Independent gate decision and reason codes.
- After execution: observed effect, actual telemetry/evidence, deviations, and reconciliation state.

A proposed plan is not an authorized command. A queued test is not a pass. An accepted simulation is not hardware qualification.

## Mathematical and frame invariants

1. **Body-aware dynamics.** Keep terrestrial SGP4/TLE propagation distinct from a cislunar numerical-propagation module. The latter needs declared celestial ephemerides, time scale, force model, integrator, event logic and validated comparison cases.
2. **Covariance-frame equality.** Mycelix Space's inspected `Propagator::propagate_to` returns a state tagged `TEME`, while `CovarianceMatrix::from_tle_age` defaults its covariance to `ECI`. Before that covariance is used in a conjunction or transformation, align the frame or transform the matrix and validate the transform. The present TLE-age model is described in source as very approximate; tag it `Estimated` and do not use it as a calibrated operational collision probability without validation.
3. **Physical covariance checks.** Validate finite entries, symmetry, positive-semidefiniteness, frame, units, epoch and transformation provenance. Reject every negative diagonal variance independently of numerical tolerance. Use pair-relative symmetry checks and a dimensionless, per-axis correlation-normalized PSD check with a documented floating-point tolerance; a matrix-wide absolute eigenvalue floor can hide invalidity in a lower-magnitude state block. Zero-variance axes must have zero covariance with every other axis. These are structural checks, not covariance-realism calibration. To change frames, transform covariance with the appropriate state-transition/Jacobian matrix, not by relabeling it. Do not silently clip or repair covariance during source admission. If a downstream probability calculation later needs a numerical repair, preserve the original matrix, emit a separately identified derived artifact and repair receipt, and qualify the repair only at the exact projection required for that calculation; NASA CARA's handbook recommends narrowly scoped conditioning rather than indiscriminate whole-matrix repair.
4. **Numerical loop preconditions.** Time steps must be finite and strictly positive; iteration counts and horizons must be bounded; failed propagation must remain explicit in output.
5. **Optimization separation.** The optimizer returns candidate solutions plus feasibility/residual evidence. The independent checker re-evaluates hard constraints, resource margins and state validity. Solver success alone does not prove mission feasibility.
6. **Lunar GIS.** Honor the selected lunar projection/datum and label semantics; use LTM/LPS/LGRS/ACC or another source-specific lunar CRS where appropriate. Never assume terrestrial WGS84/UTM or substitute LROC DTM footprints for candidate polygons without validation.
7. **Model identity.** Every run binds source-data digests, configuration, random seeds, force/contact model, numerical tolerances, solver build and outputs. Replay results must state whether determinism is bitwise, tolerance-bounded, or merely statistically comparable.
8. **Orthonormal maneuver basis.** Define the radial axis as \(\hat r=r/\|r\|\), the orbit normal as \(\hat h=(r\times v)/\|r\times v\|\), and the prograde transverse axis as \(\hat t=\hat h\times\hat r\). Using \(v/\|v\|\) directly as prograde is incorrect whenever radial velocity is nonzero. Reject degenerate geometry; do not silently substitute global coordinate axes. The active Mycelix follow-up [PR #4992](https://github.com/Luminous-Dynamics/mycelix/pull/4992) implements this shared checked basis for TLE and fused-state avoidance paths and adds a non-circular-orbit regression test. This is a geometry correction, not validation of the maneuver solver or flight authority.

## Interfaces to align with established standards

Use applicable standards as interoperability targets rather than inventing replacements:

- **CCSDS 502.0-B-3, Orbit Data Messages (May 2023), with Technical Corrigendum 1 (issued June 2026 and listed by CCSDS on 2 October 2026):** implement OPM, OMM, OEM and OCM adapters against the corrected publication. Preserve message identity, central body, reference frame, time system, covariance/metadata semantics and registered keyword constraints. Keep any internal richer schema; use corrected examples in conformance tests and do not claim interoperability until parser/serializer tests pass.
- **CCSDS Conjunction Data Message:** exchange conjunction events and covariance/metadata using the applicable controlled version, preserving frame, epoch and uncertainty semantics.
- **IETF RFC 9171, Bundle Protocol Version 7 (BPv7), as updated by RFC 9713 and RFC 9758:** use the current RFC set for store-carry-forward networking over intermittent, disrupted or long-delay links. RFC 9713 updates the administrative-record registry rules; RFC 9758 updates `ipn` endpoint-identifier syntax/encoding and registries. Keep the Symthaea telemetry queue, encrypted application envelope and durable receiver inbox distinct from a BPv7 implementation or CCSDS space-link profile; transport conformance needs its own bundle lifecycle, endpoint identity/URI tests, lifetime/expiry, integrity and interoperability tests. ([RFC 9171](https://www.rfc-editor.org/rfc/rfc9171.html), [RFC 9713](https://www.rfc-editor.org/rfc/rfc9713.html), [RFC 9758](https://www.rfc-editor.org/rfc/rfc9758.html)).
- **NASA / ESA / JAXA LunaNet Interoperability Specification v5 (published 7 February 2025):** align lunar network, communications, navigation and timing service concepts, message identity and availability assumptions.
- **USGS lunar grid releases:** enforce source-specific LTM/LPS/LGRS/ACC projection and lunar datum metadata; the named grid spacing is not a terrain-accuracy claim.
- **NASA-HDBK-7009B implementation guidance (3 February 2026):** maintain documented model credibility, verification/validation plans, uncertainty characterization and intended-use boundaries.
- **NASA software assurance/safety guidance:** define safety requirements, independent verification, hazard traceability and lifecycle evidence before a physical deployment claim.
- **NASA CARA Conjunction Assessment Handbook / Volume 2 appendices:** NASA's official CA page advertises an updated “Calculation Approaches” appendix from August 2026. Use that current appendix alongside the base handbook when extending collision-probability reference cases, and record the exact source version and test case identities rather than relying on an older downloaded PDF alone: https://www.nasa.gov/cara/.

These sources guide the design; this proposal does not claim CCSDS or NASA compliance.

The existing [Symthaea satellite telemetry research PR #7425](https://github.com/Luminous-Dynamics/symthaea/pull/7425) documents RFC 9171 as transport context and explicitly says its in-memory/SQLite queues are application-layer policy components, not BPv7. Preserve that boundary in integration work: authenticated telemetry envelopes and durable ACK/replay behavior are useful components, but do not claim BPv7, spacecraft radio/link interoperability, or operational privacy guarantees without independent conformance and deployment evidence.

## Reference campaign

Build one simulation campaign that uses all four projects:

1. Sol Atlas selects an explicitly versioned lunar environment bundle with accepted terrain/illumination/communications layers and visible unknowns.
2. Mycelix Space supplies a separately validated trajectory or transport segment and navigation-message import/export.
3. Symtropy simulates payload deployment, rover movement/contact, power/thermal/relay-resource constraints and injected faults.
4. Symthaea proposes a schedule/route and replans under uncertain estimates, but cannot override hard gates.
5. Mycelix records immutable run inputs, model/config hashes, approvals, telemetry, and a machine-verifiable conclusion.
6. A separate verifier replays the scenario and evaluates constraints from raw inputs independently of the planner.

Fault cases: stale ephemeris, frame mismatch, missing terrain tile, malformed covariance, interrupted relay, reduced power, actuator non-finiteness, expired authorization, failed deployment, and recovery-path loss.

## P1 reference lane: conjunction-probability numerics (Earth orbit only)

Mycelix issue [#4973](https://github.com/Luminous-Dynamics/mycelix/issues/4973) tracks independent validation of the current method and its heuristic uncertainty range. Use the public NASA CARA Analysis Tools source as a pinned comparison target, not as a claim of operational approval:

- Upstream baseline: [nasa/CARA_Analysis_Tools](https://github.com/nasa/CARA_Analysis_Tools/commit/ce0f00722c56808755ee0adbe47a4e3d56439484), commit `ce0f00722c56808755ee0adbe47a4e3d56439484` (2026-10-05).
- 2-D Foster implementation: [`Pc2D_Foster.m`](https://github.com/nasa/CARA_Analysis_Tools/blob/ce0f00722c56808755ee0adbe47a4e3d56439484/DistributedMatlab/ProbabilityOfCollision/Pc2D_Foster.m), Git blob `84dbc44bf6276c1cf66343a680ac7e3b87a5f528`.
- 3-D Hall implementation: [`Pc3D_Hall.m`](https://github.com/nasa/CARA_Analysis_Tools/blob/ce0f00722c56808755ee0adbe47a4e3d56439484/DistributedMatlab/ProbabilityOfCollision/Pc3D_Hall.m), Git blob `6bf7d84bc7506f3a440b59368d1de58c7fe99d6a`.

The pinned 2-D upstream assertions are recorded as JSON fixtures in [Mycelix PR #4999](https://github.com/Luminous-Dynamics/mycelix/pull/4999), stacked on the checked-conjunction work. That PR now includes a separately named composite-Simpson integration test of the Foster 2-D Gaussian area integral against all three upstream assertions; this is an independent reference-oracle check, **not** a comparison of Mycelix's production `GaussianSmallDiskApprox2D` output against Foster. A separate numerical reproduction falls within the upstream tolerance, but exact-head Rust CI remains queued and unverified. Keep issue #4973 open until the test lane passes, production-vs-reference behavior is measured in explicitly compatible cases, and the method limitations are independently reviewed.

Keep these as **separate methods**. Do not use an output from one as the expected answer for the other, and do not apply an Earth-orbit reference case to cislunar dynamics without a separately justified model. For each frozen case, retain the exact input bytes and hashes, both object states and epochs, realized reference-frame convention, covariance matrix values/units, hard-body radius, geometry selector, integration tolerances, upstream commit and file blob, toolchain/version, output Pc, and any validity/remediation flags. Tests must retain remediation status and projected encounter-plane covariance alongside the scalar result; a matching scalar alone can hide differences in covariance conditioning.

Required cases should cover at least: a well-conditioned isotropic baseline with an independent closed-form check; anisotropic covariance; low and high miss-distance regimes; changed hard-body radius; cases near documented method limits; and invalid/degenerate encounter geometry that must produce an explicit refusal rather than a numeric risk. Use an independent implementation or Monte Carlo check where appropriate, with documented tolerances and sampling uncertainty. Do not label `pc/10` or `min(pc*10, 1)` as confidence bounds or conservative guarantees; keep them explicitly heuristic unless a validated uncertainty method replaces them.

The NASA SDK uses MATLAB and its README notes that data files include Git LFS-managed `.mat` assets. Before automation or redistribution, pin the MATLAB/toolbox environment, ensure required LFS data are materialized, and review the applicable NASA Open Source Software Agreement. This contract records comparison targets, not vendoring permission or an observed numerical match.

The next differential-test increment is draft [Mycelix PR #5006](https://github.com/Luminous-Dynamics/mycelix/pull/5006). It adds four synthetic cases with non-zero encounter-plane covariance cross terms and passes them through the public `ConjunctionAnalyzer::try_assess` path, comparing `GaussianSmallDiskApprox2D` with a separately implemented Gauss-Legendre integral over a circular Gaussian region. The independent reference calculation reproduced all four frozen values to floating-point precision across radial quadrature orders 60, 100, 160 and 220. The checked Rust path and the new CI replay are authored but the exact PR head was still queued when observed; do not call this verified parity yet. These synthetic fixtures extend the selected Orekit scalar cases and NASA CARA reference work, but establish neither covariance realism nor operational Pc qualification.

**Interpretation boundary:** passing these fixtures demonstrates reproducible parity for the pinned cases only. It does not establish covariance realism, source authenticity, forecast calibration, operational risk qualification, or maneuver authority. Maintain the distinct labels for method identity, model validity, numerical verification, calibration, and operational decision.

## Priority order

**P0 — Contract integrity:** explicit frame/body/time/unit identifiers; reject incompatible inputs; fail closed on non-finite controller inputs/telemetry; bounded propagation APIs.

**P1 — Independent numerical references:** compare Mycelix Space outputs with controlled CCSDS/NAIF/SPICE or other trusted reference cases appropriate to each regime; verify state and covariance transforms separately.

**P2 — Environment provenance:** make Sol Atlas spatial-asset states machine-enforced and publish immutable environment bundles that downstream simulators can consume.

**P3 — Replayable physical scenario:** select one low-risk reference campaign and test baseline planner versus Symthaea-assisted planner across seeded fault suites.

**P4 — Mission optimization:** multiobjective transport and logistics over \(\Delta v\), delivery window, energy, communications availability, uncertainty margins, maintenance and recovery cost. Preserve the feasible/Pareto set instead of hiding all trade-offs behind a weighted winner.

## Exit criteria for the first integrated milestone

- Incompatible bodies/frames/units/time scales are rejected by contract tests.
- Covariance frame and validity are checked before conjunction probability is computed.
- Non-finite or failed controller/bridge inputs produce zero authority and a recorded reason.
- The reference campaign replays from content-addressed inputs and produces the same declared result within its stated reproducibility class.
- Fault injection demonstrates safe-state behavior and recovery feasibility; no safety-critical constraint failure is masked by a scalar mission score.
- Every claim is labelled as source-reported, computed, simulated, formally checked, independently reproduced, or unknown.

## Source references

- CCSDS Blue Book 502.0-B-3, Orbit Data Messages: https://ccsds.org/Pubs/502x0b3e1.pdf
- CCSDS active Blue Book catalogue: https://ccsds.org/publications/bluebooks/
- NASA LunaNet Interoperability Specification v5: https://www.nasa.gov/directorates/somd/space-communications-navigation-program/lunanet-interoperability-specification/
- NASA LunaNet overview and lunar services: https://www.nasa.gov/communicating-with-missions/lunanet/
- USGS down-selected Artemis III navigational grids: https://www.usgs.gov/data/down-selected-artemis-iii-candidate-landing-site-navigational-grids
- NASA-HDBK-7009B, Model and Simulation Implementation Guide (active, 3 February 2026): https://standards.nasa.gov/standard/nasa/nasa-hdbk-7009
- NASA Software Assurance and Software Safety: https://sma.nasa.gov/sma-disciplines/software-assurance-and-software-safety

## Current repository and implementation status (12 October 2026)

The standalone `Luminous-Dynamics/mycelix-space` repository was archived on 6 March 2026 and is historical/read-only. The implementation was migrated into the active `Luminous-Dynamics/mycelix` monorepo:

- Space hApp / zomes: `mycelix-workspace/mycelix-space/`
- Shared standalone native orbital-mechanics crate: `mycelix-workspace/crates/orbital-mechanics/`
- The orbital crate is intentionally excluded from the root Cargo workspace, so it needs its own explicit CI/test lane; it must not be assumed covered by `cargo test --workspace`.

Follow-up on 13 October 2026: [Mycelix PR #4986](https://github.com/Luminous-Dynamics/mycelix/pull/4986) adds an explicit negative-diagonal covariance rejection and a regression test for `-1e-11`, a value that fell inside the previous absolute eigenvalue tolerance. That PR is a draft and its exact-head workflow is queued, so this is a proposed hardening change—not a verified CI pass.

The active Mycelix follow-up [PR #4986](https://github.com/Luminous-Dynamics/mycelix/pull/4986) now rejects negative diagonal variances and adds a regression case for an indefinite covariance whose negative eigenvalue was previously hidden inside the absolute tolerance. Its latest proposed increment validates PSD after per-axis normalization; the current-head Space workflow must pass before this is treated as verified.

The same frame mismatch and non-progressing ephemeris-step behavior were confirmed in the migrated copy. Draft [Mycelix PR #4969](https://github.com/Luminous-Dynamics/mycelix/pull/4969) proposes representation/input-safety fixes and explicit CI for the standalone crate. Stacked [PR #4972](https://github.com/Luminous-Dynamics/mycelix/pull/4972) adds checked conjunction assessment across the active daemon/coordinator paths: matching epochs/state frames, finite state values, structurally plausible covariance-source metadata, positive-definite covariance, valid hard-body radius, and defined encounter geometry are required. The checked API currently accepts the explicit TEME convention emitted by the TLE propagator and rejects the underspecified generic ECI label until frame realization/origin/orientation identity is carried by the state contract. The coordinator now requires stored/manual Pc values and Pc-only bands to agree; alerts request qualified review and do not issue maneuver commands.

A separate reference check found that the current Pc formula is a Gaussian local-density/small-disk approximation, not the published Alfano/Foster integral; its `pc/10` and `pc*10` spread values are heuristic multipliers, not statistically calibrated confidence bounds. PR #4972 corrects the emitted method label to `GaussianSmallDiskApprox2D`, preserves the historical `Alfano2D` label only for old serialized records, and marks stored alerts with no recorded method as `Unspecified`. The checked API rejects invalid projected encounter covariance before its numerical kernel can fall back to zero Pc, enforces an empirical small-disk eligibility envelope, and refuses generic ECI without qualified frame identity. The coordinator also rejects mismatches between stored/manual Pc values and their Pc-only risk band, while alert recommendations request qualified review rather than maneuver execution.

The benchmark report [`PC_REFERENCE_BENCHMARKS.md`](https://github.com/Luminous-Dynamics/mycelix/blob/fix/space-checked-conjunction-assessment-2026-10-12/mycelix-workspace/crates/orbital-mechanics/PC_REFERENCE_BENCHMARKS.md) and its regression fixture compare the local-density formula with 13 scalar values from Orekit's published `Alfano2005Test`; all selected values are within 1% relative difference, while another published case exhibits ~84.4% relative error and is rejected by the eligibility gate. This remains selected-case evidence, not a global error bound or flight qualification. Follow-up expansion to NASA CARA/Foster/Elrod cases and covariance-realism validation is tracked in [Mycelix issue #4973](https://github.com/Luminous-Dynamics/mycelix/issues/4973).

Next research increment: include the currently advertised August 2026 NASA CARA Calculation Approaches appendix in the source review for [issue #4973](https://github.com/Luminous-Dynamics/mycelix/issues/4973). Preserve exact case IDs, input covariances, published expected values and eligibility/exclusion outcomes. A local-density approximation passing selected historical cases must not be described as globally equivalent to a reference implementation.

Research references: [NASA CARA publicly available software](https://www.nasa.gov/cara/publicly-available-cara-software/), [NASA CARA R&D](https://www.nasa.gov/cara/research-and-development/), [NASA CARA publications](https://www.nasa.gov/cara/cara-publications/), and [Orekit's documented Alfano 2005 implementation](https://www.orekit.org/site-orekit-13.1.3/apidocs/org/orekit/ssa/collision/shorttermencounter/probability/twod/ShortTermEncounter2DPOCMethodType.html). These are independent reference targets; the current code has not been shown numerically equivalent to them.

**Status boundary (13 October 2026):** Mycelix PRs #4969, #4972, #4999 and #5006 remain draft/proposed work. PR #5006's latest observed head was `4d4811c7cd60a5c81ef1c41a2481a2dab97377a5`; its Space orbital-mechanics and D6S canonical-qualification workflows were queued (runs [38441947840](https://github.com/Luminous-Dynamics/mycelix/actions/runs/38441947840) and [38441947676](https://github.com/Luminous-Dynamics/mycelix/actions/runs/38441947676)). PR #4999's exact-head Rust/D6S jobs also remained queued at the latest observation. Symtropy PR #1576's observed head `65dbe8f84eb9561f7094c8a74fa8a25bf64836ee` had macOS and Windows jobs passed and Ubuntu queued; its cross-platform matrix is incomplete. No unobserved or queued check is a pass. These proposed changes establish neither calibrated covariance nor validated operational collision probability, cislunar propagation, real-time flight-safety authority, or flight qualification. No complete flight-dynamics campaign was run.


## Evidence update — 14 October 2026

Mycelix's correlated collision-probability campaign has an additional independent reference route authored in draft [PR #5006](https://github.com/Luminous-Dynamics/mycelix/pull/5006): [conditional-normal replay](https://github.com/Luminous-Dynamics/mycelix/blob/test/space-pc-correlated-end-to-end-2026-10-13/mycelix-workspace/crates/orbital-mechanics/tools/verify_pc_correlated_gaussian_conditional_reference.py). It integrates the x marginal density times the conditional y probability over each disk cross-section using one-dimensional Gauss–Legendre quadrature at orders 500 and 700. This is structurally distinct from the existing tensor-product polar-area quadrature. Both replay scripts are now wired into the Mycelix Space orbital-mechanics workflow.

The latest observed Mycelix head at this update is `a6468952f76a431f36d1717f4d646aba5e319583`; [workflow run 38502816010](https://github.com/Luminous-Dynamics/mycelix/actions/runs/38502816010) is **pending** with no conclusion observed. The previous exact-head run's NumPy polar-reference step passed, but the Rust jobs stopped at formatting checks; the D6S workflow also failed because it requested integration-test targets absent from its package. The current draft adds the rustfmt output across the affected files, adds the second numerical replay, and scopes D6S qualification to changes in its own corpus inputs. Those repairs are proposed, not verified until the new exact-head checks complete. In particular, no Rust end-to-end pass, Foster/Alfano parity, calibrated collision probability, covariance realism, or operational qualification is claimed.

The new replay's (5\times10^{-9}) relative thresholds apply only to these four frozen synthetic cases and are convergence/reproduction checks, not a general approximation-error bound. A successful campaign still establishes only numerical agreement for the pinned cases; data quality, covariance provenance/realism, method eligibility, uncertainty calibration, and maneuver authority remain independent qualification layers.


## Correlated Pc campaign update — 14 October 2026 (latest observed evidence)

Mycelix draft [PR #5006](https://github.com/Luminous-Dynamics/mycelix/pull/5006) now retains four correlated-covariance synthetic cases and adds a fifth centered-isotropic analytic control. For the control, with σ = 0.2 km and hard-body radius R = 0.01 km, the exact Gaussian disk mass is `1 - exp(-R^2/(2*sigma^2)) = 0.0012492190754190835`; the small-disk local-density approximation is 0.00125. Both independent replays (polar-area tensor-product quadrature and one-dimensional conditional-normal CDF quadrature) check the frozen control against this closed form, and the Rust end-to-end test runs the case through the public checked assessment path. This is an oracle sanity check, not Foster/Alfano parity or a global error bound.

The latest observed Mycelix source head is `13fbd5043aa3b916169f6f14906f4cad2273ce57`. [Mycelix run 38503897952](https://github.com/Luminous-Dynamics/mycelix/actions/runs/38503897952) is in progress; its numerical and Rust results on that exact head are not yet known. Prior head `5e5bfd9231eb04d58531aed40eb605473bd84a90` passed the standalone orbital-mechanics test job and both NumPy oracle checks, plus shared Space and coordinator screening tests, but its WASM step failed on getrandom backend selection. The workflow now specifies Holochain's custom backend and uses `cargo build --release`; that modified release-linking gate has not yet been verified. Do not promote prior-head pass evidence to the current head.

Operational method applicability, covariance source authenticity/realism, uncertainty calibration, independent method equivalence and maneuver authority remain separate gates. Keep Mycelix issue [#4973](https://github.com/Luminous-Dynamics/mycelix/issues/4973) open and do not interpret synthetic fixture success as operational collision-risk qualification.


## Coordinator/integrity WASM boundary follow-up — 14 October 2026

A real `cargo build --release --target wasm32-unknown-unknown` check on Mycelix exposed that `mycelix-space-shared` included `gate_space_operation()`, which calls coordinator-only HDK host functions while the same shared crate is linked into an integrity zome. The build failed on unresolved host imports when producing `conjunctions_integrity.wasm`; the prior `cargo check` step did not link the artifact and therefore did not expose this boundary defect. Draft Mycelix PR [#5006](https://github.com/Luminous-Dynamics/mycelix/pull/5006) now proposes moving that existing gate into a coordinator-only `mycelix-space-coordinator-gates` crate, removing HDK from the shared types crate, and updating all five coordinator zomes to import the gate separately. Its behavior is intended to remain unchanged, and no integrity validation is weakened.

Latest proposed Mycelix head: `4957489a63873d4f2b196a3d254508633d060132`; [run 38504675549](https://github.com/Luminous-Dynamics/mycelix/actions/runs/38504675549) has passed the standalone orbital-mechanics job on that exact head (both numerical replay scripts, formatting, and Rust tests); the Space/coordinator job remains in progress at shared-contract tests, with the release WASM link still pending. The host-import fix is **not yet verified** until formatting, Rust tests, and linked coordinator/integrity WASM build all pass on the same exact head. The standalone tests and analytic control remain numerical software checks only—not Foster/Alfano parity, covariance realism, calibrated Pc, or maneuver authority.
