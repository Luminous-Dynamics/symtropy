#!/usr/bin/env python3
"""Independent standard-library reference for the tiny maritime resource oracle."""
from __future__ import annotations

import argparse
import itertools
from pathlib import Path

EXPECTED_FEASIBLE = [
    (("V1_A_EARLY", "V2_A_LATE"), (3, 7, 15, 4, 0)),
    (("V1_A_EARLY", "V2_B_LATE"), (3, 8, 12, 8, 1)),
    (("V1_B_EARLY", "V2_A_LATE"), (3, 8, 12, 7, 0)),
    (("V1_B_EARLY", "V2_B_LATE"), (3, 9, 9, 11, 1)),
]
EXPECTED_PARETO = [
    (("V1_A_EARLY", "V2_A_LATE"), (3, 7, 15, 4, 0)),
    (("V1_B_EARLY", "V2_A_LATE"), (3, 8, 12, 7, 0)),
    (("V1_B_EARLY", "V2_B_LATE"), (3, 9, 9, 11, 1)),
]


def parse_fixture(path: Path):
    vessels: set[str] = set()
    capacities: dict[tuple[str, int], int] = {}
    candidates: dict[str, dict] = {}
    uses: list[tuple[str, str, int, int]] = []

    for line_number, raw_line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
        line = raw_line.strip()
        if not line or line.startswith("#"):
            continue
        fields = line.split("\\t")
        kind = fields[0]
        if kind == "VESSEL" and len(fields) == 2:
            if fields[1] in vessels:
                raise AssertionError(f"duplicate vessel at line {line_number}")
            vessels.add(fields[1])
        elif kind == "CAPACITY" and len(fields) == 4:
            key = (fields[1], int(fields[2]))
            if key in capacities:
                raise AssertionError(f"duplicate capacity at line {line_number}")
            capacities[key] = int(fields[3])
        elif kind == "ACTION" and len(fields) == 8:
            candidate_id = fields[1]
            if candidate_id in candidates:
                raise AssertionError(f"duplicate candidate at line {line_number}")
            candidates[candidate_id] = {
                "vessel": fields[2],
                "objective": tuple(map(int, fields[3:8])),
                "reservations": [],
            }
        elif kind == "USE" and len(fields) == 5:
            uses.append((fields[1], fields[2], int(fields[3]), int(fields[4])))
        else:
            raise AssertionError(f"unknown/malformed fixture record at line {line_number}")

    for candidate_id, resource_id, slot, units in uses:
        if candidate_id not in candidates:
            raise AssertionError(f"USE references unknown candidate {candidate_id}")
        candidates[candidate_id]["reservations"].append((resource_id, slot, units))

    for candidate_id, candidate in candidates.items():
        if candidate["vessel"] not in vessels:
            raise AssertionError(f"{candidate_id} references unknown vessel")
        for resource_id, slot, _ in candidate["reservations"]:
            if (resource_id, slot) not in capacities:
                raise AssertionError(f"{candidate_id} references missing resource slot")

    options: dict[str, list[str]] = {vessel: [] for vessel in vessels}
    for candidate_id, candidate in candidates.items():
        options[candidate["vessel"]].append(candidate_id)
    if not vessels or any(not values for values in options.values()):
        raise AssertionError("every declared vessel must have at least one candidate")
    for values in options.values():
        values.sort()

    return sorted(vessels), capacities, candidates, options


def add_vectors(left: tuple[int, ...], right: tuple[int, ...]) -> tuple[int, ...]:
    return tuple(a + b for a, b in zip(left, right))


def dominates(left: tuple[int, ...], right: tuple[int, ...]) -> bool:
    return all(a <= b for a, b in zip(left, right)) and any(
        a < b for a, b in zip(left, right)
    )


def exact_search(vessels, capacities, candidates, options):
    feasible: list[tuple[tuple[str, ...], tuple[int, ...]]] = []
    for choice in itertools.product(*(options[vessel] for vessel in vessels)):
        usage: dict[tuple[str, int], int] = {}
        objective = (0, 0, 0, 0, 0)
        is_feasible = True
        for candidate_id in choice:
            candidate = candidates[candidate_id]
            objective = add_vectors(objective, candidate["objective"])
            totals: dict[tuple[str, int], int] = {}
            for resource_id, slot, units in candidate["reservations"]:
                key = (resource_id, slot)
                totals[key] = totals.get(key, 0) + units
            for key, units in totals.items():
                proposed = usage.get(key, 0) + units
                if proposed > capacities[key]:
                    is_feasible = False
                    break
            if not is_feasible:
                break
            for key, units in totals.items():
                usage[key] = usage.get(key, 0) + units
        if is_feasible:
            feasible.append((tuple(choice), objective))

    feasible.sort(key=lambda item: item[0])
    frontier = [
        item for index, item in enumerate(feasible)
        if not any(
            other_index != index and dominates(other[1], item[1])
            for other_index, other in enumerate(feasible)
        )
    ]
    frontier.sort(key=lambda item: item[0])
    return feasible, frontier


def objective_fields(objective: tuple[int, ...]) -> str:
    return "\\t".join(map(str, objective))


def receipt_lines(vessels, capacities, candidates, feasible, frontier) -> list[str]:
    lines = ["RESOURCE_ORACLE_RECEIPT\\t0.1.0"]
    for vessel_id in sorted(vessels):
        lines.append(f"SCENARIO_VESSEL\\t{vessel_id}")
    for (resource_id, slot), capacity in sorted(capacities.items()):
        lines.append(f"SCENARIO_CAPACITY\\t{resource_id}\\t{slot}\\t{capacity}")
    for candidate_id in sorted(candidates):
        candidate = candidates[candidate_id]
        lines.append(
            f"SCENARIO_CANDIDATE\\t{candidate_id}\\t{candidate['vessel']}\\t"
            f"{objective_fields(candidate['objective'])}"
        )
        for resource_id, slot, units in sorted(candidate["reservations"]):
            lines.append(
                f"SCENARIO_RESERVATION\\t{candidate_id}\\t{resource_id}\\t{slot}\\t{units}"
            )

    lines.append(f"FEASIBLE_COUNT\\t{len(feasible)}")
    for candidate_ids, objective in feasible:
        lines.append(
            "FEASIBLE\\t" + ",".join(candidate_ids) + "\\t" + objective_fields(objective)
        )
    lines.append(f"PARETO_COUNT\\t{len(frontier)}")
    for candidate_ids, objective in frontier:
        lines.append(
            "PARETO\\t" + ",".join(candidate_ids) + "\\t" + objective_fields(objective)
        )
    lines.append("SUMMARY\\tPASS")
    return lines


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--receipt", type=Path, required=True)
    parser.add_argument(
        "--fixture",
        type=Path,
        default=Path("research/maritime-exact-resource-oracle/fixtures/tiny-port.tsv"),
    )
    args = parser.parse_args()

    vessels, capacities, candidates, options = parse_fixture(args.fixture)
    feasible, frontier = exact_search(vessels, capacities, candidates, options)
    if feasible != EXPECTED_FEASIBLE:
        raise AssertionError(
            f"independent enumerator did not reproduce hand-authored feasible set: {feasible}"
        )
    if frontier != EXPECTED_PARETO:
        raise AssertionError(
            f"independent enumerator did not reproduce hand-authored Pareto set: {frontier}"
        )

    expected_from_reference = receipt_lines(
        vessels, capacities, candidates, feasible, frontier
    )
    actual = args.receipt.read_text(encoding="utf-8").splitlines()
    if actual != expected_from_reference:
        raise AssertionError(
            "Rust oracle / Python reference mismatch\\n"
            f"expected: {expected_from_reference}\\nactual: {actual}"
        )

    print(f"RESOURCE_REFERENCE\\tPASS\\tfeasible={len(feasible)}\\tpareto={len(frontier)}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
