# PHRI MULTIRATE ADAPTATION 001 — proposal admission gate

Status: deterministic, dependency-free admission/reference primitive only. It is not a controller, scheduler, actuator interface or safety supervisor.

Parent issue: https://github.com/Luminous-Dynamics/symtropy/issues/4785

## Architecture

The research programme separates:

1. Slow wearer-governed task and assistance preferences.
2. Medium-rate optimizer/NMPC proposals that may complete asynchronously.
3. A fast local layer that performs a bounded synchronous admission check and independently enforces physical limits.

This crate implements only the proposal admission/expiry contract between layers 1/2 and the fast layer. The fast layer does not wait for an optimizer. It examines a completed proposal only when one is available.

## Admission conditions

A proposal is admitted only when:

- its proposal sequence equals the latest sequence issued by the proposal source;
- its sequence is newer than the last admitted proposal;
- its clock and boot epoch match the current local context;
- its wearer-agency epoch matches the current, non-revoked epoch;
- it has not expired and its issue timestamp is not in the future;
- the source observation is no older than the declared maximum age;
- the current state/configuration revision matches the proposal's source revision;
- proposal, parameter and optimizer-receipt identities are present.

A newer issued proposal supersedes an older result even when the older optimizer finishes later. A wearer cancellation/stop increments the agency epoch and invalidates all outstanding proposals from the prior epoch.

The gate is synchronous, fixed-size and allocation-free. Rejected proposals do not mutate admission state. The application revalidation method checks expiry, state revision, latest proposal sequence, observation age and agency epoch again immediately before a downstream layer considers applying a parameter update.

## Claim boundary

- Admission means only that this envelope passed these local freshness/supersession checks.
- It does not prove that optimizer mathematics is correct or that a proposal is physically safe.
- A receipt is not actuator permission. A distinct local safety authority must still verify force/pressure/energy/thermal/actuator envelopes.
- The caller must provide a trustworthy, monotonic latest-issued sequence and current state/agency context.
- This crate does not schedule threads, prove worst-case execution time, handle concurrent memory access, or establish a real-time guarantee.
- A physical stop path must remain independently available and cannot depend on this crate or the optimizer.

## Regression coverage

Authored deterministic tests cover valid admission, late old proposal rejection, expiry, future issue time, state-revision mismatch, stale source observation, clock/boot mismatch, replay rejection, wearer revocation, application-time revalidation, supersession between admission and application, invalid identities and no state mutation on rejection.

Tests and the pinned Rust 1.96 format/test/strict-Clippy workflow are authored; no passing run is claimed.
