# PHRI QUAL BENCH 001 — matched comparison protocol

Status: protocol scaffold and scenario manifest only. No simulation benchmark has been executed and no controller has been qualified.

Parent issue: https://github.com/Luminous-Dynamics/symtropy/issues/4754

## Objectives

This benchmark separates three questions that are too often merged:

1. Does the mathematical optimizer satisfy its declared continuous-time constraints?
2. Does the sampled and executed controller satisfy a discrete physical-port energy contract under sensing/actuation imperfections?
3. Does the integrated system deliver useful assistance while preserving user agency and acceptable burden?

A pass in one layer does not imply a pass in another. The current manifest includes 22 adversarial scenario families, six cross-sensor transfer regimes, and five analytic optimizer fixtures.

## Comparator arms

Use independent implementations/adapters for fixed full-matrix impedance/admittance, energy-tank/passivity-observer methods, the Dinc diagonal VIC QP, the Dinc generalized-eigenbasis compliance method, Mühlbauer-style passive matrix stiffness/shared-control, Poignonec-style passivity-constrained NMPC, and the Luminous candidate. Record exact paper version, code revision, model assumptions, parameterization, and any deviations. Do not fabricate a baseline where source code or method details are unavailable; label that arm not_reproduced and explain why.

## Comparison regimes

### Matched common subspace

Hold the plant, port, state, task, parameterization, bounds, update period, numerical precision and compute budget constant. Where a published comparator permits it, restrict to the same scalar inertia scale and full symmetric stiffness-rate subspace.

### Native expressivity

Evaluate the native model each method was designed for. Report the dimensions/degrees of freedom and representable parameter trajectories. Never use a lower-dimensional candidate's reduced runtime or feasibility burden as evidence that it dominates a more expressive comparator.

## Frozen trial identity

Every run must serialize:

- source repository, commit SHA and dirty-tree status;
- plant/model, geometry, inertial parameter and actuator revisions;
- controller period, integration method and parameter update schedule;
- sensor calibration, measurement-profile and clock-domain identities;
- port sign convention and coordinate frames;
- trajectory, initial state, payload, disturbance and random seed;
- numeric precision, compiler, solver settings and hardware;
- safety limits, hazard-analysis reference and stop policy;
- all exclusions, failures, infeasibility and abstentions.

Tune on a development set. Freeze settings, then evaluate on a held-out set. Publish all case IDs and receipts.

## Scenario manifest

The cases-v0.json file is a machine-readable starting manifest. The receipt-v0.schema.json file defines the provenance-bound record required for each eventual benchmark arm/run, including the declared control-volume graph, external boundary ports and paired internal connections; no receipt is populated with fictitious results. It defines adversarial families and the assertions each family is intended to exercise. It also contains five hand-derived, diagonal known-answer fixtures for the proposed matrix-rate optimizer, including the scalar boundary-only case. Four additional scenario families cover sensor-identity/lot-held-out tactile transfer, matrix-valued passivity-index analysis within an explicitly declared LTI scope, multi-port control-volume/cut-set reconciliation, and local pressure/exposure constraints that must not be hidden by a favourable global energy balance, plus multirate proposal expiry and fast-loop compute-contention cases. It also distinguishes global port-energy accounting from local pressure, deformation, power and exposure-time envelopes using a non-human viscoelastic phantom. These expected outputs are analytic fixtures, not executed optimizer tests or benchmark results. The manifest deliberately does not invent hardware thresholds or physical measurements. Extend the schema version when semantics change. A case is not evidence until a concrete trace, source SHA, run receipt and independently reproduced result exist.

## Scorecards

Keep independent reports for:

- mathematical/software: passivity residual, PSD/SPD margin, KKT/primal/dual residual, false-feasible rate, coordinate invariance, solver latency/deadlines and abstention;
- physical interaction: port work, actual applied output, force/torque, pressure-time, contact area/CoP, actuator limits and thermal response;
- agency/burden: explicit stop/refusal, cancellation, predictability, workload, discomfort, fit and task independence, only after approved studies;
- product/lifecycle: capability, energy, repair/requalification, configuration provenance, cybersecurity and privacy.

Safety, dignity, privacy and user refusal are hard constraints. They are never compensated by task success or a weighted aggregate score.

## Reporting rule

Every headline number needs a unit, sample count, distribution, uncertainty method, hardware/software identity, test domain and comparator. Report p50/p95/p99/max where meaningful. Retain the failed cases and show worst-prefix energy, not only end-of-run averages. Label every result as one of: analytically derived, synthetic reference, software-in-the-loop, instrumented bench, HIL, or ethically approved human-subject evidence.

## Standards note

Standards applicability depends on intended use and regulatory classification. The research protocol is not a conformity assessment. In particular, the current scopes of ISO 13482, ISO 25785-1, ISO 10218 and ISO/TS 15066 differ; medical rehabilitation systems and external limb prostheses require separate classification analysis. See the parent issue for current references and scope caveats.
