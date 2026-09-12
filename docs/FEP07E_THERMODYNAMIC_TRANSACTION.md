# FEP-07E — fixed-tick thermodynamic transaction v0.1

## Status

Implemented source contract only. This document is not runtime qualification.

This tranche is stacked on FEP-07C / PR #757 and fixes the temporal accounting boundary exposed while preparing fixed-cadence FEP locomotion.

## Problem

The previous fixed chain registered:

`thermodynamic_enforcement_system`
→ `physics_sync_transforms`

but `thermodynamic_enforcement_system` did both sides of a thermodynamic tick before the physics system ran:

- reset per-tick reservoir counters;
- apply maintenance and regeneration;
- finalize the thermodynamic ledger;
- sample consumed/regenerated counters into the HUD.

The restored authoritative 2D physics step then ran afterward.

Any motor, collision, constraint or other consequence-time energy generated after that early finalization therefore belonged causally to tick N but could be carried into tick N+1's ledger interval. Worse, entity per-tick counters written after HUD sampling could be reset by the next begin phase before telemetry ever observed them.

## Transaction theorem

One fixed simulation interval is now divided into two authority phases:

`thermodynamic begin`
→ consequential simulation / authoritative physics
→ `thermodynamic finalize`
→ presentation/export

The key invariants are:

1. per-tick reservoir counters are reset exactly once, at begin;
2. begin applies pre-consequence maintenance/environmental flows;
3. consequence-time writers may record work, collision costs or dissipation without crossing a ledger boundary;
4. finalize runs after the attempted consequence phase;
5. finalize samples per-tick entity counters before any subsequent reset;
6. `tick_thermodynamics()` executes exactly once per fixed interval;
7. newly exhausted entities are marked Red before the next interval;
8. a refused/invalid physics step still closes the thermodynamic interval rather than leaking the begin phase into the next tick.

## Compatibility wiring

This tranche deliberately avoids rewriting the large shared `plugin.rs` registration.

The existing registered name `thermodynamic_enforcement_system` remains as a compatibility wrapper, but now delegates only to `thermodynamic_begin_system`.

The already-ordered `physics_sync_transforms` boundary then:

1. attempts the authoritative 2D physics step when `GamePhase::Playing` owns integration;
2. gathers the currently modeled Player/Crew physics handles;
3. calls `finalize_thermodynamic_tick`;
4. refuses transform export when the 2D physics step itself was rejected;
5. otherwise exports authoritative body positions.

That gives the current source chain:

`thermodynamic begin`
→ `attempted 2D physics`
→ `thermodynamic finalize`
→ `transform export`

without changing `plugin.rs`.

The future FEP-07 cadence tranche can promote the same boundaries into an explicit schedule:

`thermodynamic begin`
→ `FEP behavior`
→ `authored actions`
→ `NPC actuator`
→ `player actuator`
→ `authoritative physics`
→ `thermodynamic finalize`
→ `transform export`

without redesigning accounting again.

## Pre-step versus post-step collapse

Begin applies ambient/well/offloading regeneration before setting the pre-consequence Red hard stop. This preserves a legitimate tick-start recovery opportunity.

Finalize checks collapse again after consequence-time effects. An entity that exhausts its reservoir because of actuator work, collision or another in-tick debit is therefore Red before the next fixed interval begins.

The actuator itself still retains its direct reservoir gate as defense in depth; schedule correctness is not the only thing preventing a stale cached tier from self-propelling.

## Refused physics step

An invalid fixed timestep causes `step_physics_world` to refuse mutation.

That refusal does **not** abort thermodynamic finalization. Begin may already have applied maintenance/regeneration, so the interval must still be closed and sampled. The transform export is skipped because no valid authoritative physical step occurred.

This preserves accounting evidence while refusing to publish a false post-physics presentation.

## 2D / 3D boundary

Only 2D `GamePhase::Playing` currently grants this adapter authority to integrate `PhysicsWorld`.

`Playing3D` remains kinematic and does not acquire a second physics authority in this tranche. Because the existing fixed thermodynamic wrapper still runs under `in_playing_or_3d`, the 3D thermodynamic interval is finalized at the same fixed boundary even though no 2D world step is performed there.

FEP-07D / #759 remains responsible for giving 3D NPC movement an explicit phase-correct adapter.

## Fixed-time assumption

The current HUD converts 16 fixed ticks to seconds using the existing 64 Hz contract. Bevy 0.19 defaults `Time<Fixed>` to 64 Hz, and generic `Res<Time>` inside fixed schedules refers to that fixed clock.

This tranche does not introduce a custom fixed clock. If Symtropy later overrides `Time<Fixed>`, the HUD rate calculation must be migrated to actual accumulated fixed duration (preferred) or updated with the new declared cadence. The transaction ordering itself does not depend on 64 Hz.

## Deliberate non-goals

This tranche does not solve every thermodynamic modeling question uncovered by the audit.

Specifically out of scope:

- #765 — bounded reservoir mutator/input semantics (independent main-based PR);
- #776 — energy wells currently debit requested transfer before knowing actual accepted reservoir gain;
- #777 — the ledger's control volume / meaning of `conservation_error` requires a separate model theorem;
- #767 — constructor and persisted-state invariants for `EnergyBudget`;
- #768 — calibration of the launcher motor-authority mapping;
- #737 — 2D PlayerInput actuator adapter;
- #759 — 3D authored-AI movement authority;
- moving the FEP behavior/action/movement systems into `FixedUpdate` itself.

Keeping these separate prevents a timing repair from silently changing energy-source conservation or redefining the thermodynamic ledger.

## Source-level regression theorems

The source includes tests intended to establish, once executed:

- consequence-time consumption is sampled by finalize before the next reset;
- an entity exhausted during the consequence phase is marked Red by finalize;
- invalid physics dt is rejected by the world-step primitive;
- only 2D `Playing` owns authoritative world integration at this boundary.

The surrounding actuator tests continue to cover work limiting, braking, stale cached authority and external-momentum preservation.

## Required qualification

Before merge readiness, execute on the exact PR head:

- `cargo fmt --check`;
- default launcher check/test;
- launcher check/test with `--features consciousness-runtime`;
- launcher check/test with `--features fep-ai`;
- Clippy on the same relevant feature surfaces;
- a fixed scenario proving a known consequence-time reservoir debit appears in the same tick's HUD/ledger interval;
- a collision/dissipation scenario proving post-step callback effects are closed in the same interval;
- an invalid-dt scenario proving the thermodynamic interval still finalizes while transform export is refused;
- a 2D velocity→physics→finalize→transform scenario;
- a 3D regression proving no `PhysicsWorld` integration authority was introduced;
- qualification again after #765 semantics are incorporated/merged if this stack is rebased onto them.

The full render-cadence replay theorem remains part of the later FEP-07 schedule migration.

No runtime PASS is claimed by this document.
