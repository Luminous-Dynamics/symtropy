---
title: Rights-Aware Recovery Field Validation Protocol
version: 0.1
status: research-design
date: 2026-10-18
issue: https://github.com/Luminous-Dynamics/symtropy/issues/1929
---

# Rights-Aware Recovery Field Validation Protocol v0.1

## Purpose

Symtropy's robust frontier, adaptive regret, versioned triggers, family-held-out applicability, content-bound classification records and multi-axis pathway work are synthetic contract tests. They do not establish that the decision system performs well on real sites, assets, regulations or independently measured outcomes.

This protocol defines the bridge from contract correctness to empirical validation while explicitly separating:
- decision-time evidence from later outcomes;
- data-use rights from the authority to do physical work;
- known zero from unknown or unmeasured values;
- a model proposal from an externally authorized and actually performed operation;
- record integrity from source authenticity and truth.

The current packet is a synthetic protocol demonstration only. It has no actual field locations, permits, professional credentials, real observations, or actuation authority.

## Paradigm shifts

### From synthetic success to preregistered empirical validity

Before a retrospective replay starts, freeze sample selection, inclusion/exclusion rules, the information cutoff, missingness/censoring handling, comparator, outcomes, thresholds, measurement procedures, analysis method and decision manifest. Bind each case to an exact canonical decision manifest and digest. Later outcomes are opened for evaluation only after the decision snapshot is frozen.

### From one ground-truth label to separate event histories

The contract separates four events:
1. information observable and available at decision time;
2. the model's versioned proposal;
3. any operation separately authorized and performed by accountable people;
4. subsequent measured outcome, independent adjudication, and censoring/follow-up state.

A post-decision observation must not enter decision-time inputs. `NOT_MEASURED`, `INACCESSIBLE`, `CENSORED`, `CONTRADICTORY` and `NOT_APPLICABLE` are never numeric zero. A measured zero is represented as `measurement_state=MEASURED` with an explicit value, unit, scale, time and evidence reference.

### From an overall score to distinct hard gates

Data-use rights, legal authority, site access, owner consent, ecological review, safety review, qualified operator, transport/disposal, applicability, evidence sufficiency and outcomes remain separate axes. A BLOCK or UNKNOWN gate cannot be offset by lower cost, greater yield or faster timing. `REVIEW_ONLY_CANDIDATE` means eligible for independent study/review only; it is not permission to perform work.

### From public discoverability to rights-aware data stewardship

A public URL does not automatically grant permission for any secondary use. Each record must declare license/use status, allowed purposes, retention, source provenance and location-disclosure policy. Public receipts use pseudonymous asset/site IDs. Sensitive coordinates and personal information must not be published merely because a digest can be generated.

### From immediate deployment to a staged evidence ladder

1. **Synthetic contract tests:** schemas, mutations, time cutoffs, exact-head receipts and no operation authority.
2. **Rights-cleared retrospective replay:** preregistered outcomes, held-out episodes, frozen decision cutoffs and independent adjudication.
3. **Shadow mode:** produce proposals that do not influence decisions or physical operations; accountable professionals retain their existing process.
4. **Controlled study:** only after independent review, required permissions/consents, risk assessment, qualified personnel, measurement instrumentation and an approved study protocol.

A pass at one level does not authorize the next.

## South African context and legal boundary

Official South African government material explains that natural forests are protected and that activities affecting protected forests and protected trees are regulated. DFFE forestry forms/guidance identify supporting material such as identity details, spatial maps/GPS coordinates, and evidence of legal acquisition where applicable:
- https://www.gov.za/about-sa/forestry
- https://www.gov.za/documents/national-forests-act
- https://www.dffe.gov.za/index.php/fom_forms

This protocol does not interpret the law for an individual tree, species, site, land tenure or proposed activity. The applicable provision and commencement must be checked for the actual circumstances by the competent authority or qualified adviser. Dead, fallen, storm-damaged or salvage material must not be presumed exempt. A legal/permit decision and any other required authorization must be recorded by an accountable study owner before intervention. Unknown authority, access, consent, ecology, safety, operator or transport/disposal status means hold; a blocked gate means stop.

## Packet and decision-manifest contract

Schema: `schemas/recovery/field-validation-packet-v0.1.schema.json`

Synthetic fixture: `docs/tech/recovery-field-validation-packet-v0.1.json`

Every case includes a canonical compact JSON `decision_manifest_payload_json` and a SHA-256 `decision_manifest_sha256`. The manifest binds case ID, profile version, taxonomy version, criteria regime, decision day and sorted decision-time evidence references. The Rust verifier parses the manifest into its versioned structure, reserializes it, compares exact canonical bytes, checks the digest and cross-checks every semantic field against the record. Rehashing a semantically altered manifest cannot bypass these checks.

The packet includes:
- a versioned profile and required decision-time evidence tags;
- evidence records with `DECISION_TIME` or `POST_DECISION_OUTCOME`, observed and available timestamps, source reference, evidence tags and explicit `UNVERIFIED_SYNTHETIC` authenticity;
- pseudonymous assets, decision cutoff and manifest binding;
- a separate list of post-decision outcome references, adjudication status and censoring state;
- data rights (license/use status, permitted purposes, retention and location-disclosure rule);
- seven independent authority gates;
- typed measurement states, units/scale, observation times, uncertainty and evidence linkage;
- a claim ceiling that explicitly denies field observations, verified permits/professional credentials and operation authority.

The JSON Schema checks the outer shape. The Rust semantic audit additionally verifies record identity, reference resolution, gate/evidence binding, manifest integrity, time cutoffs, measurement semantics and disposition precedence.

## Disposition precedence

The audited study disposition is recomputed—not trusted from the input label—in this order:

1. `REASSESS_SCOPE` when profile/version/taxonomy/criteria or family classification falls outside the declared profile.
2. `BLOCKED_DATA_RIGHTS` when use is blocked or retention is expired; `HOLD_DATA_RIGHTS` when rights are unknown or insufficiently declared.
3. `BLOCKED_AUTHORITY` if any required external gate is blocked; `HOLD_AUTHORITY` if any is unknown.
4. `HOLD_EVIDENCE` if any required metric lacks an adequate measured value and valid decision-time provenance.
5. `REVIEW_ONLY_CANDIDATE` only when all previous gates pass.

A syntactically present authority record is not proof it is authentic or applicable. All records in this fixture remain synthetic. Any packet-wide audit error suppresses all candidate promotion.

## Frozen synthetic cases

The fixture retains seven cases:
- a review-only candidate with a measured zero mass;
- unknown legal authority;
- blocked data-use rights;
- incomplete hazard classification;
- blocked ecological review;
- a required metric that is not measured, with censored follow-up;
- unknown data rights.

Expected dispositions are one each of `REVIEW_ONLY_CANDIDATE`, `HOLD_AUTHORITY`, `BLOCKED_DATA_RIGHTS`, `REASSESS_SCOPE`, `BLOCKED_AUTHORITY`, `HOLD_EVIDENCE` and `HOLD_DATA_RIGHTS`. The seven episodes remain in the denominator: three `NO_ACTION`, three `REFERRED`, and one `FOLLOW_UP_CENSORED`. These are fixture counts—not real-world rates, recommendations or evidence of benefit.

## Proposed empirical benchmarks

Targets for a future rights-cleared, preregistered study—not achieved results:
- 100% of included records contain data-use rights, provenance, observation/availability times, measurement state, unit/scale where applicable, and exact decision manifest digest.
- Zero decision-time inputs whose observed or available time is after the decision cutoff.
- Zero post-decision outcome references accepted as decision-time inputs.
- Zero candidate promotion when any legal, access/consent, ecological, safety, operator, transport/disposal or data-rights gate is BLOCK/UNKNOWN.
- 100% retention of no-action/referral/censored cases under frozen inclusion rules.
- Zero conversion of an unmeasured, inaccessible, censored, contradictory or not-applicable state into numeric zero.
- On a rights-cleared held-out sample, report false in-scope promotion by failure family with denominators/confidence intervals, risk/coverage, deferral burden, outcome completeness, review backlog, decision latency and regret against a clearly labelled hindsight comparator.
- Report ecological, service, cost and access outcomes independently; do not use an aggregate weight to offset a hard-gate failure.
- No cost, recovery-yield, habitat, safety or compliance improvement claim until measurement uncertainty, counterfactual design, independent adjudication and replication support it.
- Do not infer a zero true-world error rate from observing zero errors in a finite sample.

## Research references

- South African Government forestry overview: https://www.gov.za/about-sa/forestry
- National Forests Act, 1998 and amendment/commencement details to verify for the specific activity: https://www.gov.za/documents/national-forests-act
- Department of Forestry, Fisheries and the Environment forestry forms: https://www.dffe.gov.za/index.php/fom_forms
- NIST SP 800-63A-4 distinguishes evidence authenticity, accuracy and validity; cited as a conceptual assurance reference, not a claim this packet follows identity-proofing requirements: https://pages.nist.gov/800-63-4/sp800-63a.html
- NIST FIPS 186-5 describes digital signatures; the synthetic packet has no signature, trust store or issuer-authentication mechanism: https://csrc.nist.gov/pubs/fips/186-5/final

## Claim ceiling

Every record and source is synthetic. The schema and Rust audit establish packet-contract behavior only. They do not verify a real permit, consent, credential, legal authority, source authenticity, field accuracy, safety, ecological impact, ROI or compliance. No physical action is initiated or authorized.
