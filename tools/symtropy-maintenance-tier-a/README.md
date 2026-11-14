# Symtropy Maintenance Tier-A Reference Core

**Status:** standalone deterministic micro-oracle; no factory DES or field maintenance claim.  
**Parent:** EV-FACTORY-CBM-001 / Symtropy issue #2619 and subtask #3014.

## Purpose

This dependency-free Rust crate defines the smallest auditable maintenance state-transition contract: one machine, an initial operating state, absolute-time failure and repair-completion events, a finite observation horizon, and a declared production rate.

The evaluator consumes a supplied event schedule. It does not sample failure clocks, infer degradation, process sensor data, queue a maintenance crew, simulate buffers, or issue maintenance recommendations. This is intentional: deterministic trace semantics should be correct before stochastic generators, condition policies or factory production coupling are introduced.

## Run

From the repository root:

    cargo fmt --manifest-path tools/symtropy-maintenance-tier-a/Cargo.toml --all -- --check
    cargo test --manifest-path tools/symtropy-maintenance-tier-a/Cargo.toml --locked

## Semantics

- Initial state is operating.
- Failure changes operating to repairing.
- RepairComplete changes repairing to operating.
- Events are absolute calendar times, strictly increasing, uniquely identified, and inside the declared horizon.
- Events at the horizon are applied after accruing the preceding interval. Thus a failure exactly at the horizon is counted, but contributes no downtime inside the horizon; a repair completion exactly at the horizon is completed.
- An unfinished repair is right-censored. The evaluator reports the repair time elapsed by the horizon, not an invented remaining repair duration.
- All productive and repair intervals partition the full horizon exactly.
- Output is a deterministic function of the input schedule and parameters.

The analytic helper computes steady-state availability for exponential operating-time-to-failure and deterministic repair duration:

    A = MTBF / (MTBF + MTTR)

It is an expected long-run reference, not an empirical MTBF/MTTR estimate. Finite simulation replications must be assessed with a predeclared statistical protocol rather than expected to equal the analytical value exactly.

## Claim ceiling

Passing these tests qualifies only the deterministic event ledger and arithmetic for the stated model. It does not establish a stochastic RNG, degradation law, maintenance policy, crew scheduling, finite-buffer production model, physical asset validity, plant savings, safety or SOTA.
