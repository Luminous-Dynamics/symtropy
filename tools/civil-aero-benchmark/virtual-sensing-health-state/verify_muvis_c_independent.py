#!/usr/bin/env python3
"""Independent standard-library MuViS-C corruption-protocol verifier."""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
import math
from pathlib import Path
from typing import Any

HERE = Path(__file__).resolve().parent
FIXTURE = HERE / "data" / "muvis-c-corruption-v0.1.json"
CLAIM = "synthetic_single_channel_corruption_virtual_sensing_protocol_only"
FAILURE_MODES = (
    "bias",
    "noise",
    "scaling",
    "time_varying_scaling",
    "linear_drift",
    "nonlinear_drift",
    "outliers",
    "trimming_variable",
    "trimming_constant",
    "hard_fault",
)


class AuditError(ValueError):
    pass


def require(condition: bool, message: str) -> None:
    if not condition:
        raise AuditError(message)


def validate(data: dict[str, Any]) -> None:
    require(data.get("schema_version") == "aero-muvis-c-protocol-reference/0.1.0",
            "unsupported MuViS-C protocol schema")
    require(data.get("claim_ceiling") == CLAIM, "claim ceiling changed or weakened")
    require(isinstance(data.get("problem_id"), str) and data["problem_id"].strip(),
            "problem ID must be non-empty")

    ds = data["dataset"]
    n, t, c = ds["sample_count"], ds["time_steps"], len(ds["channel_ids"])
    require(isinstance(n, int) and n >= 2, "sample_count must be >=2")
    require(isinstance(t, int) and t >= 2, "time_steps must be >=2")
    require(1 <= c <= 64, "channel count outside supported range")
    require(ds["domain"] == "synthetic_aerospace_sensor_windows",
            "fixture must remain synthetic")
    require(ds["input_layout"] == "N_T_C", "input layout mismatch")
    require(ds["normalization"] == "per_channel_zscore_frozen_fixture_statistics",
            "normalization protocol mismatch")
    require(ds["source_status"] == "synthetic_known_answer_only",
            "source status mismatch")
    ids = ds["channel_ids"]
    require(all(isinstance(x, str) and x.strip() for x in ids),
            "channel IDs must be non-empty")
    require(len(ids) == len(set(ids)), "duplicate channel ID")

    predictor = data["predictor"]
    require(predictor["model_id"] == "frozen_linear_virtual_sensor_v0",
            "predictor identity mismatch")
    require(predictor["model_class"] == "linear_temporal_mean_regressor",
            "predictor class mismatch")
    require(len(predictor["channel_weights"]) == c,
            "predictor/channel dimension mismatch")
    require(all(math.isfinite(x) for x in predictor["channel_weights"]),
            "non-finite predictor weight")
    require(math.isfinite(predictor["training_target_mean"]),
            "invalid training target mean")
    require(bool(predictor["target_units"]), "target units missing")

    protocol = data["protocol"]
    require(protocol["k"] == 3.0 and protocol["p_max"] == 0.3
            and protocol["alpha"] == 0.4, "MuViS-C severity constants changed")
    require(protocol["single_channel_corruption"] is True
            and protocol["uncorrupted_channels_remain_clean"] is True,
            "atomic single-channel corruption protocol changed")
    require(protocol["bootstrap_replicates"] == 200,
            "v0.1 requires 200 fixed bootstrap resamples")
    require(protocol["bootstrap_index_seed"] == 260916
            and protocol["bootstrap_index_method"]
            == "LCG32_1664525_1013904223_mod_sample_count",
            "bootstrap index provenance changed")
    require(protocol["noise_draw_seed"] == 260917
            and protocol["noise_draw_method"] == "LCG32_BOX_MULLER_PRECOMPUTED",
            "noise draw provenance changed")
    require(protocol["outlier_uniform_seed"] == 260918
            and protocol["outlier_delta_seed"] == 260919
            and protocol["uniform_draw_method"] == "LCG32_PRECOMPUTED_UNIFORM_32",
            "outlier draw provenance changed")
    severities = protocol["severity_grid"]
    require(len(severities) == 20, "v0.1 requires 20 severity steps")
    for index, value in enumerate(severities):
        require(math.isclose(value, (index + 1) * 0.05, abs_tol=1e-12),
                "severity grid must be 0.05, 0.10, ..., 1.00")
    definitions = protocol["metric_definitions"]
    require(definitions["mpc"]
            == "mean bootstrapped RMSE over failure_mode x channel x severity",
            "mPC definition changed")
    require(definitions["rpc"]
            == "mPC / clean_RMSE; 1 means no average degradation",
            "rPC definition changed")
    require(definitions["s_cross"]
            == "minimum severity where any failure/channel bootstrapped RMSE >= naive mean predictor RMSE; null if no crossing",
            "s_cross definition changed")
    require(definitions["npc"] == "mPC / naive_RMSE",
            "nPC definition changed")

    windows = data["windows"]
    require(len(windows) == n, "windows sample dimension mismatch")
    for sample in windows:
        require(len(sample) == t, "windows time dimension mismatch")
        for step in sample:
            require(len(step) == c and all(math.isfinite(x) for x in step),
                    "windows channel dimension/value mismatch")
    raw_windows = data["raw_windows"]
    require(len(raw_windows) == n, "raw_windows sample dimension mismatch")
    for sample in raw_windows:
        require(len(sample) == t, "raw_windows time dimension mismatch")
        for step in sample:
            require(len(step) == c and all(math.isfinite(x) for x in step),
                    "raw_windows channel dimension/value mismatch")
    stats = data["normalization_statistics"]
    require(stats["method"] == "population_zscore_over_frozen_synthetic_window_corpus",
            "normalization method changed")
    require(len(stats["mean"]) == c and len(stats["std"]) == c,
            "normalization statistics dimension mismatch")
    require(all(math.isfinite(x) for x in stats["mean"])
            and all(math.isfinite(x) and x > 0.0 for x in stats["std"]),
            "normalization statistics invalid")
    for sample in range(n):
        for time in range(t):
            for channel in range(c):
                normalized = (
                    raw_windows[sample][time][channel] - stats["mean"][channel]
                ) / stats["std"][channel]
                require(math.isclose(normalized, windows[sample][time][channel],
                                     rel_tol=0.0, abs_tol=1.0e-10),
                        "standardized window differs from frozen raw value/statistics")
    for name in ("noise_standard_normal", "outlier_uniform", "outlier_delta_unit"):
        cube = data[name]
        require(len(cube) == n, f"{name}: sample dimension mismatch")
        for sample in cube:
            require(len(sample) == t, f"{name}: time dimension mismatch")
            for step in sample:
                require(len(step) == c and all(math.isfinite(x) for x in step),
                        f"{name}: channel dimension/value mismatch")
    require(all(0.0 <= x <= 1.0 for sample in data["outlier_uniform"]
                for step in sample for x in step),
            "outlier uniforms must lie in [0,1]")
    require(all(-1.0 <= x <= 1.0 for sample in data["outlier_delta_unit"]
                for step in sample for x in step),
            "outlier delta draws must lie in [-1,1]")
    require(len(data["target"]) == n
            and all(math.isfinite(x) for x in data["target"]),
            "target vector invalid")
    require(len(data["target_noise"]) == n
            and all(math.isfinite(x) for x in data["target_noise"]),
            "target noise vector invalid")
    definitions = protocol["metric_definitions"]
    require(definitions["mpc"] == "mean bootstrapped RMSE over failure_mode x channel x severity",
            "mPC definition changed")
    require(definitions["rpc"] == "mPC / clean_RMSE; 1 means no average degradation",
            "rPC definition changed")
    require(definitions["s_cross"] == "minimum severity where any failure/channel bootstrapped RMSE >= naive mean predictor RMSE; null if no crossing",
            "s_cross definition changed")
    require(definitions["npc"] == "mPC / naive_RMSE", "nPC definition changed")

    indices = data["bootstrap_resample_indices"]
    require(len(indices) == 200 and all(len(row) == n for row in indices),
            "bootstrap matrix must be 200xN")
    require(all(isinstance(index, int) and 0 <= index < n
                for row in indices for index in row),
            "bootstrap index outside sample range")
    # Independently bind targets to the fixture's declared synthetic response equation.
    for sample in range(n):
        means = [
            sum(step[channel] for step in data["windows"][sample]) / t
            for channel in range(c)
        ]
        expected_target = (
            0.50 * means[0] - 0.27 * means[1] + 0.16 * means[2]
            + data["target_noise"][sample]
        )
        require(math.isclose(data["target"][sample], expected_target,
                             rel_tol=0.0, abs_tol=1.0e-10),
                "target vector does not match synthetic reference equation")


def apply_failure(mode: str, severity: float, value: float, noise_draw: float,
                  outlier_uniform: float, outlier_delta_unit: float,
                  time_index: int, time_steps: int, protocol: dict[str, Any]) -> float:
    s, k = severity, protocol["k"]
    fraction = time_index / (time_steps - 1)
    if mode == "bias":
        result = value + s * k
    elif mode == "noise":
        result = value + noise_draw * s * k
    elif mode == "scaling":
        result = value * (1.0 + s * (k - 1.0))
    elif mode == "time_varying_scaling":
        result = value * (1.0 + s * (k - 1.0) * fraction)
    elif mode == "linear_drift":
        result = value + s * k * fraction
    elif mode == "nonlinear_drift":
        result = value + s * k * fraction ** 2
    elif mode == "outliers":
        result = (value + outlier_delta_unit * k
                  if outlier_uniform < s * protocol["p_max"] else value)
    elif mode == "trimming_variable":
        h = (1.0 - s) * k
        if value < -h:
            result = -h + protocol["alpha"] * (value + h)
        elif value > h:
            result = h + protocol["alpha"] * (value - h)
        else:
            result = value
    elif mode == "trimming_constant":
        h = (1.0 - s) * k
        result = max(-h, min(h, value))
    elif mode == "hard_fault":
        result = s * k
    else:
        raise AuditError(f"unknown failure mode: {mode}")
    require(math.isfinite(result), "corruption produced non-finite value")
    return result


def predict(windows: list[list[list[float]]], weights: list[float]) -> list[float]:
    predictions = []
    for sample in windows:
        time_steps = len(sample)
        prediction = 0.0
        for channel, weight in enumerate(weights):
            mean = sum(step[channel] for step in sample) / time_steps
            prediction += weight * mean
        predictions.append(prediction)
    return predictions


def bootstrap_mean_rmse(predictions: list[float], target: list[float],
                        resamples: list[list[int]]) -> float:
    values = []
    for indices in resamples:
        mse = sum((predictions[index] - target[index]) ** 2 for index in indices) / len(indices)
        values.append(math.sqrt(mse))
    return sum(values) / len(values)


def evaluate(data: dict[str, Any], raw: bytes) -> dict[str, Any]:
    validate(data)
    ds, predictor, protocol = data["dataset"], data["predictor"], data["protocol"]
    n, t, c = ds["sample_count"], ds["time_steps"], len(ds["channel_ids"])
    clean = predict(data["windows"], predictor["channel_weights"])
    naive = [predictor["training_target_mean"]] * n
    clean_rmse = bootstrap_mean_rmse(
        clean, data["target"], data["bootstrap_resample_indices"]
    )
    naive_rmse = bootstrap_mean_rmse(
        naive, data["target"], data["bootstrap_resample_indices"]
    )
    require(clean_rmse > 0.0 and naive_rmse > 0.0,
            "clean and naive RMSE must be positive")

    grid = []
    for mode in FAILURE_MODES:
        for channel in range(c):
            for severity in protocol["severity_grid"]:
                corrupted = copy.deepcopy(data["windows"])
                for sample in range(n):
                    for time in range(t):
                        corrupted[sample][time][channel] = apply_failure(
                            mode, severity,
                            data["windows"][sample][time][channel],
                            data["noise_standard_normal"][sample][time][channel],
                            data["outlier_uniform"][sample][time][channel],
                            data["outlier_delta_unit"][sample][time][channel],
                            time, t, protocol,
                        )
                predictions = predict(corrupted, predictor["channel_weights"])
                rmse = bootstrap_mean_rmse(
                    predictions, data["target"], data["bootstrap_resample_indices"]
                )
                grid.append({
                    "failure_mode": mode,
                    "channel_id": ds["channel_ids"][channel],
                    "severity": severity,
                    "rmse": rmse,
                })

    mpc = sum(record["rmse"] for record in grid) / len(grid)
    rpc = mpc / clean_rmse
    npc = mpc / naive_rmse
    crossing = [record["severity"] for record in grid
                if record["rmse"] >= naive_rmse]
    worst = max(grid, key=lambda record: record["rmse"])
    return {
        "schema_version": "aero-muvis-c-protocol-reference-result/0.1.0",
        "problem_id": data["problem_id"],
        "source_sha256": hashlib.sha256(raw).hexdigest(),
        "source_bytes": len(raw),
        "claim_ceiling": CLAIM,
        "sample_count": n,
        "time_steps": t,
        "channel_count": c,
        "failure_mode_count": len(FAILURE_MODES),
        "severity_count": len(protocol["severity_grid"]),
        "bootstrap_replicates": protocol["bootstrap_replicates"],
        "clean_rmse": clean_rmse,
        "naive_rmse": naive_rmse,
        "mpc": mpc,
        "rpc": rpc,
        "npc": npc,
        "s_cross": min(crossing) if crossing else None,
        "worst_case": worst,
        "grid_result_count": len(grid),
        "grid": grid,
    }


def compare(actual: Any, expected: Any, path: str = "root") -> None:
    if isinstance(expected, float):
        require(
            isinstance(actual, (int, float))
            and math.isclose(float(actual), expected, rel_tol=1e-10, abs_tol=1e-12),
            f"{path}: {actual!r} != {expected!r}",
        )
    elif isinstance(expected, dict):
        require(isinstance(actual, dict) and set(actual) == set(expected),
                f"{path}: object keys differ")
        for key in expected:
            compare(actual[key], expected[key], f"{path}.{key}")
    elif isinstance(expected, list):
        require(isinstance(actual, list) and len(actual) == len(expected),
                f"{path}: list length differs")
        for index, (left, right) in enumerate(zip(actual, expected)):
            compare(left, right, f"{path}[{index}]")
    else:
        require(actual == expected, f"{path}: {actual!r} != {expected!r}")


def hostile_controls(data: dict[str, Any]) -> None:
    bad = copy.deepcopy(data)
    bad["protocol"]["severity_grid"][0] = 0.0
    try:
        validate(bad)
    except AuditError:
        print("negative_control=pass case=severity_grid_mutation")
    else:
        raise AuditError("mutated severity grid was accepted")

    bad = copy.deepcopy(data)
    bad["bootstrap_resample_indices"][0][0] = len(data["windows"])
    try:
        validate(bad)
    except AuditError:
        print("negative_control=pass case=bootstrap_index_out_of_range")
    else:
        raise AuditError("out-of-range bootstrap index was accepted")

    bad = copy.deepcopy(data)
    bad["outlier_uniform"][0][0][0] = 1.5
    try:
        validate(bad)
    except AuditError:
        print("negative_control=pass case=outlier_draw_out_of_range")
    else:
        raise AuditError("invalid outlier draw was accepted")

    # Check that the atomic one-channel protocol never mutates unrelated channels.
    sample = 0
    channel = 1
    corrupted = copy.deepcopy(data["windows"])
    for time in range(data["dataset"]["time_steps"]):
        corrupted[sample][time][channel] = apply_failure(
            "bias", 0.5, data["windows"][sample][time][channel],
            0.0, 0.5, 0.0, time, data["dataset"]["time_steps"], data["protocol"]
        )
    for time in range(data["dataset"]["time_steps"]):
        for other in range(data["dataset"]["channel_ids"].__len__()):
            if other != channel:
                require(
                    corrupted[sample][time][other] == data["windows"][sample][time][other],
                    "single-channel intervention changed another channel",
                )
    print("negative_control=pass case=single_channel_only_intervention")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--rust-output", type=Path, required=True)
    args = parser.parse_args()
    try:
        raw = FIXTURE.read_bytes()
        data = json.loads(raw)
        expected = evaluate(data, raw)
        require(expected["grid_result_count"] == 600,
                "MuViS-C grid must contain 10x3x20=600 entries")
        require(expected["bootstrap_replicates"] == 200,
                "bootstrap replicate count changed")
        frozen = data["expected_metrics"]
        for key in ("clean_rmse", "naive_rmse", "mpc", "rpc", "npc"):
            require(math.isclose(expected[key], frozen[key],
                                 rel_tol=1.0e-9, abs_tol=1.0e-12),
                    f"frozen MuViS-C metric changed: {key}")
        require(expected["s_cross"] == frozen["s_cross"],
                "frozen s_cross changed")
        require(expected["worst_case"]["failure_mode"]
                == frozen["worst_case"]["failure_mode"]
                and expected["worst_case"]["channel_id"] == frozen["worst_case"]["channel_id"]
                and expected["worst_case"]["severity"] == frozen["worst_case"]["severity"],
                "frozen worst-case identity changed")
        require(math.isclose(expected["worst_case"]["rmse"],
                             frozen["worst_case"]["rmse"],
                             rel_tol=1.0e-9, abs_tol=1.0e-12),
                "frozen worst-case RMSE changed")
        actual = json.loads(args.rust_output.read_text(encoding="utf-8"))
        compare(actual, expected)
        hostile_controls(data)
        print(
            "aero_muvis_c_protocol=pass modes=10 channels=3 severities=20 "
            "grid=600 bootstrap=200 rust_python=pass "
            f"clean_rmse={expected['clean_rmse']:.9f} "
            f"mpc={expected['mpc']:.9f} rpc={expected['rpc']:.9f} "
            f"s_cross={expected['s_cross']}"
        )
        return 0
    except (OSError, json.JSONDecodeError, AuditError, KeyError, TypeError, ValueError) as error:
        print(f"muvis_c_protocol_error={error}")
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
