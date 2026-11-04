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


## Deterministic Philox resolver (ROAD-PHILOX-ADAPTER-001)

The crate now includes an adapter over two pinned dependencies:

- `rand_philox = 0.2.1`, default features disabled: Philox4x32-10 block generation.
- `blake3 = 1.8.7`, default features disabled: BLAKE3 derive-key mode for deterministic semantic-address mapping.

The adapter is not a new PRNG. It uses the documented `Philox::new([u32; 2], u128)` API and reads exactly four words from exactly one block. It does not use a sequential stream seed or random-draw cursor.

### Frozen v0.1 address derivation

1. Accept a 32-byte experiment root seed and an already-qualified DrawAddress.
2. Obtain DrawAddress canonical bytes. Shared versus branch-local semantics are entirely decided upstream by the DrawAddress/coupling record.
3. Length-prefix the seed and canonical address bytes as little-endian u64 lengths.
4. Apply BLAKE3 derive-key mode with the fixed context string in the source.
5. Map digest bytes [0..8] to the two little-endian u32 Philox key words, [8..24] to a little-endian u128 counter, and retain [24..32] as an eight-byte collision audit tag.
6. Generate one Philox block and retain its four u32 words in canonical output order.

The Philox input is 192 bits (64-bit key + 128-bit counter), derived from a 256-bit digest. It is not collision-free. Each receipt retains the full derivation digest; a run-local registry rejects one key/counter pair claimed by different full digests. Cross-run receipt comparison is still required. This is not authentication or a cryptographic security boundary.

### Receipt and limits

A DrawReceipt binds the address schema ID, KDF ID/context, PRNG crate/algorithm version, full derivation digest, collision tag, key, counter and all four output words. verify_receipt recomputes the full mapping from the frozen experiment seed and canonical address.

The algorithm is a reproducible simulation RNG, not a CSPRNG. The 32-byte experiment root seed is reproducibility material, not a secret. Distinct semantic addresses do not automatically prove independent random variables or a valid causal coupling. Event identity and causal admissibility remain upstream responsibilities.

The adapter uses raw u32 words only. It deliberately adds no float conversion, bounded sampling, modulo reduction or rejection loop. Domain-specific distribution transforms and their consumption semantics must be specified separately.

### Verification

Tests include the three Random123 Philox4x32-10 known-answer vectors, same-address determinism, shared-vs-branch-specific behavior, call-order invariance, seed/source/component separation, receipt recomputation and run-local collision handling. Exact-head GitHub CI is required before this resolver is considered qualified.


## Reproducible dependency and fixture contract

The isolated package now has a committed `Cargo.lock`. Direct dependencies are exact-pinned and the focused workflow uses `cargo metadata/test/clippy --locked`; a resolver may not silently change the dependency graph.

The Philox test suite pins the three upstream Random123 Philox4x32-10 KAT rows (zero, all ones, and mixed pi-digit input) from `tests/kat_vectors`. Separate property tests check that changing key or counter changes the block; those are not mislabeled as upstream KAT rows.

The BLAKE3 adapter tests the official derive-key KAT using the official context string and zero-length key material from BLAKE3's `test_vectors/test_vectors.json`. The address serializer separately length-prefixes the 32-byte experiment seed and canonical event address.

Machine-readable `event_identity_cases.json` now records the dependency versions, KAT inputs/outputs, KDF context, address mapping, length-prefix convention and the 192-bit collision model. The workflow checks both event-identity and correspondence fixture JSON.

The locked dependency graph is a reproducibility contract. Any dependency update must update the lockfile intentionally and rerun KATs, formatting, tests, Clippy and cross-target checks.


### Additional adapter invariants

The resolver's unit tests also pin:
- the official BLAKE3 derive-key zero-length-input vector;
- exact little-endian u64 length-prefix framing for the experiment seed and canonical address as separate fields;
- fixed BLAKE3 KDF context domain separation;
- raw Philox output sensitivity to key and counter changes;
- unchanged output after unrelated address draws;
- seed/source/component separation, receipt recomputation and run-local block-address collision rejection.

The standalone manifest uses `blake3 = =1.8.7` and `rand_philox = =0.2.1`; `Cargo.lock` freezes the resolved transitive graph, and the workflow requires `--locked`.


## Address-scoped bounded integer sampling (ROAD-VARIATE-TRANSFORMS-001 #3586)

The crate now includes V0.1 of an unbiased bounded-integer transform over the raw Philox resolver.

### Transform

For a nonzero 32-bit bound n and raw u32 word x:
- Compute the 64-bit product m = x × n.
- Candidate result is the high 32 bits of m.
- Let threshold = 2^32 mod n, computed with unsigned wrapping arithmetic.
- Accept when the low 32 bits of m are at least threshold; otherwise reject and retry.

This is Lemire's multiply-high/rejection method (Daniel Lemire, 2019, https://doi.org/10.1145/3230636). The result is not computed by modulo reduction.

### Retry isolation

Each attempt's child DrawAddress is derived from:
- complete root DrawAddress canonical bytes;
- transform ID and version;
- the attempt index;
- a domain-separated BLAKE3 child derivation.

The child component binds the digest of these values. Each attempt resolves one independent Philox block through #3582. A rejected attempt cannot move the cursor of any other event because there is no shared cursor.

### Receipt

The BoundedIntegerReceipt retains the root-address digest, transform ID/version, bound, maximum-attempt budget, accepted result or typed BudgetExceeded disposition, and for every attempt:
- attempt index and child-address digest;
- complete raw Philox DrawReceipt;
- low/high multiply outputs;
- rejection threshold and accepted/rejected status.

Receipt integrity is an unkeyed checksum only. verify_against recomputes the full sampling trace from the frozen experiment seed and root event address.

### Edge cases and limits

- Bound zero rejects before any random draw.
- Bound one returns zero with an explicit DeterministicSingleton disposition and no random consumption.
- Attempt-budget exhaustion produces BudgetExceeded and no output; there is no modulo, zero, biased, or fallback result.
- V0.1 uses one Philox block per attempt and the first u32 word. The other words remain recorded for reproducibility.
- Only bounded u32 integers are implemented. Rational Bernoulli, categorical integer weights and continuous samplers remain future work under #3586.
