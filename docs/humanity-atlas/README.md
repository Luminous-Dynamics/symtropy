# Flourishing assessment contract v0.1

This directory contains the **proposed** Humanity Atlas flourishing-evaluation contract and the first executable guardrails. It is not an empirical evaluation, a policy recommendation, or deployment authorization.

## Files

- `flourishing-evaluation-contract-v0.1.md` — research and decision protocol.
- `flourishing-assessment-v0.1.schema.json` — Draft 2020-12 wire schema.
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

The CLI verifies every declared relative artifact path against bytes beneath the input JSON's parent directory by default. Use `--artifact-root DIR` when artifacts live in a separate directory, and `--report FILE` to save the machine-readable report. It rejects path traversal, symlinks resolving outside the root, missing evidence files, and digest mismatches. It never downloads a URI or treats a digest-shaped string as proof of source authenticity.

## Interpretation of the report

The verifier produces per-alternative feasibility statuses only:

- `INELIGIBLE`: at least one mandatory constraint is known to fail.
- `UNRESOLVED`: a mandatory constraint cannot be evaluated from known evidence.
- `OUTCOME_INCOMPLETE`: hard constraints pass, but at least one registered outcome is unknown.
- `READY_FOR_COMPARISON`: all registered outcomes are known and hard constraints pass.

`READY_FOR_COMPARISON` does not mean the model is valid, the intervention causes an improvement, legal or ethical approval has been granted, or the candidate is preferable. The verifier deliberately sets `ranking: null` and does not calculate a composite flourishing score. Reviewers must inspect the outcome vector, uncertainty, subgroup effects, and assumptions before drawing conclusions.

For a constraint scoped to `overall_and_subgroups`, every alternative must provide at least one subgroup record. A known violation is reported as `FAIL` even if another subgroup is unknown; missing observations do not erase observed harm.

When an outcome supplies an uncertainty envelope, constraint evaluation is conservative: for an upper-bound constraint, the full envelope must lie at or below the threshold to pass; for a lower-bound constraint, the full envelope must lie at or above it; for a between-range constraint, the full envelope must lie inside the permitted range. An envelope that overlaps a constraint boundary yields `UNKNOWN`, not a pass. An envelope wholly outside the permitted range yields `FAIL`. These semantics treat the declared interval as the qualification envelope; they do not assume an interval is a probabilistic guarantee.

The current implementation validates registry references, units, timestamps, tri-state missingness, finite numbers, uncertainty bounds, preregistered threshold chronology, and artifact digests. It does not establish the validity of an instrument, data publisher, causal design, or simulator.

## Why a vector rather than one score?

The OECD's 2025 subjective well-being guidance distinguishes life evaluation, affect, and eudaimonia rather than treating any one response as the full construct. The wider OECD well-being framework combines subjective outcomes with material conditions, health, skills, safety, social connections, environmental quality, distribution, and resources for future well-being. UNDP has likewise emphasized that composite human-development indices are useful focus measures but are not comprehensive. This project therefore keeps the unweighted outcome vector visible and never lets a composite value override a non-compensable constraint.

References:
- OECD (2025), [Guidelines on Measuring Subjective Well-being](https://www.oecd.org/en/publications/oecd-guidelines-on-measuring-subjective-well-being-2025-update_9203632a-en.html).
- OECD, [Measuring well-being and progress](https://www.oecd.org/en/topics/sub-issues/measuring-well-being-and-progress.html).
- UNDP, [The Human Development Index — what it is and what it is not](https://hdr.undp.org/content/human-development-index-what-it-and-what-it-not).

**Evidence boundary:** the fixture is explicitly not evidence. The tests establish behavior of the schema and verifier only. They do not show that any real-world intervention improves well-being.
