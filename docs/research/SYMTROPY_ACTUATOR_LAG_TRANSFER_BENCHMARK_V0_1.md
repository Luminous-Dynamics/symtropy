# Symtropy actuator-lag transfer benchmark v0.1

**Status:** controlled internal stress benchmark, pending exact-head CI. This is not a calibrated motor model, a real-flight result, or a control/safety claim.

## Research hypothesis

The core hypothesis is not simply "temporal networks are better." It is narrower and falsifiable:

> When actuator response changes between training and evaluation, explicit temporal representations should lose less one-step residual-prediction skill than a current-state model, and persistent LTC recurrence should contribute only if it beats both explicit finite history and the same HDC encoder with its LTC state reset at each step.

This targets a real system-identification problem. The 2026 IDSIA Nano-Quadrotor benchmark reports that rotational dynamics remain challenging and identifies neglected low-frequency actuation dynamics as a limitation of standard quadratic motor models that residual feedforward alone does not fully compensate for. That finding motivates an actuator-focused ablation rather than assuming architecture alone is the bottleneck. See the [published paper](https://doi.org/10.1016/j.conengprac.2026.106871), [public benchmark implementation](https://github.com/idsia-robotics/nanodrone-sysid-benchmark), and [Nonlinear Benchmark Initiative entry](https://www.nonlinearbenchmark.org/benchmarks/nano-drone).

## Controlled plant intervention

The internal \`SymtropyFlightSimulator\` can now be configured with an optional first-order command response:

\[
u_{\mathrm{effective},t+\Delta t}
= u_{\mathrm{effective},t}
+ \left(1-e^{-\Delta t/\tau}\right)
\left(u_{\mathrm{command},t}-u_{\mathrm{effective},t}\right)
\]

The four filtered channels are total thrust and the three body moments. Each episode resets the filter state to hover thrust and zero moments. \`None\` / \`0\` preserves existing direct-command behavior. Invalid negative, non-finite, or zero non-disabled time constants fail closed.

This is a controlled stress model, not a rotor-resolved actuator simulation: it does not model motor speed, per-rotor thrust, battery voltage, ESC response, saturation dynamics, or the calibrated motor-speed interface in the public Nano-Quadrotor benchmark. The time constants are declared interventions, not claimed measurements of a Crazyflie.

## Preregistered profile

- Training plant: \(\tau=20\) ms.
- Evaluation plants: direct actuation (\(\tau=0\)), 10 ms, 20 ms, and 50 ms.
- The exact same deterministic training dataset and digest must be reused across all four evaluation profiles.
- Run raw-state ridge, current-state MLP, explicit-history MLP, persistent frozen HDC/LTC, and HDC/LTC with recurrent state reset at every step while preserving the encoder's derivative history.
- Fit the two HDC readouts independently on their corresponding train representations.
- Within each profile, require matching test episode seed order, scenario IDs/families, nominal model, and plant configuration across all predictor arms. Disturbance labels remain unavailable as features.
- The learned predictors remain evaluation-only; the PID baseline drives each test plant. Model predictions do not alter the controller.

This profile estimates how prediction error shifts as the test actuator changes. It does **not** estimate the control benefit of applying a learned residual, because the predictions are not injected into the controller.

## Primary metrics

For each evaluation time constant and model, preserve the existing episode-level one-step residual RMSE, scenario-family metrics, empirical CVaR90/worst-episode RMSE, invalid prediction count, and p50/p95/p99 inference latency. Compare every arm on matched test seeds within each profile.

The new primary summary is transfer degradation relative to the matched 20 ms evaluation:

\[
D_m(\tau) =
\frac{\operatorname{RMSE}_m(\tau)-\operatorname{RMSE}_m(20\text{ ms})}
{\max(\operatorname{RMSE}_m(20\text{ ms}), \epsilon)}
\]

where \(m\) is a model arm and \(\epsilon\) is a fixed small denominator floor declared before confirmatory runs. The initial script preserves the raw paired model summaries for analysis; the current smoke result is too small to support confirmatory significance claims.

### Falsification / decision rule

A useful result would show that the explicit-history or HDC/LTC arm has a smaller paired degradation under at least one out-of-training actuator profile (0, 10, or 50 ms) without hiding a worse CVaR90 or worst episode. Persistent LTC-memory value requires persistent HDC/LTC to beat both explicit history and the matched LTC-reset-each-step arm.

For a confirmatory claim, preregister at least 30 independent test episodes per relevant family and profile (or a power calculation requiring more), repeat across independently generated seeds, and use trajectory/episode-level uncertainty rather than frame-level bootstrap. A suggested research target is at least 10% lower relative degradation than the strongest matched non-persistent baseline on two predeclared shift profiles with paired uncertainty excluding zero. This is a hypothesis threshold, not a universal standard and not a result achieved here.

Reject or narrow the claim if explicit history explains the gains, recurrent-state reset does not hurt, the model degrades more severely under lag shifts, the effect disappears under more episodes, or invalid predictions / worst-case errors increase.

## Integrity receipt

The result files record the complete plant configuration, actuator model, time constant, initialization policy, dataset digest, source SHAs, scenario matrix digest, seeds, and paired metrics. The collector records that \`applied_action_ctrl\` is the clamped command sent to the plant interface—not a measurement of the hidden filtered actuator output. The target remains observed acceleration minus the nominal direct-actuation model, so actuator-response mismatch is intentionally part of the residual.

All profiles must reference one training dataset SHA. The sweep script fails closed if the dataset digest changes. The internal time constants and metrics must not be compared numerically to the public real-flight Nano-Quadrotor results; those use measured motor angular velocities and a different, standardized model interface.

## Reproduction

From a prepared dual checkout, run:

\`\`\`bash
SYMTROPY_PATH="$PWD" \\
SYMTHAEA_PATH="../symthaea" \\
bash scripts/test-actuator-lag-transfer.sh
\`\`\`

Set \`SYMTROPY_PATH\` and \`SYMTHAEA_PATH\` to the actual checkouts. The script runs Rust unit tests once, then runs the matched residual-model sweep for the four evaluation profiles, validates each receipt, checks the training-data digest across profiles, and writes \`actuator-lag-transfer-results/actuator-lag-transfer-sweep.json\`. The command requires the normal integration dependencies, \`cargo\`, \`rustc\`, and \`jq\`.

This benchmark must pass internal data and code integrity gates before its measurements are interpreted. No internal stress-simulator score substitutes for external real-flight parity or hardware-in-the-loop control evaluation.
