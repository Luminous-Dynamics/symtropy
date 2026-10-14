# Flourishing assessment contract v0.1

This directory contains the **proposed** Humanity Atlas flourishing-evaluation contract and the first executable guardrails. It is not an empirical evaluation, a policy recommendation, or deployment authorization.

## Files

- `flourishing-evaluation-contract-v0.1.md` — research and decision protocol.
- `flourishing-assessment-v0.1.schema.json` — Draft 2020-12 assessment wire schema.
- `constraint-preregistration-v0.1.schema.json` — versioned schema for the exact constraint declaration bytes.

Each registered artifact has a closed `role` field (for example `outcome-evidence`, `pre-registration`, `source-dataset`, `model-output`, `calibration-record`, or `transformation-log`) as well as a human-readable `purpose`. The validator makes security decisions from `role`, never from free-text `purpose`.
- `fixtures/flourishing-assessment.not-evidence.json` — schema-valid example with no evidence and all outcomes unknown.
- `../../scripts/research/validate_flourishing_assessment.py` — schema, cross-record, chronology, and optional artifact-byte verification CLI.
- `../../scripts/research/test_validate_flourishing_assessment.py` — failure-oriented regression tests.

## Run locally

From the repository root:

```sh
python -m pip install "jsonschema==4.26.0"
python scripts/research/validate_flourishing_assessment.py \
  docs/humanity-atlas/fixtures/flourishing-assessment.not-evidence.json
python -m unittest discover -s scripts/research -p 'test_validate_flourishing_assessment.py' -v
```

The CLI verifies every declared relative artifact path against bytes beneath the input JSON's parent directory by default. Use `--artifact-root DIR` when artifacts live in a separate directory, and `--report FILE` to save the machine-readable report. It rejects path traversal, control characters, duplicate normalized paths across artifact IDs, distinct paths resolving through symlinks to the same file, missing evidence files, and digest mismatches. It also rejects identical SHA-256 bytes registered under different evidence roles, including a copied pre-registration record relabelled as measured-outcome evidence. The report writer refuses to overwrite the input assessment, schema, or any registered artifact. It never downloads a URI or treats a digest-shaped string as proof of source authenticity.

## Interpretation of the report

The verifier produces per-alternative feasibility statuses only:

- `INELIGIBLE`: at least one mandatory constraint is known to fail.
- `UNRESOLVED`: a mandatory constraint cannot be evaluated from known evidence.
- `OUTCOME_INCOMPLETE`: hard constraints pass, but at least one registered outcome is unknown.
- `READY_FOR_COMPARISON`: all registered outcomes are known and hard constraints pass.

`READY_FOR_COMPARISON` does not mean the model is valid, the intervention causes an improvement, legal or ethical approval has been granted, or the candidate is preferable. The verifier deliberately sets `ranking: null` and does not calculate a composite flourishing score. Reviewers must inspect the outcome vector, uncertainty, subgroup effects, and assumptions before drawing conclusions.

For a constraint scoped to `overall_and_subgroups`, every alternative must provide at least one subgroup record and all alternatives must declare the same subgroup identifiers. Represent missing data using an explicit `UNKNOWN` row for that group instead of omitting the group. If subgroup sets differ, an otherwise passing constraint becomes `UNKNOWN` with reason `subgroup_coverage_mismatch`. A known violation is still reported as `FAIL` even if another subgroup is unknown or subgroup coverage differs; missing observations do not erase observed harm.

Known outcomes must explicitly characterize uncertainty as either `QUANTIFIED` or `NOT_QUANTIFIED`. Quantified intervals identify their interpretation: `coverage_interval`, `plausible_bounds`, `scenario_envelope`, `sensitivity_range`, or `measurement_tolerance`. A coverage interval must state its coverage probability and method; this metadata does not itself prove that the stated coverage is calibrated. A known point estimate with `NOT_QUANTIFIED` uncertainty cannot clear a hard constraint.

Every hard constraint also requires a `preregistration_artifact_id`. The referenced artifact must have machine-readable role `pre-registration`, a valid SHA-256 digest, and a collection timestamp consistent with the declared pre-registration time and analysis chronology. Its bounded (64 KiB maximum) strict-JSON bytes must bind the exact constraint ID, metric, comparator, threshold(s), scope, non-compensable flag, and declaration timestamp; the wire format is defined in [constraint-preregistration-v0.1.schema.json](constraint-preregistration-v0.1.schema.json). Merely labelling an arbitrary file as a pre-registration artifact is insufficient. A pre-registration artifact cannot serve as evidence for a measured outcome.

This still does not prove signer identity or trusted time: content-addressing establishes byte identity, not authenticity. Crucially, calling the semantic checker without `verify_artifacts=True` never qualifies a hard constraint; the report records `artifact_bytes_verified: false` and constraints remain unresolved. The CLI enables byte verification and checks all referenced files. `artifact_bytes_verified` is also false when the assessment declares no artifacts, so an empty registry never looks like positive evidence verification. A missing or unmatched pre-registration artifact keeps the constraint `UNKNOWN` or makes the assessment invalid even when the outcome value appears to pass. The checked-in NOT-EVIDENCE fixture deliberately uses `null` here.

When a quantified outcome supplies an uncertainty envelope, constraint evaluation is conservative: for an upper-bound constraint, the full envelope must lie at or below the threshold to pass; for a lower-bound constraint, the full envelope must lie at or above it; for a between-range constraint, the full envelope must lie inside the permitted range. An envelope that overlaps a constraint boundary yields `UNKNOWN`, not a pass. An envelope wholly outside the permitted range yields `FAIL`. These semantics treat the declared interval as a qualification envelope; they do not assume an interval is a probabilistic guarantee.

The current implementation validates registry references, units, timestamps, tri-state missingness, finite numbers, uncertainty bounds and declared interpretation, preregistered threshold chronology, and artifact digests. It does not establish the validity of an instrument, data publisher, causal design, interval calibration, or simulator.

## Why a vector rather than one score?

The OECD's 2025 subjective well-being guidance distinguishes life evaluation, affect, and eudaimonia rather than treating any one response as the full construct. The wider OECD well-being framework combines subjective outcomes with material conditions, health, skills, safety, social connections, environmental quality, distribution, and resources for future well-being. UNDP has likewise emphasized that composite human-development indices are useful focus measures but are not comprehensive. This project therefore keeps the unweighted outcome vector visible and never lets a composite value override a non-compensable constraint.

References:
- OECD (2025), [Guidelines on Measuring Subjective Well-being](https://www.oecd.org/en/publications/oecd-guidelines-on-measuring-subjective-well-being-2025-update_9203632a-en.html).
- OECD, [Measuring well-being and progress](https://www.oecd.org/en/topics/sub-issues/measuring-well-being-and-progress.html).
- UNDP, [The Human Development Index — what it is and what it is not](https://hdr.undp.org/content/human-development-index-what-it-and-what-it-not).
- NIST, [Measurement Uncertainty](https://www.nist.gov/itl/sed/topic-areas/measurement-uncertainty) and [Reporting Uncertainty](https://www.nist.gov/pml/nist-technical-note-1297/nist-tn-1297-7-reporting-uncertainty).
- OECD, [Decision-making under uncertainty](https://www.oecd.org/en/publications/managing-climate-risks-facing-up-to-losses-and-damages_55ea1cc9-en/full-report/component-8.html).
- NASA, [Systems Engineering Handbook — Appendix D: Requirements Verification Matrix and Appendix E: Validation Plan](https://www.nasa.gov/reference/system-engineering-handbook-appendix/): link each requirement to its source, verification method, validation plan, evidence, and disposition of anomalies.
- W3C, [PROV Model Primer](https://www.w3.org/TR/prov-primer/): represent the entities, activities, and agents involved in producing data, with derivation and responsibility links.
- Research Object Crate, [RO-Crate 1.2 data entities](https://github.com/ResearchObject/ro-crate/blob/main/docs/_specification/1.2/data-entities.md): describe files and datasets as explicit entities and link them through a package-level metadata graph.

**Evidence boundary:** the fixture is explicitly not evidence. The tests establish behavior of the schema and verifier only. They do not show that any real-world intervention improves well-being.
