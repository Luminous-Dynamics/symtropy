# OLI collective competence benchmark v0

**Status:** task-fitness instrumentation added; benchmark contract frozen; sweep not yet executed  
**Owner:** Symtropy / SCM reference and OLI experiment-selection adapter  
**Machine-readable contract:** benchmarks/oli/collective-competence-scm-v0.json  
**Related:** [OLI experiment-selection adapter, Symthaea #11446](https://github.com/Luminous-Dynamics/symthaea/issues/11446)

## Paradigm shift: coordination is not competence

OLI must not infer intelligence or useful collective capability from:
- high global polar order;
- high local polar order;
- short graph path lengths;
- high consensus;
- high mutual information or transfer entropy;
- a large number of inferred interaction edges.

These may describe organization, predictability or communication structure. They do not show that the group accomplishes a useful task.

Collective competence is measured only against a registered external objective, under a fixed resource budget, with matched independent-agent, random-search and centralized controls.

The inferential quantities remain separate:
1. correlation;
2. predictive information transfer;
3. declared/model-defined interaction structure;
4. interventional causal effects;
5. objective task performance.

## New SCM task observables

The SCM step report now exposes:
- best fitness among the current agents;
- index of the best agent (ties deterministically resolve to the lowest index);
- mean current-agent fitness;
- population variance of current-agent fitness;
- best fitness ever observed, including the initial acquired positions;
- consensus and communication metrics in separate fields.

The model retains best-seen fitness across updates. If a later population loses its previous best candidate, the best-so-far objective record remains monotonic. This does not make the swarm elitist—the objective record is an audit statistic, not a replacement agent that silently re-enters the dynamics.

The objective-evaluation accounting contract is explicit. For N agents and T complete updates, N initial measurements plus N successor measurements per update yield exactly N(T+1) function evaluations. The source cache means later updates reuse the previous observation rather than re-measuring it.

## Frozen landscapes

### Unimodal bounded quadratic

[
f(x,y)=1-(x-0.6)^2-(y-0.6)^2
]

on ([0,1]^2), maximized at (0.6, 0.6) with exact fitness 1. This gives analytic simple regret and cost-to-target.

### Multimodal periodic landscape

[
f(x,y)=0.5+0.25cos(4pi x)cos(4pi y)
]

on ([0,1]^2), with exact global fitness 0.75 and multiple optima. This challenges premature consensus and recovery from suboptimal basins.

The machine-readable manifest records both objective equations, domains, gradients, and exact reference values.

## Baselines to compare

Under the same initial positions, seeds and objective-evaluation budget:
- SCM complete graph (published topology baseline);
- SCM ring;
- SCM metric-radius local graph;
- SCM explicit NoCommunication topology (empty adjacency under all states; independent local updates);
- independent parallel gradient descent (separate adapter);
- random search with the same number of objective evaluations;
- centralized gradient reference at the same evaluation budget;
- analytic exact optimum as an oracle, not an operational optimizer.

PSO and MIPA should be included as matched external baselines once the source tasks, initialization, stopping rules and evaluation accounting have been pinned. Nitti et al. (2025) report SCM success-rate parity or better than PSO/MIPA on 22 of 33 evaluated low-dimensional tasks for M≤16, but with roughly one order of magnitude more objective evaluations in aggregate. The paper also reports 86% success over 100 simulated five-AUV contaminant-localization trials; this is a computational proof of concept, not physical fleet validation. These results are comparators, not proof that SCM is generally superior.

## Primary metrics

**Task:** best-so-far fitness, current best fitness, mean and variance of population fitness, simple regret, success rate at a registered target, evaluations-to-target, and anytime best-fitness area.

**Social structure:** consensus, communicating neighbours, isolated agents, distance to local centroid, graph/communication statistics.

**Cost:** objective evaluations, cumulative message/edge count, latency and energy when genuinely measured.

**Robustness:** paired-seed results, initialization sensitivity, parameter/noise sensitivity, held-out landscapes, recovery after topology changes.

**Fairness:** quantile spread and per-agent performance distribution; a single high-performing agent must not automatically be reported as a uniformly competent collective.

No single scalar score should hide success rate, regret, communication cost or resource violations.

## Experimental protocol

- Freeze 100 independent seeds before the sweep; 30 is only a pilot minimum.
- Reuse the same initial swarm and seed in each communication-topology arm.
- Use exactly N(T+1) function evaluations for N agents and T complete SCM steps.
- Keep all frames and updates of one run within the same statistical unit.
- Report paired seed-level differences, 95% confidence intervals, failures and outliers.
- Do not tune communication radii or SCM parameters on the sealed test seeds or landscape.
- Report the selected SCM equation kernel and consensus-window convention with every run.

## Adversarial controls

1. **High consensus, low fitness:** a tightly clustered swarm in a suboptimal basin receives no competence credit for consensus.
2. **Common driver, no communication:** an exogenous common field can synchronize agents without messages or direct influence.
3. **Communication ablation:** complete, ring, local-radius and isolated topologies at equal objective measurement budgets.
4. **Deceptive multimodality:** test premature consensus and recovery from local optima.
5. **Budget overspend:** any run exceeding the registered function-evaluation budget is inadmissible, regardless of fitness.

## Qualification rule

A topology is credited with a collective task advantage only if:
- it beats the matched no-communication baseline on task utility at equal objective-evaluation cost;
- the paired-seed uncertainty interval excludes zero;
- no hard budget is exceeded;
- the result survives the held-out landscape and seed set;
- communication cost and any latency/energy overhead are reported.

The benchmark deliberately does not presume communication helps. Depending on landscape, information structure and task, communication may help, hurt, or merely increase coordination without improving objective performance.

## Current implementation status

The independent SCM reference now reports current population task fitness and best-seen fitness separately from consensus. A Rust unit test compares complete-graph, ring and no-communication topologies under the same initialization and exact objective-evaluation count. These are code-level controls only; no performance sweep or superiority result has been run yet.


### Independent-control topology hardening

The code now has an explicit `ScmCommunicationTopology::NoCommunication` variant. It always produces empty adjacency, including when two agents occupy exactly the same coordinates. The benchmark must not substitute `MetricRadius { radius: 0.0 }` for this control: a zero-distance pair can satisfy a zero-radius metric rule and accidentally create an edge. The explicit no-communication variant is the canonical independent-agent arm.
