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

The same frame mismatch and non-progressing ephemeris-step behavior were confirmed in this migrated copy. Draft [Mycelix PR #4969](https://github.com/Luminous-Dynamics/mycelix/pull/4969) now targets the active monorepo: it labels the TLE-age covariance heuristic as TEME to match SGP4 output, rejects zero/negative ephemeris steps, refuses mixed covariance frames in Mahalanobis distance, rejects non-finite covariance matrices, documents the current RTN method's rotation-only limitation, and adds a standalone-crate CI workflow.

**Status boundary:** PR #4969's targeted CI run was queued at the latest observation; no current-head pass is claimed. The code changes are proposed, not yet merged. They repair representation/input-safety issues only. They do not calibrate the TLE-age covariance model, validate the conjunction-probability algorithm, establish cislunar propagation, or provide flight qualification. This research pass did not run a complete flight-dynamics campaign.
