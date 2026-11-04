# BOCoDe trial receipts and campaign completeness v0.1

**Status:** implementation and synthetic self-tests; no real BOCoDe optimizer campaign has been run by Symtropy.  
**Parent issue:** #3533 — AERO-BOCODE-RUN-RECEIPTS-001.  
**Catalog intake:** #3525 — AERO-BOCODE-INTEROP-001.  
**Upstream:** https://github.com/rosenyu304/BOCoDe

## Principle: a process is not an experiment result

BOCoDe's upstream batch runner catches individual trial exceptions and prints `SKIP`; its process can complete with exit status zero while some scheduled optimizer calls did not produce results. Symtropy therefore treats every `(task, algorithm, seed, attempt)` as a trial with an independently verifiable receipt.

The terminal state is one of:
- `succeeded`: zero exit, no SKIP, exact summary row, retained raw NPZ and summary, and independently replayed trace structure;
- `failed`: upstream SKIP, non-zero exit, timeout, or a missing per-trial summary/NPZ;
- `invalid_trace`: raw outputs exist but fail structural or consistency checks;
- `wrapper_error`: preflight or receipt-wrapper failure, with the failure retained.

**Wrapper exit code 0 means the wrapper wrote a terminal receipt; it does not mean the optimizer succeeded.** A non-success state remains visible in `receipt.json` and `run-row.csv`. Wrapper exit code 2 means it could not complete the wrapper-level receipt path.

## Components

| Component | Responsibility |
|---|---|
| `scripts/run-bocode-single-trial.py` | Pinned one-trial execution, timeout, capture, raw artifact retention, trace preflight and immutable receipt |
| `scripts/verify-bocode-run-receipt.py` | Independent stdlib verifier of receipt identity, artifact hashes, raw metadata, command/environment, NPZ contents, upstream summary and normalized row |
| `scripts/aggregate-bocode-campaign.py` | Frozen schedule reconciliation; every missing, failed, duplicate and unmatched attempt remains visible |
| `scripts/test-bocode-run-receipt.py` | Synthetic NPZ fixtures, receipt verification, tamper controls and missing-campaign-trial control |
| `src/bocode_run_receipt.rs` | Rust parser and validation for the normalized campaign ledger rows |

The raw NPZ is not rewritten. The verifier uses the ZIP/NPY structure and standard-library scalar decoding; it does not import NumPy or allow pickle execution. It checks all archive member headers, rejects object/Fortran arrays, verifies X/y/c shapes and finite values, checks exact task/method/seed, and re-derives observed evaluation and feasibility counts where raw constraint history exists.

Some BOCoDe algorithms fold constraints into an objective penalty and do not preserve raw c observations. In those cases the receipt records `constraint_history_state=unavailable`; it does not infer feasibility from the penalty. For unconstrained problems it records `not_applicable`. Feasibility counts are reported only where raw constraint observations exist.

## One-trial command

Run inside the exact BOCoDe checkout and separately pinned environment. Obtain `metadata_sha256` from the admitted catalog row (exact raw JSON bytes), and supply the full Git commit and installed package version.

```sh
python tools/civil-aero-benchmark/scripts/run-bocode-single-trial.py \
  --bocode-root /path/to/BOCoDe \
  --result-root /path/to/immutable-results \
  --source-commit <40-character-commit> \
  --package-version <installed-version> \
  --task-id <exact-task-id> \
  --metadata-sha256 <raw-task-json-sha256> \
  --algorithm <exact-upstream-algorithm-label> \
  --seed 0 \
  --n-init 10 \
  --iters 50 \
  --timeout-seconds 3600
```

Resource/environment overrides are permitted only through the explicit reviewed allow-list and the repeatable `--environment KEY=VALUE` option. The wrapper retains:
- unique attempt/run identity and request digest;
- source commit and installed package version;
- raw task metadata bytes and digest;
- full `pip freeze --all` bytes and digest;
- exact argv and allow-listed environment projection;
- stdout/stderr, upstream summary CSV and raw NPZ;
- wall time and child user/system CPU time where measurable;
- explicit unknown states for per-trial peak memory, GPU time and energy, unless separately measured.

The BOCoDe source checkout must be clean and the installed `bocode` package must resolve inside that checkout. Result bundles are written outside the BOCoDe checkout.

## Independent verification

```sh
python tools/civil-aero-benchmark/scripts/verify-bocode-run-receipt.py \
  /path/to/immutable-results/<run-id>/receipt.json
```

Verification re-hashes each artifact and recomputes receipt, request and run identities. It independently checks:
- raw metadata SHA-256 and agreement between metadata JSON and receipt task metadata;
- exact environment projection and command digests;
- the retained dependency freeze;
- the raw NumPy archive ZIP and NPY headers, dimensions, data types, values, iteration trace and wall-time monotonicity;
- raw X row count as the evaluation count;
- constraint feasibility only when the complete raw constraint matrix is available;
- upstream summary row identity and best value against the NPZ;
- exact equality of `run-row.csv` projection to receipt fields.

This is artifact/structure verification, not solver authorship attestation, objective-semantic correctness, optimization convergence, CFD verification/validation, or physical validation.

## Campaign completeness

Freeze a schedule before execution:

```csv
task_id,algorithm,seed
Branin,random_search,0
Branin,single_task_gp,0
PressureVessel,random_search_mixed,0
```

After running all scheduled attempts:

```sh
python tools/civil-aero-benchmark/scripts/aggregate-bocode-campaign.py \
  --expected expected-trials.csv \
  --receipts /path/to/immutable-results \
  --output campaign-ledger.csv \
  --summary campaign-summary.json
```

The aggregator returns success only when every scheduled trial has exactly one independently verified successful receipt, no duplicates, no invalid receipts, no missing trials and no unmatched receipts. It still writes a ledger and summary when the campaign is incomplete. Every extra attempt is retained; there is no “keep only best attempt” behavior.

## Measurement and claim boundaries

- Evaluation count is `rows(X)`, not requested `--iters`.
- `n_init` is kept separately from observed objective calls.
- Feasible/infeasible counts are only available if raw `c` is retained and has the expected width.
- Wall/CPU measurements are recorded where available. Peak memory/GPU/energy remain unknown unless measured by a separate qualified source.
- Synthetic self-tests validate the wrapper/verifier contract only.
- A verified receipt does not prove the optimizer's mathematical correctness, objective meaning, data/license rights, model validation or physical accuracy.

## Benchmark protocol separation

For BOCoDe's official reproduction track, keep the upstream paper's method/class compatibility, shared initial design, D + 250 additional objective calls, 25 seeds and per-class metrics intact. The campaign ledger is the source for complete-case and all-attempt denominators.

A Symtropy extension may then report full engineering cost (startup, dependency setup, surrogate training, tuning, failed calls, CPU/GPU/wall/peak memory/energy, orchestration and independent verification) separately. Never alter the BOCoDe result metric and still call it the official reproduction.

The upstream campaign's 25 seeds and 250-call budget remain targets, not a campaign already run. No SOTA superiority claim is supported by receipt infrastructure or synthetic fixture results.
