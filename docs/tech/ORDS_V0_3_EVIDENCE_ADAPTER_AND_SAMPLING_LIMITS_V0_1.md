# ORDS v0.3 Evidence Adapter and Sampling Limits v0.1

**Status:** design and synthetic-contract implementation; not a field study or prevalence model.

## Purpose

This adapter imports a row-oriented ORDS v0.3 CSV into deterministic JSON Lines while preserving upstream strings, release/licence lineage, mapping uncertainty and row accounting. It is deliberately an observation adapter, not a repair-outcome oracle.

The Open Repair Alliance's October 2026 download page identifies the dataset as ORDS v0.3, CC BY-SA 4.0, last updated in October 2026 with data through July 2026. The standard covers community repair-event observations; it is not a random sample of all appliance failures. See the [download page](https://openrepair.org/open-data/downloads/), [ORDS v0.3 standard](https://standard.openrepair.org/standard.html), and [16 October 2026 release note](https://openrepair.org/announcements/over-half-a-million-records-of-open-data-on-repair/).

## Paradigm shift

Treat repair data as **observations plus an explicit selection mechanism**, not as labeled truth that directly trains a repair policy. Keep source meanings, raw text, source identity, mapping version, uncertainty and sampling limits attached to every derived row. The adapter makes task/barrier vocabulary searchable while putting a hard boundary between community-event reports and physical engineering proof.

## Output contract

- Every source row produces exactly one JSONL row, either `accepted` or `quarantined`. CSV structure errors, absent required columns, duplicate column names or invalid mapping configuration fail closed.
- The SHA-256 of the exact input bytes, mapping profile and schema is reported and copied to each row. Dataset release, ORDS version, source URL, retrieval date, CC BY-SA 4.0 licence, provider, group, source ID and event/record dates are retained.
- Every input column/value is copied into `raw_fields`. Original `partner_product_category`, `product_category`, `brand`, repair status, problem text, and barrier text are never overwritten.
- Product-category mapping is versioned and exact after whitespace collapse/case-folding. A family mapping is a low-confidence **candidate** only; aggregate categories may stay ambiguous; unknown categories remain unmapped. Brand normalization only trims/collapses whitespace and case-folds; it does not perform entity resolution.
- ORDS statuses remain distinct: `Fixed`, `Repairable`, `End of life`, and `Unknown`. Empty/zero/Unknown maps to unknown, not a failed repair. Any unrecognized status is quarantined with the raw value preserved.
- `repair_barrier_if_end_of_life` is normalized only for an `End of life` row. A barrier on any other status, or an unrecognized barrier on an end-of-life row, is quarantined.
- If `provider + group + id` repeats, **all colliding rows** are quarantined, not first-wins. Missing provider or ID gets an explicit identity exception. Group identifier is optional in ORDS v0.3, so an absent group is preserved as empty and never silently invented.
- Output can never generate verified restoration, safety qualification, part-fit, non-recurrence or population-prevalence claims. A community `Fixed` value reflects event participants' assessment; it is not a measured restoration receipt or a safety test.

## Usage

Use a copy of the mapping profile for the intended dataset release and set `profile_status` to `operational_config`; update release ID and retrieval date, retain the official URL and licence, and review category mappings. The checked-in profile and CSV are synthetic.

```sh
cargo run --manifest-path tools/household-appliance-bench-audit/Cargo.toml -- \
  --ords-schema schemas/appliances/ords_adapter_mapping_v0_1.schema.json \
  --ords-mapping fixtures/appliances/ords_adapter_mapping_v0_1.json \
  --ords-csv /path/to/ora-eee.csv \
  --ords-output /path/to/ords-observations.jsonl
```

The output is a line-oriented file; the CLI's ordinary JSON report includes row counts, quarantines and input/profile/schema digests. No dataset rows are downloaded or embedded by the test fixture.

## Contract fixture

The ten-row fixture is entirely synthetic. It covers a fixed event outcome, repairable outcome, empty/zero unknown, valid end-of-life barrier, invalid barrier, invalid status, duplicated identity, a barrier/status mismatch, ambiguous aggregate category and unmapped category. Expected accounting: 10 rows; 5 accepted and 5 quarantined (the two repeated IDs are both quarantined); all 10 are emitted. These are contract examples, not empirical results.

## Benchmarks and acceptance gates

| Measure | Acceptance rule | Meaning |
|---|---|---|
| Row reconciliation | 100% of parsed rows emitted exactly once as accepted or quarantined | No silent drop |
| Provenance coverage | 100% of emitted rows carry dataset release, standard, source URL/licence, retrieval date, source digest and raw source identity fields | Reproducible lineage |
| Raw field retention | 100% exact raw values survive import, including partner categories, brands and free-text problem fields | Normalization is reversible |
| Enum integrity | Zero unrecognized values silently coerced; all invalid status values quarantined | Fail closed |
| Barrier integrity | Zero accepted rows with barrier on non-end-of-life status or unknown barrier vocabulary | Correct status-dependent semantics |
| Duplicate identity | 100% of rows in an identity collision group quarantined | No arbitrary first-wins identity |
| Evidence claim ceiling | Zero event rows produce restoration, physical-safety, part-fit, non-recurrence or prevalence claims | No category error |
| Reproducibility | Identical input/profile/schema bytes yield identical digests and deterministic JSONL | Reproducible pipeline |

These are software contract targets. They are not evidence that the source sample represents South African washer failures, that repairs are durable, or that any appliance is safe. Before estimating prevalence, preregister target population, event/provider coverage, product-family mapping, duplicate policy, missingness, inclusion mechanism, weighting model and sensitivity bounds. ORDS may inform research taxonomy and coverage gaps; it must not be presented as a representative population sample by default.

## Sources and rights

- Open Repair Alliance, [Download the dataset](https://openrepair.org/open-data/downloads/) — October 2026 update, ORDS v0.3, CC BY-SA 4.0.
- Open Repair Alliance, [ORDS v0.3 standard](https://standard.openrepair.org/standard.html) — repair-status/barrier semantics, required and optional fields, and raw partner category lineage.
- Open Repair Alliance, [16 October 2026 release](https://openrepair.org/announcements/over-half-a-million-records-of-open-data-on-repair/) — event/provider/country coverage and community source context.

Derived mappings must retain attribution and comply with ShareAlike obligations. This document does not embed or redistribute the source dataset.
