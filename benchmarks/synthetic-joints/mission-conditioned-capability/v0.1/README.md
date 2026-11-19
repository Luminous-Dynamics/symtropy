# Mission-Conditioned Joint Capability Evaluator v0.1

**Status:** M1 scalar mechanical torque-speed trajectory evaluator and synthetic fixture. This is not a full actuator certification, thermal solver, electrical model or robot task-success demonstration.

**Research owner:** [MISSION-CONDITIONED-JOINT-CAPABILITY-001 #6230](https://github.com/Luminous-Dynamics/symtropy/issues/6230)

Related evidence tracks:
- paired actuator observability [#6171](https://github.com/Luminous-Dynamics/symtropy/issues/6171);
- thermal mission envelope [#6155](https://github.com/Luminous-Dynamics/symtropy/issues/6155);
- regenerative energy flow [#6185](https://github.com/Luminous-Dynamics/symtropy/issues/6185);
- task-space capability [#6189](https://github.com/Luminous-Dynamics/symtropy/issues/6189);
- lifecycle reliability [#6195](https://github.com/Luminous-Dynamics/symtropy/issues/6195);
- sampled passivity [#6200](https://github.com/Luminous-Dynamics/symtropy/issues/6200).

## 1. Research gap: characterization does not establish task feasibility

Torres Rodríguez et al., *Toward Reproducible Actuator Characterization: A Comprehensive Framework for Robotics Applications*, Actuators 15(6), 301 (30 May 2026), https://doi.org/10.3390/act15060301, organizes eleven no-load, locked-rotor and dynamic-load experiments. It adds bandwidth-under-load, disturbance-rejection, stability and power/energy indicators. The paper's own limitations include the lack of direct validation against task-level robotic performance.

The M1 reference in this directory addresses that missing bridge: a candidate's evidence-bounded actuator envelope is evaluated against a complete required joint trajectory, with uncertainty carried through the torque margin.

*Gear-Ratio Spectrum for Robotic Joint Motor Drive Systems: Multiphysics Coupling and Design Trade-Offs*, Electronics 15(17), 3834 (2026), https://doi.org/10.3390/electronics15173834, further motivates coupling task torque-speed profiles with ratio, reflected inertia, forward/reverse efficiency, thermal duty, reducer nonlinearities, sensing and control bandwidth. Its worked example is model-based and assumption-bound, not a universal actuator ranking.

## 2. Current public comparison boundaries

- ActuatorList, https://actuatorlist.com/benchmarks, reports 22 published runs across 17 catalog actuators in its 3 November 2026 snapshot. Its method/provenance labels distinguish raw traces, tables, digitized plots and text-reported data, and vendor/affiliated/independent sources. It reported zero lab and zero community runs in the snapshot checked on 19 November 2026.
- NSK's September 2026 technical review reports 110 N m repetitive peak, about 110 N m/kg, 80 mm × 79 mm package, mass ≤1 kg and an 18 mm hollow bore. These remain manufacturer-reported package/peak specifications, not an independent continuous-duty result.
- HEBI publishes continuous/intermittent torque-speed regions and frequency-response plots, under specified bus-voltage, ambient, cooling and controller-gain assumptions.
- Commercial knee torque and peak torque-density statements (including Unitree's public H1/M107 specifications) must remain in the vendor-specification lane when mass boundaries, output speed, peak duration or thermal limits are not disclosed.

The profile therefore preserves evidence class per envelope rather than collapsing all sources into a single confidence badge.

## 3. Candidate and mission contract

The candidate envelope is bound to:
- exact candidate configuration digest;
- operating-condition digest covering bus voltage, ambient, cooling/fixture, controller, firmware and relevant settings;
- separate Hold, forward/reverse motoring and forward/reverse regenerating branches;
- torque-speed knots with uncertainty for continuous and peak torque;
- maximum interpolation gap (large gaps are unknown, never bridged silently);
- branch-level conservative contiguous peak-duration limit;
- evidence class, source, method, calibration and matching configuration/condition digests.

The mission contains time-ordered samples of:
- signed required joint torque;
- required torque uncertainty;
- signed joint angular velocity;
- candidate configuration and operating-condition digests.

The uncertainty-adjusted demand is:

```text
required_upper = abs(required_torque) + torque_uncertainty
available_lower = measured_or_reported_torque - envelope_uncertainty
robust_margin = available_lower - required_upper
```

Interpolation is piecewise linear and only allowed between supplied knots. No extrapolation is permitted. Crucially, endpoint uncertainty alone does not make the interpolated curve conservative: every off-knot value additionally requires a validated upper bound on interpolation/model error, which is subtracted from the interpolated available torque. If that bound is missing, the off-knot sample is Unknown. Exact measured knots remain usable. The branch-level peak-duration limit must be a conservative lower bound applicable across that branch's declared speed/condition range; a single catalog peak torque is not a duration envelope.

## 4. Decisions

Every sample is classified as:
- **Continuous:** uncertainty-adjusted demand is inside the conservative continuous torque bound.
- **Peak-limited:** continuous margin is negative, peak margin is nonnegative, and a peak-duration bound is qualified.
- **Infeasible:** required torque upper bound exceeds peak torque lower bound, or consecutive peak operation exceeds its declared duration.
- **Unknown:** missing branch, missing peak duration, out-of-domain speed, excessive interpolation gap, unknown evidence provenance or ambiguous torque direction.
- **Coast:** zero nominal torque with zero torque uncertainty; no output torque capability is claimed.

Mechanical-envelope result (the Rust type is MechanicalEnvelopeDecision):
- **Feasible:** every torque-demand sample is covered and every mechanical torque/peak-duration condition passes.
- **Infeasible:** at least one known mechanical envelope violation.
- **Unknown:** no known violation, but at least one required sample is unsupported or ambiguous.

This is not a whole-robot mission-success or safety disposition. The result preserves candidate/configuration and operating-condition digests, plus source reference, method, calibration reference and evidence class on each assessed sample. A mission can have unknown coverage and a separate known infeasible sample; the per-sample record preserves both counts.

## 5. Metrics

The current M1 reference reports:
- nominal peak required torque and conservative peak required-torque upper bound;
- peak speed;
- time-weighted RMS required torque;
- positive mechanical work;
- absolute negative mechanical work (mechanical energy absorbed);
- minimum uncertainty-adjusted continuous/peak margins;
- torque-envelope coverage over samples requiring a torque branch (coast samples are excluded; a coast-only mission reports coverage as not applicable);
- torque-demand, evidence-covered, coast, unknown and infeasible sample counts;
- peak-duration violations and observed contiguous peak duration.

Negative mechanical work is **not** electrical regeneration. Electrical input, DC-bus return, storage acceptance, thermal state, passivity, task-space wrench, reliability and task completion are separate gates and must not be inferred from this M1 result.

The evaluator returns a Pareto-ready metric vector, not a single weighted score. Cost, installed mass/volume, thermal margin, current, voltage, efficiency, backdrive impedance, task performance and evidence confidence are not yet computed by this crate.

## 6. Evidence classes

The Rust API preserves:
- independent raw trace;
- independent tabulated measurement;
- source-author physical test;
- vendor physical test;
- digitized source plot;
- reported without method;
- model-derived;
- unknown.

A vendor result can support a vendor-evidence comparison, but is never promoted to an independent result. Unknown provenance cannot produce a feasible decision. Every branch's evidence must match the candidate configuration and operating-condition digests.

## 7. Synthetic known-answer tests

`examples/mission-capability-known-answer.json` contains a synthetic Hold → forward-motoring → forward-regenerating mission with three direction-specific model-derived envelopes. The mass and volume are explicitly unknown. No hardware data are included.

Twenty-one Rust test definitions cover:
1. in-domain interpolation and robust continuous margin;
2. uncertainty-driven peak classification;
3. peak-duration overrun;
4. torque above peak capability;
5. no extrapolation outside measured envelope;
6. missing direction branch;
7. uncertainty reversing nominal feasibility;
8. excessive interpolation gap;
9. candidate/configuration condition mismatch;
10. separate positive and absorbed mechanical work;
11. time-weighted RMS torque;
12. hold versus coast;
13. ambiguous operating direction when torque uncertainty crosses zero;
14. invalid knots/timestamps/uncertainty;
15. unknown evidence cannot create a feasible result;
16. vendor provenance remains vendor provenance;
17. coast-only tasks do not claim torque-envelope coverage;
18. off-knot interpolation is Unknown without a validated error bound;
19. interpolation-error bound is subtracted from available torque;
20. candidate/operating-condition digests and evidence references/methods survive into results;
21. duplicate regimes and evidence-configuration mismatch.

These are synthetic software tests, not physical actuator experiments.

## 8. Run

Dependency-free crate, Rust 1.96:

```bash
cargo fmt --manifest-path benchmarks/synthetic-joints/mission-conditioned-capability/v0.1/Cargo.toml -- --check
cargo test --manifest-path benchmarks/synthetic-joints/mission-conditioned-capability/v0.1/Cargo.toml --locked
cargo clippy --manifest-path benchmarks/synthetic-joints/mission-conditioned-capability/v0.1/Cargo.toml --all-targets --locked -- -D warnings
```

Dedicated workflow: `.github/workflows/mission-conditioned-joint-capability.yml`. The authoring environment has no local Cargo/rustc/rustfmt; report only exact-head CI results as executed validation.

## 9. M2–M5 expansion

- M2: ingest public actuator maps with source-exact operating conditions; independently reconstruct torque-speed and thermal curves; include directional efficiency and uncertainty; evaluate held-out missions.
- M3: synchronized independent torque, speed, DC voltage/current, thermal and vibration instrumentation on a guarded rig.
- M4: matched integrated robot tasks, with task success/tracking error and energy/thermal margin.
- M5: repeated missions, degradation, service, remounting and post-service requalification.

Thermal state must eventually be integrated as a dynamic history rather than a single ambient/temperature lookup. Current/voltage, regenerative storage and passivity gates will be consumed from their separate benchmark records.

## 10. Claim ceiling and safety

M1 only reports scalar mechanical torque-speed feasibility against a declared evidence envelope. It does not establish thermal, electrical, impact, passivity, human-contact, lifecycle or system safety. No energization, fabrication, endurance, destructive, high-energy impact, battery or human-contact test is authorized. Physical work needs an independently approved test plan, hazard analysis, competent operators, guarded rated fixtures and independent instrumentation.
