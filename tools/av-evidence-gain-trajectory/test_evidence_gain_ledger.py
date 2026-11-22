import copy
import hashlib
import json
import unittest

from evidence_gain_ledger import (
    EvidenceGainError,
    STOP_ACTION_ID,
    evaluate_trajectory,
)
from evaluate_evidence_certificate import BENCHMARK_ID as EVIDENCE_BENCHMARK_ID


def digest(value):
    return hashlib.sha256(value.encode("utf-8")).hexdigest()


def canonical_digest(value):
    return hashlib.sha256(
        json.dumps(value, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode("utf-8")
    ).hexdigest()


def resource_delta(
    *,
    frames=0,
    video_bytes=0,
    tool_calls=0,
    energy=None,
    energy_provenance="NOT_MEASURED",
    energy_method=None,
    energy_digest=None,
):
    return {
        "new_frames": frames,
        "video_bytes": video_bytes,
        "audio_samples": 0,
        "audio_bytes": 0,
        "input_tokens": 10,
        "output_tokens": 5,
        "model_calls": 1,
        "tool_calls": tool_calls,
        "wall_time_ms": 20,
        "peak_ram_mb": 100,
        "peak_vram_mb": 200,
        "energy_millijoules": energy,
        "energy_provenance": energy_provenance,
        "energy_method": energy_method,
        "energy_evidence_digest_sha256": energy_digest,
    }


def truth_document():
    return {
        "benchmark_id": EVIDENCE_BENCHMARK_ID,
        "schema_version": 1,
        "episode_id": "episode-gain-01",
        "claims": [
            {
                "claim_id": "alarm",
                "answerable": True,
                "acceptable_answer_values": ["alarm_heard"],
                "minimal_sufficient_evidence_sets": [["obs-alarm"]],
                "supporting_spans": [
                    {"observation_ref": "obs-alarm", "start_ms": 10, "end_ms": 20}
                ],
                "supporting_boxes": [],
            }
        ],
    }


def workload(tick, head, catalog):
    return {
        "benchmark_id": EVIDENCE_BENCHMARK_ID,
        "schema_version": 1,
        "episode_id": "episode-gain-01",
        "transcript_head_sha256": digest(head),
        "decision_cutoff_tick": tick,
        "observation_catalog": copy.deepcopy(catalog),
    }


def submission(head, *, abstain=False, refs=None):
    evidence = []
    if refs:
        for observation_id in refs:
            evidence.append({
                "observation_ref": observation_id,
                "start_ms": 10,
                "end_ms": 20,
            })
    return {
        "benchmark_id": EVIDENCE_BENCHMARK_ID,
        "schema_version": 1,
        "episode_id": "episode-gain-01",
        "transcript_head_sha256": digest(head),
        "claims": [
            {
                "claim_id": "alarm",
                "answer_value": None if abstain else "alarm_heard",
                "abstained": abstain,
                "evidence": evidence,
            }
        ],
    }


def action_forecast(turn_id, tick, head, selected, *, include_alarm=True):
    catalogue = []
    forecasts = []
    if include_alarm:
        catalogue = [
            {"action_id": "listen-alarm", "cost_units": 0.1, "arrival_tick": tick + 1}
        ]
        forecasts = [
            {
                "action_id": "listen-alarm",
                "estimated_gross_evsi": 1.0,
                "lower_gross_evsi": 0.5,
                "upper_gross_evsi": 1.5,
            }
        ]
    forecasts.append({
        "action_id": STOP_ACTION_ID,
        "estimated_gross_evsi": 0.0,
        "lower_gross_evsi": 0.0,
        "upper_gross_evsi": 0.0,
    })
    return {
        "episode_id": "episode-gain-01",
        "turn_id": turn_id,
        "transcript_head_sha256": digest(head),
        "forecast_tick": tick,
        "deadline_tick": tick + 10,
        "budget_cost_units": 1.0,
        "action_catalogue": catalogue,
        "selected_action_id": selected,
        "forecasts": forecasts,
    }


def turn(turn_id, index, previous_action, status, new_ids, work, sub, forecast, resource=None):
    if resource is None:
        resource = resource_delta()
    receipt = {
        "benchmark_id": "av-action-space-evsi-v0",
        "episode_id": "episode-gain-01",
        "turn_id": turn_id,
        "policy_id": "A5",
        "run_id": "run-001",
        "transcript_head_sha256": forecast["transcript_head_sha256"],
        "forecast_digest_sha256": canonical_digest(forecast),
        "scorecard_sha256": digest(f"evsi-scorecard-{turn_id}"),
        "selected_action_id": forecast["selected_action_id"],
        "selected_action_feasible": True,
        "missed_best_action": False,
        "decision_regret": 0.0,
        "best_feasible_net_evsi": 1.0,
    }
    return {
        "turn_id": turn_id,
        "turn_index": index,
        "preceding_action_id": previous_action,
        "action_result_status": status,
        "new_observation_ids": list(new_ids),
        "workload": work,
        "submission": sub,
        "action_forecast": forecast,
        "action_value_receipt": receipt,
        "resource_delta": resource,
    }


def alarm_observation():
    return {
        "observation_id": "obs-alarm",
        "modality": "audio",
        "available_tick": 1,
        "capture_complete_tick": 1,
        "capture_start_ms": 0,
        "capture_end_ms": 100,
        "clock_domain_id": "microphone-clock",
        "clock_generation": 1,
        "artifact_sha256": digest("alarm-audio"),
    }


def base_trajectory(*, extra_observation=False, second_turn_status="DELIVERED"):
    obs = alarm_observation()
    initial = workload(0, "head-0", [])
    after = [obs]
    if extra_observation:
        after.append({
            "observation_id": "obs-context",
            "modality": "video",
            "available_tick": 1,
            "capture_complete_tick": 1,
            "capture_start_ms": 0,
            "capture_end_ms": 100,
            "clock_domain_id": "camera-clock",
            "clock_generation": 1,
            "artifact_sha256": digest("context-frame"),
        })
    after_workload = workload(1, "head-1", after)
    first_forecast = action_forecast("turn-0", 0, "head-0", "listen-alarm")
    second_forecast = action_forecast("turn-1", 1, "head-1", STOP_ACTION_ID, include_alarm=False)
    first = turn(
        "turn-0", 0, "__initial__", "INITIAL", [],
        initial,
        submission("head-0", abstain=True),
        first_forecast,
    )
    second = turn(
        "turn-1", 1, "listen-alarm", second_turn_status,
        ["obs-alarm", "obs-context"] if extra_observation else ["obs-alarm"],
        after_workload,
        submission("head-1", refs=["obs-alarm"]),
        second_forecast,
        resource_delta(frames=1, video_bytes=100, tool_calls=1),
    )
    return {
        "benchmark_id": "av-evidence-gain-trajectory-v0",
        "schema_version": 1,
        "trajectory_id": "trajectory-001",
        "episode_id": "episode-gain-01",
        "source_cluster_id": "source-001",
        "policy_id": "A5",
        "model_digest_sha256": digest("model"),
        "runtime_digest_sha256": digest("runtime"),
        "configuration_digest_sha256": digest("config"),
        "hardware_digest_sha256": digest("hardware"),
        "evaluator_source_catalog": [alarm_observation()],
        "truth": truth_document(),
        "turns": [first, second],
    }


class EvidenceGainLedgerTests(unittest.TestCase):
    def test_new_evidence_makes_claim_grounded_and_links_preceding_action(self):
        result = evaluate_trajectory(base_trajectory())
        self.assertEqual(result["turn_count"], 2)
        self.assertEqual(result["acquisition_count"], 1)
        transition = result["turns"][1]["transition_from_previous"]
        self.assertEqual(transition["newly_grounded_claim_ids"], ["alarm"])
        self.assertEqual(transition["new_observations_cited"], ["obs-alarm"])
        self.assertEqual(transition["new_observations_uncited"], [])
        self.assertEqual(result["newly_grounded_claim_count"], 1)
        self.assertEqual(result["turns"][1]["preceding_action_id"], "listen-alarm")

    def test_uncited_new_observation_is_reported_not_called_redundant(self):
        result = evaluate_trajectory(base_trajectory(extra_observation=True))
        transition = result["turns"][1]["transition_from_previous"]
        self.assertEqual(transition["new_observations_cited"], ["obs-alarm"])
        self.assertEqual(transition["new_observations_uncited"], ["obs-context"])
        self.assertNotIn("redundant", str(transition).lower())

    def test_final_turn_must_select_stop(self):
        doc = base_trajectory()
        doc["turns"][-1]["action_forecast"]["selected_action_id"] = "listen-alarm"
        forecast = doc["turns"][-1]["action_forecast"]
        doc["turns"][-1]["action_value_receipt"]["selected_action_id"] = "listen-alarm"
        doc["turns"][-1]["action_value_receipt"]["forecast_digest_sha256"] = canonical_digest(forecast)
        with self.assertRaises(EvidenceGainError):
            evaluate_trajectory(doc)

    def test_action_forecast_must_cover_every_action_and_stop(self):
        doc = base_trajectory()
        doc["turns"][0]["action_forecast"]["forecasts"].pop()
        forecast = doc["turns"][0]["action_forecast"]
        doc["turns"][0]["action_value_receipt"]["forecast_digest_sha256"] = canonical_digest(forecast)
        with self.assertRaises(EvidenceGainError):
            evaluate_trajectory(doc)

    def test_forecast_must_bind_exact_transcript_head(self):
        doc = base_trajectory()
        doc["turns"][0]["action_forecast"]["transcript_head_sha256"] = digest("wrong-head")
        doc["turns"][0]["action_value_receipt"]["transcript_head_sha256"] = digest("wrong-head")
        doc["turns"][0]["action_value_receipt"]["forecast_digest_sha256"] = canonical_digest(
            doc["turns"][0]["action_forecast"]
        )
        with self.assertRaises(EvidenceGainError):
            evaluate_trajectory(doc)

    def test_action_receipt_must_bind_exact_forecast(self):
        doc = base_trajectory()
        doc["turns"][0]["action_value_receipt"]["forecast_digest_sha256"] = digest("different")
        with self.assertRaises(EvidenceGainError):
            evaluate_trajectory(doc)

    def test_preceding_action_must_match_previous_selected_action(self):
        doc = base_trajectory()
        doc["turns"][1]["preceding_action_id"] = "wrong-action"
        with self.assertRaises(EvidenceGainError):
            evaluate_trajectory(doc)

    def test_catalogue_growth_must_match_delivered_observation_ids(self):
        doc = base_trajectory()
        doc["turns"][1]["new_observation_ids"] = []
        with self.assertRaises(EvidenceGainError):
            evaluate_trajectory(doc)

    def test_delivered_status_requires_new_observations(self):
        doc = base_trajectory()
        doc["turns"][1]["workload"]["observation_catalog"] = []
        doc["turns"][1]["submission"] = submission("head-1", abstain=True)
        doc["turns"][1]["new_observation_ids"] = []
        with self.assertRaises(EvidenceGainError):
            evaluate_trajectory(doc)

    def test_failed_action_cannot_add_observations(self):
        doc = base_trajectory(second_turn_status="FAILED")
        with self.assertRaises(EvidenceGainError):
            evaluate_trajectory(doc)

    def test_failed_action_without_new_observation_is_preserved(self):
        doc = base_trajectory(second_turn_status="FAILED")
        doc["turns"][1]["workload"]["observation_catalog"] = []
        doc["turns"][1]["new_observation_ids"] = []
        doc["turns"][1]["submission"] = submission("head-1", abstain=True)
        doc["turns"][1]["action_result_status"] = "FAILED"
        result = evaluate_trajectory(doc)
        self.assertEqual(result["turns"][1]["new_observation_ids"], [])
        self.assertEqual(result["newly_grounded_claim_count"], 0)

    def test_previous_observations_are_immutable(self):
        doc = base_trajectory()
        doc["turns"][1]["workload"]["observation_catalog"][0]["artifact_sha256"] = digest("mutated")
        with self.assertRaises(EvidenceGainError):
            evaluate_trajectory(doc)

    def test_catalogue_cannot_shrink(self):
        doc = base_trajectory()
        doc["turns"][1]["workload"]["observation_catalog"] = []
        doc["turns"][1]["new_observation_ids"] = []
        doc["turns"][1]["submission"] = submission("head-1", abstain=True)
        with self.assertRaises(EvidenceGainError):
            evaluate_trajectory(doc)

    def test_stop_terminates_trajectory(self):
        doc = base_trajectory()
        first_forecast = action_forecast("turn-0", 0, "head-0", STOP_ACTION_ID, include_alarm=False)
        doc["turns"][0]["action_forecast"] = first_forecast
        doc["turns"][0]["action_value_receipt"]["selected_action_id"] = STOP_ACTION_ID
        doc["turns"][0]["action_value_receipt"]["forecast_digest_sha256"] = canonical_digest(first_forecast)
        doc["turns"][1]["preceding_action_id"] = STOP_ACTION_ID
        with self.assertRaises(EvidenceGainError):
            evaluate_trajectory(doc)

    def test_energy_measured_requires_external_receipt(self):
        doc = base_trajectory()
        resource = resource_delta(frames=1, video_bytes=100, tool_calls=1)
        resource["energy_millijoules"] = 12.0
        resource["energy_provenance"] = "MEASURED"
        resource["energy_method"] = "external-meter-v1"
        resource["energy_evidence_digest_sha256"] = None
        doc["turns"][1]["resource_delta"] = resource
        with self.assertRaises(EvidenceGainError):
            evaluate_trajectory(doc)

    def test_not_measured_energy_must_remain_null(self):
        doc = base_trajectory()
        doc["turns"][1]["resource_delta"]["energy_millijoules"] = 0.0
        with self.assertRaises(EvidenceGainError):
            evaluate_trajectory(doc)

    def test_new_observation_must_arrive_after_preceding_decision(self):
        doc = base_trajectory()
        doc["turns"][1]["workload"]["observation_catalog"][0]["available_tick"] = 0
        doc["turns"][1]["workload"]["observation_catalog"][0]["capture_complete_tick"] = 0
        with self.assertRaises(EvidenceGainError):
            evaluate_trajectory(doc)

    def test_claims_must_remain_bound_to_turn_transcript(self):
        doc = base_trajectory()
        doc["turns"][1]["submission"]["transcript_head_sha256"] = digest("other")
        with self.assertRaises(EvidenceGainError):
            evaluate_trajectory(doc)

    def test_replay_is_deterministic(self):
        doc = base_trajectory()
        first = evaluate_trajectory(doc)
        second = evaluate_trajectory(doc)
        self.assertEqual(first, second)
        self.assertEqual(len(first["ledger_sha256"]), 64)


if __name__ == "__main__":
    unittest.main()
