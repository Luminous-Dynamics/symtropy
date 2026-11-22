# DS-TWR and positive NLOS-bias interval validation receipt

**Related PR:** [#6793](https://github.com/Luminous-Dynamics/symtropy/pull/6793)
**Exact head:** `7ce8904b1252967e4ee951096fb32ba2d797ceb5`
**Workflow:** [run #191](https://github.com/Luminous-Dynamics/symtropy/actions/runs/45019930929)
**Status:** passed — equation-level software validation only

## Exact-head validation

- Formatting/whitespace, Clippy and all configured demos passed.
- 117 tests passed, 0 failed: validator 31; navigation research 74; MILUV ingest 12.
- Existing Student-t, adaptive PID, PL trace, zero-failure, mission-risk and temporal-alignment demos passed.
- DS-TWR interval demo passed.
- Generalization schema/split and rights-gated dataset admission CLIs passed.

## DS-TWR interval sensitivity

The synthetic nominal equation uses A=300 ns, B=300 ns, C=300 ns, D=100 ns, giving a nominal range of 29.979246 m.

| Duration-error multiplier | Interval lower | Nominal | Interval upper | Width |
|---:|---:|---:|---:|---:|
| 0 | 29.979246 m | 29.979246 m | 29.979246 m | 0.000000 m |
| 1 | 29.779051 m | 29.979246 m | 30.178775 m | 0.399724 m |
| 2 | 29.578186 m | 29.979246 m | 30.377641 m | 0.799455 m |
| 5 | 28.971540 m | 29.979246 m | 30.970295 m | 1.998755 m |

## Positive NLOS-bias interval composition

Applying a separate positive excess-path interval `[0, 1.5] m` to the multiplier-1 observed range interval gives a possible true range of `28.279051–30.178775 m`.

This is conservative interval subtraction under the model `measured_range = true_range + positive_bias`. It is only valid when the bias sign and bound are justified. The kernel does not classify NLOS or estimate the bias from CIR.

## Claim ceiling

The module consumes the four duration terms in the paper's DS-TWR equation. It does not yet map raw MILUV `tx1/rx1/...` columns into those terms, unwrap counters, establish timestamp tick conversion, or derive the duration error bounds from radio clock-skew. No MILUV payload or Vicon truth was read. These values are synthetic and are not physical range calibration or an integrity guarantee.