"""CIVDYN synthetic-survey benchmark adapter for SimulacraBench.

This adapter is a research baseline, not a production population model. It uses
only rows and schema supplied to predict(), never external/network data. The
model is fitted to the visible respondent rows in the supplied frame and emits
one categorical probability vector per blank PREDICT cell in canonical order.
"""
from __future__ import annotations

from dataclasses import dataclass
from typing import Any

import numpy as np
import pandas as pd
from sklearn.linear_model import LogisticRegression
from sklearn.preprocessing import OneHotEncoder

_MISSING = "\u241eCIVDYN_MISSING"


@dataclass
class _ItemModel:
    item_id: str
    feature_ids: tuple[str, ...]
    options: tuple[str, ...]
    encoder: OneHotEncoder | None
    classifier: LogisticRegression | None
    marginal: np.ndarray
    shrinkage: float

    def predict_conditioned(self, feature_frame: pd.DataFrame) -> np.ndarray:
        if len(feature_frame) == 0:
            return np.empty((0, len(self.options)), dtype=float)
        if self.classifier is None or self.encoder is None:
            return np.repeat(self.marginal[None, :], len(feature_frame), axis=0)

        encoded = self.encoder.transform(_string_features(feature_frame))
        raw = self.classifier.predict_proba(encoded)
        mapped = np.zeros((len(feature_frame), len(self.options)), dtype=float)
        option_index = {value: index for index, value in enumerate(self.options)}
        for model_index, label in enumerate(self.classifier.classes_):
            index = option_index.get(str(label))
            if index is not None:
                mapped[:, index] = raw[:, model_index]

        # Shrink sparse/separated fits toward a smoothed item marginal so every
        # schema-legal option retains support.
        result = self.shrinkage * mapped + (1.0 - self.shrinkage) * self.marginal[None, :]
        return _normalise(result)


def _string_features(frame: pd.DataFrame) -> pd.DataFrame:
    if frame.shape[1] == 0:
        return pd.DataFrame(index=frame.index)
    out = frame.copy()
    for column in out.columns:
        out[column] = out[column].map(
            lambda value: _MISSING if pd.isna(value) else str(value)
        )
    return out.astype(object)


def _normalise(probabilities: np.ndarray) -> np.ndarray:
    probabilities = np.asarray(probabilities, dtype=float)
    probabilities = np.where(
        np.isfinite(probabilities) & (probabilities >= 0.0), probabilities, 0.0
    )
    totals = probabilities.sum(axis=1, keepdims=True)
    empty = totals[:, 0] <= 0.0
    if np.any(empty):
        probabilities[empty] = 1.0 / probabilities.shape[1]
        totals = probabilities.sum(axis=1, keepdims=True)
    return probabilities / totals


def _item_options(item: dict[str, Any], gated_value: str) -> tuple[str, ...]:
    values = tuple(str(value) for value in item.get("values", ()))
    if not values:
        raise ValueError("PREDICT item has no declared option values")
    if item.get("gate"):
        if gated_value in values:
            raise ValueError("gated sentinel collides with a declared item option")
        return values + (gated_value,)
    return values


def _topological_predict_items(items: dict[str, dict[str, Any]]) -> list[str]:
    """Preserve schema order while ensuring each PREDICT gate parent comes first."""
    predict_ids = [
        name for name, spec in items.items() if spec.get("class") == "PREDICT"
    ]
    predict_set = set(predict_ids)
    state: dict[str, int] = {}
    ordered: list[str] = []

    def visit(item_id: str) -> None:
        marker = state.get(item_id, 0)
        if marker == 1:
            raise ValueError("cyclic PREDICT gate dependency in schema")
        if marker == 2:
            return
        state[item_id] = 1
        gate = items[item_id].get("gate") or {}
        parent = gate.get("parent")
        if parent in predict_set:
            visit(parent)
        state[item_id] = 2
        ordered.append(item_id)

    for item_id in predict_ids:
        visit(item_id)
    return ordered


def _fit_item_model(
    frame: pd.DataFrame,
    schema_items: dict[str, dict[str, Any]],
    item_id: str,
    gated_value: str,
    *,
    regularization: float = 0.35,
    prior_strength: float = 30.0,
) -> _ItemModel:
    spec = schema_items[item_id]
    options = _item_options(spec, gated_value)
    y_raw = frame[item_id]
    observed = y_raw.notna().to_numpy()
    y = y_raw.loc[observed].astype(str)

    # Use schema-declared GIVEN features plus this item's gate parent. Do not
    # use EXCLUDE fields, IDs or arbitrary respondent metadata.
    feature_ids = [
        name for name, item in schema_items.items()
        if item.get("class") == "GIVEN" and name in frame.columns
    ]
    gate = spec.get("gate") or {}
    parent = gate.get("parent")
    if (
        parent
        and parent in frame.columns
        and parent not in feature_ids
        and parent != item_id
    ):
        feature_ids.append(parent)

    n_classes = len(options)
    option_index = {value: index for index, value in enumerate(options)}
    counts = np.zeros(n_classes, dtype=float)
    for value, count in y.value_counts(dropna=False).items():
        index = option_index.get(str(value))
        if index is None:
            raise ValueError(
                f"observed response outside declared schema options for {item_id}"
            )
        counts[index] += float(count)
    marginal = (counts + 0.5) / (counts.sum() + 0.5 * n_classes)

    encoder: OneHotEncoder | None = None
    classifier: LogisticRegression | None = None
    if feature_ids and len(y) >= max(40, 4 * n_classes) and y.nunique() >= 2:
        x_train = _string_features(frame.loc[observed, feature_ids])
        try:
            encoder = OneHotEncoder(
                handle_unknown="ignore", sparse_output=True, dtype=np.float64
            )
        except TypeError:
            encoder = OneHotEncoder(
                handle_unknown="ignore", sparse=True, dtype=np.float64
            )
        encoded = encoder.fit_transform(x_train)
        classifier = LogisticRegression(
            C=regularization,
            solver="lbfgs",
            max_iter=160,
            random_state=0,
        )
        classifier.fit(encoded, y.to_numpy(dtype=str))

    shrinkage = float(len(y) / (len(y) + prior_strength))
    return _ItemModel(
        item_id=item_id,
        feature_ids=tuple(feature_ids),
        options=options,
        encoder=encoder,
        classifier=classifier,
        marginal=marginal,
        shrinkage=shrinkage,
    )


def _predict_item(
    model: _ItemModel,
    frame: pd.DataFrame,
    schema_items: dict[str, dict[str, Any]],
    item_models: dict[str, _ItemModel],
    item_predictions: dict[str, np.ndarray],
) -> np.ndarray:
    """Predict all rows; marginalize an unobserved PREDICT gate parent."""
    n_rows = len(frame)
    spec = schema_items[model.item_id]
    gate = spec.get("gate") or {}
    parent = gate.get("parent")

    if not parent or parent not in model.feature_ids:
        features = frame.loc[:, list(model.feature_ids)]
        return model.predict_conditioned(features)

    parent_is_predict = schema_items.get(parent, {}).get("class") == "PREDICT"
    if not parent_is_predict or parent not in item_predictions:
        features = frame.loc[:, list(model.feature_ids)]
        return model.predict_conditioned(features)

    # Use an observed parent value when present. For held-out parent cells,
    # integrate the child's conditional distribution over the parent's
    # predicted options (including its own gated sentinel).
    parent_values = frame[parent]
    parent_missing = parent_values.isna().to_numpy()
    result = np.zeros((n_rows, len(model.options)), dtype=float)

    observed_rows = np.flatnonzero(~parent_missing)
    if len(observed_rows):
        observed_features = frame.iloc[observed_rows].loc[
            :, list(model.feature_ids)
        ].copy()
        result[observed_rows] = model.predict_conditioned(observed_features)

    missing_rows = np.flatnonzero(parent_missing)
    if len(missing_rows):
        parent_distribution = item_predictions[parent][missing_rows]
        parent_options = item_models[parent].options
        if parent_distribution.shape[1] != len(parent_options):
            raise ValueError("parent probability vector does not match schema option order")
        conditional = np.zeros((len(missing_rows), len(model.options)), dtype=float)
        base_features = frame.iloc[missing_rows].loc[
            :, list(model.feature_ids)
        ].copy()
        for option_index, parent_value in enumerate(parent_options):
            weight = parent_distribution[:, option_index]
            if not np.any(weight > 0.0):
                continue
            features = base_features.copy()
            features[parent] = parent_value
            conditional += weight[:, None] * model.predict_conditioned(features)
        result[missing_rows] = conditional

    return _normalise(result)


def predict(frame: pd.DataFrame, schema: dict[str, Any]) -> list[list[float]]:
    """Return one probability vector per blank PREDICT cell, in canonical order.

    The estimator is deliberately modest: regularized item-wise categorical
    prediction from GIVEN fields, with explicit marginalization over a PREDICT
    gate parent when that parent is hidden. It is not a persona generator or a
    claim of external population validity.
    """
    if not isinstance(frame, pd.DataFrame):
        raise TypeError("frame must be a pandas DataFrame")
    schema_items = schema.get("items")
    if not isinstance(schema_items, dict) or not schema_items:
        raise ValueError("schema.items must be a non-empty mapping")
    if "gated_value" not in schema:
        raise ValueError("schema.gated_value is required")

    gated_value = str(schema["gated_value"])
    predict_ids = [
        name for name, item in schema_items.items()
        if item.get("class") == "PREDICT"
    ]
    for item_id in predict_ids:
        if item_id not in frame.columns:
            raise ValueError(f"missing PREDICT item column: {item_id}")

    ordered_ids = _topological_predict_items(schema_items)
    models = {
        item_id: _fit_item_model(frame, schema_items, item_id, gated_value)
        for item_id in ordered_ids
    }

    item_predictions: dict[str, np.ndarray] = {}
    for item_id in ordered_ids:
        item_predictions[item_id] = _predict_item(
            models[item_id], frame, schema_items, models, item_predictions
        )

    # The harness expects respondent-major order, then schema item order.
    result: list[list[float]] = []
    for row_index in range(len(frame)):
        row = frame.iloc[row_index]
        for item_id in predict_ids:
            if pd.isna(row[item_id]):
                vector = _normalise(
                    item_predictions[item_id][row_index].reshape(1, -1)
                )[0]
                if len(vector) != len(models[item_id].options):
                    raise ValueError("internal probability width does not match declared options")
                if (
                    not np.all(np.isfinite(vector))
                    or np.any(vector < 0.0)
                    or vector.sum() <= 0.0
                ):
                    raise ValueError("internal probability vector is invalid")
                result.append(vector.astype(float).tolist())
    return result
