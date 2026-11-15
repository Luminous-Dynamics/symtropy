# Megastructure Review Gate v0.1

**Status:** typed policy reference; not a design, safety, rights or community approval system  
**Parent:** [Symtropy #5384 — MEGASTRUCTURE-DESIGN-QUALITY-001](https://github.com/Luminous-Dynamics/symtropy/issues/5384)  
**Implementation task:** [Symtropy #5451 — MEGASTRUCTURE-REVIEW-GATE-001](https://github.com/Luminous-Dynamics/symtropy/issues/5451)  
**Policy and synthetic corpus:** [Symtropy PR #5418](https://github.com/Luminous-Dynamics/symtropy/pull/5418)

## Purpose

This standalone, dependency-free Rust crate executes the policy precedence described in the megastructure design-quality specification.

It checks whether:
- the seven hard constraints have passed;
- a design-review pack covers five non-universal quality dimensions;
- required views, stakeholder coverage and dissent handling are recorded;
- non-zero mass, power, thermal or maintenance deltas have a matched trade-study evidence reference;
- an emergency deferral has a reason, future expiry and accountable improvement owner.

It does not rank architecture styles, rate how beautiful a design is, decide what a community should prefer, or make a weighted average of safety and taste.

## Non-compensatory decision order

1. Invalid identities/dates/non-finite physical deltas -> `InvalidEnvelope`.
2. Any hard-gate fail, unknown or unresolved not-applicable -> `BlockedHardGate`.
3. Valid, accountable emergency deferral with all hard gates passed and minimum human-dignity review -> `TimeLimitedDeferment`.
4. Missing views/dimensions/stakeholder coverage/dissent handling -> `DesignReviewIncomplete`.
5. Non-zero physical lifecycle delta without a trade-study evidence ID -> `TradeStudyRequired`.
6. Otherwise -> `AdmissibleForMulticriteriaReview`.

"Admissible for multicriteria review" means only that the typed review envelope passed this policy reference. It is not a construction permit, environmental clearance, rights decision, consent finding, safety certification, human-rating decision or design approval.

## No universal beauty score

The input has no aesthetic preference number, scalar beauty score or style-ranking field. The five dimension flags only represent whether the review evidence has addressed:
- formal coherence and legibility;
- human experience and dignity;
- material honesty and repairability;
- ecological, planetary and orbital relationship;
- pluralism, place and cultural authorship.

The flags do not claim that a design is objectively beautiful. Stakeholder preference distributions and dissent remain separate evidence.

## Determinism and time

The caller supplies an ISO calendar evaluation date. The evaluator does not read the machine clock. Deferral expiry is therefore replayable for the same input and evaluation date.

## Run

```sh
cargo +1.96.0 fmt --manifest-path tools/megastructure-review-gate/Cargo.toml --all -- --check
cargo +1.96.0 clippy --manifest-path tools/megastructure-review-gate/Cargo.toml --locked --all-targets -- -D warnings
cargo +1.96.0 test --manifest-path tools/megastructure-review-gate/Cargo.toml --locked --all-targets
```

The crate is a separate Cargo workspace and has no external dependencies.

## Synthetic test coverage

The test suite covers the eight case families in the policy corpus:
- unsafe but polished;
- majority preference with unresolved rights;
- missing review evidence;
- pluralistic completed review;
- lifecycle deltas requiring trade study;
- time-limited emergency deferral;
- beautiful but unmaintainable;
- unresolved ecological/environment evidence.

Additional negative tests cover unresolved not-applicable gates, expired deferrals, non-finite physical deltas, malformed dates and missing deferment ownership.

The typed test constructors are a Rust executable reference for policy behavior. The companion JSON corpus and schema remain the machine-readable cross-project fixtures; no JSON parser or wire adapter is added in this v0.1 slice.

## Claim ceiling

This crate enforces review-process precedence only. It does not assess structural physics, accessibility compliance, ecological impact, actual community consent, legal rights, architectural merit, safety, sustainability or megastructure feasibility. Human and professional authorities remain necessary.
