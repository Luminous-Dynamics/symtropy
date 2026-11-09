# LOCO-STRUCTURAL-DAMAGE-001 — Modular amputation, module loss and repairability benchmark

**Status:** protocol and synthetic damage manifests. No new physical damage or repair result is claimed.

Issue: [#4364](https://github.com/Luminous-Dynamics/symtropy/issues/4364).

## Ownership

This is the physical experiment/comparison layer for structural damage in modular and reconfigurable legged robots. It owns damage-injection truth, held-out cut/dock locations, physical module connectivity evidence, recovery and serviceability trials.

It composes with:
- #4360/#4361/#4362: morphology candidates and nominal/degraded Pareto audit;
- #4358/#4359: actuator, drive, sensor and power fault injection;
- #4342/#4345: fault possibility/signature envelope;
- #4332/#4334: failover and mode-transition semantics;
- #4338/#4340: exact qualified operating envelope;
- #4061/#4083: robust viability/recovery and physical abstraction;
- #4326: complete adverse outcome vector;
- #4329: held-out capability and cross-fitted tail metrics.

It does not replace any of these estimators, oracles or scorecards.

## 2026 baseline: autonomous modular legs

Yu et al., *Agile legged locomotion in reconfigurable modular robots*, PNAS 123(10), e2519129123 (published 10 March 2026): https://pmc.ncbi.nlm.nih.gov/articles/PMC12974426/ — project https://modularlegs.github.io/.

Each modular leg is a self-sufficient sub-machine with local battery, motor, sensing, PCB/processing and communications. Modules can serve as legs, arms, tails or a spine; five modules yield an estimated ~10^11 configurations. An 8D VAE latent plus asynchronous Bayesian optimization explores the combinatorial design space.

An amputation-agnostic sensorimotor policy was distilled from experts trained for intact, one-module-loss, two-module-loss and four-module-loss conditions. A physical five-module quadruped was tested at three previously unseen cut locations: one hindlimb removed, both hindlimbs removed and all but one module remaining. The paper reports retained forward locomotion in all three and 105.3% of original-controller speed in the undamaged condition.

**Limits:** only three previously unseen physical amputation conditions were tested; broader coverage is simulated. The authors explicitly state autonomous module absorption, in-situ morphology reconfiguration and self-repair are not yet achieved. Wi-Fi/remote computer execution was used in their experiments, although local controller execution is described as feasible. A latent decoder sometimes fails to reconstruct exact hand-designed topology. The benchmark must therefore measure connectivity, command isolation, manual repair and actual local autonomy separately.

## Failure modes must remain distinct

- **Amputation/detachment:** module or limb is physically disconnected at a known randomized cut/dock point.
- **Attached dead module:** module remains in the structure but its motor/driver is disabled; passive mass/contact persists.
- **Local power loss:** module is attached, but battery/drive power is absent.
- **Local compute reset or hang:** mechanical assembly and power may remain, but control is unavailable.
- **Sensor loss:** module's IMU/encoder or health telemetry fails.
- **Communication isolation:** module remains mechanically/electrically independent, but the aggregate controller cannot exchange messages.
- **Dock/connection fault:** partial engagement, slip, fracture or false attachment indication.
- **Link/structure damage:** stiffness loss, crack, fracture or changed load path without complete disconnection.

A detached autonomous module that still moves on its own is not equivalent to an amputated passive limb. A mechanically attached but unpowered module is not equivalent to either. The ground-truth manifest and runtime evidence must preserve these distinctions.

## Test strata

1. Self-sufficient modular sub-machines (module-local battery/actuation/sensing/compute).
2. Central-torso systems with detachable limbs that depend on central power/control.
3. Conventional quadrupeds with non-autonomous limbs.
4. Functional appendages/arms used for post-damage support.
5. Micro-hexapods with passive magnetic coupling (overload protection, not self-sufficient modules).

No pooling across morphology, scale, task or evidence tier without a directly justified matched comparison.

## Frozen damage manifest

Machine-readable synthetic cases: [fixtures/structural_damage_v0_1.json](fixtures/structural_damage_v0_1.json).

Every scenario binds morphology/module graph, CAD/dock revision, cut location and method, initial pose/gait/task, terrain/payload, fault onset, runtime-visible evidence, evaluator-only structural truth and expected mode/viability gate. Deployable policies must not receive the scenario ID, cut truth or future damage event.

## Baseline matrix

- intact morphology-specific expert policy;
- policy trained on the exact damage family (upper bound for known damage);
- amputation-agnostic sensorimotor policy distilled across damage experts (Yu et al.-style);
- explicit topology-aware policy with verified module graph;
- topology-oblivious sensorimotor adaptation (must not be mistaken for explicit diagnosis);
- Symtropy set-valued structural state + failover + #4061 viability/recovery gate;
- manual reassembly/repair workflow (serviceability arm);
- oracle damage/topology labels (upper bound only, never a deployable controller).

The controller's authority to issue commands to a module is conditioned on a fresh connection/power/compute state receipt. A missing or stale link cannot be silently treated as a healthy connection.

## Scorecard

Keep independent:
- exact injected and measured module graph;
- damage detection, topology estimation and Unknown rates;
- false attached/detached classifications;
- command-routing attempts to disconnected or unpowered modules;
- task completion, stable steps, displacement/speed and terrain capability;
- robust viability/recovery witnesses and time/energy to recovery;
- full #4326 adverse event vector and observation coverage;
- module-local autonomy under local-only compute/power;
- manual replacement/reassembly time, tools and false assembly rate;
- post-repair calibration and return-to-service qualification time;
- battery/actuator/compute energy, mass, payload and serviceability.

## Statistical and evidence rules

- Freeze platform/module hardware, firmware, CAD/dock revision, controller, terrain/task/payload and damage-injection manifests.
- Pair intact/damaged trials by morphology, module IDs, initial state, command, terrain, payload, seed and exposure.
- Randomize held-out cut points/connection locations using a frozen distribution.
- The robot/damage episode is the experimental unit; control ticks are not independent trials.
- Keep training, validation and held-out damage locations/patterns separate.
- Simulation, structural bench tests, test-rig damage trials, hardware locomotion and field repair are separate evidence tiers.
- Independent instrumentation verifies the actual post-damage graph, load paths and module state.
- After manual repair, require fresh structure/dock, actuator, sensor and runtime assurance checks before any task action.
- Every post-damage action remains subject to #4061/#4083 and #4340. Survival in sampled episodes is not a robust physical safety proof.

## Proposed targets (not achieved)

- 100% agreement between injected and independently measured structural topology on the frozen test corpus.
- Zero command routing to a physically disconnected module unless a separately admitted isolated-module task is active.
- Zero loaded use of a partially engaged dock without a fresh structural state receipt.
- Zero return to service after repair without structural, actuator, sensor and viability requalification.
- 100% detection of frozen false-connection, unknown-cut and stale-module-state cases.
- At matched conditions, improve held-out structural damage capability over the strongest directly matched baseline without worsening any adverse category or recovery reserve.
- Report the three PNAS physical amputation cases as a baseline reproduction target, not as evidence of general arbitrary damage tolerance.

## Claim ceiling

This campaign qualifies only exact tested module configurations, cut locations, tasks, terrain, payload and evidence tiers. Three physical amputation trials do not establish arbitrary damage tolerance. Manual reassembly is not autonomous self-repair. Module-local battery/compute does not establish distributed control until evaluated without remote dependencies.
