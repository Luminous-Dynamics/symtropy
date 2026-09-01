# Stellar Q5 Qualification Profile v0.1

**Status:** design freeze for future large-scale qualification  
**Depends on:** Q2 world continuation, #97/#98/#102/#103, lower-layer time/frame/transit contracts

## 1. Purpose

Q5 asks whether the same causal and continuation guarantees that work for a local living world remain credible across planetary/interplanetary/stellar scales, long simulation intervals, and hierarchical representation changes.

Q5 is not "run a huge benchmark." It is a structured evidence profile.

## 2. Entry conditions

A candidate does not enter Stellar Q5 until:

- its exact code tree has Q2 continuation evidence;
- all enabled stellar profile contracts have portable canonical identities;
- body/local domain authorities expose the state/continuation identities needed by the candidate profile;
- interbody transit ownership and reference-frame semantics are explicit;
- no required feature relies on wall-clock time or host packet ordering.

Q3/Q4 evidence may be referenced where the same native/local domains participate.

## 3. Qualification dimensions

Q5 evidence is partitioned into:

1. **causal correctness**;
2. **continuation/replay correctness**;
3. **representation/LOD equivalence**;
4. **time/frame correctness**;
5. **epistemic correctness**;
6. **conservation/authority handoff correctness**;
7. **scale/performance boundedness**;
8. **long-horizon numerical/semantic stability**.

A performance PASS cannot compensate for a causal FAIL.

## 4. Canonical Q5 fixture family

Stable fixture IDs should be introduced, for example:

```text
Q5-FRAME-001
Q5-SIGNAL-001
Q5-KNOWLEDGE-001
Q5-PHYS-TRANSIT-001
Q5-HANDOFF-001
Q5-CATCHUP-001
Q5-LOD-001
Q5-MULTIBODY-001
Q5-LONGTIME-001
Q5-SCALE-001
```

Each fixture records exact model/config/profile identities and canonical checkpoints.

## 5. Reference-frame qualification

### Q5-FRAME-001 — transform consistency

For a selected system state/time range:

- evaluate legal frame paths between body-fixed and system frames;
- require composition agreement under declared exact/tolerance policy;
- require inverse/round-trip agreement;
- vary only ephemeris/authority state and prove transform evidence changes;
- checkpoint/restore frame context and reproduce the same transform evidence.

### Q5-FRAME-002 — long-horizon orbital/frame drift

Compare qualified coarse/analytic frame/orbit evolution to an accepted reference integration across progressively longer horizons.

Record bounded error metrics rather than asserting bitwise equality where numerical integration legitimately differs.

The error budget itself is versioned policy identity.

## 6. Finite information propagation

### Q5-SIGNAL-001 — two-body finite propagation

- emit a canonical signal;
- prove no delivery before canonical receive time;
- partition catch-up work differently and require same receive event;
- suspend/resume mid-flight;
- duplicate/reorder host transport;
- require one canonical delivery.

### Q5-SIGNAL-002 — relay graph

Route through deterministic relays with changing geometry/availability.

Require path/provenance/receive-time identity to match the declared routing policy.

### Q5-SIGNAL-003 — large pending set

Stress many in-flight transits while proving canonical ordering does not depend on container iteration or host scheduling.

## 7. Epistemic qualification

### Q5-KNOWLEDGE-001 — delayed remote truth

Construct two worlds with:

- identical local received knowledge;
- different hidden current remote state.

Require the local AI/planner to make the same decision under the finite-information profile.

Then deliver the later observation and require the next decision to be allowed to differ.

### Q5-KNOWLEDGE-002 — prediction separation

Require deterministic predictions/inferences to remain distinguishable from authority-backed observations throughout persistence, UI query, and AI planning.

### Q5-KNOWLEDGE-003 — save/reload memory

Gameplay-significant agent knowledge/memory must survive exact continuation when it influences future decisions.

## 8. Physical interbody transit

### Q5-PHYS-TRANSIT-001 — conserved two-body voyage

Checkpoints:

```text
local departure
interbody authority acquisition
coarse propagation
maneuver refinement
mid-flight suspend/resume
capture approach
local arrival handoff
```

Require exactly-once ownership and declared conservation quantities at every transfer.

### Q5-PHYS-TRANSIT-002 — failure branch

Change one canonical maneuver/forcing input and require a physically distinct outcome such as missed capture, without destination snapping or object loss.

### Q5-PHYS-TRANSIT-003 — destruction/debris

Destroy a vehicle in transit and prove conserved/promoted debris/resource consequences remain authoritative after source/destination unload.

## 9. LOD / representation qualification

### Q5-LOD-001 — transit hydration

Compare:

```text
coarse orbit -> refine -> local encounter -> coarsen
```

against the declared reference path.

Require:

- exact conserved quantities where promised;
- bounded trajectory/state error where approximation is declared;
- no stable-identity loss for promoted/persistent objects;
- representation-transfer receipts at every authority-relevant boundary.

### Q5-LOD-002 — body inactivity

Leave a body inactive while system/transit state evolves, then hydrate/catch up.

Require its result to match the selected inactive-time policy rather than the host's frame rate or absence duration in wall time.

## 10. Multibody/system qualification

### Q5-MULTIBODY-001 — hierarchical root

Create a system with multiple body manifests and system-level transit state.

Mutate one region/body and require digest changes only along its Merkle ancestry plus any causally affected system/global state.

Unchanged bodies remain content-address reusable.

### Q5-MULTIBODY-002 — partial verification

Verify one body/region subtree against a trusted system root without decoding every other body's mutable state.

### Q5-MULTIBODY-003 — independent residency

Unload and reload bodies in different orders while preserving the same semantic continuation identities under equal inputs/policies.

## 11. Long-horizon qualification

### Q5-LONGTIME-001 — years/decades

Advance representative planetary/ecological/logistical/transit systems through long horizons using declared multirate policies.

Compare canonical checkpoints and declared bounded-error metrics.

### Q5-LONGTIME-002 — centuries or longer

Only domains scientifically/semantically designed for these horizons participate.

Do not infer that an hourly local model is valid for centuries merely because it can be looped many times.

Use qualified aggregate/event-driven/analytical transitions where appropriate.

### Q5-LONGTIME-003 — interruption partition invariance

Repeatedly interrupt, suspend, migrate, and resume the long-horizon simulation. Require equal final continuation identity or declared equivalence metrics under equal policies/inputs.

## 12. Performance budgets

Q5 records explicit budgets, not vague "fast enough" claims.

Candidate measurements include:

- memory per inactive body;
- manifest bytes per body/region;
- pending-transit bytes per item;
- catch-up CPU per simulated day/year;
- frame-transform queries per second;
- hydration/coarsening latency;
- number of resident high-fidelity transit objects;
- system-root recomputation cost;
- snapshot/dedup ratio.

Benchmark classes should scale at least across representative counts such as:

```text
1 / 10 / 100 bodies or major scopes
10 / 1,000 / 100,000 pending causal transits
1 / 100 / 10,000 physical transit objects
```

Exact targets are hardware/profile specific and should be stored as policy/evidence, not embedded into semantic state digests.

## 13. Determinism and approximation

Q5 distinguishes three claim levels:

### Exact

Bit/canonical identity must match.

Examples:

- IDs;
- manifest digests;
- discrete authority epochs;
- pending transit sets;
- integer-conserved inventories where defined.

### DeterministicApproximate

Same inputs/model produce the same approximate result, with declared error bound against a reference.

### BoundedEquivalent

Different legal partitions/representations may produce non-bit-identical results but must remain within a frozen semantic/conservation error contract.

The claim level is explicit per fixture.

## 14. Relativity extension boundary

v0.1 Q5 does not require full relativity.

If velocities/distances/gameplay later require it, add a new profile that explicitly binds:

- proper-time/time-coordinate semantics;
- relativistic frame transforms;
- signal null-cone propagation;
- high-velocity trajectory integration;
- gravitational/time-dilation model identity.

Do not silently reinterpret classical v0.1 identities as relativistic ones.

## 15. Evidence capsule

A Stellar Q5 capsule extends Q2 evidence with:

```text
PROFILE.txt
SYSTEM_FIXTURE.json-or-equivalent
FRAME_VECTORS/
TRANSIT_CHECKPOINTS/
KNOWLEDGE_CHECKPOINTS/
CONSERVATION_REPORTS/
ERROR_BUDGETS/
PERFORMANCE/
LONG_HORIZON/
LOGS/
MANIFEST.sha256
```

The fixture serializer is not canonical identity; canonical digests remain explicitly encoded by their contracts.

## 16. PASS semantics

A Q5 PASS is profile-specific.

Examples:

- `Q5/InstantInformation/PhysicalInterbody`;
- `Q5/FiniteInformation/PhysicalInterbody`;
- future `Q5/StellarPhysical/Relativistic`.

Never report a generic "stellar qualified" result that hides which causal/physics profile was tested.

## 17. Design outcome

Q5 should demonstrate that Symtropy can scale from a living local world to a selectively resident stellar system **without weakening causality, authority ownership, persistence, epistemic honesty, or reproducibility merely to achieve scale**.