# Robust Actuator Pareto and Evidence-Lane Reference v0.1

**Status:** M1 candidate-disposition and first-front filter. This is not a design optimizer, a physical qualification, or proof that submitted robust margins are valid.

**Research owner:** [ROBUST-MULTIMISSION-ACTUATOR-CODESIGN-001 #6234](https://github.com/Luminous-Dynamics/symtropy/issues/6234)  
**Mission-conditioned evaluator:** [#6230](https://github.com/Luminous-Dynamics/symtropy/issues/6230) / [PR #6231](https://github.com/Luminous-Dynamics/symtropy/pull/6231)  
**Common evidence contract:** [#6140](https://github.com/Luminous-Dynamics/symtropy/issues/6140)

## 1. Why a Pareto filter needs evidence and cohort semantics

A conventional Pareto plot can make a vendor claim, a paper-reported result, an independently measured actuator and a model prediction appear directly comparable. It can also compare a mass figure for a bare motor with a complete installed joint, or compare task energy on different mission sets, cooling conditions or uncertainty distributions.

This M1 reference prevents those comparisons by construction:
- evidence classes are preserved per objective and hard constraint;
- only candidates in the same evidence lane can dominate each other;
- a comparison front is limited to one identical system boundary, operating-condition digest, mission-set digest and bounded stress-set digest;
- a known negative robust margin creates an explicit violation receipt;
- missing values or unknown evidence are **Unknown**, not zero and not a pass;
- non-dominated candidates are reported as a vector, without a weighted composite score.

## 2. Evidence lanes

Each objective and constraint carries one evidence class:
- **IndependentPhysical:** independent physical measurement with a cited method/source;
- **SourceAuthorPhysical:** physical test reported by the source authors;
- **VendorPhysical:** vendor-run physical test/specification, not independent;
- **ModelDerived:** analytically or numerically derived, not an independent physical result;
- **CalibratedModelPrediction:** a model prediction carrying a valid specimen-grouped calibration receipt. It remains Conditional, never IndependentPhysical;
- **Unknown:** source/method/evidence state is unavailable or not established.

The candidate is classified using its weakest evidence:
- **Independent:** every required metric and constraint is independent physical evidence;
- **SourceReported:** all are independent/source-author physical, with at least one source-author result;
- **Conditional:** vendor/model-derived evidence is present, or evidence classes are mixed with a conditional class;
- **Unknown:** any required field, metric value, margin or evidence classification is missing/unknown.

No Pareto dominance crosses these lanes. These are disclosure/qualification lanes, not universal quality scores. A vendor result may be carefully measured and valuable, but it is not relabelled as independent.

### Calibration-bound model predictions

A CalibratedModelPrediction record must carry a typed specimen-group calibration receipt linked to [ACTUATOR-SURROGATE-CALIBRATION-001 #6262](https://github.com/Luminous-Dynamics/symtropy/issues/6262) / [PR #6263](https://github.com/Luminous-Dynamics/symtropy/pull/6263). The receipt retains the calibration report digest, model and train/calibration/test split digests, system-boundary/condition/mission/domain identities, finite-sample rank and correction, independent specimen counts, held-out in-domain/covered/uncovered/OOD counts, empirical group coverage, Wilson interval and underlying physical measurement evidence class.

The receipt must match the candidate's model, prediction domain, complete system boundary, operating conditions and mission. It is rejected if the split identities overlap, the conformal rank is insufficient, the held-out in-domain denominator is empty, coverage counts are inconsistent, or the Wilson interval is invalid.

**This reference's current conformal calibration is for output torque only.** It can qualify an N·m torque-margin prediction; it cannot be reused to claim calibrated mass, thermal margin or energy. A calibrated torque prediction remains a model prediction and is deliberately retained in the Conditional evidence lane. A receipt attached to IndependentPhysical or another uncalibrated class is rejected rather than used to launder the claim.

## 3. Cohort identity: what must match before ranking

Candidates are compared only when all of the following digests match:
1. **System boundary:** complete installed mass/volume, power electronics, controller, cabling, remote motors, cooling, mount, ancillary supply and other declared included/excluded elements.
2. **Operating conditions:** bus voltage, ambient, cooling fixture, controller/firmware, duty and other conditions that affect capability.
3. **Mission set:** same frozen task/trajectory distribution and task-success definitions.
4. **Stress set:** same bounded uncertainty set used to compute robust constraint margins.

Every objective record carries its own candidate configuration, system boundary, operating condition, mission and stress digests. Every constraint record additionally carries the stress-set digest used to compute its worst-case margin. Mismatched evidence is rejected instead of silently pooled.

This means a 48 V cooled actuator is not dominated by an uncooled 48 V result; a two-task candidate is not ranked against a one-task candidate; and a bare-motor mass is not ranked against a complete installed-joint mass unless the declared system boundary is actually the same.

## 4. Robust-feasibility contract

The input `minimum_robust_margin` is expected to be the minimum constraint margin over the declared, preregistered bounded stress set. A margin of zero is boundary-feasible; negative is a violation. The filter:
- records a known negative margin as a violation when its evidence class is not Unknown;
- keeps a numeric negative value with Unknown provenance as Unknown, not as a known violation;
- classifies a candidate Infeasible when at least one known violation exists, even if unrelated metrics are also unknown;
- classifies it Unknown when there is no known violation but any required value or evidence is unknown.

**Important limitation:** this crate does not generate the uncertainty set or prove that a supplied margin is a mathematically valid lower bound. That computation needs its own model, calibration, stress-set digest, coverage argument and independent validation. A model-derived violation is a model-evidence violation, not proof of a physical failure; the receipt retains its evidence class, method and source.

## 5. Pareto semantics

Each objective definition declares a name, unit and direction (minimize or maximize). Candidate observations must match the exact metric name/order and units. Duplicate candidate IDs, duplicate objective/constraint definitions, non-finite values, missing dimensions, mismatched units or stale evidence configuration are rejected.

Candidate A dominates candidate B only when:
- both are robust-feasible in the same evidence lane and same exact comparison cohort;
- A is no worse on every declared objective;
- A is strictly better on at least one objective.

Equal vectors remain jointly non-dominated. Trade-offs are preserved. No arbitrary weight converts kilograms, joules, task coverage or cost into one opaque number.

The report contains:
- a disposition for every candidate;
- explicit known constraint-violation receipts;
- unknown-field reasons;
- candidate IDs dominated by a peer in the same cohort;
- the first non-dominated front for each lane/cohort;
- separate excluded lists for infeasible and unknown candidates.

It does not yet compute deeper Pareto ranks, hypervolume, uncertainty-aware objective intervals or an optimization algorithm.

## 6. Research and industry context

### Task-oriented geared actuator co-design

Huang, Dong and Zhao, *Task-Oriented Co-Design and Optimization of Geared Actuators for Robotic Applications*, arXiv:2609.22795 (19 September 2026): https://arxiv.org/abs/2609.22795.

This is a stronger comparator than a catalog-only selection process. It combines surrogate modeling, hierarchical mixed discrete/continuous design exploration and torque-profile co-design, and reports a fabricated two-DoF jumping-leg demonstration. The paper reports 35.7 N·m/kg nominal rated output torque density and 12% greater jump height under its matched twice-rated bench command. Those results are task- and prototype-specific. The open gap for our programme is not “we also optimize for a task”; it is robust multi-mission qualification with frozen stress/validation sets, lifecycle evidence and explicit provenance lanes.

### Multi-task actuation design and real component selection (ICRA 2026)

*Towards the best robot for the job: Optimising actuation design through multi-task co-design and component selection*, IEEE ICRA 2026, DOI https://doi.org/10.1109/ICRA57385.2026.11697232.

The abstract describes task-trajectory-based motor-size/gear-ratio co-design over a representative task set, followed by selection from a finite catalogue of real components. The reported motivation and result are important: the selected design can execute a set of tasks at efficiency comparable to task-specific co-design. This is a stronger baseline than one-task optimization. It still motivates a separate validation layer for measured torque/thermal envelopes, model uncertainty and evidence provenance.

### Task-aware actuator allocation across full-body missions (RA-L 2026)

Nasonov et al., *Task-Aware Actuator Parameter Allocation for Multibody Robots*, IEEE Robotics and Automation Letters (2026), DOI https://doi.org/10.1109/LRA.2026.3674006. Project/source page: https://mkakanov.github.io/actorob/

The published project page describes regression models fit to manufacturer data for 119 motors and 58 gearboxes, a continuous CMA-ES search with population 100 over 100 iterations, and inner constrained full-order trajectory optimization. It evaluates jump, five-step stair ascent with a 10 kg payload and whole-body manipulation with 5 kg in each hand. Its ablation shows the expected tension between electrical energy and gearbox friction/backdrivability: changing scalar objective weights changes gear ratio and joint allocation. The project page reports these result figures: friction-only case 57.47 kg total mass and 43.1 kJ electrical energy; electrical-focus case 48.93 kg and 20.2 kJ; combined case 51.34 kg and 26.7 kJ. These are author/project-page results, not independent lab replications.

The M1 filter here complements these optimizers; it does not replace them. It prevents candidates from different mission sets, stress sets, operating conditions, complete system boundaries or evidence classes from being presented as one comparable Pareto front.

### Multiphysics gear-ratio design

*Gear-Ratio Spectrum for Robotic Joint Motor Drive Systems: Multiphysics Coupling and Design Trade-Offs*, Electronics 15(17), 3834 (2026): https://doi.org/10.3390/electronics15173834.

This review motivates matching candidate comparisons to task torque-speed, reflected inertia, reducer losses, thermal duty, sensing and bandwidth. The Pareto filter does not itself model these factors; it insists that their candidate results and evidence be compared under identical declared conditions.

### Public actuator evidence register

ActuatorList: https://actuatorlist.com/benchmarks

The November 2026 snapshot is useful for discovery and provenance categories, but its reported run counts and lack of lab/community runs are not a fully independent industry leaderboard. This programme keeps vendor, source-author, independent and model-derived information separate rather than inferring measurement quality from a published numeric specification.

## 7. Synthetic known-answer fixture

`examples/robust-pareto-known-answer.json` contains eight synthetic candidate records:
- two independent candidates in the same cohort, with one dominating the other;
- a source-author candidate on a different mission set;
- vendor and model candidates on separate conditional evidence lanes;
- a calibrated-model torque-margin candidate with a complete synthetic grouped-conformal receipt (the receipt does not calibrate its mass, energy, coverage or thermal metrics);
- an unknown-mass candidate;
- an independently evidenced candidate with a known negative thermal margin.

Every numeric value and source entry is a fixture placeholder. It is not actuator data or an industry ranking.

## 8. Run

Dependency-free crate, Rust 1.96:

```bash
cargo fmt --manifest-path benchmarks/synthetic-joints/robust-actuator-pareto/v0.1/Cargo.toml -- --check
cargo test --manifest-path benchmarks/synthetic-joints/robust-actuator-pareto/v0.1/Cargo.toml --locked
cargo clippy --manifest-path benchmarks/synthetic-joints/robust-actuator-pareto/v0.1/Cargo.toml --all-targets --locked -- -D warnings
```

Dedicated workflow: `.github/workflows/robust-actuator-pareto.yml`. The authoring environment has no local Cargo/rustc/rustfmt; exact-head CI is required before compilation, formatting or Clippy is reported as passing.

Thirty-two Rust tests cover mixed objective directions, trade-off preservation, evidence-lane isolation, hard violation, unknown margin/evidence, known violation with unrelated unknowns, duplicate IDs, shape/unit/non-finite validation, equal objective vectors, missing objectives/source, stale configuration/conditions, metric names, mission/stress/system-boundary cohort isolation, explicit violation receipts, complete candidate retention, calibrated predictions remaining Conditional, no cross-lane dominance, missing/stale/insufficient calibration rejection, torque-only receipt use and calibration provenance in violation receipts.

## 9. M1–M5 roadmap

- **M0:** freeze objective/constraint definitions, system boundaries, mission sets, stress sets, source/metric denominators and evidence classes.
- **M1:** known-answer evidence-lane filtering and robust feasibility dispositions.
- **M2:** independently recompute robust margins, validate stress-set coverage, add objective intervals, deeper Pareto ranks and hypervolume with a frozen reference point.
- **M3:** link candidate records to instrumented actuator traces, calibration records and exact build/configuration digests.
- **M4:** multi-task robot comparison with held-out payloads/poses and all failed or aborted results retained.
- **M5:** service/aging remeasurement and post-service Pareto-front migration.

## 10. Claim ceiling

This package filters submitted records; it does not optimize a design, validate the physics of margins, certify a robot, or authorize physical testing. Synthetic fixtures are software tests only. Any physical programme requires independent requirements, hazard analysis, competent procedures, guarded rated fixtures, independent instruments and preregistered abort limits.
