#!/usr/bin/env python3
"""Independent stdlib-only parser/reconciliation oracle for DPW-8 Force/Moment V5."""
from __future__ import annotations

import csv
import math
import re
import sys
from pathlib import Path

COLUMNS = [
    "GRID_LEVEL", "GRID_SIZE", "GRID_FAC", "MACH", "REY", "ALPHA",
    "CL_TOT", "CD_TOT", "CM_TOT", "CL_WING", "CD_WING", "CM_WING",
    "CD_PR", "CD_SF", "CL_TAIL", "CD_TAIL", "CM_TAIL", "CL_FUS",
    "CD_FUS", "CM_FUS", "CL_NAC", "CD_NAC", "CM_NAC", "CL_PY",
    "CD_PY", "CM_PY", "CPU_Hours", "DELTAT", "CTUSTART", "CTUAVG", "Q/E",
]
DUPLICATE_TOLERANCE = 1.0e-7
LARGE_CL_JUMP = 0.05
LARGE_CD_JUMP = 0.005


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def assignment(line: str, prefix: str, line_number: int) -> tuple[str, str]:
    remainder = line[len(prefix):]
    require("=" in remainder, f"line {line_number}: missing assignment")
    key, raw_value = remainder.split("=", 1)
    value = raw_value.strip()
    require(value.startswith('"') and value.endswith('"'), f"line {line_number}: unquoted value")
    return key.strip(), value[1:-1]


def zone_kind(title: str, line_number: int) -> tuple[str, float | int]:
    match = re.search(r"GRID LEVEL\s+(\d+)$", title, re.IGNORECASE)
    if match:
        return "grid", int(match.group(1))
    match = re.search(r"ALPHA\s+([-+]?(?:\d+(?:\.\d*)?|\.\d+)(?:[eE][-+]?\d+)?)$", title, re.IGNORECASE)
    if match:
        return "alpha", float(match.group(1))
    raise ValueError(f"line {line_number}: unsupported zone title {title}")


def parse(path: Path) -> tuple[dict, list[dict]]:
    lines = path.read_text(encoding="utf-8").splitlines()
    title = None
    variables = None
    dataset_aux: dict[str, str] = {}
    zones: list[dict] = []
    active: dict | None = None

    for line_number, raw in enumerate(lines, start=1):
        line = raw.strip()
        if not line or line.startswith("#") or (
            line.startswith("+") and set(line[1:]) <= {"-", "+"}
        ):
            continue
        if line.startswith("TITLE"):
            require(title is None and not zones, f"line {line_number}: duplicate TITLE")
            _, title = assignment(line, "TITLE", line_number)
        elif line.startswith("VARIABLES"):
            require(variables is None and not zones, f"line {line_number}: duplicate VARIABLES")
            remainder = line[len("VARIABLES"):].strip()
            require(remainder.startswith("="), f"line {line_number}: missing VARIABLES '='")
            variables = re.findall(r'"([^"]+)"', remainder[1:])
            require(bool(variables), f"line {line_number}: no variables")
        elif line.startswith("DATASETAUXDATA"):
            require(active is None, f"line {line_number}: misplaced dataset metadata")
            key, value = assignment(line, "DATASETAUXDATA", line_number)
            require(key not in dataset_aux, f"line {line_number}: duplicate dataset metadata")
            dataset_aux[key] = value
        elif line.startswith("ZONE"):
            require(variables == COLUMNS, f"line {line_number}: unexpected V5 variables")
            key, zone_title = assignment(line, "ZONE", line_number)
            require(key == "T", f"line {line_number}: unsupported zone key")
            require(zone_title not in [zone["title"] for zone in zones], "duplicate zone title")
            view, declaration = zone_kind(zone_title, line_number)
            active = {
                "title": zone_title,
                "view": view,
                "declaration": declaration,
                "header_line": line_number,
                "aux": {},
                "rows": [],
            }
            zones.append(active)
        elif line.startswith("AUXDATA"):
            require(active is not None, f"line {line_number}: AUXDATA without zone")
            key, value = assignment(line, "AUXDATA", line_number)
            require(key not in active["aux"], f"line {line_number}: duplicate zone metadata")
            active["aux"][key] = value
        else:
            require(active is not None, f"line {line_number}: data before zone")
            tokens = line.split()
            require(len(tokens) == len(COLUMNS), f"line {line_number}: wrong column count")
            numeric = [float(token) for token in tokens]
            require(all(math.isfinite(value) for value in numeric), f"line {line_number}: non-finite")
            require(all(value != -999.0 for value in numeric[:9]), f"line {line_number}: required sentinel")
            values = tuple(
                None if index >= 9 and value == -999.0 else value
                for index, value in enumerate(numeric)
            )
            grid_level = int(numeric[0])
            grid_size = int(numeric[1])
            require(numeric[0] == grid_level and numeric[1] == grid_size, "non-integral grid fields")
            require(grid_level > 0 and grid_size > 0 and numeric[2] > 0, "invalid grid fields")
            require(numeric[3] > 0 and numeric[4] > 0, "invalid flow condition")
            alpha = numeric[5]
            if active["view"] == "grid":
                require(grid_level == active["declaration"], f"line {line_number}: zone/grid mismatch")
            else:
                require(abs(alpha - active["declaration"]) <= 1.0e-8, f"line {line_number}: zone/alpha mismatch")
            active["rows"].append({
                "line": line_number,
                "view": active["view"],
                "grid_level": grid_level,
                "grid_size": grid_size,
                "alpha": alpha,
                "values": values,
            })

    require(title is not None, "missing TITLE")
    require(variables == COLUMNS, "missing or invalid VARIABLES")
    require(bool(zones), "no zones")
    require(all(zone["rows"] for zone in zones), "empty zone")
    require({zone["view"] for zone in zones} == {"grid", "alpha"}, "both table views required")
    require(dataset_aux.get("DatasetStatus") == "synthetic_fixture_not_official", "fixture provenance metadata missing")
    require(all(zone["aux"].get("FixtureOnly") == "true" for zone in zones), "zone metadata not preserved")

    by_view: dict[str, dict[tuple[int, float], dict]] = {"grid": {}, "alpha": {}}
    for zone in zones:
        for row in zone["rows"]:
            key = (row["grid_level"], row["alpha"])
            require(key not in by_view[zone["view"]], f"duplicate key in {zone['view']} view: {key}")
            by_view[zone["view"]][key] = row

    require(set(by_view["grid"]) == set(by_view["alpha"]), "grid/alpha views have different key sets")
    canonical = []
    for key in sorted(by_view["grid"]):
        grid_row = by_view["grid"][key]
        alpha_row = by_view["alpha"][key]
        require(grid_row["values"] == alpha_row["values"], f"cross-view mismatch for {key}")
        canonical.append({
            "grid": grid_row,
            "alpha": alpha_row,
            "grid_level": key[0],
            "alpha_deg": key[1],
            "grid_size": grid_row["grid_size"],
            "cl": grid_row["values"][6],
            "cd": grid_row["values"][7],
            "cm": grid_row["values"][8],
            "missing": sum(value is None for value in grid_row["values"][9:]),
        })

    mesh_by_level: dict[int, tuple[int, float]] = {}
    level_by_mesh_size: dict[int, int] = {}
    for cell in canonical:
        row = cell["grid"]
        identity = (cell["grid_size"], row["values"][2])
        previous = mesh_by_level.get(cell["grid_level"])
        require(
            previous is None or previous == identity,
            f"inconsistent mesh identity for grid level {cell['grid_level']}",
        )
        mesh_by_level[cell["grid_level"]] = identity
        other_level = level_by_mesh_size.get(cell["grid_size"])
        require(
            other_level is None or other_level == cell["grid_level"],
            f"GRID_SIZE {cell['grid_size']} assigned to multiple grid levels",
        )
        level_by_mesh_size[cell["grid_size"]] = cell["grid_level"]

    for cell in canonical:
        row = cell["grid"]
        duplicate = any(
            other["grid_level"] == row["grid_level"]
            and abs(other["alpha_deg"] - row["alpha"]) > 0.25
            and abs(other["cl"] - cell["cl"]) <= DUPLICATE_TOLERANCE
            and abs(other["cd"] - cell["cd"]) <= DUPLICATE_TOLERANCE
            for other in canonical
        )
        jump = any(
            other["alpha_deg"] == row["alpha"]
            and abs(other["grid_level"] - row["grid_level"]) == 1
            and abs(other["cl"] - cell["cl"]) > LARGE_CL_JUMP
            and abs(other["cd"] - cell["cd"]) > LARGE_CD_JUMP
            for other in canonical
        )
        cell["duplicate"] = duplicate
        cell["jump"] = jump
        cell["warning"] = duplicate and jump

    return {
        "title": title,
        "zones": zones,
        "dataset_aux": dataset_aux,
        "raw_count": sum(len(zone["rows"]) for zone in zones),
        "grid_count": len(by_view["grid"]),
        "alpha_count": len(by_view["alpha"]),
    }, canonical


def verify_csv(path: Path, canonical: list[dict]) -> None:
    with path.open(newline="", encoding="utf-8") as handle:
        output = list(csv.DictReader(handle))
    require(len(output) == len(canonical), "Rust CSV canonical row count differs from independent parse")
    require(sum(cell["warning"] for cell in canonical) == 1, "fixture must contain exactly one preserved anomaly warning")

    by_key = {(int(row["grid_level"]), float(row["alpha_deg"])): row for row in output}
    require(len(by_key) == len(output), "Rust CSV contains duplicate canonical keys")
    for cell in canonical:
        key = (cell["grid_level"], cell["alpha_deg"])
        row = by_key[key]
        expected = {
            "grid_size": cell["grid_size"],
            "cl_total": cell["cl"],
            "cd_total": cell["cd"],
            "cm_total": cell["cm"],
            "grid_view_source_line": cell["grid"]["line"],
            "alpha_view_source_line": cell["alpha"]["line"],
            "missing_optional_count": cell["missing"],
            "duplicate_cl_cd_at_other_alpha": cell["duplicate"],
            "adjacent_grid_discontinuity": cell["jump"],
            "anomaly_warning": cell["warning"],
        }
        for field, value in expected.items():
            if isinstance(value, bool):
                actual = row[field].lower() == "true"
            elif isinstance(value, float):
                actual = float(row[field])
            elif isinstance(value, int):
                actual = int(row[field])
            else:
                actual = row[field]
            require(actual == value, f"CSV mismatch at {key} field {field}: {actual!r} != {value!r}")


def main() -> int:
    if len(sys.argv) not in (3, 4):
        print("usage: verify-dpw8-v5-independent.py fixture.dat rust-output.csv [--expect-reject]", file=sys.stderr)
        return 2
    source = Path(sys.argv[1])
    output = Path(sys.argv[2])
    expect_reject = len(sys.argv) == 4 and sys.argv[3] == "--expect-reject"
    try:
        parsed, canonical = parse(source)
        if expect_reject:
            raise ValueError("negative-control file was unexpectedly accepted")
        require(parsed["raw_count"] == 40, f"expected 40 raw rows, got {parsed['raw_count']}")
        require(parsed["grid_count"] == 20 and parsed["alpha_count"] == 20, "expected 20 rows in each view")
        require(len(canonical) == 20, f"expected 20 canonical cells, got {len(canonical)}")
        require(all(cell["grid"]["line"] != cell["alpha"]["line"] for cell in canonical), "line identity missing")
        verify_csv(output, canonical)
        print("independent_dpw8_v5_oracle=pass raw=40 grid_view=20 alpha_view=20 canonical=20 anomaly_preserved=1")
        return 0
    except (ValueError, OSError, KeyError) as error:
        if expect_reject:
            print(f"independent_dpw8_v5_negative_control=rejected reason={error}")
            return 0
        print(f"independent_dpw8_v5_oracle=fail reason={error}", file=sys.stderr)
        return 1


if __name__ == "__main__":
    raise SystemExit(main())
