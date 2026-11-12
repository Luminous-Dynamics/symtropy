# Evidence-gated inter-pass adaptive planner v0.1

Tracking: [SHIPYARD-INTERPASS-ADAPTIVE-001 #4649](https://github.com/Luminous-Dynamics/symtropy/issues/4649)

## Why this is not just another adaptive path planner

Recent adjacent SOTA already includes:
- Yu et al. (2026): 4,434 beads from 270 specimens; inter-pass 3D profiling; residual cross-sectional volume reallocation; nine additional validation specimens with reported UT acceptance.
- Wang et al. (October 2026): online synchronous welding path and torch-posture planning, reporting path RMSE ≤0.1334 mm and torch-posture RMSE ≤0.8064° across its stated test context.
- A February 2027 closed-loop robotic welding survey is forthcoming as of 12 November 2026 and is not used as current evidence. The programme's three-loop architecture below is our engineering design, not a claim derived from that future publication.

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


## October 2026 online laser-welding comparator

Moscatelli, Tanelli and Demir (2026), *Vision-based real-time autonomous trajectory planning in robotic laser welding applications*, Journal of Manufacturing Processes, 15 October 2026, DOI [10.1016/j.jmapro.2026.08.049](https://doi.org/10.1016/j.jmapro.2026.08.049), report a coaxial-camera/deep-learning online seam estimation and incremental robot trajectory loop. The study reports nominal perception-to-execution latency of 35 ms and prediction-to-seam deviations below 200 µm for its stated linear/curved laser-welding tests. Data are confidential and only processed data may be available subject to the industrial partner. This is a laser trajectory/control result, not a matched GMAW inter-pass volume-allocation or shipyard qualification benchmark; we must not compare its seam-centreline deviation directly to our synthetic geometry units.

The closest process-specific digital-twin reference is **published** ISO/TR 23247-101:2026 (May 2026), which explicitly covers monitoring/management of robotic multilayer and multipass GMAW. **Under development**, ISO/AWI 23247-8 was approved and registered on 23 October 2026 for VVUQ of manufacturing digital twins. Align evidence-lifecycle and VVUQ interfaces with those documents, but do not claim conformity to an unfinished work item or that the Technical Report itself validates our implementation.
