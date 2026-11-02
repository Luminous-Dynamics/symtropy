# SU2 convergence protocol preflight v0.2

## Why this exists

SU2 has multiple convergence mechanisms. A single configured field is not a complete description of the stopping protocol.

The Solver Setup documentation states that both `CONV_FIELD` and `CONV_WINDOW_FIELD` accept lists and that all configured fields must meet their respective stopping criteria. The time-dependent coefficient path uses `WINDOW_CAUCHY_CRIT`, `CONV_WINDOW_FIELD`, `WINDOW_START_ITER`, `CONV_WINDOW_STARTITER`, `CONV_WINDOW_CAUCHY_ELEMS`, `CONV_WINDOW_CAUCHY_EPS` and `WINDOW_FUNCTION`.

References:
- https://su2code.github.io/_docs_v7/Solver-Setup/
- https://su2code.github.io/docs_v7/Setup-Guide-Unsteady-DES/
- https://github.com/su2code/SU2/blob/master/config_template.cfg

The exact installed SU2 binary remains authoritative for dry-run field discovery. The documentation is used to understand configuration semantics, not as a substitute for a pinned v8.x binary test.

## Protocol object

`Su2ConvergenceProtocol` captures:

- All configured steady fields from `CONV_FIELD`.
- `CONV_STARTITER`, if explicitly supplied.
- `CONV_RESIDUAL_MINVAL`, if explicitly supplied.
- `CONV_CAUCHY_EPS` and `CONV_CAUCHY_ELEMS`, if explicitly supplied.
- Whether windowed Cauchy convergence is explicitly enabled or disabled.
- When enabled: every `CONV_WINDOW_FIELD`, the averaging start, convergence-monitor start, window element count, epsilon and window function.

The configuration's optional steady settings remain optional in the report. Their omission is not silently converted into a claimed effective default. The exact config bytes and executable digest are retained.

## Strict preflight rules

1. `CONV_FIELD` must be present, contain at least one unique field, and every field must occur in the exact executable/configuration `SU2_CFD -d` inventory.
2. `WINDOW_CAUCHY_CRIT` must be explicit `YES` or `NO`; an omitted or malformed flag is rejected rather than inferred from a template/default.
3. When window convergence is enabled, `CONV_WINDOW_FIELD` must contain unique fields and every field must occur in the same dry-run inventory.
4. When window convergence is enabled, `WINDOW_START_ITER`, `CONV_WINDOW_STARTITER`, `CONV_WINDOW_CAUCHY_ELEMS`, `CONV_WINDOW_CAUCHY_EPS` and `WINDOW_FUNCTION` must be explicit and valid.
5. The window function must be one of `SQUARE`, `HANN`, `HANN_SQUARE` or `BUMP`.
6. If `RESTART_ITER` is explicitly supplied, `WINDOW_START_ITER` may not precede it.
7. The configuration's active steady and time-window fields are included in the canonical preflight digest.

## Production history and independent-screen rules

A successful preflight is only configuration-compatibility evidence. It does not show that a field was emitted, that its series converged, or that the calculation is valid.

Before a run bundle is admitted:

- Every active `CONV_FIELD` and `CONV_WINDOW_FIELD` must be required and imported from the exact production history bytes.
- Every active convergence field must have a passing independent post-run screen criterion.
- A time-window field must have a terminal monitor-stability screen (range, mean drift and slope), not just a passing residual criterion.
- The production screen remains a separate computation; it is not asserted to reproduce SU2's internal Cauchy stopping algorithm.
- The bundle remains unsigned and does not establish grid convergence or experimental validation.

For a window field such as `TAVG_DRAG`, this prevents the run from being admitted when the solver was configured to stop on that quantity but the evidence chain never imported or screened it.

## Unsteady inner-loop qualification

The native time-window success message does not independently prove that the inner solver converged at every outer time step. SU2's unsteady driver can proceed when the inner solver converges or reaches its configured inner-iteration cap, then evaluate windowed time-averaged convergence.

The verified run bundle therefore adds the separate [`SU2_UNSTEADY_INNER_SCREEN_V0_1.md`](./SU2_UNSTEADY_INNER_SCREEN_V0_1.md) contract. The current implementation supports residual-based `CONV_FIELD` values only. It requires `TIME_ITER`, `INNER_ITER`, an explicit `CONV_RESIDUAL_MINVAL`, contiguous saved time steps, and a complete history interval covering the configured window convergence start.

Every saved time step must meet the residual threshold. Missing time-step records or any failed residual reject bundle admission. This is an outer-time-step endpoint screen; it does not reconstruct all inner-iteration residual traces or claim per-zone convergence for all multizone configurations.

## Evidence ceiling

`receipt_bound_su2_dry_run_multi_field_steady_and_explicit_time_window_configuration_discovery_only_not_history_convergence_or_validation`

This protocol records the configured stopping policy and verifies that the exact dry-run inventory recognizes every active field. It does not prove the chosen thresholds are scientifically sufficient, that a periodic/statistically stationary regime exists, that the time window is long enough, or that the solution is grid-independent.

## Synthetic qualification

The unit and CLI fixtures cover:
- Multiple `CONV_FIELD` values.
- `WINDOW_CAUCHY_CRIT=NO` as an explicit steady-mode declaration.
- Enabled time-window convergence with multiple fields.
- Missing time-window configuration.
- Invalid window/restart ordering.
- A dry-run inventory that omits one or more configured fields.

All of these remain fake-solver software-contract tests. They are not actual SU2 runs.
