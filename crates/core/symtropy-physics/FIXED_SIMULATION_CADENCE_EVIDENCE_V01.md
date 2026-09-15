# FIXED SIMULATION CADENCE EVIDENCE v0.1

Status: proposed product theorem; exact-head executable qualification required before PASS is claimed.

Issue lineage: PHYS-OBS-08 / #1058, stacked directly on PHYS-OBS-07 / #1092.

## Purpose

Freeze one exact positive finite binary64 simulation cadence **before** a fresh qualified temporal incarnation begins, force every strong step in that incarnation to use exactly those bits, and issue a distinct opaque receipt proving the precommitment.

The v0.1 architecture is:

```text
LocalNamespacePhysicsAuthorityWorld
+
LocalFixedSimulationCadence { exact dt_bits }
        ↓ seal
LocalFixedCadenceEvidencePhysicsAuthorityWorld
        owns
LocalReceiptedEvidencePhysicsAuthorityWorld   (#1092)
        ↓ fixed step only
LocalFixedCadenceStepReceipt
```

The cadence bits are the single source of timing truth. No second independently supplied rational duration is admitted.

## Exact predecessor

Frozen PHYS-OBS-07 source candidate:

```text
#1092
1a9a81bc224be5d5e828cfbbdc5f3d9a30a13d13
```

PHYS-OBS-07 already proves that a normally completed strong step can issue an opaque receipt containing the exact `dt.to_bits()` value passed to the engine.

PHYS-OBS-08 must compose that theorem rather than duplicate the stepping or receipt logic.

## Cadence configuration value

`LocalFixedSimulationCadence` is ordinary immutable configuration and may be copied freely. Its constructor accepts exactly one `f64` and rejects:

```text
NaN
+infinity
-infinity
+0.0
-0.0
negative finite values
```

A valid cadence stores only:

```text
dt_bits: u64
```

where those bits are the exact admitted positive finite binary64 value.

The cadence configuration itself is not evidence that any step executed under it.

## Single-source timing law

V0.1 deliberately does not store both:

```text
caller rational numerator / denominator
and
binary64 execution bits
```

because a non-dyadic policy rational such as exact mathematical `1/60` is not equal to its rounded binary64 execution value.

The canonical timing authority is instead:

```text
precommitted dt_bits
        == exact bits supplied to #1092
        == exact dyadic value mechanically derived from those bits
```

A nominal “60 Hz” binary64 configuration therefore proves the exact dyadic value represented by those bits, not exact mathematical `1/60` unless those happen to coincide.

## Exact dyadic projection

Every positive finite binary64 is exactly dyadic. `ExactDyadicCadence` represents the cadence canonically as:

```text
significand * 2^exponent2
```

with an odd nonzero `significand`.

For normal values, the initial exact representation is:

```text
(2^52 + fraction) * 2^(raw_exponent - 1023 - 52)
```

For subnormals:

```text
fraction * 2^-1074
```

Trailing factors of two are removed from the significand and transferred to `exponent2`. This creates one canonical exact dyadic representation without any floating arithmetic or rational-to-binary64 conversion oracle.

## Fresh-incarnation precommitment

`LocalFixedCadenceEvidencePhysicsAuthorityWorld::seal(namespace, cadence)` consumes the namespace boundary and delegates to #1092's fresh receipted seal.

There is no constructor from:

```text
LocalEvidencePhysicsAuthorityWorld
LocalReceiptedEvidencePhysicsAuthorityWorld
an already-issued execution receipt
```

Therefore the cadence exists before step index 1 of the new temporal incarnation.

Changing cadence or intentionally leaving the profile consumes the wrapper back to the namespace boundary. Any later fixed seal receives a fresh temporal incarnation even when the cadence bits are unchanged.

## No raw-dt strong-step path

The fixed wrapper exposes only:

```text
step_authorized_fixed()
step_authorized_fixed_with_callback(callback)
```

There is no `dt` parameter on either method.

Each method reconstructs the exact configured binary64 value with `f64::from_bits(cadence.dt_bits)` and delegates exactly once to the corresponding #1092 receipted step path.

The fixed wrapper exposes no mutable inner receipted/evidence projection, no `into_receipted`, no `into_evidence`, and no raw generic step method.

Thus a successful strong step cannot silently bypass the precommitted cadence while this wrapper exists.

## Distinct fixed-cadence receipt

A crucial anti-retroactivity rule is that PHYS-OBS-08 must **not** simply return the ordinary PHYS-OBS-07 receipt.

If it did, downstream code could execute an arbitrary generic receipted step first and choose an equal-looking cadence afterward.

Instead, successful fixed stepping returns:

```text
LocalFixedCadenceStepReceipt {
    execution: LocalQualifiedStepExecutionReceipt,
    cadence: LocalFixedSimulationCadence,
}
```

with private fields and no detached constructor.

The type is intentionally non-Clone, non-Copy, and non-Serde-authoritative.

It can be minted only by the fixed wrapper after the delegated #1092 step returns normally.

There is no constructor or `From`/`TryFrom` path from:

```text
generic execution receipt + cadence
```

so cadence provenance cannot be attached retrospectively.

A read-only `execution_receipt()` projection to the weaker #1092 receipt is permitted. Reverse promotion is not.

## Fixed-receipt identity

A fixed receipt exposes:

```text
full LocalQualifiedAuthorityStepStamp
exact dt_bits
precommitted LocalFixedSimulationCadence
exact dyadic projection
```

The execution bits and cadence bits are equal by composition: the wrapper passes `cadence.executed_dt()` directly into #1092, whose theorem stores exactly `dt.to_bits()`.

No second runtime validator is inserted after successful physics execution; doing so could create a post-commit failure path that loses the receipt after mechanics already advanced. The equality is instead frozen structurally and exercised by the qualification corpus.

## Simulation-time unit boundary

The current physics integrator interprets its `dt` argument as simulation seconds. PHYS-OBS-08 freezes that existing simulation-time convention for this evidence profile by passing the cadence value directly into the existing step path without scale conversion.

This is not an SI metrology theorem, wall-clock theorem, UTC theorem, or proof that simulation time advances at real-time pace.

## First-sample anti-overcredit law

A fixed-cadence receipt for sample `N` proves that the sample belongs to a cadence precommitted before its temporal incarnation/step.

It does **not** prove the target was Inside at the beginning of step `N`.

Therefore later PHYS-OBS-09 interval construction must preserve:

```text
first fixed-cadence Inside sample N
+ matching fixed receipt N
    -> cadence-qualified start
    -> 0 credited transitions
    -> 0 credited sampled-presence duration
```

An adjacent relation `N -> N+1` credits the fixed receipt at `N+1` exactly once.

## Required executable corpus

At minimum:

- invalid cadence values reject before a fixed wrapper can exist;
- cadence preserves exact admitted binary64 bits;
- exact dyadic projection is correct and canonical for normal values;
- exact dyadic projection is correct for the minimum positive subnormal;
- a fixed pure step returns a fixed receipt whose execution bits equal the precommitted cadence bits;
- callback-coupled fixed stepping uses the same exact cadence;
- successive fixed steps preserve the same cadence while strong step indexes advance;
- PHYS-OBS-05 endpoint capture after a fixed step has the same strong stamp as the fixed receipt;
- read-only downward projection yields the exact inner #1092 receipt;
- clean exit/reseal under the same cadence produces a fresh temporal incarnation;
- changing cadence requires namespace exit and fresh temporal incarnation;
- source audit proves fixed receipt private fields/no constructor/no Clone/Copy/Serde;
- source audit proves no generic-receipt-plus-cadence promotion path;
- source audit proves fixed wrapper construction begins only from namespace + cadence;
- source audit proves no raw `dt` fixed-wrapper step API and no mutable/consuming escape to the receipted/evidence facade;
- #1092 regression remains green and its semantic source blobs remain unchanged.

## Relationship to PHYS-OBS-09A / #1095

#1095 is an early narrow join of PHYS-OBS-06 and generic PHYS-OBS-07. It is not an ancestor requirement for this fixed-cadence branch.

PHYS-OBS-08 continues independently from #1092 so timing/cadence remains scientifically separable from sampled endpoint presence.

The later full convergence before fixed-cadence PHYS-OBS-09 remains #1094.

## Successor path

```text
PHYS-OBS-06 sampled relation           this PHYS-OBS-08 fixed timing branch
             \                           /
              \                         /
                    #1094 convergence
                           ↓
             PHYS-OBS-09 factual interval
                           ↓
             PHYS-EVID-04 policy evaluation
                           ↓
        sampled dwell / hysteresis / arrival
```

## Non-claims

No cumulative elapsed interval is established by one fixed receipt. No continuous occupancy, no no-exit guarantee, no dwell threshold, no hysteresis, no arrival, no stopped-state condition, no route compliance, no custody, no delivery, no wall-clock/UTC duration, no real-time pacing, no persistence authenticity, no cryptographic provenance, no consensus, and no settlement eligibility are established here.
