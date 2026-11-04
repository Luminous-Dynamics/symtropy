# BOCoDe single-trial receipt protocol v0.1

**Status:** external experiment wrapper and evidence ledger under test. No BOCoDe optimizer run has yet been completed by Symtropy.  
**Parent:** AERO-MDO-INTEROP-001 (#3237).  
**Catalog intake:** AERO-BOCODE-INTEROP-001 (#3525).  
**Run receipt work:** AERO-BOCODE-RUN-RECEIPTS-001 (#3533).

## Why this exists

The upstream BOCoDe batch runner catches per-trial exceptions, prints `SKIP`, appends failures to an in-memory list and continues. Its shard summary contains successful rows only; a process exit code of zero is not proof that every scheduled task/method/seed ran successfully.

Its `Result` NPZ trace contains `X`, `y`, `c`, `n_init`, `iterations`, `per_iteration_value`, and `wall_time`. The objective evaluation count can therefore be derived from the actual `X` row count, while `n_init` and the trace are retained separately. Requested `--iters` is not used as the actual objective-call count.

## Architecture

The wrapper `scripts/run-bocode-single-trial.py` starts one upstream trial per process:

```sh
python scripts/run-bocode-single-trial.py \
  --bocode-root /src/BOCoDe \
  --result-root /data/bocode-attempts \
  --source-commit <40-character-source-commit> \
  --package-version 1.0.2.dev2 \
  --task-id Car \
  --metadata-sha256 <raw-task-json-sha256> \
  --algorithm random_search_mixed \
  --seed 7 \
  --n-init 20 \
  --iters 250
```

The wrapper requires:
- exact source Git commit and clean worktree;
- the imported `bocode` package to resolve inside that checkout;
- exact installed package version;
- exact task metadata JSON bytes matching the admitted task digest;
- an explicit allow-listed environment projection (thread counts, seed/determinism and CUDA visibility settings only).

Each attempt gets a unique UUID and a unique run directory, so retries cannot reuse stale NPZ or summary files. A deterministic request digest and run ID bind the requested task/method/seed/budget/environment identity to that attempt.

The wrapper captures:
- exact command and allowed environment projection;
- pip freeze output and its SHA-256;
- stdout/stderr bytes and digests;
- process exit code, wrapper wall seconds and child CPU user/system time deltas;
- any available upstream summary and raw NPZ, even when the trial fails or emits `SKIP`.

It then validates the summary row and NPZ structure using NumPy with `allow_pickle=False`, derives objective/constraint counts and feasibility from the raw arrays, and writes a canonical `receipt.json` plus a one-row `run-row.csv`.

## Terminal states

| State | Meaning |
|---|---|
| `succeeded` | upstream process exited 0, no `SKIP`, exactly one summary row and an identity-consistent finite NPZ trace were retained |
| `failed` | upstream emitted `SKIP`, returned non-zero, or omitted required output |
| `invalid_trace` | raw NPZ/summary were retained but failed structural or identity checks |
| `wrapper_error` | provenance, setup or wrapper-level validation failed |

Wrapper process exit 0 means only that a terminal receipt was successfully written. The trial state is the receipt's `state`. A failed/invalid trial still has a receipt and is never silently dropped. Wrapper-level failure returns 2.

## Independent verification

`scripts/verify-bocode-run-receipt.py` uses only Python's standard library. It independently:
- recomputes the request digest, attempt/run identity and receipt digest;
- verifies byte lengths and SHA-256 digests for every retained artifact;
- checks receipt and normalized row state/identity binding;
- checks NPZ ZIP integrity and required NPY members;
- parses NPY headers without NumPy and checks dimensional shapes against the task metadata;
- rejects missing files, path traversal, tampering and inconsistent objective/constraint dimensions.

`scripts/test-bocode-run-receipt.py` creates synthetic NPY/NPZ fixtures and exercises:
- a valid receipt;
- artifact tampering;
- an invalid NPZ dimension;
- a campaign whose frozen schedule contains a trial with no receipt.

The Rust `bocode_run_receipt` module strictly imports the 32-column normalized trial row, validates its state/resource/hash/count invariants and invokes the Python verifier self-test under CI.

## Campaign ledger

`scripts/aggregate-bocode-campaign.py` consumes a frozen expected schedule with columns `task_id,algorithm,seed`. It discovers all receipt files, independently verifies each, and writes:
- one row for every retained receipt, including retries and failed attempts;
- an explicit `missing` row for every scheduled tuple without a receipt;
- an aggregate JSON with scheduled, succeeded, failed, invalid, duplicate, unmatched and missing counts.

It does not choose the best retry or collapse multiple attempts. `campaign_success` requires exactly one independently verified successful receipt for every scheduled task/method/seed. A complete-case optimizer ranking can be computed later, but must be accompanied by this all-attempt ledger.

## Resource accounting boundary

Wall time and child CPU time are measured by this wrapper. Peak RSS, GPU time and energy are explicitly marked `unknown` because this wrapper does not yet use a qualified live resource collector. They must not be reported as zero. The cgroup v2 parser work in #3498 is a separate prerequisite for future measured process-tree accounting.

## Evidence limits

- A receipt hash binds bytes; it does not authenticate the upstream publisher or prove the objective implementation is correct.
- NPZ structural consistency does not establish optimizer convergence, global optimality, or physical validity.
- The catalog metadata and run receipt do not clear optional extras or dataset licensing; those remain separate admission decisions.
- The wrapper initially covers single-objective/multi-objective problem trials that export `X/y/c` through BOCoDe's standard `Result`. Dataset-pool optimization paths require a distinct candidate-index trace contract.
- The official BOCoDe benchmark score and this full-cost receipt ledger are complementary. Preserve official objective-call budgets and metrics; separately report full process/setup/resource/verification cost.

## Next qualification

1. Add a controlled BOCoDe source/environment smoke run on one synthetic task.
2. Admit one sourced, licensed engineering sentinel and execute random/LHS plus one BO baseline.
3. Compare upstream raw trace and wrapper-derived call/feasibility counts independently.
4. Run a campaign with a deliberately broken method to prove SKIP/failure retention.
5. Add cgroup v2 runtime measurement only after its process-tree lifecycle and descriptor evidence are independently qualified.
