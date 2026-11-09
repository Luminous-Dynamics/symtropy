# Energy Service Capability Profile (ESCP) v0

**Status:** declarative interoperability profile and exact compatibility matcher. It is not a charging protocol, scheduler, power controller, safety interlock or permit.

## Purpose

ESCP lets a provider advertise a versioned energy-service offer independently of product name or proprietary device API. A receiver expresses a demand descriptor; the matcher checks exact bearer, field regime, hardware/geometry/frame, motion state, environment and session mode plus digest-bound evidence-role coverage.

## Profile transport

One offer evidence row per receipt. Multiple rows with the same profile ID repeat identical profile metadata and contribute separate evidence roles. Exact TSV header:

    profile_id provider_id profile_version bearer field_regime receiver_hardware_sha256 receiver_geometry_sha256 coordinate_frame_sha256 supported_motion_states supported_environment_strata session_modes evidence_role receipt_id receipt_sha256 evidence_class review_status disposition valid_until_us

Bearer: CONDUCTIVE, IPT, CPT, BATTERY_EXCHANGE, RF_MICROWAVE, OPTICAL, POWER_LINE.

Field regime: CONTACT, NEAR_FIELD, FAR_FIELD, ENERGY_CARRIER, NOT_APPLICABLE.

Motion state and session mode are explicit, never inferred from bearer. Supported sets are semicolon-delimited exact identifiers. A profile cannot declare an unknown wildcard.

Evidence class, review status and disposition are separate. Review status is NOT_REVIEWED or INDEPENDENTLY_REVIEWED; it does not replace the measurement evidence class. An independent review is represented by a distinct INDEPENDENT_REVIEW receipt role. DECLARED_ONLY is not evidence-backed. Blocking dispositions include AMBIGUOUS, STALE, OUT_OF_DOMAIN, UNKNOWN, REVOKED, NOT_ADMITTED and NOT_EVALUATED. An issuer declaration does not verify its own receipt digest; artifact resolution and independent verification remain with upstream owners and GroundAirBench.

## Receiver demand descriptor

Exact TSV header:

    request_id allowed_bearers allowed_field_regimes receiver_hardware_sha256 receiver_geometry_sha256 coordinate_frame_sha256 motion_state environment_stratum session_mode required_evidence_roles minimum_evidence_tier require_independent_review decision_time_us

A match requires exact receiver, geometry and frame digests, a declared supported motion and environment, a supported session mode, and valid nonblocking evidence for every required role. The minimum evidence tier is the weakest required-role measurement tier (SIMULATED, BENCH_MEASURED or FIELD_MEASURED). Independent review is a separate optional requirement, not a stronger measurement tier.

If a profile only has DECLARED_ONLY evidence for a required role, its output is DECLARED_ONLY, never MATCH_*. A missing required role, stale receipt, blocked disposition, mismatch or unsupported state produces NO_MATCH.

## Output classes

- NO_MATCH: at least one required constraint/evidence predicate failed.
- DECLARED_ONLY: declarative compatibility exists but evidence is absent/declared-only or below the requested tier.
- MATCH_SYNTHETIC_ONLY, MATCH_SIMULATED, MATCH_BENCH, MATCH_FIELD: every required role has nonblocking evidence and the weakest measurement tier meets the request.

INDEPENDENTLY_REVIEWED is orthogonal; if requested, a distinct INDEPENDENT_REVIEW receipt must have review_status INDEPENDENTLY_REVIEWED and a non-blocking qualified disposition. Match output never means operational authorization.

## Claim ceiling

A pass establishes only exact profile/demand matching and evidence reference semantics on supplied rows. It does not prove issuer honesty, resolve receipt artifact bytes, establish hardware interoperability, prove transfer performance or authorize connection/energization/flight.

## Run

    cargo test --locked --manifest-path tools/energy-service-capability-profile/Cargo.toml
    cargo run --locked --manifest-path tools/energy-service-capability-profile/Cargo.toml -- research/energy-service-capability-profile-v0/synthetic-offers.tsv research/energy-service-capability-profile-v0/docked-receiver-request.tsv
