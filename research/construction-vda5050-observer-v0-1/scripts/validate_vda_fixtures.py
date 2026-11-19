#!/usr/bin/env python3
"""Validate frozen VDA 5050 v3.0.0 example payloads against a pinned schema snapshot.

The VDA v3.0.0 PDF spells the factsheet field mobileRobotKinematics (plural).
The repository schema snapshot has a typo in the factsheet typeSpecification.required
list (mobileRobotKinematic, singular). Apply a transparent in-memory overlay for
that one known discrepancy; never mutate or re-label the upstream schema artifact.
"""
from __future__ import annotations

import copy
import json
import sys
from pathlib import Path

from jsonschema import Draft202012Validator

if len(sys.argv) != 3:
    raise SystemExit("usage: validate_vda_fixtures.py VDA_SCHEMA_DIR FIXTURE_DIR")

schema_root = Path(sys.argv[1])
fixture_root = Path(sys.argv[2])

mapping = {
    "order": "order.schema",
    "state-action-finished": "state.schema",
    "factsheet": "factsheet.schema",
    "zoneSet": "zoneSet.schema",
    "connection": "connection.schema",
    "instantActions": "instantActions.schema",
    "responses": "responses.schema",
}

for fixture_name, schema_name in mapping.items():
    schema_path = schema_root / schema_name
    fixture_path = fixture_root / f"{fixture_name}.json"
    schema = json.loads(schema_path.read_text(encoding="utf-8"))
    fixture = json.loads(fixture_path.read_text(encoding="utf-8"))

    Draft202012Validator.check_schema(schema)
    effective_schema = copy.deepcopy(schema)
    if fixture_name == "factsheet":
        required = effective_schema["properties"]["typeSpecification"]["required"]
        if "mobileRobotKinematic" in required and "mobileRobotKinematics" not in required:
            required[required.index("mobileRobotKinematic")] = "mobileRobotKinematics"
        else:
            raise AssertionError("pinned factsheet schema discrepancy changed; review the overlay")
    Draft202012Validator(effective_schema).validate(fixture)
    print(f"VDA 5050 v3.0.0 schema-valid fixture: {fixture_name}")

print(f"validated {len(mapping)} frozen fixtures against pinned VDA v3.0.0 schema snapshot")
