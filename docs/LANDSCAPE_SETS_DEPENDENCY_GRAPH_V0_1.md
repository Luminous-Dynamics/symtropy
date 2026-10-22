# Landscape Social-Ecological-Technological Dependency Graph v0.1

## Why this benchmark exists

A landscape can have several individually acceptable metrics and still lose a critical service when a dependency fails. Shade may depend on canopy establishment and accessible paths; canopy establishment may depend on irrigation and maintenance; stormwater performance may depend on soil infiltration and maintained drainage. Treating these as independent benefit scores hides paths through which a local disruption can propagate.

This Rust benchmark represents such relationships as an explicit directed dependency graph across social, ecological and technical layers. It reports which services remain structurally supported under declared node failures and disabled dependency edges, and provides unsatisfied dependency groups with each provider alternative's status.

## Semantics

The direction is explicit: a dependency edge means the dependent node requires the provider node. All required groups on a node must be satisfied (AND across groups); within each group, any one operational provider alternative is sufficient (OR). This supports redundancy without assuming that two alternative providers are additive.

Cycles are rejected in v0.1. Static prerequisite reachability cannot safely resolve feedback loops or mutually supporting components, so a future dynamic extension must model those explicitly rather than allowing circular self-support.

The semantics document is pinned by its exact-byte SHA-256 in the fixture. Every scenario declares failed node IDs and/or disabled edge IDs. Unresolved scenario references, duplicate IDs, undeclared or missing requirement groups, cycles, and synthetic-to-observed relabelling fail closed.

## Frozen synthetic fixture

The small fixture includes water supply, irrigation, maintenance capacity, canopy, a built shade alternative, an accessible path, infiltrating soil and drainage, plus three service nodes. It demonstrates that drought may make canopy-dependent habitat unsupported while the heat-refuge service remains operational via a separate built-shade provider; a maintenance shock breaks both canopy-dependent ecology and maintained drainage; and compound failures can remove all three services.

| Scenario | Services supported | Availability | Structural interpretation |
|---|---:|---:|---|
| Baseline | 3 / 3 | 100% | All declared dependencies resolve |
| Drought | 2 / 3 | 66.7% | Built shade preserves heat-refuge support; tree-ecology service loses canopy |
| Maintenance shock | 1 / 3 | 33.3% | Canopy and drainage are unsupported; only heat refuge remains |
| Access disruption | 2 / 3 | 66.7% | Heat-refuge service fails its access group |
| Irrigation-link outage | 2 / 3 | 66.7% | Canopy-dependent service is unsupported; built shade remains an alternative |
| Compound failure | 0 / 3 | 0% | Water, maintenance and access disruptions leave all services unsupported |

The cascade-amplification diagnostic is unsupported-service count divided by injected node/edge failures. It is a graph-structure ratio, not a physical cascade probability, impact magnitude or causal effect. The report also runs a deterministic single-failure audit over every declared node and dependency edge, then orders these by observed structural service loss. In the frozen fixture, failure of `maintenance_crew` is the top single-node failure: it removes support for both tree ecology and stormwater retention (2 of 3 baseline services). This is a graph-structure sensitivity, not a failure probability or a reason to prioritize real-world maintenance without validating the dependency model.

## Competitive benchmark ladder

- Correctness: 100% known-answer agreement, deterministic replay, zero unknown references or cyclic dependency graphs accepted.
- Redundancy: alternatives within an OR-group preserve service only when at least one provider remains operational; all mandatory groups must still hold.
- Explainability: every unsupported service exposes its unsatisfied groups and provider states, enabling reviewers to trace blockers. The single-failure audit ranks every node and edge by structurally lost service count and reports the affected service IDs.
- Single-point-of-failure coverage: audit every declared node and edge in the frozen graph; require exact 22/22 audit entries and known-answer agreement for the maintenance dependency. Larger topology targets are 100% node/edge audit coverage without a partial report.
- Generalization: expand with independently sourced asset inventories and qualified service-dependency evidence; test held-out districts and adversarial topology mutations.
- Performance: freeze graphs at 100, 1,000 and 10,000 nodes; report validation/evaluation time and peak memory against a reference implementation. These are future targets, not measured results.
- Decision usefulness: compare structural critical paths with field logs, maintenance tickets, utility outages and intervention outcomes. Do not equate structural reachability with hydrological, thermal, ecological or human outcomes.

## Research grounding and claim boundary

A 2026 review of 157 urban green-space scenario-analysis publications found gaps in systematic evaluation and stakeholder engagement: https://pmc.ncbi.nlm.nih.gov/articles/PMC13293698/. A September 2026 Social-Ecological-Technological Systems framework argues for explicitly linking ecological, governance and technical/infrastructure conditions to system-level pathways: https://doi.org/10.1016/j.crm.2026.100824. This benchmark is a small executable structural diagnostic aligned with that direction, not a reproduction or validation of either paper's methods.

All fixture nodes, edges and failures are synthetic. No physical landscape model, causal discovery, service-capacity estimate or failure-probability model is run. A passing report proves only that the declared graph and scenario contract were evaluated consistently under the stated semantics.
