#!/usr/bin/env python3
"""Emit a deterministic constructed example of the MNAR interval sensitivity curve."""
from __future__ import annotations
import hashlib
import json
import random
from pathlib import Path
import fitted_mar_interval_mnar as audit


def main() -> int:
    rng = random.Random(20261028)
    n = 60
    x = [rng.random() for _ in range(n)]
    # Constructed latent mean is at the 5pp decision margin. Missing outcomes depend
    # on the latent result, deliberately violating MAR; this is not calibrated data.
    latent = [5.0 + 3.0 * (0.5 - xx) + rng.gauss(0.0, 2.3) for xx in x]
    y = [None if rng.random() < (0.65 if yy < 5.0 else 0.05) else yy
         for xx, yy in zip(x, latent)]
    result = audit.analyze(x, y, margin=5.0, sensitivity_deltas=tuple(range(-10, 11)))
    result.update({
        "example": "constructed outcome-dependent missingness; MNAR stress case, not an estimate of real lighting savings",
        "seed": 20261028,
        "latent_true_mean_pct": 5.0,
        "source_sha256": hashlib.sha256(Path(audit.__file__).read_bytes()).hexdigest(),
        "demo_source_sha256": hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
    })
    out = Path(__file__).resolve().parents[2] / "results" / "interval-sensitivity-demo.json"
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(result, sort_keys=True, indent=2, allow_nan=False) + "\n", encoding="utf-8")
    print(f"wrote {out}; n={n}; observed={result['n_observed']}; missing={result['n_missing']}")
    print("decision at delta=0=" + json.dumps(result["pattern_mixture_sensitivity"]["decision_at_delta_zero"], sort_keys=True))
    print("negative delta tipping=" + str(result["pattern_mixture_sensitivity"]["negative_delta_tipping_point_on_supplied_grid_pp"]))
    return 0

if __name__ == "__main__":
    raise SystemExit(main())
