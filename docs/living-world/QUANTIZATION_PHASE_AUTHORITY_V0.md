# Quantization Phase Authority V0

## Status

Normative Living World authority contract. This document refines `EXACT_QUANTIZATION_RESIDUAL_V0` by distinguishing *which process phase* a residual belongs to. It does not change PR #220's arithmetic oracle.

## Problem

A generic fractional-to-integer accumulator is not sufficient once a canonical ecological tick contains multiple semantic stages.

Consider:

```text
modeled desired uptake
    -> exact integer demand
    -> scarcity arbitration
    -> exact allocated reactant
    -> reaction/yield model
    -> exact integer products
```

There may be fractional quantities at both conversions:

1. continuous/rational **desired demand -> exact requested units**;
2. allocated reactant × stoichiometry/yield -> exact **product units**.

Those residuals represent different future-bearing histories.

Reusing one accumulator across both phases would mix:

- unrealized desire;
- scarcity allocation;
- actual reacted material;
- product quantization.

That can create or erase exact authority even though each individual accumulator uses mathematically correct error diffusion.

## Core rule

Every quantization residual is bound to a typed **quantization phase** in addition to authority scope, process identity, quantity/unit, scheme version, and scale.

Conceptually:

```text
QuantizationResidualIdentity {
    authority_scope,
    process_or_reaction_id,
    phase,
    input_quantity,
    output_quantity_or_demand_class,
    exact_unit,
    scheme_version,
    scale,
}
```

The exact Rust representation is non-normative. Phase separation is normative.

## V0 phases

At minimum distinguish:

### QI — Intent/request quantization

Converts a deterministic modeled desired flux into exact integer *requested* units before scarcity arbitration.

Example:

```text
root wants 1/3 mg water per tick
    -> QI residual
    -> exact demand sequence 0,0,1,...
```

The resulting integer is a request, not a settlement and not material ownership.

### QR — Reaction/product quantization

Converts an actually allocated/committed reactant amount through a deterministic rational/fixed-point reaction yield into exact product units.

Example:

```text
4 exact units detritus allocated
    -> 3/5 product yield
    -> QR residual
    -> exact product settlement
```

QR is conditioned on the exact reactant allocation that is actually admitted to the reaction transaction.

Future phases may be added only through explicit versioned semantics.

## Scarcity boundary

Scarcity arbitration sits between QI and QR:

```text
modeled desire
   |
   v
QI quantization
   |
   v
exact demand
   |
   v
scarcity arbitration
   |
   v
exact grant
   |
   v
QR reaction/product quantization
```

A scarcity shortfall is **not** placed into QI or QR residual automatically.

If a process requests 10 exact units and receives 4:

- 4 is the current exact grant;
- 6 is unmet demand according to physiology/process policy;
- QI residual remains only the sub-unit remainder from request quantization;
- QR residual is computed from the actual allocated reactant/product relation, not from the ungranted 6.

Whether unmet demand becomes hunger, stress, deferred work, or nothing is a separate typed process rule.

## Commit semantics

### QI commit

QI residual advances when the authoritative process step that generated the modeled demand is committed.

It does not require the full demand to be granted. Otherwise scarcity would feed back into the mathematical representation of *what the process wanted* and could cause deterministic demand drift.

If the entire process step is aborted/retried before canonical commit, QI residual does not advance.

### QR commit

QR residual advances only with the reaction/product settlement that used the corresponding exact allocated reactant.

If reaction validation or product settlement fails, QR residual and all reaction stocks remain unchanged.

## Example

Suppose a root has desired uptake `1/3` exact unit per tick.

QI produces:

```text
0, 0, 1, 0, 0, 1, ...
```

On the third tick it requests 1 unit.

If scarcity grants 0:

- QI has still represented the third tick's desired uptake correctly;
- the missing 1 unit is scarcity shortfall/process state;
- QR does not run because no reactant was allocated.

If scarcity grants 1 and downstream conversion has a `2/3` exact product yield:

- QR receives the actual 1-unit reactant;
- QR may settle 0 product with a `2/3` product residual;
- that QR history must not alter the next QI demand.

## Identity / persistence

Two residuals with identical numerator, denominator, quantity, and process ID are still different authority state if their phases differ.

Save/reload, collapse/refinement, migration and replay must preserve each accumulator independently.

A product must never deserialize a QI residual as QR merely because their numeric representation is compatible.

## Rate changes

A model/rate/scale change requires explicit phase-specific migration or rebase.

Do not silently rescale an existing residual if:

- denominator changes;
- exact unit changes;
- phase changes;
- process/reaction identity changes;
- input/output quantity semantics change;
- quantization scheme version changes.

## Qualification fixtures

Minimum future evidence should include:

1. QI demand sequence is independent of scarcity grant sequence;
2. scarcity shortfall does not enter QI residual;
3. QR does not advance when zero reactant is allocated;
4. failed reaction leaves QR residual unchanged;
5. QI and QR with identical numeric states remain distinct owners;
6. save/reload preserves both independent future sequences;
7. active/coarse collapse transfers both without duplication when both exist;
8. phase mismatch fails closed during restore/migration;
9. replay of the same modeled desire + scarcity + reaction history reproduces exact demands, grants, products, and residuals;
10. unmet scarcity demand cannot be mistaken for fractional quantization remainder.

## Relationship to PRs #211 and #220

PR #220 should remain a generic arithmetic oracle: it proves residual error-diffusion semantics after a phase-specific rational input has been selected.

PR #211 should remain exact scarcity arbitration over integer demands.

The product layer composes them in the correct order and assigns residual identity/ownership by phase.

## Non-goals

This contract does not define:

- physiology debt/hunger semantics;
- scarcity priority policy;
- reaction stoichiometry;
- calibration from arbitrary floating-point models;
- persistence bytes;
- CRK event identity.

It freezes the rule that **fractional demand history and fractional reaction/product history are different canonical process state and must never share or exchange residual authority merely because the arithmetic representation is the same**.
