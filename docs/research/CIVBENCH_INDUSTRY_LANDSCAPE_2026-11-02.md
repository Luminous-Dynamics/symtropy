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
