#!/usr/bin/env python3
"""Independent stdlib-only oracle for the Rust section-polar audit.

This is CI/test tooling only. Production calculations remain in the Rust crate.
The oracle parses the pinned raw source independently and recomputes all metrics.
"""
from __future__ import annotations

import csv
import hashlib
import math
import sys
from pathlib import Path

EXPECTED_SHA256 = "78cd2f6aa4968e80f44cbf6c96f699bd9c6e45681d958ec528a10cf72ed23357"
EXPECTED_BYTES = 1343
SOURCE_REVISION = "20a39f549dbff988a6accc421ac84dafc4487695"
MODEL_VERSION = "section-polar-domain-audit/0.1.0"
CALIBRATION_LIMIT = 8.5
INTERMEDIATE_LIMIT = 16.5
ATOL = 2.0e-12


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def parse_source(path: Path) -> tuple[bytes, dict[str, list[dict[str, float | int | str]]]]:
    raw = path.read_bytes()
    digest = hashlib.sha256(raw).hexdigest()
    require(digest == EXPECTED_SHA256, f"source SHA-256 mismatch: {digest}")
    require(len(raw) == EXPECTED_BYTES, f"source byte length mismatch: {len(raw)}")
    text = raw.decode("utf-8")
    require("Data from Ladson, NASA TM 4074, 1988" in text, "Ladson source header missing")
    require("Re=6 million, with transition tripped" in text, "Re/transition header missing")
    require("M=0.15" in text, "Mach header missing")
    require('variables="alpha, deg","cl","cd"' in text, "force variables header missing")

    zones: dict[str, list[dict[str, float | int | str]]] = {}
    zone: str | None = None
    for line_number, raw_line in enumerate(text.splitlines(), start=1):
        line = raw_line.strip()
        if not line or line.startswith("#") or line.startswith("variables="):
            continue
        if line.startswith("zone,"):
            marker = 't="'
            start = line.find(marker)
            require(start >= 0, f"line {line_number}: malformed zone")
            start += len(marker)
            end = line.find('"', start)
            require(end > start, f"line {line_number}: malformed zone label")
            zone = line[start:end]
            require(zone not in zones, f"duplicate zone block: {zone}")
            zones[zone] = []
            continue
        require(zone is not None, f"line {line_number}: data before zone")
        fields = line.split()
        require(len(fields) == 3, f"line {line_number}: expected 3 numeric fields")
        alpha, cl, cd = (float(value) for value in fields)
        require(all(math.isfinite(value) for value in (alpha, cl, cd)), f"line {line_number}: nonfinite")
        require(cd >= 0.0, f"line {line_number}: negative drag")
        zones[zone].append(
            {"alpha": alpha, "cl": cl, "cd": cd, "line": line_number, "zone": zone}
        )
    require(set(zones) == {"80 grit", "120 grit", "180 grit"}, f"unexpected zones: {sorted(zones)}")
    require(sum(len(rows) for rows in zones.values()) == 53, "expected 53 force observations")
    return raw, zones


def fit(rows: list[dict[str, float | int | str]]) -> tuple[float, float]:
    require(len(rows) >= 3, "at least three calibration rows are required")
    xs = [float(row["cl"]) ** 2 for row in rows]
    ys = [float(row["cd"]) for row in rows]
    mean_x = sum(xs) / len(xs)
    mean_y = sum(ys) / len(ys)
    variance = sum((value - mean_x) ** 2 for value in xs)
    require(math.isfinite(variance) and variance > 1.0e-18, "singular calibration fit")
    slope = sum((x - mean_x) * (y - mean_y) for x, y in zip(xs, ys, strict=True)) / variance
    intercept = mean_y - slope * mean_x
    require(math.isfinite(slope) and math.isfinite(intercept), "non-finite fitted coefficients")
    return intercept, slope


def metrics(rows: list[dict[str, float | int | str]], coefficients: tuple[float, float]) -> dict[str, float | int]:
    intercept, slope = coefficients
    errors = [
        float(row["cd"]) - (intercept + slope * float(row["cl"]) ** 2)
        for row in rows
    ]
    require(bool(errors), "empty report partition")
    return {
        "n": len(errors),
        "mae": sum(abs(error) for error in errors) / len(errors),
        "rmse": math.sqrt(sum(error * error for error in errors) / len(errors)),
        "max_abs": max(abs(error) for error in errors),
        "mean_signed": sum(errors) / len(errors),
    }


def close(actual: str, expected: float, label: str) -> None:
    value = float(actual)
    require(math.isfinite(value), f"{label}: non-finite reported value")
    require(abs(value - expected) <= ATOL, f"{label}: {value:.15g} != oracle {expected:.15g}")


def main() -> int:
    if len(sys.argv) != 3:
        raise ValueError("usage: verify-section-polar-independent.py RAW_LADSON.dat RUST_REPORT.csv")
    source_path = Path(sys.argv[1])
    report_path = Path(sys.argv[2])
    _raw, zones = parse_source(source_path)
    with report_path.open(newline="", encoding="utf-8") as stream:
        reports = list(csv.DictReader(stream))
    require(len(reports) == 3, f"expected 3 Rust report rows, got {len(reports)}")

    expected_counts = {
        "80 grit": (17, 7, 7, 3),
        "120 grit": (18, 8, 7, 3),
        "180 grit": (18, 8, 7, 3),
    }
    require({row["zone"] for row in reports} == set(zones), "Rust report zone set mismatch")
    by_zone = {row["zone"]: row for row in reports}
    for zone, rows in zones.items():
        calibration = [row for row in rows if abs(float(row["alpha"])) <= CALIBRATION_LIMIT]
        intermediate = [
            row for row in rows
            if CALIBRATION_LIMIT < abs(float(row["alpha"])) <= INTERMEDIATE_LIMIT
        ]
        high_angle = [row for row in rows if abs(float(row["alpha"])) > INTERMEDIATE_LIMIT]
        counts = (len(rows), len(calibration), len(intermediate), len(high_angle))
        require(counts == expected_counts[zone], f"{zone}: split counts mismatch: {counts}")

        coefficients = fit(calibration)
        partitions = {
            "calibration": metrics(calibration, coefficients),
            "intermediate": metrics(intermediate, coefficients),
            "high_angle": metrics(high_angle, coefficients),
        }
        record = by_zone[zone]
        require(record["model_version"] == MODEL_VERSION, f"{zone}: model version mismatch")
        require(record["dataset_id"] == "NASA-TMR-NACA0012-LADSON-FORCES", f"{zone}: dataset mismatch")
        require(record["declared_source_sha256"] == EXPECTED_SHA256, f"{zone}: source digest mismatch")
        require(record["source_revision"] == SOURCE_REVISION, f"{zone}: source revision mismatch")
        require(record["evidence_level"] == "reference_data_diagnostic_not_3d_validation", f"{zone}: evidence label mismatch")
        require(
            record["source_condition"]
            == "Ladson NASA TM 4074 (1988); Re=6 million; M=0.15; transition tripped",
            f"{zone}: source condition mismatch",
        )
        require(record["calibration_split"] == "|alpha_deg| <= 8.5", f"{zone}: calibration split mismatch")
        require(
            record["intermediate_split"] == "8.5 < |alpha_deg| <= 16.5",
            f"{zone}: intermediate split mismatch",
        )
        require(record["high_angle_split"] == "|alpha_deg| > 16.5", f"{zone}: high-angle split mismatch")
        require(
            record["min_source_line"] == str(min(int(row["line"]) for row in rows)),
            f"{zone}: minimum source line mismatch",
        )
        require(
            record["max_source_line"] == str(max(int(row["line"]) for row in rows)),
            f"{zone}: maximum source line mismatch",
        )
        require(record["observation_count"] == str(counts[0]), f"{zone}: observation count mismatch")
        require(record["calibration_n"] == str(counts[1]), f"{zone}: calibration count mismatch")
        require(record["intermediate_n"] == str(counts[2]), f"{zone}: intermediate count mismatch")
        require(record["high_angle_n"] == str(counts[3]), f"{zone}: high-angle count mismatch")
        close(record["fit_cd0_section"], coefficients[0], f"{zone} Cd0")
        close(record["fit_cd2_section_coefficient"], coefficients[1], f"{zone} section coefficient")
        for partition, prefix in (
            ("calibration", "calibration"),
            ("intermediate", "intermediate"),
            ("high_angle", "high_angle"),
        ):
            result = partitions[partition]
            close(record[f"{prefix}_mae"], float(result["mae"]), f"{zone} {partition} MAE")
            close(record[f"{prefix}_rmse"], float(result["rmse"]), f"{zone} {partition} RMSE")
            close(record[f"{prefix}_max_abs_error"], float(result["max_abs"]), f"{zone} {partition} max")
            close(record[f"{prefix}_mean_signed_error"], float(result["mean_signed"]), f"{zone} {partition} bias")

        # Negative control: modifying only held-out observations must not alter calibration fit.
        modified = [dict(row) for row in rows]
        for row in modified:
            if abs(float(row["alpha"])) > CALIBRATION_LIMIT:
                row["cd"] = float(row["cd"]) + 0.123
        modified_calibration = [
            row for row in modified if abs(float(row["alpha"])) <= CALIBRATION_LIMIT
        ]
        require(fit(modified_calibration) == coefficients, f"{zone}: held-out leakage detected")

    print(
        "independent_section_polar_oracle=pass "
        f"source_sha256={EXPECTED_SHA256} observations=53 zones=3 "
        "metrics=all_fields_match atol=2e-12 heldout_leakage_negative_control=pass"
    )
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except Exception as error:  # CI should fail closed with one useful diagnostic.
        print(f"independent section-polar oracle failed: {error}", file=sys.stderr)
        raise SystemExit(1)
