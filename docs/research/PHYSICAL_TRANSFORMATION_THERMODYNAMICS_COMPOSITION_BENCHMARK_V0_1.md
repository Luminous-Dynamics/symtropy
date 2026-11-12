# Physical Transformation Thermodynamics Composition Benchmark v0.1

**Tracking:** PHYS-TRANSFORM-THERMO-001 #4886  
**Owners composed:** #2209 / PR #2238; #3577; #1693; PR #3594; PR #3597; #460; adjacent #4631.  
**Status:** draft research contract. This does not add a second ledger or solver.

## 1. Purpose

Qualify whether an exact construction, destruction, repair or recovery event can be reconciled across:

- physical topology and component/material state;
- first-law stored-energy change and all declared boundary work/heat flows;
- supported thermal body/bath entropy changes;
- uncertainty, incomplete evidence and numerical residuals.

The objective is not to minimize entropy in general. It is to determine whether an event is consistently represented under declared physical boundaries and assumptions, and to reject false conservation or second-law passes.

## 2. Compose existing owners; do not duplicate

| Owner | Existing responsibility | Composition responsibility |
|---|---|---|
| Symtropy #2209 / PR #2238 | Revisioned component/connection events, topology and exact material recovery accounting | Supplies predecessor/successor event identity and material/topology transition |
| Symtropy #3577 | Thermodynamic semantic audit and port/reservoir framing | Supplies claim boundaries and semantic constraints |
| Symtropy #1693 | Legacy typed energy-flow migration | Remains legacy owner; no replacement ledger here |
| Symtropy PR #3594 | Open-system first-law balance over a declared boundary and interval | Supplies energy-balance evaluator and uncertainty-aware status |
| Symtropy PR #3597 | Constant-heat-capacity body and fixed-temperature bath entropy references | Supplies narrowly scoped analytic entropy calculations |
| Symtropy #460 | Physics engine qualification | Supplies independent scenario/solver qualification concepts |
| Symtropy #4631 | Robotic locomotion entropy/exergy metric decision-value benchmark | Reuse semantic separation; do not copy its mission-specific metrics |

The first-law evaluator is an accounting validator, not a physical model validator. The thermal entropy reference handles only its declared constant-capacity body and fixed-temperature bath cases. The composition layer must preserve those limitations.

## 3. Paradigm: every physical transformation has explicit ports

A construction or destruction operation should not carry a generic "energy cost" or "entropy delta" field with ambiguous meaning.

Represent it as a revisioned event with:

1. a physical subject and topology/material transition;
2. a declared system boundary and time interval;
3. stored-energy state before and after;
4. external work and heat flows crossing that boundary;
5. named source/sink reservoirs and uncertainty bounds;
6. supported entropy state/transfer calculations;
7. separate numerical residual, applicability and evidence disposition.

Internal transfers between subsystems wholly inside the declared boundary cancel in the whole-boundary energy balance. They must not appear as both an internal transfer and an external input/output. If matter crosses the boundary, energy carried by that matter must either be explicitly represented in a future compatible flow model or the v0.1 boundary must be chosen so that the matter exchange is internal. Do not silently assume mass-flow energy is zero.

## 4. Balance semantics

### 4.1 First law

For the initial bounded v0.1 scenarios with no unmodelled matter-energy transport:

    Delta E_stored = net_work_into_boundary + net_heat_into_boundary
    residual = Delta E_stored - (net_work + net_heat)

Use PR #3594's typed flow categories, declared boundary and interval, unique flow identities, named external reservoirs, non-negative magnitudes, direction, and conservative absolute uncertainty bounds.

A complete evaluator can report Balanced, Imbalanced or Inconclusive; missing values or bounds produce Incomplete. A zero-output flow inventory must be explicit. A syntactically balanced report does not prove that the boundary or measurements are physically complete.

### 4.2 Entropy

For a bounded system with no matter exchange, the general entropy bookkeeping shape is:

    Delta S_system = entropy_transfer_in - entropy_transfer_out + entropy_generated

The heat entropy-transfer term requires a declared boundary temperature and applicable physical process. Work is an energy transfer, not an entropy flow merely because it carries energy.

For the narrow constant-heat-capacity body and fixed-temperature bath reference:

    Delta S_body = C * ln(T_final / T_initial)
    Delta S_bath = -Q_into_body / T_bath
    Delta S_total = Delta S_body + Delta S_bath

Use PR #3597 exactly within its assumptions: positive equilibrium endpoint temperatures, constant heat capacity, no phase transition over the interval, and a fixed-temperature bath. Also independently check the compatible first-law heat/state relationship for the fixture. Do not use Q/T_final as a universal entropy equation.

SecondLawStatus::Satisfied from the thermal reference applies only to the supplied body-plus-bath terms. It must not be represented as a general whole-world second-law proof when other reservoirs or entropy flows are omitted.

### 4.3 Exergy

Exergy destruction may be derived as:

    B_destroyed = T0 * S_generated

only if the environment reference state T0, system boundary and physical entropy generation are all established. Exergy is optional in this first composition slice. It is not equal to total energy consumption, battery discharge, impulse magnitude, information entropy or a cognitive proxy.

## 5. Composition receipt contract

A transformation-thermodynamics manifest must bind:

- exact event ID, predecessor/successor world revision and transformation profile;
- exact component/connection/material-lot identities and material mass receipt;
- boundary ID and interval ID shared by all energy and entropy terms;
- initial/final stored-energy values, units, uncertainty and explicit included storage domains (e.g. kinetic, elastic/strain, gravitational/potential, internal/thermal);
- explicit work/heat boundary flows, each with unique flow ID, reservoir ID, carrier, direction, joule magnitude and uncertainty;
- explicit statement of whether matter crosses the boundary and how any associated energy terms are handled;
- thermal body identity, heat capacity, initial/final temperatures, heat transferred and bath temperature when PR #3597 is invoked;
- exact code/model/profile identities and evidence classes;
- separate material, first-law, second-law, exergy and evidence-completeness dispositions.

No single weighted pass/fail score may hide a missing balance term, failed material ledger or unsupported entropy assumption.

## 6. Initial analytical and synthetic scenario corpus

| Scenario | Expected physical/accounting behaviour | Essential negative control |
|---|---|---|
| Ideal elastic loading and unloading | Stored elastic energy rises and returns; external work balances stored-energy change | Do not create dissipation/heat from an impulse or generic action-cost counter |
| Inelastic centered impact | Kinetic-energy loss is measured before/after and assigned to named thermal/other reservoirs under a declared partition | Impulse is momentum, not joules; missing destination makes balance incomplete |
| Two-reservoir thermal exchange | Constant-capacity body plus fixed-temperature bath reproduces PR #3597 analytic entropy result | Reverse hot/cold ordering and use invalid constant-capacity assumptions |
| Ideal assembly with external work | Topology and material state change; declared work maps to stored mechanical/elastic energy | Do not count internal subsystem transfer as external input |
| Cutting/deconstruction | Material/topology changes are revisioned; work, heat and storage change are separate | Unknown fracture/plastic work or heat rejection must remain incomplete, not assigned a guessed fraction |
| Material recovery and sorting | Exact material mass closes under #2209 categories | Material closure cannot make an energy residual pass |
| External recharge/regeneration | Named source reservoir supplies work/heat; stored energy increases | Regeneration is not negative dissipation |
| Internal transfer | Subsystem transfers cancel in whole-system boundary balance | Detect double-counted internal flow |

The first release can use synthetic, hand-computable data. These cases qualify the receipt semantics, not the accuracy of a fracture or construction solver.

## 7. Adversarial and metamorphic controls

- Missing initial/final stored-energy domain, boundary flow inventory or uncertainty bound -> Incomplete.
- Zero input with positive output -> never Balanced by a denominator guard.
- Duplicate flow ID, unknown reservoir, wrong direction, wrong carrier, wrong interval or mismatched boundary -> reject / Incomplete.
- Duplicate internal transfer or recording the same physical flow at both subsystem and system boundaries -> detected double count.
- Material mass imbalance remains visible when energy is balanced; energy imbalance remains visible when material mass closes.
- An entropy estimate inferred from energy-consumption telemetry, cognitive FEP or an action-cost proxy is not admitted as physical entropy.
- Phase transition, temperature-dependent heat capacity or non-equilibrium body state passed to the constant-C reference -> typed rejection / NotEvaluable.
- Missing bath temperature or unreported entropy-transfer channel -> no second-law pass.
- Exergy request without a declared T0 and valid S_generated -> NotEvaluable.
- Identical ordered inputs produce deterministic receipts; stale/duplicate transformations preserve atomic no-mutation semantics.
- Any failed owner receipt prevents promotion of the aggregate composition to fully qualified.

## 8. Metrics and comparison

Report separately:

- transformation validity and topology replay;
- material mass residual / unaccounted mass;
- first-law residual interval and status;
- analytic body/bath entropy error and second-law disposition for supported thermal fixtures;
- missing-flow, wrong-boundary, double-count and invalid-assumption rejection rates;
- uncertainty completeness and evidence-class preservation;
- deterministic replay and exact event-to-receipt linkage.

Compare four levels under identical frozen scenarios:

1. topology/material transformation only (#2238 baseline);
2. transformation + first-law receipt (#3594);
3. transformation + first-law + supported analytic thermal entropy (#3594 + #3597);
4. full composition with evidence completeness, uncertainty and fail-closed dispositions.

No universal physical energy/entropy tolerance is prescribed. Every fixture declares resolution, uncertainty, boundary, time window and acceptance tolerance before execution. Measured construction/deconstruction energy data are a later external-validation tier.

## 9. Industry positioning

Industrial multiphysics and fracture tools can model richer mechanics and thermal phenomena. This benchmark is not a solver-breadth contest.

The proposed differentiation is composition assurance: one physical event is linked to its exact topology/material transition, complete declared energy boundary, valid entropy process, uncertainty and replayable receipt. Specialist solver outputs may be consumed only with an exact solver/profile/version, boundary conditions, numerical settings and independent validation evidence.

A passing composition receipt does not establish that the physical model is accurate or that the chosen boundary is complete.

## 10. Definition of done

- Versioned composition manifest binds one transformation event to first-law and supported thermal-entropy receipts.
- Independent verifier checks subject/revision identity, boundary/interval equality, units, flow uniqueness, reservoir completeness, state-energy coverage and entropy assumptions.
- Positive, negative, metamorphic and double-count controls execute against frozen hand-computable fixtures.
- Existing #2209, #3594 and #3597 owners are consumed without copying their physical or thermodynamic formulas.
- Exact-head formatting, tests, Clippy and CI evidence are reported separately from external physical validation.

## 11. Claim ceiling

This benchmark qualifies the composition of owner outputs for declared reference cases. It does not establish general structural energy closure, universal entropy production, validated fracture/thermal physics, a general exergy model, building-code compliance, demolition authorization or maintenance authority.
