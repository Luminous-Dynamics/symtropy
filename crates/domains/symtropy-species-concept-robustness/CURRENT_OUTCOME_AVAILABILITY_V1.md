# SEL-10E2C — Current outcome availability qualification v1

## Governing theorem

```text
no current result supplied
    !=
qualified current-result unavailability
```

and:

```text
qualified unavailability
    !=
latent support
    !=
latent contradiction
    !=
latent non-support
```

SEL-10E2B already requires one retained row for every preregistered concept family. When a current family result is unavailable, the row remains visible rather than disappearing. SEL-10E2C adds a separate authority for the *availability claim itself*.

This layer does not change any species-model result and does not recompute the E2B robustness conclusion.

## Outcome-free policy

`OutcomeAvailabilityPolicy` is declared against a current SEL-10E2A authority before downstream consumers rely on an E2B report.

V1 binds:

- exact SEL-10E2A design digest;
- policy identity;
- exact unavailable-result observation protocol authority;
- built-in V1 availability rule authority.

A persisted policy is representation only. `ValidatedOutcomeAvailabilityPolicy` regains current authority only by replaying against the fresh current E2A authority and exact observation protocol.

Changing only the observation protocol changes policy identity and stales current replay.

## Exact report binding

`CrossModelOutcomeAvailabilityLedger` binds one exact current E2B report by:

- complete E2B report snapshot;
- exact report digest;
- exact availability policy snapshot/digest;
- exactly one availability record per report row;
- built-in availability rule authority.

The report and policy must bind the same exact E2A design.

## Availability records

Each record retains:

- exact conceptual identity;
- exact descriptor digest;
- exact model-specific evidence-surface digest;
- exact E2B row disposition;
- exact frozen observation protocol;
- typed availability disposition;
- optional unavailability observation authority;
- optional unavailability qualification authority.

V1 has only two availability dispositions:

```text
ObservedCurrentOutcome
QualifiedUnavailable
```

### Observed row

If the E2B row contains an actual current family result, the availability record must be `ObservedCurrentOutcome` and must contain no unavailability observation or qualification authority.

Supplying unavailability evidence for an observed row fails closed.

### Missing row

If and only if the E2B row is `MissingCurrentCapability`, the availability record must be `QualifiedUnavailable` and must bind both:

- a nonzero observation authority; and
- a nonzero qualification authority.

A missing row without both authorities fails closed.

This qualification says only that the current result was unavailable under the frozen observation protocol. It says nothing about what that unavailable model would have concluded.

## Completeness

No model may silently disappear from the availability surface.

The ledger must retain exactly one record for every E2B row in the exact canonical report order. Record conceptual identity, descriptor digest, evidence-surface digest and row disposition must all match the corresponding E2B row.

Unexpected, duplicate, wrong-subject or extra unavailability inputs fail closed.

## Representation versus current authority

Persisted policy, report and availability bytes are not current authority.

`ValidatedCrossModelOutcomeAvailability` requires fresh:

1. current SEL-10E2A authority through `ValidatedOutcomeAvailabilityPolicy`;
2. current validated SEL-10E2B report;
3. exact unavailability inputs for every and only missing report row.

Current replay reconstructs the entire availability ledger and requires byte-semantic equality with the persisted representation.

Qualification-only drift therefore changes ledger identity and cannot regain current authority against stale bytes.

## No robustness-status mutation

SEL-10E2C does not add, remove, upgrade or downgrade any E2B robustness state.

In particular:

```text
availability-qualified missingness
    !=
additional independent model coverage
```

and:

```text
availability qualification
    !=
robust species support
```

The E2B report status is retained verbatim inside the exact report snapshot.

## Trust boundary

The external observation and qualification authority references are explicit trust edges. Their hashes bind identities/provenance; this crate does not independently prove the truth of the external observation.

The software theorem is narrower: a downstream consumer cannot obtain E2C current authority while silently treating an unqualified missing row as qualified unavailability.

## Adversarial requirements

V1 must reject or expose:

- missing row without qualified unavailability;
- unavailability asserted for an observed row;
- wrong conceptual identity;
- wrong report or policy binding;
- observation-protocol drift;
- qualification-only drift;
- missing availability record;
- forged availability disposition;
- tampered row identity/evidence-surface identity;
- stale persisted bytes replayed against different current authorities.

Observed reports with no missing rows require no unavailability assertions.

## Nonclaims

SEL-10E2C does not establish:

- the latent conclusion of any unavailable model;
- majority-vote taxonomy;
- stronger E2B independent coverage;
- historical-transition robustness;
- nomenclatural authority;
- a philosophical winner among species concepts;
- universal species truth.
