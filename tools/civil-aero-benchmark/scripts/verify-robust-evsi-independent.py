#!/usr/bin/env python3
"""Independent stdlib oracle for the finite-model maximin EVSI known-answer fixture."""
from __future__ import annotations

import csv
import math
import sys
from pathlib import Path


def require(condition: bool, message: str) -> None:
    if not condition:
        raise ValueError(message)


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: verify-robust-evsi-independent.py output.csv", file=sys.stderr)
        return 2
    with Path(sys.argv[1]).open(newline="", encoding="utf-8") as handle:
        rows = list(csv.DictReader(handle))
    require(len(rows) == 2, "expected two candidate experiments")
    by_id = {row["experiment_id"]: row for row in rows}
    require(set(by_id) == {"fragile_perfect_or_blind", "robust_noisy"}, "unexpected experiment set")
    fragile = by_id["fragile_perfect_or_blind"]
    robust = by_id["robust_noisy"]

    # State utilities are (10,0) and (0,10), prior is (0.5,0.5), hence prior value=5.
    # Perfect information gross EVSI=5; uninformative gross EVSI=0; both members cost=1.
    fragile_net_member_a = 5.0 - 1.0
    fragile_net_member_b = 0.0 - 1.0
    # A symmetric 75%-accurate binary signal has posterior best utility 7.5 and gross EVSI 2.5.
    robust_net_both_members = 2.5 - 0.4

    expected = {
        "fragile_perfect_or_blind": (min(fragile_net_member_a, fragile_net_member_b),
                                     (fragile_net_member_a + fragile_net_member_b) / 2.0,
                                     max(fragile_net_member_a, fragile_net_member_b)),
        "robust_noisy": (robust_net_both_members, robust_net_both_members, robust_net_both_members),
    }
    for experiment_id, row in by_id.items():
        worst, mean, best = expected[experiment_id]
        require(math.isclose(float(row["worst_case_net_evsi"]), worst, abs_tol=1e-10),
                f"{experiment_id}: wrong worst-case net EVSI")
        require(math.isclose(float(row["mean_net_evsi"]), mean, abs_tol=1e-10),
                f"{experiment_id}: wrong mean net EVSI")
        require(math.isclose(float(row["best_case_net_evsi"]), best, abs_tol=1e-10),
                f"{experiment_id}: wrong best-case net EVSI")
    require(robust["selected"].lower() == "true", "robust experiment must be selected")
    require(fragile["selected"].lower() == "false", "fragile experiment must not be selected")
    require("finite_ambiguity_set_maximin_not_probability_calibrated" in rows[0]["evidence_level"],
            "finite-ensemble evidence ceiling missing")
    print("independent_robust_evsi_oracle=pass models=2 selected=robust_noisy fragile_worst_case=-1 robust_worst_case=2.1")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (ValueError, OSError, KeyError) as error:
        print(f"independent_robust_evsi_oracle=fail reason={error}", file=sys.stderr)
        raise SystemExit(1)
