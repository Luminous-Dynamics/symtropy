# Orbital Qualification Assurance Bundle Benchmark v0.1

**Status:** synthetic structural reference implementation; exact-head hosted validation pending.  
**Owner:** [ORBITAL-QUALIFICATION-ASSURANCE-BUNDLE-001 #6595](https://github.com/Luminous-Dynamics/symtropy/issues/6595).  
**Claim ceiling:** synthetic assurance-graph structure only.  
**Authority:** always false.

## Why this layer exists

The orbital interface programme now has independent evidence producers:

| Producer | Claim family | What it can say |
|---|---|---|
| #6485 | Lifecycle qualification | Lifecycle state under the declared lifecycle contract |
| #6516 | Event history integrity | Exact-byte event replay and reducer outcome |
| #6542 | Test-matrix adequacy | Whether declared conditions and interactions were exercised |
| #6553 | Mission-success probability | One-shot mission success under an article-level Bernoulli model |
| #6593 | Constant failure intensity | Failure-event rate per operating exposure under a homogeneous Poisson model |
| #6519 | Evidence custody / authority adapter | Signer, role, receipt identity and receipt-time predicates |
| #6256/#6303 | Physical observation / metrology | Ground-test and calibration evidence |

A lifecycle result cannot prove a reliability target. Coverage cannot substitute for physical endurance. Mission-success probability cannot silently become a failure intensity. A signature does not prove physical truth. A complete review graph cannot grant mission authority.

## Architectural shift

Treat each engineering result as a typed claim interface, not a PDF attachment or green check. Every claim carries its claim type, estimand and unit where relevant; exact subject, configuration, operating mode, mission profile and environment; validity interval; separate effective-time and knowledge-time cutoffs; claim state and assumptions; a validation-domain declaration where applicable; and an exact receipt identity.

The root bundle is complete only when every required typed claim is present, mandatory, supported, in exact scope, current at both cutoffs, bound to its case-level pin, and connected to the review root. Unresolved mandatory defeaters, contradictions, stale dependencies and missing authority decisions remain blockers. No weighted score or compensating positive claim can erase a negative dependency.

This makes evidence composition behave like a typed build system: incompatible claim families do not link, scope drift breaks dependencies, stale dependencies require requalification, and the receipt records exactly which declared inputs were used.

## V0.1 model

### Scope

All claim scopes must exactly equal the case scope: subject/article/assembly, configuration, operating mode, mission/service profile, environment profile, and validity start/end. This is intentionally conservative. Future profiles can add validated containment/subset relations; v0.1 must not infer component applicability to an assembly or one environment to another.

### Time

The case has separate effective and knowledge cutoffs. The effective cutoff must be inside the case validity interval; evidence validity must cover the effective cutoff; evidence recorded time must not exceed the knowledge cutoff; and knowledge cutoff must not precede effective cutoff. This catches stale/late declarations, but does not establish trusted clocks or authenticate timestamps. A missing change feed is not evidence that no change occurred.

### Typed statistical claims

A mission-success claim requires estimand mission_success_probability, unit probability, and model profile bernoulli_single_mission.

A constant-intensity claim requires estimand constant_failure_intensity, unit events_per_million_operating_hours, and model profile homogeneous_poisson_constant_intensity.

A derived_from edge between these claim types is rejected. No conversion exists in v0.1. Any future conversion needs a separately versioned mission-time model, explicit hazard/exposure assumptions, a validation domain and an independent oracle.

### Evidence-class checks

- Physical-observation and calibration claims require declared physical or bench evidence.
- Evidence-custody claims require software or authority evidence.
- External-authority decisions require authority evidence.

These are metadata checks, not cryptographic verification. The evaluator does not fetch source artifacts, verify raw bytes against their digests, verify signatures, validate credentials, or prove physical truth. Receipt pins are consistency anchors supplied by the case, not trust roots.

### Graph

- Root type is orbital_review_bundle; it has no receipt.
- Support edges point from the supporting claim to the claim it supports; v0.1 permits direct evidence-claim-to-root edges.
- derived_from dependencies are cycle-checked.
- contradicts produces a contradiction blocker.
- supersedes is retained as a typed relation but does not prove currentness; currentness comes from explicit state and time fields.
- Duplicate identities, duplicate edges, missing endpoints, self-loops and cycles are invalid.
- Every non-root claim must connect to the root.
- An external authority decision is an input only. The evaluator always returns authority_granted=false.

## Result states

- review_bundle_complete: structurally complete for independent review only.
- incomplete_argument;
- blocked_by_open_defeater;
- blocked_by_unsupported_or_stale_evidence;
- blocked_by_scope_mismatch;
- blocked_by_claim_type_mismatch;
- blocked_by_missing_authority;
- contradicted;
- indeterminate;
- invalid_graph.

The output includes every blocker and its claim ID where applicable. The summary disposition has deterministic priority but does not hide the full blocker list.

## Standards and industry alignment

- [OMG Structured Assurance Case Metamodel (SACM)](https://www.omg.org/spec/SACM/): formal SACM 2.3 and the 2.4 beta released September 2026 provide metamodel concepts for structured assurance argumentation. This work does not claim a new assurance metamodel.
- [Goal Structuring Notation](https://scsc.uk/gsn): claims, strategies, contexts, assumptions, justifications and evidence provide an established assurance structure. Structuring an argument does not make it true.
- [NASA-STD-7009B](https://standards.nasa.gov/standard/nasa/nasa-std-7009): model/simulation verification, validation, uncertainty and credibility should be linked to intended use and validation domain.
- [NASA-STD-5017B](https://standards.nasa.gov/standard/NASA/NASA-STD-5017): mechanism qualification and physical life/environmental tests remain separate obligations.
- [IEC 61124:2023](https://webstore.iec.ch/en/publication/65838) and [MIL-HDBK-781 Rev B status](https://quicksearch.dla.mil/qsDocDetails.aspx?ident_number=54097): reliability test-plan comparators for their declared model scopes.

The proposed advance is not replacing SACM, GSN, standards, engineering judgement or independent review. It is machine-checked composition across heterogeneous, independently owned receipts, with exact scope, typed estimands, temporal cutoffs, defeaters and authority separation.

## Frozen corpus and acceptance targets

The current fixture contains 60 synthetic cases and two positive controls. It includes absent dependencies, scope and receipt-pin attacks, freshness and knowledge-cutoff attacks, mission-probability / constant-intensity substitution in both directions, evidence-class mismatch, open defeaters, contradictory and indeterminate claims, stale/withdrawn/unsupported claims, missing/stale/wrong-class authority, duplicate pins/edges, dangling edges, self-loops, cycles, reversed support direction, optional orphan claims and input permutation.

Prospective targets, not measured achievements:

| Metric | Target |
|---|---:|
| False review_bundle_complete results on seeded blockers | 0 |
| Claim-type/profile substitution detection | 100% |
| Exact receipt-pin mismatch detection | 100% |
| Effective/knowledge cutoff enforcement | 100% |
| Mandatory open defeaters in complete bundles | 0 |
| Unconnected claims in complete bundles | 0 |
| Authority grants | 0 |
| Input-permutation result differences | 0 |
| Independent finite oracle mismatches | 0 |

Measure performance at 10, 100, 1,000 and 10,000 claims/edges. Report p50/p95/p99, memory, blocker count, longest dependency path and evidence fan-in. Compare manual SACM/GSN review, structural-only checking, untyped JSON validation and adversarial graph mutation at identical scope. Reviewer effort and missed-blocker rate must be measured separately; runtime alone is not evidence of risk reduction.

## Dependency-scoped invalidation v0.1

The assurance-bundle crate now includes a separate dependency-invalidation adapter. It extends this existing typed-claim owner rather than creating a second evidence graph or receipt authority.

Each node binds:
- its produced artifact digest and currently observed artifact digest;
- the scope digest under which it was produced and the currently observed scope digest;
- exact dependency pins to upstream node content and scope digests.

The evaluator distinguishes `SOURCE_CHANGED`, `SCOPE_CHANGED`, `DEPENDENCY_VERSION_MISMATCH`, and `UPSTREAM_INVALIDATED`. It propagates invalidation only along declared dependency edges. A source mutation can therefore force requalification of dependent claims without invalidating unrelated evidence.

The seven frozen synthetic cases cover:
1. unchanged baseline graph;
2. action-domain snapshot content change, invalidating closure, independent domain oracle, feasibility, query policy, loss model, regret, calibration and decision frontier;
3. loss-model change, invalidating regret/calibration/frontier while preserving feasibility and query-policy evidence;
4. unrelated evidence change, which must not invalidate the decision frontier;
5. scope-only change with unchanged content digest;
6. missing dependency, which fails closed;
7. dependency cycle, which fails closed.

The Rust unit tests include input-permutation invariance and an always-false authority invariant. The CLI emits a source-pinned receipt; a separately authored JavaScript evaluator and exact field comparison provide a second implementation path.

**Claim ceiling:** this verifies dependency-graph invalidation semantics over a declared synthetic graph. It does not authenticate source receipts, prove that the dependency graph is complete, or grant authority. Omitted dependency edges remain an open-world assurance risk and require independent graph-authoring review.

## Finite-world incumbent decision adequacy v0.1

The crate now includes a separate incumbent-retention gate consuming a finite surviving-world set, a closed action universe, per-world feasible-action sets and exact loss rows. It computes regret per world against the best feasible action in that same world and takes the worst-case regret.

The rule retains an incumbent only when it is feasible in every surviving world and worst-case regret is no greater than the explicit materiality margin plus explicit switching cost. Equality at the threshold retains. Infeasibility in any surviving world or regret strictly above the threshold yields a re-optimization review trigger; the evaluator never chooses or executes a replacement.

An empty surviving-world set, open/stale domain closure, inadequate model family, stale/contradictory evidence, scope mismatch, or incomplete loss matrix abstains. This is important: lack of evidence cannot be interpreted as proof that the incumbent remains adequate.

The 13 frozen synthetic cases cover:
- stable incumbent and decision-irrelevant shift (retain);
- different world-optimal actions but incumbent regret within the retention margin;
- regret above the threshold and exact equality at the threshold;
- incumbent infeasible in one surviving world;
- no surviving worlds, open domain, inadequate model family, stale evidence and scope mismatch;
- incomplete loss matrix and duplicate loss rows;
- threshold-overflow arithmetic (tested through a typed Rust mutation so exact i64 boundary semantics are preserved across languages).

The independent JS oracle is compared field-by-field against the Rust receipt. No probabilities are assigned to worlds. RADAR-style stochastic monitoring is a separate comparator and remains distinct from #6887's anytime-valid sequential evidence guarantees.

**Claim ceiling:** synthetic finite-world incumbent adequacy only. It does not prove that the world set is complete, that real loss models are valid, or that a re-optimization is safe/authorized.

## Finite-world incumbent decision adequacy v0.1

Consumes the current finite surviving-world set, closed action universe, world-specific feasible actions and exact integer loss matrix. Retains an incumbent only if it is feasible in every surviving world and worst-case regret is no greater than the separately reported materiality margin plus switching cost. Equality retains. Any open/stale domain, stale evidence, inadequate model family, empty world set, scope mismatch, loss-unit mismatch or incomplete matrix abstains.

Fifteen frozen controls cover decision-irrelevant shifts, different world optima within the regret margin, threshold crossing/equality, incumbent infeasibility, empty/open/stale/ambiguous states, loss-unit/scope mismatch and malformed loss matrices. The Rust suite includes an exact typed i64 overflow mutation. A separate JS oracle is compared against the complete Rust receipt. No world probabilities are inferred and no replacement action is selected/executed.

~~~bash
cargo run --manifest-path tools/orbital-assurance-bundle-bench/Cargo.toml --bin incumbent-decision-adequacy -- \
  tools/orbital-assurance-bundle-bench/fixtures/v0.1/incumbent-adequacy-cases.json \
  --source-commit <40-lowercase-hex-commit>
node tools/orbital-assurance-bundle-bench/tests/incumbent_adequacy_oracle.mjs \
  tools/orbital-assurance-bundle-bench/fixtures/v0.1/incumbent-adequacy-cases.json
~~~

## Commands


~~~bash
cargo fmt --manifest-path tools/orbital-assurance-bundle-bench/Cargo.toml -- --check
cargo test --manifest-path tools/orbital-assurance-bundle-bench/Cargo.toml --all-targets
cargo clippy --manifest-path tools/orbital-assurance-bundle-bench/Cargo.toml --all-targets -- -D warnings
cargo run --manifest-path tools/orbital-assurance-bundle-bench/Cargo.toml -- \
  tools/orbital-assurance-bundle-bench/fixtures/v0.1/assurance-cases.json \
  --source-commit <40-lowercase-hex-commit>
node tools/orbital-assurance-bundle-bench/tests/reference_oracle.mjs \
  tools/orbital-assurance-bundle-bench/fixtures/v0.1/assurance-cases.json

cargo run --manifest-path tools/orbital-assurance-bundle-bench/Cargo.toml --bin dependency-invalidation -- \
  tools/orbital-assurance-bundle-bench/fixtures/v0.1/invalidation-cases.json \
  --source-commit <40-lowercase-hex-commit>
node tools/orbital-assurance-bundle-bench/tests/dependency_invalidation_oracle.mjs \
  tools/orbital-assurance-bundle-bench/fixtures/v0.1/invalidation-cases.json
~~~

## Ownership

This crate composes; it does not duplicate lifecycle/event semantics (#6485/#6516), campaign adequacy (#6542), one-shot mission reliability (#6553), constant-intensity statistics (#6593), trust roots/credentials/signatures/revocation (#6519), physical test safety and metrology (#6256/#6303), or the broader assurance-case/adversarial challenge patterns (#5647/#5721).

A passing synthetic corpus establishes only that the reference evaluator classifies the declared graph structure and metadata as expected. It does not authenticate evidence, prove claims true, establish hazard completeness, qualify a mechanism, accept risk, establish flight readiness, or authorize operations.
