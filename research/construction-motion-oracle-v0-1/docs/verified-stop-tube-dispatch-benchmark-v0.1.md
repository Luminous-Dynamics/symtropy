# Verified Stop-Tube Dispatch Benchmark v0.1

**Status:** software contract and synthetic adversarial fixtures. No robot stop-tube geometry, occupancy experiment, braking HIL result, physical test or safety certification is claimed.

## 1. The missing distinction

A scalar stopping-distance budget and a collision-free nominal trajectory answer different questions:

1. Does the independently qualified sensor lower-bound range exceed the qualified longitudinal stopping distance plus uncertainty and policy margins?
2. Has the *complete physical geometry* of every possible stopping motion been conservatively swept and independently checked against the dynamic site occupancy?

Both must hold. A clear nominal trajectory plus a closed scalar distance budget is not enough to establish that the stop motion avoids another robot, worker, temporary support, stored material, unmodelled obstacle or workface.

## 2. Stop-tube definition

For a frozen nominal trajectory \(q(t)\), trigger domain \(\mathcal{T}\), primitive set \(\mathcal{P}\), and qualified stopping dynamics \(\Theta\), the stop-tube envelope represents the union

\[
\mathcal{S} =
\bigcup_{t_0 \in \mathcal{T}}
\bigcup_{p \in \mathcal{P}}
\bigcup_{\tau \in [0,\,T_{\mathrm{stop}}(\Theta)]}
G_p\!\left(q(t_0),\,\tau;\,\Theta\right)
\]

where \(G_p\) is the world-space occupied geometry of primitive \(p\), including link/body, tool and payload, during the stop initiated at possible trigger time \(t_0\).

The production representation may be a conservative time-indexed union of capsules/convex primitives or another independently qualified swept-volume representation. It must include:
- every possible trigger point in the frozen nominal trajectory time domain;
- detection-to-brake latency and the qualified jerk-limited stop trajectory;
- every required body, link, tool, payload and carried-material primitive;
- the exact robot/tool/payload configuration;
- the site frame and survey transform;
- the relevant clock domain and trigger-time interval;
- all qualified geometric and tracking uncertainty inputs, with owner and validity;
- a conservative outer bound and continuous-time coverage claim.

The bound may be larger than the physically reachable set. It may not omit a reachable region. The current contract checks the producer's receipt binding and declared coverage; it does not independently calculate arbitrary articulated stop geometry.

## 3. Required cross-artifact bindings

A StopTubeEnvelopeReceipt is tied to:
- the nominal trajectory digest and full trigger-domain digest;
- the composite envelope digest (nominal motion plus stop tube);
- primitive manifest digest and exact robot/tool/payload configuration hashes;
- site frame and transform hashes;
- stopping profile and qualification receipt;
- operating-condition and shared scenario-context hashes;
- site-clock domain, trigger-domain start/end ticks, normalized trigger-window duration and stop coverage duration;
- conservative stop distance and time, primitive count, coverage flags, builder build and receipt digest.

The occupancy assessment must name the exact stop-tube digest, the same composite envelope digest and primitive manifest, and explicitly attest that it evaluated every stop-tube primitive throughout the stop-time domain. Both broad-phase pair coverage and narrow-phase evaluation must be complete, with no unresolved, invalid, unqualified or stale occupants.

The integrated dependency snapshot includes separate StoppingTubeEnvelope and StoppingTubeBuilderBuild nodes. A builder implementation change therefore invalidates a prior tube receipt even if its previously emitted geometry digest is unchanged. A changed stop-tube receipt, geometry, robot/tool/payload configuration, trajectory, frame, transform, brake profile, site clock, occupancy engine or clearance policy also invalidates the readiness snapshot.

## 4. Adversarial challenge corpus

The minimum development corpus should contain:

1. **Nominal clear / stop conflict:** nominal path clear, but stop tube intersects a stationary obstruction.
2. **Crossing robot:** nominal paths are time-separated but a stop triggered at the latest allowed time creates a crossing conflict.
3. **Human/actor entry:** a dynamic actor enters the stop tube while the robot is moving.
4. **Tool extension:** body-only envelope is clear while an extended tool/payload stop tube conflicts.
5. **Missing primitive:** tool/payload absent from the stop manifest, including omission throughout all trigger times.
6. **Trigger-time gap:** stop tube covers only a subset of possible trigger points.
7. **Time gap:** stop geometry is sampled at endpoints but an intermediate continuous-time collision exists.
8. **Configuration drift:** brake, payload, frame, sensor or tool changes after stop-tube generation.
9. **Qualification expiry:** braking, detection or tube evidence expires before consumption.
10. **Clock-domain mismatch:** stop tube and occupancy use different or expired site-clock mappings.
11. **Understated bound:** declared distance or time is less than the analytic stopping reference.
12. **Occupancy mismatch:** occupancy receipt assesses nominal envelope or wrong stop-tube digest.
13. **Unresolved narrow phase:** unresolved candidate pair must hold integrated readiness.
14. **Worksite revision:** site/design change invalidates only affected stop-tube dependencies, never silently retaining stale readiness.

Every case must record expected result, mutation, owner, digest binding and why it must fail closed. Include independent analytical cases and held-out generated geometries; the stop-envelope producer must not serve as its own sole oracle.

## 5. Benchmark targets

These are research targets, not achieved results:

| Measure | Target |
|---|---|
| Stop geometry containment | Zero under-bounds against an independent analytic/numerical oracle over the frozen supported domain |
| Primitive coverage | 100% manifest coverage for all time cells |
| Trigger-domain coverage | 100% of declared possible trigger interval |
| Continuous-time coverage | No endpoint-only or chord-only substitution |
| Occupancy assessment | 100% of stop-tube candidates receive a terminal narrow-phase result; zero unresolved on a dispatch-ready result |
| Mutation detection | 100% rejection or explicit HELD/UNRESOLVED for stale frame, tool, payload, brake, sensor, clock, model, builder and receipt changes |
| Conservative inflation | Report stop-tube swept volume / independent reachable-set volume and time overbound, stratified by scenario |
| Latency / memory | p50/p95/p99 and peak memory at 1K, 10K, 100K and 1M primitive-time cells |
| Cross-backend parity | Matched stop-tube fixtures preserve candidate completeness across Symtropy, MoveIt and Tesseract adapters |
| Physical qualification | Independently measured stopping bounds across speed, payload, grade, surface, wear, battery and environment strata |

Never optimize stop-tube size by accepting under-bounds. Optimize only after containment has passed. Broad-phase false positives are a cost metric; false negatives remain a hard failure.

## 6. Baselines and reporting

Compare:
- nominal-motion-only occupancy;
- scalar-distance-only stopping budget;
- endpoint stop snapshots;
- sampled time-slice stop geometry;
- conservative continuous stop tube;
- independent analytical or high-resolution numerical reachable set.

Report false-safe count, false-positive conflict count, stop-tube volume inflation, time overbound, candidate pair count, narrow-phase evaluations, unresolved counts, p50/p95/p99 runtime and peak memory. Keep the accuracy/containment metric separate from performance and productivity.

The field productivity metric remains independently accepted output per total crew-hour. A stop-aware readiness pass is not itself a productivity result or a physical safety certification.

## 7. Claim discipline and safety boundary

The implementation added to integrated dispatch readiness checks that the receipt binds the same nominal trajectory, composite envelope, primitive manifest, robot/tool/payload, frame, transform, stopping profile, operating condition, scenario context and clock domain. It checks the declared stop distance/time bounds dominate the analytic stopping result; it requires complete trigger-domain, primitive and continuous-time flags; it requires the occupancy receipt to name the exact stop tube; and it adds the tube to the dependency graph.

The consuming application must verify canonical bytes, digest and signatures. The envelope producer must independently prove its geometric containment. The occupancy engine must independently evaluate the composite envelope. A separate safety authority must determine whether the robot, cell and site satisfy applicable machinery, mobile-robot and construction-site requirements.

This is not a safety-rated braking function, protective-field implementation, certified collision-avoidance system, actuation permit or proof of physical containment. An integrated readiness receipt is evidence for a separate authorization decision only.
