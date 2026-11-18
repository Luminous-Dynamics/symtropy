# HoloFrac Lifecycle Evidence-Impact Protocol v0

**Status:** synthetic dependency/receipt qualification fixture  
**Implementation:** \`research/holofrac-retrieval/src/lifecycle_impact.rs\`  
**Executable:** \`research/holofrac-retrieval/src/bin/lifecycle_impact_sweep.rs\`  
**Workflow output:** \`lifecycle-evidence-impact.csv\`  
**Related:** [#5959 source-manifest and solver receipts](https://github.com/Luminous-Dynamics/symtropy/issues/5959), [#5985 lifecycle ledger](https://github.com/Luminous-Dynamics/symtropy/issues/5985)

## 1. Objective

Test whether a lifecycle mutation invalidates exactly the current evidence claims that depend on the changed component state, while preserving evidence whose complete declared dependency inventory proves it is unrelated.

This composes the existing exact dependency-closure engine with a component-lifecycle scenario. It does not use HDC similarity to infer authoritative dependency edges.

## 2. Frozen evidence inventory

The finite fixture includes current receipts for:
- structural capacity;
- fire rating;
- component recoverability;
- functional adaptation;
- lifecycle readiness;
- thermal performance;
- daylight performance.

Structural claims depend on component condition, interface, material passport, functional use and the floor-load requirement. Fire-rating evidence depends on the material passport. Recoverability and lifecycle readiness depend on condition, interface, material and use. Thermal/daylight evidence depend on separate building-envelope/weather sources.

These are synthetic receipt IDs and source relationships, not engineering certificates or solver results.

## 3. Mutation corpus and expected invalidation

| Mutation | Evidence expected stale | Independent evidence expected retained |
|---|---|---|
| Component condition | Structural capacity, recoverability, functional adaptation, lifecycle readiness | Fire rating, thermal, daylight |
| Connection/interface | Structural capacity, recoverability, functional adaptation, lifecycle readiness | Fire rating, thermal, daylight |
| Material passport | Structural capacity, fire rating, recoverability, functional adaptation, lifecycle readiness | Thermal, daylight |
| Functional use | Structural capacity, recoverability, functional adaptation, lifecycle readiness | Fire rating, thermal, daylight |

The benchmark tests the full exact closure, not only the counts. The frozen oracle compares receipt IDs. Incomplete dependency coverage must set conservative full invalidation and retain zero current evidence.

## 4. Acceptance targets

- 100% agreement with the frozen finite evidence oracle;
- zero false-current evidence in the declared mutation corpus;
- zero unrelated invalidation when complete dependencies prove disjointness;
- zero evidence retention when the dependency inventory is incomplete;
- all exact changed-artifact and stale/retained receipt IDs emitted in raw CSV.

The existing exact closure engine separately tests duplicate edges, cycles, unknown endpoints, unknown mutations, shared common-cause changes and independent brute-force reachability. The new fixture composes this engine with lifecycle-relevant mutation semantics.

## 5. Boundaries and limitations

- A current evidence inventory must be scoped to a particular component/building state. Historical evidence remains historical; it must not be represented as current merely because it is unaffected by a later mutation.
- The fixture assumes its listed graph is complete when \`declare_complete\` is set. The graph cannot prove its own source-inventory completeness.
- The fixture does not bind actual source manifests, solver build digests, geometry digests or real evidence receipts. That work remains in #5959.
- A retained receipt means only that the declared graph contains no path from this mutation to the receipt. It is not proof of physical validity, solver applicability or real-world completeness.

## 6. Claim ceiling

Passing establishes exact invalidation/retention behavior for the finite declared fixture only. It does not establish engineering qualification, structural safety, code compliance, physical truth, completeness of a real source inventory or currentness of any external receipt.
