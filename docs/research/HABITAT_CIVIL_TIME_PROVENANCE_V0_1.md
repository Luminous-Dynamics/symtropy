# Habitat Civil-Time Provenance v0.1

**Status:** strict metadata/digest verifier and synthetic negative controls; not a time-zone conversion engine.
**Implementation:** [habitat-calendar-oracle/src/provenance.rs](../../tools/habitat-calendar-oracle/src/provenance.rs)
**Fixture:** [HABITAT_CALENDAR_PROVENANCE_V0_1.json](fixtures/HABITAT_CALENDAR_PROVENANCE_V0_1.json)

## Why this layer exists

Calendar scheduling is only reproducible when its civil-time interpretation is reproducible. "Work weekdays, 08:00–17:00" does not completely define physical instants without the relevant time zone, the exact rule-data release, a finite expansion horizon, and policies for local wall-clock instants that are repeated or skipped by clock transitions.

The IANA time-zone database records regional civil-time rules and can change when political bodies change local time or daylight-saving rules. Releases are versioned; the official release archive currently lists [2026e, dated September 29, 2026](https://www.iana.org/time-zones/releases). A version label alone is not sufficient provenance for an artifact: the consumed bytes and derived expansion should also be pinned and recorded.

RFC 5545 distinguishes floating local date-times from UTC or local date-times associated with a time zone. It also specifies behavior for repeated and nonexistent local times: repeated local date-times refer to the first occurrence, while nonexistent local date-times use the UTC offset before the gap. See the [RFC Editor's RFC 5545 text](https://www.rfc-editor.org/rfc/rfc5545.html). A scheduling application can choose a stricter policy (for example, reject both ambiguous and nonexistent times), but the policy must be explicit and stable.

## What is verified

The Rust module requires, validates and binds together:

- calendar source URI and revision plus source payload SHA-256;
- IANA-style region identifier (not a raw fixed offset masquerading as a region);
- explicit time-zone database release and hash of the consumed rule-data payload;
- explicit local-wall-clock interpretation, finite UTC expansion horizon, and ambiguity/nonexistence policies;
- parser/compiler version, expanded-interval payload SHA-256 and nonzero interval count;
- strict schema behavior, including rejection of unknown fields.

The frozen synthetic fixture includes two positive metadata cases and eight negative controls: missing time-zone database version, floating local time, fixed offset in place of a regional zone, corrupted source digest, absent horizon boundary, missing repeated-time policy, corrupt expanded-interval digest, and an unreviewed unknown field.

## What is deliberately *not* verified

A metadata and digest pass is labelled `METADATA_PROVENANCE_PASS_NOT_TZDB_CONFORMANCE`. It does not prove that a referenced zone exists in the pinned release; that the timezone database bytes correspond to their release label; that any UTC interval was correctly calculated; or that recurring intervals account for every daylight-saving transition, leap second, jurisdictional update, holiday or workplace-specific rule. The fixture's payloads and hashes are deliberately synthetic.

Before a calendar compiler can claim conformance, implement and differentially test the actual recurrence-to-UTC expansion against an independent library/data build with an exact time-zone database release and transition-file hashes. Required tests include:
- DST spring-forward nonexistent times and fall-back repeated times;
- jurisdictions that recently change, remove or add seasonal clock rules;
- calendar rules spanning a transition and rules spanning a rule-data update;
- fixed UTC instants versus local recurring shift rules versus floating local times;
- date-only statutory holidays, overnight shifts, lunch intervals, closures and overlapping exceptions;
- deterministic output under stable input bytes, with explicit invalidation when source, rules, compiler or horizon changes.

Never silently recompile an existing schedule under a new time-zone database. Emit a new calendar artifact, report the changed UTC intervals and impacted schedule assignments, and require a deliberate revalidation step. Store both the original civil-time rule and expanded UTC intervals so the result can be traced and recomputed.

## Commands and acceptance

From repository root:

- `cargo fmt --manifest-path tools/habitat-calendar-oracle/Cargo.toml -- --check`
- `cargo test --locked --manifest-path tools/habitat-calendar-oracle/Cargo.toml`

Targets: both accepted metadata templates validate; all eight negative controls reject; every payload digest matches its exact payload bytes; zero unspecified DST policy; zero silently accepted floating-time rules; zero claim of time-zone conversion correctness from metadata alone.

These tests are a schema/provenance contract, not an achieved solver-performance score or evidence of physical work, legal compliance or engineering safety.
