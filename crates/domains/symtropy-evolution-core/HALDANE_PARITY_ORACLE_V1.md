# Haldane Odd-Parity Threshold Oracle V1

Status: implemented/source-reviewed only. Not executable-qualified.

## Purpose

This contract freezes the deterministic numerical authority used by the future marker-marginal `PoissonCrossoversNoInterferenceV1` gamete executor.

It computes only the probability, rounded to integer parts per million, that an adjacent genetic-map interval contains an **odd** number of gametic crossovers under the declared Haldane/no-interference process.

It does not sample parity and does not create a gamete or breakpoint history.

## Mathematical law

For genetic distance `d` Morgans:

`K ~ Poisson(d)`

and

`P(K odd) = (1 - exp(-2d)) / 2`.

For integer C1 micromorgan distance `delta_um`:

`odd_ppm = round(500_000 * (1 - exp(-2 * delta_um / 1_000_000)))`.

The V1 output domain is therefore `0..=500_000 ppm`.

## Canonical arithmetic

The authoritative implementation uses no floating-point arithmetic and no platform math-library exponential.

Constants:

- fixed-point scale: unsigned Q48 (`ONE = 2^48`);
- Taylor term count: exactly 12 non-constant terms;
- range-reduction target: `y <= 1/8` in Q48;
- saturation distance: `7_000_000` micromorgans;
- maximum result: `500_000 ppm`.

For `delta_um == 0`, return `0`.

For `delta_um >= 7_000_000`, return `500_000` before forming `2 * delta_um * ONE`. This makes the public `u64` input domain overflow-safe and is analytically sound at one-ppm output resolution because

`500_000 * exp(-14) < 0.5`.

For smaller distances:

1. Compute `x = 2 * delta_um / 1_000_000` in Q48 using positive integer round-half-up division.
2. Repeatedly halve Q48 `x`, each time with round-half-up, until the reduced value `y <= ONE/8`. Record the number of halvings.
3. Evaluate `exp(-y)` with the alternating Taylor recurrence starting from `term = ONE`, `sum = ONE`.
4. For terms `n = 1..=12`, compute `term = round_half_up(term * y, ONE * n)` and subtract odd terms / add even terms.
5. Undo range reduction by exactly the recorded number of Q48 squarings, each `round_half_up(value * value, ONE)`.
6. Compute `round_half_up(500_000 * (ONE - exp_neg_x), ONE)` and clamp only to the mathematical upper bound `500_000`.

Every rounding location and operation order above is part of V1 semantic authority.

## Why Q48 is adequate for V1

The reduced Taylor interval is at most `1/8`, so the omitted alternating-series remainder after 12 non-constant terms is far below Q48 quantization. The Q48 unit is about `3.55e-15`; the result is finally quantized at one part per million. Source-level independent high-precision checks include values deliberately close to half-ppm rounding boundaries.

This document does not upgrade those source-level checks into executable qualification. The exact Rust head still needs its own build/test/rustfmt/strict-Clippy evidence.

## Golden boundary vectors

Representative high-precision rounded results include:

- `0 um -> 0 ppm`;
- `1 um -> 1 ppm`;
- `1_000 um -> 999 ppm`;
- `100_000 um -> 90_635 ppm`;
- `177_160 um -> 149_175 ppm`;
- `177_161 um -> 149_176 ppm` (true value approximately `149175.500000699 ppm`);
- `1_000_000 um -> 432_332 ppm`;
- `5_809_142 um -> 499_995 ppm`;
- `5_809_143 um -> 499_996 ppm` (true value approximately `499995.500000087 ppm`);
- `6_907_755 um -> 499_999 ppm`;
- `6_907_756 um -> 500_000 ppm`;
- `7_000_000 um -> 500_000 ppm`;
- `u64::MAX -> 500_000 ppm`.

These vectors are part of the V1 regression corpus.

## Authority boundary

A returned threshold means only:

> Under the explicitly selected Haldane/no-interference Poisson process, V1 assigns this rounded probability that the hidden crossover count in the given genetic-map interval is odd.

It does **not** establish:

- that a parity draw has occurred;
- that a crossover occurred;
- the number of crossovers;
- any breakpoint coordinate;
- physical base-pair position;
- interference biology;
- gene conversion;
- ancestry identity;
- selection or fitness.

## Common-random-number boundary

The future C3B2A2 interval opportunity draw must not include interval distance in its stochastic key. Distance changes this deterministic threshold and exact map/profile authority, while the underlying interval opportunity can remain fixed for controlled map-distance sweeps.

## Successor

C3B2A2 should combine this threshold with a separately versioned semantic interval draw keyed by reproduction event, parent role, chromosome identity, and adjacent locus identities. Odd parity toggles the current local source homolog; even parity preserves it.

The resulting evidence must state parity/marker inheritance only and must not fabricate hidden breakpoint coordinates.
