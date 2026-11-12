# PHRI QUAL BENCH 001 — matched comparison protocol

Status: protocol scaffold and scenario manifest only. No simulation benchmark has been executed and no controller has been qualified.

Parent issue: https://github.com/Luminous-Dynamics/symtropy/issues/4754

## Objectives

This benchmark separates three questions that are too often merged:

1. Does the mathematical optimizer satisfy its declared continuous-time constraints?
2. Does the sampled and executed controller satisfy a discrete physical-port energy contract under sensing/actuation imperfections?
3. Does the integrated system deliver useful assistance while preserving user agency and acceptable burden?

A pass in one layer does not imply a pass in another. The current manifest includes 34 adversarial scenario families, 14 comparator/negative-control arms, six cross-sensor transfer regimes, and five analytic optimizer fixtures. Twelve added families qualify cross-domain composition, identity coherence, missing evidence, authorization, actuator authority, revocation/re-arm and time-of-check/time-of-use races.

## Comparator arms

Use independent implementations/adapters for fixed full-matrix impedance/admittance, energy-tank/passivity-observer methods, the Dinc diagonal VIC QP, the Dinc generalized-eigenbasis compliance method, Mühlbauer-style passive matrix stiffness/shared-control, Poignonec-style passivity-constrained NMPC, matrix-valued LTI passivity-index analysis, the Luminous candidates, a conjunctive interaction-envelope gate, and two negative controls: a weighted composite-risk score and independently evaluated but identity-unbound evidence. These last three arms are architecture comparisons, not claims that all methods have a ready-made public implementation. Record exact paper version, code revision, model assumptions, parameterization, and any deviations. Do not fabricate a baseline where source code or method details are unavailable; label that arm not_reproduced and explain why.

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

The cases-v0.json file is a machine-readable starting manifest. The receipt-v0.schema.json file defines the provenance-bound record required for each eventual benchmark arm/run, including the declared control-volume graph, external boundary ports and paired internal connections; no receipt is populated with fictitious results. The manifest now contains 34 scenario families across optimizer conditioning, sampled energy, multi-port cut sets, tactile metrology/transfer, local exposure, wearer agency, proposal expiry, fast-loop contention, and compositional admission. The compositional extension includes explicit negative controls for weighted-score compensation and evidence checks that do not bind to a shared identity. It also includes five hand-derived optimizer known-answer fixtures and six tactile-transfer regimes. A fixed-seed campaign targeting 30,000 generated fault combinations (seed 20261112) is a planned benchmark target, not an executed test. Expected fixtures are analytic and hand-derived; no simulation, HIL, physical phantom, or human-subject benchmark has been executed. The manifest deliberately does not invent hardware thresholds or physical measurements. Extend the schema version when semantics change. A case is not evidence until a concrete trace, source SHA, run receipt and independently reproduced result exist.

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

## Compositional admission extension

The v0 manifest compares a conjunctive interaction-envelope gate against two explicit negative controls: (1) a weighted risk score that can trade a local contact failure against an energy surplus, and (2) separate domain checks that do not bind all evidence to the same command and state revision. The candidate architecture admits only when sampled-energy, local-contact, explicit wearer authority, freshness/identity, and independent actuator-authority predicates all pass for the same event.

The planned generated-fault campaign uses seed 20261112 and targets at least 30,000 combinations. Report false admissions and false rejections separately, preserve the per-domain rejection vector, and retain the exact case manifest. The target is zero false admissions in the declared finite matrix; this is a target, not a current result or universal risk guarantee.

Target performance reporting:
- gate runtime p50/p95/p99/maximum on each declared target computer; aspirational p99 less than 10% of the local-loop period, measured rather than assumed;
- zero blocking or waiting of the local loop on the optimizer in injected contention scenarios;
- 100% exact identity match for every admitted bundle;
- zero use of receipts after revocation, supersession, expiry or evidence-digest change;
- false rejection, abstention, and infeasibility remain visible rather than being optimized away.

These are software/protocol targets only. Hardware timing, physical stop latency, calibration uncertainty, local contact metrology, and user-agency outcomes require their own instrumented evidence and must not be inferred from host unit tests.


## Related work and baselines

See [RELATED-WORK-AND-BASELINES.md](RELATED-WORK-AND-BASELINES.md) for the current comparator map, evidence boundaries, proposed targets and explicit anti-overclaim rules. The comparison includes recent variable-impedance, matrix-stiffness, tactile-transfer, contact-aware benchmark and agency-aware autonomy work, along with NIST measurement-science context.
