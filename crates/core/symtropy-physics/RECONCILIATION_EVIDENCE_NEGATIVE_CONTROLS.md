# Reconciliation Evidence Negative Controls

This companion note records the adversarial cases that must remain rejected by the checked reconciliation evidence surface.

- A NaN residual is invalid evidence; it must not disappear from max/count reporting because comparisons against NaN return false.
- A stored residual that does not equal `measured_delta - ledger_delta` is invalid even when every scalar is finite.
- An appearing reservoir may not simultaneously carry a numeric measured delta or residual.
- A disappearing reservoir may not simultaneously carry a numeric measured delta or residual.
- A stable reservoir may not be labeled as appeared/disappeared.
- Presence-change metadata must refer to exactly one tracked reservoir entry.
- Duplicate tracked reservoirs are invalid.
- Duplicate untracked ports are invalid.
- External ports may not be classified as untracked internal state.
- A tracked port may not also appear in the untracked-port set.
- Boundary closure must recompute from the stored initial total, final total, and external flow.
- NaN, infinity, or negative tolerance values are evidence errors, not policy values.

These controls are intended to prevent a serialized or manually altered audit from producing a more favorable validation result than the original constructor would have allowed.
