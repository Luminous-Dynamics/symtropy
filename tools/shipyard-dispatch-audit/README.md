# Shipyard independent dispatch audit

This dependency-free crate independently checks the abstract scheduling
constraints emitted by a dispatch policy. It is intentionally separate from
the policy crate: it consumes plain operation/resource/history/assignment
records and recomputes coverage, timing, precedence, unary-resource overlap,
known outage conflicts, and exact preservation of frozen commitments.

## Run

    cargo test --locked --offline --manifest-path tools/shipyard-dispatch-audit/Cargo.toml

## What a passing report means

A passing report means only that the schedule satisfies this crate's abstract
operation/resource/time model for the supplied state and event truth. The
evaluator may know disturbances that were not yet visible to the policy; a
policy cannot access these evaluator inputs. If a schedule overlaps a physically
effective outage, the audit rejects it even if the policy had not observed the
outage. This is a scoring rule, not proof the policy was clairvoyant.

The verifier does not model polygon geometry, blocking, transport paths,
crane/vehicle kinematics, capacity greater than one, lifting charts, environment,
or safety envelopes. It is not a proof of shipyard feasibility and must be
composed with independent spatial and physical validators. Infeasibility is
never inferred from a heuristic's partial plan.
