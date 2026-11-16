# AeroWeft Experiment Execution Fidelity Oracle

Dependency-free, synthetic-first Rust verifier for comparing a frozen Track V experiment plan with an actual run record and its time-segmented evidence.

It verifies exact candidate/policy/design/procedure identity, actual aircraft/model/control/sensor/solver/reference scope, authority receipt scope and validity, realized operating strata, deviation/abort state, telemetry segment identity, stream sequencing, interval coverage and minimum sample counts.

Positive status is `EXECUTION_CONFORMING` only when every planned data window is fully captured under the exact measurement/time/frame/datum contract and no material deviation or abort exists. The strongest status is still not physical validation or operational permission.

The crate does not run tests, control aircraft, authenticate receipts or raw-data bytes, evaluate signer authority, calculate model residuals, or certify aviation operations. Hash strings are syntax-checked only. Real provenance and authority remain Track Q/Mycelix owners.
