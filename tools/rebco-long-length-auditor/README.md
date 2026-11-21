# REBCO long-length auditor

Run from the Symtropy repository root:

```sh
cargo test --locked -p rebco-long-length-auditor
cargo run --locked -p rebco-long-length-auditor -- docs/research/fixtures/rebco_long_length_synthetic_v0_1.json
```

The auditor validates reel/segment geometry, observation identity, evidence-state requirements, source digest shape and conservative full-width critical-current coverage. It distinguishes direct transport Ic from calibrated non-contact magnetization-proxy Ic; proxy coverage is reported separately and never counted as direct transport coverage. Missing or false calibration status excludes a reading from the corresponding qualified coverage and is reported separately as observed-but-unqualified. It explicitly does not evaluate conductor acceptance. The bundled fixture is synthetic only. See `docs/research/superconductivity/SC_LONG_LENGTH_QUALIFICATION_V0_1.md` for scope and claim limits.


## Condition-profile-specific Ic coverage (follow-on contract)

The condition-profile child PR #6602 partitions direct transport, calibrated non-contact proxy, inferred and calibration-unqualified Ic spatial coverage by an exact operating-condition identity and measurement unit. It does not union 77 K self-field and 20 K / 5 T intervals into one coverage figure. Geometry remains observation-specific; different measurement geometries may target the same operating condition, but field, temperature, orientation, current criterion and other operating values must match exactly. The report has no cross-profile global Ic coverage number and performs no T/B interpolation.


## Adaptive inspection value-of-information prototype

Schema: `docs/research/schemas/rebco_adaptive_inspection_voi_v0_1.schema.json`.

Synthetic prior/action fixture: `docs/research/fixtures/rebco_adaptive_inspection_voi_synthetic_v0_1.json`.

Run:

```bash
cargo test --locked -p rebco-long-length-auditor --bin adaptive_inspection_voi
cargo run --locked --quiet -p rebco-long-length-auditor --bin adaptive_inspection_voi -- \
  docs/research/fixtures/rebco_adaptive_inspection_voi_synthetic_v0_1.json
```

The policy evaluates one next action against a weighted ensemble of complete spatial weak-cell maps. It computes the probability of an application-defined contiguous weak run, asymmetric false-accept / false-reject expected decision loss, expected posterior loss after positive/negative measurement outcomes, measurement cost and net VOI. It distinguishes direct-transport actions from calibrated non-contact proxy actions and enforces the exact condition-profile ID, calibration state, validation artifact digest and inspection budget. After a real observation is independently recorded, append its result and rerun to select the next action. The binary itself recommends but does not perform an inspection.

This v0.1 is a one-step greedy synthetic decision-support policy, not a global multi-step optimum, physical product acceptance decision, or claim that the illustrative sensitivities, specificities, costs, prior scenarios or loss values describe a manufacturer or real application.


## Finite-horizon sequential inspection planner

Schema: `docs/research/schemas/rebco_sequential_inspection_dp_v0_1.schema.json`.

Synthetic joint-scenario fixture: `docs/research/fixtures/rebco_sequential_inspection_dp_synthetic_v0_1.json`.

Run:

```bash
cargo test --locked -p rebco-long-length-auditor --bin sequential_inspection_dp
cargo run --locked --quiet -p rebco-long-length-auditor --bin sequential_inspection_dp -- \
  docs/research/fixtures/rebco_sequential_inspection_dp_synthetic_v0_1.json
```

This exact finite-horizon dynamic program minimizes inspection cost plus terminal asymmetric false-accept / false-reject decision loss over a contingent policy tree. It uses a canonical posterior-weight state key and exact memoization for small synthetic grids. At each policy node, it can inspect once or stop; each action is used at most once per path, and cost / horizon / exact condition-profile / calibration gates are enforced. The receipt compares the optimal finite-horizon policy with the best one-step policy on the same fixture and hashes the exact fixture bytes.

This v0.1 is intentionally capped at 16 actions and horizon 6. It is a correctness/oracle prototype, not a scalable planner, globally optimal policy for continuous spatial fields, physical inspection, manufacturer-calibrated sensitivity model or product-release authority.


## Exact finite-horizon sequential inspection planner

Schema: `docs/research/schemas/rebco_sequential_inspection_dp_v0_1.schema.json`.

Synthetic joint-scenario fixture: `docs/research/fixtures/rebco_sequential_inspection_dp_synthetic_v0_1.json`.

Run:

```bash
cargo test --locked -p rebco-long-length-auditor --bin sequential_inspection_dp
cargo run --locked --quiet -p rebco-long-length-auditor --bin sequential_inspection_dp -- \
  docs/research/fixtures/rebco_sequential_inspection_dp_synthetic_v0_1.json
```

The exact finite-horizon planner minimizes total inspection cost plus terminal asymmetric false-accept/false-reject decision loss. It creates a contingent policy tree, uses a canonical posterior belief state with memoization and can stop at any node. It has a hard action-count/horizon ceiling for v0.1 and validates exact condition profile, calibration status, method identity, input digest, prior weights and total budget. Its tests compare the memoized planner against an exhaustive recursive oracle on a small grid, demonstrate a myopic-vs-sequential case, and validate path constraints. It recommends; it never performs a measurement or releases a reel.

The fixture's numbers are synthetic decision-loss units, not money or validated risk estimates. The result is not a scalable planner or a globally optimal policy over real continuous spatial fields. Use it as a correctness prototype before implementing robust uncertain-likelihood planning or larger-grid approximations.
