# Love-Based Economy: Rights-First Provisioning Architecture and Benchmark Contract

**Version:** 0.1  
**Status:** Research specification / proposal — not an implemented economic system  
**Owner:** Luminous Dynamics  
**Primary simulation surface:** Symtropy  
**Coordination surface:** Mycelix  
**Supporting systems:** Symthaea, Sol-Atlas, Praxis  
**Tracking:** [LBE-BENCH-001](https://github.com/Luminous-Dynamics/symtropy/issues/3041) · [LBE-BENCH-002: Love's shadow](https://github.com/Luminous-Dynamics/symtropy/issues/3090)  
**Related accounting defect:** [Symtropy issue #3043](https://github.com/Luminous-Dynamics/symtropy/issues/3043)

---

## 0. Executive thesis

A love-based economy should not be implemented as a currency, reputation system, universal score or moralized marketplace.

The proposed system is a **rights-first, polycentric provisioning economy**. It protects a foundation of essential needs, coordinates shared resources through accountable commons, supports cooperative and independent production, retains markets where useful, and subjects institutions to ecological limits and contestable governance.

Its primary object is not the transfer of tokens. It is the relationship between:

- needs that people are entitled to have met;
- real productive capacity and scarce resources;
- institutions that allocate, maintain and expand that capacity;
- reciprocal obligations and financial claims;
- governance powers, appeals and remedies;
- measurable outcomes for people and ecosystems.

The central hypothesis is that a rights-first hybrid economy can reduce unmet essential needs and improve resilience, agency and distribution without reducing productive capacity or exceeding ecological constraints. This is a hypothesis to test, not a result to claim.

## 1. Constitutional invariants

These are hard constraints, not weighted objectives. No productivity, trust, reputation, welfare or aggregate flourishing metric may compensate for violating them.

### 1.1 Dignity and entitlement firewall

Eligibility for essential services and equal civic standing must be independent of:

- MYCEL or any reputation / recognition score;
- SAP balances, wealth, collateral or token ownership;
- TEND balances, debt, contribution history or transaction volume;
- Phi, consciousness indicators, inferred intelligence or psychological profiles;
- political agreement, voting history, popularity or social connections;
- employment status, productivity, disability, caregiver status or ability to participate digitally.

Need and genuine physical constraints may inform service delivery and scarcity triage. They must not become a pretext for ranking human worth.

A property-based test must mutate every factor above while holding need and physical availability constant. The entitlement result must remain identical.

### 1.2 Due process

Every adverse allocation decision must expose, to the affected person:

1. the decision and the service/resource involved;
2. the applicable rule and its version;
3. the evidence used, with uncertainty;
4. the responsible institution or delegated authority;
5. an accessible route to correction and appeal;
6. the remedy and time limit.

No automated model may be the final authority for denying an essential service.

### 1.3 Privacy and data sovereignty

Need, health, care, disability, household and location data are sensitive. Collect the minimum necessary, separate identity from service records where feasible, provide non-digital access, and make data access and retention auditable.

Raw sensitive records must not be placed on a public DHT. Public verifiability should prove the integrity of a decision or aggregate, not expose the person who needed help.

### 1.4 Consent, exit and non-domination

Participation in optional economic associations must be voluntary. People must be able to leave, recover portable credentials and personal records, challenge obligations, and settle legitimate liabilities without losing essential rights.

A community cannot make access to necessities conditional on membership in its currency, platform, ideology or cooperative.

### 1.5 Ecological and material honesty

No ledger entry creates food, water, energy, housing, care capacity or ecological regeneration. Every resource commitment must be backed by a declared source, capacity, time horizon and delivery risk.

Externalized costs, including imported materials, emissions, water depletion and waste, remain part of the accounting boundary.

## 2. Economic architecture

The five layers are complementary coordination mechanisms, not a rigid hierarchy.

### Layer A — Protected essential foundation

Guarantee access to locally defined essentials: food security, safe shelter, water and sanitation, healthcare, education, energy for basic needs, connectivity and protection from violence.

Delivery can combine public provision, regulated providers, cooperatives, mutual aid and cash support. The guarantee concerns the outcome and access, not a single delivery ideology.

### Layer B — Commons and shared infrastructure

Manage resources that benefit from shared stewardship: land, water systems, local energy, public knowledge, tools, care infrastructure and common facilities.

Use clear boundaries, member-defined rules, monitoring, graduated responses to misconduct, dispute resolution and nested institutions. Local governance should not be mistaken for immunity from outside capture or unequal power.

### Layer C — Cooperative and mission-led production

Support worker, consumer, producer, multi-stakeholder and community-owned enterprises. Make access to patient capital, business support, procurement, training and technical infrastructure part of the design.

Ownership alone does not guarantee efficient production, decent work or fair internal distribution. Measure capital formation, reinvestment, productivity, job quality, management capability and resilience.

### Layer D — Independent enterprise and markets

Retain markets for many discretionary, differentiated and experimental goods and services. Protect entry, interoperability, competition and consumer choice. Prevent market income from becoming the gatekeeper to essential rights.

### Layer E — Polycentric coordination and ecological limits

Coordinate systems with cross-community effects: energy grids, watersheds, transport, public health, supply chains, ecological thresholds and shared reserves.

Apply subsidiarity: decide at the lowest level capable of handling the consequences, while giving higher-level institutions explicit, bounded responsibilities for spillovers and rights protection.

## 3. Mycelix economic instruments: clarify roles before expansion

The existing Mycelix Finance design describes three distinct instruments:

| Instrument | Intended role | Required boundary |
|---|---|---|
| MYCEL | Non-transferable recognition / credential substrate | Never a measure of human worth, an entitlement key or a general civic-weight multiplier |
| SAP | Transferable exchange / treasury / capital instrument | Issuance, redemption, reserves, demurrage and external settlement must be explicitly accounted for |
| TEND | Mutual-credit and time-exchange instrument | Complementary exchange rail, not a substitute for wages, public services or the essential rights floor |

The instruments should not be treated as interchangeable. A credential is not money; money is not a right; a mutual-credit balance is not proof of trustworthiness.

### 3.1 MYCEL

Retain domain-scoped recognition where it provides evidence for a specific responsibility—for example, a current credential to maintain a water pump. Do not aggregate such recognition into a universal rank that grants more rights, more political personhood or a greater claim to food, healthcare or shelter.

Where a decision needs expertise, use verified, relevant, current credentials as evidence for that task. Keep the decision, appeal and accountability separate from the credential.

### 3.2 SAP

Before SAP is used for real-world settlement, the system needs an auditable account of issuance, backing, redemption, liquidity, counterparty risk, reserves, fees, demurrage, custody, bridge failures and loss allocation.

Demurrage is not automatically egalitarian. It can affect low-income savers, working-capital buffers and people who cannot easily spend. Compare it with alternatives, publish incidence by balance cohort, and make it a governed policy choice rather than a universal default.

The current Mycelix Finance README explicitly notes that several SAP execution, UTXO shadow-write, checkpoint and external-settlement gates remain disabled pending validation. Preserve those fail-closed boundaries; a design description is not evidence of production settlement.

### 3.3 TEND

Time exchange can help make some forms of care and mutual aid visible. A one-hour-for-one-hour rule is simple and can express equal recognition, but it does not capture differences in training, risk, responsibility, equipment, scarcity or working conditions. It should complement, not replace, fair wages and professional services.

Mutual-credit systems need explicit credit limits, clearing rules, default handling, demurrage semantics, debt-relief funding and exit settlement. All changes must be recorded in a balanced journal.

**Accounting invariant:** for a zero-sum mutual-credit instrument, the sum of participant, treasury, clearing, issuance and forgiveness accounts must reconcile exactly after every committed event. If the system intentionally creates or destroys units, that operation must be explicit and auditable.

### 3.4 Existing implementation issue

The game-specific Symtropy routine in src/systems/economy.rs applies a minimum one-unit demurrage adjustment independently to every positive and negative balance while documenting that the system remains zero-sum. For the starting portfolio [+100, -50, -50], one interval can produce [+99, -49, -49], changing the total from zero to +1. Tiny balances are also adjusted by a full unit rather than by the nominal fractional rate.

This is a concrete source-level simulation accounting defect, not evidence that the separate Mycelix Finance SAP/TEND ledger has the same bug. In the standalone public repository, this routine is behind the mycelix feature, which is currently commented out in Cargo.toml because private integration dependencies are unavailable; the full monorepo feature composition must be checked separately. The defect is reproducible from the arithmetic, but no enabled-feature runtime test is claimed. Track and fix it independently with an isolated, testable accounting kernel and an exact feature / target / command record before using the system as an economic benchmark.

## 4. Technical architecture and division of responsibility

### Symtropy — physical and institutional simulation

Build a deterministic Rust benchmark harness around agents, households, enterprises, services, supply chains, commons resources, treasuries and governance rules.

Represent physical constraints explicitly: production rates, inventory, energy, water, transport, labour time, maintenance, degradation, queues and shock propagation. Money and trust may affect decisions, but neither creates physical capacity.

### Mycelix — coordination, provenance and rights-preserving records

Provide identity under user control, needs/offers matching, cooperative membership, mutual-aid commitments, treasury authorization, dispute and appeal records, scoped credentials and auditable settlement events.

The rights firewall belongs in the integrity / validation layer. It must not rely solely on a frontend promise or a policy setting that a privileged administrator can silently change.

### Symthaea — bounded decision support

Generate candidate policies, search for counterexamples, estimate uncertainty, compare scenarios and explain trade-offs. It may help identify unmet needs or operational bottlenecks.

It must not define human worth, assign moral eligibility, infer that a person deserves fewer rights, or make an unappealable allocation decision. A model's confidence is not authority.

### Sol-Atlas — spatial and infrastructure evidence

Represent where needs, resources, networks, capacity and physical bottlenecks exist. Track data quality, measurement uncertainty, ownership, update time and geographic coverage.

### Praxis — capability development

Help people access learning, credentials, apprenticeships, skills transition and practical knowledge. Learning records should expand opportunity, not become prerequisites for essential services.

## 5. LBE-Bench: benchmark suite

The benchmark should compare policy mechanisms, not merely compare token designs.

### 5.1 Baseline policy families

Each policy family must be configured with the same population, physical resource budget, initial infrastructure, exogenous shocks and evaluation horizon:

1. Market allocation with an explicit minimum safety net.
2. Means-tested transfer and service provision.
3. Universal basic services / direct public provision.
4. Unconditional cash transfer with market purchasing.
5. Worker or multi-stakeholder cooperative provision.
6. Community commons and mutual-aid coordination.
7. Hybrid rights-first provisioning with polycentric governance.

No baseline is a straw man. Implement realistic administrative costs, incentives, capacity constraints, failure modes and transition costs for each.

### 5.2 Scenario families

| ID | Scenario | What it tests |
|---|---|---|
| LBE-01 | Normal operation | Coverage, service quality, cost and participation |
| LBE-02 | Household income shock | Job loss, income interruption and arrears |
| LBE-03 | Essential-goods price spike | Whether purchasing power or direct provision protects access |
| LBE-04 | Supply shortfall | Fair rationing, substitution, appeals and capacity investment |
| LBE-05 | Energy / water interruption | Critical-load allocation, repair, reserves and cascading failure |
| LBE-06 | Care surge | Unpaid care burden, respite, staffing and continuity |
| LBE-07 | Credit stress | Defaults, liquidity, clearing, treasury solvency and contagion |
| LBE-08 | Cooperative failure | Leadership loss, capital shortage, succession and member exit |
| LBE-09 | Capture and collusion | Sybil identities, bribery, elite capture, retaliation and vote buying |
| LBE-10 | Governance emergency | Delegation scope, sunset, review, succession and abuse |
| LBE-11 | Data compromise | Linkability, sensitive-data exposure and unauthorized inference |
| LBE-12 | Externalized ecological costs | Imported emissions, water stress, waste and rebound effects |

For each scenario, pre-register shock size, duration, seed, affected population, available substitutes and recovery assumptions. Include expected events and adversarial edge cases.

### 5.3 Constitutional property tests

These are must-pass tests, not average scores:

- **Rights non-interference:** mutating wealth, currency balance, reputation, Phi, contribution or participation history never changes essential entitlement when need and physical availability are unchanged.
- **No score-based civic exclusion:** a person with no credential history, no device, no bank account or no prior contribution can still access the rights floor and due process.
- **Explainable adverse decisions:** every denial, queue displacement or shortage allocation includes its rule version, reasons, evidence, uncertainty and appeal route.
- **Ledger conservation:** exact balances reconcile across participant and system accounts; every mint, burn, transfer, demurrage charge and debt forgiveness is journaled.
- **Deterministic replay:** identical signed inputs, policy versions and seed produce identical simulation results and event digests.
- **Privacy boundary:** public records do not reveal raw sensitive records or permit unauthorized cross-domain linkage.
- **Exit and portability:** leaving an optional association cannot erase personal credentials, destroy access to unrelated essentials or create arbitrary debt.
- **Power limits:** no single actor can unilaterally change rights, mint spendable currency, suppress an appeal or extend emergency authority indefinitely.

### 5.3.1 Love's shadow: coercion and domination tests

Positive measures such as care delivered, trust, donations, participation or cohesion cannot compensate for coercion or rights violations. LBE-Bench must adversarially test benevolent paternalism, coerced reciprocity, compulsory participation, unpaid care extraction, social-credit capture, in-group closure, intimate surveillance, forced forgiveness, compassionate impunity, donor/provider capture, exit retaliation, emotional manipulation and conflict suppression.

The dedicated [LBE-BENCH-002 threat model](https://github.com/Luminous-Dynamics/symtropy/issues/3090) and [benchmark specification](https://github.com/Luminous-Dynamics/symtropy/pull/3092) define 20 adversarial scenarios (SH-01–SH-20), hard red-line invariants, metrics, baselines and acceptance rules. These are separate from the ordinary economic shock scenarios.

Add the following hard-gate properties:
- refusal of optional labour, gratitude, publicity, data collection or reconciliation has exactly zero effect on unrelated essential access;
- exit, dissent, complaint and appeal trigger no retaliation;
- no authority can waive a victim's remedy by invoking forgiveness or community harmony;
- care hours and costs shifted to unpaid households remain visible in the accounting boundary;
- a high-contribution actor remains auditable and subject to remedy;
- no donor, provider, majority faction or algorithm can unilaterally override the rights floor.

Report each breach individually; do not create a weighted "love score" or average away a severe failure.

### 5.4 Outcome metrics

Never compress the results into a single love score, Phi score or moral ranking.

**A. Material security**
- unmet essential-need hours by service category and severity;
- share of people with adequate access, not merely nominal enrollment;
- median and 95th-percentile time from request to delivery;
- false denials, non-delivery after approval, stockouts and repeat unmet need;
- affordability and out-of-pocket burden, where applicable.

**B. Distribution and fairness**
- service coverage and unmet need by income decile, household type, age, gender, disability, geography and other locally appropriate groups;
- bottom-decile and bottom-quintile outcomes;
- distribution of income, wealth, ownership and risk;
- exclusion and appeal reversal rates by group.

**C. Care and time**
- paid and unpaid care hours separately;
- distribution of unpaid care by gender and household role;
- time poverty, respite access and caregiver overload;
- care-worker pay, safety, training and social protection;
- continuity and quality of care.

**D. Productive capacity**
- essential output per unit of labour, energy, material and capital;
- service backlog and asset utilization;
- maintenance completion, downtime and replacement needs;
- enterprise survival, reinvestment, productivity and job quality;
- cost per successfully met need, including administration and transition cost.

**E. Agency and governance**
- meaningful participation and representation of affected groups;
- proportion of proposals that influence real resource allocation;
- time to resolve appeals and disputes;
- ability to exit, portability and consent withdrawal;
- concentration of ownership, agenda-setting, veto and administrative powers;
- privacy incidents and unauthorized data access.

**F. Resilience**
- coverage maintained under pre-registered shocks;
- time to restore essential service;
- days of critical reserves and supplier concentration;
- default contagion, treasury drawdown and recovery;
- distribution of losses and recovery time across households.

**G. Ecological performance**
- absolute and per-capita greenhouse-gas emissions;
- water withdrawal and water stress;
- material footprint, waste and repair / reuse rates;
- land-use and biodiversity effects where measurable;
- cross-boundary leakage and rebound effects.

**H. Financial integrity**
- liquidity and solvency under stress;
- payment failure, default and loss rates;
- mutual-credit clearing imbalance and time to clear;
- reserve adequacy and concentration;
- transaction cost, settlement latency and bridge exposure;
- distribution of demurrage, fees and debt forgiveness by balance cohort.

### 5.5 Reporting discipline

Report means, medians, tails, subgroup distributions, confidence intervals and missingness. Averages must not hide the people for whom the system fails.

Use multi-objective and Pareto analysis. Treat rights, due process and ecological thresholds as constraints. For outcomes that remain in tension, publish the trade-off surface and let accountable human institutions deliberate; do not hide value judgments inside arbitrary weights.

## 6. Initial benchmark targets

The targets below are proposed research gates. They are not evidence of current performance and should be locked after stakeholder review and statistical power analysis.

### Must-pass engineering and constitutional targets

| Measure | Initial target |
|---|---:|
| Property tests in which reputation, wealth, Phi or contribution alter essential entitlement | **0** |
| Essential-service adverse decisions with a reason, rule version and appeal path | **100%** |
| Mutual-credit transactions reconciled exactly, including system accounts | **100%** |
| Deterministic replay mismatches for identical inputs | **0** |
| Unreviewable unilateral rights changes or non-expiring emergency delegations | **0** |
| Unauthorized exposure of raw sensitive records in public data | **0** |

### Pilot outcome targets

For a bounded real-world pilot, compare against the strongest feasible baseline under a matched resource envelope:

- at least **10% relative reduction in unmet essential-need hours** over the pre-registered evaluation period;
- no pre-specified protected subgroup experiences a material worsening in essential coverage or service quality;
- no breach of the pilot's absolute ecological constraints;
- full reporting of operating, administration, capital, maintenance and transition costs;
- independently reviewable evidence that the improvement is not merely a transfer of cost or workload to unpaid caregivers, another community or a future period.

A 10% improvement is a minimum research ambition, not a universal guarantee. A pilot that misses it should publish why and which mechanism failed.

## 7. Evaluation protocol

1. **Specify the causal question.** What mechanism is being tested, for whom, over what period, and against which alternative?
2. **Freeze the benchmark contract.** Publish metric definitions, service standards, policies, subgroup definitions, seeds, shock schedules and exclusion rules before running comparisons.
3. **Fingerprint all inputs.** Record code SHA, policy/config digest, dataset license and digest, model version, random seeds and environment.
4. **Calibrate before predicting.** Fit baseline capacity and behaviour to observed data; reserve separate data for out-of-sample evaluation.
5. **Run deterministic and stochastic tests.** Exact property tests first; then repeated, seeded simulation and sensitivity analysis.
6. **Red-team the policy.** Include strategic participants, institutional capture, administrative mistakes, data gaps, collusion and adversarial shocks.
7. **Publish failures.** Include negative controls, rejected hypotheses, missing data, confidence intervals and deviations from the protocol.
8. **Pilot only with consent and oversight.** Obtain appropriate community governance, legal, privacy and independent evaluation review before using real service data or allocating real resources.

A simulator is useful for discovering failure modes and narrowing choices. It does not prove that a community, institution or real economy will behave like the model.

## 8. First bounded pilot candidate

A sensible first domain is **community energy resilience linked to essential services**, rather than attempting to redesign an entire national economy.

A pilot could examine a defined solar / storage service arrangement serving critical loads such as water pumping, food refrigeration, communications and care facilities. **Do not assume that community ownership is the winning design.** A 2025 review of 23 Sub-Saharan African mini-grid cases found that community ownership was not, by itself, a key success factor; for-profit models tended to perform better on technical and economic sustainability, while ownership forms had different strengths and weaknesses. A rural South African case study based on 44 stakeholder interviews found stronger participation in technical and social domains than in governance and economic inclusion, with limited decision influence and formal economic opportunities for women. These findings are context-specific, but they invalidate a simplistic equation of cooperative ownership with energy justice.

Treat ownership / operator model as one experimental factor, independent of participation and accountability. Compare, where legally and operationally feasible:
- municipal / public provider;
- community or multi-stakeholder cooperative;
- non-profit or social-enterprise operator;
- regulated private / for-profit operator;
- public-private or multi-actor partnership.

For every model, measure the same service standard, subsidy and capital assumptions, lifecycle costs, risk allocation, maintenance capability, customer protections and ecological boundary. Separately measure four participation domains: governance influence, technical engagement and capability, economic inclusion / benefit-sharing, and social inclusion. A provider can be privately operated and still be accountable to users; a community-owned asset can still be captured by local elites or leave women without decision power. The benchmark should evaluate actual control, access, voice, compensation and remedies—not use the ownership label as a proxy.

Questions to test:
- Can the system preserve critical services during a supply interruption?
- Are tariffs, rationing and service standards transparent and appealable?
- Are low-income households protected without a reputation or contribution gate?
- Who controls dispatch, budgets, procurement, data and tariff changes?
- Who performs maintenance and care work, who is paid, and who gets training or progression?
- Are user and worker representatives able to change decisions, not merely attend consultations?
- Can the operator maintain reserves, replacement funding and technical capability?
- Does the arrangement improve access and resilience at a defensible lifecycle cost?
- Are imported material, water, emissions and ecological costs counted beyond the immediate site?

Use Sol-Atlas for spatial and infrastructure baselines, Symtropy for deterministic scenario comparisons, Mycelix for consent-based commitments and governance evidence, and Symthaea for bounded scenario exploration. Do not make the simulated system the authority over real households.

A later pilot can add food distribution, housing maintenance or care services once accounting and governance controls have been demonstrated.

### Beyond-GDP measurement crosswalk

The UN High-Level Expert Group's final Beyond GDP report, released on 7 May 2026, proposes a 31-indicator global dashboard covering progress for people and planet. LBE-Bench should map its indicators to this existing statistical framework, OECD well-being measures and relevant national statistics before inventing new metrics. The incremental contribution should be operational: connect selected outcomes to service-level events, physical capacity, subgroup incidence, institutional powers and redress. Record where LBE metrics are exact matches, narrower operational measures, or genuinely new proposals. Do not imply that a private benchmark replaces national statistical systems.

For the care scenarios, preserve the ILO's 5R structure (recognize, reduce and redistribute unpaid care; reward and represent paid care workers). The ILO's 2025 modelling estimated that employment-intensive investment could support up to 299 million care-economy jobs by 2035 under the scenarios studied; this is a modelled potential, not a guaranteed job forecast.

## 9. What better than state of the art should mean

The relevant state of the art is distributed across several fields:

- OECD well-being measurement provides a mature multidimensional framework and disaggregation guidance.
- ILO research provides care-economy measures, labour standards and investment methods.
- World Bank adaptive social protection connects programmes, data, finance and institutions to shock response.
- Ostrom-informed commons research provides tested institutional principles for shared resource governance.
- Worker-ownership and cooperative research provides evidence about job quality, resilience, capital formation and firm performance.
- WIR and Sardex show that mutual credit can support exchange and liquidity in defined business networks.
- Participatory-budgeting research emphasizes real decision rights, inclusion, transparency and follow-through.

LBE-Bench should not claim to supersede any one of these. Its differentiator is an executable comparison across them, combining rights invariants, material constraints, distributional outcomes, governance attack tests, accounting integrity and ecological limits in one reproducible protocol.

The objective is to **outperform the best relevant baseline on pre-registered outcomes**, not to claim superiority by definition.

## 10. Research and implementation sequence

### P0 — Contract and integrity
- Adopt this benchmark contract for review.
- Audit all Mycelix governance, finance, commons, care and mutual-aid pathways for rights / score coupling.
- Fix the Symtropy TEND per-account rounding / conservation defect in its own game-economy scope.
- Define canonical ledger events for issuance, demurrage, clearing, debt relief and reserves.

### P1 — Rust benchmark kernel

Create a deterministic, dependency-light Rust benchmark module with:
- typed needs, supplies, service capacities and resource constraints;
- household / enterprise / community agents with bounded, inspectable state;
- pluggable allocation policies for each baseline;
- an event journal and exact accounting checks;
- scenario manifests, seeded shocks and reproducible output bundles;
- property tests for constitutional invariants.

Avoid making the initial benchmark dependent on Holochain conductors, GPU execution or external runners. Test the pure model locally first; add integration adapters only after its semantics are stable.

### P2 — Calibrated scenarios

Start with public aggregate data and synthetic households. Add South African Statistics South Africa household, poverty, labour and service-access data where licensing and metadata permit. Clearly separate observation, estimation and simulation assumptions.

### P3 — Independent validation

Invite economists, cooperative operators, care workers, public-service practitioners, local communities, ecologists and security researchers to review the protocol. Require independent replication before making comparative performance claims.

### P4 — Small real pilot

Select one service domain and a willing community partner. Use a pre-registered comparison, independent oversight, a defined exit route and a public failure-reporting commitment. Do not place essential public services at risk to test an experimental token.

## 11. Evidence base and design references

### Measurement and public provisioning
- [OECD Well-being Data Monitor](https://www.oecd.org/en/data/tools/well-being-data-monitor.html) — multidimensional current well-being and future resources.
- [OECD Guidelines on Measuring Subjective Well-being, 2025 update](https://www.oecd.org/en/publications/oecd-guidelines-on-measuring-subjective-well-being-2025-update_9203632a-en.html) — distributional reporting, group comparisons, data sovereignty and context-sensitive measurement.
- [UNECE Guidelines on Measurement of Well-being, September 2025](https://unece.org/info/publications/pub/407877) — ten dimensions and operational indicator guidance.
- [UN High-Level Expert Group on Beyond GDP, 2026](https://unctad.org/publication/counting-what-counts-compass-progress-people-and-planet) — global framework emphasizing well-being, equity, sustainability and spillovers.
- [World Bank: Adaptive Social Protection](https://www.worldbank.org/en/publication/human-capital/publication/adaptive-social-protection-building-resilience-to-shocks-key-findings) — programme, information, finance and institutional building blocks.
- [World Bank: Delivery Systems for Social Protection, 2025](https://www.worldbank.org/en/brief/2025/09/19/digital-delivery-systems) — access, registration, payment and case-management constraints.

### Care and South African context
- [ILO: Employment-intensive investments for advancing decent work in the care economy, 16 July 2025](https://www.ilo.org/publications/employment-intensive-investments-advancing-decent-work-care-economy) — care investment and the 5R framework.
- [ILO World Social Protection Report 2024–26, Chapter 5](https://webapps.ilo.org/static/english/reports/flagship/world_social_protection_report_2024-26/Chapter%2005.html) — care workers, social protection and distribution of unpaid care.
- [Statistics South Africa: Poverty Trends, 11 December 2025](https://www.statssa.gov.za/?p=19078) — poverty estimates from the 2022/23 Income and Expenditure Survey.
- [Statistics South Africa: General Household Survey 2025 key findings](https://www.statssa.gov.za/?PPN=P0318&SCH=74563&page_id=1856) — food security, water, sanitation, electricity and social grant access.
- [Institute for Economic Justice: SRD grant exclusion, 14 March 2025](https://iej.org.za/resource/research-report/srd-grant-exclusion-in-south-africa/) — evidence of administrative exclusion among a selected sample of excluded applicants; the figures must not be misrepresented as national prevalence.
- [Oxford Open Energy: Community participation and the viability of decentralized renewable energy systems in rural South Africa, 14 August 2025](https://academic.oup.com/ooenergy/article/doi/10.1093/ooenergy/oiaf006/8234320) — 44 stakeholder interviews; distinguishes technical / social participation from governance / economic decision-making.
- [Oemmelen, Page & Parikh (2025), Shared Power, Shared Benefits?](https://doi.org/10.1016/j.rser.2025.115948) — review of 23 Sub-Saharan African mini-grid cases; ownership form alone is not a success guarantee, and participation must be assessed across multiple actors and domains.

### Commons, cooperatives and complementary finance
- [Agrawal, Erbaugh & Pradhan (2023), The Commons, Annual Review of Environment and Resources](https://doi.org/10.1146/annurev-environ-112320-113509) — research directions include power, equity and causal methods.
- [Cox, Arnold & Villamayor Tomás (2010), Review of Design Principles for Community-Based Natural Resource Management](https://hdl.handle.net/10535/7430) — review of 91 studies of Ostrom's principles.
- [UC Berkeley Institute for the Study of Employee Ownership and Profit Sharing, Literature Review on Worker Ownership (2025)](https://haas.berkeley.edu/ibsi/research/literature-review-on-worker-ownership/) — conditional findings on resilience, job quality, productivity and limitations.
- [Abebe & Gebreeyesus (2024), Worker Cooperatives in Manufacturing](https://doi.org/10.1086/726179) — evidence that capital formation, upgrading and managerial capability matter to cooperative performance.
- [Stodder & Lietaer (2016), The Macro-Stability of Swiss WIR-Bank Credits](https://doi.org/10.1057/s41294-016-0001-5) — countercyclical use of a complementary credit network.
- [Bertoni et al. (2018), Cyclic Motifs in the Sardex Monetary Network](https://www.nature.com/articles/s41562-018-0450-0) — network patterns in one established mutual-credit ecosystem.
- [Lafuente-Sampietro & Reyns (2025), Territorial Characteristics and the Emergence of Mutual Credit Systems: Sardex](https://doi.org/10.1080/2833115X.2025.2539717) — territorial proximity and local economic conditions in network diffusion.

---

## 3.5 Live Mycelix access-control findings

A source-level review on 31 October 2026 found a mismatch between the rights-first design and several active request-intake call paths:

The source audit found that several Commons coordinator paths called require_civic with civic_requirement_basic(), whose requirement is CivicTier::Participant. The profile tier function derives Participant at a score of 0.30, and the active gate is documented to reject ineligible callers.

The remediation is now split by verified authorization boundary:
- [Mycelix PR #3781](https://github.com/Luminous-Dynamics/luminous-dynamics/pull/3781) removes all nine civic-score gates from the MutualAid Needs coordinator. This includes need intake and the user's own withdraw/participant actions. Source inspection confirmed authenticated author binding and owner/participant checks in coordinator/integrity paths; it remains a draft and has not been Rust/Holochain tested here.
- [Mycelix PR #3783](https://github.com/Luminous-Dynamics/luminous-dynamics/pull/3783) removes the profile gate only from Care Timebank service-offer and service-request intake, where integrity callbacks bind provider/requester to the action author. It deliberately retains exchange-settlement and rating gates pending separate consent/authorization review; the settlement gate is not treated as proof of bilateral consent.
- MutualAid Requests remains unchanged pending [issue #3780](https://github.com/Luminous-Dynamics/luminous-dynamics/issues/3780): its coordinator copies caller-supplied requester/offerer DIDs, while the integrity path checks syntax rather than binding them to authenticated authors, and update operations lack verified caller authorization.

The parent rights-floor requirement remains tracked in [Mycelix issue #3779](https://github.com/Luminous-Dynamics/luminous-dynamics/issues/3779). Do not reintroduce a civic-score gate as a substitute for identity binding, anti-spam protection or participant consent.

The same inspection found that the MutualAid Requests coordinator copies a caller-supplied requester DID into AidRequest, while the integrity path checks basic DID syntax but does not bind the claimed requester to the committing agent. Removing the civic gate must not leave this impersonation surface unaddressed. Use authenticated author identity for self-service and a separate signed, scoped delegation for assisted intake. This is tracked in [Mycelix issue #3780](https://github.com/Luminous-Dynamics/luminous-dynamics/issues/3780).

The governance-policy simulation work is tracked separately in [CIV-GOV-002 / issue #3766](https://github.com/Luminous-Dynamics/luminous-dynamics/issues/3766), which includes equal civic standing and a rights-floor hybrid as candidate model families. This specification does not claim that voting policy is already corrected.

The MutualAid Needs path has two further consent/state issues independent of civic scoring: [#3785](https://github.com/Luminous-Dynamics/luminous-dynamics/issues/3785) tracks the fact that one participant can currently set the shared match status to Accepted without a distinct second-party acceptance receipt; [#3784](https://github.com/Luminous-Dynamics/luminous-dynamics/issues/3784) tracks the apparent conflict between `confirm_fulfillment` and the integrity validator's blanket rejection of Fulfillment updates. These are source-level findings and require HDI tests.

These are source-level findings, not conductor-tested claims about a deployed network. Verify all affected DNA compositions, alternate intake routes, integrity validators and live call flows before making deployment-level statements.

---

## 12. Explicit non-claims

This specification does not establish that:
- love-based or cooperative economies always outperform markets;
- reputation, tokenization, demurrage or mutual credit are inherently fair;
- a simulator can replace economic evidence, democratic legitimacy or local knowledge;
- Phi measures moral worth, civic competence or entitlement;
- current Mycelix Finance settlement is production-ready;
- a pilot can safely replace public services or social protection.

The programme succeeds only if its measured results justify its claims, its institutions protect people who cannot contribute, and its own power can be challenged and corrected.
