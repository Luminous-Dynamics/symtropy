#!/usr/bin/env python3
"""Compatibility entry point for the canonical long-horizon evidence evaluator.

The implementation and tests live beside the matched memory-condition
comparator in tools/av-modality-necessity-evaluator. Keep this file as a
thin launcher so older documentation paths execute the same implementation.
"""
from __future__ import annotations

import sys
from pathlib import Path

CANONICAL_TOOLS = Path(__file__).resolve().parents[1] / "av-modality-necessity-evaluator"
sys.path.insert(0, str(CANONICAL_TOOLS))

from long_horizon_evaluate import main  # noqa: E402


if __name__ == "__main__":
    raise SystemExit(main())
