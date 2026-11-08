#!/usr/bin/env python3
"""Independent Python oracle for the read-only NASTRAN BDF intake.

This parser intentionally does not import or execute the Rust implementation.
It checks the frozen synthetic small-field fixture and, when supplied, compares
the Rust JSON inventory against an independently assembled inventory.

This verifies syntax/topology only. It does not solve, validate or qualify FEA.
"""

from __future__ import annotations

import argparse
import copy
import hashlib
import json
import math
import sys
from pathlib import Path
from typing import Any

HERE = Path(__file__).resolve().parent
FIXTURE = HERE / "data" / "synthetic-equivalent-beam.bdf"


class AuditError(ValueError):
    """Raised when a BDF source or inventory violates the frozen contract."""


def require(condition: bool, message: str) -> None:
    if not condition:
        raise AuditError(message)


def fixed_fields(line: str, number: int) -> list[str]:
    require(line.isascii(), f"line {number}: non-ASCII input")
    require("$" not in line, f"line {number}: inline comments are outside the admitted syntax")
    require(len(line) <= 80, f"line {number}: line exceeds small-field 80-column width")
    padded = line.ljust(80)
    return [padded[index * 8 : (index + 1) * 8].strip() for index in range(10)]


def parse_deck(data: bytes) -> tuple[list[dict[str, Any]], list[str]]:
    require(data, "empty source")
    require(data.isascii(), "source contains non-ASCII bytes")
    text = data.decode("ascii")
    cards: list[dict[str, Any]] = []
    executive: list[str] = []
    in_bulk = False
    enddata = False
    pending: dict[str, Any] | None = None

    for number, raw_line in enumerate(text.splitlines(), start=1):
        line = raw_line.rstrip("\r")
        stripped = line.strip()
        if not stripped or stripped.startswith("$"):
            continue
        if not in_bulk:
            executive.append(line)
            if stripped.upper() == "BEGIN BULK":
                in_bulk = True
            continue
        if stripped.upper() == "ENDDATA":
            if pending is not None:
                require(pending["next"] is None,
                        f"line {number}: ENDDATA with unresolved continuation {pending['next']}")
                cards.append(pending)
                pending = None
            enddata = True
            break

        fields = fixed_fields(line, number)
        first = fields[0]
        if not first:
            require(pending is not None, f"line {number}: orphan implicit continuation")
            require(pending["next"] is None,
                    f"line {number}: card {pending['name']} requires explicit continuation {pending['next']}")
            pending["fields"].extend(fields[1:9])
            pending["lines"].append(number)
            next_cont = fields[9] or None
            if next_cont is not None:
                require(next_cont.startswith("+"), f"line {number}: unsupported continuation label")
            pending["next"] = next_cont
            continue

        require(not first.startswith("*"), f"line {number}: large-field card not admitted in v0.1")
        if first.startswith("+"):
            require(pending is not None, f"line {number}: orphan continuation")
            require(
                first == pending["next"],
                f"line {number}: expected continuation {pending['next']}, received {first}",
            )
            pending["fields"].extend(fields[1:9])
            pending["lines"].append(number)
            pending["next"] = fields[9] or None
            if pending["next"] is None:
                cards.append(pending)
                pending = None
            continue

        if pending is not None:
            require(pending["next"] is None,
                    f"line {number}: new card before required continuation {pending['next']}")
            cards.append(pending)
            pending = None
        require(not first.endswith("*"), f"line {number}: large-field card not admitted in v0.1")
        next_cont = fields[9] or None
        if next_cont is not None:
            require(next_cont.startswith("+"), f"line {number}: unsupported continuation label")
        pending = {
            "name": first.upper(),
            "fields": fields[1:9],
            "lines": [number],
            "next": next_cont,
        }

    require(in_bulk, "BEGIN BULK missing")
    require(enddata, "ENDDATA missing")
    require(pending is None, "source ended with unresolved continuation")
    return cards, executive


def as_int(value: str, label: str) -> int:
    try:
        result = int(value.strip())
    except (TypeError, ValueError):
        raise AuditError(f"{label}: invalid integer {value!r}") from None
    require(result >= 0, f"{label}: negative ID")
    return result

def as_id(value: str, label: str) -> int:
    result = as_int(value, label)
    require(result > 0, f"{label}: entity ID zero is reserved")
    return result


def validate_component_digits(value: str, label: str) -> None:
    require(
        bool(value) and all("1" <= char <= "6" for char in value),
        f"{label}: components must contain only digits 1 through 6",
    )
    require(len(set(value)) == len(value), f"{label}: duplicate component digit")


def case_control_references(executive: list[str]) -> tuple[set[int], set[int]]:
    spc: set[int] = set()
    loads: set[int] = set()
    for line in executive:
        if "=" not in line:
            continue
        key, value = line.split("=", 1)
        key = key.strip().upper()
        if key == "SPC":
            spc.add(as_id(value, "case-control SPC"))
        elif key == "LOAD":
            loads.add(as_id(value, "case-control LOAD"))
    return spc, loads


def visit_load_set(
    sid: int,
    dependencies: dict[int, set[int]],
    force_sets: set[int],
    visiting: set[int],
    completed: set[int],
) -> None:
    if sid in force_sets:
        require(sid not in dependencies, f"load set {sid} is both LOAD and FORCE")
        return
    if sid in completed:
        return
    require(sid in dependencies, f"LOAD references unresolved load set ID {sid}")
    require(sid not in visiting, f"cyclic LOAD dependency at set {sid}")
    visiting.add(sid)
    for child in dependencies[sid]:
        visit_load_set(child, dependencies, force_sets, visiting, completed)
    visiting.remove(sid)
    completed.add(sid)


def as_float(value: str, label: str) -> float:
    token = value.strip().replace("D", "E").replace("d", "E")
    if not token:
        raise AuditError(f"{label}: blank numeric value")
    try:
        result = float(token)
    except ValueError:
        split = None
        for index in range(1, len(token)):
            if token[index] in "+-":
                split = index
        if split is None:
            raise AuditError(f"{label}: invalid number {value!r}") from None
        try:
            result = float(f"{token[:split]}e{token[split:]}")
        except ValueError:
            raise AuditError(f"{label}: invalid implicit-exponent number {value!r}") from None
    require(math.isfinite(result), f"{label}: non-finite value")
    return result


def inventory(data: bytes) -> dict[str, Any]:
    cards, executive = parse_deck(data)
    names: dict[str, int] = {}
    grids: set[int] = set()
    cbeams: set[int] = set()
    elements: set[int] = set()
    materials: set[int] = set()
    pbeam_materials: dict[int, int] = {}
    rbe2_ids: set[int] = set()
    spc_sets: set[int] = set()
    force_sets: set[int] = set()
    load_sets: set[int] = set()
    beam_refs: set[int] = set()
    beam_property_refs: set[int] = set()
    rbe_refs: set[int] = set()
    spc_refs: set[int] = set()
    force_refs: set[int] = set()
    load_dependencies: dict[int, set[int]] = {}
    opaque = 0

    def unique(target: set[int], value: int, kind: str) -> None:
        require(value not in target, f"duplicate {kind} ID {value}")
        target.add(value)

    for card in cards:
        name = card["name"]
        fields = card["fields"]
        names[name] = names.get(name, 0) + 1
        if name not in {
            "GRID", "CBEAM", "MAT1", "PBEAM", "CONM2", "RBE2",
            "SPC", "SPC1", "FORCE", "LOAD", "PARAM",
        }:
            opaque += 1
            continue

        def required(index: int, label: str) -> str:
            require(index < len(fields) and fields[index].strip(), f"{name}: {label} missing")
            return fields[index]

        if name == "GRID":
            gid = as_id(required(0, "GRID ID"), "GRID ID")
            unique(grids, gid, "GRID")
            if fields[1].strip():
                require(as_int(fields[1], "GRID CP") == 0,
                        f"GRID {gid}: non-basic CP coordinate system unsupported")
            if fields[5].strip():
                require(as_int(fields[5], "GRID CD") == 0,
                        f"GRID {gid}: non-basic CD displacement frame unsupported")
            for index in (2, 3, 4):
                as_float(required(index, f"coordinate {index - 1}"), f"GRID {gid} coordinate")
        elif name == "CBEAM":
            eid = as_id(required(0, "CBEAM EID"), "CBEAM EID")
            pid = as_id(required(1, "CBEAM PID"), "CBEAM PID")
            beam_property_refs.add(pid)
            ga = as_id(required(2, "CBEAM GA"), "CBEAM GA")
            gb = as_id(required(3, "CBEAM GB"), "CBEAM GB")
            require(ga != gb, f"CBEAM {eid}: identical end grids")
            unique(elements, eid, "element")
            cbeams.add(eid)
            beam_refs.update((ga, gb))
            orientation = [fields[index] for index in (4, 5, 6)]
            present = sum(bool(value.strip()) for value in orientation)
            require(present in (0, 3), f"CBEAM {eid}: partial orientation vector")
            orientation_numbers = []
            for index, value in zip((4, 5, 6), orientation):
                if value.strip():
                    orientation_numbers.append(as_float(value, f"CBEAM {eid} orientation"))
            if present == 3:
                require(sum(value * value for value in orientation_numbers) > 1.0e-24,
                        f"CBEAM {eid}: zero orientation vector")
        elif name == "MAT1":
            mid = as_id(required(0, "MAT1 MID"), "MAT1 MID")
            unique(materials, mid, "MAT1")
            for index in range(1, 8):
                if fields[index].strip():
                    as_float(fields[index], f"MAT1 {mid} field {index + 1}")
        elif name == "PBEAM":
            pid = as_id(required(0, "PBEAM PID"), "PBEAM PID")
            mid = as_id(required(1, "PBEAM MID"), "PBEAM MID")
            require(pid not in pbeam_materials, f"duplicate PBEAM {pid}")
            pbeam_materials[pid] = mid
            for index in range(2, 8):
                if fields[index].strip():
                    as_float(fields[index], f"PBEAM {pid} core field {index + 1}")
        elif name == "CONM2":
            eid = as_id(required(0, "CONM2 EID"), "CONM2 EID")
            grid = as_id(required(1, "CONM2 GRID"), "CONM2 GRID")
            unique(elements, eid, "element")
            if fields[2].strip():
                require(as_int(fields[2], "CONM2 CID") == 0,
                        f"CONM2 {eid}: non-basic coordinate system CID unsupported")
            mass = as_float(required(3, "CONM2 mass"), "CONM2 mass")
            require(mass >= 0.0, f"CONM2 {eid}: negative mass")
            for index in range(4, min(13, len(fields))):
                if fields[index].strip():
                    as_float(fields[index], f"CONM2 {eid} field {index + 1}")
            beam_refs.add(grid)
        elif name == "RBE2":
            eid = as_id(required(0, "RBE2 EID"), "RBE2 EID")
            independent = as_id(required(1, "RBE2 GN"), "RBE2 GN")
            validate_component_digits(required(2, "RBE2 component"), "RBE2 components")
            unique(elements, eid, "element")
            rbe2_ids.add(eid)
            rbe_refs.add(independent)
            dependents: set[int] = set()
            for token in fields[3:]:
                if token.strip():
                    dep = as_id(token, "RBE2 dependent grid")
                    unique(dependents, dep, "RBE2 dependent GRID")
                    rbe_refs.add(dep)
            require(dependents, f"RBE2 {eid}: no dependent grids")
        elif name == "SPC":
            sid = as_id(required(0, "SPC SID"), "SPC SID")
            spc_sets.add(sid)
            group_count = 0
            index = 1
            while index < len(fields):
                grid_field = fields[index] if index < len(fields) else ""
                component_field = fields[index + 1] if index + 1 < len(fields) else ""
                value_field = fields[index + 2] if index + 2 < len(fields) else ""
                if not grid_field.strip() and not component_field.strip() and not value_field.strip():
                    break
                require(grid_field.strip() and component_field.strip(),
                        f"SPC {sid}: incomplete GRID/component pair")
                spc_refs.add(as_id(grid_field, "SPC GRID"))
                validate_component_digits(component_field, f"SPC {sid} components")
                if value_field.strip():
                    as_float(value_field, "SPC value")
                group_count += 1
                index += 3
            require(group_count > 0, f"SPC {sid}: no GRID/component groups")
        elif name == "SPC1":
            sid = as_id(required(0, "SPC1 SID"), "SPC1 SID")
            spc_sets.add(sid)
            validate_component_digits(required(1, "SPC1 component"), "SPC1 components")
            index = 2
            while index < len(fields):
                if not fields[index].strip():
                    index += 1
                    continue
                start = as_id(fields[index], "SPC1 grid")
                if index + 1 < len(fields) and fields[index + 1].upper() == "THRU":
                    require(index + 2 < len(fields), "SPC1 THRU end missing")
                    end = as_id(fields[index + 2], "SPC1 THRU end")
                    require(start <= end and end - start <= 100_000, "invalid SPC1 THRU range")
                    spc_refs.update(range(start, end + 1))
                    index += 3
                else:
                    spc_refs.add(start)
                    index += 1
        elif name == "FORCE":
            sid = as_id(required(0, "FORCE SID"), "FORCE SID")
            grid = as_id(required(1, "FORCE GRID"), "FORCE GRID")
            if fields[2].strip():
                require(as_int(fields[2], "FORCE CID") == 0,
                        f"FORCE SID {sid}: non-basic coordinate system CID unsupported")
            for index in range(3, 7):
                as_float(required(index, "FORCE number"), "FORCE number")
            force_sets.add(sid)
            force_refs.add(grid)
        elif name == "LOAD":
            sid = as_id(required(0, "LOAD SID"), "LOAD SID")
            require(sid not in load_dependencies, f"duplicate LOAD set ID {sid}")
            load_sets.add(sid)
            as_float(required(1, "LOAD scale"), "LOAD scale")
            end = max((i + 1 for i, value in enumerate(fields) if value.strip()), default=2)
            require(end >= 4 and (end - 2) % 2 == 0, f"LOAD {sid}: malformed scale pairs")
            dependencies: set[int] = set()
            for index in range(2, end, 2):
                as_float(fields[index], "LOAD scale factor")
                dependencies.add(as_id(fields[index + 1], "LOAD set reference"))
            load_dependencies[sid] = dependencies
        elif name == "PARAM":
            required(0, "PARAM name")
            required(1, "PARAM value")

    for pid in beam_property_refs:
        require(pid in pbeam_materials, f"CBEAM references missing PBEAM property ID {pid}")
    for pid, mid in pbeam_materials.items():
        require(mid in materials, f"PBEAM {pid}: missing MAT1 {mid}")

    all_refs = beam_refs | rbe_refs | spc_refs | force_refs
    for grid in all_refs:
        require(grid in grids, f"referenced GRID {grid} missing")

    visiting: set[int] = set()
    completed: set[int] = set()
    for sid in load_dependencies:
        visit_load_set(sid, load_dependencies, force_sets, visiting, completed)

    selected_spc, selected_loads = case_control_references(executive)
    for sid in selected_spc:
        require(sid in spc_sets, f"case control selects missing SPC set {sid}")
    for sid in selected_loads:
        visit_load_set(sid, load_dependencies, force_sets, visiting, completed)

    connected = beam_refs | rbe_refs | spc_refs | force_refs
    return {
        "schema_version": "aero-nastran-bdf-intake/0.1.0",
        "source_sha256": hashlib.sha256(data).hexdigest(),
        "source_bytes": len(data),
        "card_count": len(cards),
        "card_counts": dict(sorted(names.items())),
        "grid_ids": sorted(grids),
        "cbeam_ids": sorted(cbeams),
        "element_ids": sorted(elements),
        "material_ids": sorted(materials),
        "property_materials": {str(key): value for key, value in sorted(pbeam_materials.items())},
        "rbe2_ids": sorted(rbe2_ids),
        "spc_set_ids": sorted(spc_sets),
        "force_set_ids": sorted(force_sets),
        "load_set_ids": sorted(load_sets),
        "opaque_cards": opaque,
        "orphan_grid_count": len(grids - connected),
        "units_status": "UNKNOWN",
        "material_allowables_status": "UNKNOWN",
        "solver_result": "NOT_ASSESSED",
        "structural_response": "NOT_ASSESSED",
        "claim_ceiling": "syntax_and_topology_inventory_only",
    }


def expect_rejected(label: str, action) -> None:
    try:
        action()
    except (AuditError, KeyError, IndexError, ValueError):
        print(f"negative_control=pass case={label}")
        return
    raise AuditError(f"negative control unexpectedly passed: {label}")


def fixed(fields: list[str]) -> str:
    require(len(fields) <= 10, "test fixture has too many fields")
    return "".join(value.ljust(8) for value in fields + [""] * (10 - len(fields)))


def synthetic_text() -> bytes:
    return FIXTURE.read_bytes()


def hostile_controls(data: bytes) -> None:
    lines = data.decode("ascii").splitlines()

    duplicate = lines.copy()
    duplicate.insert(duplicate.index("ENDDATA"), fixed(["GRID", "1", "0", "9.", "9.", "9."]))
    expect_rejected("duplicate_grid_id", lambda: inventory(("\n".join(duplicate) + "\n").encode("ascii")))

    zero_id = lines.copy()
    index = next(i for i, line in enumerate(zero_id) if line.startswith("GRID"))
    zero_id[index] = fixed(["GRID", "0", "0", "0.", "0.", "0."])
    expect_rejected("zero_entity_id", lambda: inventory(("\n".join(zero_id) + "\n").encode("ascii")))

    dangling = lines.copy()
    index = next(i for i, line in enumerate(dangling) if line.startswith("CBEAM"))
    dangling[index] = fixed(["CBEAM", "101", "20", "1", "99", "0.", "1.", "0."])
    expect_rejected("dangling_cbeam_grid", lambda: inventory(("\n".join(dangling) + "\n").encode("ascii")))

    missing_material = lines.copy()
    index = next(i for i, line in enumerate(missing_material) if line.startswith("PBEAM"))
    missing_material[index] = fixed(["PBEAM", "20", "999", ".001", ".00001", ".00001", "0.", ".00001", "0."])
    expect_rejected("dangling_pbeam_material", lambda: inventory(("\n".join(missing_material) + "\n").encode("ascii")))

    missing_pbeam = lines.copy()
    index = next(i for i, line in enumerate(missing_pbeam) if line.startswith("CBEAM"))
    missing_pbeam[index] = fixed(["CBEAM", "101", "99", "1", "2", "0.", "1.", "0."])
    expect_rejected("cbeam_missing_property", lambda: inventory(("\n".join(missing_pbeam) + "\n").encode("ascii")))

    bulk_index = lines.index("BEGIN BULK")
    cyclic_load = lines.copy()
    index = next(i for i, line in enumerate(cyclic_load[bulk_index + 1 :], start=bulk_index + 1)
                 if line.startswith("LOAD"))
    cyclic_load[index] = fixed(["LOAD", "2", "1.", "1.", "4"])
    cyclic_load.insert(cyclic_load.index("ENDDATA"), fixed(["LOAD", "4", "1.", "1.", "2"]))
    expect_rejected("cyclic_load_dependency", lambda: inventory(("\n".join(cyclic_load) + "\n").encode("ascii")))

    nested_load = lines.copy()
    index = next(i for i, line in enumerate(nested_load[bulk_index + 1 :], start=bulk_index + 1)
                 if line.startswith("LOAD"))
    nested_load[index] = fixed(["LOAD", "2", "1.", "1.", "4"])
    nested_load.insert(nested_load.index("ENDDATA"), fixed(["LOAD", "4", "1.", "1.", "3"]))
    inventory(("\n".join(nested_load) + "\n").encode("ascii"))
    print("positive_control=pass case=nested_load_dependency")

    missing_control_load = lines.copy()
    load_control_index = next(i for i, line in enumerate(missing_control_load) if line.strip().upper() == "LOAD = 2")
    missing_control_load[load_control_index] = "LOAD = 99"
    expect_rejected("case_control_load_missing", lambda: inventory(("\n".join(missing_control_load) + "\n").encode("ascii")))

    nonbasic_grid_frame = lines.copy()
    index = next(i for i, line in enumerate(nonbasic_grid_frame) if line.startswith("GRID"))
    nonbasic_grid_frame[index] = fixed(["GRID", "1", "9", "0.", "0.", "0."])
    expect_rejected("unsupported_grid_coordinate_frame", lambda: inventory(("\n".join(nonbasic_grid_frame) + "\n").encode("ascii")))

    nonbasic_force_frame = lines.copy()
    index = next(i for i, line in enumerate(nonbasic_force_frame) if line.startswith("FORCE"))
    nonbasic_force_frame[index] = fixed(["FORCE", "3", "3", "1", "1000.", "0.", "1.", "0."])
    expect_rejected("unsupported_force_coordinate_frame", lambda: inventory(("\n".join(nonbasic_force_frame) + "\n").encode("ascii")))

    invalid_spc_components = lines.copy()
    index = next(i for i, line in enumerate(invalid_spc_components[bulk_index + 1 :], start=bulk_index + 1)
                 if line.startswith("SPC"))
    invalid_spc_components[index] = fixed(["SPC", "1", "1", "0", "0."])
    expect_rejected("invalid_spc_components", lambda: inventory(("\n".join(invalid_spc_components) + "\n").encode("ascii")))

    bulk_index = lines.index("BEGIN BULK")
    implicit_cbeam = lines.copy()
    cbeam_index = next(i for i, line in enumerate(implicit_cbeam[bulk_index + 1 :], start=bulk_index + 1)
                       if line.startswith("CBEAM"))
    implicit_cbeam.insert(cbeam_index + 1, fixed(["", "0.1", "0.2", "0.3"]))
    inventory(("\n".join(implicit_cbeam) + "\n").encode("ascii"))
    print("positive_control=pass case=implicit_cbeam_continuation")

    implicit_conm2 = lines.copy()
    conm2_index = next(i for i, line in enumerate(implicit_conm2[bulk_index + 1 :], start=bulk_index + 1)
                       if line.startswith("CONM2"))
    implicit_conm2.insert(conm2_index + 1, fixed(["", "1.", "2.", "3."]))
    inventory(("\n".join(implicit_conm2) + "\n").encode("ascii"))
    print("positive_control=pass case=implicit_conm2_continuation")

    orphan_implicit = lines.copy()
    orphan_implicit.insert(bulk_index + 1, fixed(["", "1.", "2."]))
    expect_rejected("orphan_implicit_continuation",
                    lambda: inventory(("\n".join(orphan_implicit) + "\n").encode("ascii")))

    blank_spc_value = lines.copy()
    spc_index = next(i for i, line in enumerate(blank_spc_value[bulk_index + 1 :], start=bulk_index + 1)
                     if line.startswith("SPC"))
    blank_spc_value[spc_index] = fixed(["SPC", "1", "1", "123456", ""])
    inventory(("\n".join(blank_spc_value) + "\n").encode("ascii"))
    print("positive_control=pass case=blank_spc_enforced_value")

    wrong_continuation = lines.copy()
    index = next(i for i, line in enumerate(wrong_continuation) if line.startswith("+A"))
    wrong_continuation[index] = fixed(["+B", "3"])
    expect_rejected("continuation_label_mismatch", lambda: inventory(("\n".join(wrong_continuation) + "\n").encode("ascii")))

    nonfinite = lines.copy()
    index = next(i for i, line in enumerate(nonfinite) if line.startswith("GRID"))
    nonfinite[index] = fixed(["GRID", "1", "0", "NaN", "0.", "0."])
    expect_rejected("nonfinite_coordinate", lambda: inventory(("\n".join(nonfinite) + "\n").encode("ascii")))

    missing_end = [line for line in lines if line.strip().upper() != "ENDDATA"]
    expect_rejected("missing_enddata", lambda: inventory(("\n".join(missing_end) + "\n").encode("ascii")))


def verify_rust_output(output: dict[str, Any], expected: dict[str, Any]) -> None:
    for key, value in expected.items():
        require(output.get(key) == value, f"Rust/Python mismatch for {key}: {output.get(key)!r} != {value!r}")
    require(output.get("claim_ceiling") == "syntax_and_topology_inventory_only", "claim ceiling changed")
    require(output.get("units_status") == "UNKNOWN", "unit status was promoted")
    require(output.get("material_allowables_status") == "UNKNOWN", "material allowable status was promoted")
    require(output.get("structural_response") == "NOT_ASSESSED", "structural response was promoted")
    require(output.get("solver_result") == "NOT_ASSESSED", "solver result was promoted")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--rust-output", type=Path)
    args = parser.parse_args()
    try:
        data = synthetic_text()
        expected = inventory(data)
        require(expected["card_counts"]["GRID"] == 3, "fixture GRID count changed")
        require(expected["card_counts"]["CBEAM"] == 2, "fixture CBEAM count changed")
        require(expected["card_counts"]["RBE2"] == 1, "fixture RBE2 count changed")
        require(expected["opaque_cards"] == 1, "unknown card should remain opaque")
        require(expected["orphan_grid_count"] == 0, "fixture unexpectedly contains orphan grids")

        if args.rust_output is not None:
            actual = json.loads(args.rust_output.read_text(encoding="utf-8"))
            verify_rust_output(actual, expected)
        hostile_controls(data)

        mode = "rust_python_boundary=pass" if args.rust_output is not None else "rust_python_boundary=not_run"
        print(
            "nastran_bdf_independent_oracle=pass "
            f"cards={expected['card_count']} grids={len(expected['grid_ids'])} "
            f"elements={len(expected['element_ids'])} {mode}"
        )
    except (OSError, json.JSONDecodeError, AuditError, KeyError, TypeError, ValueError) as error:
        print(f"nastran_bdf_oracle_error={error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
