# EXACT QUALIFIED STEP EXECUTION RECEIPT v0.1

Status: proposed product theorem; exact-head executable qualification required before PASS is claimed.

Issue lineage: PHYS-OBS-07 / #1057. This is a parallel timing successor of PHYS-OBS-05 / #1088, not a child of PHYS-OBS-06 / #1090.

## Purpose

Bind every successful qualified physics step in one deliberately receipted temporal profile to the exact positive finite binary64 `dt` value that was actually passed to the engine.

This theorem proves execution provenance only:

```text
qualified step S completed normally
with exact input dt bits B
        ↓
LocalQualifiedStepExecutionReceipt { S, B }
```

It does not prove that `dt` follows a fixed cadence, and it does not accumulate elapsed time.

## Exact predecessor

Frozen base:

```text
#1086 sealed qualified temporal evidence authority
#1088 opaque qualified stamped endpoint observation
head: 28383cfa1e8d3464f0b59b54bc0f0202413bf24e
```

PHYS-OBS-07 does not modify `LocalEvidencePhysicsAuthorityWorld`. Instead it introduces an exclusive successor profile that owns it.

## Exclusive receipted profile

```text
LocalNamespacePhysicsAuthorityWorld
        ↓ seal
LocalReceiptedEvidencePhysicsAuthorityWorld
        owns
LocalEvidencePhysicsAuthorityWorld
```

The timed profile can only be created from the namespace boundary. There is deliberately no constructor from an already-running `LocalEvidencePhysicsAuthorityWorld`.

Therefore the profile begins at a fresh temporal incarnation with step index zero and no current qualified step.

While the wrapper exists, it exposes no:

```text
&mut LocalEvidencePhysicsAuthorityWorld
into_evidence()
DerefMut
raw world mutation
unreceipted qualified step path
```

The only qualified step methods are the receipted pure and callback paths.

Leaving this profile consumes it back to the namespace boundary, preserving #1086's quarantine result if a step was interrupted. Strong stepping can resume only after a later fresh seal / temporal incarnation.

## Receipt theorem

`LocalQualifiedStepExecutionReceipt` has private fields and no public detached constructor.

It is intentionally non-Clone, non-Copy, and non-Serde-authoritative.

It binds:

```text
LocalQualifiedAuthorityStepStamp
u64 dt_bits
```

where `dt_bits == dt.to_bits()` for the exact `dt` argument passed to the successful inner #1086 step call.

Read-only accessors expose the strong stamp, exact bits, and a reconstructed `f64` convenience value.

## Minting law

For both:

```text
step_authorized(dt)
step_authorized_with_callback(dt, callback)
```

the wrapper delegates to the owned #1086 facade using the exact same local `dt` value.

Only after that call returns a successfully committed strong stamp does it construct and return a receipt using `dt.to_bits()`.

Therefore:

```text
rejected step   -> no receipt
panic/unwind    -> no receipt
normal success  -> exactly one returned receipt value
```

A caught callback/physics panic leaves the owned #1086 facade tainted. Future strong stepping fails and consuming downgrade follows the existing quarantine path.

## Invalid duration law

The underlying #1086 validation remains authoritative:

```text
NaN
+infinity
-infinity
+0.0
-0.0
negative finite values
```

are rejected and mint no receipt.

The first later successful valid step remains strong step index 1.

## Exact binary64 semantics

This theorem stores the exact bits of the executed `f64` input. It introduces no cumulative `f64` clock and no alternate rounded/configured duration value.

Because admitted `dt` is finite and strictly positive, receipt identity has no admitted NaN/signed-zero equivalence ambiguity.

## Observation binding

The wrapper exposes only an immutable:

```text
&evidence_authority()
```

projection so PHYS-OBS-05 can capture endpoint evidence after a receipted step.

A valid post-step endpoint token should have the same strong stamp as the just-returned execution receipt.

For adjacent post-step endpoint samples at indexes `N` and `N+1`, the simulated transition between them is governed by the receipt whose stamp is **`N+1`**.

The receipt for sample `N` must not be counted retroactively as time spent inside; inside presence was not yet established at the beginning of that step.

## Parallel relationship to PHYS-OBS-06

PHYS-OBS-06 / #1090 proves exact adjacent sampled presence without duration.

PHYS-OBS-07 proves exact executed step input without endpoint membership.

Neither theorem should depend on the other. A later convergence theorem should require:

```text
ConsecutiveSampledPresence(N -> N+1)
+
StepExecutionReceipt(stamp = N+1)
        ↓
exact duration-bearing sampled transition
```

PHYS-OBS-08 / #1058 may further require that the receipt's exact bits equal a precommitted fixed cadence representation.

## Required executable corpus

At minimum:

- all invalid dt forms mint no receipt and do not consume step index 1;
- pure step receipt equals the exact executed `dt.to_bits()`;
- callback step receipt uses the same exact-bit rule;
- successive distinct positive dt values produce distinct exact receipt bits;
- receipt stamp exactly equals the current qualified step stamp;
- PHYS-OBS-05 capture through immutable `evidence_authority()` has exactly the same stamp as the receipt;
- caught callback panic returns no receipt, exposes no current stamp, rejects future stepping, and downgrades to quarantine;
- clean downgrade/reseal mints a new temporal incarnation;
- source audit proves no public receipt fields/constructor/Serde/Clone/Copy;
- source audit proves no mutable inner projection, no `into_evidence`, and no alternative unreceipted step method;
- source audit proves receipt construction occurs only after the inner successful step call;
- no cumulative clock state is introduced.

## Successor path

```text
PHYS-OBS-06 adjacent sampled presence    PHYS-OBS-07 exact execution receipt
                 \                       /
                  \                     /
                   + PHYS-OBS-08 cadence
                           ↓
              #1067 factual sampled interval
                           ↓
                 #1066 policy evaluation
```

## Non-claims

No fixed-step policy, cumulative elapsed simulation duration, wall-clock time, UTC time, real-time pacing, continuous occupancy, trajectory, dwell, hysteresis, arrival, route compliance, custody, delivery, persistence authenticity, network consensus, or settlement authority is established here.
