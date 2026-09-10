# PB-01 Proposal IR — invariants and successor gates

PB-01 intentionally stops before physical authority.

The implementation candidate on `building/pb01-proposal-ir-v0.1` attempts these invariants:

1. **Exact intent identity** — canonical authored elements, exact coordinate frame, revision ancestry, source refs, proposer and site descriptor all bind the intent digest.
2. **Free placement without float identity** — authoring translation uses integer micrometres and rotation uses deterministic turn-phase coordinates.
3. **Frame drift is identity drift** — the same local numbers under a changed exact coordinate-frame digest produce a different exact intent.
4. **Same-revision authority conflict fails closed** — one `(authority, subject, revision)` cannot appear twice with competing digests in a canonical exact-input set.
5. **Exact planner provenance** — a plan binds the exact compiler/profile reference, not only a human version string.
6. **Deterministic operation DAG** — caller insertion order is irrelevant; the canonical topological order uses operation identity as the tie-break.
7. **Join dependency theorem** — a join of two newly-authored elements requires one realization operation for each and explicit dependencies on both.
8. **Stored plan re-resolution** — a persisted plan must be checked against the exact intent it names before a future authority crossing.
9. **Constraint facets remain non-authoritative** — hard failures and advisories are explicit, but PB-01 does not turn a caller-authored `HardFailure` into physical/safety authority.
10. **Projection cannot re-enter as truth** — `BuildProjection` is generated from exact intent + plan and is Serialize-only.
11. **Bounded atomic proposals** — one intent/plan is intentionally bounded; settlement/city scale must use hierarchy rather than an unbounded atomic payload.
12. **Unknown adapter profiles remain inert** — PB-01 exposes no generic execute-payload API.

## PB-02 gate

Do not implement the physical adapter from stale assumptions. Before PB-02:

- resolve the then-current qualified `symtropy-construction` / Fabrication / Universal Matter ingress;
- bind each supported PB-01 action to one explicit adapter profile/version;
- require current exact intent, plan, planning-context and adapter identities at PREPARE and again at COMMIT;
- separate material requirement from reservation/allocation;
- preserve idempotent acknowledgement-loss retry;
- prove a proposal cannot manufacture conserved matter, work completion, structural success, commissioning, ownership or permission;
- freeze at least one direct-vs-blueprint equivalence fixture showing both routes enter the same lower physical authority path.

## PB-03 gate

Snapping must be a deterministic suggestion/projection over explicit context. A snapped placement and a manually-authored numerically identical placement must compile to equivalent proposal semantics. Rendering cadence, camera state and hardware are invalid snap selectors.

## Evidence status

These are implementation/static invariants until an exact-head Rust 1.96 qualification run executes format, check, tests and strict Clippy successfully. Test design and queued workflows are not runtime evidence.
