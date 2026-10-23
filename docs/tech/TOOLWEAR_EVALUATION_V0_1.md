# MFG-DATA-001 — pinned tool-wear evaluation v0.1

Status: **external-dataset baseline evaluation only; unqualified for process control**.

Implementation code in `tools/toolwear-eval/` is licensed under GNU AGPL-3.0-or-later; its full license text is `tools/toolwear-eval/LICENSE`. This does not relicense the separately licensed upstream CSV.

## Purpose

Create the first reproducible, non-synthetic measurement-data seam for Symtropy manufacturing work. The scope is deliberately narrower than a calibrated digital twin: verify a published feature table, freeze a tool-disjoint split, fit simple baselines on training tools only, and report performance by held-out tool. The evaluation must not be promoted to sensor qualification, machine-independent prediction, maintenance advice, or manufacturing release.

## Source admission record

Source repository: Vicomtech/dataset-machine-tool-wear

- Revision: fa7941101847541cdd31339d6481ee61d152d80f
- File: Dataset_Machine_tool_wear.csv
- Git blob SHA-1: 4d44a731dcef6dac26bd4f02904edbb7cabdbc50
- Exact bytes: 3,346,122
- SHA-256: 0199bc69cae5e74be3d668244f708da78450c669be641d254b5ca6aa712dda56
- Layout: 2,054 rows, 154 columns, 13 distinct numeric tool IDs; exact repeated data rows: zero; malformed row widths: zero; non-finite Vb labels: zero. There are 1,142 blank cells, all in two unused sensor columns; required model fields are complete.
- Identity columns: tool (lowercase in the actual CSV) and Vb (flank-wear label).
- Declared licence: the pinned LICENSE.txt says CC BY-SA 4.0. GitHub's machine-readable licence metadata is NOASSERTION; retain this caveat and verify obligations before broader redistribution.
- The source CSV is fetched by a path- and commit-pinned workflow and is not copied into this repository.

## Frozen split

Sort the observed tool IDs numerically and hold out every fourth ID starting at index zero. The resulting test set is [3, 7, 12, 16]; the training set is [4, 5, 6, 9, 10, 11, 13, 14, 15]. This is deterministic, uses no label or feature values to choose groups, and keeps all records from a tool on one side. The report emits a SHA-256 split identity over the source digest and exact train/test tool lists.

This split is frozen before fitting baselines. Do not try multiple split seeds and keep the best result. With only four held-out tools, uncertainty about population generalization remains substantial; show all per-tool metrics rather than hiding weak groups behind the pooled score.

## Baselines

1. **Tool-balanced constant**: average the training mean Vb per tool, assigning equal total weight to each training tool.
2. **Tool-balanced one-feature linear regression**: fit independent weighted least-squares lines using the predeclared features AE_RMS, F_c_RMS, and F_p_RMS. Each training tool contributes equal total weight. No feature selection or threshold tuning is performed against held-out labels.

Report pooled held-out MAE/RMSE, per-tool MAE/RMSE, and the arithmetic mean of tool-level metrics. No confidence interval or prediction interval is claimed in this initial slice. Feature values and label values remain in their source representation; physical units are not guessed.

## Fail-closed checks

- Verify exact source byte count and SHA-256 before parsing.
- Require UTF-8, the exact pinned schema shape, unique non-empty column names, required columns, numeric tool IDs, finite values in present cells, fixed row/column/missing-cell counts and fixed tool ID set. Missing values are accepted only in unused columns and are reported without imputation.
- Reject missing required model fields, malformed widths, blank records, quoted fields outside the deliberately narrow source contract, non-finite values and exact duplicate data rows.
- Test the SHA-256 implementation against standard known-answer vectors.
- Verify the split assignment, tool-disjointness, weighted linear-fit arithmetic, malformed-row rejection and constant-feature failure.
- CI downloads only the pinned upstream object, rechecks its digest and size, and uploads the report as an artifact. The workflow itself must pass on the exact PR head before software conformance is claimed.

## Limits and next experiments

The source is a feature table from one turning-process dataset. It does not validate raw-signal transformations, an instrument calibration chain, a complete feedback policy, manufacturing economics or the synthetic state equations in QDRIFT-SYNTH-001. The source README does not state physical units for Vb, so this evaluator does not translate values into micrometres, millimetres, or a maintenance threshold.

Follow-up gates:
1. Have an independent reviewer reproduce the data digest and held-out assignment.
2. Compare these baselines against an untouched, separately documented dataset if one is admitted.
3. Establish measurement-system repeatability/reproducibility, calibration identity and uncertainty before linking wear predictions to physical claims.
4. Keep the Mycelix receipt contract explicit: source/model/configuration identities, evidence class, split digest, report digest, evaluator identity, domain limits, and independent verification remain separate from calibration or approval.
5. Do not connect this evaluator to machine actuation or release authority.
