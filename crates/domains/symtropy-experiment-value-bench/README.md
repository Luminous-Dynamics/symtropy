# Symtropy Experiment Value Bench

Standalone, read-only, finite-state one-step expected value of sample information (EVSI) evaluator for selecting candidate physical research experiments.

Implements the decision-calculation slice tracked by [issue #3914](https://github.com/Luminous-Dynamics/symtropy/issues/3914), composing with #3853's eligibility/Pareto evaluator and the measurements owned by #3864/#3876, #3877/#3913, #3888/#3890 and #3891/#3910.

## Run

    cargo test --manifest-path crates/domains/symtropy-experiment-value-bench/Cargo.toml

## Calculation

For a finite hypothesis/state space S, prior p(s), actions A, declared utility U(a,s), and experiment e with outcomes z and likelihood L(z|s,e):

- Current decision value: V0 = max_a sum_s p(s) U(a,s)
- Predictive outcome probability: P(z|e) = sum_s p(s)L(z|s,e)
- Posterior after outcome z: p(s|z,e) = p(s)L(z|s,e)/P(z|e)
- Expected post-experiment decision value: V1(e) = sum_z P(z|e) max_a sum_s p(s|z,e)U(a,s)
- EVSI(e) = V1(e) - V0
- NetEVSI(e) = EVSI(e) - experiment cost

The evaluator is **myopic one-step EVSI**, not an optimal multi-step experimental policy. It does not compute EVPI, adaptive policy trees, correlated observations, continuous parameter posteriors or risk measures beyond the declared expected utility.

## Evidence and decision boundaries

- Priors and likelihoods have evidence role, method and source references.
- Expert elicitation is explicit, never silently converted to measurement.
- Simulation-only and synthetic fixtures can demonstrate arithmetic but receive SYNTHETIC_ONLY / NOT_DECISION_QUALIFIED dispositions.
- Utilities must be supplied by a named decision owner and bound to a validated decision profile. Utility values are not inferred from benchmark accuracy or convenience.
- Experiment cost must already be expressed in the same declared net-benefit unit. The conversion from actual labor, rig time, money, energy, opportunity cost or consumables to that utility unit must be externally justified and referenced.
- A stop threshold and remaining budget are enforced. A positive EVSI does not guarantee positive net EVSI.
- This evaluator does not recommend machine settings, line pressures, velocities, cutting depths, blockage-clearing actions or operating procedures.

The bundled case is fabricated arithmetic conformance data. Its EVSI is not a real excavation recommendation.
