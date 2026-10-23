# Energy System Design Studio v0.1

**Status:** draft integration contract; synthetic-only first demonstrator. No field-operational or safety claim is implied.

Related tracking:
- Symtropy: [ENERGY-SYSTEMS-STUDIO-001 #2721](https://github.com/Luminous-Dynamics/symtropy/issues/2721)
- Symthaea: [ENERGY-ARCHITECT-001 #8496](https://github.com/Luminous-Dynamics/symthaea/issues/8496)
- Symtropy: [thermal-store twin #2683](https://github.com/Luminous-Dynamics/symtropy/issues/2683), [mission-safe energy resilience #2686](https://github.com/Luminous-Dynamics/symtropy/issues/2686), [controller benchmark #2719](https://github.com/Luminous-Dynamics/symtropy/issues/2719), [source-to-site delivery #2085](https://github.com/Luminous-Dynamics/symtropy/issues/2085)
- Symthaea: [generation evidence catalog #8381](https://github.com/Luminous-Dynamics/symthaea/issues/8381), [policy scenario corpus #7248](https://github.com/Luminous-Dynamics/symthaea/issues/7248), [SAM battery cross-validation #7305](https://github.com/Luminous-Dynamics/symthaea/issues/7305)
- Sol-Atlas: [bounded exact dispatch oracle #309](https://github.com/Luminous-Dynamics/sol-atlas-leptos/pull/309), [dispatch regret benchmark #311](https://github.com/Luminous-Dynamics/sol-atlas-leptos/pull/311)

## Decision

Yes: Symthaea should be able to design candidate energy systems for evaluation in Symtropy. But it must not be both designer and certifier. Use four separable planes:

| Plane | Responsibility | Authority boundary |
|---|---|---|
| **Symthaea — architect** | Generate diverse typed system graphs, scenario sets, experiment plans and explainable trade-offs. | A proposed design is a hypothesis, not a validated design. |
| **Sol-Atlas — evidence and optimization** | Resource/site evidence, service and spatial boundaries, typed energy accounting, exact bounded reference problems and objective reports. | Unknown input stays unknown; missing cost/capacity never silently becomes zero. |
| **Symtropy — digital twin** | Run deterministic, versioned time-coupled component/network approximations and fault/recovery trajectories within explicitly stated model scope. | The rigid-body engine is not an AC load-flow solver; a model trajectory is not metered truth or hardware-in-the-loop evidence. |
| **Mycelix — provenance and review** | Preserve source/data/model/code digests, approvals, evidence class, transformations and rights metadata. | Attestation binds reviewed bytes; it does not make false input true or promote simulation to measurement. |

This should be an orchestration and assurance layer, not a clone of the existing thermal store, energy accounting, or Symthaea scheduler implementations.

## Paradigm shift

Move from "ask an AI for the best design" to **generate → falsify → simulate → independently verify → compare → qualify**.

1. Build an evidence envelope for each resource, load, asset, cost, conversion and uncertainty assumption.
2. Have Symthaea propose several structurally distinct architectures and the smallest experiments most likely to reject weak candidates.
3. Reject malformed units, typed-port mismatches, impossible capacities, unsupported evidence transfers and violated hard constraints before simulation.
4. Run exact bounded Sol-Atlas cases for the subproblems they represent; retain transparent and random/diverse baselines alongside Symthaea candidates.
5. Run the candidate trajectories through the relevant deterministic Symtropy component twins and fault scenarios.
6. Use an independent verifier to recompute balance, state, service shortfall, hard bounds and summary statistics from raw per-step outputs.
7. Compare Pareto outcomes; preserve infeasible candidates and their counterexamples rather than quietly removing them.
8. Require engineering review before any claims about field operation, protection, safety, cost savings or deployment.

Do not treat Phi, HDC similarity, free-energy reduction or a confidence estimate as a substitute for energy conservation, evidence quality or independently verified physical constraints. Ablate the Symthaea-specific mechanisms: they must earn their place through better validity, candidate diversity, decision quality or compute efficiency against matched baselines.

## Candidate design contract

The companion schema at schemas/energy-system-candidate-v0.1.schema.json establishes an initial interchange shape. A candidate carries:
- stable candidate ID, schema and generator version;
- a source/service/system boundary and time horizon;
- typed component nodes, energy-carrier ports and explicit directed connections;
- service requests with criticality and required temperature grade where relevant;
- evidence-classed parameter claims, units, provenance, uncertainty and applicability;
- operating scenarios, availability and forecast-information boundaries;
- hard constraints and independently stated objective dimensions;
- declared evaluations/solver budgets and a claim ceiling.

Adapters should map this contract to the existing Sol-Atlas request/report contracts rather than silently reinterpret their fields. Carrier kinds include electricity, thermal heat, chemical fuel, mechanical work, radiant energy, useful water service and mobility service. The last two are services, not energy forms: do not equate litres of water or vehicle-kilometres with kWh without an explicitly defined conversion boundary.

## First end-to-end design

Use one small synthetic service hub—not all energy technologies at once:
- electric essential and flexible loads;
- a heat demand with explicit minimum delivery temperature;
- an essential water/communications service contract;
- PV, grid/import or generator, one electrical or thermal store and one explicit conversion device;
- a short, contiguous chronology with resource variation and an outage;
- at least three different feasible/infeasible architecture candidates.

The same input digest, time base, scenario, terminal reserve and information boundary must be passed to every baseline. Keep the whole test offline and deterministic.

### Initial known-answer and negative controls

1. Recover the frozen Sol-Atlas PV-surplus → storage → outage service known answer.
2. Reject an energy-port connection with mismatched carrier or incompatible heat-temperature grade.
3. Reject hidden energy creation, over-capacity storage, impossible efficiency, omitted critical service, and grid import during a flagged outage.
4. Reject a design whose only "useful heat" is below the declared service temperature.
5. Preserve a failure report when Symthaea produces an invalid candidate.
6. Verify that adding unsupported or stale cost data cannot improve a candidate merely because a missing field became zero.
7. Repeat the run and compare byte-stable receipts; independently recalculate all invariants from raw trajectory samples.

## Benchmarks and targets

These are release targets, not measured results:

- 100% hard-constraint compliance for every result labelled feasible.
- 100% recovery of frozen known-answer fixtures and 100% detection of designated negative controls.
- At least 10,000 deterministic candidate/scenario cases before publishing aggregate ranking claims.
- Report candidate validity rate, architecture diversity, Pareto coverage, essential unserved energy, longest critical-service deficit, outage recovery time, resource curtailment, cost, carbon, water, material/logistics burden where evidence exists, runtime, peak memory and measured energy-to-compute where practical.
- Separate infeasibility rate, model discrepancy, forecast error and optimization regret. Do not hide critical-service failure in a single weighted score.
- Use frozen site/temporal/episode holdouts. Do not tune a generator and report only the same cases it saw.
- Include at least a random/diverse baseline, transparent rule-based baseline and established optimizer on matched subproblems.
- Track units, model revision, source digests, quantization, tuning/compute budget and exact comparator status for every run.
- No better-than-SOTA or compliance claim before a public, reproducible, matched evaluation supports it.

## Industry comparison / reference lanes

Use established software as independent comparators when the same model can be represented:
- [NREL System Advisor Model (SAM)](https://sam.nlr.gov/) supports multiple generation technologies and storage. Its battery model covers voltage, temperature and degradation behavior; align only variables whose parameters are genuinely mappable.
- [REopt](https://developer.nlr.gov/docs/energy-optimization/reopt/) provides site and microgrid techno-economic sizing/dispatch; compare identical demands, tariffs, assets, outage windows, constraints and terminal SOC.
- [PyPSA](https://docs.pypsa.org/latest/) covers broader network and multi-energy optimization, including intertemporal storage; report time-step, topology, objective and solver differences.
- [GenX](https://genxproject.github.io/GenX.jl/stable/User_Guide/workflow/) is a later comparison for system planning and dispatch questions, not a proxy for a tiny single-site twin.
- [NREL controller hardware-in-the-loop methodology](https://www.nrel.gov/docs/fy20osti/71338.pdf) offers a later controller qualification path; simulation-only tests must not be described as HIL.
- [IEEE 2030.8-2018](https://standards.ieee.org/ieee/2030.8/6169/) is an active published microgrid-controller testing standard. IEEE's [P2030.8 revision project](https://standards.ieee.org/ieee/2030.8/12543/) was approved March 26, 2026 and is an active project, not an approved replacement. Do not claim compliance with draft requirements.

External results should be imported as versioned comparators with their assumptions and known model differences. Numerical disagreement should be classified as input/translation mismatch, implementation bug, numerical error, or genuinely different physical model—not automatically hidden by retuning.

## Qualification ladder

1. **Schema and analytic fixtures:** exact units/known answers, deterministic offline tests.
2. **Property/negative-control tests:** independent conservation/state/service re-audit, bounded generated cases.
3. **External model cross-validation:** SAM battery trajectories and matched optimization subproblems.
4. **Software-in-the-loop:** controller/model exchange on frozen chronological scenarios.
5. **Controller/power hardware-in-the-loop:** only on appropriate laboratory equipment, with expert-reviewed protocol.
6. **Field trial and independent audit:** only after separate electrical/process safety, protection, cybersecurity and regulatory engineering approvals.

Passing one stage does not imply passing the next.

## Stop conditions

Stop ranking or return an explicit unknown/invalid state when units, chronology, input evidence, interface mapping, balance, model boundary or result identity cannot be checked. Do not convert a missing datum to zero, silently extrapolate a component outside its tested range, or use a successful optimizer exit code as proof of feasibility.

The first milestone is deliberately small: agree the shared contract, connect the current Sol-Atlas exact oracle, and run three synthetic design alternatives through independent re-audit and deterministic Symtropy trajectories. Only expand the architecture space after those gates produce reproducible evidence.
