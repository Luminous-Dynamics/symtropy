# AERO Independent Assurance Benchmark Protocol v0.3

**Status:** preregistration proposal; no benchmark executions or performance claims  
**Scope:** structural-material / component-evidence claim family, initially synthetic and non-operational  
**Authority ceiling:** research benchmark only; no physical qualification, airworthiness or certification

## 1. Research question

Can an assurance system reconstruct a bounded engineering claim disposition from independently verifiable owner evidence, preserve historical knowledge and trust semantics, detect adversarial substitutions, and identify the minimal safe requalification frontier more accurately and efficiently than current review workflows?

The benchmark evaluates the complete decision process, not merely schema validation, retrieval quality, model accuracy or report generation.

## 2. What is novel and what is not

Assurance cases, model-based traceability, formal argument analysis and change-impact automation are established research areas. ACCESS demonstrates model-based traceability, formal-method evidence invocation, machine checking and automated impact analysis. NASA-STD-7009B establishes lifecycle expectations for model/simulation credibility and uncertainty. NASA's 2025 study of LLM-generated/assessed assurance arguments concludes that fitness for critical engineering use remains to be demonstrated.

AERO's research hypothesis is the integration of:
- claim identity and operating-context binding;
- owner-native provenance and authority receipts;
- receiver knowledge time and historical trust-policy replay;
- non-compensatory evidence/model obligations;
- typed assurance-case argument structure and defeaters;
- observer-coverage-qualified change-impact analysis;
- portable replay and independent oracle reconstruction;
- evidence-acquisition recommendations that never self-authorize acceptance.

This is a hypothesis to test, not a claim of superiority.

## 3. Frozen benchmark unit and independence

The primary unit is an **engineering claim episode**, not an individual JSON mutation.

An episode includes:
1. claim specification and physical/configuration context;
2. evidence manifest with owner, producer, chronology, applicability and receipt data;
3. dependency graph plus observer coverage declaration;
4. trust-policy snapshot(s);
5. model credibility and validation-stratum records where relevant;
6. a known causal fault family or positive-control status;
7. independently authored expected disposition, reason codes and affected-claim set;
8. the exact as-known cutoff and replay policy.

All derived mutations from one causal scenario remain in one partition. Train/dev/visible challenge/blind test partitions are separated by root-cause family, not random row split. Shared evidence templates, producer keys, graph motifs and near-duplicate episodes must not cross partitions unless explicitly part of a transfer test.

The independent statistical unit is the root-cause family or independently sourced engineering episode. Ten thousand variants of one bug are not ten thousand independent trials.

## 4. Independent oracle protocol

The expected result is authored before running the system under test by at least two reviewers:
- one assurance/verification reviewer;
- one relevant engineering-domain reviewer.

They independently label disposition, reason codes, evidence applicability, historical knowledge, dependency impact and permitted next action. Disagreements are adjudicated by a third reviewer and retained with rationale. The validator implementation must not generate its own oracle.

For synthetic cases, the scenario generator and oracle are separate implementations with distinct data structures and no shared decision helper. For historical cases, source records and rights/authorization must be documented.

Blinded test cases are held by a custodian. The system team receives only a frozen input digest before evaluation. Test-set changes require a new version and a documented reason.

## 5. Mandatory benchmark families

### A. Claim and context correctness
Test wrong article, part, batch, configuration, operating envelope, intended use, requirement revision, unit, load case and test-article substitution. Include same-part-number/different-serial and same-model/different-configuration cases.

Primary metrics:
- hard-gate false-review rate;
- cross-context substitution acceptance;
- mandatory-obligation omission rate;
- reason-code exact agreement.

### B. Evidence provenance and authority
Test byte tampering, owner-record substitution, producer authorization, revoked/expired authority, receipt replay, signature/key rotation, digest-only evidence and selective disclosure. Keep cryptographic verification separate from semantic correctness.

Primary metrics:
- unauthorized-producer acceptance;
- payload/record binding error rate;
- receipt replay acceptance;
- provenance completeness and verifier coverage.

### C. Temporal and policy replay
Preserve acquisition/physical time, issuer-asserted event time, receiver-first-seen time, and trust-policy effective/observation time as distinct values. Test backdating, delayed receipt, retroactive correction, policy rotation, clock uncertainty and contradictory chronology.

Required replay modes:
- historical evidence + historical trust policy;
- historical evidence + current trust policy;
- current evidence + current trust policy.

Primary metrics:
- disposition and reason-code agreement with independent temporal oracle;
- false historical-knowledge rate;
- correction-chain preservation.

### D. Model credibility and domain applicability
Test model/solver identity, source and build provenance, validation split leakage, missing strata, unsupported operating domains, uncertainty budget, sensitivity, numerical convergence, benchmark representativeness and model discrepancy. A model report digest alone is not a credibility result.

Primary metrics:
- unsupported-domain false review;
- missing-stratum acceptance;
- leakage detection;
- applicability boundary agreement;
- uncertainty obligation omission.

### E. Change impact and observer completeness
Mutate material batch, design configuration, process parameter, calibration, inspection status, model version, trust policy, supplier record and observer coverage. Include events the observer cannot see.

A claim may be preserved only when:
1. its dependency frontier is complete for the relevant change class;
2. the observer coverage is qualified for the relevant source and interval;
3. no affected dependency exists;
4. the preserved claim retains its original evidence and time snapshot.

Unknown coverage must produce UNKNOWN/INCOMPLETE impact, never an empty affected set.

Primary metrics:
- affected-claim recall (safety critical);
- false preservation rate;
- unnecessary requalification rate;
- impact-set precision;
- observer coverage detection and unknown propagation.

### F. Reproducibility and decision portability
Move a capsule to a clean independent environment. Remove network access, reorder manifests, vary filesystem paths, rotate trusted certificates under a declared historical policy and attempt reconstruction with partial evidence.

Primary metrics:
- exact independent disposition/reason-code agreement;
- reconstruction success by disclosure class;
- undeclared dependency rate;
- reproducibility classification correctness;
- time and compute cost to replay.

### G. Evidence acquisition and value of information
For unresolved claims, provide candidate actions with expected information gain, cost, safety constraints, authority prerequisites and predicted resolution conditions. Evaluate ranking only against a preregistered utility model and expert-adjudicated action set.

The system is not rewarded for recommending risky or unauthorized physical work. It must distinguish:
- information that could resolve a proposition;
- permission to collect it;
- completion of collection;
- verification of the resulting artifact;
- acceptance of the engineering obligation.

Primary metrics:
- expert top-k action agreement;
- cost-adjusted information gain;
- unsafe/unauthorized action recommendation rate;
- evidence-resolution prediction calibration.

## 6. Baselines and fair comparison

Run all systems against the same frozen claim episodes, source evidence, decision cutoffs, permitted tools and reviewer instructions.

Baselines:
1. independent expert review using the source records;
2. conventional checklist + document/PLM/QMS workflow;
3. graph-only dependency/change-impact analysis;
4. rules-only assurance validator;
5. single LLM-assisted reviewer with retrieval;
6. multi-agent LLM-assisted reviewer;
7. AERO without temporal replay;
8. AERO without observer-coverage gating;
9. full AERO.

Ablations are essential: they show whether each AERO subsystem contributes measurable value. All systems must receive equal source access; tool use and human intervention are logged. No baseline is intentionally handicapped.

## 7. Primary metrics and thresholds

Thresholds below are **prospective qualification targets**, not achieved results.

| Metric | Target |
|---|---:|
| Hard-gate false-review count | 0 observed in frozen qualification set |
| Cross-context substitution accepted | 0 |
| Unauthorized producer accepted | 0 |
| Mandatory obligation bypass | 0 |
| Unsupported model domain promoted | 0 |
| Late evidence treated as historically known | 0 |
| Critical invalidation missed | 0 |
| Exact independent oracle agreement | >= 99.5% overall, 100% on critical disposition gates |
| Reason-code agreement | >= 99% |
| Affected-claim recall | 100% on critical dependency families |
| False preservation under unknown coverage | 0 |
| Unnecessary requalification | <= 5%, only after zero missed invalidations |
| Independent replay success | >= 99% for fully disclosed capsules |
| Matched cost per reviewed decision | <= 80% of expert/checklist baseline, with non-inferior safety |

No aggregate score can compensate for a failed critical gate. Report the complete confusion matrix and all per-stratum outcomes.

### 7.1 Metric definitions, denominators and gate semantics

The machine-readable registry's `metric_definitions` array is normative. It currently defines 30 metrics across the seven benchmark families. Each definition fixes the numerator, denominator, unit, optimization direction, population, independence unit, gating class and interpretation notes; the 19 threshold records refer to these stable metric IDs.

Scoring rules:

- Publish numerator and denominator as integer counts alongside every proportion. A missing denominator is **NOT_REPORTED**, never zero.
- Report macro-averages by independent root-cause family and the worst critical stratum; do not let a large easy family hide a small dangerous one.
- Use the exact same frozen episode set, time cutoff, source evidence and completion criteria for each comparison arm.
- Economic targets are conditional: unnecessary requalification and cost improvements are considered only after the critical invalidation-miss and unknown-observer gates pass.
- Positive-control retention and false-rejection rates prevent an always-reject evaluator from passing.
- Unauthorized evidence-acquisition recommendations are hard-gated even if their predicted information gain is high.
- Report cost with reviewer labor, escalations, abstentions, corrections, compute and tool costs included.

The registry distinguishes **hard gates**, **qualification metrics**, **diagnostic metrics** and **economic metrics**. A diagnostic or economic improvement cannot compensate for a hard-gate failure. Metric thresholds are prospective targets and not measurements.

### 7.2 Challenge-seed interpretation corrections

The 32-case challenge seed remains unadjudicated. Three semantic distinctions are made explicit in the current fixture:

- The unrelated-document change in IAB27 leaves the unaffected claim reviewable; unnecessary requalification is scored by the separate impact outcome, not by marking the claim incomplete.
- Missing authority to perform a proposed test (IAB29), or declaring a claim resolved before the resulting evidence is received and verified (IAB30), leaves the claim incomplete. The evaluator must not interpret an unauthorized action recommendation as a valid resolution.
- IAB31 explicitly uses a current-policy counterfactual replay. Historical-as-known replay remains a separate mode and must retain the trust policy actually in force at the cutoff.

These corrections improve the semantics of the proposed oracle; they do not constitute independent adjudication.

### 7.3 Exact statistical confidence and minimum independent sample sizes

Critical failure-rate claims are assessed with one-sided exact Clopper-Pearson confidence bounds, not only point estimates. For zero observed adverse outcomes in \(n\) independent Bernoulli units, the exact one-sided upper confidence bound at confidence \(1-\alpha\) is:

\[
U = 1 - \alpha^{1/n}
\]

The minimum number of independent units required to place the upper bound at or below target adverse-event probability \(p\), when zero adverse outcomes are observed, is:

\[
n = \left\lceil \frac{\ln(\alpha)}{\ln(1-p)} \right\rceil
\]

At 95% one-sided confidence (\(\alpha=0.05\)):

| Maximum adverse-event probability | Minimum independent zero-event units |
|---:|---:|
| 1% | 299 |
| 0.1% | 2,995 |
| 0.01% | 29,956 |

These are preregistered sample-size requirements, not achieved sample sizes and not evidence that the corresponding risk has been attained. The familiar \(3/n\) rule is an approximation only; the exact formula and values in the machine-readable registry are normative.

**Independence discipline:** one independently sourced engineering episode/root-cause family is the statistical unit. Ten thousand generated variants from one fault seed are one correlated cluster, not ten thousand independent trials. Keep all siblings in one partition and report independent episodes, unique root-cause families, and generated variants separately.

**Stratification discipline:** report every critical metric by preregistered fault family and critical stratum, and publish the worst critical stratum. Do not allow a large nominal population to dilute an identity, authority, temporal, model-domain or observer-coverage failure. For simultaneous claims over multiple critical metrics, publish per-metric 95% bounds and Bonferroni-adjusted family-wise bounds using \(\alpha/M\), where \(M\) is the preregistered number of critical hypotheses.

A denominator of zero is \`NOT_ESTIMABLE\`, not zero and not a pass. When adverse outcomes are observed, publish the exact one-sided Clopper-Pearson upper bound using the frozen confidence level and include numerator and denominator. Success metrics publish the complementary exact lower bound. Do not use Wald/normal approximations for rare critical events.

The registry now gives every hard-gate metric a critical threshold, including receipt replay and missing required model strata. CI recomputes the zero-event sample sizes and rejects a registry that silently drops a hard-gate threshold.

## 8. Statistical qualification

For zero observed failures in n independent Bernoulli trials, the approximate one-sided 95% upper bound is 3/n (rule of three). Thus:
- n=300 gives approximately 1%;
- n=2,995 gives approximately 0.1%;
- n=29,956 gives approximately 0.01%.

These bounds are meaningful only when the independence assumptions are defensible. Clustered variants must be analyzed by root-cause family, with cluster-aware confidence intervals or conservative effective sample size. Report the number of independent source episodes, root-cause families and generated mutations separately.

Zero observed failures is not proof of zero risk.

## 9. Anti-gaming and validity controls

- Freeze benchmark version, source inputs, system commit and configuration before evaluation.
- Keep blind cases and expected outputs inaccessible to implementers.
- Hash the full input, oracle, run configuration and result bundle.
- Include positive, negative, coupled-fault and metamorphic cases.
- Require critical mutation tests to be killed; report surviving mutations.
- Detect duplicate/near-duplicate scenarios and partition leakage.
- Include abstention and coverage metrics so an always-reject system cannot win.
- Include false rejection, reviewer effort and latency so a uselessly conservative system cannot win.
- Report failures and excluded cases, with exclusions adjudicated before unblinding.
- Repeat with an independent implementation and independent verifier.
- Publish raw anonymized results and analysis scripts when rights permit.

## 10. Release gates

**R0 — structural:** schemas, identifiers, graph invariants, deterministic canonicalization.

**R1 — semantic:** independently authored oracles, reason-code agreement, non-compensatory decisions, temporal and applicability tests.

**R2 — adversarial:** blinded root-cause families, mutation campaigns, coupled faults, observer-coverage failures.

**R3 — independent replay:** clean-room package reconstruction, independent verifier, cryptographic and provenance boundary tests.

**R4 — matched workflow:** expert and conventional workflow comparison with effort, cost and coverage.

**R5 — authorized pilot:** only synthetic or rights-cleared historical records first; physical tests require competent engineers, approved procedures, owner permission and applicable authority.

A pass at one release gate does not imply a pass at the next.

## 11. Research hypotheses that could exceed current practice

1. **Assurance build system:** update only claims whose proof obligations are causally affected, but require complete observer coverage before preserving any claim.
2. **Defeater-first review:** search for counterexamples and competing explanations before assembling positive support.
3. **Proof-carrying evidence acquisition:** every recommended action states the exact unresolved predicate it could address, its authorization requirements and the artifact contract expected on completion.
4. **Bitemporal counterfactual replay:** reconstruct what was known then, what the same record means under today's trust policy, and what today's evidence supports.
5. **Portable independent execution:** make a decision reconstructable outside the originating platform without granting the verifier authority over the physical claim.
6. **Evidence economics:** optimize engineering effort only after critical safety recall has been demonstrated.

## 12. Non-claims

This protocol, its synthetic cases and any passing software tests do not establish physical truth, model credibility, airworthiness, regulatory compliance, certification, or safety of an aircraft or component. Benchmark superiority requires independent execution, domain-relevant evidence, matched baselines and peer scrutiny.
