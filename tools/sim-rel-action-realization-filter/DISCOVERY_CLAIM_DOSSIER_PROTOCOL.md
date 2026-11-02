# SIM-REL-047 — bounded scientific discovery claims: novelty, truth and replication

Status: first typed claim-dossier verifier on a research branch. It is a structural evidence gate; it does not authenticate reviewers, labs, apparatus, corpus completeness or scientific truth.

## 1. Why one "discovery score" is a category error

A scientific claim has at least three orthogonal properties:

1. **Empirical support:** does the preregistered experiment support the stated proposition?
2. **Prior-art status:** is a relevant prior result found in a dated and declared corpus using a documented search protocol?
3. **Reproducibility:** does the result survive replay or an independently conducted reproduction/replication?

A claim may be true but already known, novel-sounding but contradicted, computationally reproducible but not independently reproduced, or successfully reproduced in one context but not externally replicated. These are different outcomes and must not collapse into one leaderboard number.

This design follows current benchmark lessons:
- FIRE-Bench evaluates rediscovery of known findings with executable research tasks and claim-level P/R/F1; it does not claim absolute novelty.
- AgentIdeaBench shows that active literature search can improve feasibility, clarity and specificity without increasing measured originality.
- EmbodiedLab grounds scientific answers in actual readings from the episode and the physical state left behind.
- Autonomous Science Bench separately tests discovery and learning on designs that were not measured.

Primary references:
- FIRE-Bench (ICML 2026): https://proceedings.mlr.press/v306/wang26d.html
- AgentIdeaBench (September 2026): https://arxiv.org/abs/2609.07611
- EmbodiedLab (26 October 2026): https://arxiv.org/abs/2610.35385
- Autonomous Science Bench 0.1 (1 October 2026): https://www.autonomous-science-bench.ai/

## 2. Atomic claim dossier

Each dossier binds:
- stable claim ID, claim type and canonical atomic proposition digest;
- source campaign/run/operator/team/laboratory/apparatus identity;
- source data, raw-observation, task-manifest, code, harness and calibration digests;
- source preregistration and SealedHoldout result/evidence-verification receipts;
- frozen prior-art corpus version, snapshot digest, as-of timestamp and declared domain/source coverage;
- prior-art query bundle and full search transcript digest;
- at least two distinct adjudicators who reviewed the same frozen transcript;
- all same-artifact replay, independent reproduction and external replication receipts.

A paper title or free-form summary is not an atomic claim. Decompose claims into propositions whose truth conditions and evidence can be evaluated separately.

## 3. Prior-art scope semantics

The verifier returns one of three bounded statuses:

- `KnownPriorArtFound`: independent adjudicators agree that a relevant prior work exists in the searched corpus.
- `NoPriorArtFoundWithinDeclaredCorpus`: the search was complete under the declared version, date, sources and domains, and at least two distinct reviewers agree no relevant match was found.
- `PriorArtSearchInconclusive`: search incomplete, coverage/date invalid, too few reviewers, or adjudicators disagree.

"No prior art found" is strictly bounded by the declared corpus and protocol. It is never promoted to "absolutely novel". Corpus incompleteness and retrieval/index blind spots remain limitations.

The corpus snapshot must predate the claim issue time. Search start/end times and query budget are recorded. Reviewers must adjudicate the exact frozen search transcript, not a later mutable result set.

## 4. Replication levels

### SameArtifactReplay

Replays the source's exact raw observation/data, task manifest, source code and harness. It tests deterministic replay and computational reproducibility. It does **not** establish independent reproduction.

### IndependentReproduction

Requires:
- a distinct replication campaign and run;
- a different operator and team identity;
- a new dataset/raw-observation artifact;
- a distinct seed namespace;
- a distinct preregistration receipt;
- explicit criterion, analysis and evidence-verification receipts.

The same lab/apparatus may be used, provided those independence properties are met.

### ExternalReplication

Requires all IndependentReproduction conditions plus:
- a distinct laboratory;
- a distinct apparatus;
- a distinct calibration-chain digest.

The verifier records every replication result, including failed and inconclusive attempts. A successful replay cannot erase a failed independent replication.

The current implementation verifies identity inequality over supplied values. It does not authenticate people, laboratories, instruments or calibration records; production assurance requires trusted identity and provenance providers.

## 5. Bounded claim dispositions

The verifier combines the source preregistered outcome, prior-art scope and strongest successful replication level:

- `ReproducedKnownFinding`: source experiment supports the claim, relevant prior art is found, and an independent or external replication succeeds.
- `KnownFindingWithoutIndependentReproduction`: source supports a known finding, but the only success is same-artifact replay or no replication is available.
- `ProvisionalNovelCandidate`: source experiment supports the claim, no prior art was found within the declared corpus, and an independent or external replication succeeds. This remains provisional; it is not a universal novelty certificate.
- `NoPriorArtCandidateNotYetIndependentlyReproduced`: no prior art was found within the declared corpus, but the only success is replay or independent replication has not succeeded.
- `Contradicted`: the preregistered source campaign itself records a contradicted outcome.
- `Inconclusive`: source evidence or prior-art assessment does not support a bounded disposition.

A successful external replication increases the reported strongest successful level, but failed external attempts remain explicitly present. This permits a claim with an earlier independent success and a later external failure to be reported as mixed evidence rather than silently promoted.

## 6. Independence and statistical boundaries

The dossier verifier does not implement the statistical test for a replication effect. It accepts a preregistered outcome and evidence receipt digest whose semantics belong to the relevant physics/statistics verifier.

SIM-REL-042 remains the fixed-sample inferential route; SIM-REL-045 remains the anytime-valid sign-process route; SIM-REL-046 reports task-clustered repeated-run reliability. Replication attempts on one claim are not independent scientific task clusters. Do not feed them into the task-cluster denominator or count same-task repetitions as new generalization units.

A claim supported by an external replication still has a domain boundary: one external lab is not universal generalization.

## 7. Qualification

Focused tests cover:
- prior art found plus external replication remains a reproduced known finding;
- no prior art within a frozen corpus plus independent reproduction is only a provisional candidate;
- incomplete search cannot support novelty;
- same-artifact replay does not upgrade to independent reproduction;
- repeated-run/operator/team/data/seed/pre-registration identity substitutions fail closed;
- external lab/apparatus/calibration must differ;
- failed and inconclusive replication outcomes remain in the receipt;
- sealed-only source evidence;
- distinct reviewers and consistent adjudication;
- contradictory source outcomes cannot be overwritten by a successful replay.

Qualification requires pinned Rust 1.96 formatting, focused tests, full locked crate tests, strict Clippy, exact formatted-head and clean-tree checks.

## 8. Limits and future work

This increment is a typed consistency verifier. Its digests are shape-checked, not cryptographically recomputed here. Reviewer/operator/lab IDs are declared strings, not independently authenticated identity credentials. It does not determine whether a corpus includes all relevant literature, whether an independent lab is truly independent, whether an instrument is calibrated, or whether a mechanism is physically correct.

Next required increments:
- bind dossier evidence digests to a cryptographically verified SIM-REL-043 evidence package;
- add trusted issuer and reviewer identity / conflict-of-interest attestations;
- add corpus snapshot manifests and a reproducible search adapter;
- add domain-specific statistical and physical replication evaluators;
- add blinded expert adjudication and audit trails;
- test claim decomposition precision/recall and overclaim rates on held-out scientific dossiers.
