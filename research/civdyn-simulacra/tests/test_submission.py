import numpy as np
import pandas as pd

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from submission.main import predict


def test_schema_driven_prediction_and_parent_gate_marginalization():
    n_train, n_holdout = 300, 20
    n = n_train + n_holdout
    region = np.where(np.arange(n) % 2 == 0, "north", "south")
    q1 = np.where(region == "north", "yes", "no")
    q2 = np.where(q1 == "yes", np.where(np.arange(n) % 3 == 0, "x", "y"), "NA_GATED")
    q3 = np.where(region == "north", "high", "low")
    frame = pd.DataFrame(
        {
            "respondent_id": np.arange(n),
            "region": region,
            "Q1": q1,
            "Q2": q2,
            "Q3": q3,
        }
    )
    for name in ("Q1", "Q2", "Q3"):
        frame.loc[n_train:, name] = np.nan

    schema = {
        "gated_value": "NA_GATED",
        "items": {
            "respondent_id": {"class": "EXCLUDE"},
            "region": {"class": "GIVEN", "values": ["north", "south"]},
            "Q1": {"class": "PREDICT", "values": ["yes", "no"]},
            "Q2": {
                "class": "PREDICT",
                "values": ["x", "y"],
                "gate": {"parent": "Q1", "observed_if": ["yes"]},
            },
            "Q3": {"class": "PREDICT", "values": ["high", "low"]},
        },
    }
    original_keys = list(schema["items"]["Q2"].keys())
    result = predict(frame, schema)

    assert len(result) == n_holdout * 3
    assert all(
        len(vector) == expected
        for vector, expected in zip(result, [2, 3, 2] * n_holdout)
    )
    for vector in result:
        assert np.isfinite(vector).all()
        assert (np.asarray(vector) >= 0).all()
        assert abs(sum(vector) - 1.0) < 1e-8

    assert np.asarray(result[0]).shape == (2,)
    assert np.asarray(result[1]).shape == (3,)
    # The submitted function must not mutate the organizer's schema.
    assert list(schema["items"]["Q2"].keys()) == original_keys


def test_missing_features_and_single_class_fallback_are_legal():
    frame = pd.DataFrame(
        {
            "A": ["x"] * 30 + [np.nan] * 4,
            "Y": ["a"] * 30 + [np.nan] * 4,
        }
    )
    schema = {
        "gated_value": "NA_GATED",
        "items": {
            "A": {"class": "GIVEN", "values": ["x", "y"]},
            "Y": {"class": "PREDICT", "values": ["a", "b"]},
        },
    }
    output = predict(frame, schema)
    assert len(output) == 4
    assert all(len(vector) == 2 and np.isfinite(vector).all() for vector in output)
    assert all(abs(sum(vector) - 1.0) < 1e-8 for vector in output)


def test_cyclic_predict_gate_dependencies_fail_closed():
    frame = pd.DataFrame({"A": ["x"], "Y1": [np.nan], "Y2": [np.nan]})
    schema = {
        "gated_value": "NA_GATED",
        "items": {
            "A": {"class": "GIVEN", "values": ["x"]},
            "Y1": {
                "class": "PREDICT",
                "values": ["a"],
                "gate": {"parent": "Y2", "observed_if": ["b"]},
            },
            "Y2": {
                "class": "PREDICT",
                "values": ["b"],
                "gate": {"parent": "Y1", "observed_if": ["a"]},
            },
        },
    }
    try:
        predict(frame, schema)
    except ValueError as exc:
        assert "cyclic" in str(exc)
    else:
        raise AssertionError("cyclic gate graph should fail closed")
