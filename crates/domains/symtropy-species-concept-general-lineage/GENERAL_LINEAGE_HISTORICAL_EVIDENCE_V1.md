# General-Lineage Historical Evidence V1

## Status

SEL-10F1A V1 materializes a preregistered, replay-qualified temporal evidence ledger for the general-lineage species-concept family.

This contract is stacked directly on frozen general-lineage product subject:

`6aafa8cba53cce79b01b87562b3d8b4d5518de59`

The F1A candidate is not scientifically qualified merely because this document or the implementation exists. Qualification requires the exact candidate subject to pass the isolated execution gate defined below.

## Governing theorem

```text
current general-lineage species status
    != historical lineage-separation transition

persistent lineage history
    != complete temporal multi-channel evidence

complete historical evidence
    != historical transition classification
```

F1A therefore has no `TransitionSupported` result and no historical-transition classifier.

## Scope

F1A is an evidence-materialization authority only. It binds one exact current general-lineage classification design to one exact SEL-10A lineage-history subject and materializes the current evidence-channel ontology across a preregistered temporal interval.

F1A does not establish:

- a speciation event instant;
- historical-transition support, contradiction, or non-support;
- a universal species threshold;
- nomenclature or taxonomic truth;
- a scalar historical species score;
- majority voting across channels;
- a third species concept;
- scientific authority from persisted bytes alone.

## Outcome-free historical design authority

`GeneralLineageHistoricalEvidenceDesign` is frozen before historical outcomes are interpreted.

It binds:

- the exact current `GeneralLineageClassificationDesign` and digest;
- the exact SEL-10A history start and end generations;
- the exact lineage pair already bound by that history design;
- a candidate interval of at least two generations;
- at least one history generation before the interval;
- at least one history generation after the interval;
- every current general-lineage evidence channel;
- every current channel's exact ID, kind, role, dependency-group identity, dependency-group qualification, protocol authority, and applicability authority;
- one channel-specific temporal-projection protocol per current channel;
- a preregistered minimum independent temporal support-group threshold of at least two;
- common-source qualification protocol;
- historical-context qualification protocol;
- counter-history completeness protocol;
- missing-evidence policy.

The historical projection may add temporal semantics only. It may not redefine current channel ontology.

```text
historical projection
    cannot redefine
current channel ontology
```

An optional current channel cannot become required historically. A dependency source cannot be relabeled after observing its trajectory. A native channel cannot be replaced by an opaque external assertion.

## Canonical temporal windows

Every channel is materialized in exactly three ordered windows:

1. `BeforeCandidateInterval`
2. `CandidateInterval`
3. `AfterCandidateInterval`

Window bounds are derived from the preregistered design rather than supplied by evidence producers.

The F1A reference corpus uses SEL-10A generations 1 through 5 with candidate interval generations 2 through 3:

```text
generation 1 | generations 2-3 | generations 4-5
     1       |        2        |        2
```

A one-generation candidate interval is forbidden because F1A is a transition-evidence tranche, not an exact-event detector.

## Native SEL-10A longitudinal evidence

The core `LongitudinalLineageSeparation` channel must materialize directly from current validated SEL-10A history.

The ledger preserves every source generation record at native resolution. It does not reduce the history to an aggregate score or a transition verdict.

For each canonical window, generation records must be complete and in exact generation order. The ledger also binds the exact current SEL-10A history digest.

The reference corpus preserves a lineage fusion at generation 5 in the post-interval window. That fusion remains explicit counter-history; F1A is forbidden from converting it into a historical-transition classification.

## Native SEL-09B reproductive-isolation evidence

When the current classification preregisters a `ReproductiveIsolation` channel, F1A requires current validated SEL-09B evidence for the same lineage pair.

The ledger preserves individual reproductive opportunities, including:

- study-unit identity;
- opportunity generation;
- contact-study digest;
- complete native contact record.

Opportunities are partitioned by generation into the same before/candidate/after windows and canonically ordered by generation, study unit, and opportunity identity.

Opportunities outside the SEL-10A historical bounds are counted explicitly rather than silently discarded.

SEL-09B aggregate status is not a historical-transition verdict.

## Externally qualified temporal channels

Current evidence channels without a native historical authority remain explicit external temporal evidence.

Each external window binds:

- the preregistered temporal-projection protocol;
- one explicit disposition;
- observation authority;
- qualification authority;
- design-derived window bounds.

The V1 disposition vocabulary is evidence-level only:

- `SupportsLineageSeparation`;
- `DoesNotSupportLineageSeparation`;
- `ContradictsLineageSeparation`;
- `Unavailable`;
- `OutsideChannelDomain`.

These dispositions describe a channel window. They are not F1B transition outcomes.

Under `FailClosed`, `Unavailable` is rejected. Under `ReportIncompleteEvidence`, unavailable evidence remains explicit and cannot silently become support, contradiction, or non-support.

## Persisted identity versus current scientific authority

`GeneralLineageHistoricalEvidenceLedger` is serializable representation. Its canonical digest binds:

- ledger version;
- exact historical-design digest;
- common-source evidence authority;
- historical-context evidence authority;
- counter-history completeness evidence authority;
- every ordered channel declaration;
- every native or external temporal evidence record.

Native SEL-10A and SEL-09B records participate in ledger identity through their frozen serialized representation. `serde_json` is therefore a production representation dependency.

Representation identity is not scientific authority.

`ValidatedGeneralLineageHistoricalEvidence` is deliberately non-Serde and is obtainable only through fresh `validate_current` replay.

Fresh replay must reconstruct the ledger from:

- current validated F1A design;
- current validated SEL-10A history;
- current validated SEL-09B evidence when preregistered;
- the exact current external temporal evidence inputs;
- current common-source evidence authority;
- current historical-context evidence authority;
- current counter-history completeness evidence authority.

The reconstructed ledger must equal the persisted ledger exactly.

```text
persisted ledger
    + local structural validity
    + canonical digest
    != current F1A qualification

current F1A qualification
    = exact fresh authority replay
      + exact ledger equality
```

## Restored-state adversarial boundary

The adversarial restored-state corpus freezes the distinction between structural validity and current authority.

It must demonstrate that a round-tripped persisted ledger can remain locally well-formed and digestable while still being rejected for current qualification after any scientifically material stale-state mutation.

V1 adversarial cases include:

1. changing an externally qualified candidate-window disposition while preserving valid structure;
2. changing a persisted evidence authority while preserving its preregistered protocol and nonzero revision;
3. replacing the persisted native SEL-10A history digest with the digest of a different valid history while preserving structurally valid embedded windows.

In every case, local canonical identity may still be computable, but `ValidatedGeneralLineageHistoricalEvidence::validate_current` must reject the restored state with `LedgerReplayMismatch` when replayed against the unchanged current authorities.

This is intentional:

```text
restorable bytes
    != restorable trust
```

## Counter-history preservation

The post-candidate window is part of the scientific subject, not cleanup metadata.

Later fusion, renewed gene flow, demographic reconvergence, loss of diagnosability, or other preregistered counter-history must remain observable at source resolution when the underlying authority provides it.

F1A therefore cannot select only the interval that looks most transition-like and discard later contradictory history.

## Independence boundary

F1A preserves the current classification's dependency-group ontology and preregisters a minimum independent temporal support-group threshold, but F1A does not itself emit a transition verdict from that threshold.

At least one temporal evidence channel beyond mere reuse of the SEL-10A trajectory is required for a scientifically meaningful later transition classifier. Multiple windows or multiple transformations of one source do not manufacture independence.

F1B, if qualified later, must consume F1A's exact typed evidence and must not retroactively alter F1A channel grouping or temporal projections.

## Exact isolated candidate gate

No compile, test, Clippy, or scientific PASS exists for an F1A subject until the exact frozen candidate commit is executed in isolation.

The minimum product gate is:

```text
cargo check --manifest-path crates/domains/symtropy-species-concept-general-lineage/Cargo.toml
cargo test --manifest-path crates/domains/symtropy-species-concept-general-lineage/Cargo.toml
cargo clippy --manifest-path crates/domains/symtropy-species-concept-general-lineage/Cargo.toml --all-targets -- -D warnings
```

The qualification receipt must bind the exact candidate commit SHA and must execute from that exact subject. A queued job with no executed steps is not evidence. A helper branch is not product authority. A later commit cannot inherit an earlier commit's PASS.

## Freeze rule

Before publication, F1A must satisfy all of the following:

- exact ancestry from `6aafa8cba53cce79b01b87562b3d8b4d5518de59` is reviewed;
- the exact changed-file diff is reviewed;
- no historical-transition classifier or transition result type has entered the F1A subject;
- restored-state adversarial coverage is present;
- this contract is present in the exact product subject;
- the product branch is frozen to one exact candidate SHA;
- an isolated exact-subject qualification run executes the minimum product gate;
- any helper is never-merge and cannot become product ancestry;
- publication records executed evidence only.

Until those conditions are satisfied, F1A remains a candidate rather than a qualified historical-evidence authority.
