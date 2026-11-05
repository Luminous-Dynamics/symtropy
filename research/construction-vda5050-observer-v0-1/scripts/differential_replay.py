#!/usr/bin/env python3
"""Independent CI-only VDA 5050 v3 projection oracle.

This intentionally uses Python's standard library and is not production code.
It independently parses the same exact fixture bytes and compares its normalized
projection with the Rust CLI. It does not import Rust output as its expected value.
"""
from __future__ import annotations

import argparse
import hashlib
import json
import subprocess
import tempfile
from pathlib import Path
from typing import Any


def reject_duplicate_keys(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def parse_exact_json(raw: bytes) -> dict[str, Any]:
    value = json.loads(raw.decode("utf-8"), object_pairs_hook=reject_duplicate_keys)
    if not isinstance(value, dict):
        raise ValueError("message root must be a JSON object")
    return value


def require(mapping: dict[str, Any], key: str, expected_type: type | tuple[type, ...] | None = None) -> Any:
    if key not in mapping:
        raise ValueError(f"missing required field: {key}")
    value = mapping[key]
    if expected_type is not None and not isinstance(value, expected_type):
        raise ValueError(f"invalid type for {key}")
    return value


def nonempty(value: Any, key: str) -> str:
    if not isinstance(value, str) or not value.strip():
        raise ValueError(f"{key} must be a non-empty string")
    return value


def unsigned(value: Any, key: str) -> int:
    if isinstance(value, bool) or not isinstance(value, int) or value < 0:
        raise ValueError(f"{key} must be a non-negative integer")
    return value


def project(topic: str, raw: bytes, topic_path: str) -> dict[str, Any]:
    doc = parse_exact_json(raw)
    parts = topic_path.split("/")
    if len(parts) != 5 or parts[0] != "vda5050" or parts[1] != "v3":
        raise ValueError("expected the frozen five-level v3 topic")
    manufacturer_claim, serial_claim, topic_name = parts[2], parts[3], parts[4]
    if topic_name != topic:
        raise ValueError("fixture topic disagrees with topic path")

    header_id = unsigned(require(doc, "headerId"), "headerId")
    timestamp = nonempty(require(doc, "timestamp"), "timestamp")
    version = nonempty(require(doc, "version"), "version")
    manufacturer = nonempty(require(doc, "manufacturer"), "manufacturer")
    serial_number = nonempty(require(doc, "serialNumber"), "serialNumber")
    if version != "3.0.0":
        raise ValueError("unsupported VDA version")
    if manufacturer_claim != manufacturer or serial_claim != serial_number:
        raise ValueError("topic/header identity mismatch")

    header = {
        "header_id": header_id,
        "timestamp": timestamp,
        "manufacturer": manufacturer,
        "serial_number": serial_number,
    }
    observation: dict[str, Any]

    if topic == "order":
        order_id = nonempty(require(doc, "orderId"), "orderId")
        update_id = unsigned(require(doc, "orderUpdateId"), "orderUpdateId")
        nodes = require(doc, "nodes", list)
        edges = require(doc, "edges", list)
        action_count = 0
        for node in nodes:
            if not isinstance(node, dict):
                raise ValueError("node must be an object")
            nonempty(require(node, "nodeId"), "nodeId")
            unsigned(require(node, "sequenceId"), "sequenceId")
            require(node, "released", bool)
            actions = require(node, "actions", list)
            for action in actions:
                validate_order_action(action)
                action_count += 1
        for edge in edges:
            if not isinstance(edge, dict):
                raise ValueError("edge must be an object")
            nonempty(require(edge, "edgeId"), "edgeId")
            unsigned(require(edge, "sequenceId"), "sequenceId")
            require(edge, "released", bool)
            nonempty(require(edge, "startNodeId"), "startNodeId")
            nonempty(require(edge, "endNodeId"), "endNodeId")
            actions = require(edge, "actions", list)
            for action in actions:
                validate_order_action(action)
                action_count += 1
        observation = {
            "kind": "transport_order_received",
            "order_id": order_id,
            "order_update_id": update_id,
            "node_count": len(nodes),
            "edge_count": len(edges),
            "action_count": action_count,
        }

    elif topic == "state":
        order_id = require(doc, "orderId", str)
        update_id = unsigned(require(doc, "orderUpdateId"), "orderUpdateId")
        last_node_id = require(doc, "lastNodeId", str)
        last_sequence = unsigned(require(doc, "lastNodeSequenceId"), "lastNodeSequenceId")
        nodes = project_route_items(require(doc, "nodeStates", list), "nodeId")
        edges = project_route_items(require(doc, "edgeStates", list), "edgeId")
        action_states = project_action_states(require(doc, "actionStates", list))
        instant_action_states = project_action_states(require(doc, "instantActionStates", list))
        power = require(doc, "powerSupply", dict)
        charge = require(power, "stateOfCharge")
        if isinstance(charge, bool) or not isinstance(charge, (int, float)) or not 0 <= charge <= 100:
            raise ValueError("invalid powerSupply.stateOfCharge")
        require(power, "charging", bool)
        operating_mode = nonempty(require(doc, "operatingMode"), "operatingMode")
        errors = require(doc, "errors", list)
        safety = require(doc, "safetyState", dict)
        emergency = nonempty(require(safety, "activeEmergencyStop"), "activeEmergencyStop")
        if emergency not in {"MANUAL", "REMOTE", "NONE"}:
            raise ValueError("invalid activeEmergencyStop")
        require(safety, "fieldViolation", bool)
        driving = require(doc, "driving", bool)
        requests = project_zone_requests(doc.get("zoneRequests", []))
        maps = project_maps(doc.get("maps", []))
        observation = {
            "kind": "robot_state_reported",
            "order_id": order_id,
            "order_update_id": update_id,
            "last_node_id": last_node_id,
            "last_node_sequence_id": last_sequence,
            "nodes": nodes,
            "edges": edges,
            "action_states": action_states,
            "instant_action_states": instant_action_states,
            "driving": driving,
            "operating_mode": operating_mode,
            "error_count": len(errors),
            "power_supply_reported": True,
            "zone_requests": requests,
            "navigation_maps": maps,
            "safety_state_reported": True,
        }

    elif topic == "factsheet":
        type_spec = require(doc, "typeSpecification", dict)
        nonempty(require(type_spec, "seriesName"), "seriesName")
        nonempty(require(type_spec, "mobileRobotKinematics"), "mobileRobotKinematics")
        nonempty(require(type_spec, "mobileRobotClass"), "mobileRobotClass")
        require(type_spec, "maximumLoadMass", (int, float))
        require(type_spec, "localizationTypes", list)
        require(type_spec, "navigationTypes", list)
        physical = require(doc, "physicalParameters", dict)
        for key in ("minimumSpeed", "maximumSpeed", "maximumAcceleration", "maximumDeceleration",
                    "minimumHeight", "maximumHeight", "width", "length"):
            require(physical, key, (int, float))
        limits = require(doc, "protocolLimits", dict)
        require(limits, "maximumStringLengths", dict)
        require(limits, "maximumArrayLengths", dict)
        timing = require(limits, "timing", dict)
        require(timing, "minimumOrderInterval", (int, float))
        require(timing, "minimumStateInterval", (int, float))
        features = require(doc, "protocolFeatures", dict)
        require(features, "optionalParameters", list)
        require(features, "mobileRobotActions", list)
        require(doc, "mobileRobotGeometry", dict)
        require(doc, "loadSpecification", dict)
        observation = {
            "kind": "factsheet_capability_claim_reported",
            "capability_payload_sha256": hashlib.sha256(raw).hexdigest(),
            "type_specification_present": True,
        }

    elif topic == "zoneSet":
        zone_set = require(doc, "zoneSet", dict)
        map_id = nonempty(require(zone_set, "mapId"), "mapId")
        zone_set_id = nonempty(require(zone_set, "zoneSetId"), "zoneSetId")
        zones = []
        for zone in require(zone_set, "zones", list):
            zone_id = nonempty(require(zone, "zoneId"), "zoneId")
            zone_type = nonempty(require(zone, "zoneType"), "zoneType")
            if zone_type not in {"BLOCKED", "LINE_GUIDED", "RELEASE", "COORDINATED_REPLANNING",
                                 "SPEED_LIMIT", "ACTION", "PRIORITY", "PENALTY", "DIRECTED", "BIDIRECTED"}:
                raise ValueError("invalid zoneType")
            vertices = require(zone, "vertices", list)
            if len(vertices) < 3:
                raise ValueError("zone needs at least three vertices")
            for vertex in vertices:
                require(vertex, "x", (int, float))
                require(vertex, "y", (int, float))
            zones.append({"zone_id": zone_id, "zone_type": zone_type})
        observation = {
            "kind": "fleet_zone_set_reported",
            "map_id": map_id,
            "zone_set_id": zone_set_id,
            "zones": zones,
        }

    elif topic == "connection":
        state = nonempty(require(doc, "connectionState"), "connectionState")
        if state not in {"ONLINE", "OFFLINE", "HIBERNATING", "CONNECTION_BROKEN"}:
            raise ValueError("invalid connectionState")
        observation = {"kind": "robot_connection_state_reported", "connection_state": state}

    elif topic == "instantActions":
        actions = []
        for action in require(doc, "actions", list):
            action_id = nonempty(require(action, "actionId"), "actionId")
            action_type = nonempty(require(action, "actionType"), "actionType")
            blocking = nonempty(require(action, "blockingType"), "blockingType")
            if blocking != "NONE":
                raise ValueError("instant action blockingType must be NONE")
            actions.append({"action_id": action_id, "action_type": action_type, "blocking_type": blocking})
        observation = {"kind": "instant_actions_reported", "actions": actions}

    elif topic == "responses":
        responses = []
        for response in require(doc, "responses", list):
            request_id = nonempty(require(response, "requestId"), "requestId")
            grant = nonempty(require(response, "grantType"), "grantType")
            if grant not in {"GRANTED", "QUEUED", "REVOKED", "REJECTED"}:
                raise ValueError("invalid grantType")
            responses.append({"request_id": request_id, "grant_type": grant})
        observation = {"kind": "fleet_zone_responses_reported", "responses": responses}

    else:
        raise ValueError(f"unsupported topic: {topic}")

    return {
        "protocol": "VDA-5050",
        "version": version,
        "topic": topic,
        "header": header,
        "topic_identity": {
            "manufacturer": manufacturer_claim,
            "serial_number": serial_claim,
        },
        "payload_sha256": hashlib.sha256(raw).hexdigest(),
        "observation": observation,
    }


def validate_order_action(action: Any) -> None:
    if not isinstance(action, dict):
        raise ValueError("order action must be an object")
    nonempty(require(action, "actionId"), "actionId")
    nonempty(require(action, "actionType"), "actionType")
    blocking = nonempty(require(action, "blockingType"), "blockingType")
    if blocking not in {"NONE", "SOFT", "SINGLE", "HARD"}:
        raise ValueError("invalid order action blockingType")


def project_route_items(values: list[Any], id_field: str) -> list[dict[str, Any]]:
    result = []
    for value in values:
        identifier = nonempty(require(value, id_field), id_field)
        sequence = unsigned(require(value, "sequenceId"), "sequenceId")
        released = require(value, "released", bool)
        result.append({
            "node_id" if id_field == "nodeId" else "edge_id": identifier,
            "sequence_id": sequence,
            "released": released,
        })
    return result


def project_action_states(values: list[Any]) -> list[dict[str, Any]]:
    result = []
    for value in values:
        action_id = nonempty(require(value, "actionId"), "actionId")
        status = nonempty(require(value, "actionStatus"), "actionStatus")
        if status not in {"WAITING", "INITIALIZING", "RUNNING", "PAUSED", "RETRIABLE", "FINISHED", "FAILED"}:
            raise ValueError("invalid actionStatus")
        result.append({
            "action_id": action_id,
            "action_type": value.get("actionType"),
            "status": status,
        })
    return result


def project_zone_requests(values: Any) -> list[dict[str, Any]]:
    if not isinstance(values, list):
        raise ValueError("zoneRequests must be an array")
    result = []
    for value in values:
        request_type = nonempty(require(value, "requestType"), "requestType")
        status = nonempty(require(value, "requestStatus"), "requestStatus")
        if request_type not in {"ACCESS", "REPLANNING"}:
            raise ValueError("invalid zone request type")
        if status not in {"REQUESTED", "GRANTED", "REVOKED", "EXPIRED"}:
            raise ValueError("invalid zone request status")
        result.append({
            "request_id": nonempty(require(value, "requestId"), "requestId"),
            "request_type": request_type,
            "zone_id": nonempty(require(value, "zoneId"), "zoneId"),
            "zone_set_id": nonempty(require(value, "zoneSetId"), "zoneSetId"),
            "request_status": status,
        })
    return result


def project_maps(values: Any) -> list[dict[str, Any]]:
    if not isinstance(values, list):
        raise ValueError("maps must be an array")
    result = []
    for value in values:
        map_version = nonempty(require(value, "mapVersion"), "mapVersion")
        map_status = nonempty(require(value, "mapStatus"), "mapStatus")
        if map_status not in {"ENABLED", "DISABLED"}:
            raise ValueError("invalid mapStatus")
        result.append({
            "map_id": nonempty(require(value, "mapId"), "mapId"),
            "map_version": map_version,
            "map_status": map_status,
        })
    return result


def expect_rejected(binary: Path, topic_path: str, raw: bytes, identity: tuple[str, str]) -> None:
    with tempfile.NamedTemporaryFile(prefix="vda-diff-", suffix=".json") as handle:
        handle.write(raw)
        handle.flush()
        result = subprocess.run(
            [str(binary), "decode", topic_path, handle.name,
             "--manufacturer", identity[0], "--serial-number", identity[1]],
            capture_output=True,
            text=True,
            check=False,
        )
        if result.returncode == 0:
            raise AssertionError(f"Rust accepted a negative control: {topic_path}")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", required=True, type=Path)
    parser.add_argument("--fixtures", required=True, type=Path)
    args = parser.parse_args()
    binary = args.binary.resolve()
    if not binary.exists():
        raise FileNotFoundError(binary)

    fixtures = [
        ("order", "order.json"),
        ("state", "state-action-finished.json"),
        ("factsheet", "factsheet.json"),
        ("zoneSet", "zoneSet.json"),
        ("connection", "connection.json"),
        ("instantActions", "instantActions.json"),
        ("responses", "responses.json"),
    ]
    identity = ("ACME", "R-01")
    for topic, filename in fixtures:
        path = args.fixtures / filename
        raw = path.read_bytes()
        topic_path = f"vda5050/v3/{identity[0]}/{identity[1]}/{topic}"
        expected = project(topic, raw, topic_path)
        result = subprocess.run(
            [str(binary), "decode", topic_path, str(path),
             "--manufacturer", identity[0], "--serial-number", identity[1]],
            capture_output=True,
            text=True,
            check=False,
        )
        if result.returncode != 0:
            raise AssertionError(f"Rust observer rejected valid {topic}: {result.stderr}")
        actual = json.loads(result.stdout)
        if actual != expected:
            raise AssertionError(
                f"differential mismatch for {topic} (sha256={hashlib.sha256(raw).hexdigest()})\n"
                f"oracle={json.dumps(expected, sort_keys=True)}\n"
                f"rust={json.dumps(actual, sort_keys=True)}"
            )
        print(f"equal: {topic} sha256={hashlib.sha256(raw).hexdigest()}")

        # JSON whitespace is semantically irrelevant, but raw-byte integrity must change.
        padded = b" " + raw
        padded_expected = project(topic, padded, topic_path)
        with tempfile.NamedTemporaryFile(prefix="vda-diff-space-", suffix=".json") as handle:
            handle.write(padded)
            handle.flush()
            spaced = subprocess.run(
                [str(binary), "decode", topic_path, handle.name,
                 "--manufacturer", identity[0], "--serial-number", identity[1]],
                capture_output=True,
                text=True,
                check=True,
            )
        if json.loads(spaced.stdout) != padded_expected:
            raise AssertionError(f"whitespace metamorphic mismatch for {topic}")
        if padded_expected["payload_sha256"] == expected["payload_sha256"]:
            raise AssertionError("whitespace-only byte change did not change raw payload digest")

    # Negative controls: wrong topic, identity, duplicate key and illegal action status.
    connection = (args.fixtures / "connection.json").read_bytes()
    expect_rejected(binary, "vda5050/v3/ACME/R-01/order", connection, identity)

    connection_doc = parse_exact_json(connection)
    mismatch = "vda5050/v3/OTHER/R-01/connection"
    with tempfile.NamedTemporaryFile(prefix="vda-diff-identity-", suffix=".json") as handle:
        handle.write(connection)
        handle.flush()
        result = subprocess.run(
            [str(binary), "decode", mismatch, handle.name],
            capture_output=True, text=True, check=False,
        )
        if result.returncode == 0:
            raise AssertionError("Rust accepted topic/header identity mismatch")

    duplicate = b'{"headerId":1,"headerId":2}'
    expect_rejected(binary, "vda5050/v3/ACME/R-01/connection", duplicate, identity)

    state = (args.fixtures / "state-action-finished.json").read_bytes()
    state_text = state.decode("utf-8").replace('"FINISHED"', '"ACCEPTED"')
    expect_rejected(binary, "vda5050/v3/ACME/R-01/state", state_text.encode("utf-8"), identity)

    print(f"differential replay passed for {len(fixtures)} exact-byte VDA messages and 4 negative controls")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
