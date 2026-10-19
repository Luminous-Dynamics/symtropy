# QDRIFT-SYNTH-001 — Quality, yield, and process-drift simulation

Status: **reference model / synthetic benchmark only**. This is not a calibrated process twin, a machine controller, a metrology procedure, or manufacturing qualification.

## Why this is a new engineering slice

Existing manufacturing work already covers schedule feasibility, instrument-option auditing, tolerance gates, industrial heat exchange, discrepancy envelopes, and evidence contracts. The next gap worth testing is the feedback loop connecting **latent process drift → measurement → maintenance action → part quality → rework and escaped defects**.

The paradigm change is to optimize *conforming output per total constrained process time*, not gross unit count or nominal cycle time; evaluate candidate policies against the same exogenous disturbance streams; and explicitly test where condition-based control fails because the measurement system itself is biased.

This adds a small, dependency-free Rust reference simulator under tools/quality-drift-sim/. It is intentionally standalone rather than introducing a competing scheduler or claiming that a discrete screening model is a detailed finite-element, cutting-force, welding, or thermal model.

## Model boundary

The discrete process has two latent state variables:

- process_bias: an autoregressive drift term with occasional persistent signed shifts.
- tool_wear: a non-negative increment accumulated per produced part.

Each part also has an independent dimensional-noise term. A part is initially defective when the absolute sum of latent bias, wear, and part noise exceeds the scenario tolerance. Inspection detects a fraction of initial defects; detected parts incur rework time, and only a declared fraction are recovered. Undetected defects and failed rework are recorded as escaped defects.

Three decision policies are compared:

1. no_maintenance: no corrective maintenance; this is a deliberately weak lower control, not a realistic universal baseline.
2. fixed_cycle: reset process bias and wear every 25 parts.
3. condition_based: inspect a noisy sensor signal and reset when its absolute reading or the wear proxy crosses a fixed threshold.

Four scenario families are frozen in code:

- nominal: reference stochastic disturbance and process noise.
- gradual_wear: fewer shifts and smaller process noise, with wear still accumulating.
- shock_stress: more frequent persistent shifts and larger variations.
- biased_sensor_negative_control: identical to the nominal process except that the sensor has a constant offset which masks some true drift.

The numeric values are dimensionless synthetic knobs, and simulated time is an arbitrary time unit. They are selected to exercise the logic, not fitted to a material, CNC machine, additive process, welding procedure, production cell, or particular factory. The sensor is not assumed trustworthy just because it returns a number.

## Reproducibility and analysis

Running the standalone Cargo package emits a JSON report. For every scenario and policy it reports first-pass yield, escaped-defect rate, rework rate, conforming units, maintenance events and downtime, total simulated time, and conforming throughput.

The frozen run design is:

- 100 deterministic seeds per scenario and policy;
- 500 parts per seed;
- three policies and four scenarios (600,000 scenario-policy-part evaluations);
- identical exogenous inputs for the same scenario and seed across policies (common random numbers);
- paired treatment-minus-control differences, with a normal-approximation 95% interval across seeds, labelled descriptive rather than physically calibrated or decision-grade.

A check mode verifies accounting invariants, replay determinism, the expected condition-based advantage inside the three designed validity cases, and the deliberate reversal under the biased-sensor negative control.

## Acceptance contract

Software/benchmark conformance gates:

- Every part is counted exactly once as either conforming output or escaped defect; counts reconcile for every run.
- No run returns non-finite or non-positive simulated time.
- Replaying the same scenario, policy, and seed returns exactly identical metrics.
- For the three designed positive-control scenarios, the condition-based policy's point estimates must improve first-pass yield, escape rate, and conforming throughput relative to fixed-cycle maintenance.
- The report must include paired descriptive intervals. If an interval crosses zero, the result must be described as inconclusive for that metric; a favorable point estimate is not a robust improvement claim.
- In biased_sensor_negative_control, fixed-cycle maintenance must outperform the condition-based policy on those same three point estimates. The negative control forbids claims that adaptive maintenance is universally superior.
- A focused workflow runs formatting, tests, Clippy, the standalone check, JSON schema assertions, and report artifact publication.

Targets for later physical qualification—not current results:

- 100% reconciliation of the digital thread from raw inspection measurement to derived result and maintenance decision.
- Zero false robust-feasible results on seeded critical-tolerance and calibration-expiry controls.
- Evaluate decision quality against an independently computed small-instance oracle before scaling.
- On held-out physical lots, report coverage and worst miss distance for prediction intervals, plus false acceptance of out-of-tolerance parts; never use the training/calibration set as final evaluation.
- Compare against fixed-cycle, no-control and existing shop practice on paired batches with pre-registered criteria.
- Repeat across independent lots, machines, operators, environments and calibration states. Include measurement-system analysis and cost accounting before any business claim.

No numeric performance target is treated as achieved until the Rust implementation is built and tested on its exact commit. No simulated throughput, yield, cost, maintenance benefit, or escaped-defect rate is evidence of a real process.

## How this composes with the stack

- **Symtropy:** owns deterministic process state transitions and counterfactual simulation. The current slice is a standalone research tool; integrating it with the reusable simulation substrate is a follow-on after model review.
- **Symthaea:** can propose discriminating tests and rank experiments by decision value, but should not choose a sensor or process policy from uncertainty scores alone. It must consume the declared model domain and falsifier results.
- **Mycelix:** should carry typed evidence links from raw measurement through calibration identity, scenario/model version, simulation run, review, and eventual physical observation. Hashes prove byte identity, not measurement truth.
- **Sol-Atlas:** can later contribute evidence-qualified supply, energy, and facility capacity constraints, but those are outside this model.

## Research anchors

- NIST, [Digital Twins for Advanced Manufacturing](https://www.nist.gov/programs-projects/digital-twins-advanced-manufacturing): emphasizes verification, validation, uncertainty quantification, trustworthiness, and interoperability.
- NIST, [Digital Twins Workshops Summary Report](https://www.nist.gov/publications/digital-twins-workshops-summary-report), published 21 July 2026: reports persistent needs in interoperability, VVUQ, cybersecurity, and workforce readiness.
- Yang, Chen, and Wu, [Active learning of model discrepancy with Bayesian experimental design](https://doi.org/10.1016/j.cma.2025.118198), *Computer Methods in Applied Mechanics and Engineering* (2025): motivates selecting informative measurements to characterize model discrepancy rather than treating a digital twin as exact.

These sources motivate validation discipline. They do not validate this model or support claims about actual factory performance.

## Next tranche

1. Review the state equations and policy semantics with a manufacturing/process-control engineer.
2. Pin one open and properly licensed process dataset or run a controlled benign bench experiment.
3. Define measurand, traceable calibration, uncertainty, and a held-out split before fitting anything.
4. Compare simulated outcomes to the held-out measured trajectory; reject or widen the domain when discrepancy is excessive.
5. Add a decision receipt consumable by Symthaea and Mycelix only after independent audit.

Never connect this reference simulator directly to machine actuation. Safety-critical maintenance, inspection, or release decisions require qualified procedures and competent human authorization.
