# Energy Service Capability Profile (ESCP) v0

**Status:** declarative interoperability profile and exact compatibility matcher. It is not a charging protocol, scheduler, power controller, safety interlock or permit.

## Purpose

ESCP lets a provider advertise a versioned energy-service offer independently of product name or proprietary device API. A receiver expresses a demand descriptor; the matcher checks exact bearer, field regime, hardware/geometry/frame, motion state, environment and session mode plus digest-bound evidence-role coverage.

## Profile transport

One offer evidence row per receipt. Multiple rows with the same profile ID repeat identical profile metadata and contribute separate evidence roles. Exact TSV header:

    profile_id provider_id profile_version bearer field_regime receiver_hardware_sha256 receiver_geometry_sha256 coordinate_frame_sha256 supported_motion_states supported_environment_strata session_modes evidence_role receipt_id receipt_sha256 evidence_class review_status disposition valid_until_us claim_ceiling schema_version

Every row repeats the profile's exact claim_ceiling and schema_version. v0 accepts schema_version=1 and profile_version=1 only. The claim ceiling is a declarative non-authority statement carried with the profile.

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


## Relationship to existing interoperability standards

ESCP is a **capability/evidence registry layer**, not a replacement for normative WPT standards or certification.

- SAE J2954_202408 covers stationary WPT interoperability, safety, performance, alignment and test methods for light-duty road EVs. It defines three levels up to 11 kVA and references companion communications/use-case standards SAE J2847/6 and J2836/6. Its scope is stationary light-duty road vehicles; UAV applicability is not established by this standard. https://saemobilus.sae.org/standards/j2954_202408-wireless-power-transfer-light-duty-plug-electric-vehicles-alignment-methodology
- IEC PAS 61980-4:2025 covers stationary high-power magnetic-field WPT for electric road vehicles, including positioning, electrical safety, basic communication and EMC. It explicitly excludes vehicles primarily designed for off-road use; it should not be treated as a UAV approval path. https://webstore.iec.ch/en/publication/67337
- IEC PAS 61980-5:2024 covers dynamic magnetic-field WPT for electric road vehicles in motion and related safety/EMC requirements. It does not itself establish airborne-UAV applicability. https://webstore.iec.ch/en/publication/64273

A service profile may reference an applicability assessment receipt from #4141 / PR #4174. The profile must preserve `APPLICABILITY_UNKNOWN` as unknown; a standard's existence, title or frequency reference is not permission to operate. Any UAV-specific standardization proposal should begin with a scoped standards gap analysis and regulator/standards-body engagement.

## Research positioning

The novel hypothesis is not "an interoperability standard is missing." Mature road-EV WPT standards already define interoperability and safety within their scope. The gap is whether a **proof-carrying, multi-bearer energy-service declaration** can safely compose receiver revision, field regime, motion/environment domain, calibrated pose evidence, session lifecycle, source-to-battery metrology, maintenance/lifecycle evidence and regulatory applicability across heterogeneous UAV service architectures. ESCP v0 only validates the declaration structure and matching rules; it does not prove the physical interoperability hypothesis.


## Signed statements and transparency are delegated to existing owners

ESCP v0 is unsigned. It does not invent a new signing or transparency protocol.

The intended domain payload maps to the existing SCITT scientific statement workstream #3289 / SIM-REL-043 and scoped Xenia receipt-attestation workstream #1255. A future signed ESCP statement should bind the exact canonical profile digest, provider/station subject, schema version, referenced evidence receipt IDs/digests, expiry, claim ceiling and predecessor/supersession identity. The exact statement can then be registered with one or more SCITT transparency services.

Keep these claims separate:
- issuer signature: the identified issuer signed these exact profile bytes;
- transparency receipt: the signed statement was registered/included in the specified verifiable data structure;
- evidence verifier: the referenced physical/measurement artifacts independently validate under their own semantics;
- ESCP compatibility: the exact declared receiver demand matches the profile;
- operational permission: remains a separate authority decision.

Neither a signature nor a transparency receipt proves the physical capability claim true. A signed stale, ambiguous, revoked, out-of-domain or not-admitted statement remains blocked. Do not add an arbitrary `sign(hash)` endpoint; reuse the typed, scoped attestation and offline-verification owners.
