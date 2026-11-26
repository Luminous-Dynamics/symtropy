# Orbital inspection safety claim v0.1

## Purpose

The physical-safety key in [#7484](https://github.com/Luminous-Dynamics/symtropy/issues/7484) must mean more than “an upstream Boolean said PASS.” Its meaning must state exactly which safety obligation was verified, for which protected object, geometry, state, dynamics and time interval.

This adapter owns the shape and composition of those claims. The orbital dynamics, trajectory optimization, control shield, robust reachability, collision-risk estimation and proof verification remain separate provider concerns.

## Two distinct deterministic obligations

**Finite-horizon keep-out exclusion.** The entire executed action trajectory, over the full action interval [t0,t1), must remain outside the declared keep-out set with an outward lower bound on minimum clearance. The proof must state the geometry and uncertainty set it covers. A sampled endpoint check is not a continuous-time enclosure.

**Safe continuation.** At the action terminal epoch t1, the terminal state must enter a declared passive invariant set or a separately qualified abort/retreat reachable set. This prevents a finite-horizon safe trajectory from depending on an unqualified future action to remain safe.

These components are non-compensatory. Both are required for SAFETY_CLAIMS_COMPLETE. A finite-horizon claim cannot stand in for continuation; continuation cannot stand in for the executed trajectory.

## Statistical risk is a different claim

A Pc/3D-Pc/Nc/Monte Carlo confidence interval and expected consequence are valuable evidence, but they are not a deterministic trajectory shield. The validator labels an isolated statistical risk receipt RISK_ONLY. It must never turn a low probability or high confidence level into a robust keep-out exclusion.

This is consistent with NASA's separation of conjunction screening, conjunction risk assessment and conjunction mitigation. The exact action shield is a further engineering obligation, not a renaming of collision probability.

## Exact action scope

Each action scope binds:
- action ID and action revision;
- vehicle/servicer ID and configuration revision;
- trajectory revision and action horizon;
- protected object ID and revision;
- keep-out profile ID and geometry revision;
- reference frame and realization revision;
- transform convention;
- dynamics/propagator revision;
- initial relative-state revision;
- uncertainty-set revision;
- canonical time-basis ID;
- action start/end epoch;
- required operational minimum clearance.

Each safety claim repeats this full scope. A claim for another object, frame, keep-out set, state, force model, uncertainty set, or action revision is OUT_OF_SCOPE. The action's required clearance is authoritative; a provider cannot weaken it by returning a smaller self-declared requirement.

## Claim forms

1. FINITE_HORIZON_KEEP_OUT:
   - assurance class ROBUST_TRAJECTORY_ENCLOSURE;
   - explicit temporal enclosure complete=true;
   - validity interval covers the full action interval in the canonical time basis;
   - minimum-clearance lower bound and required clearance;
   - upstream state PASS/FAIL/UNKNOWN/STALE/REVOKED.

2. SAFE_CONTINUATION:
   - assurance class PASSIVE_INVARIANT_SET or ABORT_REACHABLE_SET;
   - terminal set ID/revision;
   - terminal epoch exactly equal to action end;
   - validity includes that terminal epoch;
   - proof receipt digest and explicit upstream state.

3. STATISTICAL_RISK:
   - assurance class PROBABILISTIC_RISK_BOUND;
   - risk measure such as Pc or Nc;
   - its own validity and exact same action scope.
   - This remains RISK_ONLY unless a separate deterministic finite-horizon and safe-continuation claim exists.

A source may compose these with other evidence in its own certificate, but it must explicitly declare both deterministic components and preserve their individual proof references.

## Dispositions

- SAFETY_CLAIMS_COMPLETE: both required deterministic components are current, scope-identical, time-valid and satisfy the declared clearance.
- SAFETY_CLAIM_FAILED: an explicit component FAIL or the finite-horizon lower-clearance bound is below the required margin.
- SAFETY_CLAIM_INCOMPLETE: only one deterministic component is present.
- RISK_ONLY: only a statistical risk component is present.
- OUT_OF_SCOPE: known action/asset/keep-out/frame/model/uncertainty/time mismatch or inadequate interval coverage.
- INDETERMINATE: unknown, stale, revoked or unresolved source state.
- INVALID_INPUT: malformed or contradictory input, claim type, interval, digest shape or clearance declaration.

Malformed data dominates; known mismatch dominates unresolved state; explicit component failure dominates completeness. No utility or decision-value score participates.

## Frozen adversarial corpus

The 26 cases cover:
1. composite finite-horizon + passive continuation pass;
2. finite-only and continuation-only inputs;
3. incomplete finite-horizon intervals (start late / end early);
4. vehicle, protected object, keep-out profile, frame, dynamics, initial-state, uncertainty-set and time-basis mismatches;
5. Pc-only and Nc-only risk receipts;
6. unknown, stale and revoked evidence;
7. lower clearance below the action requirement;
8. terminal epoch mismatch;
9. wrong assurance class;
10. malformed digest and missing terminal proof;
11. a complete abort-reachable terminal-set alternative;
12. finite-horizon validity ending exactly at the half-open action end;
13. terminal proof validity ending at the terminal epoch, which must be OUT_OF_SCOPE.

The independent Python oracle and Rust implementation must match complete receipts. The claim order and case order are canonicalized.

## Competitive benchmark

Reproduce Stephenson & Schaub (2026), “Two Approaches to Autonomous Spacecraft Inspection: Shielded Reinforcement Learning versus Probabilistic Roadmaps” (DOI 10.1007/s40295-026-00631-w), on its declared eccentric-LEO scenario.

The paper's shield projects policy burns to the closest safe action and requires both a finite-horizon keep-out inequality and a terminal passive-safety condition. It dynamically refines its sample grid but estimates inter-step maximum speed from endpoint speeds. This is a strong baseline, but the endpoint-speed approximation should be challenged under non-monotonic velocity, high eccentricity and uncertainty.

Compare:
- the paper-style endpoint-speed adaptive shield;
- fixed fine time grids (diagnostic only);
- interval/validated continuous-time reachable-tube checking;
- shielded RL, offline PRM–MIQCQP and a hybrid PRM + onboard receding-horizon repair.

Report false-safe admissions, missed inter-step violations, terminal safe-set membership, clearance conservatism, false holds, shield intervention rate, minimum clearance, subdivision count, onboard p50/p95/p99 verification latency, delta-v, elapsed orbits, and evidence-valid inspection yield separately.

Prospective software target: zero incomplete or risk-only claims admitted as complete; zero known scope/time mismatches accepted; 100% independent oracle parity. Physical zero-violation performance must be measured on a declared dynamics/uncertainty model and independent truth source, not inferred from synthetic fixtures.

## Ownership and boundary

- #7484 / #7486 own two-key action admission.
- #6828 / #6746 own statistical conjunction risk and risk evidence.
- #7264 owns coupled post-action event portfolios.
- #7227 / #7281 own generic temporal enclosures and proof-carrying collision culling; their generic scope is not assumed to prove orbital dynamics.
- This issue owns typed safety-claim composition only.

The evaluator does not propagate orbits, validate force models, prove a keep-out invariant, verify a passive-safe theorem, authenticate issuers/bytes, authorize burns or establish operational mission safety.
