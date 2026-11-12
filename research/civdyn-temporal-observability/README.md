# CIVDYN temporal observability reference v0.1

A deterministic four-period benchmark that separates the information available when a policy must decide from the records that arrive later and improve only the retrospective archive.

It composes:
- CIVDYN closed-loop policy value (#4688 / PR #4737);
- policy-dependent outcome observability and MNAR bounds (#4750 / PR #4756);
- state-feature observability (#4759 / PR #4762);
- cost-aware active and contingent sensing (#4782 / PR #4798; #4832).

It does not implement a general stream processor, sensor clock synchronizer or actuator AoI measurement. It adopts the event-time / ingestion-time / processing-time distinction familiar from Apache Flink, but evaluates how late arrivals affect *historical policy information sets*.

## Frozen scenario

Two policies are run under two latent structural worlds across four periods. All service values lie in the declared support [0,100].

| Policy | Moderate-world trajectory | Severe-world trajectory | Observation process |
|---|---|---|---|
| `transparent_audit` | 60,60,60,60 | 60,60,60,60 | Four measurements delivered on time |
| `defer_suppress` | 100,20,20,50 | 100,0,0,50 | Period 0 timely; periods 1–2 suppressed; period 3 captured at event tick 3 but ingested at tick 5 |

The decision cutoff is tick 4. The retrospective archive is assessed as of tick 5.

## Known answers

For the transparent policy, both worlds have true mean 60, decision-time mean 60, retrospective mean 60 and 4/4 coverage.

For the defer-and-suppress policy:
- true mean is 47.5 in the moderate world and 37.5 in the severe world;
- decision-time complete-case mean is 100 with coverage 1/4;
- retrospective complete-case mean is 75 with coverage 2/4;
- decision-time [0,100]-support bounds are [25,100];
- retrospective bounds are [37.5,87.5].

The complete-case mean falsely ranks the defer-and-suppress policy above the transparent policy both at the decision cutoff (100 > 60) and retrospectively (75 > 60). The true policy value is lower in both worlds. But the identified intervals overlap 60, so the correct robust comparison is **unresolved**, not a fabricated winner.

The defer-and-suppress policy incurs three decision-time unobserved service-periods. After the late record arrives, retrospective missingness falls to two. The late event repairs the archive, not the information set that existed at the earlier decision.

## Candidate/evaluator information boundary

The candidate observation export contains only the policy label, event tick, whether an observation was visible by the decision cutoff, observed payload (if available) and cutoff tick. It does not contain hidden world IDs, latent truth, missingness cause, future ingest tick or retrospective data.

The evaluator export separately records world identity, capture/ingest receipts and hidden missingness causes. Historical candidate inputs are never backfilled.

## Metrics

Report true service value, decision-time and retrospective complete-case values, missing-as-zero value, support-based interval, truth containment, evidence coverage, decision-time observability debt, late-arrival count and robust interval comparison separately. Do not merge them into one composite score.

## External baseline semantics

Apache Flink distinguishes event time (when the event occurred), ingestion time (when the system received it) and processing time (when an operator processed it). Its documentation also explains watermarks as a completeness/latency trade-off, and late events can update an archive/window after the earlier result was emitted:
- https://nightlies.apache.org/flink/flink-docs-stable/docs/concepts/time/
- https://nightlies.apache.org/flink/flink-docs-stable/docs/learn-flink/streaming_analytics/

CIVDYN's extra requirement is that a retrospective correction must not silently rewrite the policy's prior decision input.

## Run

```bash
cargo test --locked --manifest-path research/civdyn-temporal-observability/Cargo.toml
cargo run --locked --manifest-path research/civdyn-temporal-observability/Cargo.toml --bin temporal-observability-sweep
```

## Claim ceiling

The reference qualifies only deterministic timeline semantics and finite support-bound arithmetic. It does not establish real sensor delivery distributions, identify real MNAR mechanisms, validate empirical policy effects, or claim SOTA superiority.
