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

The first reference ISRU fixtures should include:

- regolith -> oxygen + metal-rich stream
- metal-rich stream -> structural stock
- structural stock -> replacement tooling
- tooling -> additional processing capacity

The simulator must preserve the causal chain between these outputs. A kilogram of oxygen or metal cannot appear merely because a resource deposit is known.

## 18. Mercury Reference Sequence

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

## 19. Mercury Operating Bands

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

## 19. Regression Fixtures

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

## 20. Acceptance Tests

The first implementation should prove:

- no critical capability closes while an unrecognized terminal dependency remains;
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
- estimated resources require an explicit evidence-certification transition before entering inventory;
- process executions reference declared input material and a concrete source batch;
- process executions cannot exceed available feedstock or energy budgets;
- validated process executions emit deterministic causal inventory events;
- industrial stage progression cannot skip an unresolved earlier stage.

## 21. Kill Criteria

Do not add a subsystem that:

- collapses closure into one technology score;
- hides critical dependencies behind an aggregate number;
- allows free machine or material duplication;
- assumes perfect global communication;
- treats speculative resources as established reserves;
- makes reproduction a boolean;
- rewards extraction while reducing recovery capability;
- cannot produce a deterministic post-failure explanation.

## 22. Strategic Principle

The objective is not:

> extract Mercury.

It is:

> **turn planetary matter and solar energy into an increasingly autonomous, repairable, evidence-grounded industrial ecology without surrendering human authority over its purposes.**

Mercury is a valuable proving ground because it forces the architecture to confront energy abundance, thermal hostility, communication delay, uncertain resources, autonomous maintenance, industrial dependency, and long recovery horizons in one environment.

## 23. Digital-Thread Evidence and Causal Execution

The bootstrap runtime should treat industrial state as a digital thread: claims, process definitions, executions, products, and verification evidence remain linked instead of being flattened into disconnected scalar state. NIST's digital-thread work emphasizes traceability across engineering, manufacturing, and quality data, including conformance checking and persistent identifiers. NASA systems-engineering guidance likewise distinguishes verification/validation from simply asserting that a capability exists. [NIST digital-thread research](https://www.nist.gov/programs-projects/digital-thread-manufacturing) and [NASA systems-engineering verification guidance](https://www.nasa.gov/reference/5-0-product-realization/)

The kernel therefore now separates three transitions:

```text
resource claim
  -> explicit evidence certification
  -> inventory-eligible quantity

declared process
  + identified input batch
  + available feedstock budget
  + available energy budget
  -> validated process execution
  -> causal inventory events
```

A mass-balanced process is not sufficient by itself. The run must reference the declared input material, a non-empty source batch, and sufficient feedstock and energy budgets. Validated executions can then emit deterministic consumed/produced inventory events carrying a stable causal provenance identifier.

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

No speculative deposit, unbudgeted process, or disconnected production event should silently cross that boundary.

Closure stages are similarly monotonic. A later industrial stage must not be reported merely because its local requirement happens to be satisfied while an earlier required stage is unresolved. The stage ladder is therefore evaluated in order and stops at the first unmet requirement.

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
