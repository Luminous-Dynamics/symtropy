# Transport Policy Resolver Contract v0.1

**Status:** Rust resolver prototype with adversarial unit tests; not yet CI-verified or connected to the provisional Panama CSV.  
**Implementation:** `crates/domains/symtropy-transport-optimization/src/policy_applicability.rs`  
**Tracking:** [TRANSPORT-POLICY-001 #1681](https://github.com/Luminous-Dynamics/symtropy/issues/1681) and [source admission #1677](https://github.com/Luminous-Dynamics/symtropy/issues/1677)  
**Current CSV:** [TRANSPORT_PANAMA_POLICY_GOLDEN_CASES_V0_1.csv](TRANSPORT_PANAMA_POLICY_GOLDEN_CASES_V0_1.csv)

## Paradigm shift

A transport rule is not a timeless boolean such as "slot available". A replayed decision depends on at least three clocks and a scope: when the rule was published, when the source entered the replay corpus, and when the rule was effective for the event under evaluation. A rule can be published but not yet effective; effective for a past event but published after the historical decision; or public but absent from the evidence corpus at the cutoff being replayed.

The resolver therefore separates **knowledge time**, **event/effective time**, and **rule scope**. It resolves a version of a named rule family. It does not allocate a reservation, reconstruct private awards or bids, prove physical transit feasibility, or authorize dispatch.

## Resolver contract

| Field / rule | Contract |
|---|---|
| `family_id` | Stable logical rule family, for example a named cancellation rule |
| `version_id` | Immutable identity for one version |
| `published_at` | Must be at or before the replay knowledge cutoff |
| `recorded_at` | Source must have entered this evidence corpus by the replay cutoff |
| `effective_from` | Inclusive start of the rule's valid-time interval |
| `effective_until` | Exclusive end; absent means open-ended |
| `scope` | Optional lock system, market segment, and booking period predicates; a missing query dimension never matches a rule that requires it |
| `supersedes` | Explicit references to same-family predecessor versions; timestamps alone never supersede |
| evidence state | A qualified result requires a 64-hex-character source SHA-256 and exact source locator. Unqualified evidence is surfaced, never treated as an affirmative rule decision |

All times are UTC Unix seconds at this API boundary. Source adapters must do explicit parsing and time-zone conversion before building the rule book; the resolver intentionally does not guess the Canal's local-time conventions.

### Outcomes fail closed

- `Resolved`: exactly one active, scoped, temporally valid version remains and its evidence is qualified.
- `UnqualifiedEvidence`: one version matches but source admission/use qualification is incomplete.
- `Ambiguous`: overlapping candidates remain and their relationship is not expressed by explicit supersession.
- `UnknownFamily`, `NoKnownVersion`, `OutOfScope`, and `NoEffectiveVersion`: distinct reasons no affirmative version was resolved.

None of the non-resolved states means "allow". The downstream evaluator must map each into an explicit unknown/rejected/unsupported result; it must not convert absence of a rule into eligibility.

The rule book constructor rejects duplicate version IDs, malformed effective intervals, qualified evidence without provenance, unknown or cross-family supersession references, and supersession cycles. Query resolution is order-independent over rule insertion order. A scope-specific amendment cannot retire a predecessor when that amendment does not match the query's scope.

## Unit and adversarial tests

The initial Rust unit tests cover:

1. publication-time lookahead;
2. evidence-corpus ingestion later than publication;
3. inclusive start / exclusive end boundaries distinct from publication;
4. Panamax versus Neopanamax scope isolation;
5. overlapping rule versions without explicit supersession;
6. unqualified source evidence;
7. scope filtering before supersession;
8. required source hash and clause locator;
9. unknown supersession references;
10. supersession cycles;
11. order-independent resolution.

A separate integration test now parses the 19-row CSV and checks its 17-column header, row widths, unique IDs, date-field shape, official source URLs, explicit nonclaims, and selected policy-scope / temporal / cancellation negative controls. This is **structural integrity only**: the CSV cases are not yet semantically executed against policy predicates, and the test does not qualify source rights or transcription correctness. Semantic integration requires corpus-ingestion time, evidence qualification, exact clause provenance, and a reviewed mapping from each case to a rule predicate. Do not mark the CSV corpus "executable" until those links exist and fixtures are manually checked against admitted source artifacts.

## Benchmark ladder

| Gate | Target | Evidence required |
|---|---|---|
| R0 — resolver invariants | 100% of hand-authored boundary/adversarial tests; deterministic results under all input permutations | Exact source commit, toolchain, test output, and immutable fixture manifest |
| R1 — public golden cases | 100% agreement with all manually reviewed clause-level cases in the declared rule domain | Original source bytes and SHA-256, page/clause locators, use-rights review, independent case review |
| R2 — bitemporal fuzz/replay | Zero rule application before publication or corpus-ingestion cutoff; zero application outside effective interval/scope; zero implicit supersession | Deterministic generated sequences with fixed seeds, saved failing counterexamples, and an independent slow reference selector |
| R3 — ambiguity and unknowns | 100% of incomplete/conflicting/unqualified records yield explicit non-resolved outcomes; zero fail-open conversions | Negative-control corpus and downstream decision mapping tests |
| R4 — input scale | Report p50/p95/max time, memory where measured, and versions/families examined at 1, 10, 100, 1,000, and 10,000 rule versions | Pinned hardware/toolchain and exact corpus manifest; establish latency budgets only after a repeatable baseline |
| R5 — policy-to-transport integration | Zero policy/physical-capacity conflation; zero inferred actual awards; no route claims from eligibility alone | Separate policy, physical resource/time-window, and award/dispatch interfaces with negative integration tests |

A passing resolver unit suite would prove only its finite version-selection contract. It would not establish completeness of a policy transcription, a booking outcome, empirical transit-time prediction, transport-system optimality, or a deployable allocation system.

## Research and benchmark context

- [Vehicle routing with time-dependent travel times: Theory, practice, and benchmarks](https://www.sciencedirect.com/science/article/pii/S1572528624000276) develops dedicated time-dependent routing methods and benchmark instances. Symtropy should benchmark static and time-dependent cases separately rather than treating a time-invariant edge cost as an empirical travel-time model.
- [BonnTour time-dependent VRP benchmarks](https://bonndata.uni-bonn.de/catalog/datasets/10.60507/FK2/X22BKR/) has explicit CC BY-NC 4.0 terms. Review those restrictions before copying instances into any distributable or commercial benchmark corpus; citation does not grant reuse rights.
- The [Panama Canal official advisory archive](https://pancanal.com/en/maritime-services/advisory-to-shipping/) is the discovery point for changing public rules. Individual original artifacts still need to be acquired, hash-frozen, clause-located, and reviewed before they can qualify a resolver result.

## Next implementation slices

1. Add a checked parser and schema for immutable source manifests; preserve URL, acquisition time, SHA-256, exact page/table/paragraph locator, rights state, and supersession links.
2. Extend synthetic case records with corpus-ingestion time, rule-family/version IDs, and explicit scope; then build a table-driven evaluator that compares specific clause outcomes, not "actual allocation".
3. Build an independently implemented reference selector and property-based temporal/scope permutations.
4. Only then integrate reviewed policy predicates with the separate physical-resource/time-window model.

Current source records are provisional. The existing Panama case CSV still states that original source bytes are not yet acquired/hash-frozen and rights review is incomplete; the resolver's qualified status is a control, not a claim that those sources have been qualified.
