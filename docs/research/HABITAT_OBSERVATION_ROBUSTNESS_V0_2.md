# Habitat Observation Robustness Benchmark v0.2

Status: proposed synthetic benchmark; implementation and CI verification pending.

## Paradigm shift: model the observer, not just the world

The v0.1 decision-impact fixture partitions only the underlying site states and implicitly assumes each survey reports its declared deterministic partition correctly. This v0.2 companion makes observation quality an explicit adversarial dimension. The state under consideration is now the pair:

`(site condition, observation mode)`

A “clear” result is not evidence of absence if a declared false-negative mode can produce it. A stale, inconclusive, or contradictory report is unresolved—not a favorable negative. False-positive modes are also represented, because over-caution and wasted investigation have costs even though they do not justify relaxing a hard safety gate.

The fixture is intentionally pessimistic: until an evidence contract rules out an observation mode for a particular instrument, protocol, coverage area, and time window, the verifier must consider that mode possible. In this v0.2 corpus, every reported class remains compatible with every underlying condition because false-positive, false-negative, and unresolved modes are all explicitly admissible. The only candidate robust across all compatible conditions is the inland fallback. This is a deliberate demonstration of the cost of unqualified evidence—not a recommendation about real sites.

## Frozen synthetic corpus

`docs/research/fixtures/HABITAT_OBSERVATION_ROBUSTNESS_V0_2.json` declares:

- 4 invented truth conditions: neither hazard, canyon-flood hazard, ridge-rock-instability hazard, both.
- 6 observation modes: `valid`, `false_negative`, `false_positive`, `stale`, `inconclusive`, `contradictory`.
- 24 composite truth × observation-mode states.
- 3 synthetic alternatives and 2 observation actions.
- Complete, pairwise-disjoint positive/negative/unresolved partitions for every action.

These are enumerated adversarial modes, not frequencies. The fixture provides no priors, likelihoods, hazard probabilities, empirical sensitivity/specificity, or expected value of information.

## Rust verifier

`tools/habitat-observation-robustness` is a standalone Rust 2024 package. It pins the exact UTF-8 fixture digest before parsing, rejects unknown fields, validates the hazard and mode schemas, requires complete disjoint partitions, and independently validates every outcome against the declared observation-mode semantics.

For each action/outcome, it calculates the set of candidates that remain feasible across every compatible underlying site condition. Candidate selection is the least-cost member of that robust-feasible set (cost and ID tie-break); it never replaces a missing set with a guessed probability or treats an unobserved hazard as absent. A direct exhaustive test oracle separately enumerates candidate × condition pairs.

The report is limited to synthetic observation semantics. A zero false-safe count means only that the implementation upheld the declared finite fixture. It is not evidence of geotechnical adequacy, survey quality, flood safety, construction suitability, code compliance, or an approval for occupancy.

## Acceptance targets

| Property | Gate |
|---|---:|
| Composite states assigned exactly once per action | 100% |
| Unknown modes, undeclared hazards, unknown fields, incomplete/overlapping partitions rejected | 100% |
| Valid-mode semantics match the measured hazard in the truth condition | 100% |
| False-negative mode with a present measured hazard remains a negative observation and does not erase the underlying hazard | 100% |
| Stale, inconclusive and contradictory modes remain unresolved | 100% |
| Selected candidate feasible across every compatible underlying condition | 100% |
| False-safe selections across frozen controls | 0 |
| Input-order invariance for semantically unordered arrays | 100% |
| Exact agreement with independently enumerated robust-feasible candidate sets | 100% |
| Probabilities or expected-VoI fields without separate calibration evidence | 0 |

The initial suite also checks frozen-input digest enforcement and SHA-256 against a known-answer test. Future corpora must add separately declared coverage gaps, instrument failure, correlated observations, changed terrain after measurement, false positives/negatives conditional on known validated operating envelopes, and independent semantic goldens. A future calibrated observation model must be a separate versioned evidence contract; calibration claims must be tested against held-out field observations and drift, never inferred from this synthetic corpus.

## Why this research direction matters

The U.S. Geological Survey notes that landslide observations are localized and episodic, while historical inventory data vary in quality, availability, and coverage; a nationwide susceptibility surface cannot serve as a parcel-level safety certificate. See the [USGS landslide inventory and susceptibility map](https://www.usgs.gov/node/278589) and its research on [data limitations in susceptibility modelling](https://www.usgs.gov/publications/overcoming-data-limitations-landslide-susceptibility-modelling).

NIST's digital-twin guidance also treats interoperability and cybersecurity/trust as first-class concerns. For buildings, provenance and authority over the evidence itself matter alongside geometry and simulation. See [NIST IR 8356](https://www.nist.gov/publications/security-and-trust-considerations-digital-twin-technology-digital-twin-technology) and [Building Digitization and Semantic Interoperability](https://www.nist.gov/programs-projects/building-digitization-and-semantic-interoperability).

## Boundaries and next step

This is a synthetic assurance harness, not a geological model or building design tool. The intended next increment is to represent an evidence validity envelope explicitly: source, survey procedure and coverage, collection time, triggering events, instrument/protocol version, independent reviewer, and what changes invalidate a result. Only after a domain expert supplies and reviews those rules may a real observation mode be narrowed. The implementation must never self-authorize the evidence envelope it uses to approve itself.
