#!/usr/bin/env python3
"""Independent standard-library challenger for pose-observability-bench."""
from __future__ import annotations

import argparse
import json
import math
import sys
from pathlib import Path
from typing import Any

SCHEMA_VERSION = 1
TWIST_ORDER = "left_camera_frame_scaled_xyz_then_rx_ry_rz"


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def finite(value: Any, name: str) -> float:
    require(isinstance(value, (int, float)) and not isinstance(value, bool), f"{name} must be numeric")
    result = float(value)
    require(math.isfinite(result), f"{name} must be finite")
    return result


def sha256_shape(value: Any, name: str) -> str:
    require(isinstance(value, str) and len(value) == 64
            and all(ch in "0123456789abcdef" for ch in value),
            f"{name} must be lowercase SHA-256")
    return value


def validate(request: dict[str, Any]) -> None:
    allowed = {
        "schema_version", "episode_id", "frame_id", "sensor_id",
        "coordinate_frame", "transform_direction", "twist_order",
        "camera_intrinsics_sha256", "target_geometry_sha256",
        "correspondence_ledger_sha256", "translation_scale_m",
        "rank_relative_tolerance", "rank_absolute_tolerance",
        "camera", "observations",
    }
    require(set(request) == allowed, "request has missing or unknown fields")
    require(request["schema_version"] == SCHEMA_VERSION, "unsupported schema version")
    for key in ("episode_id", "frame_id", "sensor_id", "coordinate_frame", "transform_direction"):
        require(isinstance(request[key], str) and request[key].strip(), f"{key} must not be empty")
    require(request["twist_order"] == TWIST_ORDER, "unsupported twist convention")
    for key in ("camera_intrinsics_sha256", "target_geometry_sha256", "correspondence_ledger_sha256"):
        sha256_shape(request[key], key)

    camera = request["camera"]
    require(set(camera) == {"width", "height", "fx_px", "fy_px", "cx_px", "cy_px"},
            "camera fields differ from schema")
    require(type(camera["width"]) is int and camera["width"] > 0, "camera width must be positive")
    require(type(camera["height"]) is int and camera["height"] > 0, "camera height must be positive")
    for key in ("fx_px", "fy_px", "cx_px", "cy_px"):
        camera[key] = finite(camera[key], f"camera.{key}")
    require(camera["fx_px"] > 0 and camera["fy_px"] > 0, "focal lengths must be positive")

    scale = finite(request["translation_scale_m"], "translation_scale_m")
    rel = finite(request["rank_relative_tolerance"], "rank_relative_tolerance")
    absolute = finite(request["rank_absolute_tolerance"], "rank_absolute_tolerance")
    require(scale > 0, "translation scale must be positive")
    require(0 < rel <= 1 and absolute > 0, "rank tolerances are invalid")

    observations = request["observations"]
    require(isinstance(observations, list) and 3 <= len(observations) <= 100_000,
            "observation count must be in 3..=100000")
    ids: set[str] = set()
    effective = 0
    for index, row in enumerate(observations):
        require(set(row) == {"landmark_id", "point_camera_m", "measured_pixel_uv", "sigma_px", "weight"},
                f"observation {index} has missing or unknown fields")
        ident = row["landmark_id"]
        require(isinstance(ident, str) and ident.strip(), f"observation {index} landmark_id is empty")
        require(ident not in ids, f"duplicate landmark_id {ident}")
        ids.add(ident)
        point = row["point_camera_m"]
        pixel = row["measured_pixel_uv"]
        require(isinstance(point, list) and len(point) == 3, f"observation {index} point must be vec3")
        require(isinstance(pixel, list) and len(pixel) == 2, f"observation {index} pixel must be vec2")
        row["point_camera_m"] = [finite(v, f"observation {index} point") for v in point]
        row["measured_pixel_uv"] = [finite(v, f"observation {index} pixel") for v in pixel]
        require(row["point_camera_m"][2] > 1e-9, f"observation {index} has invalid depth")
        row["sigma_px"] = finite(row["sigma_px"], f"observation {index} sigma")
        row["weight"] = finite(row["weight"], f"observation {index} weight")
        require(row["sigma_px"] > 0, f"observation {index} sigma must be positive")
        require(0 <= row["weight"] <= 1, f"observation {index} weight must be in [0, 1]")
        effective += int(row["weight"] > 0)
    require(effective >= 3, "at least three positive-weight observations are required")


def zero_matrix(rows: int, cols: int) -> list[list[float]]:
    return [[0.0 for _ in range(cols)] for _ in range(rows)]


def matmul_2x3_3x6(left: list[list[float]], right: list[list[float]]) -> list[list[float]]:
    result = zero_matrix(2, 6)
    for row in range(2):
        for col in range(6):
            for inner in range(3):
                result[row][col] += left[row][inner] * right[inner][col]
    return result


def jacobi_eigen_6(matrix: list[list[float]]) -> tuple[list[float], list[list[float]]]:
    a = [row[:] for row in matrix]
    vectors = zero_matrix(6, 6)
    for index in range(6):
        vectors[index][index] = 1.0

    for _ in range(6 * 6 * 128):
        p, q = 0, 1
        largest = 0.0
        diagonal_scale = 0.0
        for row in range(6):
            diagonal_scale = max(diagonal_scale, abs(a[row][row]))
            for col in range(row + 1, 6):
                if abs(a[row][col]) > largest:
                    largest, p, q = abs(a[row][col]), row, col
        tolerance = max(diagonal_scale * 1e-14, 1e-14)
        if largest <= tolerance:
            pairs = []
            for index in range(6):
                value = a[index][index]
                require(math.isfinite(value), "FIM eigensolver produced non-finite eigenvalue")
                require(value >= -tolerance, "FIM is not positive semidefinite within tolerance")
                pairs.append((max(0.0, value), [vectors[row][index] for row in range(6)]))
            pairs.sort(key=lambda pair: pair[0])
            values = [pair[0] for pair in pairs]
            sorted_vectors = [[pairs[col][1][row] for col in range(6)] for row in range(6)]
            return values, sorted_vectors

        app, aqq, apq = a[p][p], a[q][q], a[p][q]
        tau = (aqq - app) / (2.0 * apq)
        if tau >= 0:
            t = 1.0 / (tau + math.sqrt(1.0 + tau * tau))
        else:
            t = -1.0 / (-tau + math.sqrt(1.0 + tau * tau))
        cosine = 1.0 / math.sqrt(1.0 + t * t)
        sine = t * cosine

        for index in range(6):
            if index != p and index != q:
                aip, aiq = a[index][p], a[index][q]
                new_p = cosine * aip - sine * aiq
                new_q = sine * aip + cosine * aiq
                a[index][p] = new_p
                a[p][index] = new_p
                a[index][q] = new_q
                a[q][index] = new_q
        a[p][p] = app - t * apq
        a[q][q] = aqq + t * apq
        a[p][q] = a[q][p] = 0.0
        for row in range(6):
            vip, viq = vectors[row][p], vectors[row][q]
            vectors[row][p] = cosine * vip - sine * viq
            vectors[row][q] = sine * vip + cosine * viq
    raise ValueError("FIM Jacobi eigensolver did not converge")


def evaluate(request: dict[str, Any]) -> dict[str, Any]:
    validate(request)
    camera = request["camera"]
    length = request["translation_scale_m"]
    fim = zero_matrix(6, 6)
    residuals: list[float] = []
    effective = 0

    for observation in request["observations"]:
        x, y, z = observation["point_camera_m"]
        u = camera["fx_px"] * x / z + camera["cx_px"]
        v = camera["fy_px"] * y / z + camera["cy_px"]
        du = u - observation["measured_pixel_uv"][0]
        dv = v - observation["measured_pixel_uv"][1]
        residuals.append(math.sqrt(du * du + dv * dv))

        weight = observation["weight"]
        if weight == 0:
            continue
        point_j = [
            [length, 0.0, 0.0, 0.0, z, -y],
            [0.0, length, 0.0, -z, 0.0, x],
            [0.0, 0.0, length, y, -x, 0.0],
        ]
        projection_j = [
            [camera["fx_px"] / z, 0.0, -camera["fx_px"] * x / (z * z)],
            [0.0, camera["fy_px"] / z, -camera["fy_px"] * y / (z * z)],
        ]
        jacobian = matmul_2x3_3x6(projection_j, point_j)
        information_weight = weight / (observation["sigma_px"] ** 2)
        for row in range(6):
            for col in range(6):
                fim[row][col] += information_weight * (
                    jacobian[0][row] * jacobian[0][col]
                    + jacobian[1][row] * jacobian[1][col]
                )
        effective += 1

    require(all(math.isfinite(value) for row in fim for value in row),
            "FIM accumulation produced non-finite values")
    eigenvalues, eigenvectors = jacobi_eigen_6(fim)
    strongest, weakest = eigenvalues[-1], eigenvalues[0]
    threshold = max(request["rank_absolute_tolerance"],
                    request["rank_relative_tolerance"] * strongest)
    rank = sum(value > threshold for value in eigenvalues)
    condition = strongest / weakest if rank == 6 and weakest > threshold else None
    logdet = sum(math.log(value) for value in eigenvalues) if rank == 6 else None
    trace = sum(eigenvalues)
    square_sum = sum(value * value for value in eigenvalues)
    participation = trace * trace / square_sum if square_sum > 0 else None
    weak_mode = [eigenvectors[row][0] for row in range(6)]
    trans_norm = math.sqrt(sum(value * value for value in weak_mode[:3]))
    rot_norm = math.sqrt(sum(value * value for value in weak_mode[3:]))
    sorted_residuals = sorted(residuals)
    pos = 0.95 * (len(sorted_residuals) - 1)
    lo, hi = math.floor(pos), math.ceil(pos)
    p95 = sorted_residuals[lo] * (1.0 - (pos - lo)) + sorted_residuals[hi] * (pos - lo)

    return {
        "schema_version": 1,
        "episode_id": request["episode_id"],
        "frame_id": request["frame_id"],
        "sensor_id": request["sensor_id"],
        "coordinate_frame": request["coordinate_frame"],
        "transform_direction": request["transform_direction"],
        "twist_order": request["twist_order"],
        "observation_count": len(request["observations"]),
        "effective_observation_count": effective,
        "translation_scale_m": length,
        "fim_6x6": fim,
        "eigenvalues_ascending": eigenvalues,
        "numerical_rank": rank,
        "rank_threshold": threshold,
        "weakest_eigenvalue": weakest,
        "strongest_eigenvalue": strongest,
        "condition_number": condition,
        "log_determinant": logdet,
        "participation_ratio": participation,
        "weakest_mode": weak_mode,
        "weakest_mode_translation_norm": trans_norm,
        "weakest_mode_rotation_norm": rot_norm,
        "mean_reprojection_error_px": sum(residuals) / len(residuals),
        "p95_reprojection_error_px": p95,
        "max_reprojection_error_px": max(residuals),
        "physical_truth_claim": False,
        "pose_probability_claim": False,
        "correspondence_authentication_claim": False,
    }


def close(a: Any, b: Any, tolerance: float = 2e-8) -> bool:
    if a is None or b is None:
        return a is None and b is None
    if isinstance(a, bool) or isinstance(b, bool):
        return type(a) is type(b) and a == b
    if isinstance(a, (int, float)) and isinstance(b, (int, float)):
        return math.isclose(float(a), float(b), rel_tol=tolerance, abs_tol=tolerance)
    if isinstance(a, list) and isinstance(b, list):
        return len(a) == len(b) and all(close(x, y, tolerance) for x, y in zip(a, b, strict=True))
    return a == b


def compare(rust: dict[str, Any], python: dict[str, Any]) -> list[str]:
    failures: list[str] = []
    for key, expected in python.items():
        actual = rust.get(key, object())
        if key == "weakest_mode":
            dot = sum(a * b for a, b in zip(actual, expected, strict=True)) if isinstance(actual, list) else 0.0
            if abs(abs(dot) - 1.0) > 2e-6:
                failures.append("weakest_mode differs beyond sign ambiguity")
        elif not close(actual, expected):
            failures.append(f"{key} mismatch")
    return failures


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--input", required=True)
    parser.add_argument("--compare", help="Rust JSON output to compare against")
    args = parser.parse_args()
    request = json.loads(Path(args.input).read_text(encoding="utf-8"))
    result = evaluate(request)
    if args.compare:
        rust_result = json.loads(Path(args.compare).read_text(encoding="utf-8"))
        failures = compare(rust_result, result)
        if failures:
            for failure in failures:
                print(f"parity failure: {failure}", file=sys.stderr)
            return 2
        print("independent Python/Rust parity: PASS")
    else:
        json.dump(result, sys.stdout, indent=2, sort_keys=True)
        print()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
