# AERO Assurance Delta Impact v0.1

**Status:** synthetic dependency-impact evaluator; exact-head compilation, tests and Clippy pending  
**Authority ceiling:** change-impact proposal only; no change approval, engineering acceptance, physical-work authorization, airworthiness or certification  
**Relationship:** implementation seed for #7923; additive to the existing ClaimGate and frozen benchmark v0.3

## 1. Purpose

A document/path-based change list is not an assurance impact analysis. A changed file may affect several claims through a model, solver, material process, observer, evidence package or configuration dependency; a seemingly unrelated file may be outside the claim's dependency closure.

This module models a single claim's assurance dependencies as a directed graph. Edges are oriented from a dependent node to the prerequisite it depends on:

`dependent → dependency`

A change to a prerequisite invalidates every downstream dependent obligation reachable by following the reverse dependency relation. For each obligation, the evaluator returns one of three outcomes:

- **Affected:** a concrete path from the obligation to the changed node is present.
- **Retained:** there is no such path, the graph declares complete dependency coverage, and every node in the obligation's transitive dependency closure declares its own dependency manifest complete.
- **Incomplete:** no changed-node path is known, but the graph or any node in that closure cannot justify a complete unchanged-dependency claim.

A missing or incomplete graph can never support a retained result. This is the key fail-closed property.

## 2. Input and source-binding contract

The v0.1 input binds:
- one claim and exact configuration;
- an immutable dependency graph snapshot;
- a graph approval receipt, bound to snapshot digest, claim, configuration, first-seen time and effectivity interval;
- one owner-issued change receipt, bound to target node, previous/replacement content digests, claim, configuration, first-seen time and effectivity interval;
- an explicit historical as-known cutoff.

All timestamps use Unix milliseconds. Effective intervals are half-open: `[effective_from, effective_until)`. A receipt received after the selected as-known cutoff cannot backfill historical knowledge.

The graph digest is recomputed over a sorted node/edge manifest using the declared `rust-serde-json-struct-order-v1` serialization profile. Each node commits its owner namespace, kind, content digest and per-node dependency-completeness declaration. The change receipt owner must match the owner namespace of the changed node. A retained result also requires the claim root's dependency manifest to be complete, so hidden obligations cannot be skipped by an incomplete root. Edge order and node order do not affect the result.

The evaluator checks the upstream `VERIFIED` dispositions, exact scope, digests, chronology and effective intervals. **It does not authenticate signatures or trust roots.** Production integration must authenticate the graph and change receipts with the relevant source-owner trust policy before passing them to this pure evaluator.

## 3. Result semantics

| Disposition | Meaning | Safe next interpretation |
|---|---|---|
| `IMPACT_IDENTIFIED` | At least one obligation is reached and every other obligation has a complete retained closure. | Requalification scope can be proposed for the identified obligations, subject to human/domain authority. |
| `NO_AFFECTED_OBLIGATIONS` | No obligation is reached and every obligation has a complete retained closure. | No impact is identified within this exact, complete graph snapshot. This is not a universal no-impact claim. |
| `INCOMPLETE_IMPACT` | At least one obligation cannot be proven affected or retained because dependency coverage is incomplete. | Do not issue a no-impact conclusion; obtain missing dependency evidence or conservatively expand review. |
| `INVALID_INPUT` | Digest, scope, receipt, chronology, effectivity or graph constraints fail. | Reject this assessment input. |

Each affected obligation carries an ordered dependency path to the changed node. Each retained obligation carries a digest of the exact closure's sorted node identities/content digests and internal edges, bound to the graph snapshot. The closure digest makes the calculation reproducible; it does not prove the owner's completeness declaration is truthful.

## 4. Safety properties and hostile cases

The Rust unit tests and CI-only independent Python oracle cover:
- transitive dependency invalidation and path reconstruction;
- retention only for a disjoint closure with complete dependency coverage;
- global incomplete graph status prevents retained conclusions;
- one incomplete node or claim-root dependency manifest prevents retained conclusions;\n- a change receipt from an owner other than the changed node's owner is rejected;
- late change receipt cannot backfill the historical cutoff;
- previous digest must match the graph node's content digest;
- node/edge order permutation preserves the graph commitment and assessment;
- adding a known dependency cannot reduce the affected-obligation set;
- changed graph content without a refreshed snapshot commitment fails closed.

The implementation does not reinterpret an existing claim gate or modify the assurance state. All authority booleans remain false.

## 5. Deliberate limitations in v0.1

This first slice is deliberately narrow:
- one claim, one configuration and one change event per evaluation;
- single graph snapshot, not a distributed/merged dependency graph;
- no signed-receipt verification inside this module;
- no compatibility policy that allows a changed model/evidence item to remain applicable;
- no dependency discovery from untrusted documents or LLM output;
- no execution of requalification tasks or approval of engineering changes;
- no probabilistic estimate of hidden dependencies.

A subsequent version should add multi-claim impact, supersession/compatibility relations, graph-delta certificates, source-native adapters (PLM, ALM, model registries, owner evidence stores), multi-change event batching, and independent graph-completeness adjudication. Compatibility must be an explicit, versioned relation with its own authority and evidence, never an implicit “similar enough” rule.

## 6. Benchmark programme

Use independent, adjudicated engineering change episodes. Mutations generated from one root cause are clustered under that episode.

Compare:
1. full requalification of every potentially dependent claim;
2. file/path-based change impact;
3. static requirements traceability;
4. conventional graph dependency closure;
5. this exact-scope temporal impact evaluator;
6. expert adjudication as the reference;
7. poisoned and incomplete graphs.

Measure:
- false-negative affected hard-gate obligations (hard gate: zero in the bounded adjudicated holdout);
- precision/recall and affected-set Jaccard similarity;
- percentage of retained obligations with complete proof paths;
- incomplete-graph detection and unknown-edge rate;
- unnecessary requalification work versus full baseline;
- change-to-impact computation time and independent replay agreement;
- worst-case dependency blast radius;
- reviewer override and adjudication time.

Do not optimize for minimal scope until the false-negative hard-gate rate is zero on a held-out, independently adjudicated corpus. An aspirational target of ≤5% unnecessary requalification and ≥50% review-effort reduction is an experiment target, not a demonstrated result.

## 7. Non-claims

This module does not establish physical truth, prove that an owner has declared every dependency, qualify tools, validate a model, approve an engineering change, replace configuration-management authority, or confer certification credit. It only computes a reproducible impact projection from a supplied, scope-bound dependency graph and upstream-verified receipts.
