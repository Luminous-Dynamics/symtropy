# Tick transaction formalization: scope and evidence

## Why this slice

Symtropy already has a proof-carrying physics RFC (#1682), a pinned Lean engineering prototype (#1691), and a Rust tick-transaction primitive under review (#1726). This contract connects these strands without duplicating the broad RFC or overstating what Lean proves.

The abstract model in \`formalization/lean/SymtropyFormal/TickTransactions.lean\` covers six lifecycle properties: strictly increasing logical tick IDs, continuity of prior/result state digests, legal phase ordering, rejection of finalization before physics, idempotent same-result retries, and rejection of conflicting retries.

## Research transfer from OpenAI's public proof work

OpenAI's public [math repository](https://github.com/openai/math) is a useful process reference: it separates manuscript claims from Lean artifacts, catalogs formalized results, pins build inputs, preserves revisions, and explicitly warns that unformalized manuscripts may contain errors. We adopt the *verification workflow*, not unrelated mathematical results or a claim that a frontier-model proof is correct merely because it was generated.

The Symtropy adaptation is deliberately narrower than research theorem discovery:
1. Write an explicit invariant and its assumptions.
2. State a model in a pinned proof assistant.
3. Keep a negative control for each important reject condition.
4. Record theorem names and status in a machine-readable manifest.
5. Test the implementation against shared known-answer cases.
6. Establish a refinement relation before calling the implementation verified.

## Verification ladder

| Level | Evidence | Permitted claim |
|---|---|---|
| 0 | Narrative or informal model | Hypothesis / design intent |
| 1 | Executable reference model and hand-checkable cases | Reference behavior for covered inputs |
| 2 | Seeded property and mutation tests against independent oracles | Tested implementation properties for the tested domain |
| 3 | Lean theorem checks in the pinned environment | The named abstract propositions are machine-checked |
| 4 | Shared cross-language corpus, independent Rust conformance and mutation testing | Implementation matches the modeled contract over the tested corpus |
| 5 | Proof/refinement linking implementation semantics to the model, plus separate numerical and empirical validation | Only the specific properties and assumptions covered by those artifacts |

A result must not skip levels through wording alone. A passing Lean build at level 3 does not establish level 4 or 5.

## Current boundary

The Lean model uses natural numbers as opaque digest stand-ins. This keeps the specification independent of the hash implementation, but it is not the cryptographic byte-array representation used by Rust. No Rust-to-Lean refinement relation is established in this change. The source mapping pins the observed open #1726 head only to make the intended comparison explicit; it is not proof that the branch will remain unchanged or that its runtime behavior conforms.

This model does not prove:
- digest authenticity, uniqueness, or collision resistance;
- rollback of physics side effects;
- physical conservation, solver correctness, or scientific validity;
- equality of floating-point results across processors;
- correctness of callers that compute canonical input/state digests.

## Benchmarks and gates

The first benchmark is correctness before speed: zero accepted illegal transitions, zero stale-ID accepts, zero state-chain-linkage violations, and zero conflicting-finalize accepts across exhaustive small state combinations and seeded traces. Report coverage counts and rejected-case counts, not just a pass percentage.

For a future runtime trace campaign, target:
- 100% rejection of enumerated illegal state transitions;
- 100% receipt continuity on valid traces;
- 1,000,000-tick deterministic replay with zero trace mismatches for a fixed build/platform/configuration;
- p50/p95/p99 transition overhead and bytes per receipt at 1k, 10k, and 100k ticks;
- mutation score at least 95% for targeted lifecycle faults, with every surviving mutation documented.

These are proposed gates, not measured results. Performance is a secondary goal; correctness and traceability gates cannot be traded away for throughput.

## References

- [OpenAI math repository and Lean catalogue](https://github.com/openai/math)
- [Symtropy proof-carrying physics invariant RFC (#1682)](https://github.com/Luminous-Dynamics/symtropy/pull/1682)
- [Symtropy Lean housing formalization (#1691)](https://github.com/Luminous-Dynamics/symtropy/pull/1691)
- [Symtropy tick transaction primitive (#1726)](https://github.com/Luminous-Dynamics/symtropy/pull/1726)
- [THERMO-PROOF-001 issue (#1755)](https://github.com/Luminous-Dynamics/symtropy/issues/1755)
