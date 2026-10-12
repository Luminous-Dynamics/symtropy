# Space Stack Interoperability Contract

**Status:** engineering proposal and current code-audit findings; not flight qualification, a safety certification, or a claim that the listed interfaces are already implemented across all repositories.  
**Date:** 12 October 2026  
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
3. **Physical covariance checks.** Validate finite entries, symmetry, positive-semidefiniteness within a stated numerical tolerance, frame, units, epoch and transformation provenance. To change frames, transform covariance with the appropriate state-transition/Jacobian matrix, not by relabeling it.
4. **Numerical loop preconditions.** Time steps must be finite and strictly positive; iteration counts and horizons must be bounded; failed propagation must remain explicit in output.
5. **Optimization separation.** The optimizer returns candidate solutions plus feasibility/residual evidence. The independent checker re-evaluates hard constraints, resource margins and state validity. Solver success alone does not prove mission feasibility.
6. **Lunar GIS.** Honor the selected lunar projection/datum and label semantics; use LTM/LPS/LGRS/ACC or another source-specific lunar CRS where appropriate. Never assume terrestrial WGS84/UTM or substitute LROC DTM footprints for candidate polygons without validation.
7. **Model identity.** Every run binds source-data digests, configuration, random seeds, force/contact model, numerical tolerances, solver build and outputs. Replay results must state whether determinism is bitwise, tolerance-bounded, or merely statistically comparable.

## Interfaces to align with established standards

Use applicable standards as interoperability targets rather than inventing replacements:

- **CCSDS 502.0-B-3, Orbit Data Messages (May 2023):** OPM, OMM, OEM and OCM formats for orbital data exchange. Keep any internal richer schema and provide explicit standards-aware import/export adapters.
- **CCSDS Conjunction Data Message:** exchange conjunction events and covariance/metadata using the applicable controlled version, preserving frame, epoch and uncertainty semantics.
- **NASA / ESA / JAXA LunaNet Interoperability Specification v5 (published 7 February 2025):** align lunar network, communications, navigation and timing service concepts, message identity and availability assumptions.
- **USGS lunar grid releases:** enforce source-specific LTM/LPS/LGRS/ACC projection and lunar datum metadata; the named grid spacing is not a terrain-accuracy claim.
- **NASA-HDBK-7009B implementation guidance (3 February 2026):** maintain documented model credibility, verification/validation plans, uncertainty characterization and intended-use boundaries.
- **NASA software assurance/safety guidance:** define safety requirements, independent verification, hazard traceability and lifecycle evidence before a physical deployment claim.

These sources guide the design; this proposal does not claim CCSDS or NASA compliance.

## Reference campaign

Build one simulation campaign that uses all four projects:

1. Sol Atlas selects an explicitly versioned lunar environment bundle with accepted terrain/illumination/communications layers and visible unknowns.
2. Mycelix Space supplies a separately validated trajectory or transport segment and navigation-message import/export.
3. Symtropy simulates payload deployment, rover movement/contact, power/thermal/relay-resource constraints and injected faults.
4. Symthaea proposes a schedule/route and replans under uncertain estimates, but cannot override hard gates.
5. Mycelix records immutable run inputs, model/config hashes, approvals, telemetry, and a machine-verifiable conclusion.
6. A separate verifier replays the scenario and evaluates constraints from raw inputs independently of the planner.

Fault cases: stale ephemeris, frame mismatch, missing terrain tile, malformed covariance, interrupted relay, reduced power, actuator non-finiteness, expired authorization, failed deployment, and recovery-path loss.

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

The same frame mismatch and non-progressing ephemeris-step behavior were confirmed in the migrated copy. Draft [Mycelix PR #4969](https://github.com/Luminous-Dynamics/mycelix/pull/4969) proposes representation/input-safety fixes and explicit CI for the standalone crate. Stacked [PR #4972](https://github.com/Luminous-Dynamics/mycelix/pull/4972) adds checked conjunction assessment across the active daemon/coordinator paths: matching epochs/state frames, finite state values, structurally plausible covariance-source metadata, positive-definite covariance, valid hard-body radius, and defined encounter geometry are required. The checked API currently accepts the explicit TEME convention emitted by the TLE propagator and rejects the underspecified generic ECI label until frame realization/origin/orientation identity is carried by the state contract. The coordinator now requires stored/manual Pc values and Pc-only bands to agree; alerts request qualified review and do not issue maneuver commands.

A separate reference check found that the current Pc formula is a Gaussian local-density/small-disk approximation, not the published Alfano/Foster integral; its `pc/10` and `pc*10` spread values are heuristic multipliers, not statistically calibrated confidence bounds. PR #4972 corrects the emitted method label to `GaussianSmallDiskApprox2D`, preserves the historical `Alfano2D` label only for old serialized records, and marks stored alerts with no recorded method as `Unspecified`. The checked API rejects invalid projected encounter covariance before its numerical kernel can fall back to zero Pc, enforces an empirical small-disk eligibility envelope, and refuses generic ECI without qualified frame identity. The coordinator also rejects mismatches between stored/manual Pc values and their Pc-only risk band, while alert recommendations request qualified review rather than maneuver execution.

The benchmark report [`PC_REFERENCE_BENCHMARKS.md`](https://github.com/Luminous-Dynamics/mycelix/blob/fix/space-checked-conjunction-assessment-2026-10-12/mycelix-workspace/crates/orbital-mechanics/PC_REFERENCE_BENCHMARKS.md) and its regression fixture compare the local-density formula with 13 scalar values from Orekit's published `Alfano2005Test`; all selected values are within 1% relative difference, while another published case exhibits ~84.4% relative error and is rejected by the eligibility gate. This remains selected-case evidence, not a global error bound or flight qualification. Follow-up expansion to NASA CARA/Foster/Elrod cases and covariance-realism validation is tracked in [Mycelix issue #4973](https://github.com/Luminous-Dynamics/mycelix/issues/4973).

Research references: [NASA CARA publicly available software](https://www.nasa.gov/cara/publicly-available-cara-software/), [NASA CARA R&D](https://www.nasa.gov/cara/research-and-development/), [NASA CARA publications](https://www.nasa.gov/cara/cara-publications/), and [Orekit's documented Alfano 2005 implementation](https://www.orekit.org/site-orekit-13.1.3/apidocs/org/orekit/ssa/collision/shorttermencounter/probability/twod/ShortTermEncounter2DPOCMethodType.html). These are independent reference targets; the current code has not been shown numerically equivalent to them.

**Status boundary:** PRs #4969 and #4972 are draft, proposed changes. The current Mycelix head's dedicated orbital/Space workflow remained queued at the latest observation; no current-head pass is claimed. These changes establish neither calibrated covariance nor validated collision probability, cislunar propagation, real-time flight-safety authority, or flight qualification. This research pass did not run a complete flight-dynamics campaign.
