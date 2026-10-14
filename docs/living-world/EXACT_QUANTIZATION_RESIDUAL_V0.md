# Exact Quantization Residual V0

## Status

Normative Living World numerical-authority contract. This document constrains future adapters that convert continuous/rational modeled fluxes into exact integer stock/property units. It does not prescribe how an upstream floating model is calibrated into a rational/fixed-point rate.

## Problem

Canonical ecological stock may use exact integer units while physiology/reaction/rate models produce fractional desired flux.

Naively rounding each tick creates systematic matter/authority drift.

Example:

```text
desired uptake = 1/3 exact unit per tick
floor each tick = 0
```

After 3,000 ticks the model intended 1,000 units of cumulative uptake but settled zero.

Rounding to nearest independently each tick can bias in the opposite direction.

## Core rule

Fractional quantization error that affects future exact settlement must have an explicit owner and must persist as state.

For a rational desired flux:

```text
numerator / denominator exact units
```

and residual numerator `r` where `0 <= r < denominator`:

```text
accumulated = r + numerator
grant       = floor(accumulated / denominator)
next_r      = accumulated mod denominator
```

with widened integer arithmetic.

This is a numerical error-diffusion accumulator, not a material stock.

## Invariant

For a fixed rate and quantization context, cumulative settled exact units plus the remaining fractional residual equal the cumulative modeled rational quantity.

Equivalently, cumulative integer settlement differs from the exact rational integral by strictly less than one exact unit.

## Residual is future-bearing state

Residual affects later settlement and therefore cannot be treated as disposable cache once its process is authoritative.

It must be bound conceptually to:

```text
process / reaction identity
+ authority scope
+ quantity/property
+ exact unit
+ quantization scheme version
+ denominator/fixed-point scale
+ residual numerator
```

Two different processes must not accidentally share one residual accumulator unless their reaction contract explicitly defines a joint accumulator.

## Persistence / unload

If an authoritative process can continue after save/reload, region unload/reload, migration, or fidelity collapse, its residual must either:

- persist in a sufficient lower-fidelity process/cohort state; or
- be explicitly transformed under a qualified rebase/closure.

Silently resetting residual to zero changes future matter flow.

Therefore:

```text
continuous run
==
save/reload run with identical residual state
```

for the canonical settlement sequence.

## No duplicate residual ownership

The same logical process accumulator cannot exist simultaneously in active and coarse state.

Promotion/collapse must transfer residual ownership exactly once, just like any other future-bearing process state.

Residual itself is not counted as conserved matter, but duplicating it can cause future matter to be settled twice.

## Transaction semantics

Intent generation may compute a proposed next residual, but authoritative residual state changes only with the canonical tick/process transaction.

Preferred conceptual flow:

```text
snapshot process residual
      |
compute rational desired flux
      |
plan integer request + next residual
      |
arbitration / reaction validation
      |
atomic tick/process commit
      +--> committed process residual
      +--> committed canonical settlement
```

If the enclosing transaction aborts, the committed residual remains unchanged.

Whether residual advances when a valid demand is partially denied by scarcity is a process-policy choice that must be explicit. The residual represents **quantization of modeled desired flux**, not automatically the backlog of unmet ecological demand.

## Unmet demand is not residual

Do not conflate:

- fractional rounding residual;
- resource scarcity debt;
- hunger/resource deficit;
- delayed reaction work.

If a grazer requests 10 mg and receives 4 mg, the missing 6 mg is not a quantization remainder. It may influence physiology/homeostasis, but belongs to a different state variable.

## Scale / denominator changes

A residual numerator has meaning only with its exact scale/denominator and scheme version.

Changing from denominator 1,000 to 1,000,000 cannot simply retain the same integer remainder.

A rebase must either:

- convert residual exactly when the scales are commensurate;
- carry a wider rational representation through migration;
- or use another explicitly qualified conversion with bounded/accounted error.

Silent reinterpretation is forbidden.

## Floating-point boundary

This contract begins **after** an upstream model has chosen a rational/fixed-point desired flux representation.

If an `f32`/`f64` rate is converted into that rational representation, the conversion policy itself must be deterministic/versioned and its approximation error qualified separately.

Do not hide long-term physical authority in a binary floating accumulator whose exact replay semantics are unspecified.

## Extreme values

Intermediate arithmetic must be widened so valid `u64` exact units/scales do not overflow during `residual + numerator` or equivalent cumulative calculations.

Any representation limit is fail-closed before canonical mutation.

## Qualification fixtures

Minimum executable evidence should include:

1. `1/3` unit per tick settles exactly 1,000 units after 3,000 ticks with zero residual;
2. intermediate prefixes remain within strictly less than one unit of the rational integral;
3. continuous execution equals split save/reload execution when residual is restored;
4. resetting residual during reload demonstrably changes the future sequence, proving it is future-bearing state;
5. invalid denominator zero fails closed;
6. residual `>= denominator` is rejected as non-canonical;
7. numerator larger than denominator can settle multiple exact units per tick correctly;
8. valid `u64` boundary inputs use widened arithmetic;
9. scheme/denominator mismatch rejects silent residual reuse;
10. failed enclosing transaction does not advance committed residual;
11. two independent process accumulators do not share residual state;
12. quantization residual is not included in material-stock totals.

## Non-goals

This contract does not define:

- the physiological rate model;
- scarcity backlog semantics;
- a universal denominator for all ecology;
- arbitrary-precision chemistry;
- the conversion from every floating model to fixed/rational rates.

It freezes the rule that **fractional numerical error affecting future exact ecological settlement is explicit, owned, replayable state—not invisible rounding loss**.
