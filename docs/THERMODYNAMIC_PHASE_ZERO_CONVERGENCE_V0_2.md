# Thermodynamic Phase-Zero Convergence v0.2

## Status

Design/source convergence contract only. This document amends the generic account vocabulary in `THERMODYNAMIC_TRANSFER_ARCHITECTURE_V0_2.md` by binding it to the physical-energy authorities that already exist on the Phase-Zero thermodynamics lineage.

It does not claim that the divergent FEP/#793 and Phase-Zero branches are already composed or executable-green.

## Discovery

The newer FEP reservoir line and the earlier Phase-Zero energy-authority line diverged after common commit `3f8e470647b2befe40b4fcbbd90e6efb8c21fc93`.

Comparing Phase-Zero operational-hardening head #46 (`dd7ac25c552912ead8f181552993fa7aab005919`) to #793 (`564a56cdb138fe77fdac681c5fffe3aa62212813`) reports a diverged history: the #793 side is 21 commits ahead while eight Phase-Zero commits are not present on that lineage.

Therefore the v0.2 program is a **semantic convergence**, not a wholesale preference for either branch.

## Existing authority is canonical

The abstract v0.2 terms must map onto the existing core implementation rather than create parallel physical-energy infrastructure.

### Physical energy

Canonical implementation lineage:

- #45 — Energy Authority Contract;
- core `EnergyOwner`;
- core `EnergyForm`;
- core `EnergyPort`;
- core `EnergyTransferKind`;
- core `EnergyTransferLedger`;
- #10 `EnergyStateSnapshot` reservoir reconciliation;
- #50 checked aggregate reductions and strict boundary closure.

A future v0.2 `EnergyTransferV1` name is therefore **conceptual vocabulary only** unless it is an additive receipt around these existing objects. It must not become a third physical ledger.

### Thermal energy

Canonical implementation lineage:

- #40 — one thermal authority;
- `RigidBody::thermal` / `ThermalBody`;
- `ThermalMaterial`;
- `ThermalState`;
- core physical thermal-energy measurement and reconciliation.

The v0.2 conceptual account `Thermal(body)` means the core thermal reservoir. It does **not** authorize an independent consciousness-domain temperature/entropy owner.

### Operational energy

`EnergyBudget` / launcher fallback budget remain operational/capability state unless a separately validated physical calibration says otherwise.

The newer #793 state-invariant work is valuable because operational authority still requires trustworthy persistent state, bounded transfers, exact accepted-credit telemetry and fail-closed behavior.

But:

```text
EnergyBudget.available
!= automatically total internal energy
!= automatically chemical/electrical physical state
!= automatically physical source for ThermalBody
```

A calibrated conversion may be added later with explicit source reservoir, units, efficiency and reconciliation.

### Heuristic evidence

Impulse, Phi, harmony/resonance, prediction error, sanctuary attenuation and similar signals retain their actual units/meaning. They may affect policy or operational costs without entering the physical Joule ledger.

## Existing Phase-Zero repairs to port

The converged FEP descendant must preserve these already-reviewed semantic repairs.

### #43 — epistemic offloading

Resonance reduces bounded maintenance demand **before debit**. It is not regeneration and cannot become a source merely because multiple partners exist.

### #44 — collapse recovery

Ambient support may refill a live operational reservoir but may not resurrect collapse. Recovery requires an explicit allowed source such as a finite energy well.

### #46 — invalid evidence

Invalid/non-finite Phi, harmony, configuration or derived arithmetic cannot create free maintenance, favorable offload or motor authority.

### #51 — physical-claim cleanup

- collision impulse does not become heat;
- sanctuary absorbed impulse does not become heat;
- operational collision cost does not become physical heat;
- uncertified solver dissipation does not mutate physical temperature;
- positive work does not mint arbitrary 10% legacy heat;
- regenerative work does not directly mutate lifetime totals;
- accepted boundary inflow/outflow uses explicit direction rather than signed dissipation.

These semantics are prerequisites for positive physical coupling, not temporary documentation choices.

## Existing core physical proof to reuse

PR #9 already supplies the first measured mechanical-to-thermal reference primitive.

For a centered dynamic/dynamic friction impulse it:

1. measures pair modeled kinetic energy before and after;
2. admits only measured positive loss as physical dissipation;
3. partitions heat explicitly;
4. records typed physical transfers;
5. mutates `ThermalBody`;
6. reconciles all staged kinetic and thermal reservoirs;
7. rolls back on failure.

That proof intentionally rejects off-center impulses, static/kinematic partners and other cases whose omitted/uncertain reservoirs would overstate the claim.

#809 should integrate this theorem into the world contact solver rather than reimplement it through the legacy scalar callback.

## Reconciliation is stronger than balance

PR #10 and #50 already establish the correct first-law evidence shape.

A valid interval needs more than total numeric closure:

- stable reservoir identity;
- explicit presence/absence rather than `None == 0`;
- complete represented internal ports for the claim;
- measured per-reservoir state deltas;
- ledger net deltas;
- bounded residuals;
- checked external boundary flow;
- overflow-safe deterministic reductions.

Therefore #777 should consume these primitives and produce an integration/campaign theorem. It should not reconstruct conservation from the legacy `energy_in` and `energy_out` counters.

## Legacy consciousness ledger

The consciousness-domain `ThermodynamicLedger` is compatibility/research telemetry.

Its historical `conservation_error` is not physical first-law evidence because its channels have mixed operational debit, heuristic dissipation, incomplete regeneration, direct cumulative mutation and omitted state reservoirs.

Potential retained research metrics must name their numerator precisely, for example:

- operational-budget units per DeltaPhi;
- measured external-work joules per DeltaPhi;
- measured thermal dissipation joules per DeltaPhi.

Those numerators may not be silently mixed.

## Thermal migration correction

The finite-step constant-heat-capacity identity discovered during the #793 review remains a useful negative control:

```text
DeltaS = C * ln(T1/T0)
```

rather than legacy `Q/T_final` accumulation for a finite heating step.

But the correct response is **not** to deepen `EnergyBudget` into a better second physical thermodynamic state.

#819 now owns migration away from that duplicate state toward core `ThermalBody`. The exact production entropy theorem must come from the core thermal model selected by that authority.

## Actuator correction

#818 distinguishes:

```text
operational debit
measured mechanical work
physical source debit (only in calibrated profile)
physical loss / heat (only in calibrated profile)
```

The safe compatibility profile may use operational reserve solely to gate actuator authority without claiming physical source conversion.

A physically calibrated actuator profile must explicitly identify a physical source reservoir and reconcile:

```text
physical source debit
= mechanical work
+ modeled physical loss
```

within the selected tolerance.

No physical conversion is inferred merely because the operational budget is numerically expressed in joule-like units.

## Fixed-tick transaction identity

#783 provides the useful temporal split:

```text
begin
-> consequential simulation
-> physics
-> finalize
```

#824 strengthens that into an intrinsic transaction theorem. Begin/finalize must carry a fixed-tick identity so duplicate begin/finalize cannot reapply maintenance, reset counters, duplicate transfers or double-sample HUD evidence.

The core physical ledger interval and operational transaction should bind to the same fixed simulation interval when they are compared, without becoming the same authority channel.

## Convergence owner

#829 owns composition of the divergent branches.

The preferred sequence is:

1. port Phase-Zero operational semantics (#43/#44/#46) onto the selected #793 descendant;
2. port #51 physical-claim cleanup;
3. bind embodied identity to existing core thermal/energy authority (#40/#819);
4. integrate measured friction using #9 as reference (#809/#791);
5. add measured normal-impact successor (#810) only where modeled reservoirs support the claim;
6. compose #757 with #818 transactional actuator semantics;
7. enforce #824 fixed-tick transaction identity;
8. run #777 as strict core-ledger/reservoir reconciliation campaign;
9. only then promote the complete intent -> paid action -> mechanics -> heat -> receipt theorem.

## Required anti-regression matrix

A composed descendant must preserve tests proving:

- full-reservoir credit does not create phantom regeneration;
- finite well source debit equals accepted destination gain;
- sub-epsilon collapse removes and records the exact remaining operational reserve;
- invalid operational state fails authority closed;
- offloading reduces cost before debit;
- invalid Phi/harmony cannot unlock favorable cost;
- ambient cannot resurrect collapse;
- explicit finite well can recover collapse;
- impulse alone cannot create physical heat;
- legacy callback dissipation cannot become authoritative physical heat;
- arbitrary dimension scaling cannot multiply measured Joules;
- actuator authority cannot leave an unpaid body mutation;
- measured physical transfers use core typed ledger/reservoir reconciliation;
- unrelated contact participants do not receive heat;
- duplicate begin/finalize cannot duplicate state or evidence.

## Claim boundary

The v0.2 architecture may eventually establish a strong theorem about the **declared modeled reservoirs and exact software lineage**.

It must not overclaim:

- operational reserve as biological/metabolic energy without calibration;
- heuristic impulse/resonance signals as Joules;
- legacy consciousness temperature/entropy as physical authority;
- off-center contact heat before angular dynamics are qualified;
- static-boundary closed-pair conservation without the boundary reservoir;
- a balanced journal as sufficient evidence without measured endpoint reconciliation;
- exact software closure as real-world validation.