# Simulation Timebase Continuation Contract v0.1

**Status:** design freeze; implementation pending  
**Tracks:** #95  
**Consumed by:** World Continuation Manifest v0.1, inactive-world evolution, Q2 evidence

## 1. Purpose

Symtropy already has two useful time representations:

- `symtropy-game-state::SimulationClock` — deterministic fixed-step tick time;
- `symtropy-sim-contracts::SimInstant` — wide-range absolute simulation coordinate.

They solve different problems. They must not become two ambiguous meanings of “world time”.

This contract defines the bridge.

## 2. Core invariant

For every scope that advances a fixed-step clock:

> `(timebase identity, tick)` maps to exactly one `SimInstant`, and save/resume/migration may not reinterpret that mapping implicitly.

The world continuation root binds the timebase identity required to understand local ticks.

## 3. Timebase record

A canonical fixed timebase binds at minimum:

```text
schema_version
timebase_id
genesis_or_epoch_identity
origin_tick
origin_sim_instant
step_duration
mapping_policy
```

For the current exact-nanosecond gameplay clock, `step_duration` is one positive integer number of nanoseconds.

A future rational timebase may instead bind a positive numerator/denominator representation under a new schema/policy.

## 4. Exact integer-nanosecond mapping

For an exact-nanosecond profile:

```text
delta_ticks = tick - origin_tick
delta_ns = delta_ticks * step_nanoseconds
sim_instant = origin_sim_instant + delta_ns
```

All arithmetic is checked.

No floating-point conversion participates in canonical time identity.

## 5. Reverse mapping

A `SimInstant` maps back to a tick only when:

1. the instant is on or after/before the origin according to the allowed tick domain;
2. its exact nanosecond delta is divisible by the fixed step duration;
3. the resulting tick fits the declared representation.

A non-aligned instant fails exact reverse conversion.

The implementation must not silently round to the nearest tick.

If an interpolation or quantization policy is ever required, it is a separately identified policy and cannot masquerade as exact conversion.

## 6. Current `SimulationClock::from_hz` semantics

The current clock accepts a frequency only when:

```text
1_000_000_000 % hz == 0
```

This guarantees an integer nanosecond step.

That property is useful and should remain explicit.

For example:

- 20 Hz -> 50,000,000 ns exactly;
- 100 Hz -> 10,000,000 ns exactly;
- 60 Hz does not divide 1 second into integer nanoseconds exactly.

Do not “support” 60 Hz by truncating `16,666,666.666... ns` to an integer and silently drifting canonical simulation time.

A future 60 Hz canonical clock should use a rational timebase contract if needed.

## 7. World/child manifest semantics

A `WorldContinuationManifest` binds `fixed_timebase_identity` or an equivalent typed timebase digest.

A child manifest may:

### Inherit

Use the exact parent timebase without repeating the full record.

### Declare mapped local timebase

Use a domain/body-specific local fixed step only when it declares a canonical mapping to the common `SimInstant` coordinate.

The child cannot define an unrelated notion of elapsed time and still claim same-world continuation.

## 8. Inactive scopes

Inactive-time policies operate on `SimInstant`, not host time.

### Paused

Source and resumed `SimInstant` remain equal until the scope is explicitly advanced.

### Deterministic catch-up

The policy binds:

```text
source_sim_instant
target_sim_instant
timebase_identity / evolution policy
forcing context
```

Work scheduling may split the advance into chunks, but the target semantic instant remains fixed.

### Coarse evolution

A coarse/analytical domain still exposes source and target `SimInstant` under the common coordinate and binds its evolution policy/receipt.

## 9. Timebase migration

Changing the fixed step of an existing world is not a metadata edit.

A migration must bind:

- source timebase identity;
- destination timebase identity;
- exact source tick/instant;
- exact destination tick/instant if representable;
- migration policy/receipt;
- handling of state variables whose numerical integration depends on step size.

Equal physical state at the migration boundary does not prove that future numerical evolution is equivalent under a different time step.

Therefore a timebase migration may require domain-specific requalification beyond simple time-coordinate conversion.

## 10. Long-timescale domains

Geology, ecology, cities, orbital systems, and other slow domains may use coarser stepping internally.

They still need a declared relationship to `SimInstant`.

The contract permits:

- exact submultiples/multiples of a base time unit;
- deterministic analytical evolution between absolute instants;
- event-driven evolution with explicit event instants;

but does not permit hidden “days since loaded” or wall-clock elapsed time to become authority input.

## 11. Canonical timebase digest

The digest is serializer-independent and binds at minimum:

```text
domain separator
schema_version
timebase_id
genesis/epoch identity
origin_tick
origin SimInstant seconds+nanos
step representation
mapping policy identity
```

Changing step duration, epoch, origin, or mapping policy changes the digest.

## 12. Required validation failures

Fail closed for:

- zero step duration;
- unknown required timebase schema;
- overflow during tick -> instant conversion;
- non-aligned instant -> tick exact conversion;
- different child/parent timebases with no declared mapping;
- same-world resume with mismatched timebase identity;
- catch-up target earlier than source unless a separate rewind/branch policy exists;
- silent timebase change during snapshot migration;
- implicit wall-clock-to-simulation-time conversion.

## 13. Required Q2 tests

Stable fixture IDs should include equivalents of:

```text
Q2-TIMEBASE-MAP-001
Q2-TIMEBASE-ROUNDTRIP-001
Q2-TIMEBASE-NONALIGNED-001
Q2-TIMEBASE-RESUME-001
Q2-TIMEBASE-MISMATCH-001
Q2-TIMEBASE-CATCHUP-001
```

### Exact mapping vector

For a timebase with:

```text
origin tick = 0
origin SimInstant = genesis
20 Hz
step = 50,000,000 ns
```

require:

```text
tick 400 -> SimInstant(20 seconds, 0 ns)
```

### Resume

Suspend at a known tick/instant pair, restore, and require:

- same timebase digest;
- same tick;
- same `SimInstant`;
- exact bidirectional mapping.

### Mismatch

Restore identical domain snapshots under a changed timebase and require continuation-manifest mismatch/failure unless an explicit migration applies.

### Catch-up

Different legal host scheduling partitions to the same target `SimInstant` must not change canonical target-time identity or final authority state where the declared evolution policy promises scheduling independence.

## 14. Relationship to event journals

Event ticks are meaningful only relative to a timebase.

A canonical v2 event journal should either:

- store canonical `SimInstant`; or
- bind every tick-based event chain to an immutable timebase identity.

A bare numeric tick is not globally meaningful across different step durations or origins.

This requirement complements the serializer-independent causal event work in #82.

## 15. Non-goals

v0.1 does not:

- force every domain to execute at the same update rate;
- require 60 Hz or prohibit it under a rational future profile;
- use floating-point seconds for canonical identity;
- equate host wall-clock time with world time;
- claim that changing numerical step size preserves domain dynamics;
- merge gameplay time and geological model internals into one scheduler.

## 16. Outcome

The intended result is simple:

> a tick is never just a number — it is a coordinate under a named timebase, and every fixed-step coordinate can be mapped exactly to Symtropy's shared `SimInstant` world timeline.
