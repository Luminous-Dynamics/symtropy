---
title: Space Bootstrap and Industrial Closure Runtime
version: 0.1
status: implementation-spec
scope: autonomous space-industrial bootstrapping, dependency closure, provenance, machine ecology, energy, manufacturing resilience
owner: engineering/simulation/autonomy/civic
related:
  - MK0_BOOTSTRAPPER_PROTOCOL.md
  - VEHICLE_SPACECRAFT_PHYSICS_AND_OPERATIONS_RUNTIME_V0_1.md
  - DEEP_SPACE_LOGISTICS_TRANSFER_WINDOWS_RESCUE_AND_SALVAGE_RUNTIME_V0_1.md
  - PLANETARY_INFRASTRUCTURE_NETWORKS_AND_CORRIDOR_RUNTIME_V0_1.md
  - LIGHT_DELAY_COMMUNICATION_TIMEKEEPING_AND_ASYNC_COORDINATION_RUNTIME_V0_1.md
---

# Space Bootstrap and Industrial Closure Runtime

## Purpose

Treat space bootstrapping as a measurable capability-growth problem rather than a destination checklist.

The system optimizes for:

```text
growth of autonomous industrial capability
per unit of imported mass and unresolved critical dependency
```

This specification does not claim that autonomous self-replication, Mercury mining, local semiconductor fabrication, or electromagnetic launch are solved engineering problems.

## 1. Design Thesis

The decisive transition is:

```text
imported equipment
  -> repair
  -> local feedstock
  -> local parts
  -> local machines
  -> local factories
  -> local energy infrastructure
  -> local computation/control
  -> autonomous reproduction
  -> diversified industrial ecology
```

The milestone is not tonnes of ore. The milestone is the ability to manufacture and repair the next useful generation of industrial capability.

## 2. Mercury as a Bootstrap Proving Ground

Current measurements make Mercury attractive and severe:

- NASA gives Mercury a mean solar distance of about 0.387 AU and says sunlight there can be as much as seven times brighter than at Earth. The 9,083 W/m² mean-distance value is a derived inverse-square estimate, not a direct NASA table value.
- JPL gives surface gravity of about 3.70 m/s² and escape velocity of about 4.25 km/s.
- MESSENGER verified that Mercury's polar deposits are dominantly water ice in permanently shadowed regions.
- MESSENGER also found Mercury unexpectedly rich in moderately volatile elements including potassium, sulfur, sodium, and chlorine.

This suggests a split architecture:

```text
sunlit industrial belt:
    high solar flux
    thermal processing
    power generation
    bulk metallurgy

polar cold traps:
    water and volatile inventory
    protected extraction
    low-temperature processing
    life-support / propellant feedstock
```

BepiColombo is the next major evidence upgrade: ESA currently schedules Mercury-orbit insertion for November 21, 2026 and routine science for April 2027. The simulator should therefore carry resource priors as uncertain evidence and make them updateable rather than hard-coded reserves.

**Research refresh — 2026-10-04.** ESA reports that BepiColombo entered its Mercury-arrival phase on 3 September 2026; its current plan calls for Mercury orbit insertion on 21 November 2026, spacecraft separation in December, and routine science beginning in April 2027. This should be treated as a forthcoming evidence upgrade rather than evidence of a resource or landing-site advantage. NASA's 2026 lunar surface-technology work also reports integrated prototype testing of concentrated-solar carbothermal oxygen production and describes an autonomous molten-regolith electrolysis architecture producing multiple co-products from regolith simulants. Those lunar demonstrations are engineering precedents for process-chain accounting, not Mercury validation.

## 3. Bootstrap State

```rust
struct BootstrapState {
    epoch: SystemEpoch,
    environment: EnvironmentId,
    imported_mass: Mass,
    local_mass: Mass,
    embodied_energy: Energy,
    available_power: PowerBudget,
    stored_energy: Energy,
    capability: CapabilityVector,
    dependencies: Vec<CriticalDependency>,
    facilities: Vec<FacilityId>,
    machines: Vec<MachineId>,
    inventories: Vec<MaterialBatchId>,
    knowledge_frontier: KnowledgeFrontier,
    authority: AuthoritySnapshot,
}
```

The state is serializable, content-addressable, and replayable.

A transition is valid only if mass, energy, inventory, authority, and causal ledgers remain consistent.

## 4. Capability Vector

Do not represent civilization with one technology score.

Track at least:

```text
survey
mining
beneficiation
metallurgy
ceramics
glass
polymers
chemistry
machining
fabrication
joining
seals/bearings
power electronics
sensors
communications
compute
storage
software/control
propulsion
thermal management
construction
metrology
maintenance
recycling
```

Every capability records:

```text
level
throughput
quality envelope
precision
failure rate
energy intensity
feedstock requirements
machine dependencies
labor dependencies
imported dependencies
```

A one-off successful part does not establish closure.

## 5. Critical Dependency Graph

Example:

```text
autonomous rover
  -> motor
     -> bearings
        -> hardened steel
           -> alloy chemistry
              -> refining
                 -> feedstock separation
                    -> mining

autonomous rover
  -> controller
     -> power electronics
        -> semiconductor processing
           -> ultra-pure feedstock
              -> chemical purification
```

Terminal dependencies are classified:

```text
LocalClosed
LocalOpen
ImportedConsumable
ImportedDurable
ReplaceableBySubstitution
Unknown
```

Unknown never counts as closed.

This prevents a high local-mass fraction from hiding one tiny imported component that can halt the whole industrial base.

## 6. Closure Metrics

### Mass closure

```text
local_manufactured_mass /
(local_manufactured_mass + imported_manufactured_mass)
```

Useful, but not sufficient.

### Critical closure

```text
weighted locally satisfiable critical dependencies /
total weighted critical dependencies
```

Weights increase for dependencies whose loss causes:

```text
loss of power
loss of mobility
loss of control
loss of repair
irreversible production collapse
loss of life-support capability
```

### Bootstrap efficiency

```text
delta(critical industrial capability) / imported mass
```

### Recovery horizon

```text
time to restore required capability
after removal of one critical dependency
```

The bootstrapper should improve recovery horizon while increasing redundancy.

### Multi-objective process frontier

Do not turn bootstrap planning into one opaque utility score. Candidate transitions should expose independent dimensions:

```text
critical closure gained           maximize
dependency criticality removed    maximize
imported mass consumed            minimize
energy burden                     minimize
time to capability                minimize
failure risk                      minimize
```

A candidate dominates another only when it is no worse on every declared dimension and strictly better on at least one. The runtime therefore returns a deterministic Pareto frontier instead of pretending that one universal weighting is objective. Higher layers can then apply policy-specific preferences without discarding the underlying measurements.

## 7. Industrial Closure Ladder

```text
A  Seed
B  Repair
C  Feedstock
D  Structural independence
E  Machine independence
F  Control independence
G  Factory independence
H  Ecological independence
I  Expansion
```

Interpretation:

- Seed: imported hardware dominates.
- Repair: imported machines can be inspected, repaired, refurbished, and cannibalized.
- Feedstock: useful local material can be recovered.
- Structural: bulk structures and noncritical mechanical parts are local.
- Machine: machine tools can reproduce substantial productive equipment.
- Control: electronics, sensors, communications, and storage are locally serviceable.
- Factory: factories reproduce major fractions of their tooling.
- Ecological: loss of one major factory class does not collapse the system.
- Expansion: the system can manufacture the seed hardware for another industrial node.

Promotion is evidence-based and requires repeatable demonstrations.

## 8. Machine Ecology

Model machines as a population.

Each machine records:

```text
identity
capability profile
health
maintenance state
spare-part dependencies
energy demand
software provenance
sensor confidence
authority scope
repairability
salvage value
failure history
```

Population behavior includes:

```text
load balancing
maintenance scheduling
parts cannibalization
quarantine
graceful degradation
capability substitution
fleet reproduction
```

Keep repair, refurbishment, cannibalization, recycling, and destruction distinct.

## 9. Factory-of-Factories

Every production facility exposes:

```text
inputs
transformation
tooling
energy
throughput
yield
quality envelope
maintenance requirements
waste streams
outputs
```

When proposing a new facility, the simulator must answer:

1. What makes its tooling?
2. What supplies its feedstock?
3. What supplies replacement parts?
4. What happens if it fails?
5. What fraction remains imported?
6. Can a second copy be built without increasing the first facility's imported dependency?

This makes "self-replication" a dependency problem rather than a boolean.

## 10. Energy and Thermal State

Every production transition has explicit energy accounting.

```text
generation
storage
conversion losses
reserve floor
peak power
maintenance power
thermal rejection
dispatch priority
```

Mercury additionally requires:

```text
solar incidence
surface orientation
radiative balance
shadow state
thermal inertia
emissivity
radiator capacity
heat rejection failure
```

Polar extraction must budget thermal intrusion: the act of mining a volatile deposit must be able to change its future recoverability.

## 11. Resource Provenance

Every material batch records:

```text
origin
geological confidence
mass estimate
composition estimate
extraction process
processing losses
contamination
custody
measurement evidence
uncertainty
```

A resource claim moves through:

```text
remote observation
  -> mapped deposit
  -> in-situ confirmation
  -> estimated recoverable resource
  -> processed inventory
```

The simulator must not silently promote a model estimate into usable inventory.

## 12. Epistemic Loop

```text
observe
  -> classify
  -> estimate
  -> simulate
  -> act
  -> measure result
  -> compare prediction to reality
  -> update model
```

Evidence classes:

```text
Measured
Calibrated
CrossConfirmed
ModelDerived
Projected
Unverified
Retracted
```

Physical observations retain their own provenance and confidence; internal model coherence never replaces external evidence.

## 13. Autonomy Boundary

Action authority increases through:

```text
Observe
Estimate
Recommend
Simulate
Stage
Execute-Reversible
Execute-Irreversible
Reproduce
```

Rules:

- perception failure never increases actuator authority;
- stale world models reduce allowed action scope;
- unverified recipes cannot silently enter critical production;
- reproduction requires explicit policy and resource bounds;
- manual emergency halt outranks autonomy.

## 14. Mycelix Integration

Mycelix owns distributed coordination and provenance, not physical equations.

Recommended durable records:

```text
FacilityIdentity
MachineIdentity
MaterialBatch
ResourceObservation
CapabilityClaim
WorkOrder
EnergyReservation
MaintenanceRecord
HandoffReceipt
QualityCertificate
AuthorityGrant
FailureReport
RecoveryPlan
BootstrapMilestone
```

A milestone records:

```text
milestone_id
prior_state_hash
result_state_hash
claimed_capability
evidence_refs
dependency_delta
imported_mass_delta
energy_delta
failure_rate
recovery_test
authority_basis
```

The important property is verifiability of the claim, not merely storage of the claim.

## 15. Symthaea Integration

Symthaea supplies bounded cognition for:

```text
state estimation
anomaly detection
world-model maintenance
predictive maintenance
resource-allocation proposals
process optimization
fault diagnosis
multi-step planning
```

Closed loop:

```text
Symthaea:
    predict -> plan -> confidence -> proposed action

runtime:
    authority check -> resource check -> safety envelope

physical system:
    execute -> observe

Mycelix:
    attest -> provenance -> distribute evidence

Symthaea:
    compare -> learn -> update
```

The cognitive model proposes; typed runtime authority and physical state decide whether execution is permitted.

## 16. Symtropy Integration

Symtropy owns deterministic physical simulation and worldline replay.

The bootstrap layer should expose:

```text
MassFlow
EnergyFlow
ThermalState
MachineState
FacilityState
ResourceDeposit
LogisticsEdge
FailureCascade
MaintenanceDebt
KnowledgeFrontier
```

LOD transitions must preserve:

```text
mass
energy
inventory
machine count
reservations
causal ordering
authority boundaries
durable evidence
```

## 17. Logistics

A bootstrap node is part of a physical transport graph:

```text
surface extraction
surface processing
launch/export
orbital staging
relay transfer
destination intake
```

Every transfer consumes:

```text
delta-v or equivalent transport budget
time
power
vehicle capacity
maintenance budget
risk budget
```

No global inventory may teleport between worlds.

The key Mercury question is therefore not only "can Mercury be mined?" but:

> Can Mercury-produced material create more useful industrial capability than the material, energy, and logistics capacity consumed to obtain and move it?

## 18. ISRU Process Architecture

Current NASA surface-technology work reinforces a useful modeling rule: resource extraction should be represented as an integrated process chain, not as a single "mine resource" action.

NASA's current lunar work includes concentrated-solar carbothermal reduction for oxygen production and molten-regolith electrolysis concepts that produce oxygen alongside metal-rich material. Blue Alchemist is explicitly framed as an autonomous end-to-end sequence producing silicon solar cells, aluminum wire, oxygen, iron, and slag from regolith simulants. These are lunar technologies, not Mercury demonstrations, but they provide a concrete engineering pattern for the simulator: co-products and process dependencies matter.

The generic process graph should therefore support:

Output and waste stream identifiers are part of the process schema, not presentation metadata. The kernel rejects duplicate product stream declarations and product/waste name collisions before an execution receipt can be minted; emitted waste batches retain the declared waste-stream identity so downstream recycling/disposal can distinguish material classes.

- resource characterization
- beneficiation / sorting
- feed preparation
- reduction / electrolysis
- co-product separation
- purification
- stock certification
- manufacturing
- recycling

Every transformation records:

- feed mass
- product mass by stream
- waste mass
- energy consumed
- peak power
- heat rejected
- yield
- quality
- contamination
- tooling wear

A process may be valuable because one operation produces several useful streams. Conversely, a process may look attractive on gross extraction while being poor at bootstrap because its purification, thermal, or tooling dependencies remain imported.

Graph definitions are themselves evidence-bearing input. Capability and terminal-dependency IDs must be non-empty and unique. Capability and terminal-dependency namespaces must also be disjoint; a shared identifier would otherwise make dependency resolution dependent on implementation lookup order. Duplicate definitions and cross-namespace collisions are rejected into an invalid graph state rather than silently letting later input overwrite or shadow earlier topology. An invalid graph cannot report full closure or non-zero closure ratios, and the exact definition errors remain available in the closure report. A valid graph with no positive critical-weight total likewise cannot claim full closure: the empty/vacuous denominator is not evidence of industrial independence.

The first reference ISRU fixtures should include:

- regolith -> oxygen + metal-rich stream
- metal-rich stream -> structural stock
- structural stock -> replacement tooling
- tooling -> additional processing capacity

The simulator must preserve the causal chain between these outputs. A kilogram of oxygen or metal cannot appear merely because a resource deposit is known.

## 19. Mercury Reference Sequence

```text
1. orbital reconnaissance
2. polar thermal/resource mapping
3. landing-zone characterization
4. autonomous survey rover
5. protected energy system
6. thermal-hardened fabrication cell
7. feedstock processing
8. bulk structural manufacturing
9. oxygen / metal / silicate processing
10. volatile pilot extraction
11. local consumables loop
12. machine-tool expansion
13. redundant factory cells
14. export logistics
15. second independent industrial node
```

The simulator should be able to stop at any stage and report the highest-value unresolved dependency.

## 20. Mercury Operating Bands

Mercury should not be modeled as one uniform industrial environment.

### Sunlit belt

Use high-irradiance regions for power capture and processes that benefit from high-temperature operation.

Candidate loads:

```text
solar concentration
thermal processing
metallurgy
glass / ceramics
bulk material handling
```

### Terminator corridor

Mercury's solar day is about 176 Earth days. Using the mean planetary radius, the apparent solar terminator moves around the equator at roughly 3.6 km/h (derived).

This creates a useful simulation hypothesis:

```text
follow dawn/dusk
  -> avoid peak noon heating
  -> avoid deepest night
  -> maintain moderate thermal gradients
  -> couple mobile industry to power availability
```

The runtime should permit a mobile industrial corridor to trade travel time against thermal stability rather than assuming a fixed base is always optimal.

This is a **design hypothesis**, not a demonstrated Mercury-operating strategy. Terrain, illumination geometry, communication, mobility energy, and actual thermal envelopes must be modeled before treating it as advantageous.

### Polar cold-trap zone

Polar operations should minimize heat leakage into volatile deposits.

The simulator must support:

```text
cold-region resource inventory
shadow-dependent thermal state
volatile loss from disturbed material
protected transport containers
energy cost of extraction and ascent
```

The objective is not maximum volatile removal. It is maintaining a durable consumables loop while preserving the resource environment.

## 21. Regression Fixtures

### Critical component loss

Mining, refining, and power remain online but bearing production fails.

Expected:

```text
closure claim fails
maintenance backlog rises
cannibalization begins
substitute designs are evaluated
energy is reallocated
recovery campaign is recorded
```

### Communication loss

Expected:

```text
local autonomy continues
remote information becomes stale
actions remain inside local authority
messages queue
remote state is not read directly
```

### Factory-class loss

Expected:

```text
production degrades
dependent capabilities are identified
spare capacity / mutual aid is invoked
recovery horizon is measured
```

## 22. Acceptance Tests

The first implementation should prove:

- no critical capability closes while an unrecognized terminal dependency remains;
- duplicate, empty, or cross-namespace capability/dependency identifiers make the graph invalid and fail closure closed;
- a graph with no positive critical-weight total cannot claim full industrial closure merely through a vacuous `0 == 0` comparison;
- critical-weight aggregation overflow is an invalid closure report rather than a wrapped or panicking result;
- authored dependency topology beyond the bounded resolution depth is an invalid closure report rather than a stack-exhaustion risk;
- mass-closure denominator overflow makes the closure report invalid rather than saturating the denominator;
- blocker-weight aggregation overflow is rejected rather than saturating the reported dependency importance;
- process-efficiency comparison remains exact at u64 boundary values without saturating cross-products;
- removing a critical machine class causes deterministic degradation, not free resource creation;
- inventory only increases through causal production events;
- energy deficits alter scheduling and survive LOD transitions;
- resource estimates cannot become inventory without an explicit evidence transition;
- machine reproduction consumes real inputs and produces real outputs;
- failed facilities leave maintenance, dependency, and provenance consequences;
- offline operation preserves local safety without creating remote omniscience;
- bootstrap milestones replay from evidence references;
- LOD transitions preserve mass, energy, machines, reservations, and unresolved failures;
- the simulator can identify the single largest unresolved blocker to industrial closure;
- execution recovery cannot replay a valid Pending receipt against a later or otherwise different budget/inventory state even when the textual journal frontier is reused;
- estimated resources require an explicit evidence-certification transition before entering inventory;
- process definitions reject duplicate output-stream IDs and reject a waste stream that collides with a product stream;
- process executions reference declared input material and a concrete source batch;
- executable authorization reserves that concrete source batch before committing aggregate feed/energy budget capacity;
- multiple executions may reserve disjoint quantities from the same source batch only when their aggregate reservation does not exceed current stock;
- process executions cannot exceed available feedstock or energy budgets;
- a funded process execution must reserve feedstock and energy exactly once within its authorization budget;
- an execution must have a non-empty execution identity before a receipt can be minted;
- validated process executions emit deterministic causal inventory events with unique event identities and provenance;
- validated process executions emit replayable energy-consumption events with unique event identities and provenance;
- recycling is represented as an explicit consumed source batch plus produced destination batch, never as implicit mass creation;
- process waste is emitted as a named produced batch, so strict process mass balance is preserved in inventory history rather than allowing waste to disappear between process and ledger;
- a certified resource must have a non-empty claim identity and cannot be constructed directly outside the evidence gate;
- candidate failure-risk and resource-confidence values outside 0..1,000,000 ppm are rejected rather than clamped into a valid-looking value;
- validated candidate and resource-claim value objects seal bounded constructor invariants so external struct literals cannot bypass them;
- certification-derived inventory events use the certificate identity as their event identity, preventing the same certified quantity from being minted twice;
- ledger replay rejects missing or duplicate event identities, unprovenanced events, and overdrawn history;
- stateful ledger append rejects duplicate identities, non-monotonic sequences, and empty physical account identifiers before mutation;
- stateful ledger batch append validates the complete batch before committing any event;
- a durable execution anchor is bound to the exact pre-Pending budget/inventory state with a deterministic SHA-256 commitment;
- recovery rejects a Pending receipt when either the verified journal frontier or the supplied kernel state commitment differs, before mutating budget or inventory;
- the explicit in-memory `UNANCHORED` sentinel cannot be promoted into a state-bound durable anchor by attaching a commitment;
- the canonical receipt commitment is compared with the authenticated durable lifecycle record before recovery can confer executable authority;
- rejected ledger appends leave the prior state and accepted history unchanged, including when a later event in a batch fails;
- one authorized process execution commits budget settlement, source-batch settlement, material, and energy as one kernel transaction, with all three mutable state objects staged before live replacement;
- cross-ledger commit failure leaves budget, ledgers, and concrete source-batch reservations unchanged and keeps the execution pending for retry;
- settling one partial source reservation does not consume or erase another execution's reservation on the same batch;
- the pending budget reservation is bound to the complete immutable receipt, including process identity, source batch, inventory and energy sequence positions, energy node, and the complete process run;
- the pending physical source reservation is bound to the same execution, batch, and exact feed quantity;
- budget-only authorization returns the distinct `BudgetOnlyProcessExecutionReceipt` type; it remains inspection/provenance-only, while only the executable receipt type can cross the physical commit/abort boundary or materialize causal inventory/energy events;
- budget-only authorization has its own explicit compensation path: `abort_budget_only_execution` releases reserved feed/energy without requiring physical source state, while the physical `abort_pending_execution` boundary rejects such receipts;
- output and waste batch identities are derived from the authorized execution identity as well as process/sequence context, so distinct executions cannot alias the same physical product batch merely by reusing a process and sequence position;
- commit and abort reject a receipt that does not exactly match the still-pending authorization, even when its feedstock and energy quantities match;
- a pending execution can be explicitly rehydrated from the authoritative budget receipt plus matching source reservation without minting a second authorization;
- durable integration has an explicit pending-authorization boundary: the raw pending receipt must cross persistence before executable activation, while budget-only or merely in-memory authorization cannot cross that boundary;
- a persisted Pending receipt can be reconstructed into the raw receipt type and restored against the pre-transition budget/inventory state without minting a new execution identity;
- a Pending receipt carries an explicit domain-bound execution-state anchor, and durable recovery rejects a receipt whose anchor does not equal the independently verified journal/state frontier;
- executable and budget-only receipt wrappers expose read-only accessors rather than `Deref` to the raw receipt, so raw-receipt APIs cannot acquire either authority class through implicit coercion;
- legal execution lifecycle transitions are encoded by the kernel as `Pending -> Committed` or `Pending -> Aborted`; terminal-to-terminal, terminal-to-pending, and repeated-state transitions are invalid rather than implicitly idempotent;
- persistence failure after pending authorization has a safe raw-receipt compensation path that aborts and refunds the reservation without exposing executable event materialization;
- budget-only reservations have a distinct cancellation path and cannot be mistaken for executions with concrete physical-source reservations;
- execution lifecycle state is explicitly distinguishable as Pending, Committed, or Aborted, and each lifecycle record retains the exact immutable receipt that caused the transition, giving durable recovery an unambiguous terminal outcome and causal identity;
- an explicitly aborted execution restores its reserved feedstock and energy and releases its concrete source-batch reservation without touching ledger history;
- an aborted execution becomes terminal and cannot reuse its execution identity;
- a committed execution cannot be committed again because its execution identity is settled;
- distinct authorized executions with the same process and sequence positions cannot emit aliased product or waste batch identities;
- re-emitting the same execution receipt is detectable as duplicate causal history rather than a second valid execution;
- invalid closure reports cannot qualify an industrial stage, even if their assessment payload happens to contain closed capabilities;
- industrial stage progression cannot skip an unresolved earlier stage;
- empty or duplicate stage requirements are invalid and cannot qualify an industrial stage.

## 23. Kill Criteria

Do not add a subsystem that:

- collapses closure into one technology score;
- hides critical dependencies behind an aggregate number;
- silently overwrites duplicate graph capability/dependency definitions;
- resolves a capability/dependency ID collision by namespace order instead of rejecting the ambiguous topology;
- treats an empty or zero-weight critical assessment set as evidence of full closure;
- permits critical-weight arithmetic to overflow, wrap, or panic instead of producing an invalid report with an explicit error;
- qualifies an industrial stage from an invalid closure report;
- qualifies a stage from an empty requirement set or ambiguous duplicate requirements for the same stage;
- allows free machine or material duplication;
- assumes perfect global communication;
- treats speculative resources as established reserves;
- makes reproduction a boolean;
- reuses the same feedstock or energy authorization to mint multiple valid executions;
- authorizes multiple executions against the same unreserved physical source stock by checking only aggregate feed capacity;
- lets an aggregate budget-only receipt cross the executable commit/abort or causal-event-materialization boundary without a concrete source reservation proof;
- lets one execution's settlement consume another execution's reserved portion of a shared source batch;
- publishes inventory or energy changes before budget settlement is itself safely staged;
- constructs or reuses the same certified resource quantity to mint multiple inventory events;
- treats recycling as a scalar balance increase without an explicit source batch;
- permits process waste to disappear from causal inventory history even though the process reports it as accounted mass;
- permits ambiguous process stream definitions or a waste/product stream collision;
- accepts duplicate ledger events under different sequence numbers when their event identities are the same;
- admits an inventory event with an empty batch ID or an energy event with an empty node ID;
- mutates a ledger before append validation completes;
- partially commits a funded execution to one ledger while another required ledger rejects it;
- lets unrelated inventory consumption bypass a pending source-batch reservation;
- loses a reserved execution silently when cross-ledger commit fails;
- loses a concrete source-batch reservation when authorization fails or an execution is explicitly aborted;
- permits a pending execution reservation to be paired with a different process, source batch, sequence position, energy node, run, or journal/state frontier while retaining the same execution identity;
- creates a fresh authorization when recovering an interrupted pending execution instead of rehydrating the still-authoritative reservation;
- collapses committed and aborted terminal executions into one indistinguishable state in the lifecycle boundary;
- persists a terminal execution outcome without retaining the exact receipt/process/source/run identity that produced it;
- permits distinct execution identities to alias one physical product or waste batch merely because process and sequence fields match;
- reuses an aborted execution identity as a fresh authorization;
- rewards extraction while reducing recovery capability;
- cannot produce a deterministic post-failure explanation.

## 24. Strategic Principle

The objective is not:

> extract Mercury.

It is:

> **turn planetary matter and solar energy into an increasingly autonomous, repairable, evidence-grounded industrial ecology without surrendering human authority over its purposes.**

Mercury is a valuable proving ground because it forces the architecture to confront energy abundance, thermal hostility, communication delay, uncertain resources, autonomous maintenance, industrial dependency, and long recovery horizons in one environment.

## 25. Digital-Thread Evidence and Causal Execution

The bootstrap runtime should treat industrial state as a digital thread: claims, process definitions, executions, products, and verification evidence remain linked instead of being flattened into disconnected scalar state. NIST's digital-thread work emphasizes traceability across engineering, manufacturing, and quality data, including conformance checking and persistent identifiers. NIST's September 2026 circular-manufacturing research likewise identifies system-level modeling, digital threads, measurement science, comparable metrics, test methods, and interoperability as open research needs. NASA systems-engineering guidance distinguishes verification/validation from simply asserting that a capability exists and emphasizes recorded objective evidence and end-to-end configuration tracing. [NIST digital-thread research](https://www.nist.gov/programs-projects/digital-thread-manufacturing), [NIST circular-manufacturing research](https://www.nist.gov/publications/manufacturing-circular-economy-research-needs-design-systems-modeling-and-digital), and [NASA systems-engineering verification guidance](https://www.nasa.gov/reference/5-3-product-verification/)

The kernel therefore now separates three transitions:

For a process execution, strict mass balance applies to the complete causal inventory event set. The consumed source batch is offset by every declared product stream and by a named waste batch; any subsequent disposal, containment, or recycling of that waste must be represented by another explicit event.

```text
resource claim
  -> explicit evidence certification
  -> inventory-eligible quantity

declared process
  + identified input batch
  + consumable feedstock/energy budget
  -> single-use execution authorization
  -> immutable process execution receipt
  -> causal inventory events
  -> causal energy-consumption event
```

A mass-balanced process is not sufficient by itself. The run must reference the declared input material, a non-empty source batch, and sufficient feedstock and energy. Process definitions and ledger events also require non-empty physical account identifiers so causal state cannot be silently attached to an anonymous batch, node, or process account. Executable authorization first reserves the concrete source batch in the inventory ledger, then consumes aggregate feedstock and energy capacity from an `ExecutionBudget`. The two reservations are released together on authorization failure or abort. A source batch may be partitioned among multiple pending executions, but each reservation is an exact quantity claim and settlement removes only the owner's claim. The resulting executable receipt owns the process/run context and is the only public capability that can materialize stable inventory/energy events; those events derive stable event identities and execution-bound product/waste batch identities; downstream ledger replay rejects re-emission of those identities even when a caller presents the duplicated history at different sequence numbers. This avoids conflating two distinct executions when process and local sequence values happen to coincide. The public executable-receipt wrapper has a private constructor and is only minted after the concrete source reservation succeeds; a budget-only ProcessExecutionReceipt therefore cannot be accidentally accepted by the executable commit/abort APIs.

The live ledgers expose the same invariant at append time. `InventoryLedger` and `EnergyLedger` maintain their balance, accepted event history, unique event-ID set, and sequence frontier together. `InventoryLedger` additionally tracks pending concrete source-batch reservations; a reserved source cannot be consumed through the ordinary append path by another execution. An append validates identity, provenance, sequence monotonicity, overflow, and underflow before mutating state; a rejected append therefore cannot partially alter the ledger. Batch append validates the entire event group against staged balances and identities before committing any mutation, so one bad event cannot leave a process half-applied. Full-history replay remains available as a deterministic reconstruction/checking path, while the stateful ledger is the runtime admission boundary.

Cross-ledger execution follows a prepared/commit pattern:

```text
authorize
  -> reserve feedstock + energy
  -> immutable execution receipt

prepare
  -> stage inventory batch
  -> stage energy event
  -> validate both completely

commit
  -> replace both live ledgers
  -> settle execution reservation
```

The kernel performs preparation against inventory and energy clones and settles the budget on its own clone. No one of the three live state objects is replaced until all staged operations succeed. The inventory clone first consumes and settles the exact source reservation; if the energy stage then fails, that clone is discarded and the live source reservation remains pending for retry. The budget stores the complete immutable receipt as the pending authorization, rather than only its quantities, so the commit/abort boundary is bound to the exact process, source batch, sequence positions, energy node, and run that were authorized. A different receipt cannot consume the reservation merely because it requests the same feedstock and energy amounts.

A failed preparation leaves the live budget, ledgers, and source reservation untouched; the exact receipt can therefore be retried after the required physical state is restored. An execution that will not be retried can be explicitly aborted, which returns its reserved feedstock and energy while permanently retiring that execution identity. Abort stages the budget and source-reservation changes before replacing either live state. This mirrors the prepare/commit/rollback shape used by transactional systems, but the current kernel remains an in-memory deterministic coordination boundary: it is not a durable transaction log, distributed consensus protocol, or crash-recovery mechanism for external systems. PostgreSQL's two-phase transaction model similarly separates prepare from later commit/rollback and requires an external manager to close prepared transactions promptly; it is therefore a reference for the lifecycle shape, not evidence that this kernel is already durable. (PostgreSQL 18, https://www.postgresql.org/docs/18/two-phase.html)

The remaining durability boundary is explicit: a process can be authorized and physically reserved in memory, then the process can terminate before commit/abort, leaving no durable pending-execution record. Within the in-memory kernel, an interrupted caller can rehydrate the executable proof from the still-pending budget receipt and matching source reservation without creating fresh capacity. A durable integration must additionally bind that Pending receipt to the exact verified journal/state frontier from which the reservation was created. The receipt therefore carries an opaque, domain-bound execution-state anchor containing the verified frontier plus a SHA-256 commitment to the exact kernel state immediately before the Pending transition. Durable authorization uses `authorize_pending_execution_with_inventory_at_anchor`, and recovery uses `restore_pending_execution_with_inventory_at_anchor` with the independently verified current frontier. The kernel verifies both anchor equality and the supplied budget/inventory state commitment before any recovery mutation. The bootstrap kernel remains storage-independent: the adapter supplies the verified journal frontier and state-derived commitment, while the kernel enforces the binding.

### Durable adapter contract

The next integration layer should use an append-only, journal-first execution record rather than independently persisting the budget, inventory, and energy ledgers and attempting to infer atomicity afterward. Budget-only inspection authorizations are separate: they reserve only aggregate budget capacity and must be canceled through the budget-only compensation path rather than being represented as physically executable work. The durable record for each lifecycle transition must retain enough information to reconstruct the same causal execution without inventing a new authorization:

```text
execution_id
lifecycle_state
complete_process_execution_receipt
  reconstructed as a raw receipt before executable activation:
    process_id
    input_batch_id
    waste_stream
    first_inventory_sequence
    energy_sequence
    energy_node_id
    execution_state_anchor:
      domain
      frontier
      state_commitment (lowercase SHA-256 of the exact pre-Pending budget/inventory state)
    complete_process_run
receipt_commitment (canonical SHA-256 of the exact complete receipt)
causal event identities / payloads required by the adapter
schema_version
```

The adapter must use the explicit pending-authorization API and enforce this ordering:

```text
authorize_pending_execution_with_inventory_at_anchor
    -> derive state-bound ExecutionStateAnchor from verified frontier + exact pre-Pending budget/inventory state
    -> durably record Pending + exact receipt + receipt commitment + source reservation + verified frontier + state commitment
    -> reconstruct raw ProcessExecutionReceipt from persisted fields
    -> recompute receipt commitment and compare it with the authenticated Pending record
    -> restore_pending_execution_with_inventory_at_anchor against the same verified frontier and state
    -> resume_pending_execution only after durable Pending exists
    -> stage deterministic ledger effects
    -> durably record exactly one terminal outcome
    -> rebuild live state from the journal
```

The existing authorize_execution_with_inventory convenience path remains valid for purely in-memory callers, but a durable adapter must use `authorize_execution_with_inventory_at_anchor` and must not expose its executable result before the Pending record has crossed the persistence boundary. On recovery, the adapter must use `restore_pending_execution_with_inventory_at_anchor` with the exact independently verified frontier that is bound into the receipt and must reconstruct the budget/inventory state represented by that frontier. The kernel then recomputes the state commitment and refuses recovery when the supplied state differs, even when the textual frontier matches. The budget-only `authorize_execution` path is likewise type-separated, so its reservation can only be released through `abort_budget_only_execution`. The raw ProcessExecutionReceipt intentionally cannot materialize causal events; executable activation is the explicit proof transition after the pending reservation has been durably represented. When persistence of the Pending record fails, the adapter should use `abort_pending_execution` with the raw receipt to release the reservation and retire the execution without ever minting executable proof. This makes failed persistence a compensating abort, not a reason to weaken the activation boundary.

Commit and abort are terminal alternatives, never independent facts inferred from whichever ledger happened to contain a later event. A committed transition must carry the same receipt identity that was pending; an aborted transition must carry the same receipt identity while carrying no product/energy consumption effects. A second terminal transition for the same execution ID is invalid.

Recovery is therefore deterministic:

```text
journal verification
    -> replay lifecycle transitions
    -> Pending with no terminal successor:
           reconstruct reservation state
           verify concrete source reservation
           call resume_pending_execution
    -> Committed:
           terminal; never re-authorize or rehydrate
    -> Aborted:
           terminal; never re-authorize or rehydrate
```

The adapter must fail closed on an unknown lifecycle state, duplicate terminal outcome, terminal receipt mismatch, receipt commitment mismatch, missing causal predecessor, impossible sequence transition, an unbound durable execution anchor, a Pending receipt whose execution-state anchor does not match the verified journal frontier, a Pending receipt whose state commitment does not match the reconstructed budget/inventory state, or a committed outcome whose required causal events cannot be reconstructed exactly. Receipt reconstruction does not itself confer execution authority; process-definition validation and pending reservation restoration remain mandatory before executable activation. The kernel's `ExecutionState::can_transition_to` provides the common legal transition rule so adapters do not invent a second lifecycle semantics.

Snapshots may be used as checkpoints, but they are not the authority for resolving an interrupted execution unless their journal anchor covers the corresponding lifecycle record. The existing symtropy-persistence journal and snapshot machinery is therefore a natural implementation substrate: its verified `event_head_hash` supplies the frontier, while the adapter reconstructs the corresponding kernel state before deriving the state commitment. The bootstrap kernel remains independent of filesystem or database APIs.

This design deliberately differs from treating the three in-memory ledgers as three separately durable transactions. NIST IR 8536 emphasizes linked traceability information that can support independent product-history verification; the execution lifecycle record is the corresponding causal link for process authorization and industrial state. PostgreSQL 18's two-phase transaction model is a useful analogy for the Pending -> terminal shape, but it relies on an external transaction manager to resolve prepared work and is not evidence that this kernel or adapter is durable merely because it resembles 2PC.

Resource certification follows the same rule. A certified resource has a stable certificate identity derived from its claim identity, while the certificate's fields are private so inventory authorization cannot be forged by direct construction. The certificate-derived inventory event reuses that certificate identity as its event identity; replay therefore rejects the same certified quantity being emitted again under a different sequence or batch.

Recycling uses the same conservation boundary. A recovery operation must consume a named source batch and separately produce the recovered destination batch. This prevents a convenient `Recycled` balance mutation from hiding where recovered mass came from or creating mass without a source.

These identities establish deterministic causal linkage, not cryptographic authenticity by themselves. Higher layers may bind claims and certificates to signed external evidence or verifiable credentials. W3C's Verifiable Credential Data Integrity 1.0 is a Recommendation for cryptographic authenticity/integrity mechanisms; the 2026 1.1 document is still a Working Draft, so the runtime does not claim to implement either standard merely by naming certificate fields.

This is intentionally an authorization boundary, not a claim that physical execution occurred. The receipt proves that the deterministic kernel admitted one funded execution from the declared state. Physical telemetry, quality measurements, thermal observations, and external evidence remain separate evidence inputs that higher layers must attach before asserting realized product performance.

The bootstrap runtime therefore has an explicit lifecycle vocabulary for future durable integration:

```text
Pending -> Committed
Pending -> Aborted
Committed / Aborted -> terminal
```

A terminal outcome is retained together with the exact immutable receipt as execution identity state rather than inferred indirectly from ledger effects. A recovery adapter can therefore replay lifecycle records deterministically, distinguish an unfinished execution from one that was deliberately canceled, and carry the same causal process/source/run context across a persistence boundary.

This also gives the simulator a stronger digital-thread boundary:

```text
evidence
  -> claim
  -> certification
  -> stock
  -> process definition
  -> process execution
  -> product batch
  -> later manufacturing/recycling event
```

No speculative deposit, unbudgeted process, disconnected production event, double-reserved source batch, or unprovenanced ledger delta should silently cross that boundary.

Closure stages are similarly monotonic. A later industrial stage must not be reported merely because its local requirement happens to be satisfied while an earlier required stage is unresolved. The stage ladder is therefore evaluated in order and stops at the first unmet requirement. Stage definitions themselves are part of the qualification boundary: an empty requirement set or duplicate requirements for the same stage are treated as ambiguous and fail closed.

These rules are intentionally general: they apply to Mercury, the Moon, asteroids, terrestrial closed-loop industry, and simulation environments where evidence quality and causal reconstruction matter more than optimistic point estimates.

## References

- NASA, Mercury Facts: https://science.nasa.gov/mercury/facts/
- JPL Solar System Dynamics, Planetary Physical Parameters: https://ssd.jpl.nasa.gov/planets/phys_par.html
- NASA, MESSENGER Mission: https://science.nasa.gov/mission/messenger/
- NASA, Water Ice on Mercury: https://science.nasa.gov/photojournal/water-ice-on-mercury/
- NASA, MESSENGER volatile-rich Mercury findings: https://astrobiology.nasa.gov/missions/messenger/
- ESA, BepiColombo arrival updates: https://www.esa.int/Science_Exploration/Space_Science/BepiColombo/Latest_updates_BepiColombo_s_arrival_at_Mercury
- NASA, Lunar Surface Technology: https://www.nasa.gov/lunar-surface-technology/
- NASA TechPort, ISRU-Based Power on the Moon (Blue Alchemist): https://techport.nasa.gov/projects/146991
- NIST, Digital Thread for Manufacturing: https://www.nist.gov/programs-projects/digital-thread-manufacturing
- NIST, Manufacturing in a Circular Economy: Research Needs in Design, Systems Modeling, and Digital Thread: https://www.nist.gov/publications/manufacturing-circular-economy-research-needs-design-systems-modeling-and-digital
- NASA, Product Verification: https://www.nasa.gov/reference/5-3-product-verification/
- NIST, UUIDs in Product Data Standards: https://www.nist.gov/publications/research-results-and-recommendations-universally-unique-identifiers-product-data
- W3C, Verifiable Credential Data Integrity 1.0: https://www.w3.org/TR/vc-data-integrity/
- PostgreSQL, Two-Phase Transactions: https://www.postgresql.org/docs/current/two-phase.html
- NASA, Product Implementation: https://www.nasa.gov/reference/5-1-product-implementation/
- NASA, Product Realization / Verification Guidance: https://www.nasa.gov/reference/5-0-product-realization/
- NASA, Product Validation: https://www.nasa.gov/reference/5-4-product-validation/
