# SIM-REL-051 — protected-anchor linking and equating for dynamic scientific benchmarks

Status: first common-anchor/common-reference-panel linking implementation on a research branch. Exact-head qualification pending. This implementation provides conservative structural eligibility plus an exact-rational mean/sigma link; it is not a complete IRT calibration or proof of population measurement invariance.

## 1. Why benchmark scores need longitudinal equating

SIM-REL-049 intentionally refreshes scientific tasks to resist saturation. That improves security and task novelty, but a score shift between releases can reflect a changed task mix, construct blueprint, domain distribution, generator, harness, resource budget or reference-agent panel—not an improvement in the agent.

SIM-REL-050 solves hierarchical pseudo-replication within a release. SIM-REL-051 addresses comparability across releases.

Princeton SAgE evaluation guidance recommends psychometric linking/equating with a common anchor subset for dynamic benchmarks, distribution-shift tracking and versioning of every benchmark component. The guidance distinguishes longitudinal score continuity from simply publishing a new leaderboard.

## 2. Strict result semantics

The linker returns exactly one status:

- `Equated`: same declared construct family and blueprint, score semantics, harness/toolset, resource budget and reference panel; byte-identical protected anchors; complete anchor coverage; stable paired anchor responses; common-person score panel and valid transformation.
- `LinkedOnly`: construct family and scoring/harness/panel semantics are compatible and stable, but the blueprint differs. The receipt may document a relation for exploratory analysis; it may not transfer scores or label the forms interchangeable.
- `NotComparable`: construct family, score semantics, harness, tools, budget or reference panel mismatch; anchor identity drift; excessive anchor-response drift; insufficient coverage; exhausted reuse budget; or invalid reference score distributions.

A drift failure overrides LinkedOnly: unstable anchors mean even a weak link is not justified.

## 3. Immutable form and anchor identities

Each form binds:
- benchmark ID/version, immutable sealed release ID/digest;
- construct family and blueprint digests;
- score semantics digest;
- harness/toolset digests;
- fixed reference-panel digest;
- exact resource budget.

Each anchor task binds:
- exact public task digest;
- sealed oracle commitment and independent oracle-verification receipt digest;
- environment digest;
- domain/mechanism family;
- concept and recipe-template digests;
- difficulty band.

The anchor task identity must be byte-identical in the private bank and both compared forms. A changed task, oracle, environment, concept or recipe is anchor identity drift and makes the link NotComparable.

## 4. Protected anchor bank and reuse limits

The immutable bank snapshot binds:
- bank ID/version and snapshot digest;
- reuse policy digest;
- anchor identities;
- current reuse count and maximum per-item count;
- prior exposed release IDs.

The link plan freezes:
- exact selected anchor IDs;
- domain × mechanism-family minimum coverage;
- difficulty-band coverage;
- reference-agent strata and minimum counts;
- minimum reference panel size;
- repetitions per anchor/agent;
- maximum permitted reuse count;
- maximum per-anchor paired response disagreement.

Each link increments an anchor's reuse by two form exposures. It rejects duplicate exposure IDs, current-count/ledger mismatch, current release already in exposure history, or reuse above either the item cap or plan cap. The anchor bank owner must rotate anchors before the cap is reached.

Reuse caps reduce contamination; they do not prove zero exposure. The exposure ledger must include private and public access. A released anchor is no longer secret.

## 5. Common reference-agent panel and paired anchor responses

Both forms must use the same:
- exact reference agent IDs and versions/configuration digests;
- prompt/scaffold identity represented by the configuration digest;
- stratum labels;
- reference-panel digest;
- resource budget;
- anchor task/environment/oracle identity.

The response grid is complete and paired over:
`selected anchor × reference agent × configuration × repetition index`

Each pair binds the same seed commitment and agent configuration. Missing, duplicate or mismatched rows reject the link.

The drift report separates:
- safe-success disagreements;
- any-hazard disagreements;
- abstention disagreements;
- response-validity disagreements;
- valid and invalid paired responses.

A validity mismatch is itself a drift comparison. It is not counted as a valid paired observation, but cannot be dropped from the drift denominator.

Anchor disagreement is reported per item and by maximum/mean item rates. The drift threshold is fixed before evaluation. A failing item cannot be silently removed after outcomes are observed; changing the anchor set requires a new preregistered link plan and a fresh independent review.

## 6. Initial linear mean/sigma link

The first implementation computes exact rational mean and population variance of the full-form reference-agent score for each form. The newer form is placed on the older form's reference scale using:

`old_mean + slope × (new_score − new_mean)`

where:

` slope = sqrt(old_reference_variance / new_reference_variance) `

All means/variances are exact rationals. The square-root is enclosed by integer lower/upper micro-unit bounds; these are deterministic rounding bounds, **not** a statistical confidence interval for the slope.

The receipt separately reports the paired mean score difference between the same reference agents and a pointwise conservative 95% Hoeffding interval for the bounded [-1,1] per-agent differences. That interval does not account for all model-panel sampling, anchor selection or psychometric estimation uncertainty; a production equating claim should propagate those uncertainties with an independently qualified psychometric provider.

The transformation is permitted only for Equated. LinkedOnly may retain the descriptive transform record, but `score_transfer_permitted=false`. NotComparable has no score transform.

## 7. Measurement invariance and DIF boundary

Paired anchor response disagreement is a first diagnostic, not full differential-item-functioning (DIF) analysis. It detects observed item drift across the fixed reference panel; it does not show that anchor difficulty/discrimination is invariant across all agent families, prompts, domains or future model generations.

Production expansion should add:
- stratified item-response and hazard/abstention drift;
- DIF tests across model/scaffold families and domain/family strata;
- item characteristic curves / IRT provider with independently qualified calibration;
- uncertainty propagation for anchor selection, scale transformation and reference-panel sampling;
- common-item and common-person designs where feasible;
- fixed reference forms to prevent link-chain uncertainty accumulation.

When item behavior differs materially by agent stratum, report stratum-specific links or NotComparable; never average a material DIF signal away.

## 8. Qualification

Focused tests cover:
- exact rational mean, variance and ratio transforms;
- stable common anchors and identical construct produce Equated;
- blueprint changes produce LinkedOnly with no score transfer;
- excessive item drift produces NotComparable;
- validity mismatch is counted as drift and retained separately from valid paired observations;
- incomplete anchor × agent × repetition grids reject;
- changed agent configuration/stratum/seed pairing rejects;
- changed anchor task/oracle/environment identity rejects;
- reuse count caps and exposure inconsistencies reject;
- changed panel identity rejects;
- integer square root is bounded correctly across the u128 domain.

Qualification requires pinned Rust 1.96 formatting, focused tests, full locked crate tests, strict Clippy, exact formatted-head and clean-tree verification.

## 9. Claim boundary

A passing receipt supports longitudinal score linking for the exact registered pair of forms, anchor bank snapshot, reference-agent panel, harness and budget. It does not prove:
- that the anchor bank represents all scientific concepts;
- that common reference agents represent the future agent population;
- full IRT invariance or absence of DIF across unobserved subgroups;
- zero contamination outside the declared exposure ledger;
- that two different blueprints measure the same construct;
- external-world improvement by an agent.

Report the exact releases, anchors, panel and status with every longitudinal score. If the form is only LinkedOnly, label the result as exploratory and do not compare its raw score to the prior form as though the scales were equivalent.
