# Construction Observation Portfolio Core — v0.1

Status: bounded exact synthetic campaign planner. It plans a set of observation actions; it does not authorize site access, prove coverage, inspect work or accept construction output.

Parent: CONSTRUCTION-OBSERVATION-PORTFOLIO-001 #5164.

Stack:
interop / resource-time index -> accepted-floor ledger -> space-time occupancy -> observation planner -> this portfolio solver.

## Why this exists

The observation planner in #5107 chooses one action. A floor often needs a campaign of multiple scans or surveys. Individually feasible actions can conflict when combined because they may occupy the same workface, access corridor, hoist or instrument resource.

This crate provides both a deterministic greedy multi-action baseline and bounded exact search over up to 20 candidate actions. Every include branch clones the current planning books and uses the existing space-time/resource admission owner. It therefore checks the candidate campaign against already active work and against previously selected campaign actions without mutating the caller's live books.

The exact solver is checked against a tiny independent subset oracle whose expected incompatibility pairs are authored in the frozen fixtures rather than derived from the production overlap/capacity code.

## Objective

The search compares campaigns lexicographically:
1. maximize covered work needs (complete campaigns always outrank partial campaigns);
2. minimize total crew minutes;
3. minimize direct cost in one explicit three-letter uppercase currency code;
4. minimize energy;
5. minimize the uncertainty-expanded reservation span;
6. minimize action count;
7. stable action IDs.

It does not combine unlike units into an arbitrary weighted scalar. It does not compare across currencies.

## Search evidence

The greedy baseline chooses the feasible action with the greatest marginal coverage, then minimizes crew minutes, cost, energy, duration and action ID. It is a heuristic and never claims optimality. A frozen counterexample demonstrates why largest-coverage-first can be more expensive than the exact set-cover optimum.

The result always carries:
- best complete or partial campaign found;
- covered and uncovered work units;
- crew-minutes, cost, currency, energy, duration and schedule span;
- nodes visited and whether the search completed;
- optimality proven only when exhaustive search completed with full coverage;
- infeasibility proven only when exhaustive search completed without a full campaign;
- contextual rejected candidate details from the existing spatial/resource owner.

A node-budget-exhausted search is explicitly incomplete. It is neither optimal nor proof of infeasibility.

The Linux CI benchmark compares exact and greedy planning over 8, 12, 16 and 20 actions, five repetitions per size. It reports median and p95 runtime, exact search nodes, crew totals and selected action counts. These are synthetic runner measurements, not field or deployment SLOs.

## Local validation commands

- cargo fmt --manifest-path research/construction-observation-portfolio-v0-1/Cargo.toml -- --check
- cargo test --manifest-path research/construction-observation-portfolio-v0-1/Cargo.toml --locked
- cargo clippy --manifest-path research/construction-observation-portfolio-v0-1/Cargo.toml --locked --all-targets -- -D warnings
- cargo bench --manifest-path research/construction-observation-portfolio-v0-1/Cargo.toml --locked --bench campaign_scale

## Claim ceiling

The region mapping is declared benchmark input, not sensor coverage evidence. A completed campaign means only that the selected actions can be scheduled/admitted in the synthetic planning state. Actual observations remain subject to their own coverage/calibration/receipt authority, and resulting work remains subject to independent acceptance. No safety, field productivity or cost-saving claim is made.
