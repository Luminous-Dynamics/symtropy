# Power restoration benchmark run receipt v0.1

**Owner:** Symtropy #6352  
**Case contract:** #6346 / PR #6347  
**Action contract:** #6348 / PR #6350  
**FLARE comparator:** #6363  
**Programme:** #5241

## Purpose

A planner's score is not a benchmark result. This crate validates a content-addressed run receipt connecting one frozen case, planner and adapter identity, decision observations, action proposals, transition attempts, independent validation, service scoring, execution budget and hardware metadata.

It checks provenance and stage accounting. It does not run a planner, simulator, electrical solver or service model.

## Required anti-survivorship semantics

There must be exactly one ordered decision record for every case time step. A step is either:
- `executed`: contains an observation and explicit outcomes for planner, transition, validator and service stages; or
- `not_run_after_termination`: explicitly identifies why the run completed before the remaining horizon; or
- `not_run_after_abort`: explicitly identifies why the remaining horizon was not reached after abort, timeout or incomplete evidence.

The receipt rejects missing steps, duplicate/out-of-order steps, a not-run record containing fabricated execution payload, a proposal whose case/observation/epoch differs, or a transition marked applied without an accepted independent validator receipt.

Every timeout, malformed output, abstention, rejection, failed transition and human override remains a counted event. A malformed output has a raw output artifact digest and no parsed proposal. A timeout/abstention/error is not converted into a successful empty plan.

## Identity and run pairing

Each run binds:
- exact canonical case digest and maturity;
- paired scenario group, seed, replicate and observation profile;
- planner source commit, model/provider/revision, prompt and configuration digest;
- action adapter, simulator, independent validator and scorer version/config hashes;
- objective ID/expression/unit from the case;
- wall-clock/solver/token/API budgets, per-step and run-level observed usage totals, and, when measured, hardware/runtime details.

Synthetic fixtures mark hardware as not measured; no fake CPU/GPU capacity is accepted.

## Score semantics

A completed run requires a final service trajectory reference and an independent aggregate scorer receipt whose metric and unit match the case objective. A timed-out/aborted/incomplete run may retain per-step receipts but cannot publish a completed aggregate score or final trajectory.

Service metric values are recorded with explicit units. The receipt makes no cross-unit sum across energy, water volume, telecom customer-hours or financial cost.

## Synthetic fixture

The included 24-step case records:
- step 0: one structurally valid synthetic repair proposal, validator receipt, transition receipt and service trajectory;
- steps 1–23: explicit not-run-after-restoration-complete records.

Every identity is synthetic. The example is a schema/receipt test, not an executed simulation or measured service restoration.

## Observed usage and budget integrity

Every executed decision must explicitly record planner latency, token counts as a pair, API cost (zero for local/no-charge planners), and optional solver time/gap as a pair. The run-level elapsed wall-clock, token, API-cost, solver-time and maximum-gap totals are recomputed from the complete decision ledger and must match. A completed run cannot exceed its declared wall-clock, solver-time, solver-gap, token or API-cost budget. Budget-exceeded runs can still be recorded as timed out/aborted/incomplete, never as successful completed runs.

Not-run reasons must match the run-level terminal reason. This stops a run from labeling a timed-out tail as if restoration had been achieved.

## What it verifies

- case schema/semantic validation and exact case digest;
- full-horizon decision record coverage and ordering;
- explicit attempted/not-run accounting;
- proposal is validated against the same case and step observation;
- proposal planner identity equals run-level planner identity;
- independent validator result receipt present where a validator ran;
- applied/failed transitions require accepted validation and a transition receipt;
- service-scored steps include a trajectory receipt and unit-bearing metrics;
- completed-run score/final trajectory is mandatory; incomplete runs cannot claim completed score;
- hardware measured/not-measured honesty;
- deterministic canonical receipt SHA-256.

## Limitations

A SHA-256 identifies bytes; it does not authenticate the issuer. Production receipts need signed producer attestations or a trusted immutable registry. Artifact URIs must resolve to immutable content outside this crate. The run receipt does not verify that an external scorer actually ran its claimed code; it binds the scorer identity and receipt digest for an independent verifier to check.

## Verification

There are 21 authored Rust unit tests. No Rust toolchain is available in the authoring environment, so tests are not locally executed. Dedicated exact-head CI performs schema validation, cross-file checks, Rust formatting, Clippy, tests and CLI replay. No pass is claimed before that run completes.

## Claim boundary

Benchmark run provenance and stage-accounting receipt only. It does not establish physical feasibility, authority, electrical safety, or field service restoration.
