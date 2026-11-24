# Manufacturing Policy v0.2 Incremental Prefix Gateway

**Package:** `mfg-policy-v02-prefix-gateway/0.2.0`  
**Benchmark:** `MFG-POLICY-V02-CHALLENGE-001`  
**Related:** [#7189](https://github.com/Luminous-Dynamics/symtropy/issues/7189), [v0.2 corpus PR #7279](https://github.com/Luminous-Dynamics/symtropy/pull/7279)

## Purpose

The v0.2 corpus separates three data planes:
1. Static policy case configuration.
2. Simulator-only complete scenario trajectories.
3. Evaluator-only hidden terminal outcomes.

This gateway is the first executable boundary between (1) and (2). It accepts the static corpus and simulator sidecar, selects a case and simulator branch inside the harness, then emits a decision frame containing only the static case and event observations through the requested epoch.

It is a prefix projection utility, not a simulator, scheduler, policy, evidence authenticator, MES adapter or machine controller.

## CLI

```text
mfg-policy-v02-prefix-gateway <policy-corpus.json> <scenario-streams.json> <case-id> <scenario-slot-0..3> <decision-epoch-0..3>
```

The simulator harness selects the scenario slot. The policy-facing frame intentionally omits:
- scenario identifiers and slot;
- scenario probabilities;
- complete future event trajectories;
- evaluator-only terminal-quality labels.

The frame exposes a deliberately narrowed operational projection—case identity, horizon, jobs, resources, setup transitions and inspection configuration—plus the observed history from epoch 0 through the requested decision epoch, inclusive. It strips scenario-family labels, split labels, replication/generator seeds, paired-seed IDs and visibility-rule metadata from the policy-facing frame.

## Invariants

- Both input files must pass their strict Draft 2020-12 schemas.
- Policy and stream benchmark, case and family identities must match.
- Scenario and decision epoch bounds fail closed.
- Event observations must have the four declared epochs in canonical order.
- The projection is a deterministic function of the static case and observable prefix.
- Identical visible prefixes from distinct simulator scenarios produce identical policy frames.
- Mutation of future events cannot change an earlier decision frame.
- The serialized frame contains no scenario slot, scenario ID, probability, family label, split label, generator/replication seed, paired-seed ID, or future event.

## Validation

The focused workflow runs formatting, eight Rust tests, a live generation-to-projection smoke test using the v0.2 generator, frame-schema validation, negative grep controls and strict Clippy. Paired-scenario equality is checked only through the current decision cutoff: a divergence in a not-yet-observed epoch cannot alter an earlier policy frame.

## Claim ceiling and remaining work

The gateway establishes only a synthetic information-boundary projection. It does not implement epoch-by-epoch policy execution, schedule state transitions, setup processing, capacity reservation, SLR2 selective repair, CP-SAT, an exact oracle, independent schedule replay, production evidence admission, or physical control. The simulator must still be built to advance an episode and call the policy once per epoch; a policy tournament and statistical protocol remain separate work.
