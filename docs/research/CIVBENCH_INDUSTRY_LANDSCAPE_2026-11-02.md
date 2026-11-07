# CivBench Industry and Research Infrastructure Landscape

**Date:** 2026-11-02  
**Status:** Comparative strategy; this is not a benchmark result or ranking.  
**Purpose:** Identify mature infrastructure to reuse, the gaps CSS can address, and a fair path to demonstrate superiority on bounded tasks.

## 1. Strategic position

Civilizational Systems Science should not set out to replace every agent-based modelling environment, historical database, climate-impact intercomparison, or integrated-assessment scenario platform.

Those systems already solve important parts of the problem. Rebuilding them would waste effort and create a weak comparison target.

The defensible ambition is to create an **evidence-preserving composition and evaluation layer** that connects:
- source-grounded historical and contemporary knowledge;
- physical, ecological, demographic, economic, institutional and cultural mechanisms;
- versioned scenarios and exact run artifacts;
- causal intervention tests and uncertainty;
- cross-context and transfer evaluation;
- evidence-backed spatial and planetary projections.

The unit of competition must be a named scientific task, not "the whole industry."

## 2. Existing platforms and what they already do well

### Mesa

Official documentation: https://mesa.readthedocs.io/stable/

Mesa is an Apache-2.0 Python framework for agent-based modelling. Its current stable documentation describes built-in modelling components, AgentSet-based agent management, browser-based Solara visualization, data collection/analysis tools and example models.

**Strength:** accessible framework for constructing, instrumenting, visualizing and analysing agent-based models.

**Not implied:** a built-in theory or validation of civilization, a standard historical evidence ontology, or automatic physical conservation across domains.

**CSS response:** do not compete on a generic agent scheduler. Compare against Mesa implementations for bounded agent tasks; make the scientific model and evidence contract portable so a Mesa model could be used as an external baseline or data producer.

### GAMA

Official platform and version history:
- https://gama-platform.org/wiki/next/Home
- https://gama-platform.org/wiki/next/OlderVersions
- https://github.com/gama-platform/gama

GAMA is an open-source, spatially explicit agent-based modelling and simulation environment. Its documentation lists urban mobility, climate adaptation, epidemiology, disaster evacuation and urban planning among use cases. The documentation reports the 2026-06 release as current on the date of this audit. GAMA emphasizes spatial models, extensibility, plugins, interoperability and headless/server use.

**Strength:** spatially explicit agent-based modelling, GIS use cases and a mature modeller-facing environment.

**CSS response:** reuse spatial inputs and compare against GAMA for bounded regional or settlement experiments. CSS can contribute evidence provenance, chronology uncertainty, institutional causality, multi-domain stock-flow checks and common benchmark manifests around those models.

### NetLogo

Official documentation: https://docs.netlogo.org/

The official documentation describes NetLogo as a programmable environment for simulating natural and social phenomena. The current documentation identifies NetLogo 7.0.4 (April 2026); its release notes describe BehaviorSpace export recording the NetLogo version used.

**Strength:** accessible, established multi-agent modelling and experiment tooling; useful for transparent baseline models and teaching.

**CSS response:** include NetLogo reference models where a compact, interpretable ABM is more appropriate than a high-complexity Rust or cognitive model. Do not assume our stack is superior just because it is lower-level or faster.

### Seshat Global History Databank

Official data portal: https://seshatdatabank.info/data

Seshat provides historical variables and polity data through article replication datasets, curated data snapshots and an API. Its data portal identifies Polaris-2026 as the latest snapshot at the time of review and describes scripts for comparing it with an earlier release.

**Strength:** historical data curation and replication materials for comparative historical research.

**Limit:** variable coverage, temporal resolution, coding choices and representativeness depend on the curated data and individual research questions.

**CSS response:** consume and cite Seshat data rather than rebuilding its curation. Add explicit uncertainty, missingness, spatial/temporal intervals, alternative interpretations and mechanistic tests. Treat the data as a sample of coded history, not the totality of civilization.

### D-PLACE

Official data source guide: https://d-place.org/source  
About: https://d-place.org/about

D-PLACE connects cultural descriptions to language, time, geography and environmental context, and links coded observations to primary sources and coding comments. Its documentation specifically warns users to consider intracultural variation, uncertainty and the original sources behind cultural codes.

**Strength:** cross-cultural comparison with time, place, language and source context.

**CSS response:** use D-PLACE as a key source for culture/environment questions, but retain coding comments, source lineage, temporal focus, spatial focus and uncertainty. Never treat cultural codes as immutable traits of populations.

### World Values Survey / Integrated Values Surveys

Official time-series documentation: https://www.worldvaluessurvey.org/AJDocumentationSmpl.jsp?CndWAVE=-1&INID=&SAID=-1  
WVS/EVS trend data: https://www.worldvaluessurvey.org/WVSEVStrend.jsp

The WVS V3.0 time-series file covers waves from 1981–2022. The official documentation explicitly states that it is a repeated cross-sectional time series, not a panel following the same respondents. The WVS/EVS trend data provide repeated cross-national questions over a long period.

**Strength:** repeated cross-national measures of values and attitudes.

**Limit:** not a person-level longitudinal panel; countries, sampling designs, questions, translations and response structures require careful harmonization.

**CSS response:** model country/wave aggregates only where justified, preserve survey weights and methodology, and avoid claiming individual-level belief change from repeated cross-sections.

### ISIMIP

Official project: https://www.isimip.org/  
Protocols: https://www.isimip.org/protocol/  
ISIMIP4 protocol: https://protocol4.isimip.org/

ISIMIP provides common simulation protocols, forcings and output structures for inter-sectoral climate-impact model comparison. The project reports that ISIMIP4 was launched in August 2026 with the first forcing data and related protocol, and that ISIMIP4a/4b cover model evaluation and future climate-impact experiments.

**Strength:** structured intercomparison of climate impacts across sectors, scales, forcing datasets and experiments.

**CSS response:** do not write a replacement climate-impact model. Use ISIMIP-compatible data/outputs as an external environmental forcing or comparison source. CSS's additional hypothesis is how physical impacts connect to institutions, migration, trade, access and collective adaptation.

### IAMC and IIASA SSP scenario infrastructure

IAMC scenario databases: https://www.iamconsortium.org/resources/databases/  
IIASA SSP data: https://iiasa.ac.at/models-tools-data/ssp

The IAMC has coordinated collections of quantitative integrated-assessment scenarios used in IPCC and related policy assessments. IIASA's SSP database links population, economic development, technology and related socioeconomic assumptions with climate pathways. The IIASA portal reports updated GDP and population projections released in 2025 and further IAM quantifications under development during 2026.

**Strength:** structured, quantitative, cross-model scenario comparison for long-run socioeconomic and climate pathways.

**CSS response:** use SSP/IAMC scenarios as scenario families and boundary conditions, not as empirical forecasts or a single true future. Add local, institutional, cultural and distributional mechanisms while maintaining compatibility with shared scenario identifiers and units.

## 3. Comparative capability map

| Capability | Mature external infrastructure | CSS contribution |
|---|---|---|
| Generic ABM execution | Mesa, GAMA, NetLogo | Mechanism-specific models and portable task contracts |
| Spatial ABM / GIS | GAMA and geospatial toolchains | Historical location/boundary uncertainty plus causal spatial-resource links |
| Comparative historical variables | Seshat | Source lineage, time intervals, competing interpretations, and mechanism validation |
| Cross-cultural variables | D-PLACE, WVS/EVS | Situated cultural data and transfer tests with uncertainty; not cultural ranking |
| Climate-impact intercomparison | ISIMIP | Coupling climate impact to institutions, access, migration, trade and adaptation |
| Long-run socioeconomic scenarios | IAMC / IIASA SSP | Explicit institutional/cultural mechanisms and distribution-sensitive outcome vectors |
| Run provenance and evidence state | Existing Humanity Atlas contract and Mycelix evidence-plane work | Cross-project scientific evaluation receipts and independent re-execution |
| Cognitive / cultural transmission | Specialized cognitive and social-learning research | Falsifiable tests showing whether cognitive mechanisms add value over simple baselines |
| Historical/planetary visual exploration | Sol-Atlas / GIS / digital-twin systems | Observed vs inferred vs scenario display with source/time/space uncertainty |

This table is an architectural positioning, not a claim that every external platform lacks the CSS capability. Where capabilities overlap, benchmark the same task and compare.

## 4. The real research gaps CSS can own

### 4.1 Provenance continuity across model transformations

The source identity, source version, transformation, unit, uncertainty and time semantics must survive conversion from a historical dataset to an indicator, from an indicator to a model parameter, and from model input to scenario output.

A SHA-256 digest establishes byte identity. It does not authenticate a publisher, guarantee data accuracy, establish independence, or prove scientific truth.

### 4.2 Causal-path and mechanism qualification

A model should expose the path from an intervention to independently measured outcomes. The multiworld simulator's Phase 2 null result demonstrates why this matters: intervention-internal variables moved, but the stated aggregate outcome inputs did not. CivBench should identify disconnected pathways and preserve null results.

### 4.3 Physical closure coupled to social outcomes

Test mass/energy/material/water/population accounting independently from the social model, then test whether the social/institutional layer changes resource use and distribution through declared processes. Do not let a single aggregate welfare score hide resource depletion, unequal exposure or unaccounted flows.

### 4.4 Pattern-oriented multi-scale validation

A civilizational model should reproduce a pre-registered vector of spatial, temporal, distributional, institutional and physical patterns for its intended use—not just a single aggregate curve. Pattern selection, thresholds, source dependencies and acceptable discrepancy measures must be declared before fitting.

### 4.5 Cross-context transfer without universalizing local history

Hold out sites, source families, regions, periods, ecologies or institutional forms. A model that explains one case may not explain another. Failed transfer is a result, not a reason to add ad hoc parameters until the model fits.

### 4.6 Crisis-tail calibration

Ordinary calibration does not guarantee reliable estimates of low-frequency, high-impact tail events. Include exceedance and excess-severity calibration for crisis loss, service interruption and ecological thresholds when data support it. Report event rarity and confidence intervals; do not overclaim with small samples.

## 5. Where to aim higher than current practice

The following are proposed targets, not attained performance:

1. **Benchmark interoperation:** one shared manifest can wrap an existing Symtropy, Mesa, GAMA, NetLogo, ISIMIP or external scientific model output without erasing its native model authority.
2. **Exact provenance:** every scored run pins model commit, toolchain, resolved config, source datasets, transforms, seeds, evaluation split and output digests.
3. **Independent verifier:** a second implementation checks physical invariants, artifact binding and score calculation rather than trusting the producer.
4. **No hidden leakage:** historical origins, source publication cutoffs and spatial/source-family holdouts are machine-checkable.
5. **Matched-baseline competition:** complex models must beat simple statistical and fixed-rule alternatives on the identical task and budget.
6. **Mechanism evidence:** every added mechanism has positive controls, negative controls, ablations and a measured effect on a predeclared outcome vector.
7. **Distribution and power:** report who gains, who bears costs, access, autonomy and concentration, not only aggregate output.
8. **Open negative results:** null and failed models remain in the evidence record with explanations and reproducible runs.

## 6. How to claim superiority responsibly

A result may claim improvement only when it names:
- the exact benchmark task and version;
- the strongest relevant baseline(s);
- identical source snapshots and evaluation cases;
- matched compute/runtime budgets where relevant;
- the frozen holdout and source/time leakage controls;
- proper scoring rules and confidence intervals;
- calibration, tail behavior, and transfer performance;
- ablations showing which mechanism contributed;
- an independent exact-artifact rerun.

"Better than SOTA" is not a valid claim across a field. A defensible claim is narrow, for example:

"On HIST-001D v0.1, for the pre-registered held-out site/phase set, model family A improves the specified chronology/occupation pattern score over baseline family B by X, with interval Y, under the same source cut-off and compute budget; it does not transfer to case C."

If no credible baseline can be beaten, publish that. If the benchmark is too small to distinguish models, improve the task or collect more evidence rather than expanding the claim.

## 7. Immediate implementation consequence

CivBench should remain a task and evaluation layer over the existing Humanity Atlas run contract and multiworld simulator.

Do not create a second generic:
- agent-based runtime;
- evidence ledger;
- scenario/run envelope;
- historical database;
- climate model;
- spatial atlas.

The next technical milestone is an adapter/qualification matrix showing which existing external data/model systems can be used as sources, forcing inputs, baseline models, or independent validators, with exact version and license rules.


## 8. Collective intelligence, social generalization and cultural transmission (4 November 2026)

The landscape above needs one important addition: collective intelligence and social generalization are now an active benchmark area in their own right. This strengthens the case for composing established suites, not replacing them.

### DeepMind Melting Pot — social generalization

- Official repository: https://github.com/google-deepmind/meltingpot
- Research overview: https://deepmind.google/blog/melting-pot-an-evaluation-suite-for-multi-agent-reinforcement-learning/

Melting Pot evaluates agents on unfamiliar social situations, with current repository documentation describing more than 50 substrates and over 256 test scenarios. Its scenarios cover cooperation, competition, deception, reciprocity, trust and related social interactions.

**CSS/CivBench response:** adopt held-out partner strategies, social compositions and scenario splits. Add resource, institutional and cultural-transmission mechanisms around those tasks. Do not claim that a custom social-dilemma model replaces Melting Pot's coverage.

### LiveCultureBench — cross-cultural norms and evaluation reliability

- ACL Anthology: https://aclanthology.org/2026.acl-long.1247/
- DOI: https://doi.org/10.18653/v1/2026.acl-long.1247

LiveCultureBench (Pham et al., ACL 2026) places LLM agents in a dynamic multi-cultural town and evaluates task completion alongside socio-cultural norm adherence. It explicitly reports verifier uncertainty and when human oversight is needed.

**CSS/CivBench response:** report task effectiveness, norm adherence, rights/consent constraints, norm conflicts and evaluator reliability as separate outcomes. Use human-coded reference cases to calibrate automated evaluators. Do not use national or demographic labels as deterministic behavioral features, and do not treat an LLM judge as ground truth.

### Hundreds of LLM agents — collective welfare and low-welfare equilibria

- Paper: https://arxiv.org/abs/2602.16662
- Implementation: https://github.com/willis-richard/emergent_llm

Willis et al. (2026) evaluate inspectable fixed strategies across populations of hundreds in public-goods, collective-risk and common-pool-resource dilemmas. Their work highlights a system-level failure mode: individually motivated agents can converge to low-welfare collective outcomes, particularly as population and incentive conditions change.

**CSS/CivBench response:** add population-mixture sweeps, resource-stock dynamics, welfare attractor analysis and lower-tail distribution reporting. Treat strategy code as inspectable policy artifacts. Compare against simple deterministic strategies and resource-dynamics oracles before adding LLM-based policies.

### Human experiments on intentional and cumulative transmission

- Mackintosh et al. (Scientific Reports, 19 September 2026): https://www.nature.com/articles/s41598-026-62734-6
- PNAS Nexus (26 March 2026): https://doi.org/10.1093/pnasnexus/pgag090
- Reward Network III (Nature Communications, 2026): https://www.nature.com/articles/s41467-026-76113-2

Recent human experiments distinguish intentional teaching from inadvertent information sharing, and cultural transmission from individual causal reasoning. Reward Network III provides a particularly useful source-specific machine-to-human transmission anchor. These studies should remain distinct experiments with their own population, task, dataset and scoring protocols.

**CSS/CivBench response:** measure sender selection, learner acquisition, actual task execution, retained capability, transfer and correction separately. Include individual-learning controls and matched transmission budgets. A successful message delivery is not evidence of learning; a stored content hash is not an acquired capability.

### Adaptive transmission programs — recipient-conditioned routing

- Colas et al. (2026): https://arxiv.org/abs/2608.24545

Colas et al. formalize transmission programs that condition on the collective and recipient states. Their reported up-to-37% improvement is specific to the paper's Collective LittleAlchemy setting and should not be compared numerically to a different CivBench DAG without source-task reproduction.

**CSS/CivBench response:** extend the existing Symthaea bounded TransmissionProgram and grid-search runner. Do not create another routing DSL. Evaluate positive knowledge, scoped negative evidence, recipient need, privacy leakage and cross-task transfer under matched realized communication costs.

### Revised comparative capability map

| Capability | Existing benchmark / infrastructure | CSS/CivBench's proposed additional contribution |
|---|---|---|
| Social generalization | Melting Pot | Couple held-out partners and incentives to resource, institutional and cultural-memory dynamics |
| Cross-cultural norms | LiveCultureBench | Norm/task/rights vectors, human-coded calibration and evaluator uncertainty |
| Population-scale welfare | Hundreds-of-LLM-agents framework | Add ecological stock closure, transmission history, institutional memory and low-welfare attractor analysis |
| Human cumulative transmission | Reward Network III; intentional-transmission studies; PNAS Nexus transmission chains | Reproducible evidence→acquisition→execution→retention→transfer chain |
| Adaptive knowledge routing | Colas et al. Collective LittleAlchemy | Add scoped negative knowledge, recipient authorization, privacy attacks and physical prerequisites |
| Civilizational dynamics | Existing socio-ecological model and external ABM ecosystems | Compose the above with physical constraints, historical evidence, institutions and service-floor outcomes |

This positioning is complementary. The programme's competitive edge should be demonstrable only on named tasks where the extra evidence, physical closure or long-horizon cultural mechanisms improve an independently scored outcome.


## 9. 2026 social-dilemma adaptation, welfare allocation and population-scale effects

### Collective-risk adaptation — source task plus analytical oracle

**Huynh et al. (2026), "Cooperation Without Adaptation: LLM Agents in a Collective-Risk Dilemma"**  
https://arxiv.org/abs/2610.22882

The paper's task uses six seats, 40 initial units per seat, ten rounds, choices of 0/2/4, target 120 and catastrophe probability p when the target is missed. Its expected-welfare optimum switches at p*=0.5. Its central methodological contribution for CivBench is not another cooperation score: it separates correct payoff reasoning from actual action adaptation, exact best response to fixed partners, mixed-model group composition and population-size-dependent evolutionary selection.

**CivBench response:** use the exact six-seat task as a source-aligned reference; build an analytical welfare oracle and exhaustive best-response enumeration before comparing cognitive or LLM agents. The public task reconstruction is not a reproduction of prompts or model results.

### Cooperation-support mechanisms — compare mechanisms under partner shift

**Tewolde et al. (ICML 2026), "CoopEval: Benchmarking Cooperation-Sustaining Mechanisms and LLM Agents in Social Dilemmas"**  
https://proceedings.mlr.press/v306/tewolde26a.html

CoopEval compares repeated interaction, reputation, third-party mediation and outcome-contingent contracts. Its results indicate that repetition-based cooperation is fragile to changing partners in the studied games, whereas contracts and mediation more consistently support cooperative outcomes.

**CivBench response:** compare mechanism families in homogeneous self-play, unfamiliar partner mixes, one-shot settings, noisy reputation and breach/dispute conditions. Keep mechanism authority, verification and appeal explicit. Do not count a simulated reputation label or contract name as an implemented mechanism.

### Welfare allocation — productivity is not distributive legitimacy

**Shi et al. (Findings ACL 2026), "Social Welfare Function Leaderboard: On the Emergence of LLM Agents as the Welfare Dictator"**  
https://aclanthology.org/2026.findings-acl.1919/

The SWF benchmark gives an LLM scarce allocation authority over heterogeneous recipients and measures a productivity-versus-inequality trade-off. It reports that general model ability does not predict allocation quality, and that allocations are sensitive to persuasion.

**CivBench response:** add a bounded allocation task with auditable recipient data, verified needs/returns, essential floors, explicit objective families (utilitarian, max-min and leximin), Pareto-frontier reporting and adversarial persuasion. Facts and recipient claims must be immutable across rhetorical prompt perturbations unless an actual evidence input changes. Report rights-floor violations separately from productivity and Gini.

### Population scaling — no linear extrapolation from self-play

**Flint et al. (PNAS, 18 August 2026), "Group size effects and collective misalignment in LLM multi-agent systems"**  
https://doi.org/10.1073/pnas.2531697123

The paper varies population size and reports nonlinear effects on collective biases and emergent dynamics; a mean-field analysis is used to characterize large-population regimes.

**CivBench response:** distinguish interaction-group size from total population size, evaluate overlapping group composition and policy-selection dynamics, and compare finite-population simulations with an explicit mean-field reference. Report transition thresholds and uncertainty, not a single large-N point estimate.

These four works reinforce a general design rule: test incentives, partner turnover, distributional effects and scaling as independent dimensions. Do not interpret cooperative self-play or aggregate payoff as evidence of robust collective intelligence.


## 10. Operational validation, AI-mediated governance and resilience (4 November 2026)

### Operational validation — compare distributions and structure, not narrative plausibility

**Tomašević et al. (EPJ Data Science, 11 June 2026), "Towards operational validation of LLM-agent social simulations".**  
https://doi.org/10.1140/epjds/s13688-026-00674-x

This study runs 30 independent 30-day simulations and compares them with 30 matched, non-overlapping empirical windows. It evaluates activity patterns, network structure, toxicity, topical coverage and stylistic convergence. Several aggregate activity measures align, while thread depth, network concentration/repeated interactions and toxicity by content layer diverge.

**CivBench response:** a reproducible pattern-vector protocol with independent runs, matched windows, calibration/holdout separation and stratified errors. Report KS/Wasserstein distribution distances, quantile differences and tail statistics alongside mean intervals. Never allow root-post and comment errors, or different regions/population groups, to cancel in one pooled mean. Similarity alone does not establish mechanism validity.

Implementation: [Symtropy issue #3616](https://github.com/Luminous-Dynamics/symtropy/issues/3616), [draft PR #3619](https://github.com/Luminous-Dynamics/symtropy/pull/3619). Its current 30-sample synthetic fixture only validates the comparison code; it is not empirical validation.

### AI-mediated deliberation — satisfaction is not influence neutrality

**Parisi et al. (FAccT 2026), "Real-Time Group Dynamics with LLM Facilitation: Evidence from a Charity Allocation Task".**  
https://arxiv.org/abs/2605.14097  
https://deepmind.google/research/publications/224297/

Two studies (N=879) used incentive-compatible groups of three allocating a real charity budget. The study reports that LLM facilitation did not significantly improve aggregate consensus, but participants preferred it; some charity-level allocations shifted by up to 5.5 percentage points, while measured participation equity did not improve. Participants also reported greater process trust under conditions with detectable directional influence.

**CivBench response:** separate consensus, allocation effect, whose proposal is adopted, speaking/turn share, factual accuracy, self-reported inclusion/trust and actual participation equity. Rhetoric-only framing and agenda-order perturbations are required negative controls.

Implementation: [Symtropy issue #3617](https://github.com/Luminous-Dynamics/symtropy/issues/3617).

### Social-ecological resilience — dynamics and uncertainty, not static scores

**Balali et al. (Ecological Indicators, 2026), "Toward convergence in social-ecological systems resilience assessment".**  
https://doi.org/10.1016/j.ecolind.2026.114792

The systematic review analyzes 121 peer-reviewed studies and reports that 59% used static assessments, 74% did not explicitly account for uncertainty and 96% neglected cross-scale dynamics or interactions between endogenous and exogenous shocks. It also reports that indicator-based approaches dominate but are often not standardized.

**CivBench response:** treat resilience as a response trajectory with resistance/absorption, service loss, recovery time and quality, adaptation, transformation, distributional burden and ecological/resource condition. Evaluate compound shocks, scale coupling, endogenous deterioration and exogenous hazards. Do not infer resilience from a static indicator or a single weighted composite.

Implementation: [Symtropy issue #3621](https://github.com/Luminous-Dynamics/symtropy/issues/3621).

These studies are research anchors, not evidence that a single shared resilience or governance metric exists.


## 11. Population-relative ranking and non-transitive strategy ecologies

### Population-dependent selection in collective-risk games

**Huynh et al. (2026), "Cooperation Without Adaptation: LLM Agents in a Collective-Risk Dilemma".**  
https://arxiv.org/abs/2610.22882

The paper's selection protocol samples five partners without replacement, computes hypergeometric composition-weighted fitness, applies pairwise Fermi imitation (beta=1 per payoff unit), and uses rare mutation/fixation to construct a Markov chain over homogeneous populations. It compares N=6 (one population-wide group) with N=30 (overlapping random groups), and reports that selection changes with population size/composition.

**CivBench response:** report N, group size, partner composition, risk, beta and mutation assumptions beside every ranking. Use exact finite-population group composition and a fixation/stationary reference before running expensive cognitive agents. Implementation: [issue #3646](https://github.com/Luminous-Dynamics/symtropy/issues/3646), [draft PR #3648](https://github.com/Luminous-Dynamics/symtropy/pull/3648).

### Non-transitive opponent pools and regret certificates

**Deng et al. (Scientific Reports, 4 June 2026), "Population-dependent agent performance in non-transitive games: a multi-agent rock–paper–scissors benchmark".**  
https://www.nature.com/articles/s41598-026-55417-9

The study compares 54 agents across 18 archetypes in 500-round double round-robin play over ten seeds (14.31 million evaluated rounds). It reports opponent-roster-sensitive rankings (Spearman rho=0.83 between two rosters), 134 detected three-agent cycles in the empirical meta-game, and a regret certificate bounding best-response regret by twice the L1 error in predicted opponent action probabilities for its normalized stage payoff. It also records end-to-end CPU milliseconds per decision to expose resource cost.

**CivBench response:**
- Freeze multiple opponent rosters and held-out compositions; do not use one static leaderboard.
- Report the complete cross-play payoff matrix and ranking changes across rosters.
- Detect non-transitive three-cycles and invasion cycles instead of forcing a total order.
- Record exploitability/regret against exact best-response oracles where the game permits.
- When policies expose probabilistic predictions, log proper scoring/calibration and a game-specific regret certificate; do not claim the RPS constant transfers unchanged to CRG.
- Report seed-paired uncertainty, decision latency and compute budgets beside payoff.

### SOTA comparison discipline

The ICML 2026 position paper *Time to Close the Validation Gap in LLM Social Simulations* argues for consolidation through shared benchmarks, open data and reproducible evaluation rather than expansion of unvalidated simulators.  
https://proceedings.mlr.press/v306/puelma-touzel26a.html

Our advantage should therefore be independently checkable reference tasks, exact group-composition oracles, distributional and procedural validity checks, and explicit claim ceilings—not the number of agent personas or the size of a simulation.


## 11. Human-behaviour fidelity and audit-format sensitivity (4 November 2026)

### Individual human simulation — macro fit is not individual fidelity

**Hu et al. (ICLR 2026), "SimBench: Benchmarking the Ability of Large Language Models to Simulate Human Behaviors".**  
https://proceedings.iclr.cc/paper_files/paper/2026/hash/2fc4d6a6400addda7493b9db258c34e0-Abstract-Conference.html  
https://simbench.tiancheng.hu/

SimBench unifies 20 diverse datasets (including moral decisions and economic choices). Its top published simulation-fidelity score is 40.80/100. The authors report log-linear scaling with model size, no gain from increased inference-time compute in their setup, an alignment–simulation trade-off (instruction tuning can improve low-entropy consensus tasks while harming high-entropy diversity), and difficulties simulating specific demographic groups.

**CivBench response:** reproduce the upstream benchmark and per-task breakdown before adapting it. Evaluate micro/conditional/meso/macro fidelity, high-entropy response diversity, individual heterogeneity, participant holdouts and cohort-specific support. An aggregate score can be reported only with the upstream scoring semantics; it cannot replace the per-task vector.

### Macro–micro dissociation — similar cooperation curves can come from wrong agents

**Ferraz de Arruda et al. (2026), "Collective cooperation without individual fidelity in LLM agents".**  
https://arxiv.org/abs/2606.30454

In a networked Prisoner's Dilemma comparison with human data, the paper reports that an LLM population can reproduce several macro cooperation patterns while underestimating individual heterogeneity and misrepresenting conditional cooperation rules.

**CivBench response:** score per-agent response distributions, conditional action curves and group interaction patterns as well as population trajectories. Random agents or noise should not be rewarded for restoring marginal variance if their conditional policies are wrong.

Implementation: [CIVDYN-HUMSIM-001 issue #3665](https://github.com/Luminous-Dynamics/symtropy/issues/3665).

### Audit-format sensitivity — bias direction can depend on the audit

**Lukk (Findings EMNLP 2026), "FairFund-Bench: Evaluating Distributive Bias in LLM Resource Allocation".**  
https://aclanthology.org/2026.findings-emnlp.1340/  
https://arxiv.org/abs/2607.28934

FairFund-Bench evaluates 14 models over 600 human-templated requests, varying rating/ranking/allocation, isolated versus multi-stimulus presentation and transparent versus disguised audits. The paper reports that audit format changes the direction of measured group bias and that causal framing effects exceed demographic effects in its benchmark.

**CivBench response:** cross rating, ranking and allocation tasks with stimulus context and audit disclosure. Report demographic disparity, declared-rule/deservingness alignment, cross-task consistency and cross-context consistency separately. Test name/order counterfactuals and avoid assuming equal allocations are appropriate when verified need differs.

Implementation extension: [CIVDYN-SWF-001 issue #3610](https://github.com/Luminous-Dynamics/symtropy/issues/3610).

### Benchmark target discipline

SimBench's 40.80/100 is an upstream result, not a directly comparable Symthaea score. Reproduce its exact evaluation, datasets, prompts, splits, scoring and uncertainty before making a comparison. The aspirational +5-point target is valid only under that matched protocol and must be accompanied by per-dataset and cohort non-regression checks.


## 12. Cross-scale resilience trajectory implementation

CIVDYN-RES-001 has its first executable slice in [Symtropy draft PR #3666](https://github.com/Luminous-Dynamics/symtropy/pull/3666), stacked on the conserved resource-ledger PR #3391. The reference uses 7 inventory nodes across basin/district/household scales, three shock types and four transfer policies. It reports sustained recovery only after every household clears the 0.95 service threshold for two consecutive post-shock ticks. This is a synthetic code fixture, not regional evidence or a forecast.

Source motivation: Balali et al. (2026) find major fragmentation and a shortage of dynamic, uncertainty-aware, cross-scale socio-ecological resilience assessments. https://doi.org/10.1016/j.ecolind.2026.114792


## 13. Human simulation reliability: R1/R2 and cross-scenario trajectories

### Reliability is not one end-to-end score

**Wang et al. (EMNLP 2026), "Understanding Reliability in LLM-based Human Behavior Simulation" (ReliMap).**  
https://aclanthology.org/2026.emnlp-main.1269/

ReliMap splits individual reliability (R1) from population reliability (R2), across model capacity, profile completeness and population coverage. The paper reports that R1 and R2 can move in opposite directions; more participants reduce variance but do not necessarily remove systematic distribution bias, with R2 stabilizing around 50–100 participants in its tested tasks.

**CivBench response:** report micro proper-scoring rules and population distribution distances separately, stratify by declared cohorts/context, and publish sample-coverage curves. Variance reduction is not proof of bias reduction.

### Isolated tasks miss behaviour sequences

**Chen et al. (2026), "Towards Real-world Human Behavior Simulation: Benchmarking LLMs on Long-horizon, Cross-scenario, Heterogeneous Behavior Traces" (OmniBehavior).**  
https://arxiv.org/abs/2604.08362

OmniBehavior uses anonymized real-world platform traces and reports that isolated scenarios miss cross-scenario links and long-horizon heterogeneity. Its benchmark description covers 200 sampled users, three months, 2.12M interactions, 22 action types and five scenarios. Its authors report positive-average-person bias, hyperactivity and homogenization in simulated behaviour.

**CivBench response:** add context-sequence holdouts and transition metrics. Keep rare, low-activity and long-tail behaviours visible. Treat real trace reuse as a source-rights/privacy gate, not merely a data-download task.

### Context heterogeneity is a mechanism, not an entropy hack

**Zhen et al. (Findings EMNLP 2026), "Context Modeling for LLM-Based Behavioral Simulation".**  
https://aclanthology.org/2026.findings-emnlp.1306/

The study constructs latent individual decision contexts before response generation and samples structured context variation; it reports gains in high-entropy and subgroup response fidelity and recovery of minority/low-probability options.

**CivBench response:** benchmark direct prompting, evidence-grounded profile conditioning and structured context sampling under identical task splits and compute budgets. Do not award diversity gains that come from uncalibrated random noise.

Implementation: [CIVDYN-HUMSIM-001 issue #3665](https://github.com/Luminous-Dynamics/symtropy/issues/3665). A synthetic Rust reference is being started; no human benchmark score is claimed.


## 14. Human fidelity implementation status

[CIVDYN-HUMSIM-001 issue #3665](https://github.com/Luminous-Dynamics/symtropy/issues/3665) now has a first executable reference in [draft PR #3668](https://github.com/Luminous-Dynamics/symtropy/pull/3668). It separates individual Brier/log-loss/accuracy from population TVD/JS/entropy/tail-option coverage, cohort/context slices and adjacent-context transition discrepancy. Its deterministic N={10,20,30,50,60} curve is explicitly a diagnostic, not an inference interval. The synthetic fixture compares a mode-collapse control to a structured-context oracle and uses no human/platform data.

The source motivation includes ReliMap (EMNLP 2026), OmniBehavior (2026) and Context Modeling (Findings EMNLP 2026); see the HUMSIM issue for the exact protocol and data-rights boundaries.


## 15. Shared benchmark measurement discipline: Measurement-as-Distribution

Three 2026 papers strengthen the argument for a shared measurement standard rather than further proliferation of simulators:

- **Afli & Frank, Findings EMNLP 2026**, The Benchmark Is Not the Construct: Toward Distribution-Based Evaluation in NLP. https://aclanthology.org/2026.findings-emnlp.1992/  
  Argues for principled data generation, validated perturbations, repeated evaluation and distributional reporting, with evidence requirements scaling to the breadth of the claim.
- **Puelma Touzel et al., ICML 2026**, Position: Time to Close The Validation Gap in LLM Social Simulations. https://proceedings.mlr.press/v306/puelma-touzel26a.html  
  Calls for shared benchmarks, open data and reproducible protocols rather than unvalidated simulation expansion.
- **Wu et al., ICML 2026**, Position: LLM-Based Social Simulations Require a Boundary. https://proceedings.mlr.press/v306/wu26co.html  
  Argues that evaluation depth must match the heterogeneity demanded by the research question and that behavioural variance must be measured alongside mean fit.

**CivBench implementation:** [CIVBENCH-MAD-001 issue #3731](https://github.com/Luminous-Dynamics/symtropy/issues/3731).

The contract distinguishes observation units from resampling units; source families and claim scopes; epistemic, stochastic and measurement uncertainty; valid perturbation families and their construct rationale; and six evidence tiers from executable integrity to operational decision readiness. Thirty replications is not a universal statistical threshold: independence, power and the sampling design determine the correct replication unit and count.

The key design consequence is that model, prompt, context, roster, population, period and intervention form an evaluation distribution. A benchmark result is conditional on that declared distribution and protocol. No global CSS/CivBench score is emitted.


## 16. SOTA refresh: behavioural foundation models, study replay and long-horizon collaboration (5 November 2026)

### OdysSim / SOUL-Index: the current human-behaviour foundation-model reference

**Zhou et al., "OdysSim: Building Foundation Models for Human Behavior Simulation" (12 June 2026).**  
Paper: https://arxiv.org/abs/2606.14199  
Code: https://github.com/sunnweiwei/OdysSim

The paper defines five axes (CONV, SS, COG, ROLE, EVAL), aligns 62 public sources and 21.4M interactions (~10B tokens), and evaluates 23 tasks. The published unweighted average is OSim-8B 64.6 (best/tied-best on 8/23), Claude Opus 4.7 65.5, GPT-5.5 65.2 and Gemini 3.1 Pro 64.8. OSim-8B's 93.2 vs 93.5 user reaction alignment is on a specific τ-USI task, not a general human-fidelity percentage.

The paper's training recipe combines behaviour-oriented midtraining, per-task GRPO or verbal-feedback RL, and expert distillation. It reports judge manipulation and short-generic-response reward hacking; one detected Sotopia manipulation mode falls from approximately 20–25% to near zero after its detector. The authors acknowledge the text-only / performed-conversation / LLM-judge limitations.

**Protocol warning:** the published paper fixes 23 tasks; the current repository's README describes a newer 27-task evaluation script. Treat these as separate releases. Exact paper score comparison is only against the frozen 23-task list, model IDs, scoring and data snapshot. The 27-task script gets its own protocol ID and no retrospective mixing.

Implementation: [CIVDYN-HUMSIM-001 #3665](https://github.com/Luminous-Dynamics/symtropy/issues/3665), synthetic kernel [PR #3668](https://github.com/Luminous-Dynamics/symtropy/pull/3668).

### HumanStudy-Bench: reproduce experimental inference, not only participant answers

**Liu et al., "HumanStudy-Bench: Towards AI Agent Design for Participant Simulation" (2026).**  
Paper: https://arxiv.org/abs/2602.00685  
MIT repository: https://github.com/HumanStudy-Hub/HumanStudy-Bench

The initial suite contains 12 reconstructed human-subject studies and more than 6,000 trials. It replays the original study protocol and analysis procedures and proposes PAS (probability alignment at the scientific-finding level) and ECS (effect consistency in the pattern and magnitude of observed effects).

**CivBench response:** require original-test reproduction and report participant-level R1, population R2, treatment-effect sign/magnitude, uncertainty and study-level conclusion separately. A model can match answer frequencies but fail the experiment's inference.

Implementation plan: [CIVDYN-HUMANSTUDY-REPRO-001 #3786](https://github.com/Luminous-Dynamics/symtropy/issues/3786).

### Naturalistic multi-turn action traces: a simulator must predict what a person does next

**Lu et al., ACL 2026, "Can LLM Agents Simulate Multi-Turn Human Behavior? Evidence from Real Online Customer Behavior Data".**  
https://aclanthology.org/2026.acl-long.2034/

The study analyzes 31,865 online shopping sessions and 230,965 user actions. Its prompt-only LLM baseline obtains 11.86% next-action accuracy on this particular e-commerce task; its fine-tuned Qwen2.5-7B reaches 17.26% action accuracy and 33.86% final-purchase F1. These are task-specific figures, not a general human-simulation leaderboard.

**CivBench response:** add a privacy- and license-gated naturalistic action lane with session-level holdouts, next-action likelihood, sequence/transition fidelity, terminal-choice F1 and calibrated uncertainty. Preserve low-activity and long-tail trajectories.

### Long-horizon collaboration: test actual contribution under role asymmetry

**Shu et al., COLM 2026, "AgentWorld: Benchmarking Long-Horizon Collaboration of Multi-agent LLMs".**  
Paper: https://arxiv.org/abs/2609.31590  
Project: https://agentworld.io/

The paper describes 100 human-annotated tasks plus 100 augmented variants, 3–20 agents with asymmetric roles, and 25–55 maximum rounds. It evaluates SR, PSR and graph-based CCE. The best reported main-set SR is 52.0% (Gemini 3 Flash); the paper's main-set PSR and CCE are 71.5% and 0.320.

CCE's use of judge-mediated causal-enablement decisions is itself an evaluation assumption. CivBench therefore preserves the paper's score for exact reproduction while separately evaluating verified causal lower bounds, candidate upper bounds and human-audited edge precision/recall.

Implementation: [CIVDYN-AGENTWORLD-REPRO-001 #3785](https://github.com/Luminous-Dynamics/symtropy/issues/3785), [causal collaboration PR #3655](https://github.com/Luminous-Dynamics/symtropy/pull/3655).

### What would count as doing better?

1. Reproduce each source protocol before making a leaderboard comparison.
2. Report per-task/per-study/per-role vectors; keep official macro-averages only for direct comparability.
3. Win on an external naturalistic action-trace or human-study holdout, not merely the training-aligned SOUL tasks.
4. Beat the reference without losing behavioural diversity, minority/rare-option coverage, individual consistency or uncertainty calibration.
5. Demonstrate causal contribution and task success under 50+ round, role-asymmetric collaboration.
6. Audit judge gaming and outcome-based reward hacking with independent verifiers and blinded human annotation.

No single SOUL/HUMSIM/CCE/CivBench number is a universal intelligence, fairness or civilizational-quality score.


## 17. Accountability must include the simulation stack (6 November 2026)

### From plausible agents to accountable simulation

**Liu, United Nations University Macau (29 September 2026), "From Plausible Agents to Accountable Simulation: A Technical and Governance Framework for LLM-Enabled Agent-Based Modelling".**  
https://unu.edu/publication/plausible-agents-accountable-simulation-technical-and-governance-framework-llm-enabled

This 22-page review examines 14 public LLM-agent projects and 9 traditional ABM platforms, with source-code inspection where available. Its central methodological warning is that fluent/plausible agents can still be empirically unrealistic; parsers, repair/retry logic, default actions, schedulers, exposure rules, memory/retrieval settings and environment machinery can drive aggregate results as much as the language model. It notes that some systems leave controllable randomness unseeded and that hidden fallbacks may silently replace invalid model outputs with fixed actions.

**CivBench response — stack-level validity manifest.** Every LLM-agent result must bind together:
- base model/version, prompt/template and decoding;
- persona/profile source and completeness;
- memory/retrieval settings and retained-state policy;
- action parser, schema validator, retries, repair and fallback policy;
- scheduler, update order, feed/exposure and interaction constraints;
- environment dynamics, transaction settlement and measurement code;
- RNG streams/seeds and external nondeterminism;
- data capture, metric implementation, holdout/split manifest and scoring;
- independent output receipt and exact source/config digest.

The benchmark must add anti-shortcut controls: malformed output, timeout, duplicate action, omitted agent, reordered event, alternate scheduler, altered feed, unseeded source and changed fallback. Every fallback must be observable and counted; silent defaults fail conformance. Compare the same declared environment with ablations of agent cognition and environment machinery to estimate their separate contributions.

This complements, rather than replaces, [CIVDYN-ENGINE-COMPARE-001 #3787](https://github.com/Luminous-Dynamics/symtropy/issues/3787). The runtime lane requires behavioural equivalence before timing; the accountability lane requires stack-level provenance before claiming validity. The two lanes must not be merged into one score.

### Current implementation map

- [CIVDYN-DELIB-001 #3617](https://github.com/Luminous-Dynamics/symtropy/issues/3617) → procedural audit draft [PR #3682](https://github.com/Luminous-Dynamics/symtropy/pull/3682); its exact-head hosted workflow passed.
- [CIVDYN-RES-001 #3621](https://github.com/Luminous-Dynamics/symtropy/issues/3621) → cross-scale service resilience [PR #3666](https://github.com/Luminous-Dynamics/symtropy/pull/3666); exact-head workflow passed.
- [CIVDYN-HUMSIM-001 #3665](https://github.com/Luminous-Dynamics/symtropy/issues/3665) → micro/population/trajectory fidelity [PR #3668](https://github.com/Luminous-Dynamics/symtropy/pull/3668); exact-head workflow passed.
- [CIVDYN-NTG-001 #3671](https://github.com/Luminous-Dynamics/symtropy/issues/3671) → opponent-roster cycles and regret [PR #3674](https://github.com/Luminous-Dynamics/symtropy/pull/3674); exact-head workflow passed.
- [CivBench independent ledger/counterfactual issue #3319](https://github.com/Luminous-Dynamics/symtropy/issues/3319) → matched causal-path conformance [PR #3803](https://github.com/Luminous-Dynamics/symtropy/pull/3803); draft, exact-head CI pending.

All listed PRs remain drafts. Hosted software-fixture passes do not constitute empirical validation or a SOTA result.


### Stack-level accountability is now part of the machine-checkable MAD protocol

The [CIVBENCH-MAD-001 draft schema PR #3735](https://github.com/Luminous-Dynamics/symtropy/pull/3735) now requires execution-stack inventory and negative controls for hidden fallback, unseeded RNG, policy/component mismatch and dropped events. This operationalizes the UNU 2026 accountability warning without adding another run-envelope: Humanity Atlas remains the authoritative run-artifact contract; MAD references it and declares the evaluation conditions.


### Operational validity sample-size discipline

The `civdyn-operational-validity` reference has been updated in [draft PR #3619](https://github.com/Luminous-Dynamics/symtropy/pull/3619): minimum observed/simulated counts, independent units, dependence assessment and sample-size rationale are now declared per metric. The 30+30 fixture mirrors the June 2026 replicated-forum study only. This avoids turning one study's sampling design into a universal rule; independent-unit declarations still require source-design justification.


## 18. Configuration, not just model, is the audit unit (6 November 2026)

**Loghmani et al. (EMNLP 2026), "Hidden Configuration Effects on Fairness in LLM-Based Multi-Agent Decision Systems".**  
https://aclanthology.org/2026.emnlp-main.2531/

The authors sweep persona, interview protocol, deliberation, aggregation and order across three models and two applicant datasets. Their abstract reports within-model persona effects up to 91 percentage points in positive-prediction rate, chair-dependent changes in disparate-impact outcomes, and speaking-order changes flipping up to 34% of applicants in the evaluated panel setup. These are results from that study's tasks and configurations, not general constants.

**CivBench response:** every decision audit is bound to a complete configuration passport, not just a model ID. A canonical digest covers model/version, persona, elicitation/interview, facilitator/chair, aggregation, order, prompt, tools, evidence and decoding, with task/data/split identity. Changing any decision-relevant value triggers a fresh audit identity.

Implementation: [CIVDYN-CFG-001 #3849](https://github.com/Luminous-Dynamics/symtropy/issues/3849), [draft PR #3850](https://github.com/Luminous-Dynamics/symtropy/pull/3850). The current crate validates digest format/uniqueness supplied by the caller and compares the same fixed case/slice set across configurations. Its fixture is synthetic and is not a reproduction of the EMNLP result.

The benchmark reports case-flip rate, overall selection rates, declared-cohort numerator/denominator, reference ratio and matched single-facet contrasts. Multi-facet changes are not described as causal single-factor effects. Meaning-preserving reorderings are negative controls; substantive config changes are not forced to be invariant, but must be measured and re-audited.

### Consolidation rule

The ICML 2026 position paper argues that the field should consolidate around shared benchmarks, open data and reproducible protocols rather than continuing to multiply unvalidated simulators: https://proceedings.mlr.press/v306/puelma-touzel26a.html

Accordingly, CIVDYN-CFG is not a new simulator or run-envelope. It is an orthogonal conformance slice that feeds the existing Humanity Atlas run contract and CIVBENCH-MAD-001. Its metrics remain part of the outcome vector; no global fairness/validity scalar is added.


## 19. Configuration passport and re-audit identity: executable conformance

CIVDYN-CFG-001 is the first matched decision-configuration reference: [issue #3849](https://github.com/Luminous-Dynamics/symtropy/issues/3849), [draft PR #3850](https://github.com/Luminous-Dynamics/symtropy/pull/3850).

Exact-head CI passed on 6 November 2026:
- 9 library tests and 2 binary tests;
- format, strict Clippy and locked compile;
- 13-row/21-column CSV schema and semantic controls;
- zero flips under semantic option permutation;
- four of twelve flips under position-biased order, with unchanged 50% overall selection rate;
- cohort B/reference cohort A selection-rate ratio reverses from 0.5 to 2.0.

A bug discovered by the synthetic test was corrected before qualification: each cohort ratio now uses the reference cohort rate *within the same configuration*, not the baseline configuration's rate. The test remains as a regression guard.

### MAD configuration passport

The shared [CIVBENCH-MAD-001 issue #3731](https://github.com/Luminous-Dynamics/symtropy/issues/3731) and [draft PR #3735](https://github.com/Luminous-Dynamics/symtropy/pull/3735) now require both:
1. `configuration_passport`: model/version, persona, interview protocol, facilitator, aggregation, order, prompt, tools, evidence snapshot, decoding, task/data identity and canonical digest;
2. `execution_stack`: parser, validator, retries, fallback events, scheduler, exposure, RNG and event capture.

The distinction prevents an unchanged model name from being mistaken for an unchanged decision system, and prevents prompt identity from substituting for runtime accountability. The validator recomputes the canonical SHA-256 and includes a negative test: a persona change without a new digest must fail. Its exact-head synthetic fixture workflow passed on commit `8431a109da905986e9f80fdd48cde8a86b14dde8`, run [#42217641736](https://github.com/Luminous-Dynamics/symtropy/actions/runs/42217641736).

These are Tier 1 schema/software controls. They are not a reproduction of the EMNLP 2026 fairness study, a human-effect estimate or a universal fairness test.


## 20. What the current research frontier actually says (6 November 2026)

### The field's central problem is validation, not simulator scarcity

Puelma Touzel et al. (ICML 2026), *Position: Time to Close the Validation Gap in LLM Social Simulations*, argue for consolidation around shared benchmarks, open data and reproducible protocols rather than unvalidated expansion. The paper explicitly warns that high-stakes policy, epidemiology and computational-social-science uses risk premature deployment when the simulator's validity is not established.
- https://proceedings.mlr.press/v306/puelma-touzel26a.html

Afli & Frank (Findings EMNLP 2026), *The Benchmark Is Not the Construct*, formalize Measurement-as-Distribution: build a defensible distribution of conditions, validated perturbations, repeated measurements and uncertainty reporting instead of treating one scalar score on one fixed test as the capability itself.
- https://aclanthology.org/2026.findings-emnlp.1992/

**CivBench response:** preserve the protocol's task-specific metric vector; bind it to the complete configuration passport and behavior-affecting execution stack; evaluate across principled perturbations, strata, independent units and holdouts. The evidence plane and Humanity Atlas run contract remain the authority for run identity and receipts. MAD is the measurement specification, not a new simulator or run-envelope.

### Human simulation: published baselines remain modest and task-specific

SimBench (ICLR 2026) unifies 20 human-behaviour datasets and reports a top published score of 40.80/100. It finds high-entropy response diversity and specific-cohort fidelity challenging, and reports an alignment/simulation trade-off between consensus-style and diverse-response tasks.
- https://proceedings.iclr.cc/paper_files/paper/2026/hash/2fc4d6a6400addda7493b9db258c34e0-Abstract-Conference.html

OdysSim/SOUL (2026) reports 62 datasets and 23 benchmark tasks, with OSim-8B first or tied-first on 8/23 tasks in the authors' evaluation. It also reports judge-reward hacking and the need for detectors. This is a different suite and score protocol from SimBench; task counts are not directly comparable.
- https://arxiv.org/abs/2606.14199

**CivBench response:** reproduce each upstream benchmark exactly before comparison. Report R1 individual response scores separately from R2 population-distribution scores, transition/trajectory fidelity, subgroup support, tail-option coverage, judge disagreement and external transfer. Do not compare the SimBench 40.80/100 to an SOUL task count or our synthetic kernel.

### Configuration is part of the scientific object

Loghmani et al. (EMNLP 2026) report large within-model effects across persona, interview, deliberation, aggregation and order; in their evaluated tasks, persona could shift positive-prediction rates by up to 91 percentage points and speaking-order reversal flipped up to 34% of applicants.
- https://aclanthology.org/2026.emnlp-main.2531/

**CivBench response:** a model name is not a complete configuration identity. A decision claim must be tied to model/version, persona, elicitation protocol, facilitator, aggregation, option order, prompt, tools, evidence snapshot, decoding, task contract and data split. Any decision-relevant change creates a new audit identity. Full configuration stability is separate from normative fairness.

### Multi-agent performance is population-conditional

Deng et al. (Scientific Reports, 4 June 2026) test 54 agents from 18 archetypes in 500-round double round-robin RPS tournaments over 10 seeds. Their study reports 134 directed three-cycles in the induced payoff matrix and roster-dependent rankings (Spearman rho 0.83 between two pools).
- https://www.nature.com/articles/s41598-026-55417-9

**CivBench response:** rank only against a named roster and protocol; publish pairwise payoff matrices, cycles, exploitability, rank correlations, seed-paired uncertainty and held-out-roster performance. Our three-action RPS kernel is an exact control, not a reproduction of the 54-agent benchmark.

### Socio-ecological resilience requires dynamics and cross-scale uncertainty

Balali et al. (Ecological Indicators, April 2026) review 121 studies and report 59% static assessment, 74% without explicit uncertainty, and 96% neglecting cross-scale dynamics or endogenous/exogenous shock interactions. The authors explicitly state that the review does not test the empirical accuracy of individual measures.
- https://doi.org/10.1016/j.ecolind.2026.114792

**CivBench response:** service recovery, stock/ecological recovery, adaptation, transformation, worst-served groups and cross-scale shock propagation remain separate. Thirty or more independent replications is a proposed floor for our inferential portfolio only when the independent unit and dependence design justify it; it is not a universal number.

## 21. Measurement-as-Distribution integration status

The CivBench MAD extension is in [draft PR #3771](https://github.com/Luminous-Dynamics/symtropy/pull/3771), based on [MAD protocol PR #3735](https://github.com/Luminous-Dynamics/symtropy/pull/3735). Exact-head workflow run [#42232988239](https://github.com/Luminous-Dynamics/symtropy/actions/runs/42232988239) passed the upgraded schema and semantic validation on 6 November 2026.

The branch now validates eight task-specific manifests:
1. Engine comparison
2. Human fidelity (HUMSIM)
3. Cross-scale resilience
4. Deliberation trace audit
5. Opponent-roster / non-transitive game
6. Causal collaboration
7. Full configuration sensitivity
8. Matched causal-path qualification

Each manifest binds a **configuration passport** (canonical decision configuration and digest) and an **execution-stack inventory** (scheduler, parser, validator, retry/fallback, exposure, RNG and event capture). The validator rejects stale configuration digests, hidden fallback, unseeded RNG under a validity tier, unknown scheduler bindings and dropped events under fail-closed capture.

This is the important architectural boundary:
- **Humanity Atlas** owns exact run identity, configuration, outputs and evidence receipts.
- **CivBench MAD** owns the experimental distribution, observational/resampling/independent unit declarations, metrics, baselines, perturbations, controls, holdouts and claim ceiling.
- **Symtropy** owns executable model/reference implementations.
- **Symthaea** supplies candidate cognitive/social policies; it does not define ground truth.
- **Mycelix** owns provenance, authority, consent, contestability and evidence stewardship.
- **Sol-Atlas** owns source-backed spatial/temporal interpretation and map layers.

The branch remains a draft and its eight manifests are Tier-1 synthetic references. The schema pass is not an empirical validity result.

## 22. Explicit benchmark gates for claims beyond current practice

| Claim tier | Required evidence | Target for this programme |
|---|---|---|
| T0 integrity | Schema, type, source identity, unit, missingness and fail-closed validation | Zero known silent coercions or unsupported zero substitutions |
| T1 reference | Hand-calculated oracle, negative/positive controls, deterministic replay | Exact agreement on every declared reference fixture |
| T2 empirical distribution | Licensed source data, source-pinned splits, independent units, repeated measurement and calibrated uncertainty | Reproduce the strongest protocol-matched baseline first; then positive paired improvement with no unexplained material stratum regression |
| T3 external generalization | Held-out participant, site, region, period, source family or opponent roster | Report transfer degradation and uncertainty; do not pool in-domain and external results |
| T4 intervention validity | Matched counterfactuals, mechanism ablations, negative controls and independently measured outcomes | No qualified effect when a required mechanism path is disconnected or placebo-only |
| T5 operational readiness | External pilot/HIL, incident evidence, governance/appeal process and domain-specific acceptance | No Tier-5 claim from simulation-only evidence |

### Concrete aspirational targets (not current results)

- **HUMSIM:** reproduce SimBench's official score and 20 per-dataset breakdown; only then target +5 absolute points over the 40.80/100 published reference, with paired uncertainty and no unexplained regression above 2 points on any task or declared cohort. Also compare against SOUL/HumanStudy-Bench on their own protocols.
- **Configuration robustness:** exact zero semantic-order flips in the deterministic reference; in empirical group/decision studies, measure all facets and require every decision-relevant configuration change to trigger a re-audit. Do not demand invariance to legitimate changes in evidence or preferences.
- **Causal mechanisms:** positive-control propagation plus null negative controls; every qualified treatment effect must be supported by a contiguous path, mediator ablation, matched exogenous state and independent outcome receipt.
- **Resilience:** preserve separate water/energy/material/ecological units; report service-floor duration, worst-group loss, stock depletion, recovery time/quality and compound-shock interaction. Add source-calibrated ensembles and held-out regions before prediction claims.
- **Multi-agent:** publish a roster × strategy performance surface, not a global leaderboard; report cycles, exploitability, regret, rank stability and performance on sealed rosters.
- **Civilizational forecasting:** compare every model against transparent historical-mean, random-walk, persistence, linear and domain-specific baselines under time-forward validation. Use proper scores (Brier/log loss/CRPS) and calibration; report the full outcome vector rather than a pooled CVS.

The programme should claim to beat the state of the art only on a named benchmark, fixed evaluation protocol, held-out split and uncertainty-aware metric. A strong synthetic software kernel is an integrity achievement, not SOTA.

## 23. Immediate research priorities

1. Close exact-head CI and review the causal-path reference; don't call the matched path qualified until its full ledger-backed execution passes.
2. Extend MAD's factor manifests and configuration passports to every new benchmark before adding more simulation code.
3. Reproduce at least one external human-behaviour benchmark (SimBench or HumanStudy-Bench) under its official data, licence, scoring and split rules.
4. Extend the deterministic resilience fixture to parameter/shock ensembles, then source-calibrated regional hindcasts and held-out basins.
5. Reproduce the 54-agent non-transitive benchmark before making model-family rankings; keep our RPS control as a conformance test.
6. Convert the Shashe–Limpopo HIST-001 case into an uncertainty-preserving archaeological data ingestion and held-out-site reconstruction benchmark.

The governing rule remains: no increase in sophistication, aggregate score, collective intelligence proxy or simulation realism is allowed to compensate for missing evidence, disconnected mechanisms, poor calibration, or worse outcomes for a hidden subgroup.


### Component-digest semantics and exact causal-path qualification (6 November 2026)

The eight-fixture MAD branch [PR #3771](https://github.com/Luminous-Dynamics/symtropy/pull/3771) now canonicalizes every execution-stack component descriptor using `sorted-key-value-lines-v1` over:
- `component_id`;
- role;
- version;
- behavior-affecting flag;
- notes.

The validator recomputes each descriptor digest. This establishes the identity of the *declared component inventory*, not a signature of compiled code or proof that runtime behavior matches the declaration. Source commit, runtime artifacts and exact outputs remain tied to the Humanity Atlas run receipt. The exact-head all-manifest run [#42234375524](https://github.com/Luminous-Dynamics/symtropy/actions/runs/42234375524) passed: 8 manifests across 8 benchmark IDs, schema and semantic checks, configuration passports, execution-stack references and injected negative controls.

The matched causal-path implementation also passed exact-head hosted CI in [run #42233601189](https://github.com/Luminous-Dynamics/symtropy/actions/runs/42233601189): 14 library tests, 1 binary test, formatting, strict Clippy, locked compilation, ledger execution, mediator ablation, conservation and counter-only negative control. Draft [PR #3803](https://github.com/Luminous-Dynamics/symtropy/pull/3803) remains a deterministic synthetic reference; it does not establish an empirical policy effect.


## 15. Long-horizon activity fidelity and temporal-scale trade-offs

**Cheng et al. (EMNLP 2026), "Measuring the Behavioral Fidelity of Long-Horizon Human Activity Simulations".**
https://aclanthology.org/2026.emnlp-main.625/

The study compares six simulation approaches on a 43-hour multi-camera office activity dataset. Statistical transition and time-of-day priors improve some aggregate activity and local transition distributions but can over-segment routines and suppress within-person variability.

**CivBench response:** extend HUMSIM with event/local/daily/long-horizon views; score transition fidelity and dwell-time distributions alongside within-person variance, routine fragmentation and temporal autocorrelation. Do not reward a simulator for matching group-level activity by making every individual repetitive. Real multi-camera data requires explicit licensing, privacy and ethics gates.

Implementation: [CIVDYN-HUMSIM-001 issue #3665](https://github.com/Luminous-Dynamics/symtropy/issues/3665).


## 16. Whole-of-event service cascades and knowledge resilience

### From asset failure to downstream service disruption

CIVDYN-CASCADE-001 ([issue #3894](https://github.com/Luminous-Dynamics/symtropy/issues/3894), [draft PR #3896](https://github.com/Luminous-Dynamics/symtropy/pull/3896)) adds a typed service-dependency graph with ALL/ANY redundancy, progressive functionality, sustained recovery and counterfactual component criticality. It distinguishes internal infrastructure nodes from beneficiary service endpoints to prevent double-counting. The reference is synthetic and does not replace specialist physical network solvers.

Research anchors:
- Hoff et al. (2025), coupled power/water network cascades and 120,000 stochastic simulations: https://doi.org/10.1061/AOMJAH.AOENG-0045
- Tong et al. (2026), Fault Tree + Dynamic Bayesian Network for water/electricity/road resilience: https://doi.org/10.1016/j.ress.2026.112541
- Zhang et al. (11 October 2026), infrastructure resilience state-of-the-art review: https://doi.org/10.1061/NHREFO.NHENG-2965
- Schneider et al. (2025), DISruptionMap for cascading dependent-service disruption: https://doi.org/10.1038/s41598-025-89469-0

### Knowledge as durable capability, not content diffusion

CIVDYN-KNOW-001 ([issue #3897](https://github.com/Luminous-Dynamics/symtropy/issues/3897), [draft PR #3898](https://github.com/Luminous-Dynamics/symtropy/pull/3898)) separates delivery, acquisition, comprehension, performance, delayed retention, independent teaching, evidence lineage, source-family independence, and archive recovery.

Research anchors:
- Mackintosh et al. (19 September 2026), intentional knowledge transmission and cumulative culture: https://doi.org/10.1038/s41598-026-62734-6
- Walker & Fay (21 February 2026), attention filtering and external representations in large-group cumulative culture: https://doi.org/10.1038/s41598-026-40973-x
- "Propagation and preservation of AI-discovered problem-solving strategies in human culture" (Nature Communications, 2026): https://doi.org/10.1038/s41467-026-76113-2
- "On the transmission of texts: Written cultures as complex systems" (PNAS Nexus, 7 July 2026): https://doi.org/10.1093/pnasnexus/pgag207

The implementation uses synthetic traces only; it does not reproduce these human experiments or historical manuscript corpora.


## 15. Institutional authority and effective voting power

CIVDYN-AUTH-001 (#3684) now has a first exact reference in [draft PR #3685](https://github.com/Luminous-Dynamics/symtropy/pull/3685). It enumerates small weighted councils and reports nominal weights, normalized/raw Banzhaf power, Shapley–Shubik pivotality, minimal winning coalitions, veto/dictator status, HHI and Gini. A separate permission-topology audit records caller-declared role conflicts, single points of control and appeal independence. The exact enumerator is capped at 20 actors; no universal democracy/legitimacy score is emitted.


## 15. Decision robustness: from output uncertainty to decision quality (7 November 2026)

**The new gap:** parameter sensitivity identifies which assumptions move outputs; it does not tell us whether an intervention remains defensible when those assumptions vary. We now separate these questions in [CIVDYN-ROBUST-001 #4020](https://github.com/Luminous-Dynamics/symtropy/issues/4020), implemented in [draft PR #4024](https://github.com/Luminous-Dynamics/symtropy/pull/4024).

**Research comparisons**
- Shavazipour & Stewart, *A novel multi-stage multi-scenario multi-objective optimisation framework for adaptive robust decision-making under deep uncertainty*, Omega 138 (January 2026), DOI 10.1016/j.omega.2025.103405. It extends robust multi-objective decisions with risk attitudes and moving-horizon adaptation. https://doi.org/10.1016/j.omega.2025.103405
- *Cost-effective and reliable power system transitions under deep uncertainty*, IJEPES (2026), DOI 10.1016/j.ijepes.2026.111766. It contrasts RDM, stochastic programming, min–max robust optimization and min–max regret, showing that lower average cost need not mean lower regret or better reliability. https://doi.org/10.1016/j.ijepes.2026.111766
- Yamao et al., *Robust Decision-Focused Learning via Worst-Case Regret Minimization*, UAI 2026 / PMLR 337. It emphasizes that predictive accuracy can diverge from decision quality under measurement uncertainty and deployment distribution shift. https://proceedings.mlr.press/v337/yamao26a.html
- IPCC AR6 WGII Chapter 17 summarizes low-regret and adaptation-pathway approaches, including signposts and triggers. https://www.ipcc.ch/report/ar6/wg2/chapter/chapter-17/

**CivBench response:** compare all policies under the same parameter/shock points; evaluate non-compensatory service constraints; calculate regret separately for water, energy, coverage, breach and recovery outcomes; retain ties, unknown values and policy-switch counts. The 30 LHS rows are finite deterministic design points, not calibrated probabilities or stochastic replications. "Robust-feasible" means feasible at every declared point only, not proof over a continuous uncertainty set.

Current exact-head qualification for PR #4024 is in progress. No new robust-policy result is claimed yet.


## 15. Exact-head research status and next frontier (7 November 2026)

The current programme has moved from isolated prototypes to several composable evaluation slices:

| Research slice | Current evidence | Claim ceiling |
|---|---|---|
| Human-fidelity micro/population/trajectory | HUMSIM PR #3668 green; participant-cluster bootstrap PR #4015 green | Synthetic conformance and uncertainty implementation, not external human validity |
| Deliberation and procedural audit | PR #3682 green | Recorded trace audit, not a reproduction of human facilitation effects |
| Matched causal-path qualification | PR #3803 green | Ledger-backed causal-path and ablation controls, not a complete empirical causal-effect estimate |
| Resilience uncertainty | PR #3996 green | Declared finite parameter design and Morris screening |
| Robust policy comparison | PR #4024 green on exact head; 30-point LHS, 3 shocks, 4 policies | Finite-design service feasibility and per-metric native-unit regret only |
| Collective-risk→physical ledger integration | PR #4037 exact-head workflow #42368221801 passed; 14 library tests, ledger replay and CSV/evidence checks | Synthetic CRG action replay through conserved energy-transfer accounting only |

Green here means the exact commit completed the configured software workflow; it does not upgrade the scientific claim ceiling in the last column.

### What changed in the industry landscape

**Validation standardization is now the primary research frontier.** Puelma Touzel et al. (ICML 2026) explicitly call for consolidation around shared benchmarks, open data and reproducible evaluation protocols rather than multiplying unvalidated social simulators. https://proceedings.mlr.press/v306/puelma-touzel26a.html

**Human-simulation scores remain far from a reliable universal proxy.** SimBench reports 40.80/100 for its best tested model across 20 datasets, with weaker results on higher-entropy choices and particular demographic groups. https://proceedings.iclr.cc/paper_files/paper/2026/hash/2fc4d6a6400addda7493b9db258c34e0-Abstract-Conference.html ReliMap subsequently shows R1 individual reliability and R2 population reliability can move in opposite directions, while population coverage reduces variance without necessarily reducing systematic bias. https://aclanthology.org/2026.emnlp-main.1269/

**Evaluation context itself is part of the model.** FairFund-Bench shows that rating, ranking and allocation formats and single-versus-batch presentation can change both measured magnitude and direction of allocation disparity. https://aclanthology.org/2026.findings-emnlp.1340/ The new audit therefore treats task, context, order, framing and transparency as explicit factors.

**Resilience needs trajectory and uncertainty evidence.** The 2026 systematic review of 121 SES studies reports frequent static assessments and weak treatment of uncertainty and cross-scale shocks, but it does not evaluate the predictive accuracy of individual models. https://doi.org/10.1016/j.ecolind.2026.114792

**There is no universal top agent in non-transitive environments.** A 2026 Scientific Reports RPS benchmark compares 54 agents from 18 archetypes, with 500-round double round-robin matches over 10 seeds, and reports opponent-pool-dependent rankings and 134 three-cycles. https://www.nature.com/articles/s41598-026-55417-9

### The next engineering objective

The most valuable next step is not another standalone agent model. It is an evidence-qualified loop:

`policy → action trace → ledgered physical transfer → service/ecological outcome → matched counterfactual → uncertainty-qualified decision`

The CRG-to-ledger bridge in PR #4037 begins that loop. The robust-policy analysis in PR #4024 supplies metric-specific comparison over a declared parameter design. The causal-path kernel in PR #3803 supplies mediator/ablation checks. The next iteration should combine these as one frozen run protocol without duplicating the ledger, run envelope or evidence registry.

No result should be called “better than SOTA” unless it reproduces a specific published protocol and exceeds it on a held-out test with compatible metrics, uncertainty and non-regression checks. Synthetic reference performance is not a leaderboard comparison.


### Exact-head bridge qualification (7 November 2026)

[PR #4037](https://github.com/Luminous-Dynamics/symtropy/pull/4037) now passes exact-head workflow [#42368221801](https://github.com/Luminous-Dynamics/symtropy/actions/runs/42368221801) on commit `b48fa8e324f64c95ceb3bff7d7e7275ff08822d3`. It passes format, strict Clippy, locked compile, 14 library tests and a deterministic 15-roster action replay through the shared energy ledger. The run uploads a source-bound CSV bridge artifact. This qualifies software integration and accounting behaviour for its synthetic contract; it does not establish real-world energy conversion, human cooperation effects or predictive validity.
