# MFG-DATA-001 — tool-wear evaluator

This is a narrow, deterministic Rust evaluator for a pinned machine-tool-wear feature table. The evaluator software is licensed under GNU AGPL-3.0-or-later; see this directory's full `LICENSE` text. The upstream measurement data has its own separate licence and is not copied into this repository. This is not a calibrated process twin, maintenance controller, machine-independent qualification, or manufacturing-release tool.

## Run

Obtain the pinned CSV from the source repository and verify its bytes using tools/toolwear-eval/source.lock before running these commands:

    cargo test --locked --offline --manifest-path tools/toolwear-eval/Cargo.toml
    cargo run --locked --offline --manifest-path tools/toolwear-eval/Cargo.toml -- --data /path/to/Dataset_Machine_tool_wear.csv

The executable rechecks the expected SHA-256 itself. Any byte change, altered schema, malformed row, non-finite numeric value, unexpected tool identity set, or duplicate record fails closed. It uses only the Rust standard library; the evaluator does not ingest or mirror the source dataset in this repository.

## Locked source and attribution

- Source: https://github.com/Vicomtech/dataset-machine-tool-wear/tree/fa7941101847541cdd31339d6481ee61d152d80f
- Data file: https://github.com/Vicomtech/dataset-machine-tool-wear/blob/fa7941101847541cdd31339d6481ee61d152d80f/Dataset_Machine_tool_wear.csv
- Source licence: https://github.com/Vicomtech/dataset-machine-tool-wear/blob/fa7941101847541cdd31339d6481ee61d152d80f/LICENSE.txt; its text declares CC BY-SA 4.0.
- Attribution: the dataset is developed by Vicomtech; the README thanks Mondragon Unibertsitatea for facilities and the underlying raw data.
- Nuance: GitHub's repository metadata reports NOASSERTION despite the checked-in licence text. This intake record preserves both facts and is not a legal opinion about downstream use.
- The raw CSV is deliberately not committed or redistributed here. Keep source attribution and licence details with every reproduction of this evaluation.

## Fixed evaluation protocol

- Label: Vb. The pinned README describes flank wear but does not specify its physical units; the report therefore uses source-native values and does not infer units.
- The pinned table contains 1,142 blank cells in two unused sensor columns. Those are reported as missing, not imputed; tool ID, label, and all modeled feature values must be present and finite.
- Split: sort the 13 numeric tool IDs ascending and hold out every fourth identity starting at index zero. Test tools are 3, 7, 12, 16; training tools are 4, 5, 6, 9, 10, 11, 13, 14, 15. A tool can never appear in both sets.
- Baseline: a constant equal to the equally weighted mean of training-tool label means.
- Linear baselines: one-feature, tool-balanced least squares fitted on training tools only, independently for the predeclared features AE_RMS, F_c_RMS, and F_p_RMS.
- Metrics: pooled held-out MAE/RMSE and per-tool MAE/RMSE, plus the unweighted mean of tool-level metrics. This prevents a large tool group from silently becoming the only reported result.
- No hyperparameter selection, threshold choice, prediction interval, previous-segment baseline, or controller policy is fit on the held-out tools.

The report's evidence class remains external_dataset_evaluation_unqualified_for_process_control. Results on four held-out tools do not establish machine-independent generalization; a feature table does not validate raw-signal processing, sensor calibration, a full maintenance loop, or QDRIFT-SYNTH-001's synthetic process parameters.
