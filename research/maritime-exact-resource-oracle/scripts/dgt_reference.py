#!/usr/bin/env python3
"""Independent exhaustive reference for the synthetic DGT direct/diversion fixture."""
from __future__ import annotations

import argparse
import itertools
from pathlib import Path

EXPECTED_FEASIBLE = [
    (("V1_DGT_DIRECT", "V2_NGQURA_FEEDER"), (1, 7, 7, 5, 0)),
    (("V1_NGQURA_FEEDER", "V2_DGT_DIRECT"), (1, 7, 7, 5, 0)),
]


def parse(path: Path):
    vessels: set[str] = set()
    capacities: dict[tuple[str, int], int] = {}
    actions: dict[str, dict] = {}
    uses = []
    for number, raw in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        fields = line.split("\t")
        kind = fields[0]
        if kind == "VESSEL" and len(fields) == 2:
            if fields[1] in vessels:
                raise AssertionError(f"duplicate vessel on line {number}")
            vessels.add(fields[1])
        elif kind == "CAPACITY" and len(fields) == 4:
            key = (fields[1], int(fields[2]))
            if key in capacities:
                raise AssertionError(f"duplicate capacity on line {number}")
            capacities[key] = int(fields[3])
        elif kind == "ACTION" and len(fields) == 8:
            if fields[1] in actions:
                raise AssertionError(f"duplicate action on line {number}")
            actions[fields[1]] = {
                "vessel": fields[2],
                "objective": tuple(map(int, fields[3:8])),
                "uses": [],
            }
        elif kind == "USE" and len(fields) == 5:
            uses.append((fields[1], fields[2], int(fields[3]), int(fields[4])))
        else:
            raise AssertionError(f"malformed row on line {number}: {fields}")

    for action_id, resource, slot, units in uses:
        if action_id not in actions:
            raise AssertionError(f"reservation for unknown action {action_id}")
        if (resource, slot) not in capacities:
            raise AssertionError(f"reservation for unknown resource/slot {(resource, slot)}")
        actions[action_id]["uses"].append((resource, slot, units))

    options = {vessel: [] for vessel in vessels}
    for action_id, action in actions.items():
        if action["vessel"] not in options:
            raise AssertionError(f"unknown vessel {action['vessel']}")
        options[action["vessel"]].append(action_id)
    for values in options.values():
        values.sort()
    return sorted(vessels), capacities, actions, options


def dominates(a: tuple[int, ...], b: tuple[int, ...]) -> bool:
    return all(x <= y for x, y in zip(a, b)) and any(x < y for x, y in zip(a, b))


def solve(vessels, capacities, actions, options):
    feasible = []
    for selected in itertools.product(*(options[v] for v in vessels)):
        used: dict[tuple[str, int], int] = {}
        objective = [0, 0, 0, 0, 0]
        ok = True
        for action_id in selected:
            action = actions[action_id]
            objective = [a + b for a, b in zip(objective, action["objective"])]
            for resource, slot, units in action["uses"]:
                key = (resource, slot)
                proposed = used.get(key, 0) + units
                if proposed > capacities[key]:
                    ok = False
                    break
                used[key] = proposed
            if not ok:
                break
        if ok:
            feasible.append((tuple(selected), tuple(objective)))
    feasible.sort(key=lambda value: value[0])
    pareto = [
        item for index, item in enumerate(feasible)
        if not any(other_index != index and dominates(other[1], item[1])
                   for other_index, other in enumerate(feasible))
    ]
    pareto.sort(key=lambda value: value[0])
    return feasible, pareto


def receipt_lines(feasible, pareto):
    lines = ["DGT_RESOURCE_RECEIPT\t0.1", f"FEASIBLE_COUNT\t{len(feasible)}"]
    for ids, obj in feasible:
        lines.append("FEASIBLE\t" + ",".join(ids) + "\t" + "\t".join(map(str, obj)))
    lines.append(f"PARETO_COUNT\t{len(pareto)}")
    for ids, obj in pareto:
        lines.append("PARETO\t" + ",".join(ids) + "\t" + "\t".join(map(str, obj)))
    lines.append(f"SUMMARY\tPASS\t{len(feasible)}/{len(feasible)}")
    return lines


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--receipt", type=Path, required=True)
    parser.add_argument(
        "--fixture",
        type=Path,
        default=Path("research/maritime-exact-resource-oracle/fixtures/dgt-recovery.tsv"),
    )
    args = parser.parse_args()
    vessels, capacities, actions, options = parse(args.fixture)
    feasible, pareto = solve(vessels, capacities, actions, options)
    if feasible != EXPECTED_FEASIBLE:
        raise AssertionError(f"unexpected frozen DGT feasible set: {feasible}")
    if len(pareto) != 2:
        raise AssertionError(f"expected two Pareto alternatives, got {pareto}")
    expected = receipt_lines(feasible, pareto)
    actual = args.receipt.read_text(encoding="utf-8").splitlines()
    if actual != expected:
        raise AssertionError(f"DGT Rust/reference mismatch\nexpected={expected}\nactual={actual}")
    print("DGT_RECOVERY_REFERENCE\tPASS\t2/2")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
