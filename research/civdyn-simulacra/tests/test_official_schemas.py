import json
import os
from pathlib import Path

import numpy as np
import pandas as pd
import pytest

import sys

PROJECT_ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(PROJECT_ROOT))
from submission.main import predict


@pytest.mark.parametrize("schema_name", ["unhcr.json", "unicef.json", "world_bank.json"])
def test_public_official_schema_contract(schema_name):
    schema_dir = os.environ.get("SIMULACRABENCH_DATA_DIR")
    if not schema_dir:
        pytest.skip("official public schemas are supplied by the pinned CI checkout")

    schema = json.loads((Path(schema_dir) / schema_name).read_text(encoding="utf-8"))
    schema["gated_value"] = "NA_GATED"
    items = schema["items"]
    visible_items = [
        name for name, spec in items.items()
        if spec["class"] in ("GIVEN", "PREDICT")
    ]
    predict_items = [
        name for name, spec in items.items()
        if spec["class"] == "PREDICT"
    ]
    n_train, n_holdout = 48, 6
    rng = np.random.default_rng(7)
    frame = pd.DataFrame({"respondent_id": np.arange(n_train + n_holdout)})

    for item_id in visible_items:
        spec = items[item_id]
        options = list(spec["values"])
        if spec.get("gate"):
            options.append(schema["gated_value"])
        frame[item_id] = rng.choice(options, size=n_train + n_holdout).astype(object)

    for item_id in predict_items:
        frame.loc[n_train:, item_id] = np.nan

    predictions = predict(frame, schema)
    assert len(predictions) == n_holdout * len(predict_items)
    cursor = 0
    for _row in range(n_train, n_train + n_holdout):
        for item_id in predict_items:
            vector = np.asarray(predictions[cursor], dtype=float)
            expected = len(items[item_id]["values"]) + int(bool(items[item_id].get("gate")))
            assert vector.shape == (expected,), item_id
            assert np.isfinite(vector).all(), item_id
            assert (vector >= 0.0).all(), item_id
            assert abs(float(vector.sum()) - 1.0) < 1e-8, item_id
            cursor += 1
    assert cursor == len(predictions)
