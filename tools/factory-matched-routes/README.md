# Matched Factory Route Benchmark v0.1

**Owner:** Symtropy FACTORY-MATCHED-ROUTES-001 #5432  
**Related:** Symthaea MULTIPURPOSE-FACTORY-001 #12355; Symtropy FACTORY-RECONFIG-ACCEPTANCE-001 #5411  
**Status:** authored synthetic benchmark and Rust tests; compile/test status is pending exact-head CI.

## Purpose

Compare the same two product routes across four deliberately different synthetic factory architectures: dedicated product line, functional job shop with backup machining and metrology, multipurpose microfactory with a shared production cell, and hybrid local factory with a qualified specialist partner node.

The routes are a pump/valve subassembly and a returned-pump repair/refurbishment. Their operation sequences and required capabilities are identical across all architecture comparisons.

## Evaluation semantics

The evaluator performs a deterministic, sequential, single-unit route feasibility analysis. It validates module and operation identity before evaluation, requires explicit calibration declarations for calibrated modules, and requires partner-node identity for partner-operated modules. The `route-matrix` binary emits the complete architecture × product × scenario matrix as tab-separated rows, including disposition, feasible-assignment count, Pareto objective triples, blocked operations and input rejection reasons. Each operation may use any module whose capability, material profile, process qualification, required calibration, workforce, utilities, segregation and quality evidence are current. A stale or unknown required gate is rejected. Partner-node and module-role outages are applied before assignment.

All admissible assignments are enumerated. Their elapsed ticks, cost units and energy units are compared using a non-dominated Pareto frontier. No weighted sum or hidden preference is used. The frontier is lexicographically ordered for deterministic serialization; this ordering is not a preference ranking.

Elapsed ticks are the sum of processing, setup and synthetic transfer ticks for the selected operation modules. Cost and energy are dimensionless relative units. They are not dollars, kWh, measured cycle times, calibrated estimates or supplier quotes.

## Reference scenarios

1. BASELINE
2. PRIMARY_MACHINING_OUTAGE
3. SHARED_CELL_OUTAGE
4. PARTNER_NODE_OUTAGE
5. METROLOGY_CALIBRATION_STALE
6. PRESSURE_TEST_SKILL_UNKNOWN
7. POWER_UTILITY_INTERRUPTION

The scenario profiles are deliberately diagnostic and matched across architecture types. The primary-machining outage names the architecture-specific primary cell in each profile; only the job shop preserves the route through a separately identified backup machine. The hybrid partner-node outage removes every operation assigned to that common partner dependency.

## Claim ceiling and exclusions

A SyntheticAccepted result means only that this reference route has a complete assignment through the synthetic modules and every modeled hard gate is current in the supplied fixture.

It does not establish actual process/equipment compatibility or physical capability; workplace safety, legal compliance, certification or production authorization; dimensional/pressure/flow product quality; first-article acceptance by an independent inspector; realistic capacity, utilization, queueing, WIP, batching or multi-order scheduling; stochastic yield, process variation, maintenance reliability or measured lead times; real material prices, labour rates, CAPEX, OPEX, ROI or customer demand; physical digital-twin fidelity or off-Earth production capability.

This first version is intentionally a transparent route-feasibility and resource-selection benchmark. The next benchmark generation should add matched multi-order capacity, queueing, sequence-dependent changeovers, transport/WIP, stochastic disruption with declared distributions and small-instance MILP/CP-SAT cross-checks. Physical claims must remain tied to independently measured pilot evidence.

## Local verification

Run from this directory:

    cargo clippy --all-targets -- -D warnings
    cargo test --all-targets
    cargo run --bin route-matrix

The repository authoring environment used for this change did not have Rust installed. No local Rust compile, formatter or test pass is claimed; the focused workflow is the exact-head verification path.
