# Evidence-gated inter-pass adaptive planner v0.1

Tracking: [SHIPYARD-INTERPASS-ADAPTIVE-001 #4649](https://github.com/Luminous-Dynamics/symtropy/issues/4649)

## Why this is not just another adaptive path planner

Recent adjacent SOTA already includes:
- Yu et al. (2026): 4,434 beads from 270 specimens; inter-pass 3D profiling; residual cross-sectional volume reallocation; nine additional validation specimens with reported UT acceptance.
- Wang et al. (October 2026): online synchronous welding path and torch-posture planning, reporting path RMSE ≤0.1334 mm and torch-posture RMSE ≤0.8064° across its stated test context.
- 2026 closed-loop robotic welding survey: the forward perception→modelling→decision→execution path is more mature than the backward assessment→model/decision learning loop, and calls for comparable multi-layer evaluation with clear system maturity.

The opportunity is not to claim we are better at point-cloud path fitting or to introduce RL by default. The differentiator is a machine-checkable, evidence-governed recommendation contract: exact predecessor scan binding, freshness, registration/coverage/occlusion limits, uncertainty-aware Pareto output, procedure-scope checks, resource/reachability checks, and explicit abstention. No action is automatically selected; no robot is commanded.

## V0 implemented scope

A dependency-free Rust library accepts only **synthetic contract observations**. It rejects measured or simulated evidence until separate source-rights, measurement, model and procedure qualification adapters exist. This is intentional: the BAM raw archive is still rights-pending.

For a scan-derived residual-fill estimate R with uncertainty U, each candidate pass deposition D is scored using the worst error over the interval [max(0,R-U), R+U]:

error(D) = max(|max(0,R-U)-D|, |R+U-D|)

The planner rejects a zero uncertainty bound and preserves five separate lower-is-better objectives:
1. worst-case residual geometry error;
2. duration;
3. material;
4. energy;
5. robustness risk.

It returns the nondominated Pareto set, not a weighted sum and not one automatically selected pass. Candidates must bind to the exact WID, joint revision, layer, predecessor pass, scan ID, SHA-256 of scan bytes, SHA-256 of the registration/transform receipt, and procedure revision. In v0.1 these 32-byte values are synthetic contract identifiers, not hashes of the unavailable BAM raw files; a future measured adapter must calculate and verify them from authorized bytes and a reproducible transform chain. Infeasible candidates are excluded with reasons. A stale/unregistered/low-coverage/high-occlusion/out-of-domain/future scan causes abstention.

All numeric quantities in v0 are **symbolic synthetic units**, not mm², deposited volume, joules, physical distortion or actual procedure limits. A procedure scope labelled synthetic is not a WPS/PQR/WPQT qualification.

## Files and verification

- `crates/tools/shipyard-interpass-adaptive`: dependency-free Rust 2024 library.
- `docs/tech/SHIPYARD_INTERPASS_ADAPTIVE_V0_1.schema.json`: Draft 2020-12 input contract.
- `docs/tech/SHIPYARD_INTERPASS_ADAPTIVE_V0_1.md`: benchmark and claim-boundary protocol.
- Focused CI runs rustfmt, tests and Clippy with warnings denied.
- 29 Rust unit/mutation tests cover Pareto preservation, uncertainty, stale scan, wrong predecessor, low coverage, occlusion, registration error, future observation, stale observation, future leakage, OOD, measured-data rejection, live execution, procedure scope/revision, wrong scan, collision, resource infeasibility, duplicate candidates, no-feasible-plan and numeric overflow.

This crate is an isolated nested Cargo workspace so it does not add another competing root Cargo.toml/Cargo.lock edit while the other three shipyard draft PRs are outstanding. It has no external crate dependencies.

## Benchmark ladder

1. Synthetic exact contract tests and Pareto known answers.
2. BAM geometry replay only after archive rights are cleared; keep five-WID limitations explicit.
3. Matched comparison against fixed teach-and-playback and geometry-only residual-area reallocation.
4. Multimodal process+robot+geometry fusion, with equal observation prefix and WID-grouped holdouts.
5. Prospective physical validation with qualified procedures, calibrated metrology and independent NDT disposition.

## Claim ceiling

A v0 PASS proves only that the synthetic input and candidate set satisfy the declared recommendation contract. It does not physically predict bead shape, qualify a process, establish weld quality, accept an NDT result or authorize robot execution.


## Matched baselines and receding-horizon known answers — 2026-11-11

The synthetic fixture includes two intentionally different single-objective baselines over the same candidate set:
- geometry-only selects FILL-MID (worst-case residual error 10 symbolic units);
- time-only selects FILL-OVER (duration 10 symbolic units);
- the multi-objective planner preserves FILL-MID, FILL-OVER and FILL-UNDER as a three-member nondominated set.

A separate two-observation test advances from PASS-02 to PASS-03, supplies a new scan ID plus new scan/registration digests, updates residual geometry from 100±10 to 55±5 units, and verifies that the geometry-only comparator changes from FILL-MID to FILL-55. This demonstrates deterministic replanning on the latest observation, not a physical performance improvement.

The planner's measurement identity now binds the scan-byte digest and registration/transform receipt digest. These are symbolic identifiers in the synthetic v0.1 fixture; physical adapters must calculate and verify real digests from authorized source bytes and transform artifacts.


## Three-timescale closed-loop architecture

The planner is part of a governed three-loop system, not a replacement for the welding controller:

1. **Fast process loop (milliseconds):** independent, qualified equipment/controller regulates the process using current/voltage, wire feed, travel, arc stability and torch-state measurements. The inter-pass planner does not operate at this cadence and does not issue low-level control commands.
2. **Medium pass loop (between deposited passes):** clean/scan/register the new surface, reconcile it with the exact predecessor pass and process trace, quantify coverage/occlusion/registration error/geometry uncertainty, then compute a next-pass Pareto set. If scan uncertainty exceeds its declared decision limit, abstain and request a better observation rather than forcing a recommendation.
3. **Slow quality/lifecycle loop (after inspection/repair):** qualified NDT/repair disposition and production outcomes update the evidence graph, calibration/validation data, procedure applicability and model requalification. Rejected and repaired welds remain represented; they are not silently filtered from training/evaluation. Post-hoc NDT results must never leak into an earlier online decision.

A 2026 survey of closed-loop robotic arc welding identifies a maturity asymmetry: perception-to-execution is comparatively developed, while feedback from assessment into modeling/decision remains fragmented. The slow loop is therefore a core research target, not administrative bookkeeping.

## Industry comparison and fair SOTA boundary

| Comparator | Published evidence | What is comparable | What is not yet comparable |
|---|---|---|---|
| Yu et al. (2026), adaptive CJP T-joint welding | 4,434 beads across 270 T-joint specimens; pass-by-pass residual-volume reallocation; nine additional validation specimens; reported 100% UT acceptance for those nine under AWS D1.1/D1.8 study context | Hierarchical pass allocation, inter-pass scan feedback, residual-volume management, quality-linked outcome | Their 2G horizontal CJP continuity-plate application differs from BAM's 1G flat Tandem-GMAW; nine accepted specimens do not establish a universal failure rate. The paper also reports 31 poor beads (0.7%) identified by NDT and excluded from subsequent parameter analysis; preserve this selection boundary when comparing training data |
| Wang et al. (2026), online path/torch posture planning | Reports path RMSE ≤0.1334 mm and torch-posture RMSE ≤0.8064° in its stated test context | Online geometric adaptation, path/pose accuracy and measurement/reporting discipline | Not a matched pass-allocation or shipyard-production benchmark; no claim that this planner beats those numbers |
| HII / HD Hyundai intelligent mechanized welding pilot (2026) | Workpiece/condition recognition, real-time position correction and process-data capture are described | Workpiece-aware process adaptation and industrial integration | Announced pilot, not an independently measured productivity/quality result |
| Fincantieri BOOST² (2026) | Company reports programme scale across 13 yards and 450+ employees, with work categorized as operational, validating or future | Deployment/maturity context and multi-yard integration | Programme scale is not a matched weld-level performance KPI |

The field's 2026 reviews also caution that physical validation methods and error metrics differ enough that cross-paper numeric rankings can be misleading. We therefore compare only against **matched** specimen geometry, material, process, position, WPS, measurement definition, uncertainty, observation history, compute budget and acceptance method.

## What “better than SOTA” will mean here

- **Geometry:** lower held-out inter-pass residual-area/volume error and surface/profile error than fixed teach-and-playback and geometry-only reallocation, with scan uncertainty/registration error reported.
- **Path and torch posture:** matched metrology and coordinate frames before comparing against published path/posture RMSE.
- **Process:** better calibrated prediction/abstention for process deviations, not just classification accuracy; never infer NDT quality from process signals alone.
- **Accepted production:** improvement in accepted weld length/joints per labour-hour and cell-hour, full fit-up-to-inspection cycle, repair/reinspection burden and cost per accepted output, with identical inspection denominator.
- **Robustness:** higher feasible rate and better worst-stratum outcomes under held-out fit-up, scan, material, position and disturbance conditions.
- **Evidence:** zero unbound scan/transform/procedure identity, zero future-information leakage, zero synthetic-to-physical promotion and zero unauthorized live action.

Aspirational target: at least 5% improvement on one preregistered quality-adjusted outcome versus the strongest matched baseline, with no protected constraint violation or deterioration in other critical outcomes. This is not an achieved result. The BAM dataset's five WIDs remain a small exploratory cohort; WID-grouped evaluation and uncertainty reporting are mandatory.

