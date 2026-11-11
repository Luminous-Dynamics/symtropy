# LOCO-JOINT-METROLOGY-001

Owner: Symtropy issue LOCO-JOINT-METROLOGY-001.

This protocol composes existing AoI, actuator response, actuator loss, energy-boundary and thermal-state owners into one instrumented single-joint physical experiment. It does not create a duplicate controller, actuator model, thermal model or energy ledger.

## Measurement chain

    source capture
      -> decision / regret / calibration
      -> optional remote inference request / admission
      -> action prefix / projection / assurance
      -> actuator command latch
      -> torque / motion response
      -> electrical input and mechanical shaft work
      -> per-node thermal / persistent reserve transition
      -> terminal safe-stop / recovery

Keep separate:
- source-to-actuator AoI;
- command-to-motion onset and torque response;
- inference/network/queue/projection/dispatch delay;
- electrical energy entering the declared boundary;
- mechanical work crossing the declared shaft boundary;
- thermal state and measured heat;
- remaining actuator authority and recovery reserve.

## Declared physical boundaries

Freeze the actuator/gearbox/drive/controller/load identities; electrical supply boundary; mechanical shaft boundary and torque sign; thermal control-volume nodes; robot and DAQ clock identities / bounded mapping; sensor calibration identities and uncertainties; sampling and anti-alias requirements; and exact firmware/model/policy/configuration digests.

At minimum measure source capture and sequence, actual actuator command latch, delivered torque, shaft position/velocity, load torque, bus voltage/current, per-node temperatures, faults/saturation/derating, energy boundary terms and terminal safe-stop/recovery.

Sampling frequency must be selected from actuator bandwidth and transducer/instrument specifications, then frozen before confirmatory comparisons. The fixture rate values are synthetic known answers, not a recommended universal rate.

## Physical accounting

For a declared boundary:

    E_electrical_in = integral(V_bus * I_bus dt)
    W_shaft_out = integral(torque * angular_velocity dt)

These are independent measurements with units and sign conventions. Their difference is not automatically “motor heat”: gear, drive, wiring, auxiliary loads, stored energy and uncertainty must be accounted or marked Unknown.

Use Symtropy #3612/#3615/#4044 for open-system first-law closure and energy-path instrumentation; #4206 for command-to-motion / torque response; #4179 for actuator losses; #4610 for coupled thermal state; #4713 for AoI; #3597/#4641 for scoped analytical entropy/exergy references. Entropy/exergy, joules, task-loss regret and EVSI must remain distinct.

A balance is only as complete as its declared boundary and instrument coverage. Compare the residual to a predeclared combined uncertainty budget, not an arbitrary universal percentage.

## Sequential protocol

Use a declared sequence of baseline, repeated starts/stops and reversals, step/ramp loads, sustained thermal operation, sensor/time uncertainty, queue/dispatch delays, derating/degradation, then recovery and safe-stop. Do not reset temperature, wear proxies, actuator authority or reserve state between perturbations.

Pilot runs may establish variance and sample size. Freeze the perturbation order, paired/crossover protocol, held-out split, independent episode unit, success/adverse-event definitions and confidence procedure before confirmatory trials.

## Benchmark arms

Use matched hardware, policy and plant:
- fixed AoI;
- fixed action chunk;
- progress-triggered replanning;
- hard-bound commitment without decision sufficiency;
- decision-sufficiency plus hard bounds;
- synchronous / asynchronous execution;
- local / admitted remote inference;
- alternative context retention policies where remote context is involved.

Ablate one architecture factor at a time. Preserve identical independent hardware interlocks and emergency stop.

## Synthetic known answers

- `fixtures/joint_metrology_v0_1.json`
- `fixtures/joint_metrology_v0_1.schema.json`

Eight cases cover complete closure, actuator queue dwell, response delay independent of AoI, unknown clock mapping, incomplete energy boundary, missing thermal node, hidden physical-state reset and decision-regret expiry before hard physical limits. JSON and schema are parsed and semantically preflighted when authored; this is not JSON Schema validator execution or a physical trial.

## Metrics

Report source-to-actuator AoI intervals per modality; regret and calibration coverage; inference/network/queue/projection/dispatch/response latency separately; torque tracking and derating; electrical joules, shaft work and energy residual/uncertainty; per-node thermal trajectory coverage; future torque/mobility reserve; RTA vetoes/safe-stops/adverse outcomes; and p50/p95/p99 planning, application and response latencies.

Use episodes / independent perturbation sequences as statistical units. Correlated samples from one joint trace are not independent replicates.

## Prospective targets (not achieved)

- 100% exact agreement on the finite synthetic fixture corpus.
- Zero accepted cross-clock AoI without a qualified bounded mapping.
- No energy PASS when the physical boundary is incomplete.
- No thermal coverage PASS when a required node is absent.
- No reset of persistent state between sequential disturbances.
- Zero action execution beyond its calibrated decision horizon or qualified hard physical bound.
- Demonstrate lower replanning/compute/energy only at matched torque tracking and safe-success.
- Publish null results and regimes where fixed or simpler controls dominate.

## Claim boundary

The schema and fixtures qualify only synthetic contract semantics. They do not qualify clock truth, sensor calibration, actuator dynamics, thermal forecasts, energy-boundary completeness, viability, safe control or whole-robot readiness. Physical claims require calibrated instruments, independent reference channels, held-out sequential trials and exact trace provenance.
