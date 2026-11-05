# Road-autonomy exogenous coupling fixtures v0.1

**Status:** standalone semantic reference and conformance fixtures; not a production RNG and not integrated into physics execution.  
**Parent:** Symtropy #3543.  
**Consumers:** #3532 reciprocal loop, #3502 actor adapter, #3539 pose/replay substrate, Symthaea #10356/#10372.

## Research position

The general event-keyed RNG design is established prior art. Buffalo, Pearson & Klein (2026), [Realizing Common Random Numbers: Event-Keyed Hashing for Causally Valid Stochastic Models](https://arxiv.org/abs/2603.11084), define execution invariance and show how counter-based generators plus stable event identifiers avoid execution-path-dependent draw shifts. Salmon et al. (SC'11) established counter-based RNGs for parallel computation ([DOI](https://doi.org/10.1145/2063384.2063405)).

Our bounded contribution is a road-autonomy conformance layer for the meaning of event identity across interactive scene branches. The paper explicitly treats event matching as a scientific modeling choice. This crate makes three candidate identity bases explicit:

- Actor event: one scenario-authored actor, tick and event kind.
- Directed actor pair: initiator, recipient, tick and event kind; roles are not interchangeable.
- Scene slot: one encounter/opportunity slot, tick and event kind; using it across different actors asserts a slot-level exchangeability assumption.

These are distinct coupling hypotheses, not interchangeable encodings. Keep alternative hypotheses as sensitivity branches when evidence cannot select one.

## Contract

DrawAddress.canonical_bytes creates a versioned, length-prefixed representation. It is intentionally not a hash, random generator or security primitive. A later pinned counter-based generator can map canonical addresses to its counter/key space.

Shared addresses omit branch identity but require an explicit equivalence group and justification reference. Branch-local and independent-replicate addresses include their branch identity. Random values are not generated here.

An actor decision may receive the same exogenous policy-noise input in both branches while its delivered observation differs. The policy output must still be recomputed from that branch's observation. Do not automatically relabel every actor-policy draw after an ego intervention as branch-specific.

DrawPresence.NotRealized records an event address that exists in the potential event space but was not realized in that branch. It requires a causal-condition witness. A missing record is different: the pair audit marks it incomplete rather than assuming conditional absence.

The API deliberately does not accept action, cue contents, velocity, pose, planner output or collision state as identity fields. Those are endogenous state or mechanism inputs and must not silently contaminate an invariant exogenous key.

Stable actor IDs are scenario-authored. Runtime BodyHandle, allocation order, mutable vector index, spawn order and current pose are not admissible identity sources. Dynamic births require separately justified lineage semantics and are out of this v0.1 crate.

## Executable checks

The standalone Rust crate has no third-party dependencies and is isolated from the main Cargo workspace. Its tests cover invariant shared addresses, branch separation, call-order independence, distinct identity bases, directed-pair role preservation, duplicate consumption, shared-value mismatch, conditional presence, missing counterparts, branch-scope enforcement and malformed IDs.

## Claim ceiling

Passing establishes only that this small reference implementation serializes the declared identity contract and checks represented pair records. It does not validate the causal graph, select an event identity map, prove random-number quality, identify real-world counterfactuals, validate actor behavior or physics, establish a safety case, or authorize vehicle actuation.


## Event-correspondence envelope (stacked extension for #3571)

The crate now also represents multiple admissible partial maps between two branch-local event universes. It distinguishes:

- Actor-keyed identity: the same scenario-authored actor and event family.
- Directed-dyad identity: same initiator and recipient roles.
- Encounter-slot identity: same declared opportunity slot, even when the actor occupant differs. This explicitly asserts an exchangeability assumption.

Each candidate hypothesis must partition every event in its declared branch-local universe into exactly one matched pair or an explicit unmatched disposition. Endpoints cannot be reused. Pair compatibility is checked against the identity basis. An event with an observed counterpart in one branch and a causal-condition witness for non-realization in the other is not confused with a missing tracker record.

The correspondence set retains multiple hypotheses rather than choosing whichever map is nearest, most numerous, or most variance-reducing. Two independent completeness claims are represented: whether the branch-local event universe is exhaustive within its scope, and whether the matching-hypothesis family is exhaustive within a declared model family. A complete event universe with a budget-limited matching search is still an incomplete correspondence envelope. Both completeness claims are caller-supplied and evidence-bound; internal coverage does not prove that all scientifically plausible events or maps were discovered. Unknown presence, unknown correspondence, or either incomplete status yields an incomplete assessment.

The canonical_bytes method sorts event records, hypotheses, pairings and unmatched events before serialization. It is a deterministic representation only—not a hash, signature, authenticated receipt or proof of identity. A downstream receipt may apply its separately qualified digest/authentication mechanism.

The new conformance module deliberately does not:
- infer matches from geometry, runtime BodyHandle, actor enumeration or nearest-neighbour rank;
- assign probabilities to correspondence hypotheses;
- optimize the correspondence for CRN variance;
- decide action rankings or causal identification (SAM #10372 remains responsible);
- create or generate random values.

Tests cover unique identity, ambiguous actor-vs-slot mappings, directed-role mismatch, conditional event non-realization, missing/unknown events, unmatched events, invalid partitions, duplicate endpoints, incomplete candidate universes and canonical order invariance.


## Correspondence-conditioned stochastic address projection (ROAD-CORRESPONDENCE-COUPLING-001)

The new \`coupling_projection\` reference composes the finite correspondence set with causal draw-source dispositions. It is intentionally downstream of #3571 and upstream of the existing #3582 Philox resolver.

Important separation:
- A correspondence pair says which branch-local events are admitted as counterparts under one declared map.
- A causal disposition separately says whether a particular stochastic source is invariant, branch-specific, sensitivity-only, not identified, or incomplete.
- Shared pair addresses require the exact source/mechanism/version/component profile, tick and event-kind to agree across branches.
- Branch-local addresses are keyed by the branch and local event identity, not by whichever counterpart hypothesis happened to be considered. Thus changing a correspondence hypothesis does not perturb a branch-local random input.
- An unknown event presence prevents shared addressing. A conditionally non-realized event retains its potential-event address; presence and its condition witness are receipt metadata, not address input.
- Every admitted correspondence hypothesis produces a separately retained plan. An incomplete event universe or budget-limited hypothesis search marks the overall plan incomplete even when known-hypothesis plans are available.

The output is a typed, versioned v0.2 canonical address byte string. It is not a hash or Philox output. The downstream pinned resolver remains the owner of BLAKE3/Philox. The module has finite fixtures for actor-vs-slot ambiguity, cause-class separation, branch-local invariance, conditional absence, unknown presence, incomplete search, source/version mismatch and sensitivity-only coupling.

No event map is selected as true and no common-random-number variance benefit is assumed.


### Pinned Philox resolution for correspondence v0.2

The correspondence projection now feeds the same pinned rand_philox 0.2.1 Philox4x32-10 resolver and blake3 1.8.7 derive-key dependency used by ROAD-PHILOX-ADAPTER-001 (#3582). This adds a v0.2 byte-address entry point; it does not introduce another PRNG or sampling algorithm.

- v0.1 DrawAddress bytes retain the original ROAD-EXO-COUPLING-001 derivation context and receipt schema.
- Correspondence-projected v0.2 canonical bytes use a distinct ROAD-CORRESPONDENCE-COUPLING-001 KDF context, derivation ID and address schema ID. The canonical bytes are passed unchanged.
- Each hypothesis plan resolves every retained branch/event address. The returned wrapper retains the exact original correspondence map (paired endpoints, identity bases, hypothesis justification and unmatched-event rationale/evidence references), in addition to each hypothesis ID.
- Each draw receipt retains branch/event ID, sharing disposition, presence witness, exact canonical address bytes and the raw four-word Philox receipt. Every draw can therefore be replay-verified from the bundle without recovering a separate address plan.
- The address plan and resolved bundle retain the original candidate-universe and hypothesis-set status objects, including their evidence-reference IDs. These are provenance references supplied by the caller, not independent validation that the referenced evidence is genuine or that the search was scientifically exhaustive.
- The existing 192-bit key/counter collision registry is applied across the entire plan bundle. Repeated branch endpoints of one exact shared address are permitted only when their full 256-bit derivation digests match.
- Incomplete correspondence sets may retain draws for known hypotheses, but the draw bundle preserves both Incomplete status and the original universe/search evidence references; resolving a random block does not upgrade identification or completeness.
- The resolved bundle has a versioned BLAKE3 content digest over the full correspondence maps, per-hypothesis causal-rule manifests, scope/search evidence references, hypothesis/pair and causal-rule justifications, address bytes, presence/sharing labels and raw Philox receipts. Causal disposition remains a first-class inspectable field; consumers do not need to reverse-engineer canonical address bytes.
- The independent verifier checks the bundle digest, verifies every raw draw against the frozen experiment seed, repeats the bundle-wide Philox collision check, and checks that every correspondence pair has exactly one matching causal rule whose disposition agrees with both branch receipts and their presence states. It also verifies that every paired and unmatched event has exactly one branch-scoped receipt.
- The digest detects changes relative to an externally retained commitment; it is not a signature, does not authenticate the caller, and does not validate the truth of referenced evidence.
- Tests replay every returned draw from its retained canonical bytes, reject altered address/receipt values, verify incomplete-scope references survive resolution, exercise bundle-level tampering of search provenance/correspondence/causal justifications/draw outputs, and reject a rehashed bundle whose causal rule contradicts its sharing labels.

The resolver emits raw u32 words only. Distribution sampling/transforms remain #3586's separate responsibility. Shared draw alignment is a causal/execution contract; its statistical variance effect is reported independently and can be positive, neutral or negative.

Additional exact tests cover: same Philox words for both endpoints of one justified shared event under a given hypothesis; distinct draws for alternative actor-vs-slot mappings; stable branch-local draws across alternative maps; v0.2 receipt replay from canonical bytes; preservation of incomplete status; and separation of v0.1/v0.2 KDF domains.


### Semantic projection closure (ROAD-CORRESPONDENCE-RECEIPT-CLOSURE-001)

The resolved draw bundle now retains the exact finite projection inputs:
- complete left/right candidate event inventories, including identity fields and presence evidence;
- the complete branch/event scope table, including world/episode, tick, event kind, source, mechanism version and draw component;
- the existing correspondence hypotheses, causal rules, canonical addresses and Philox receipts.

The bundle digest commits to these structured inputs as well as the correspondence and draw results. Independent verification reconstructs the declared `CorrespondenceSet`, reruns finite correspondence assessment, reprojects the canonical address plans from the retained scopes and hypothesis-scoped causal rules, and compares every regenerated address and disposition against its receipt before accepting the Philox replay.

This closes the gap between replaying an arbitrary retained byte string and verifying that the byte string follows from the declared event/scope/coupling contract. It also lets the verifier recheck that every candidate event in each retained branch universe is classified exactly once as paired or explicitly unmatched.

The new negative controls mutate a tick, substitute and correctly re-resolve arbitrary address bytes, and add an unclassified candidate event while recomputing the outer content digest. All must fail semantic verification. A stale or contradictory incomplete disposition is likewise rejected.

The retained universe and scope are still caller-supplied inputs. The verifier establishes internal finite self-consistency, not scientific exhaustiveness, authenticity of external evidence references, causal truth, or common-random-number variance reduction.
