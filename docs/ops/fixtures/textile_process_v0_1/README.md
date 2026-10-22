# Textile Process Benchmark v0.1 fixtures

This directory freezes the first **synthetic contract corpus** for the Textile Process Optimization Benchmark.

## Files

- `textile_process_scenario.schema.json` defines the JSON shape and allowed evidence/admission states.
- `scenario_catalog.example.json` contains ten adversarial contract cases spanning scheduling, quality, circular recovery, durability, resilience, missing environmental authority, partial supply closure, provenance conflicts, determinism, and unknown impact factors.

## Scope limits

These are contract fixtures, **not yet executable numerical factory schedules**. The illustrative quantities are synthetic, and values such as electricity factors are not universal factors. They establish expected semantics and failure behavior only; they do not show that an optimizer exists, passes the cases, improves throughput, or saves resources.

The next implementation step is a small deterministic process/flow evaluator that loads separately frozen numerical instances, followed by independent-oracle checks on tiny instances. A future schema generation should have a distinct version and fixtures should be migrated only with explicit review.

## Review rules

1. Preserve `unknown` and `conflicting` as distinct states.
2. Do not turn unknown required evidence into zero or `PASS`.
3. Hard constraints are non-compensatory: a favorable objective vector cannot rescue a blocked candidate.
4. Keep material/fiber grade, observed performance, model prediction, supplier declaration, custody and lifecycle claim as different evidence types.
5. Never treat synthetic fixtures as observed factory data or use their values as public performance claims.
