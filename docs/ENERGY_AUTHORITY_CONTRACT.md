# Energy Authority Contract

## Status

This document defines the Phase-Zero accounting boundary for energy-like state in
Symtropy. It exists to prevent a simulation from becoming internally coherent
while still being physically false.

The central rule is:

> **A quantity may enter the physical energy ledger only when its units,
> reservoir ownership, transfer endpoints, and conversion mechanism are known.**

Anything else must remain operational budget state or heuristic evidence until a
validated conversion exists.

This contract complements the core `symtropy-physics` thermodynamic program and
issue #40, which converges duplicate thermal state onto one physical authority.

## Why this contract is needed

The repository currently contains several concepts that historically used the
word "energy" but do not all mean the same thing:

- rigid-body kinetic and potential energy;
- physical sensible/latent thermal energy;
- finite gameplay/agent capability budgets;
- maintenance and action costs;
- energy-well inventory;
- motor work and regenerative braking;
- collision impulse and sanctuary absorption heuristics;
- prediction error and harmony/resonance signals;
- a legacy consciousness-domain `ThermodynamicLedger`;
- the newer core double-entry `EnergyTransferLedger`.

Those quantities must not be collapsed into one scalar merely because they are
all useful for gameplay or modeling.

## Three authority channels

### 1. Physical energy

Physical energy is state or transfer that is allowed to participate in first-law
closure.

Examples include, once validated:

- translational and rotational kinetic energy;
- gravitational potential energy;
- elastic strain energy;
- sensible and latent thermal energy;
- fracture-surface energy;
- chemical/electrical/radiant reservoirs;
- externally supplied or removed work/heat.

Physical energy belongs in the core `symtropy-physics` accounting path:

- `EnergyOwner`;
- `EnergyForm`;
- `EnergyPort`;
- `EnergyTransferKind`;
- `EnergyTransferLedger`;
- measured reservoir reconciliation.

Requirements for entry into this channel:

1. the value is finite;
2. the transfer is strictly positive and expressed in joules;
3. source and destination reservoirs are explicit;
4. the physical mechanism is explicit;
5. the numerical state change can be reconciled against the transfer;
6. the value is not inferred from a dimensionally incompatible proxy.

A balanced journal is not sufficient by itself. First-law evidence additionally
requires complete measured endpoint reservoirs.

### 2. Operational / capability budget

Operational budget controls whether an embodied agent or gameplay system has the
capacity to act. It may be inspired by metabolic, electrical, computational, or
resource constraints, but it is **not automatically total physical internal
energy**.

Current examples include the consciousness-domain `EnergyBudget.available`,
maintenance debit, explicit energy-well credit, collapse, and motor-authority
budgeting.

Until a physical state equation is defined, this channel should be treated as an
operational reservoir with explicit debit/credit semantics.

Permitted operations include:

- bounded debit;
- bounded credit;
- finite source-to-destination transfer;
- collapse / recovery policy;
- derived capability or authority state;
- research telemetry such as cost per task or cost per Φ change.

Operational budget may use joules only when the calibration is explicit. If the
calibration is merely a gameplay scale, the API/documentation must say so.

An operational debit does **not** become physical heat merely because some real
systems eventually dissipate their consumed free energy as heat. That conversion
requires an explicit efficiency/mechanism model and a physical destination
reservoir.

### 3. Heuristic / semantic evidence

Heuristic evidence may influence prediction, scheduling, safety, gameplay, or
future model selection, but cannot enter a physical joule balance directly.

Examples include:

- collision impulse magnitude (`N·s`);
- contact surprise / prediction error;
- harmony similarity or resonance;
- HDC similarity / novelty;
- sanctuary attenuation factors;
- arbitrary damage or stress scores;
- learned CfC/LTC predictions without calibration to a physical error metric.

This channel must retain its actual unit or remain explicitly dimensionless.

A heuristic can become a physical transfer only through a validated conversion
whose input/output units and calibration protocol are documented.

## Prohibited implicit conversions

The following patterns are forbidden as physical accounting evidence:

### Impulse multiplied by an arbitrary scalar and labeled joules

Impulse has units of momentum (`kg·m/s`), not energy. Expressions such as:

`heat = abs(impulse) * 0.1`

or

`absorbed_energy = absorbed_impulse * 0.5`

are not physical energy conversions without additional velocity/effective-mass
information and a validated derivation.

They may remain heuristic gameplay costs, but must not enter the physical energy
ledger as joules.

### Negative dissipation used to mean energy input

Dissipation is not a signed boundary-flow channel. Incoming energy must be an
explicit source-to-destination transfer.

Do not encode external inflow as `record_dissipation(-x)`.

### Cost reduction represented as regeneration

Avoid this sequence:

1. debit full cost;
2. create a synthetic regeneration credit to represent reduced work.

If the mechanism means less work was required, reduce the debit before it occurs.
This is the policy used by the epistemic-offloading correction in #43.

### Direct mutation of lifetime accounting totals

Code must not bypass the transfer/event API by directly adding/subtracting a
cumulative accounting field. Regenerative braking or external injection needs a
normal transfer record with provenance.

### Duplicate physical temperatures

One embodied object may not have independent physical temperatures in both a
consciousness budget and its core rigid body. Issue #40 owns migration to one
`ThermalBody` authority.

## Current-path classification

The following table describes the intended classification during migration.

| Current path | Current meaning | Authority channel | Migration direction |
| --- | --- | --- | --- |
| `RigidBody` kinetic/potential state | measured mechanical state | Physical | core ledger + reconciliation |
| `RigidBody::thermal` / `ThermalBody` | physical thermal state | Physical | canonical thermal owner |
| `EnergyBudget.available` | agent operational reserve | Operational | retain as capability/metabolic budget after T1 semantics |
| `EnergyWell.remaining` | finite operational source inventory | Operational | source/destination conservation, then calibrate physical meaning if desired |
| consciousness maintenance | operational debit | Operational | do not call physical dissipation by default |
| epistemic offloading | reduced duplicated processing cost | Operational | pre-debit cost reduction, no synthetic credit |
| ambient regeneration | operational environmental support | Operational | explicit non-resurrection policy unless promoted to a modeled source |
| collision impulse | momentum-transfer evidence | Heuristic/physical input | derive measured mechanical energy loss before heat conversion |
| prediction error | inference/surprise signal | Heuristic | never joules without calibration |
| harmony/resonance | semantic/social coupling signal | Heuristic | may alter policy/cost but is not energy itself |
| sanctuary absorbed impulse | impulse attenuation signal | Heuristic | derive actual mechanical-energy delta before physical ledger entry |
| legacy `ThermodynamicLedger` | mixed historical telemetry | Legacy / non-authoritative | retire first-law claims; migrate physical entries to core ledger |
| core `EnergyTransferLedger` | typed double-entry physical transfers | Physical | canonical physical transfer journal |

## Source policy for operational recovery

Operational recovery sources must be explicit.

The current intended policy after #42-#44 is:

- finite energy well: allowed recovery source;
- ambient regeneration: may refill a live entity but cannot resurrect collapse;
- epistemic offloading: reduces work and is never a recovery source;
- future medical, electrical, mechanical, or dimensional recovery: must declare a
  source type and accounting semantics before it can recover a collapsed entity.

This policy prevents generic `regenerate()` calls from silently changing the
meaning of collapse.

## Physical heat admission gate

Before any gameplay/cognitive mechanism can deposit physical heat into
`ThermalBody`, all of the following must be known:

1. **source reservoir** — where the energy came from;
2. **destination reservoir** — the exact body/world thermal port;
3. **joule amount** — derived from dimensionally valid state;
4. **conversion efficiency** — if not all source work becomes heat;
5. **timing** — the transfer belongs to the same authoritative fixed tick;
6. **reconciliation** — source decrease and destination increase are measured;
7. **failure policy** — missing/invalid evidence fails closed instead of inventing
   a transfer.

Examples:

- friction: measure pre/post mechanical energy around the friction impulse, then
  transfer only measured positive loss;
- motor inefficiency: measured external/chemical/electrical work × validated loss
  fraction -> thermal destination;
- inelastic collision: derive mechanical-energy loss from pre/post state, not
  impulse magnitude alone.

## Legacy `ThermodynamicLedger` claim boundary

The consciousness-domain legacy ledger must **not** currently be treated as a
first-law validator.

Reasons include:

- `energy_in` historically means action/operational debit, not external physical
  energy entering the modeled boundary;
- `energy_out` mixes maintenance, heat-like costs, and heuristic dissipation;
- regeneration is not represented consistently;
- some call sites historically encoded incoming energy as negative dissipation;
- reservoir endpoint deltas are not part of its balance equation;
- impulse-derived approximations can enter the same counters;
- direct lifetime-field mutation exists in regenerative-braking code.

Therefore its `conservation_error` and `lifetime_error_rate` are legacy diagnostic
ratios only. They are not evidence that physical energy is conserved.

Physical conservation claims require the core typed transfer ledger plus complete
reservoir reconciliation.

## Relationship to Joules-per-Φ

Joules-per-Φ may remain useful research telemetry, but only after its numerator
has a declared authority source.

Valid variants should be named precisely, for example:

- operational-budget-joules per ΔΦ;
- measured external-work joules per ΔΦ;
- measured thermal dissipation joules per ΔΦ.

Do not combine those numerators silently.

A correlation between operational cost and Φ is not itself a thermodynamic law.

## Migration gates

### E0 — inventory and claim freeze

- classify every energy-like field/call site into Physical, Operational, or
  Heuristic;
- remove first-law wording from legacy mixed telemetry;
- forbid new impulse-to-joule shortcuts.

### E1 — operational budget semantics

- define whether the budget is metabolic free energy, electrical reserve,
  gameplay capability, or another explicit quantity;
- define units and source types;
- keep actual accepted debit/credit arithmetic transactional (#39).

### E2 — operational source conservation

- finite wells debit only accepted destination transfer (#42);
- cost reductions are pre-debit rather than synthetic credits (#43);
- recovery sources are explicit (#44);
- add deterministic multi-agent source-sharing tests.

### E3 — physical thermal authority

- complete #40 T0-T4;
- consciousness temperature effects read core `ThermalBody`;
- retire duplicate physical temperature/entropy mutation from `EnergyBudget`.

### E4 — measured mechanical-to-thermal conversion

- remove impulse-proportional friction/collision heat;
- use measured kinetic/mechanical losses;
- journal transfers through core `EnergyTransferLedger`.

### E5 — boundary-source accounting

- model external work, leakage, charging, radiation, and other boundary flows with
  explicit external owners/ports;
- remove signed-negative-dissipation conventions.

### E6 — validation and claims

For each claimed conserved interval require:

- complete reservoir inventory at both endpoints;
- deterministic ledger replay;
- finite typed transfers;
- measured state delta;
- external-flow closure;
- negative controls with omitted reservoir/unjournaled transfer;
- declared tolerance and numerical-residual policy.

Only then may the result be labeled first-law validation.

## Negative controls

The authority contract should eventually have executable tests proving:

1. impulse-only evidence cannot create a physical-joule transfer;
2. negative dissipation cannot encode an incoming boundary flow;
3. full operational receiver cannot drain a finite source (#42);
4. social/offload benefit cannot increment regeneration telemetry (#43);
5. ambient support cannot resurrect a collapsed reservoir (#44);
6. a finite explicit source can recover collapse with matched source/destination
   deltas (#44);
7. operational debit does not alter physical temperature without an explicit
   conversion transfer;
8. physical heat transfer changes exactly one authoritative thermal state;
9. legacy ledger balance cannot satisfy a first-law gate by itself;
10. omitting one declared reservoir makes strict physical accounting incomplete,
    even if the remaining arithmetic closes numerically.

## Long-term target

The desired architecture is:

```text
Gameplay / cognition / AI
        │
        ├── operational budget state ── capability / collapse / policy
        │
        ├── heuristic evidence ──────── prediction / resonance / novelty
        │
        └── validated conversions only
                    │
                    ▼
        symtropy-physics physical authority
        ├── mechanical reservoirs
        ├── ThermalBody
        ├── typed EnergyPorts
        ├── EnergyTransferLedger
        └── measured reconciliation / numerical residual
```

The important property is not that every subsystem uses the word "energy." The
important property is that every physical joule has one owner, one source, one
destination, one mechanism, and one auditable causal history.
