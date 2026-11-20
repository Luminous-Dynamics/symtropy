# Stop-Tube Geometry Builder v0.1

**Status:** executable conservative geometry producer for a restricted translation model, with synthetic known-answer tests and a scale harness. No physical robot stop trajectory, hardware-in-the-loop result, field occupancy assessment or safety qualification is claimed.

## 1. Why this is a separate engineering owner

A braking-distance scalar answers how far a qualified robot may travel after a trigger. It does not describe where the complete body, links, tools and payload can be while every possible trigger time is considered.

The stop-tube builder converts:
- an already validated nominal motion-envelope report;
- its complete primitive manifest and time slices;
- a qualified jerk-limited stopping result;
- a site-clock domain and validity horizon;
- a declared kinematic model;

into a deterministic, time-indexed set of conservative 3D outer boxes.

The occupancy engine still owns dynamic actor/obstacle comparison, broad-phase indexing, narrow-phase evaluation, site-policy clearance and the non-actuating occupancy assessment. The integrated dispatch gate still checks that the exact stop-tube digest and all trigger/primitive/time coverage are bound to the readiness snapshot. The builder does not reserve resources or authorize actuation.

## 2. Supported mathematical model

For a source trajectory segment with a fixed tangent \(\hat d\), each possible trigger position lies on the nominal translation segment \(p(t_0)\). The qualified stopping profile supplies conservative total path distance \(D_s\) and elapsed stop time \(T_s\), including detection-to-brake latency.

For a trigger-time subinterval \(W=[t_a,t_b)\), the supported stop-tube set for primitive \(i\) is contained in:

\[
\mathcal S_{i,W} =
\left\{
p_i(t_0) + \lambda \hat d :
t_0 \in W,\ 0 \leq \lambda \leq D_s
\right\}
\oplus M_i
\]

where \(M_i\) contains the primitive radius, path-deviation, localization, tracking, model-uncertainty, numerical-enclosure and declared stopping-reachability margins. The output's time interval is \([t_a,t_b+T_s)\).

The spatial AABB is the exact axis-aligned bound of the trigger-position segment Minkowski-summed with the fixed-tangent stop segment, then expanded by the declared margins and an explicit 1 mm per-axis outward projection guard. Temporal and spatial dimensions are deliberately combined into an outer box. This is conservative, but may be inflated because every stop phase is represented over the complete occupancy-time interval for that trigger window.

The builder partitions each source segment into deterministic half-open trigger windows no wider than the requested maximum trigger-window width. Smaller windows generally reduce the spatial/temporal box inflation at the cost of more output records. Every source segment is fully covered; a gap, invalid interval, missing primitive or overflow is rejected.

## 3. Explicit restriction: no articulated or steering stop

v0.1 only accepts **rigid translation with fixed orientation and a fixed braking tangent**:
- every primitive centre in a source time slice must share the same translation vector;
- primitive-relative offsets must remain identical across the full nominal trajectory;
- the robot does not steer or rotate during the braking trajectory;
- the qualified distance/time bounds apply to the exact brake configuration, payload, operating condition and scenario context.

This is not an assumption to hide in an adapter. It is part of the model identity and build contract.

The builder explicitly rejects:
- articulated motion or changing relative primitive offsets;
- a stop model that allows steering during braking;
- a nonzero stopping distance without a defined nominal tangent;
- mismatched robot/tool/payload/frame/transform/clock identities;
- a source report that cannot be reconstructed from the validated motion-envelope builder;
- a stopping result that differs from recomputation under the supplied qualification;
- stale qualifications, malformed digests, overflow and incomplete primitive/time coverage.

A real construction manipulator or mobile manipulator with changing link configuration is **outside the supported v0.1 domain**. Such systems require an articulated reachable-set/interval-kinematics builder, including joint limits, tool/payload transforms, swept orientation and uncertainty. Do not use this translation builder for them.

## 4. Margin ownership and double-expansion rules

The stop tube uses the source physical margins, plus the source's explicit stopping-reachability margin, and the qualified stopping-distance projection. It deliberately does **not** include project policy clearance. Policy clearance is owned by the occupancy/policy layer and must be applied exactly once there.

The original coordination outer envelope must not be fed back into this builder; doing so would add policy clearance to the physical stop tube and make later margin accounting ambiguous. The builder reconstructs and validates the source report from its original centreline primitives and declared margins, then constructs the new stop tube separately.

The output includes the exact nominal report, trajectory, primitive manifest, frame, transform, design revision, site epoch, robot/tool/payload, collision model, brake configuration, stopping profile, operating condition, scenario context, clock domain and builder build identities. A change to any of those inputs requires a new build.

## 5. Independent mathematical checking

For the supported fixed-tangent model, the exact axis-aligned outer bound can be independently checked from the four vertices of the parallelogram:

\[
\{p_a,\ p_b,\ p_a+D_s\hat d,\ p_b+D_s\hat d\}
\]

expanded by the same declared axis margins. The unit tests implement this vertex oracle separately from the production bound-construction routine and cover positive/negative axes, diagonal translation, multiple primitive offsets, trigger-window partitioning, digest binding, tampered stop-distance output, clock mismatch, stale qualification, overflow, invalid kinematics and deterministic canonical serialization.

The oracle is independent at the geometry-formula level, but it is still in the same software repository and is not an external laboratory. A stronger research claim requires a separately implemented reference (for example, interval arithmetic in a second language/toolchain) and held-out cases not used to tune the builder.

## 6. Benchmark programme

The new stop-tube-builder-bench binary measures full source-report revalidation, stopping-result recomputation, time partitioning, bound generation and canonical serialization. It emits:
- source time segments and primitive count;
- trigger windows and output primitive-window slices;
- p50/p95/p99 elapsed time;
- output slices/second and canonical bytes;
- qualified stopping distance and time.

The development scale points are 400, 4,000 and 100,000 primitive-window slices. The first benchmark run is a synthetic single-host baseline; it is not a production throughput claim.

Required next benchmarks:
1. **Containment:** zero under-bounds against an independent analytical/interval oracle over a frozen supported-domain corpus.
2. **Coverage:** 100% trigger-domain, primitive and continuous-time coverage; zero missing IDs or silent time gaps.
3. **Mutation:** every brake, payload, tool, frame, transform, site-clock, trajectory, builder-build and policy change either creates new evidence or holds readiness.
4. **Inflation:** compare stop-tube spatial-time volume against an independently computed reachable-set reference; stratify straight, diagonal, segmented, long-duration and high-speed cases.
5. **Performance:** p50/p95/p99, peak memory, output size and bounds built at 1K, 10K, 100K and 1M primitive-window slices.
6. **Differential backends:** compare candidate completeness and false-positive rate against MoveIt Bullet CCD, Tesseract LVS_CONTINUOUS/CONTINUOUS, and a separately pinned FCL CCD evaluator where the geometry/motion model is supported.
7. **HIL / physical:** independently track the robot, each tool/payload primitive and stopping motion across speed, load, grade, surface, wear, battery and environmental strata. No software receipt substitutes for measured physical stop behaviour.

A false-safe/under-bound is a hard failure. Optimize volume inflation and runtime only after containment passes. Do not trade a missed stop region for speed.

## 7. Industry comparison and differentiation

MoveIt and Tesseract already provide mature robot state/trajectory collision checking; FCL supports discrete collision, distance, tolerance and continuous-collision queries. Tesseract documents the distinction between per-state DISCRETE, interpolated LVS_DISCRETE, and cast-based LVS_CONTINUOUS; its continuous cast uses the convex hull between poses. MoveIt's Bullet integration documents continuous collision capability, while FCL and Bullet have different scope and runtime characteristics. The right comparison is a pinned, matched backend on the same geometry and trajectory, not a generic claim that one detector is better.

This builder does not replace any of those motion planners or collision libraries. Its potential contribution is construction-specific: a complete stopping envelope across all possible trigger times, bound to the work package, robot/tool/payload, source design revision, site frame, brake qualification, clock domain and independently assessed dynamic occupancy. The wider site-state ledger then preserves the difference between robot-reported progress, measured physical output and independently accepted work.

The system should eventually choose among:
- avoid the task or redesign the work package;
- prefab or change the assembly sequence;
- perform the work manually or with human assistance;
- use a robot under a verified stop-aware work envelope.

The optimization objective remains independently accepted functional output per full workflow cost, not robot utilization or nominal path speed.

## 8. Standards and assurance boundary

Use ISO 10218-1/-2:2025 for industrial-robot machinery/application scope where applicable; screen ISO 3691-4:2023 for driverless industrial trucks/AMRs where the machine definition and scope apply; use AMRA-201:2026 for mobile-robot general requirements and test methods on solid travel surfaces; and build measurement methods with NIST MPRS and relevant ASTM F45 work. These references require a clause-by-clause applicability assessment by a competent safety assessor. This prototype is not certified, not a protective function, and not an authorization to move.

The output report's canonical bytes are deterministic. The caller must calculate and verify the report digest and signature against those bytes, authenticate the source model and brake/sensor qualification receipts, verify the independent occupancy assessment, and then pass the complete context to the separate authorization authority.
