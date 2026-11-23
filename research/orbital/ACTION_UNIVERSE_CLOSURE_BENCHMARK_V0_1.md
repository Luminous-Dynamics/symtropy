# Proof-carrying, scope-bounded action-universe closure — benchmark v0.1

**Status:** research benchmark specification and synthetic reference implementation.  
**Owner:** ORBITAL-ACTION-CLOSURE-001 (#6959).  
**Parent integration:** scenario-dependent feasibility/regret (#6952 / PR #6958).  
**Claim ceiling:** finite source-bound closure structure only. No real-world completeness, statistical coverage, issuer authenticity, physical qualification, or execution authority is established.

## The decision problem is larger than the loss matrix

A solver can be exactly optimal over its declared action list and still be wrong because the declared list is incomplete. Hashing the list makes the supplied set reproducible; it does not prove that every in-scope action was enumerated. Two implementations may also agree because they read the same incomplete registry, API, ontology, or upstream model.

The research shift is to treat **the boundary of the decision problem as an object that must carry evidence**. The closure verifier should not attempt to prove an unbounded statement such as “all possible actions.” It should ask a narrower, auditable question:

> For this explicit configuration, time window, component inventory, actor/authority boundary, operating envelope, prerequisites and policy version, do the planned domain partitions, positive action witnesses, exclusions, independent enumerations and source snapshots agree without unresolved common causes?

A successful result is named `CLOSED_WITHIN_DECLARED_SCOPE`. The scope is a premise—not a conclusion.

## v0.1 finite closure package

The Rust evaluator in `tools/orbital-observation-surface-bench/src/action_universe_closure.rs` consumes:

1. **Scope manifest.** System/configuration version, start/end time, component-inventory digest, actor and authority boundary, operating-envelope identifier, prerequisite/policy versions, immutable snapshot digest and current snapshot digest.
2. **Planned partition inventory.** A predeclared list and digest, distinct from the rows that an enumerator actually returned. Each partition is classified exactly once as present, excluded, not applicable, or indeterminate.
3. **Positive action witnesses.** Every action ID is attached to one partition, a payload digest and a witness receipt. Both the ID inventory and the sorted action-ID/payload-digest pairs are bound.
4. **Negative-space witnesses.** Excluded and not-applicable partitions carry explicit exclusion IDs, reason codes, evidence receipts and negative-space witness receipts. An empty exclusion or missing receipt is not interpreted as “nothing else exists.”
5. **Distinct enumerator receipts.** At least two enumerators bind method ID, algorithm digest, source registry, the same domain snapshot, execution receipt, dependency IDs and action inventory.
6. **Common-cause screening.** Shared registries and declared dependencies are exposed. Each enumerator binds its dependency-ID list to a digest and an opaque dependency-inventory receipt ID. Matching outputs do not establish independence when source dependencies overlap, when dependency evidence is absent, or when undeclared dependencies remain possible.

The evaluator uses exact set and digest checks. It requires dependency-inventory receipt identifiers, but v0.1 does **not** authenticate those receipts, prove the declared dependency lists are exhaustive, recompute the supplied component/scope snapshot digests from source records, or inspect the underlying action payloads. Its common-cause rule is therefore a conservative screen over declared dependencies—not a proof of independent enumeration. It does not infer probabilities from the number of enumerators or partitions and does not grant downstream global optimality on an open or unresolved universe.

## Status semantics

| Status | Meaning |
|---|---|
| `CLOSED_WITHIN_DECLARED_SCOPE` | Finite declared boundary; planned and observed partitions match; witnesses and inventories are complete; distinct enumerators agree; no declared common dependency overlaps. |
| `ENUMERATOR_DISAGREEMENT` | At least one enumerator or the observed action inventory differs from the planned action universe. |
| `PARTITION_OMITTED` | The observed partition set differs from the planned partition inventory. |
| `NEGATIVE_SPACE_UNWITNESSED` | An excluded/not-applicable region or its declared negative-space evidence is missing or inconsistent. |
| `COMMON_CAUSE_UNRESOLVED` | Enumerator independence is not evidenced: shared source registry, overlapping dependency, duplicate method or duplicate algorithm digest. |
| `SCOPE_DRIFT` | Current domain snapshot differs from the snapshot bound by the closure case. |
| `STALE_SNAPSHOT` | An enumerator receipt does not apply to the case's scope/snapshot. |
| `CLOSURE_INDETERMINATE` | The domain boundary is not finite or a partition remains indeterminate. |

Malformed manifests and digest mismatches are input errors, not a closure status. The receipt never grants physical qualification or action authority.

## Frozen synthetic challenge corpus

The v0.1 corpus contains ten cases:

- Fully witnessed finite scope with two distinct enumerators (positive control).
- Two enumerators sharing the same registry/API (common-cause negative control).
- One enumerator omitting an action while another returns the planned inventory.
- A planned partition omitted by the observed partition rows.
- An exclusion lacking a negative-space witness.
- Domain-snapshot drift after enumeration.
- An enumerator bound to an older snapshot.
- An unbounded scope presented as if it could be closed.
- Missing dependency-inventory evidence despite distinct enumerator labels.
- An extra, unplanned exclusion that is otherwise fully witnessed.

The independent Node oracle recomputes the canonical SHA-256 inventories (including each dependency inventory), dependency intersections, status precedence and summary. The Rust receipt is compared field-for-field against that independently authored implementation.

## Industry and research comparison

This is not a replacement for robust optimization, K-adaptability, conformal prediction, model checking, safety cases, or formal verification. It adds an explicit closure/evidence boundary to the inputs those methods assume.

- **Robust decision-focused learning** addresses uncertainty in objective coefficients and deployment shift; compare matched regret, feasibility and shift cases: [Yamao et al., UAI 2026](https://proceedings.mlr.press/v337/yamao26a.html).
- **Conformal robustness control** treats prediction-set construction as a decision problem under robustness constraints; compare matched false-feasible risk and calibration burden: [Hu et al., ICLR 2026](https://openreview.net/forum?id=4mT3WQO6hJ).
- **Decision-dependent robust optimization** studies paid queries that shrink uncertainty sets; compare query cost and recourse against the non-anticipative policy branch: [Ley et al., OR Spectrum (2026)](https://link.springer.com/article/10.1007/s00291-026-00856-z).
- **Robust K-adaptability** bounds the number of policies needed for exactness in certain settings and exposes complexity growth under constraint uncertainty; compare exact finite policy trees against bounded K-policy menus: [Kurtz, Mathematical Programming (2026)](https://link.springer.com/article/10.1007/s10107-026-02329-1).
- **Assurance cases and systems security engineering** provide the industry pattern for claims, assumptions, argument and evidence; the contribution here is a machine-checkable, replayable closure sub-claim, not a replacement for independent assurance review: [NIST SP 800-160 Vol. 1 Rev. 1](https://csrc.nist.gov/pubs/sp/800/160/v1/r1/final) and [NIST assurance-case terminology](https://csrc.nist.gov/glossary/term/assurance_case).
- **Safety-critical software assurance** requires process and verification evidence beyond a passing unit-test suite; compare the evidence packaging and exact baseline binding with [FAA AC 20-115D](https://www.faa.gov/documentLibrary/media/Advisory_Circular/AC_20-115D.pdf) and DO-178C / DO-333 practice.

### Baselines to beat

1. Single registry query + action-list digest (reproducible but no completeness evidence).
2. Two code paths querying the same registry (implementation diversity without source diversity).
3. Two registries with explicit dependency cut sets.
4. Registry enumeration plus a separately derived model-based enumerator.
5. Partition-by-partition positive and negative-space witnesses.
6. Metamorphic omission, insertion, duplication, stale-snapshot and re-scope attacks.

A benchmark must report false closure, missed-action rate on independently generated bounded domains, false disagreement, common-cause detection recall, partition coverage, stale-snapshot detection, witness completeness, enumeration/adjudication cost and receipt replay cost. Jaccard overlap alone is not an adequate metric: two lists can agree perfectly and both be wrong.

See also the broader [Decision-Assurance Tournament v0.1](DECISION_ASSURANCE_TOURNAMENT_V0_1.md), which defines matched baselines including GARO, robust decision-focused learning, Conformal Robustness Control, multi-distribution conformal prediction, paid-query robust optimization and K-adaptability. It also specifies the independent domain oracle required to detect actions omitted by both enumerators.

## Prospective acceptance targets

These are **targets**, not achieved results:

- 100% exact set agreement against a separately generated exhaustive oracle on bounded synthetic domains.
- Zero `CLOSED_WITHIN_DECLARED_SCOPE` results with a missing planned partition, invalid witness, stale snapshot or unresolved common cause.
- Zero independence claims when registry/method/algorithm/dependency overlap is unresolved.
- Zero downstream global optimum/regret claim when closure is open, stale, omitted or indeterminate.
- Zero stale closure receipt survives a mutation to its bound domain snapshot.
- Held-out missed-action rate and false-closure rate reported with exact confidence intervals on independently generated domains.
- Every reported cost remains a native dimension; never trade missed safety constraints against lower enumeration cost in a weighted score.

A useful next extension is adversarial **scope generation**: independently synthesize the domain boundary and then generate omitted-but-in-scope actions unknown to both enumerators. That is the first test that challenges the truth of the scope itself, rather than just conformance to a manifest both sides were handed.

## Reproducible execution

```sh
cargo fmt --manifest-path tools/orbital-observation-surface-bench/Cargo.toml -- --check
cargo test --locked --manifest-path tools/orbital-observation-surface-bench/Cargo.toml --all-targets
cargo clippy --locked --manifest-path tools/orbital-observation-surface-bench/Cargo.toml --all-targets -- -D warnings

cargo run --locked --manifest-path tools/orbital-observation-surface-bench/Cargo.toml --bin action-universe-closure -- \
  tools/orbital-observation-surface-bench/fixtures/v0.1/action-universe-closure-cases.json \
  --source-commit <40-character-commit-sha>
```

CI also runs the independent Node oracle, validates the generated receipt schema and compares Rust/Node outputs. A successful CI run is synthetic software conformance only.
