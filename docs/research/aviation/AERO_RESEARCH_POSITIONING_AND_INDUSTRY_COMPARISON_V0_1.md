# AERO Research Positioning and Industry Comparison v0.1

**Status:** literature-informed research positioning; not a market survey or certification assessment  
**Date:** 2026-11-27  
**Purpose:** identify what is established, what AERO must not claim as novel, and where a defensible research contribution may exist.

## 1. Executive conclusion

AERO should not claim to invent machine-readable assurance cases, assurance-argument formalization, model-based traceability, automated change-impact analysis, model credibility assessment, or lifecycle digital threads. These are established research and industry directions.

The strongest research opportunity is a rigorously evaluated **cross-owner assurance replay and requalification protocol** that joins:
1. typed claim/context identity;
2. independently verifiable owner evidence and authority;
3. separate physical/issuer/receiver/trust time semantics;
4. non-compensatory model applicability and validation obligations;
5. observer-coverage-qualified change impact;
6. portable independent reconstruction;
7. defeater-first and bounded evidence-acquisition workflows.

The differentiator is the *end-to-end contract and empirical qualification*, not any single graph, digest, LLM, or assurance-case notation.

## 2. Relevant prior art and standards

### Assurance-case-centric engineering (ACCESS)

The ACCESS work describes assurance-case-centric development, model-based traceability to engineering artefacts, invoking formal-method evidence (including Isabelle/HOL), machine-checking argument formalizations, and automated change-impact analysis. It also discusses evolving development-time assurance cases into runtime assurance cases.

Implication: AERO's argument graph, formal evidence invocation and change-impact concepts overlap established prior art. The research contribution must be more specific: cross-owner identity/provenance, temporal replay, observer completeness, independent capsule reconstruction, and an adjudicated benchmark.

Reference: [ACCESS: Assurance Case Centric Engineering of Safety-critical Systems](https://doi.org/10.1016/j.jss.2024.112034).

### Assurance-case semantic analysis

Recent work on goal-directed Answer Set Programming (ASP) formalizes semantic analysis of assurance cases. This reinforces that graph well-formedness is not argument soundness. AERO should eventually expose a formal semantics or independently checked inference obligations, not merely validate node and edge structure.

Reference: [Automating Semantic Analysis of System Assurance Cases Using Goal-Directed ASP](https://www.cambridge.org/core/journals/theory-and-practice-of-logic-programming/article/automating-semantic-analysis-of-system-assurance-cases-using-goaldirected-asp/1515C06F4BC02013C302239B8C6C4562).

### NASA model and simulation credibility

NASA-STD-7009B establishes requirements/recommendations for models and simulations used in NASA activities, including credibility, uncertainty, transparency, repeatability and traceability across the M&S lifecycle. NASA's 2026 B-series handbook provides implementation guidance while explicitly leaving discipline-specific methods to responsible engineering disciplines.

Implication: AERO should map model owner reports to context-of-use, credibility, uncertainty and validation obligations; it should not replace the underlying discipline-specific VVUQ.

References:
- [NASA-STD-7009B](https://standards.nasa.gov/sites/default/files/standards/NASA/B/1/NASA-STD-7009B-Final-3-5-2024.pdf).
- [NASA-HDBK-7009B, dated 2026-02-03](https://standards.nasa.gov/node/198).

### Computational Materials for Qualification and Certification (CM4QC)

The March 2026 NASA/FAA-linked CM4QC strategy is a particularly relevant domain comparator. It describes a multi-year government, industry and academic effort to mature computational-materials methods for aviation-focused qualification and certification of metal additive manufacturing. It explicitly separates high-, mid- and low-technology-readiness work, and identifies the transition from research methods to trusted engineering capability as a central challenge. The report also states that computational-materials models are not yet generally mature enough to be trusted by regulatory authorities and major OEM product-assurance / chief-engineer functions for Q&C use.

The strategy emphasises transferability of validated model workflows, measurement and model uncertainty, sensitivity analysis, hierarchical benchmarks spanning feedstock through build/post-processing/mechanical behaviour, and FAIR data. It notes that high-quality controlled benchmark measurement data are expensive and scarce. Its AM Bench discussion describes challenge problems and coupled measurement datasets rather than a general-purpose assurance decision engine.

Implication: CM4QC is not a direct software competitor. It is a domain-level reference for the engineering evidence and validation content AERO must consume. AERO should integrate owner-native VVUQ, AM Bench / hierarchical benchmark results and model-use bounds; it must not claim to supply those measurements, model validations or transferability evidence itself. Its hypothesis is that the chain from those owner-controlled artefacts to a claim-level, time-correct and independently replayable disposition can be made more trustworthy and cheaper to review.

References:
- [NASA/TM-20260001729 / DOT-FAA/TC-26/10, CM4QC Strategy Document (March 2026)](https://ntrs.nasa.gov/api/citations/20260001729/downloads/NASA-TM-20260001729.pdf).
- [NIST Additive Manufacturing Benchmark Test Series (AM Bench)](https://www.nist.gov/ambench).

### NASA software assurance guidance for AI-enabled workflows

NASA Software Engineering Handbook Version D, Topic 8.25, extends software-assurance expectations to foundation models, compound AI systems and agentic systems across their lifecycle. It calls for first-class evaluation, traceability, human oversight, uncertainty communication, security, modularity/resilience and continuous change management. Its applicability text recommends non-safety-critical use until a documented AI safety case and risk controls are approved by the appropriate authority.

Implication: AERO can use AI to search evidence gaps, propose candidate defeaters and rank safe acquisition actions, but must retain human/authority control over review, acceptance and physical work. Those AI functions require their own frozen evaluation datasets, traceable versions, confidence/abstention policy and hazard-oriented negative tests.

Reference: [NASA SWEHB Version D, §8.25 – Super Intelligence and Software Assurance](https://swehb.nasa.gov/spaces/SWEHBVD/pages/184320002/8.25%2B-%2BSuper+Intelligence+And+Software+Assurance).

### LLMs and assurance arguments

NASA/TM-20250001849 (March 2025) surveys proposals for LLM generation and assessment of assurance arguments. Its abstract concludes that much remains to be demonstrated before LLMs can be considered fit for generating or assessing assurance arguments in engineering and certification practice.

Implication: Symthaea should be an investigator, defeater generator and evidence-action planner under explicit limits. It must not become the source of truth or an autonomous certification reviewer. Evaluate hallucinated evidence, unsupported inference, omission of defeaters, overconfident disposition and automation bias.

Reference: [NASA/TM-20250001849, Examining Proposed Uses of LLMs to Produce or Assess Assurance Arguments](https://ntrs.nasa.gov/api/citations/20250001849/downloads/NASA-TM-20250001849.pdf?attachment=true).

### MBSE and digital-thread workflows

Industrial MBSE and PLM workflows already provide requirements traceability, configuration management, verification management, product lifecycle data and digital-thread integration. Automated validation suites can query model completeness and maturity.

Implication: AERO should integrate with PLM/MBSE/QMS rather than compete as a replacement. Its target is portable cross-boundary claim admissibility and independent replay across owners and platforms.

References:
- [Siemens: Digital approach to airworthiness](https://static.sw.cdn.siemens.com/siemens-disw-assets/public/OuFPhhtGA5To47IB60A4m/en-US/VM---Technical-guide---Digital-approach-to-airworthiness_tcm27-106699.pdf).
- [IEEE Aerospace 2024: Automating Model Validation for Quantifying System Maturity & Quality Assurance](https://doi.org/10.1109/AERO58975.2024.10521227).

### Current eVTOL simulation assurance: EASA MOC-5 and EUROCAE ED-342

The fifth publication of EASA's Means of Compliance with the Special Condition VTOL, Issue 2 dated 5 November 2026, gives a current, concrete reference for simulation-supported compliance demonstration. It requires the representativeness and limitations of simulation benches and models to be established, including failure cases; where flight-test data are available they should support representativeness, and the guidance addresses validation of models for hazardous or catastrophic failure cases. It also requires relevant functional/operational impacts of model limitations to be documented and included in configuration management. The EASA text emphasises aircraft-level, multi-system representativeness rather than a narrow isolated component view.

EUROCAE ED-342, published September 2026, describes a structured approach to using simulation in VTOL handling-qualities certification and presents simulation as a means of compliance for cases where flight testing may be reduced or partially substituted by pilot-in-the-loop simulation. This increases—not decreases—the importance of documenting simulator configuration, flight domain, failure cases, model representativeness and limitations.

Implication: this is a strong transfer domain for AERO after the structural-material benchmark is established. A future eVTOL challenge should test whether the same claim capsule can bind: (a) aircraft and configuration identity; (b) simulator bench and model version; (c) failure-case and flight-envelope coverage; (d) flight-test / rig / validation artefacts; (e) owner-stated model representativeness and limits; and (f) the configuration-management state used for the compliance demonstration. AERO should report missing or mismatched evidence; the model owner and authorised applicant/authority retain the engineering and approval decisions.

References:
- [EASA MOC-5 SC-VTOL, Issue 2, 5 November 2026](https://www.easa.europa.eu/en/downloads/144423/en).
- [EUROCAE ED-342: Guidance on Using Simulation in Handling Qualities Certification of VTOL (September 2026)](https://www.eurocae.net/product/ed-342-guidance-on-using-simulation-in-handling-qualities-certification-of-vtol/).

### Reproducible builds and software supply-chain attestations

SLSA provenance and the in-toto attestation model are important adjacent precedents for the “claim as build artifact” direction. SLSA defines a verifiable provenance statement describing where, when and how an artifact was produced, including the builder and resolved dependencies; downstream consumers are expected to verify provenance rather than trust fields merely because they are present.

Implication: AERO should reuse these established supply-chain ideas for the *assurance computation itself*: identify the verifier source tree and build, toolchain/runtime, locked dependencies, invocation, inputs, output manifest, builder identity, network policy and execution receipt. The independent consumer must be able to reconstruct the disposition from the declared inputs and verify the verifier's provenance. A digest of a verifier binary is not evidence that its logic is correct, and software build provenance does not prove physical truth, model validity, engineering authority or certification credit.

The research opportunity is the composition across two different provenance layers:

1. **Engineering provenance:** exact claim specification, physical subject/configuration, owner-native evidence cuts, trust-policy snapshot, model-credibility cut, observer coverage and source chronology.
2. **Computation provenance:** exact verifier source/build, dependency graph, runtime, invocation, disclosed inputs and output manifest.

AERO's novelty hypothesis must remain at the boundary between these layers: whether a third party can independently replay the exact claim decision across owner boundaries and historical time cuts, while retaining source-owner and regulatory authority. Provenance attestations themselves are established technology, not an AERO novelty claim.

References:
- [SLSA Build Provenance specification](https://slsa.dev/spec/v1.1/provenance).
- [in-toto Attestation Framework](https://github.com/in-toto/attestation).

### EASA VIRTUA: lifecycle authenticity and revocation

EASA's VIRTUA project is a directly relevant adjacent workstream. Its stated objective is the use of virtualisation and decentralised-ledger concepts to manage approved aircraft parts and EASA Form 1 certificates across their lifecycle, from issuance by production organisations through design/maintenance changes. The motivating challenges include checking certificate authenticity and revoking certificates when defects or suspected unapproved parts are identified. The project is listed as closed.

Implication: provenance and revocation are not an AERO novelty claim. AERO should interoperate with owner-native certificate/part lifecycle services and compare their receipts, revocation events and historical states. Its proposed contribution is claim-level composition: determine which exact engineering claim, physical/configuration context, model-use envelope and downstream evidence depend on the affected certificate at a particular knowledge/policy snapshot. This must work regardless of whether the owner uses a distributed ledger, signed database events or another trustworthy mechanism.

Reference: [EASA VIRTUA project](https://www.easa.europa.eu/en/research-projects/virtua-digital-transformation-case-studies-aviation-safety-standards).

### Certification acceptance and existing assurance tooling

AERO must operate alongside established certification and assurance infrastructure, not imply that it replaces it.

The FAA describes DO-178C/ED-12C, DO-254/ED-80 and aspects of ARP4754 as established development-assurance references for airborne software, electronic hardware and aircraft/system development. Its joint FAA/EASA Abstraction Layer work explicitly recognises that emerging methods may be evaluated as alternatives only through defined assessment criteria; the Task Force published twenty criteria and guidance for considering alternate standards. AERO's evidence contracts and benchmark are therefore best positioned as a complementing integration/replay method until an applicable authority accepts a method for a defined use.

Reference: [FAA Abstraction Layer Information](https://www.faa.gov/aircraft/air_cert/design_approvals/air_software/abstraction_layer).

The tooling landscape is also non-empty. NIST's assurance-case tools page lists ASCE (Adelard) and its Claims-Arguments-Evidence / GSN support. Siemens markets integrated aircraft-certification and digital-thread workflows connecting certification plans, verification data and lifecycle information. These are important comparison arms and integration targets, not straw-man competitors.

References:
- [NIST Assurance Case Tools](https://www.nist.gov/itl/csd/secure-systems-and-applications/assurance-case-tools).
- [Siemens Aircraft Certification](https://www.siemens.com/en-us/digital-thread/integrated-lifecycle-management/aircraft-certification/).
- [Siemens traceable certification data chain](https://blogs.sw.siemens.com/aerospace-defense/2025/03/20/streamline-the-aircraft-certification-process-part-2-establishing-a-traceable-chain-of-data/).
- [SAE ARP4754B](https://saemobilus.sae.org/standards/arp4754b-guidelines-development-civil-aircraft-systems).

**Positioning consequence:** compare AERO against both assurance-case authoring/analysis tools and PLM/QMS/digital-thread workflows. The measurable gap is not "traceability exists" but whether a third party can independently reconstruct a claim-level decision across owner boundaries, temporal trust-policy cuts and qualified observer coverage, while preserving source-owner authority and a clear non-approval ceiling.

## 3. Competitive capability matrix

| Capability | Established industry/research position | AERO's testable opportunity |
|---|---|---|
| Requirements and configuration traceability | Mature MBSE/PLM capability | Preserve claim-specific physical/configuration identity across owner boundaries |
| Assurance-case graph and argument structure | Established standards and research | Typed executable contract with defeaters and explicit non-claims |
| Formal semantic checking | Active research, including ASP and theorem-prover workflows | Independent semantic oracle and explicit reason-code reconstruction |
| Model credibility / VVUQ | NASA-STD-7009B and discipline-specific practices | Bind owner-native credibility results to exact claim context and model cut |
| Evidence integrity / signatures | Established cryptographic and supply-chain mechanisms | Separate payload integrity, producer authority, semantic validity, applicability and receiver knowledge |
| Change impact | Existing traceability and assurance-case research | Permit selective requalification only when observer completeness is qualified |
| Historical replay | Bitemporal/event-sourcing concepts exist in adjacent fields | Reconstruct evidence and trust as known at the original decision time |
| Portable decision reconstruction | Reproducible build and archival methods exist | Portable assurance capsule with detached package integrity and explicit disclosure class |
| LLM support | Early research; critical-use efficacy remains unproven | Defeater-first investigation and bounded evidence acquisition with measured error/authority limits |

## 4. Defensible novelty claims

AERO may claim these as **research hypotheses**, pending independent evidence:

1. **Cross-owner claim replay:** a single portable claim episode can be reconstructed from multiple owner-native receipts without transferring owner authority to AERO.
2. **Temporal non-collapse:** evidence knowledge time and trust-policy time are preserved independently through historical and counterfactual replay.
3. **Observer-qualified selective requalification:** no claim is preserved as unaffected unless the relevant dependency observer's completeness is itself qualified.
4. **Non-compensatory claim composition:** no aggregate score or model quality can erase a mandatory identity, provenance, applicability or chronology failure.
5. **Evidence-action contracts:** every proposed investigation is tied to a precise unresolved predicate, authorization boundary and expected artifact contract; completion alone does not satisfy the claim.
6. **Independent reconstruction benchmark:** a third-party verifier reconstructs disposition and reason codes from exported inputs, not from the originating AERO runtime.

Do not describe any of these as industry-first or SOTA without a structured prior-art review and a matched independent benchmark.

## 5. How to outperform responsibly

AERO should aim to outperform on **measured assurance quality and cost**, not on model fluency or dashboard breadth.

- Safety: zero observed critical false-review decisions in a frozen, independently adjudicated challenge set, accompanied by exact one-sided confidence bounds and per-critical-stratum sample counts. For example, zero adverse events in 299 independent episodes supports only an upper 95% bound below 1%, not zero risk.
- Recall: 100% detection of affected claims in critical dependency families before reducing unnecessary requalification.
- Temporal fidelity: exact disposition/reason-code agreement on independently authored historical replay oracles.
- Reproducibility: clean-room replay with explicit evidence disclosure classes and no undeclared dependencies.
- Human factors: reduce reviewer time without increasing critical error, false rejection or automation bias.
- Robustness: blinded root-cause families, coupled faults, mutation testing, and independent implementations.
- Transfer: repeat the benchmark on at least one unrelated engineering domain only after the structural-material family is independently qualified.

AERO should publish negative results and per-stratum failures. Any benchmark that hides false rejection, abstention, reviewer intervention, or incomplete observation is not a credible comparison.

## 6. Research risks and anti-claims

- A valid JSON Schema does not prove an assurance argument is sound.
- A cryptographic digest does not prove producer authority or source truth.
- A valid signature does not prove semantic applicability.
- A validated model is not automatically valid outside its context of use.
- An empty impact set does not prove no impact when observer coverage is incomplete.
- A reproducible decision is not necessarily a correct decision.
- An LLM-generated defeater is a hypothesis until verified.
- A zero-failure synthetic benchmark is not a field failure-rate estimate.
- No software-only benchmark establishes airworthiness or certification.

## 7. Recommended next scientific experiments

1. **Ablation study:** compare full AERO with no temporal replay, no owner authority binding, no observer coverage, no non-compensatory gate, and no capsule replay.
2. **Cross-owner substitution challenge:** same part number, different serial/batch/configuration/producer; independently adjudicate exact identity.
3. **Temporal counterfactual challenge:** one evidence episode replayed across three evidence/trust cut combinations.
4. **Observer fault injection:** remove, delay or corrupt source observations and measure whether AERO overclaims unaffected claims.
5. **Clean-room replay:** export capsules under included, digest-only, withheld and authorized-access-required disclosure classes.
6. **Defeater utility study:** measure whether generated defeaters find independently seeded omissions without increasing unsupported rejection.
7. **Human factors study:** blinded reviewers with and without AERO; measure error, time, reliance, disagreement and confidence calibration.

8. **CM4QC evidence-chain integration:** for one tightly bounded material/process use case, connect an owner-controlled model report, its context-of-use envelope, validation-data provenance, uncertainty budget and hierarchy of benchmark measurements to a claim capsule. Evaluate whether reviewers can reconstruct the evidence chain without AERO interpreting a model beyond its owner-stated domain.

These experiments should be frozen and independently reviewed before performance claims are made.
