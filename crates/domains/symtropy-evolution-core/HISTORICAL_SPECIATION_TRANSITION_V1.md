# Historical Speciation Transition V1

## Scope

SEL-10D represents **model-bound evidence for a historical speciation transition interval**. It is deliberately downstream of SEL-10A lineage-divergence history, SEL-09B reproductive-isolation evidence, SEL-10B biological-species model authority, and SEL-10C current species status.

The governing separation is:

`current species status != historical transition evidence != exact transition time != universal species truth`.

A present-day or current-window species classification is neither a proof that a historical transition occurred nor a timestamp for such a transition.

## V1 subject

A `SpeciationTransitionDesign` freezes, before transition evidence is evaluated:

- the exact ordered lineage pair;
- the exact SEL-10A lineage-history design digest;
- the exact SEL-09B reproductive-isolation design digest;
- the exact SEL-10C current-species classification design digest;
- the exact SEL-10B model digest, model-content digest, and validity-domain digest;
- the complete SEL-10A history bounds;
- a candidate transition interval;
- the immediately adjacent pre-transition and post-transition generations;
- the exact temporal evidence protocols;
- the temporal missing-evidence policy; and
- the built-in V1 transition rule authority.

The design contains no realized transition status.

## Exact transition time is intentionally unrepresentable

V1 rejects a one-generation candidate interval. A candidate transition interval must span at least two generations and must leave at least one SEL-10A generation before and after it.

Therefore V1 can represent:

`candidate_start <= unresolved historical transition <= candidate_end`

but cannot represent a field such as `transition_generation`, `event_time`, or `speciation_time`.

A future authority may narrow temporal resolution only if new evidence actually supports that stronger theorem. SEL-10D V1 must not manufacture it.

## Required temporal criteria

Every materialized transition evidence object contains exactly seven criteria in canonical order:

1. `PreTransitionCommonSource`;
2. `DivergenceTiming`;
3. `ReproductiveBarrierTiming`;
4. `DemographicHistory`;
5. `IntervalCompleteness`;
6. `ModelApplicability`; and
7. `LaterCounterHistory`.

Caller-selected windows are not accepted. Each criterion has a deterministic generation window derived from the frozen design.

Each criterion record binds:

- the exact evidence protocol;
- the evidence authority/content identity;
- a separate qualification protocol and qualification authority/content identity;
- the transition-design digest;
- the exact SEL-10A history digest;
- the exact SEL-09B isolation-evidence digest;
- the exact SEL-10C current-status digest; and
- the exact SEL-10B model digest.

Hashes and authority references bind identity and provenance. The core does **not** infer scientific competence, correctness, or truth merely because a reference or digest exists. Those judgments remain external qualification responsibilities.

## Historical and current model applicability are different claims

SEL-10C applicability concerns the target pair under the model for the current classification context.

SEL-10D `ModelApplicability` concerns the model over the historical candidate interval.

They may differ. A lineage pair can be outside a model's present-day/current applicability while historical evidence for the candidate interval remains within scope, or the reverse.

Only the interval-bound `ModelApplicability` temporal criterion can produce SEL-10D `OutsideModelValidityDomain`.

Current SEL-10C status remains an exact identity-bearing input and replay dependency, but it is not a hidden historical classifier.

## Status semantics

V1 exposes:

- `TransitionSupportedUnderModel`;
- `TransitionNotSupportedUnderModel`;
- `TransitionContradictedUnderModel`;
- `InsufficientTemporalEvidence`;
- `TransitionIntervalOnly`; and
- `OutsideModelValidityDomain`.

`OutsideModelValidityDomain` has highest applicability meaning when the historical `ModelApplicability` criterion says the candidate interval is outside the frozen model domain.

A qualified contradiction in any temporal criterion yields `TransitionContradictedUnderModel`.

A qualified `DoesNotSupport` result yields `TransitionNotSupportedUnderModel`.

Under `ReportInsufficientTemporalEvidence`, unavailable evidence yields:

- `TransitionIntervalOnly` when the interval bracket itself is supported by pre-transition common-source, divergence-timing, interval-completeness, and historical model-applicability evidence but one or more mechanism-level criteria remain unavailable;
- otherwise `InsufficientTemporalEvidence`.

Under `FailClosed`, unavailable temporal evidence is rejected rather than classified.

A fully supported exact seven-criterion surface yields `TransitionSupportedUnderModel`.

## Later counter-history is preserved, not erased

A supported historical transition does not imply permanent reproductive isolation, irreversible lineage separation, or zero future gene flow.

SEL-10D mechanically extracts later counter-history after the candidate interval from the embedded SEL-10A and SEL-09B evidence surfaces where observable, including:

- recontact;
- realized gene flow;
- admixture or introgression episodes;
- lineage fusion or remerger;
- lineage loss or extinction; and
- viable fertile cross-lineage offspring.

These observations are canonical, identity-bearing evidence and remain present even when the historical transition itself is supported.

The `LaterCounterHistory` temporal criterion means that the later-history evidence surface is qualified and sufficiently accounted for under its protocol. `Supports` **does not mean that the later history is empty**.

Later fusion therefore does not rewrite an earlier supported transition out of history. It is retained as later counter-history. Whether a later event changes current species status is a separate SEL-10C question.

## Persisted evidence versus current authority

`SpeciationTransitionEvidence` is serializable so evidence can be archived and inspected. It embeds:

- the full frozen transition design;
- the full SEL-10A lineage-history snapshot and digest;
- the full SEL-09B isolation-evidence snapshot and digest;
- the full SEL-10C current-status snapshot and digest;
- the full SEL-10B model snapshot and digest;
- all seven temporal evidence records;
- mechanically derived later counter-history; and
- the derived transition status.

Local validation recomputes these bindings, the fixed criterion windows and order, the counter-history surface, and the status theorem. Serialized status fields cannot self-authorize.

`ValidatedSpeciationTransitionEvidence<'_>` is intentionally non-Serde. Current authority can be regained only by replaying the persisted object against fresh current SEL-10D design, SEL-10A history, SEL-09B isolation evidence, SEL-10C current status, SEL-10B model, and fresh temporal evidence inputs.

## Adversarial invariants

V1 fails closed against at least these classes of error:

- a one-generation exact transition claim;
- a candidate interval lacking before/after history coverage;
- duplicate or missing temporal criteria;
- caller-chosen temporal windows;
- temporal evidence bound to a different transition/history/isolation/current-status/model subject;
- evidence or qualification protocol drift;
- `OutsideValidityDomain` attached to anything except historical `ModelApplicability`;
- unavailable evidence under `FailClosed`;
- serialized transition-status tampering;
- serialized upstream digest tampering;
- deleting or rewriting mechanically derived later counter-history; and
- stale current-authority replay.

## Explicit non-claims

SEL-10D V1 does not establish:

- an exact generation, timestamp, or instant at which speciation occurred;
- a universally correct species concept;
- irreversible reproductive isolation;
- permanent absence of introgression or gene flow;
- a mechanism beyond what the qualified temporal criteria support;
- scientific truth merely from hashes or signatures; or
- that a supported historical transition implies the two lineages remain distinct species today.

Its strongest positive statement is narrower:

**Under the exact frozen species model, exact upstream evidence identities, exact candidate interval, and exact qualified temporal evidence surface, the historical speciation-transition claim has the reported V1 status.**
