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

- A digest-bound, vessel-specific service/rate contract and trigger/hard-gate rosters.
- A trigger-time resource state with conservative uncertainty intervals.
- Bounded action-graph reachability from the current node to a declared terminal.
- Action trigger, component and capability preconditions; exact hard-gate coverage; action evidence and digest.
- Per-axis successor intervals from bounded resource deltas.
- An exact two-cone inter-sample envelope during every bounded action duration, using the mission's declared maximum absolute rate per hour. Both endpoints and the entire action interval must remain within all service floors/ceilings.
- Total worst-case recovery time must fit within the intervention deadline.
- Safe-haven, shore-resupply, repair-restoration and controlled-safe-hold terminals count only when destination access, required service capacity/availability, authority and terminal evidence are verified and unexpired.
- Every rejected action edge and every feasible recovery path is retained. No weighted path score or aggregate utility is produced.

## Synthetic scenario results

| Fault trigger | Expected result | Why |
|---|---|---|
| Main generator trip | RECOVERABLE | A load-shed then safe-haven path survives all resource and inter-sample floors; fast diversion and generator repair are rejected |
| Watermaker trip | RECOVERABLE | Shore-resupply path reaches a port with verified access, capacity, service and authority evidence |
| Common-cause generator + watermaker outage | INFEASIBLE/ABSTAIN | The candidate diversion breaches both energy and freshwater floors; the controlled-hold alternative depends on unavailable generation |
| Intervention deadline exhausted | INFEASIBLE/ABSTAIN | The only route arrives after the declared intervention deadline |
| Unverified port services | INFEASIBLE/ABSTAIN | Geographic reachability is not terminal success when required shore service evidence is unverified |

All numbers, action deltas, rate bounds, authority receipts and terminal evidence are synthetic known-answer fixtures.

## Industry and research boundary

- 2026 adaptive-hex/ensemble ship-routing work already handles member-coherent CVaR and edge-level IMO second-generation stability pruning (Zhang et al., *Ocean Engineering*, 15 Oct 2026, DOI: https://doi.org/10.1016/j.oceaneng.2026.127661).
- August 2026 work couples all-electric voyage planning and energy management with day-ahead and intraday optimization (Gao et al., *Journal of Energy Storage*, 30 June 2026, DOI: https://doi.org/10.1016/j.est.2026.122206).
- Recent safe-control literature moves beyond a nominal plan to set-based reachability and uncertainty-qualified safety. A November 2026 *Automatica* paper derives finite-sample reachable-set guarantees for GP dynamics (DOI: https://doi.org/10.1016/j.automatica.2026.113204); a September 2026 robust data-driven MPC paper separates epistemic and aleatoric uncertainty in analytic probabilistic safety barriers (DOI: https://doi.org/10.1016/j.rico.2026.100789).

The differentiator to test is not a new route optimizer. It is a **separate recovery verifier** that requires an evidenced, authorized, resource-safe continuation after a fault, and abstains before the last intervention point when no such path is demonstrated.

## Claim ceiling

This is a bounded finite action-graph audit. It is not a continuous viability-kernel proof, hydrodynamic model, COLREG solver, refuge guarantee, navigation authority, helm controller or actuator executor. Physical deployments require qualified vessel models, verified refuge/access/service data, independent review and SIL/PIL/HIL progression.
