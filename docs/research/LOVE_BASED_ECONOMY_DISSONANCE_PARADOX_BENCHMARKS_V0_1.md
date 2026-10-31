# LBE-BENCH-003: Constructive Dissonance and Paradox-Preserving Decisions v0.1

**Status:** research and benchmark specification  
**Companion code:** `crates/lbe-kernel/src/dissonance.rs` on the kernel draft branch  
**Tracking:** [issue #3110](https://github.com/Luminous-Dynamics/symtropy/issues/3110)  
**Principle:** preserve genuine disagreement; resolve what evidence can resolve; decide what must be decided; never erase dissent to manufacture harmony.

## 1. The central distinction

An economy and its institutions must process disagreement. Their goal should not be zero dissonance, permanent consensus, maximum debate, or maximal "integrative complexity". The goal is the capacity to learn, choose and coordinate while protecting the people and evidence that make disagreement visible.

### Constructive dissonance

A procedural condition in which materially different positions can be expressed, fairly represented, challenged with evidence, and retained in a decision record. Participants have meaningful voice, safe dissent, and access to correction and appeal. The process remains capable of updating beliefs and changing decisions.

Constructive conditions are not proof that the disagreement is harmless or that all parties have equal power. They are safeguards that make inquiry possible.

### Corrosive dissonance

A process in which disagreement causes or conceals harm: intimidation, coercion, retaliation, agenda capture, evidence suppression, deliberate misrepresentation, status competition, discriminatory exclusion, surveillance, or unequal distribution of the emotional and facilitation burden.

Corrosive status must not be inferred from tone, emotional intensity, political identity, or text classification alone. Record reported hazards, evidence state and independent findings separately.

### The critical caution

The same disagreement may move between states over time. A genuine task dispute can become status conflict; a safe forum can be captured; a decision that initially protects a minority can later become paternalistic. Assess the process trajectory at defined intervals, not just a one-off label.

A high-conflict environment is not automatically intellectually productive. A low-conflict environment is not automatically healthy: it may reflect agreement, fatigue, fear, exclusion or suppressed dissent.

## 2. Do not confuse a contradiction, a trade-off and a paradox

Use explicit tension types:

| Type | Meaning | Appropriate treatment |
|---|---|---|
| Empirical disagreement | Incompatible claims about observable facts | Test, replicate, update or resolve by evidence within stated limits |
| Normative conflict | Competing rights, values or legitimate priorities | Public justification, rights constraints, plural representation and accountable choice |
| Resource trade-off | Demands exceed a finite physical/social capacity | State the constraint, expose distributional choices, ration fairly, invest or substitute |
| Temporal tension | Short-term and long-term requirements interact | Sequence, hedge, stage, or use a reviewable transition |
| Structural paradox | Persistent, contradictory and interdependent demands | Preserve both poles; integrate where possible, oscillate, sequence, contextualize or hold open |
| Interpretive difference | Distinct frames, experiences or meanings | Maintain multiple accounts; test factual components without erasing situated knowledge |

A paradox is not simply "two opinions". It is a persistent relationship between demands that are contradictory yet interdependent. Examples include autonomy and collective resilience, innovation and reliability, local self-determination and system-wide coordination, privacy and accountability, immediate relief and long-term capacity.

Do not label every conflict a paradox. A testable factual error is not made true by being paired with a competing claim. A finite resource constraint is not dissolved by calling it "both/and". And two incompatible rights claims may require transparent adjudication rather than a false synthesis.

## 3. Hold paradox without collapsing to simplistic truths

Use a **plurality-preserving decision record** with:

1. Stable IDs for each position, source/constituency, statement, rationale, evidence references and correction history.
2. A typed description of the tension (empirical, normative, resource, temporal, structural or interpretive).
3. Explicit dependencies between poles: what each needs from the other and what fails if either is ignored.
4. An uncertainty record: known evidence, missing evidence, assumptions, confidence intervals where meaningful, and what would change each position.
5. A power map: who controls agenda, resources, time, data, expertise, publicity and the ability to exit.
6. A process-safeguard record: representation fidelity, accessible voice, dissent preservation, evidence review, appeal, conflict-of-interest disclosure.
7. A hazard record with states such as reported, corroborated, substantiated, refuted and unresolved.
8. A decision disposition, if any, with reasons, review date, reversal or stop conditions, and the preserved positions.
9. A durable dissent record: minority arguments are not silently deleted when a decision is made.
10. An append-only process event stream: actor, time, event type, prior receipt, current receipt and evidence references for each safeguard, hazard and disposition transition. The genesis position set must also be anchored by a host-verified receipt.

The system may produce a provisional decision without claiming the underlying paradox is resolved. An unresolved tension can be a valid and deliberate state, provided the decision record specifies review and consequences.

## 4. A disposition portfolio, not a one-size-fits-all synthesis

| Disposition | Use when | Required safeguards |
|---|---|---|
| Hold open | Evidence or legitimacy is insufficient, or no safe decision is possible | Review date, continued monitoring, no manufactured consensus |
| Evidence resolution | A factual dispute has been tested with adequate evidence | Pre-registered test, evidence bundle, uncertainty and residual limits |
| Integrate | A design can preserve multiple poles | Each affected pole represented; outcome tests for both |
| Prioritize provisionally | Urgency or binding constraints require one pole to lead temporarily | Explicit reason, retained alternatives, review date, reversible where feasible |
| Sequence | Both demands matter but cannot be satisfied at once | Named phases, dependency logic, stop/rollback trigger |
| Experiment | Competing designs can be tested safely | Pre-registration, bounded exposure, stop condition, independent measurement |
| Contextualize | Different contexts legitimately require different rules | Public boundary conditions, equal rights floor, review for boundary gaming |

This extends both/and thinking with an explicit **either/or under constraint** option. Both/and is valuable when poles remain interdependent and can be held in play. Either/or is sometimes necessary when resources, safety or legal duties force a choice. Neither should be the default ideology.

Do not assume that every paradox should be balanced 50/50. "Dynamic equilibrium" is not a numerical equality requirement.

## 5. Red-line process invariants

Consequential joint action must be blocked or independently reviewed when the following are active or unresolved:

- retaliation for dissent, refusal, complaint or exit;
- coercion or threats connected to dependency;
- essential service access conditioned on agreement or conformity;
- suppression or selective destruction of evidence;
- persistent material misrepresentation after correction;
- intimidation, harassment or status-based silencing;
- disproportionate surveillance of participants or dissenters.

A report is not proof of wrongdoing. It must nevertheless be preserved and routed for independent review. Do not automatically punish an accused person, erase their position, or silence the reporter. Safety measures should be proportionate, non-retaliatory and reviewable.

The process must also protect against the opposite failure: using "corrosive" as a label to delegitimize anger, protest, minority speech or necessary confrontation. Strong emotion is not itself a hazard. The relevant questions are coercion, rights, evidence, power, access and consequences.

## 6. The benchmark protocol

Run all policy arms on matched populations, capacities, budgets, shocks, seeds and time horizons wherever possible. Include conventional consensus facilitation, majority vote, expert-led decision, adversarial debate, structured deliberation, and the proposed plurality-preserving protocol.

### Scenario suite

| ID | Scenario | Required result |
|---|---|---|
| DD-01 | Factual disagreement with a pre-registered decisive test | Evidence updates the positions; no false equivalence |
| DD-02 | One factual claim is demonstrably false | Correct it within the tested scope, preserving a fair audit history |
| DD-03 | Normative conflict remains after evidence review | Decision is justified as a value choice, not a discovered empirical truth |
| DD-04 | Binding scarcity makes all needs impossible to satisfy | Physical limit disclosed; rationing and burden distribution reviewable |
| DD-05 | Autonomy versus collective resilience | Both poles and mutual dependencies remain explicit |
| DD-06 | Innovation versus reliability | Time-bounded sequencing or experiment with stop conditions |
| DD-07 | Task disagreement escalates into status/relationship conflict | Trajectory is detected; voice and task inquiry are protected |
| DD-08 | Dominant actor controls agenda, evidence or speaking time | Power asymmetry is disclosed and countermeasures activated |
| DD-09 | Minority view is fairly represented but loses the vote | Position remains recorded, appealable and reviewable |
| DD-10 | High-cohesion group manufactures unanimity | Private dissent channel detects suppressed disagreement |
| DD-11 | Evidence is selectively withheld or reinterpreted after results | Provenance and pre-registration expose post hoc changes |
| DD-12 | Crisis requires immediate action | Provisional scope, authority, expiry, review and retrospective audit |
| DD-13 | Facilitation and emotional labour burden concentrates on a few people | Workload distribution and compensation are measured |
| DD-14 | No convergence is possible | Hold-open is an accepted outcome, not reported as success or failure |
| DD-15 | "Both/and" is used to avoid a necessary hard choice | Binding constraint or rights conflict is named and adjudicated |
| DD-16 | "Either/or" is used to erase a genuine structural dependency | Missing pole and long-term consequences are exposed |
| DD-17 | An allegation of corrosive conduct is made but not substantiated | Preserve allegation and due process; do not punish by automated label |
| DD-18 | A hazard is refuted after review | Retain audit trail without continuing automatic process blockage |
| DD-19 | Emergency decision becomes permanent through inertia | Expiry and independent renewal required |
| DD-20 | Provisional policy causes unexpected subgroup harm | Stop/review trigger fires; affected parties and remedy recorded |
| DD-21 | Task disagreement escalates across short, medium and long horizons | Preserve the event trajectory; distinguish immediate process harm from long-run structural effects |
| DD-22 | Leadership turnover attempts to remove a minority position | Replay preserves the original record, source attribution and correction lineage |
| DD-23 | A hazard is refuted, then new evidence reopens the concern | Refutation and reopening are separate attributed events; no historical laundering |
| DD-24 | A previously available safeguard becomes inaccessible | Record the regression; block new consequential action until the safeguard is independently restored |
| DD-25 | A provisional decision creates delayed subgroup harm | Trigger review/rollback and record the affected group, remedy and residual uncertainty |
| DD-26 | A resolved tension reactivates after an external shock | Reopen inquiry without deleting the prior disposition or claiming permanent resolution |
| DD-27 | A short-term benefit transfers unpaid care or environmental cost to the future | Expose cross-horizon and subgroup incidence rather than optimizing only near-term output |
| DD-28 | Repeatedly deferred disagreement accumulates material burden | Report the unresolved issue, age, owner, review date and burden by affected group; do not collapse into one score |

### Required metrics

Do not collapse these into one "dissonance score".

**Representation**
- counter-position fidelity, confirmed by the source of each position;
- number and diversity of materially distinct positions preserved;
- position correction and misrepresentation rates;
- minority-position retention after decisions and after leadership turnover.

**Epistemic performance**
- calibration of empirical claims where probability is meaningful;
- accuracy and scope of evidence-based resolutions;
- update latency after decisive evidence;
- rate of false equivalence and premature closure;
- separation of empirical uncertainty from normative disagreement.

**Paradox handling**
- proportion of identified interdependent poles preserved in decision records;
- quality of dependency mapping judged by affected parties;
- ability to choose integration, sequencing, provisional priority, contextualization or hold-open for the right reasons;
- whether a selected disposition remains responsive to both poles over time.

**Safety and power**
- retaliation, coercion and essential-access conditioning incidents;
- agenda, speaking-time, resource and information concentration;
- dissent accessibility by subgroup and connectivity;
- appeal availability, uptake, resolution time, remedies and reversal rates;
- facilitation/emotional labour hours, compensation and burden concentration.

**Decision quality**
- service/outcome quality against matched baselines;
- time to a justified decision (not time to consensus);
- reversibility, stop-condition response and recovery time;
- regret or harm under later evidence and changed conditions.

## 7. Benchmark acceptance targets

Targets are divided into exact software properties and contextual empirical measures.

### Append-only event and replay requirements

- Every safeguard snapshot, hazard report, hazard-finding change and disposition change appends exactly one process event.
- Every event has a strictly increasing sequence number, a non-decreasing host timestamp, an actor identifier, a predecessor receipt reference and a current receipt reference.
- Every new hazard report starts in `Reported`. Every hazard-finding transition names the prior finding and a new evidence reference. Refutation does not delete the report; reopening requires a new event and evidence reference.
- Experiments account for every recorded position as tested or explicitly retained.
- Replay of a complete, valid event stream from the same genesis positions reproduces the same state and history.
- Experiments account for every recorded position as tested or explicitly retained.
- Hazard-finding transitions follow a declared state graph, require new evidence references and preserve prior findings; reopening a refuted hazard is a new event.
- Missing, duplicated, reordered, backdated or incorrectly linked events fail closed.
- The kernel does not authenticate actors or verify receipt signatures/digests. The host adapter must bind the genesis state and canonical event payload to signed/content-addressed receipts and enforce an independently authorized, append-only store.
- Current dispositions expose their mandatory review time; overdue review is a detectable state, not automatic resolution.

### Longitudinal evaluation

The benchmark must evaluate micro-, meso- and macro-temporal dynamics rather than a single meeting or vote. The current research review distinguishes short (0–4 years), medium (5–24 years) and long (25+ years) horizons, and sequential, cyclical and entangled process styles. A software benchmark can stress these through simulated event sequences and staged policy shocks, but real-world effects need longitudinal pilots.

Track at least:
- time to detect escalation from task disagreement to relationship/status conflict;
- hazard report-to-review and review-to-remedy latency;
- retention of minority positions across leadership changes;
- safeguard regression and restoration;
- whether review dates are missed, renewed or silently extended;
- short-term benefits versus delayed service, care, ecological and subgroup costs;
- the age and distribution of unresolved disagreement burdens ("disagreement debt"), reported as a vector rather than a composite score;
- how frequently a previously closed issue is legitimately reactivated after new evidence or changed context.

### Deterministic conformance targets

- 100% of recorded positions survive a disposition unless the source explicitly withdraws them; withdrawal does not erase history.
- 100% of provisional-priority decisions retain every non-selected position.
- 100% of empirical resolutions identify an evidence bundle and reviewed scope.
- 0 normative conflicts are marked as empirically resolved by the kernel.
- 0 consequential actions proceed while required safeguards are missing or an active/unresolved corrosive hazard is present.
- 100% of provisional/experimental decisions have a future review point; experiments and sequences have stop/rollback conditions.
- 100% of crisis exceptions have bounded scope, explicit expiry and retrospective review.

### Empirical targets

Do not set universal percentage targets before baseline data exist. Report distributions, uncertainty and subgroup outcomes for:
- time to justified decision;
- decision quality, implementation quality and longer-term regret;
- voice and representation;
- unresolved disagreement burden;
- escalation and retaliation;
- learning and updating after evidence;
- costs of facilitation and participation.

A system that takes longer to decide but prevents a high-impact unjust decision may be preferable. A system that never resolves anything is not necessarily wise. The benchmark must expose both failure modes.

## 8. Research comparison and contribution

This programme should integrate rather than claim to replace several established areas:

- **Team conflict research:** distinguishes task, relationship, process and status conflict; recent meta-analysis finds negative average performance relationships and meaningful heterogeneity.
- **Paradox theory:** examines persistent contradictory/interdependent demands; recent work argues for a portfolio of both/and and either/or approaches rather than doctrinaire balance.
- **Agonistic democracy:** warns that consensus processes can suppress conflict and power asymmetries; critiques also warn against romanticizing conflict.
- **Structured expert elicitation:** can quantify uncertainty and preserve disagreement without forcing consensus; scores must not imply precision that reviewers do not possess.
- **Adversarial collaboration:** competing theories jointly define fair tests, pre-register expectations and publish results whether either, both or neither is supported.
- **Bayesian adversarial collaboration:** treats scientific disagreement as accumulating evidence and changing model support, not only a binary winner/loser.

The proposed contribution is an executable institutional benchmark connecting these insights to real resource allocation, rights, consent, governance power, privacy, care work and reversible decisions.

## 9. Architecture mapping

- **Symtropy:** stores typed tension records, disposition transitions, scenario manifests, matched policy runs and outcomes.
- **Mycelix:** supplies authenticated provenance, scoped participation, consent, dissent records, appeal and portable evidence receipts; it does not decide whose view is morally superior.
- **Symthaea:** may generate candidate hypotheses and alternative framings, but must expose assumptions and counterexamples; it cannot infer that a view is corrosive from text alone.
- **Sol-Atlas:** grounds resource, location, infrastructure and time constraints.
- **Praxis:** teaches disagreement, evidence, power, paradox, conflict de-escalation, rights and repair.

## 10. Explicit non-claims

This benchmark does not measure a person's moral worth, consciousness, love, maturity or psychological fitness. It does not assume that conflict improves performance, that consensus is always bad, that both sides are equally valid, or that paradox must remain unresolved. It tests whether a process can distinguish evidence from values, make defensible choices, preserve dissent, protect rights and learn over time.

## 11. Research references

- Yuan, Yin & Sun (2026), [The paradox of team conflict revisited: An updated meta-analysis](https://doi.org/10.1037/apl0001315), *Journal of Applied Psychology*, 111(2), 195–224.
- Johansen & Nielsen (2026), [A process-temporal perspective on organizational paradoxes](https://doi.org/10.1111/ijmr.70032), *International Journal of Management Reviews*, review of 81 empirical studies.
- [Whether and when team task conflict and relationship conflict transform over time](https://doi.org/10.1108/IJCMA-09-2024-0238), *International Journal of Conflict Management* (2025); meta-analysis of 32 longitudinal studies, N=2,361.
- Krautzberger & Tuckermann (2024), [Navigating Both/And and Either/Or Approaches in Response to Paradoxical Demands](https://doi.org/10.1177/26317877241290249).
- Asenbaum (2026), [A multiperspectival approach to democratic theory](https://doi.org/10.1177/02633957251320041).
- August (2022), [Understanding democratic conflicts: The failures of agonistic theory](https://doi.org/10.1177/14748851221120120).
- [Reimagining peer review as an expert elicitation process](https://pmc.ncbi.nlm.nih.gov/articles/PMC8981826/) (2022), describing structured elicitation without consensus pressure and cautioning against false numerical precision.
- [Bayesian adversarial collaboration](https://doi.org/10.1016/j.neuron.2023.08.027), *Neuron* (2023).
- [Adversarial Collaboration Project](https://web.sas.upenn.edu/adcollabproject/), University of Pennsylvania; fair joint tests of competing perspectives.
- [Make science more collegial: why the time for adversarial collaboration has come](https://www.nature.com/articles/d41586-025-01379-3), *Nature* (6 May 2025).

These references motivate test design; they do not validate the LBE implementation or prove that a proposed process improves outcomes.
