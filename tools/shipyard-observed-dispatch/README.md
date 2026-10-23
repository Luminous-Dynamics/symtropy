# Shipyard observed-history dispatch baseline

This crate supplies four deterministic serial schedule-generation rules:
fixed source order, earliest due date, critical ratio, and least slack. It
builds a plan from the supplied current decision state, retains frozen
commitments, respects operation precedence and resource outages visible by the
decision tick, and can be called again at the next epoch with updated facts.

## Run

    cargo test --locked --offline --manifest-path tools/shipyard-observed-dispatch/Cargo.toml
    cargo run --locked --offline --manifest-path tools/shipyard-observed-dispatch/Cargo.toml --example synthetic_recovery

The example uses synthetic ticks and a synthetic known maintenance window. It
is a smoke-test fixture, not a measured yard result.

## Model boundary

The input contains operations, compatible unary resource kinds, completed work,
frozen commitments, and policy-visible outage intervals. Any outage with an
observed tick after the decision tick is rejected. Unknown future events are
not supplied to the policy interface. The same complete input produces the same
plan regardless of inventory order.

This is a dispatching baseline, not an industrially calibrated optimizer. It
does not model irregular polygon placement, crane/vehicle kinematics, transport
routes, capacity above one, material handling constraints, load charts, weather,
or safety envelopes. Its output must be checked by independent geometry,
resource, precedence, and reservation evaluators before any plan is labelled
feasible.

If the horizon or resource inventory cannot accommodate an operation, the
result is partial; the baseline does not call that mathematical infeasibility.
Conflicts already present in frozen commitments are retained and reported as
FrozenConflict, not silently rewritten. Such plans must not enter the
feasible-performance frontier.

At each new decision epoch, pass observed completions, commitments, and only
the outage observations delivered by that epoch. The caller is responsible for
binding the input snapshot to the immutable experiment digest and for retaining
timeouts, invalid candidates, partial plans, and all evaluation receipts.
