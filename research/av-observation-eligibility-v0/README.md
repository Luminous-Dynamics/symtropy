# AV Observation Eligibility Benchmark v0

**Issue:** #6253  
**Compose:** Symtropy #6201/#6183, camera producer #4595/#4616, acoustic/PCM continuity Symthaea #10894/#10788, clock transform Symthaea #621 / Symtropy #633, response window Symthaea #5169/#12837.

## The key distinction

An observation has at least three independent properties:

1. **Transport outcome:** Delivered, Reordered, Duplicate, StaleReplay, Dropped.
2. **Source freshness:** Fresh, Delayed, Stale, Incomparable, NotAssessed.
3. **Decision-time eligibility:** usable for this response window, late but historically available, unavailable at the as-known cutoff, superseded for the current task, or clock-incomparable.

Never map one enum onto another. A sample can be Reordered but Fresh; Delivered but Stale; Duplicate and Stale; or stale for the live response but usable for a later historical query. A duplicate does not create a new measurement or refresh the source age.

## Clock rule

Compute age only when source and arrival are on the same declared clock generation or a separately qualified transform exists. Equal numeric timestamps from different domains/generations do not establish comparability. When incomparable, report Incomparable rather than inventing an age.

## Fixtures

- `fixtures/agent/workload-v1.json`: only the event history/current delivery and public freshness/decision policies available at the decision point.
- `fixtures/evaluator-only/outcomes-v1.json`: hidden source/arrival truth, future schedule and oracle labels.
- `fixtures/expected/known-answer-v1.json`: author-authored expected three-axis classification.

Directory separation is packaging, not OS isolation. Agent runtime must not mount evaluator-only artifacts.

## Ownership boundary

- #6201 owns artifact references, sensor metadata and causal replay/transport events.
- #4595/#4616 owns the bounded software camera's Fresh/Delayed/Stale timing-policy profile and policy receipt.
- Symthaea #10894/#10788 owns audio sample continuity and loss-accounted PCM.
- #621/#633 own clock mapping and dependence-aware fusion.
- #5169/#12837 own interaction deadlines and AV task semantics.
- #9075/#10644/#9177 own historical/as-known retrieval and evidence sufficiency.

The benchmark does not add a clock service, stream controller, sensor driver or memory engine.

## Known-answer cases

1. Reordered but fresh delivery.
2. In-order first delivery that is stale by age.
3. Duplicate that cannot refresh freshness.
4. Stale replay that cannot update current state.
5. Missed immediate response deadline but valid historical retrieval.
6. Clock reset / incomparable generation.
7. Explicit dropped audio interval that must not be filled by previous samples.
8. Valid observation arriving after the task revision was superseded.

## Qualification

The dedicated workflow checks JSON syntax, aligned episode identities, forbidden evaluator-only fields in public inputs, and the three-axis known-answer counterexamples. It does not qualify physical clocks, camera/microphone capture, clock-transform accuracy or policy performance.
