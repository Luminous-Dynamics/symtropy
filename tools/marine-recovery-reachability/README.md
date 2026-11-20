# Marine Bounded-Action Recovery Reachability Auditor

Standalone Rust 1.96-compatible evaluator for Symtropy issue #6298. It extends the sampled trajectory and inter-sample envelope stack with **post-trigger recovery-path qualification**.

## Commands

```sh
cargo fmt --manifest-path tools/marine-recovery-reachability/Cargo.toml -- --check
cargo clippy --manifest-path tools/marine-recovery-reachability/Cargo.toml --all-targets -- -D warnings
cargo test --manifest-path tools/marine-recovery-reachability/Cargo.toml
cargo run --manifest-path tools/marine-recovery-reachability/Cargo.toml -- audit-recovery-graph tools/marine-recovery-reachability/fixtures/synthetic-recovery-graph-0001.json
```

## What it verifies

- A digest-bound, vessel-specific service/rate contract, hard-gate roster, exact one-scenario-per-trigger roster and capability-qualification registry bound to the exact operating-envelope scope.
- A trigger-time resource state with conservative uncertainty intervals.
- Bounded action-graph reachability from the current node to a declared terminal.
- Typed action classes, trigger/component/capability preconditions, exact hard-gate coverage, action evidence and digest. A capability must be in the qualified registry and currently available in the trigger state before an action can use it.
- Per-axis successor intervals from bounded resource deltas.
- An exact two-cone inter-sample envelope during every bounded action duration, using the mission's declared maximum absolute rate per hour. Both endpoints and the entire action interval must remain within all service floors/ceilings.
- Total worst-case recovery time must fit within the intervention deadline.
- Human handover is a distinct terminal: explicit acknowledgement, named authority role, communications evidence and worst-case response latency must all fit inside the remaining intervention window. The remaining intervention margin is part of the receipt.
- Search is bounded by action count, path count, path depth and total expanded edges. Reaching a bound returns a top-level SEARCH_INCOMPLETE disposition and scenario-level SEARCH_INCOMPLETE/ABSTAIN rather than claiming exhaustive reachability.
- Safe-haven, shore-resupply, repair-restoration, controlled-safe-hold and human-handover terminals count only when destination/access, required service availability, authority and terminal evidence are verified and unexpired.
- Every rejected action edge and every feasible recovery path is retained. No weighted path score or aggregate utility is produced.

## Synthetic scenario results

| Fault trigger | Expected result | Why |
|---|---|---|
| Main generator trip | RECOVERABLE | Two continuations survive: load-shed then safe haven, and acknowledged remote-master handover. Fast diversion and generator repair are rejected |
| Watermaker trip | RECOVERABLE | Shore-resupply path reaches a port with verified access, capacity, service and authority evidence |
| Common-cause generator + watermaker outage | INFEASIBLE/ABSTAIN | The candidate diversion breaches both energy and freshwater floors; the controlled-hold alternative depends on unavailable generation |
| Intervention deadline exhausted | INFEASIBLE/ABSTAIN | The only route arrives after the declared intervention deadline |
| Unverified port services | INFEASIBLE/ABSTAIN | Geographic reachability is not terminal success when required shore service evidence is unverified |

The human handover fixture has a 2-hour worst-case transfer duration plus a 0.5-hour response upper bound against a 6-hour intervention deadline; remaining margin is emitted explicitly. Missing acknowledgement or a 5-hour response bound makes that handover non-viable. The generator-repair skill is qualified in the mission registry but unavailable in the generator-trip state, so the repair edge is rejected for availability rather than misclassified as an unqualified skill. All numbers, action deltas, rate bounds, authority receipts and terminal evidence are synthetic known-answer fixtures.

## Industry and research boundary

- 2026 adaptive-hex/ensemble ship-routing work already handles member-coherent CVaR and edge-level IMO second-generation stability pruning (Zhang et al., *Ocean Engineering*, 15 Oct 2026, DOI: https://doi.org/10.1016/j.oceaneng.2026.127661).
- August 2026 work couples all-electric voyage planning and energy management with day-ahead and intraday optimization (Gao et al., *Journal of Energy Storage*, 30 June 2026, DOI: https://doi.org/10.1016/j.est.2026.122206).
- Recent safe-control literature moves beyond a nominal plan to set-based reachability and uncertainty-qualified safety. A November 2026 *Automatica* paper derives finite-sample reachable-set guarantees for GP dynamics (DOI: https://doi.org/10.1016/j.automatica.2026.113204); a September 2026 robust data-driven MPC paper separates epistemic and aleatoric uncertainty in analytic probabilistic safety barriers (DOI: https://doi.org/10.1016/j.rico.2026.100789).

The differentiator to test is not a new route optimizer. It is a **separate recovery verifier** that requires an evidenced, authorized, resource-safe continuation after a fault, and abstains before the last intervention point when no such path is demonstrated.

The 20 November increment adds a pinned Rust workflow and **24 authored Rust semantic/negative-control tests** for trigger coverage, qualified-versus-available capabilities, handover acknowledgement and timing, action authority, and graph exploration bounds. No local Cargo pass is claimed; exact-head runner results are tracked in the PR.

## Claim ceiling

This is a bounded finite action-graph audit. It is not a continuous viability-kernel proof, hydrodynamic model, COLREG solver, refuge guarantee, navigation authority, helm controller or actuator executor. Physical deployments require qualified vessel models, verified refuge/access/service data, independent review and SIL/PIL/HIL progression.


## Human handover is a timed safety path, not a checkbox

The human-handover terminal is counted only when the record binds an affirmative acknowledgement, a named accountable remote authority role, qualified communications evidence, an upper bound on human response time, and confirmation before the intervention deadline. A terminal with no acknowledgement is not a handover. A response arriving after the last safe intervention point is not a fallback.

## Capability qualification versus capability availability

The capability registry is part of the canonical mission snapshot and binds a capability ID, qualification status, evidence digest, operating-envelope scope and validity horizon. A scenario may declare only current, qualified capabilities in that registry. Each action then requires those capabilities to be available in that particular trigger state. A vessel may have a qualified engine-repair capability in its registry while that option is unavailable after a generator trip because the engine or repair kit is unavailable. Qualification is not availability.

## Reproducible bounds

The evaluator enforces a maximum action count, action depth, successful-path count and total expanded-edge count per scenario. If the search stops at a bound, the report marks enumeration incomplete. SEARCH_INCOMPLETE/ABSTAIN is intentionally distinct from INFEASIBLE/ABSTAIN: lack of a discovered path under an incomplete search is not proof that no path exists.


See the full [2026 state-of-practice and matched benchmark contract](../../../docs/research/marine/recovery-reachability-human-handover-and-comparison-0001.md), including comparisons with multi-energy rolling-horizon ship management, timed-automata planning, field-tested temporal mission planning, and remote-operator human factors.
