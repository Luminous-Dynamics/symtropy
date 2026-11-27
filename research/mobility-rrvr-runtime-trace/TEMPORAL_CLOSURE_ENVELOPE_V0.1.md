# RRVR Temporal Closure Envelope v0.1

**Status:** research contract. This document defines a temporal proof obligation for downstream quiescence. It is not an implementation, model-check result, transport qualification, or physical safety claim.

**Tracking:** [#7433 — temporal closure across in-flight commands and bounded late arrival](https://github.com/Luminous-Dynamics/symtropy/issues/7433)

## 1. Problem

The downstream command graph and per-layer receipts are already owned by [#6817](https://github.com/Luminous-Dynamics/symtropy/issues/6817) and its reference implementation [#6866](https://github.com/Luminous-Dynamics/symtropy/pull/6866). The observer dependency model is owned by [#6877](https://github.com/Luminous-Dynamics/symtropy/issues/6877), and endpoint token/epoch enforcement is modeled in [#7405](https://github.com/Luminous-Dynamics/symtropy/pull/7405).

This contract isolates one remaining question:

> Can an old command still arrive at a downstream holder after the system has declared the command path quiescent?

A process restart, empty queue, expired middleware sample, controller lifecycle transition, timeout, or status string cannot answer that question by itself. A message may already be in flight, retained by another queue, re-enqueued by an upstream component, or buffered below the layer that produced the receipt.

**Quiescence is a closed cut through the execution graph, not merely a quiet interval.**

## 2. Terms

- **Admission closure:** every ingress that can accept the old attempt or epoch has rejected further old work.
- **In-flight work:** an accepted command that has left one holder but has not yet been retired, cancelled, expired at an enforcing receiver, or delivered to the next holder.
- **Retention bound:** a qualified upper bound on how long a component may retain and later forward an accepted command.
- **Transit bound:** a qualified upper bound on how long a command can remain in an edge, bridge, bus, or transport.
- **Sequence barrier:** a monotonic generation/sequence boundary proving that all commands through a specified high-water mark have been received and retired or cancelled.
- **Temporal-closure certificate:** an attempt- and profile-scoped result that all declared paths satisfy a closure rule at a stated observation point.
- **Unknown horizon:** a path for which no finite upper bound or terminal actuation fence has been established.

A temporal-closure certificate does **not** establish physical NoEffect. A command may have acted before closure. Physical outcome remains a separate claim and evidence owner.

## 3. Closure proof paths

A certificate may use one of two proof paths. The exact path and evidence must be retained for every node, edge, fan-out branch, and terminal actuator.

### 3.1 Fenced-and-drained

For each reachable command-holding path:

1. Every ingress rejects the old attempt/epoch after a monotonic fence acknowledgement.
2. The final accepted generation and high-water sequence are identified.
3. Every accepted sequence through that high-water mark is accounted for as retired or cancelled.
4. Every message in transit between holders is accounted for by an ordered barrier, explicit receipt, or downstream query.
5. No upstream component can re-enqueue an old sequence after a downstream layer reports drained.
6. Every terminal actuator either enforces the current authority epoch at the actuation transition or has an independently checked terminal closure receipt.

A queue count of zero without a generation-bound accepted/retired watermark is insufficient. Where delivery can reorder, a single maximum sequence is insufficient unless the protocol proves that every lower sequence is complete; the receipt must expose the relevant gap/reorder semantics.

### 3.2 Bounded expiry

A time-based proof is admissible only if every edge and every retaining node on every reachable path has a finite, qualified upper bound.

The envelope must include:

- maximum edge transit time;
- maximum per-node queue/buffer residence time;
- endpoint scheduling/dispatch bound;
- source and receiver clock offset, drift, holdover and reset uncertainty;
- the time origin used to calculate expiry;
- receiver-side enforcement evidence;
- any re-serialization or bridge behavior that could refresh or extend message age.

Expiry must be **end-to-end and non-renewable**. A forwarder must not reset the command's original age or create a later expiry merely because it republishes the same command. If the platform cannot preserve a trustworthy expiry identity across a hop, that hop has no qualified expiry bound for this claim.

For a path p consisting of edges e and retaining nodes v, a conservative horizon is:

    H_p = sum(D_e) + sum(R_v) + U_clock,p + J_p

where D_e is the qualified maximum transit bound, R_v the maximum retention bound, U_clock,p the conservative clock-uncertainty contribution, and J_p the scheduling/endpoint execution allowance. The profile-wide horizon is the maximum H_p over all paths from ingress to terminal.

This arithmetic is valid only when every term is finite, measured or contractually bounded, and supported by the exact platform profile. It is not permission to invent a bound from observed percentiles. A measured p99 is a performance statistic, not a deterministic maximum.

If any reachable path segment is unbounded, unsupported, unqueryable, or has invalid clock semantics, then the horizon is Unknown/Infinite. No amount of elapsed wall time can turn that path into a finite proof.

### 3.3 Preferred composition

Use sequence barriers and monotonic endpoint fencing as primary mechanisms. Use bounded expiry only for components where that exact behavior is enforced and testable. A coordinator waiting for the horizon is not itself a fence and cannot stop a hidden or newly introduced queue.

Cycles require special treatment. A forwarding cycle must either be broken by fencing, constrained by a non-renewable hop/age budget, or proven to terminate. A cycle that can refresh age or re-enqueue old work prevents a finite temporal-closure claim.

## 4. Minimal evidence envelope

The machine-readable envelope is expected to bind:

### Scope
- schema and profile version;
- device/robot identity and exact attempt/logical-effect identity;
- old and new authority epochs, process boot IDs and controller epochs;
- platform, firmware, middleware, bridge, driver and endpoint configuration commitments;
- command-holding graph digest and applicability interval.

### Per-node evidence
- stable node identity and implementation/configuration digest;
- role: ingress, forwarder, queue, bridge, driver, PLC/fieldbus, firmware, or terminal endpoint;
- accepted, forwarded, retired, cancelled and outstanding sequence/generation state where supported;
- old-epoch admission-fence disposition and source receipt;
- queue query, drain and retention semantics;
- reboot, rollback, reset and buffer-loss behavior;
- raw artifact digest, observation interval and verifier identity.

### Per-edge evidence
- stable source/destination edge identity;
- ordering and delivery semantics;
- in-flight message disposition;
- finite transit bound or explicit unbounded/unknown value;
- timestamp/clock domain and any age-preservation semantics;
- evidence for the stated bound, including exact test/profile revision.

### Decision
- Closed, Unknown, or Hold;
- proof path: fenced-and-drained, bounded-expiry, or none;
- certificate time/sequence boundary and freshness interval;
- all blockers and invalidation reasons;
- independent verifier result and exact input digests.

The verifier must derive the result from the node/edge evidence. A coordinator-supplied quiescent=true field is not sufficient evidence.

## 5. Invalidation

A certificate is invalidated by any of the following within its scope:

- late old-epoch command arrival or execution;
- graph node/edge addition, removal, or undisclosed path discovery;
- profile, driver, firmware, bridge or middleware change;
- endpoint epoch rollback or endpoint history loss;
- sequence-generation reset or high-water receipt mismatch;
- source/receiver clock step outside the declared uncertainty budget;
- PTP loss/holdover beyond the qualified bound;
- stale, duplicate, contradictory or wrong-attempt receipts;
- an upstream component capable of re-enqueueing old work after its alleged fence.

Invalidation cannot be repaired by changing the evidence after the fact. It requires a new closure evaluation under the current exact profile.

## 6. Failure model and negative controls

The reference model and campaign should exercise:

- a message in flight while the upstream queue is empty;
- a fence racing with enqueue, forwarding, cancellation or actuator start;
- two fan-out branches where one terminal receives a late command;
- message reordering and a gap below a reported high-water mark;
- a hidden driver/firmware buffer;
- duplicated, delayed, stale or wrong-generation receipts;
- sequence and epoch rollback across reboot;
- a clock step, drift, holdover, reboot, timestamp substitution or timestamp-origin change;
- middleware expiry with downstream retention still active;
- forwarder age refresh and unbounded queue retention;
- a cyclic forwarding path;
- profile or inventory change after certificate issuance;
- a late old command after a fresh attempt has been prepared.

Required mutants include:
1. quiet-time-only closure;
2. upstream-only fencing;
3. expiry refreshed by a forwarder;
4. hidden edge treated as empty;
5. sequence gap treated as drained;
6. clock uncertainty omitted from the horizon;
7. closure retained after profile/epoch invalidation.

Each mutant should produce a counterexample preserving the exact property/witness signature. The minimized witness must retain the causal event that makes late execution possible.

## 7. Benchmark metrics

Keep safety and availability separate.

**Hard safety/evidence measures**
- false Closed certificates;
- old-epoch effects after qualified closure;
- closure with unaccounted in-flight work;
- missing/unknown graph-edge coverage;
- stale or invalidated certificates still used for admission;
- false NoEffect claims caused by closure-only evidence.

**Availability/performance measures**
- positive-control closure admission rate;
- Unknown/Hold fraction by layer and reason;
- false-hold rate;
- time to fenced-and-drained closure;
- conservative bounded-expiry horizon;
- measured versus declared transit/retention distribution;
- late-arrival detection latency;
- p50/p95/p99/p99.9 for fence, drain, transit, expiry and reconciliation.

Report results separately for schema/reference model, simulator, process-kill, storage fault, HIL, brownout/power-cut and physical campaigns. Do not promote one tier into another.

## 8. ROS 2 integration implications

The ROS 2 Control and QoS assay is tracked in [#7415](https://github.com/Luminous-Dynamics/symtropy/issues/7415). The temporal-closure envelope should record exact distro, package/commit, RMW/DDS, hardware plugin, driver and firmware because lifecycle behavior is not safely generalized across releases.

The current Lyrical hardware lifecycle documentation says INACTIVE command interfaces are unavailable but leaves handling of previously received command-interface values to the hardware component implementation. The ROS QoS design document states that Deadline tracking is at the rmw boundary, not proof that user code consumed a message; Lifespan expiry is based on a source timestamp and depends on sufficiently synchronized clocks. Those mechanisms are useful transport evidence, but they do not prove that a driver, fieldbus, firmware buffer or actuator has no retained work.

The exact platform assay must measure receiver-side expiry and endpoint effect counts. A DDS event, lifecycle transition or publisher configuration must never be promoted into physical NoEffect or end-to-end quiescence.

## 9. Claim ceiling

A schema/reference-model PASS means only that the supplied evidence satisfies the declared contract over its tested finite domain. It does not prove inventory completeness, real transport bounds, clock accuracy, driver/firmware behavior, physical NoEffect, endpoint stop, HIL, functional safety, certification or regulatory conformity.

All claims remain exact-attempt, exact-graph, exact-configuration and fault-model scoped.
