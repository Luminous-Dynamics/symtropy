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

The crate includes a finite-world incumbent-retention gate. It consumes the current surviving-world set, a closed action universe, world-specific feasible sets, and exact loss rows. It does **not** compare against a different optimum in each hidden world. Instead, it intersects feasible sets and compares the incumbent's worst-case loss with the best single action feasible in every surviving world.

For the common feasible set (F_{common}=\cap_wF(w)), define (M(a)=\max_wL(a,w)). The reference comparator minimizes (M(a)) over (F_{common}), with deterministic action-ID tie-breaking. The common-action improvement is the incumbent worst-case loss minus the reference common-action worst-case loss. The reference action is a benchmark comparator, not an action recommendation.

The gate retains only when the incumbent is feasible in every world and common-action improvement is no greater than the explicit materiality margin plus switching cost. Equality retains. Infeasibility in any surviving world or improvement strictly above threshold yields a review trigger. An empty/stale world set, open/stale domain, inadequate/stale/indeterminate dependency graph, inadequate model, stale/contradicted evidence, scope/loss-unit mismatch or incomplete matrix abstains.

This is a static non-anticipative comparator: the same reference action must be feasible across all current worlds. It avoids clairvoyant hidden-world switching. Adaptive observation-contingent policies are a separate extension owned by #6997 and must branch only on admissible observables.

The 16 frozen controls cover:
- stable incumbent with a common reference action that improves worst-case loss by five units;
- a decision-irrelevant world change;
- opposite world-specific optima where a per-world hindsight comparator would report a four-unit gain and trigger at margin three, but the best common action yields zero improvement and correctly retains;
- threshold crossing/equality and incumbent infeasibility;
- empty/open/stale/inadequate world, action-domain, dependency-graph and evidence states;
- incomplete loss matrix, duplicate loss rows, scope mismatch and loss-unit mismatch.

The Rust suite includes an exact typed i64 overflow mutation. A separately authored JS oracle is compared field-by-field against the Rust receipt. No world probabilities are assigned, no replacement action is selected or executed, and authority remains false.

~~~bash
cargo run --manifest-path tools/orbital-assurance-bundle-bench/Cargo.toml --bin incumbent-decision-adequacy -- \
  tools/orbital-assurance-bundle-bench/fixtures/v0.1/incumbent-adequacy-cases.json \
  --source-commit <40-lowercase-hex-commit>
node tools/orbital-assurance-bundle-bench/tests/incumbent_adequacy_oracle.mjs \
  tools/orbital-assurance-bundle-bench/fixtures/v0.1/incumbent-adequacy-cases.json
~~~


## Joint physical-world / loss-model incumbent bridge v0.1

The incumbent gate now consumes two separate source-pinned inventories: the finite compatible-world receipt from #6835 and the coherent joint loss-model scenario receipt from #6939. A joint scenario is a (physical-world ID, loss-model ID) row; several loss models may share one physical world. It verifies exact planned and observed IDs, source receipt/schema/claim-ceiling pins, per-scenario evidence status and receipt IDs, and a digest of the projected scenario rows consumed by the comparator.

The source rational loss is carried alongside its projected integer loss. The projection has an explicit source unit, target loss unit, projection ID and rational scale. The evaluator requires exact divisibility and equality with the projected integer; it never rounds or uses floating-point arithmetic. The 29-case corpus covers a valid fractional projection (1/10 × 1000 = 100), inexact 1/3 conversion rejection, integer projection mismatch, stale/indeterminate/out-of-profile loss manifests, per-scenario evidence blockers, omitted joint scenarios, omitted physical-world coverage and multiple loss models for one physical world.

**Important source boundary:** v0.1 consumes the finite #6835 decision-identifiability receipt. It does not consume or inherit #6887's anytime-valid sequential sampling/stopping guarantee. That requires a separate typed adapter. Source digests are structural pins, not issuer authentication or proof of loss-model calibration/completeness.

Detailed contract: [INCUMBENT_LOSS_MODEL_BRIDGE_V0_1.md](../../research/orbital/INCUMBENT_LOSS_MODEL_BRIDGE_V0_1.md).

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
