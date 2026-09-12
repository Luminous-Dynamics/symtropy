# Thermodynamic Transfer Architecture v0.2

## Status

Design/source contract only. This document does not claim runtime qualification, physical realism beyond the declared simplified models, or full-system conservation PASS.

This tranche is stacked on #793 exact head `564a56cdb138fe77fdac681c5fffe3aa62212813` and coordinates the follow-up work in #804, #809, #810, #818, #819, #791, #777, #757 and the fixed-tick thermodynamic scheduling line.

## Purpose

The thermodynamic stack now has several individually useful pieces:

- a validated finite entity energy reservoir;
- finite-source regeneration;
- bounded actuator authority;
- mechanical rigid-body state;
- contact friction and impact callbacks;
- entity temperature/entropy state;
- a global thermodynamic ledger.

The remaining risk is semantic composition. A system can have locally reasonable counters and still fail physically if the same transfer is named differently at adjacent boundaries, a momentum quantity is treated as energy, useful work is double-charged, heat is minted without a source, or a ledger numerically cancels unrelated mistakes.

The governing rule for v0.2 is therefore:

> Every Joule that crosses a modeled boundary has one explicit source account, one explicit sink account, one cause, one tick/transaction identity, and one exactly-once transfer record.

Residuals are diagnostics. They are never balancing entries.

## Core account model

The minimum energy accounts are:

```text
EnergyAccount
├── Reservoir(BodyHandle)
├── Mechanical(BodyHandle | World)
├── Thermal(BodyHandle)
├── Environment(ThermalZone | World)
└── ExternalSource(SourceId)
```

`Reservoir(body)` is the finite non-thermal usable/metabolic/chemical energy store that authorizes self-propelled work and other agent costs.

`Mechanical(...)` contains energy represented by the rigid-body/mechanical simulation, including kinetic energy and, only where explicitly modeled, potential/elastic energy.

`Thermal(body)` is thermal energy associated with body temperature under the declared thermal model.

`Environment(...)` receives exported heat/losses that remain inside the selected simulation control volume but are no longer stored by an entity.

`ExternalSource(...)` represents energy entering from outside the local modeled control volume, including finite wells, charging infrastructure or other declared sources.

No account may silently alias another account. In particular:

```text
usable reservoir energy != thermal energy
usable reservoir energy != mechanical kinetic energy
impulse != energy
entropy != energy
energy telemetry counter != source/sink account
```

## Transfer identity

Every authoritative physical energy movement should be representable as:

```text
EnergyTransferV1
├── transfer_id
├── tick_id
├── joules
├── source: EnergyAccount
├── sink: EnergyAccount
├── cause: TransferCause
├── subject / participant identities where applicable
└── provenance / model identity
```

Required transfer properties:

- `joules` is finite and strictly positive;
- source and sink are explicit and distinct unless a specifically modeled internal conversion requires a more specialized representation;
- a transfer cannot be committed twice under the same `transfer_id`;
- a transfer cannot be silently rewritten after commit;
- invalid/non-finite terms fail closed;
- callbacks may propose a transfer, but the final committed receipt is the evidence object.

Suggested causes include:

```text
TransferCause
├── Maintenance
├── Regeneration
├── PositiveActuation
├── DissipativeBraking
├── RegenerativeBraking
├── ContactFriction { body_a, body_b }
├── NormalImpact { body_a, body_b }
├── HeatExchange
├── ToolPower
└── ExternalWork
```

The cause enum is descriptive provenance, not authority to change account semantics.

## Reservoir model

For v0.2, `EnergyBudget::available` should be interpreted as a finite non-thermal usable-energy reservoir rather than total internal energy `U`.

The reservoir theorem for one tick is:

```text
opening_reservoir
+ accepted_inbound
- committed_outbound
= closing_reservoir
```

within a named numerical tolerance.

Sub-threshold collapse normalization remains a numerical usability policy. If removing a requested amount would leave only an unusable `<= ENERGY_EPSILON` residue, consuming the entire remainder is valid only if the full actual debit is returned and recorded.

Entropy is not generated merely because usable reservoir energy is withdrawn. Entropy belongs to explicit irreversible conversion / thermal processes.

`available_work()` may remain as a compatibility API during migration, but it must not be described as Helmholtz `F = U - TS` unless the implemented state really is a closed thermodynamic `U,T,S,...` model. A conservative motor-authority bound may combine usable reservoir state with independent thermal/safety derating, but that derating must not destroy Joules in the reservoir balance.

## Positive actuator transfer

Positive actuator work must distinguish mechanical work from source debit.

For actuator efficiency `η`, where `0 < η <= 1`:

```text
source_debit_j = mechanical_work_j / η
waste_heat_j   = source_debit_j - mechanical_work_j
```

Therefore:

```text
source_debit_j
= mechanical_work_j
+ waste_heat_j
```

The corresponding account transfers are conceptually:

```text
Reservoir(body) -> Mechanical(body) : mechanical_work_j
Reservoir(body) -> Thermal(body) or Environment : waste_heat_j
```

These may be represented as one compound transaction containing two child transfers or as two transfers bound to one actuator transaction identity.

The source budget limits required source debit before body mutation:

```text
mechanical_work_max = η * available_source_budget
```

The authoritative body velocity/state must not be mutated first and charged later through a side-effect API that cannot prove the debit succeeded. Prefer reserve/commit or an atomic actuator transaction:

```text
validate request
-> compute requested mechanical work
-> compute required source debit
-> reserve/accept source debit
-> apply body mutation
-> commit mechanical + waste-heat transfers
-> emit receipt
```

If body mutation fails, the reserved source transfer must not remain committed. If source reservation fails, physical mutation must not occur.

An ideal v0 actuator may use `η = 1`, in which case waste heat is exactly zero. An ideal source debit plus an additional arbitrary 10% heat term is forbidden.

## Braking and regeneration

Negative actuator work is not represented as negative positive-work accounting.

Dissipative braking:

```text
Mechanical(body) -> Thermal(body) or Environment
```

Regenerative braking:

```text
Mechanical(body) -> Reservoir(body) : accepted_regeneration
Mechanical(body) -> Thermal/Environment : conversion_loss
```

with:

```text
mechanical_energy_removed
= accepted_regeneration
+ conversion_loss
```

plus any separately measured solver residual.

Regeneration must use actual accepted reservoir transfer, not the offered amount. Rejected excess because the reservoir is full remains with the source or is routed to an explicit loss account according to the model. It must not disappear.

Cumulative energy counters must never be made to represent regeneration by directly subtracting from a consumed-energy lifetime scalar.

## Friction theorem

Friction dissipation must be measured in Joules.

At the current physics boundary, a robust v0.2 oracle is to sample the participating pair's kinetic energy immediately around the friction impulse only:

```text
K0 = K_a_before + K_b_before

apply friction linear + angular impulses

K1 = K_a_after + K_b_after
```

Then classify the finite signed delta:

```text
if K0 > K1:
    friction_dissipation_j = K0 - K1
    solver_injection_j = 0

if K1 > K0:
    friction_dissipation_j = 0
    solver_injection_j = K1 - K0
```

within a named numerical tolerance.

Do not use `abs(friction_impulse) * dimensionless_constant` as Joules.

Do not multiply already measured Joules merely because simulation dimension `D` is larger. Higher dimensionality may alter the simulated dynamics and therefore the measured energy change, but a downstream callback cannot create extra energy from a dimension-count multiplier.

For the current solver, the kinetic-energy oracle is qualified only relative to the current mean/isotropic inertia approximation until anisotropic tensor dynamics are separately implemented and verified.

## Contact-local attribution

Once friction loss is genuinely Joule-valued, #791 should preserve the existing body identities through the callback boundary.

For a contact between A and B:

- unrelated entity C must receive no contact-local heat;
- if both A and B have thermal accounts, the split policy must be explicit and deterministic;
- if only one participant has a thermal account, the handling of the other share must be explicit;
- if neither participant has a thermal account, the mechanical loss may still be valid physics evidence and should be routed to an environment/unattributed-contact sink rather than fabricated entity heat;
- ledger/account recording occurs exactly once;
- normal-impact loss and friction loss remain distinct causes.

A compatibility scalar callback may exist temporarily, but it must not erase participant identities for new contact-local evidence.

## Normal impact theorem

Collision impulse is not itself energy.

A gameplay/biological impact-response cost may be modeled from impulse if the coefficient has explicit units such as J/(N*s), but that term is a reservoir cost caused by impact, not mechanical collision dissipation.

Physical normal-contact dissipation must be Joule-valued from a valid mechanical-energy calculation or measurement.

The model must distinguish at least:

```text
normal_contact_dissipation_j
normal_contact_solver_injection_j
friction_dissipation_j
friction_solver_injection_j
impact_response_cost_j (optional gameplay/physiology term)
```

A perfectly elastic collision may have a large impulse and approximately zero normal-contact dissipation.

## Thermal state model

Under the v0.2 simple constant-heat-capacity model, thermal state is separate from the usable reservoir.

Let:

```text
C > 0
T_ref > 0
T > 0
```

Define thermal energy relative to a reference:

```text
E_thermal = C * (T - T_ref)
```

For positive accepted thermal input `Q`:

```text
T1 = T0 + Q/C
E_thermal1 - E_thermal0 = Q
```

For the same constant-C state model, relative entropy is:

```text
S_rel = C * ln(T/T_ref)
```

so a finite heating step must satisfy:

```text
S1 - S0 = C * ln(T1/T0)
```

rather than the finite-step approximation `Q/T1`.

The same total accepted Q applied as one pulse or partitioned into multiple pulses must end at the same T and relative entropy within numerical tolerance.

Body entropy is a state theorem, not a Joule-balancing term. Total-universe entropy production additionally depends on source/environment boundary conditions and is outside the simple body-state equation unless those reservoirs are explicitly modeled.

## Heat transfer versus dissipation

These concepts must remain distinguishable:

```text
mechanical/electrical dissipation -> local heat
external heat transfer -> body thermal state
body cooling -> environment
thermal telemetry only
```

A compatibility `dissipate_heat()` wrapper may remain during migration, but evidence should identify the originating transfer cause and source account.

Pure heating must not increase usable reservoir authority unless an explicit heat-engine path with a cold sink/source theorem is modeled.

## Control-volume closure

#777 should derive closure from account snapshots and committed transfers rather than from two scalar counters named `energy_in` and `energy_out`.

For a declared control volume over one tick:

```text
opening_stored_energy
+ external_inbound
- external_outbound
= closing_stored_energy
+ residual
```

where `opening_stored_energy` and `closing_stored_energy` are explicit sums of included Joule-valued accounts under the declared scope.

Internal transfers between accounts inside the same control volume cancel by construction but remain present as provenance evidence.

Examples:

- Reservoir -> Mechanical is internal if both accounts are inside the control volume;
- Mechanical -> Thermal is internal if both are inside;
- ExternalSource -> Reservoir is inbound if the source is outside;
- Environment -> outside-world heat export is outbound if the environment account is outside the selected boundary.

The exact boundary must be named in every conservation receipt.

## Residual semantics

A residual is evidence about model/numerical incompleteness:

```text
residual_j
= opening + inbound - outbound - closing
```

It must never be silently inserted as an energy source/sink to make the equation balance.

Track signed residuals where useful and classify known causes, for example:

```text
SolverInjection
IntegratorDrift
UnmodeledPotentialEnergy
NumericalRounding
UnknownResidual
```

A small residual supports a bounded closure claim only under the exact declared account set, model and numerical tolerance. It is not proof that every relevant physical energy mode is modeled.

## Exactly-once finalize

Thermodynamic finalization must be idempotent per tick/transaction identity.

Acceptable policies include:

- second finalize returns the already committed receipt; or
- second finalize returns a typed `AlreadyFinalized` result with no mutation.

Unacceptable behavior:

- counters reset twice;
- transfers committed twice;
- maintenance/work/heat charged twice;
- receipt changes because render cadence called finalize again.

Fixed simulation cadence, not render cadence, owns thermodynamic transaction boundaries.

## Minimum end-to-end theorem

The first full executable campaign should prove this chain for one controlled locomotion scenario:

```text
perceived state
-> typed intent
-> bounded actuator request
-> validated finite source budget
-> committed source debit
-> bounded mechanical work
-> authoritative body state change
-> local physical contact if exercised
-> measured Joule-valued friction/impact loss
-> participant-local thermal conversion
-> same-tick finalize
-> typed transfer receipt set
-> declared control-volume closure
```

The evidence must preserve each non-equivalence:

```text
intent != authority
requested work != accepted source debit
source debit != mechanical work when η < 1
impulse != energy
mechanical loss != gameplay impact cost
heat != usable reservoir energy
entropy != energy
ledger cancellation != conservation proof
simulation theorem != real-world validation
```

## Ordered implementation plan

Recommended order:

1. #804 — freeze usable-reservoir vs thermal-state ontology.
2. #819 — make thermal energy/entropy transitions state-consistent.
3. #818 — make positive actuator source/mechanical/waste-heat transfer explicit and transactional.
4. #809 — replace impulse-scaled friction pseudo-energy with measured Joules and separate solver injection.
5. #810 — separate normal-impact impulse, gameplay impact cost and measured mechanical loss.
6. #791 — propagate contact participant identities and localize the now-Joule-valued friction/impact thermal sinks.
7. #777 — implement typed account/transfer receipts and control-volume closure over the corrected quantities.
8. Rebase/fold #757 onto the resulting source/actuator transaction contract.
9. Add NPC `MoveTarget` adapter and player adapter through the same motor primitive.
10. Finish fixed-cadence FEP + thermodynamic transaction scheduling and run the end-to-end campaign.

The ordering may be split into smaller PRs, but downstream work must not claim a theorem whose upstream quantity still has ambiguous units or ownership.

## Qualification matrix

A future executable qualification campaign should include at least:

### Reservoir

- construction/state validation;
- source-backed accepted regeneration;
- collapse-boundary residue accounting;
- no entropy creation from plain reservoir withdrawal.

### Actuator

- `η=1` exact ideal transfer;
- `η<1` source/mechanical/waste split;
- source exhaustion before body mutation;
- reversal with separately accounted braking + propulsion;
- zero authority preserves external momentum.

### Friction

- zero friction -> zero dissipation;
- off-center friction includes angular KE;
- measured ΔK oracle;
- solver injection surfaced separately;
- no dimension-count energy multiplier.

### Impact

- elastic collision -> near-zero physical dissipation despite nonzero impulse;
- inelastic collision -> positive measured loss;
- gameplay impact cost, if retained, does not masquerade as physical loss.

### Thermal

- one heat pulse vs partitioned pulses -> same final thermal state;
- `ΔE_thermal == Q`;
- `ΔS == C ln(T1/T0)` under constant-C model;
- thermal energy alone cannot self-propel the body.

### Ledger / control volume

- exactly-once transfer IDs;
- internal transfer cancellation by account algebra;
- explicit external source/sink terms;
- signed residual without hidden balancing entry;
- double finalize cannot mutate state;
- frozen fixed-tick replay invariant to render cadence.

## Claim boundary

A successful v0.2 campaign may establish that the declared simplified Symtropy energy accounts and transfers close numerically under the tested software model and exact lineage.

It does not by itself establish:

- biologically calibrated human metabolism;
- exact real-material heat capacities;
- anisotropic rigid-body energetics beyond the solver model;
- continuum thermodynamics;
- hardware energy use;
- real-world safety/certification;
- scientific validation of consciousness-energy coupling.

Those require separate models and evidence.