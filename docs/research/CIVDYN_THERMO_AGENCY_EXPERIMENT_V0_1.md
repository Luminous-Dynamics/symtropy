# CIVDYN Thermodynamic Viability and Multi-Agent Normativity Experiment V0.1

**Status:** research/experiment specification; no new coupled experiment result claimed  
**Owner:** [Symtropy #4014 — CIVDYN-THERMO-AGENCY-001](https://github.com/Luminous-Dynamics/symtropy/issues/4014)  
**Normativity bridge:** [Symthaea #10852](https://github.com/Luminous-Dynamics/symthaea/issues/10852)  
**Parent simulation programme:** [CIVDYN-001 #3390](https://github.com/Luminous-Dynamics/symtropy/issues/3390)

## 1. Answer to the lineage question

Yes: the project has already used Symtropy and multi-agent simulation in two distinct ways. They must not be conflated.

### A. Socio-economic / collective-action agents

- [CIVDYN-CRG-001 #3606 / PR #3609](https://github.com/Luminous-Dynamics/symtropy/pull/3609) implements six seats over ten rounds, six inspectable scripted policies, an analytical expected-payoff pivot, and exhaustive best-response enumeration against fixed scripted partners. Its exact-head workflow passed at `dbe1e16650e17179c83d72d7c21dea6bd7ac617e`, run [41882797326](https://github.com/Luminous-Dynamics/symtropy/actions/runs/41882797326).
- [CIVDYN-SWF-001 #3610 / PR #3614](https://github.com/Luminous-Dynamics/symtropy/pull/3614) implements exact small-instance allocation policies, hard service floors, and persuasion/floor-blind negative controls. Its exact-head workflow passed at `561457375be8ddcb537cf39975440860f92415bb`, run [41883322246](https://github.com/Luminous-Dynamics/symtropy/actions/runs/41883322246).
- [CIVDYN-RES-001 #3621 / PR #3666](https://github.com/Luminous-Dynamics/symtropy/pull/3666) implements a deterministic basin → district → household stock-flow shock/recovery fixture.
- [CIVDYN-POP-001 #3646 / PR #3648](https://github.com/Luminous-Dynamics/symtropy/pull/3648) adds a separate finite-population imitation/fixation model; it must be validated independently of the CRG parent.

These are valid executable reference tasks and simulations. The scripted policies are not production Symthaea cognitive agents, and these results do not reproduce the human/LLM experiments that motivated the tasks.

### B. Embodied interacting entities in the consciousness-physics simulation

The existing `symtropy-consciousness-physics` module contains a `ConsciousnessField` with a `BTreeMap<BodyHandle, EntityConsciousness>`, per-entity energy budgets, a shared harmony field, optional Φ-gravity, collision-feedback coupling, emotional-contagion updates and a collective-Φ summary. The README reports multi-entity operation benchmarks, including 16-agent full-step timing.

**Source-level qualification:** `AgentMemory` declares partner-history storage and a `record_partner` method, but the inspected `coupling.rs` has no call site for that method. The `harmony_resonance_regen_rate` parameter is declared, but no partner-to-partner energy-transfer implementation was established by this audit. The `resource_regeneration_multiplier()` method exposes a value based on aggregate Φ; that alone does not prove the simulation applies per-agent partner transfers. Do not claim partner-history learning or resonance-funded mutual aid as implemented until the concrete call path and tests are inspected.


This is a second multi-agent substrate: embodied entities interact through motion, collisions, harmony/resonance and configured energy rules. It is not the same experiment as the six-seat collective-risk task.

**Crucial physical boundary:** the current `EnergyBudget` and `ThermodynamicLedger` are configured simulation/resource rules. The current ledger records selected input/output counters but not a complete first-law closure. The current `consume()` entropy update and `dissipate_heat()` relation do not establish general physical entropy production. PR #3580 correctly narrows the Landauer claim to logically irreversible bit erasure. See [PHYS-THERMO-SEMANTICS-001 #3577](https://github.com/Luminous-Dynamics/symtropy/issues/3577), [PR #3580](https://github.com/Luminous-Dynamics/symtropy/pull/3580), and the module README.

Therefore the project has already run multi-agent simulations with energy-like resource dynamics, but it has **not yet validated one integrated thermodynamic-to-morality experiment**. This specification defines that next experiment without retroactively upgrading the claim ceiling of existing work.

## 2. Core research question

For multiple autonomous agents whose decisions affect their own and others' access to scarce resources:

1. Can the model predict individual and group viability under declared resource/energy constraints?
2. How do cooperation, reciprocity, stewardship, exploitation and rights-constrained care change those trajectories?
3. Do policies generalize to unfamiliar partners, population mixtures and shocks?
4. Which conclusions are physical/causal, which are agent-relative prudential evaluations, and which depend on additional normative premises?
5. Can the same physical trajectory yield different conditional moral conclusions under different explicit premise sets without treating one as a physical fact?

This is a simulation research programme, not a claim that physics alone derives an ought.

## 3. Reuse two simulation substrates, not one premature fusion

| Existing substrate | What it can currently support | What it cannot establish by itself |
|---|---|---|
| `civdyn-resource-ledger` | Native-unit water/energy/food stock-flow, transfers, demand, service and conservation | Thermal state, entropy production, exergy, moral worth |
| `civdyn-collective-risk` | Six-seat strategic interaction, expected-payoff reference, fixed-partner best response | Physical resource dynamics, human/LLM fidelity |
| `civdyn-cross-scale-resilience` | Basin/district/household shock and recovery trajectories | Agent strategic adaptation or empirical forecast validity |
| `civdyn-welfare-allocation` | Exact constrained allocation, floor violations, return/equity trade-offs | Universal welfare or justice ordering |
| `symtropy-consciousness-physics` | Interacting embodied entities, partner history, harmony-field coupling and model energy budgets | Complete physical energy/entropy closure, consciousness or moral patienthood |

The first integration should pass frozen scenario/run records and independently read outcome receipts. It should not merge the engines or convert a model-energy counter into a physical thermodynamic observable.

## 4. Experiment ladder

### E0 — reproduce existing references

Before adding new coupling, preserve exact baseline outputs from:
- CRG analytical pivot and fixed-partner best-response controls;
- exact welfare-allocation oracle and floor/rumour negative controls;
- resource-ledger conservation and cross-scale shock/recovery controls;
- consciousness-physics multi-entity coupling microfixtures, with current model-only labels.

Pin exact source revisions and output digests. Do not rely on the fact that a parent PR passed to qualify a later child.

### E1 — six-agent resource-sharing world

Use two abstract communities with three agents each, a shared resource basin and a 10–30 tick horizon. The first world is synthetic and deliberately small.

State variables:
- agent-specific water, energy and food access in native units;
- community and basin stocks, production/regeneration and transfers;
- essential service demand and declared floors;
- one infrastructure capacity/maintenance state;
- exogenous shock state and observation history;
- agent action, information and partner-history records.

Policies, each with a transparent, inspectable rule:
1. self-regarding payoff maximization;
2. fixed solidarity;
3. conditional reciprocity;
4. commons stewardship with a reserve rule;
5. exploitative cost externalization (adversarial control);
6. rights-constrained care, which may assist but does not override recipient refusal or exit.

Use matched initial conditions and common random numbers for stochastic shocks. Keep each agent's actions and outcomes; do not report only population averages.

### E2 — cross-scale shocks and recovery

Compose with the existing cross-scale resource ledger:
- drought / reduced renewable inflow;
- energy interruption;
- compound water + energy shock;
- infrastructure failure and maintenance backlog;
- repeated shock before reserve recovery.

Report service recovery and reserve recovery separately. A community can regain service temporarily by exhausting its reserve; that is not sustainable recovery.

### E3 — population and partner generalization

Separate interaction-group size from total population size. First targets: group sizes 2 and 6; total populations N=6, 30 and 100, then 300 where computationally justified.

Hold out:
- partner policy types;
- community compositions;
- network topology;
- group sizes;
- resource scarcity / shock distributions;
- incentive and information regimes.

Use the exact hypergeometric / finite-population references where appropriate. Treat imitation/fixation as a separate evolutionary model from an agent's within-episode action policy.

### E4 — embodied multi-agent comparison

Run a distinct Symtropy consciousness-physics lane:
- isolated-agent baseline;
- interaction with one unfamiliar partner;
- multiple familiar/unfamiliar partners;
- no-resonance / random-resonance / current harmony-resonance model controls;
- assistance/recovery after one entity becomes depleted;
- matched movement, collision and energy-demand workloads.

The current harmony-resonance resource rule is a simulation intervention. This experiment tests its model consequences, not a physical energy-transfer law or evidence of consciousness/love.

### E5 — normative premise sensitivity

Freeze the physical world, observation stream and action/outcome receipts. Evaluate them under separately versioned premise sets:
- self-regarding prudence;
- aggregate-consequence evaluation;
- equal consideration / avoidable-harm constraint;
- consent and non-domination constraints;
- reciprocity / contractualism;
- intergenerational stewardship;
- pluralist profile with explicit unresolved conflicts.

Then hold a premise set fixed and vary physical facts. Any moral conclusion must trace to explicit normative premises and applicable empirical claims. The evaluator must report underdetermined or inconsistent cases rather than force a winner.

## 5. What counts as thermodynamics in this experiment

### Phase 0: physically dimensioned resource accounting

The resource ledger handles water (m³), energy stock (kWh) and food (kg) as separate stocks/flows. It is the physical-resource layer for the initial CIVDYN experiment. Its energy stock is not equivalent to heat, temperature, entropy production or exergy.

### Phase 1: physical thermodynamics only after qualification

Do not add an entropy-production score until the #3577 physical semantics gate is complete. A future thermal sub-study must bind:
- system boundary and reservoirs;
- stored-energy changes;
- external work and heat;
- inter-reservoir transfers;
- named dissipation destinations;
- valid state/process assumptions and temperature measurements;
- independent analytic controls and uncertainty.

Landauer applies only to identified logically irreversible erasures. FLOPs, prediction error, action choice, Shannon entropy and Φ are not erased bits by default. Keep Φ-coupling as a model hypothesis until independent validation.

## 6. SOTA and industry comparison

### Direct multi-agent comparators

- **Huynh et al. (2026), Cooperation Without Adaptation:** source-matched six-seat collective-risk task; risk adaptation, partner response and population composition. https://arxiv.org/abs/2610.22882
- **CoopEval (ICML 2026):** repetition, reputation, mediation and outcome-contingent contracts across social dilemmas. Its reported finding that repetition-induced cooperation deteriorates under varying partners makes held-out partner rosters essential. https://proceedings.mlr.press/v306/tewolde26a.html
- **Willis et al. (2026), Evaluating Collective Behaviour of Hundreds of LLM Agents:** inspectable strategy programs, public-goods, collective-risk and common-pool resource tasks at populations of hundreds. https://arxiv.org/abs/2602.16662
- **DeepMind Melting Pot:** held-out social situations and unfamiliar partners for multi-agent generalization. It is a complementary social-interaction benchmark, not a physical resource simulator. https://github.com/google-deepmind/meltingpot
- **Collective cooperation without individual fidelity (2026):** direct empirical human-network comparison showing macro/micro behavioral mismatch in LLM social-agent simulations. https://arxiv.org/abs/2606.30454
- **Social Welfare Function Leaderboard (ACL Findings 2026):** tests efficiency/inequality trade-offs and persuasion sensitivity in LLM allocation. https://aclanthology.org/2026.findings-acl.1919/

### How to do better

Do not chase a higher cooperation score. Seek a stronger *decision and generalization contract*:
1. couple resource stocks and strategic decisions without violating conservation;
2. measure who benefits and who bears costs, including worst-served groups;
3. test individual actions and population-level equilibria separately;
4. require adaptation to unfamiliar partners and changing stakes;
5. test stability-is-not-legitimacy and persistence-is-not-good countermodels;
6. preserve explicit normative disagreement;
7. keep physical model qualification and moral conclusion qualification separate.

A SOTA claim requires reproducing a source task exactly before claiming superiority. Otherwise the defensible claim is complementary coverage: physical/resource constraints plus multi-agent partner/population generalization and premise-traceable evaluation.

## 7. Required metric vectors (no composite morality score)

- **Physical/resource:** per-unit conservation residual, stock depletion, regeneration, unmet essential demand, service-floor duration, reserve and maintenance backlog.
- **Individual:** payoff distribution under declared objective, agent-specific viability duration, recovery, assistance received/given, choice/exit and action burden.
- **Collective:** group service, lower-decile/worst-group outcomes, inequality, externalized costs, collective-risk success, repeated-shock recovery.
- **Strategic:** best-response gap, risk adaptation, partner-conditional performance, exploitability and population-mixture sensitivity.
- **Normative:** explicit premise traceability, framework-switch fidelity, hidden-premise detection, underdetermination, consent/rights violations and same-physics/different-premise controls.
- **Computational:** CPU/GPU, memory, wall-clock, runs/second, scaling with N and full reproducibility receipts.

No aggregate may conceal a hard service-floor, consent or resource-conservation failure.

## 8. Qualification targets (prospective, not results)

- Exact agreement with hand-calculated two-agent and six-seat controls.
- Zero unexplained native-unit stock/flow residuals beyond preregistered numerical tolerance.
- Zero hard resource/floor violations by policies advertised as constrained.
- At least five held-out partner/population compositions before a social-generalization claim.
- N=6, 30 and 100 population sweep, with interaction group size independently varied.
- At least 30 independent seeds per initial stochastic confirmatory regime; increase where confidence intervals require it.
- All failed, censored and nonterminating runs retained in the denominator.
- No physical entropy-production claim until #3577 is qualified.
- No SOTA or moral-superiority claim without exact source protocol, matched budgets and independent reproduction.

## 9. First executable child

[**CIVDYN-CRG-RESOURCE-ADAPTER-001 #4022**](https://github.com/Luminous-Dynamics/symtropy/issues/4022) is the first code implementation: replay the existing six-seat collective-risk action trace through the existing resource ledger using a frozen synthetic contribution-unit-to-kWh mapping, then independently report donor and shared-service outcomes. It is an open-loop adapter; it does not create a second agent simulator, feed ledger state back into policy selection, or claim physical thermodynamics. This should be completed and independently validated before attempting closed-loop agent/resource co-adaptation.

## 10. Exit criteria

1. Exact-head reference output census captured for all consumed parent kernels.
2. Frozen six-agent manifest and explicit policy/observation/action contracts.
3. Deterministic resource-coupled run with independent ledger-read outcome receipts.
4. Negative controls for conservation, externalities, manipulation, coercion, exit and service-floor regressions.
5. Held-out partner/population design and an independent evaluation protocol.
6. Normative premise traces and same-physics/different-premise tests.
7. Physical thermodynamic quantities withheld unless their own measurement/model gates pass.

## References

1. Huynh et al. (2026). [Cooperation Without Adaptation: LLM Agents in a Collective-Risk Dilemma](https://arxiv.org/abs/2610.22882).
2. Tewolde et al. (2026). [CoopEval](https://proceedings.mlr.press/v306/tewolde26a.html).
3. Willis et al. (2026). [Evaluating Collective Behaviour of Hundreds of LLM Agents](https://arxiv.org/abs/2602.16662).
4. [DeepMind Melting Pot](https://github.com/google-deepmind/meltingpot).
5. Ferraz de Arruda et al. (2026). [Collective cooperation without individual fidelity in LLM agents](https://arxiv.org/abs/2606.30454).
6. Shi et al. (2026). [Social Welfare Function Leaderboard](https://aclanthology.org/2026.findings-acl.1919/).
