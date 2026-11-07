# CivDyn CRG → Resource Ledger Adapter v0.1

This is the first executable cross-domain slice for Symtropy #4022. It is deliberately an **open-loop trace adapter**, not another agent simulator or resource ledger.

## Composition and provenance

The crate depends on the public APIs of:

- \`civdyn-collective-risk\` at \`../civdyn-collective-risk\`;
- \`civdyn-resource-ledger\` at \`../civdyn-resource-ledger\`.

The adapter branch is stacked on \`research/civdyn-collective-risk-2026-11\`, which already contains both parent crates and their dedicated workflows. This deliberately preserves the existing dependency chain: resource ledger → collective-risk reference → bridge. Do not rebase this PR directly onto \`main\` until the parent PRs have been reconciled.

## Frozen synthetic mapping

One CRG contribution unit is represented as one kWh of requested transfer **only in this synthetic fixture**. This is a declared scenario mapping, not an empirical or physical conversion.

- Six donor regions, one per CRG seat; each opens with 50 kWh.
- Each donor has 1 kWh of essential energy demand per tick.
- One shared-service region has population 12 and 1 kWh/person/tick of essential demand (12 kWh/tick).
- Ten ticks; no exogenous production; no water or food demand.
- Each seat's action at tick t becomes a transfer request from that donor to the shared-service region.
- The ledger applies transfers before essential demand. Donor opening stock 50 kWh covers the game's maximum 40 contribution units plus 10 kWh of donor demand over the ten-tick horizon.
- Unrestricted transfers are explicit. Any delivery shortfall is emitted and checked; no transfer is silently assumed delivered.

## Conditions

Five transparent rosters are run at p ∈ {0.1, 0.5, 0.9}: all-zero, all fixed fair-share, all fixed maximum, all risk-pivot, and one heterogeneous six-policy roster. The tie at p=0.5 is preserved: all-zero and risk-pivot are both included, because the source game has two welfare-optimal modes at the pivot.

## Output

The executable emits deterministic CSV with:
- one episode summary per risk/roster;
- every agent's requested and delivered transfer, action, essential demand/service/unmet demand and closing stock for every tick;
- shared-service demand, service, unmet demand and closing stock for every tick;
- game aggregate welfare/target outcomes and each region's water/energy/food conservation residuals.

The CSV has a versioned header contract in the source. The workflow executes the binary twice and byte-compares the artifacts. Unit tests independently recompute contribution totals, verify donor service floors, verify the shared-service positive/negative controls, check the pivot tie, test seat-permutation equivariance and check each region's native-unit ledger.

## Paradigm boundary

The experiment answers a narrow question: **what service outcomes follow when an already-generated collective-risk action trace is replayed through a declared stock-flow ledger?** It does not answer what agents would choose after observing resource state. It is not a closed-loop social simulation.

The result does not validate human causal effects, the 1-unit/1-kWh mapping outside this fixture, physical heat/entropy/exergy, biological metabolism, consciousness, love, welfare or moral worth. A stable/high-welfare policy is not thereby morally legitimate. Physical thermodynamic claims remain gated by #3577; normative interpretation remains gated by Symthaea #10852.

## Run

\`\`\`sh
cargo fmt --manifest-path research/civdyn-crg-resource-adapter/Cargo.toml --all -- --check
cargo clippy --locked --manifest-path research/civdyn-crg-resource-adapter/Cargo.toml --all-targets -- -D warnings
cargo check --locked --manifest-path research/civdyn-crg-resource-adapter/Cargo.toml --all-targets
cargo test --locked --manifest-path research/civdyn-crg-resource-adapter/Cargo.toml --all-targets
cargo run --locked --manifest-path research/civdyn-crg-resource-adapter/Cargo.toml --bin crg-resource-bridge
\`\`\`
