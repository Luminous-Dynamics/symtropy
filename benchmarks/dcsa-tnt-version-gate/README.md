# DCSA Track & Trace version gate v0.1

A dependency-free Rust reference gate for the public DCSA Track & Trace 2.2.0 profile and the restricted 3.0.0 migration boundary.

This is deliberately not a complete DCSA event parser, official conformance runner, cryptographic verifier, source-authentication layer, cargo-identity registry, custody engine, or physical delivery verifier.

## Why two versions must coexist

DCSA's 30 September 2026 announcement says Track & Trace 3.0.0 adds IoT and reefer event types to the existing shipment, transport and equipment event domains. DCSA's current Standards Public Reference marks v3.0.0 as available through Member and Partner Access. The announcement says initial distribution is limited to DCSA members and DCSA+ partners, with public release planned for the second half of 2027.

Sources:
- DCSA, Track & Trace 3.0.0: IoT and Reefer Visibility, 30 September 2026: https://dcsa.org/newsroom/track-and-trace-3-0-0-iot-reefer-visibility
- DCSA Standards Public Reference, Track & Trace v3: https://reference.dcsa.org/content/standards/releases/track-and-trace/v3
- DCSA, Implementing the DCSA Track & Trace Standard (v2.2): https://reference.dcsa.org/content/standards/guidelines/implementing-track-and-trace
- DCSA, Track & Trace Interface Standard v2.2 release announcement: https://dcsa.org/newsroom/dcsa-releases-track-trace-interface-standard-version-2-2

The v2.2.0 event-family gate recognizes SHIPMENT, TRANSPORT and EQUIPMENT. IOT and REEFER are explicitly unsupported under that profile; they are not silently coerced into equipment events.

For v3.0.0, the gate requires a caller-declared member/partner access class plus a syntactically valid artifact digest and non-empty scope reference. A successful SUPPORTED_V3_PROFILE_ONLY result means only that the candidate belongs to a locally declared, access-authorized, pinned v3 profile. It does not mean that access rights have been independently authenticated, that the normative artifact bytes have been hashed here, or that DCSA conformance has passed.

The actual v3 schemas, code lists, migration rules and official conformance scenarios must be imported from the authorized normative release before a v3 parser or certification claim is attempted.

## Reference-gate contract

The crate currently checks:
- recognized standard version and event family;
- version-specific family scope;
- explicit caller-declared access class;
- SHA-256-shaped normative artifact pin and non-empty conformance-scope reference;
- source message ID presence;
- separation of source event classifier and eventDateTime roles;
- no promotion of PLN or EST to ACT;
- ACT remains a source assertion, never a physical-realization verdict;
- event message identity remains distinct from a referenced physical subject.

The digest check verifies syntax only. It does not retrieve, hash, authenticate, or validate normative DCSA files.

## Frozen corpus

fixtures/version-gate-v0.1.tsv is the expected-result table consumed by Rust unit tests. It includes all three v2.2 event families; v2.2 rejection of IOT and REEFER; v3 access and artifact-pin gates; unsupported version/family; malformed digest and missing scope.

The source-claim tests cover ACT, PLN, EST, missing event time, missing classifier, unsupported classifier, missing message ID and message-ID/subject-reference separation.

Fixture digest strings are deliberately synthetic sentinels. They are not DCSA artifact digests.

## Run

From the repository root:

    cargo fmt --manifest-path benchmarks/dcsa-tnt-version-gate/Cargo.toml -- --check
    cargo test --manifest-path benchmarks/dcsa-tnt-version-gate/Cargo.toml --locked
    cargo clippy --manifest-path benchmarks/dcsa-tnt-version-gate/Cargo.toml --locked --all-targets -- -D warnings

## Claim ceiling

A passing test suite establishes only that this deterministic reference gate applies its declared version/access/family and source-claim rules to its frozen fixtures. It does not establish DCSA certification or official conformance; full parsing or semantic completeness of either standard version; access rights to restricted v3 normative artifacts; authenticity or physical truth of an event; capture completeness, custody, container location, transformation, mass balance or delivery; interoperability with a live carrier, terminal, TOS, PCS or platform; production readiness or authority to change an operational system.

This is a read-only admission boundary. No provider/terminal writes, event generation, booking, dispatch, inventory change or payment action is performed.