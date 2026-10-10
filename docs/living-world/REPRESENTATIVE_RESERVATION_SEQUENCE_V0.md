# Living World Representative Reservation Sequence v0

Status: companion selection contract. This document refines earlier wording that named largest-remainder apportionment as the representative reservation mechanism.

## Why refine the earlier rule

Largest-remainder/Hamilton apportionment is deterministic and close to ideal proportions for one fixed reservation size, but allocations for size `k` and `k+1` are not guaranteed to be nested. A larger active budget can theoretically reduce the selected count of one bin while increasing another.

For Living World activation/refinement this is undesirable because changing an active budget should preferably add/remove the smallest possible amount of ecological authority instead of reshuffling already-active composition.

The normative requirement is therefore **behavioral**, not allegiance to one apportionment formula.

## Required properties

A representative reservation selector should provide:

1. **exact requested count** — selecting `k` returns exactly `k` members when `k <= N`;
2. **capacity safety** — no bin/stratum contributes more than its canonical count;
3. **determinism** — identical authoritative inputs produce identical allocation;
4. **stable tie-breaking** — no hash-map iteration or renderer order influences selection;
5. **representativeness** — each prefix stays close to ideal proportional allocation;
6. **prefix stability / house monotonicity** — the reservation for `k` is a prefix/subset of the reservation for `k+1` under the same source generation and selection policy;
7. **full recovery** — the first `N` selections contain each bin exactly its canonical count;
8. **bounded integer arithmetic** — no floating-point authority decisions;
9. **selection-policy separation** — representative and targeted cohort selection remain distinct APIs/policies.

Largest-remainder may remain useful for one-shot diagnostics, but it is no longer normative for active reservation when a prefix-stable sequence can satisfy the stronger properties.

## Preferred v0 candidate: largest-deficit sequence

For source bins with counts `c_i`, total count `N`, and already-selected counts `a_i(t)` after `t` selections, define the next-step proportional deficit numerator conceptually as:

```text
D_i(t+1) = (t + 1) * c_i - a_i(t) * N
```

At each step choose an unsaturated bin with the greatest deficit, using canonical key order (or a separately qualified deterministic policy salt) only to break exact ties.

Then increment that bin's selected count.

Conceptually:

```text
for t in 0..k:
    choose i maximizing ((t + 1) * c_i - selected_i * N)
    subject to selected_i < c_i
    selected_i += 1
```

This creates one deterministic representative **sequence**. Reservation size `k` is simply its first `k` selections, so increasing the budget cannot revoke earlier selections.

## Arithmetic comparison without overflow-prone signed conversion

`u64 * u64` products fit in `u128`, but a signed deficit can exceed `i128` range in the most extreme theoretical case.

An implementation need not materialize the signed deficit directly.

To compare bins `i` and `j`:

```text
A_i - B_i > A_j - B_j
```

where:

```text
A_i = (t + 1) * c_i
B_i = selected_i * N
```

is equivalent to:

```text
A_i + B_j > A_j + B_i
```

Each product fits `u128`. The addition can require one carry bit beyond `u128`, so exact comparison can represent each sum as `(carry, low_u128)` using `overflowing_add`, then compare carry first and low word second.

This keeps the authority decision integer-exact without adding arbitrary-precision dependencies to the low-level crate.

## Proportional-error gate

The exact discrepancy bound for the selected v0 algorithm must be qualified rather than assumed.

At minimum the observatory should compute, for every prefix `k` and bin `i`:

```text
error_i(k) = selected_i(k) - k * c_i / N
```

using exact rational/integer comparison.

The implementation should establish and freeze an acceptable maximum absolute discrepancy for the chosen algorithm across exhaustive small fixtures plus adversarial/random large distributions.

A target of at most one organism of per-bin proportional error is desirable for the largest-deficit candidate and should be treated as a qualification claim only after proven/tested over the intended domain.

## Targeted selection remains different

Representative reservation answers:

> "Give me a small active subset that preserves coarse composition as fairly as possible."

Targeted reservation answers a canonical ecological query such as:

> "Activate organisms occupying these cells / this interaction region / this named cohort."

Targeted selection may be intentionally non-representative. It must conserve exact source quantities and state its canonical criteria, but it should not be distorted merely to satisfy representative proportions.

## Refinement hysteresis benefit

Prefix stability gives a useful compute/authority property:

```text
active budget 32 -> 40
```

can add eight newly selected contributions while preserving the original 32 selections under the same source generation.

Likewise reducing the budget can remove a suffix rather than recomputing the entire cohort.

This reduces:

- active-member churn;
- visible identity popping;
- unnecessary realization/collapse transactions;
- network/federation authority movement;
- replay complexity.

## Interaction with prospective projection

A Level-P prospective projection and a Level-A representative reservation are distinct operations, but they may share compatible deterministic selection primitives.

If a displayed prospective candidate is realized, candidate-preserving realization has priority over recomputing a generic representative prefix, provided the candidate can be reserved consistently from current canonical source state.

The remaining active budget can then be filled by the representative sequence over the residual source.

## Stratum-aware operation

Once sparse strata are canonical, the representative sequence should operate over canonical strata (count and eventually extensive quantities) rather than separately over independent marginals.

This prevents representative selection from destroying joint correlations that coarse authority explicitly retained.

Within a selected stratum, unresolved individual microstate may be chosen by the qualified refinement/projection derivation rule.

## Qualification requirements

Evidence must establish at least:

1. exact `k` selections for every `0 <= k <= N`;
2. no selected bin count exceeds source capacity;
3. selection for `k` is a prefix/subset of selection for `k+1` under fixed source generation;
4. full prefix `N` recovers exact source bin counts;
5. identical inputs are deterministic across runs;
6. canonical tie-breaking is stable;
7. arithmetic comparison is exact at `u64` boundary/adversarial values;
8. measured proportional discrepancy stays within the frozen qualified bound;
9. zero/single-bin/full-selection edge cases pass;
10. representative selection never substitutes for an explicitly targeted ecological reservation policy.

## Design principle

**An active-budget change should reveal more or less of the same ecological population, not reshuffle which coarse categories supposedly existed merely because the compute budget moved by one.**
