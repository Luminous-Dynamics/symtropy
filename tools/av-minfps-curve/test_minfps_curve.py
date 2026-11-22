import hashlib
import json
import unittest

from evaluate_minfps_curve import (
    BENCHMARK_ID,
    MinFpsEvaluationError,
    evaluate_document,
)


RATES = [1.0, 2.0, 4.0]


def digest(value):
    return hashlib.sha256(value.encode("utf-8")).hexdigest()


def resource(
    frames,
    *,
    energy=None,
    energy_provenance="NOT_MEASURED",
    energy_method=None,
    energy_digest=None,
):
    return {
        "frames_delivered": frames,
        "video_bytes": 100 * frames,
        "audio_samples": 16000,
        "audio_bytes": 32000,
        "input_tokens": 120,
        "output_tokens": 20,
        "model_calls": 1,
        "wall_time_ms": 250,
        "peak_ram_mb": 1000,
        "peak_vram_mb": 2000,
        "energy_millijoules": energy,
        "energy_provenance": energy_provenance,
        "energy_method": energy_method,
        "energy_evidence_digest_sha256": energy_digest,
    }


def rate_run(fps, frames, grounded, status="COMPLETED", energy_resource=None):
    frame_manifest = [
        {
            "frame_id": f"frame-{int(round(index * 1000 / fps))}",
            "timestamp_ms": float(index * 1000 / fps),
        }
        for index in range(frames)
    ]
    if energy_resource is None:
        energy_resource = resource(frames)
    return {
        "fps": fps,
        "run_status": status,
        "frame_manifest": frame_manifest,
        "frame_manifest_sha256": hashlib.sha256(
            json.dumps(
                frame_manifest, sort_keys=True, separators=(",", ":"), ensure_ascii=False
            ).encode("utf-8")
        ).hexdigest(),
        "grounded_success": grounded if status == "COMPLETED" else None,
        "answer_correct": grounded if status == "COMPLETED" else None,
        "evidence_sufficient": grounded if status == "COMPLETED" else None,
        "decision_on_time": True if status == "COMPLETED" else None,
        "certificate_scorecard_sha256": digest(f"scorecard-{fps}") if status == "COMPLETED" else None,
        "resource": energy_resource,
    }


def document(success_by_rate, *, seeds=("seed-1",), minimum_success_fraction=1.0,
             minimum_seed_count=1, statuses=None):
    all_seeds = []
    for seed in seeds:
        runs = []
        for index, rate in enumerate(RATES):
            status = statuses.get((seed, rate), "COMPLETED") if statuses else "COMPLETED"
            runs.append(rate_run(
                rate,
                int(round(rate)),
                success_by_rate[seed][rate],
                status=status,
            ))
        all_seeds.append({"seed_id": seed, "runs": runs})
    return {
        "benchmark_id": BENCHMARK_ID,
        "schema_version": 1,
        "protocol_id": "MINFPS-TEST-v1",
        "episode_id": "episode-01",
        "source_cluster_id": "source-01",
        "policy_id": "A5",
        "workload_digest_sha256": digest("episode-workload-v1"),
        "model_digest_sha256": digest("model-v1"),
        "runtime_digest_sha256": digest("runtime-v1"),
        "configuration_digest_sha256": digest("model-config-v1"),
        "hardware_digest_sha256": digest("hardware-v1"),
        "sampler_id": "nested-uniform-v1",
        "sampler_digest_sha256": digest("sampler-v1"),
        "fps_grid": list(RATES),
        "sampling_design": "NESTED_FIXED_RATE",
        "media_duration_ms": 1000.0,
        "sampling_tolerance_ms": 1.0,
        "minimum_success_fraction": minimum_success_fraction,
        "minimum_seed_count": minimum_seed_count,
        "seeds": all_seeds,
    }


class MinFpsCurveTests(unittest.TestCase):
    def test_threshold_found_at_first_stable_success_suffix(self):
        doc = document({"seed-1": {1.0: False, 2.0: False, 4.0: True}})
        result = evaluate_document(doc)
        self.assertEqual(result["minimum_fps"]["status"], "MIN_FPS_FOUND")
        self.assertEqual(result["minimum_fps"]["min_fps"], 4.0)

    def test_all_rates_success_is_left_censored(self):
        doc = document({"seed-1": {1.0: True, 2.0: True, 4.0: True}})
        result = evaluate_document(doc)
        self.assertEqual(result["minimum_fps"]["status"], "LEFT_CENSORED")
        self.assertEqual(result["minimum_fps"]["min_fps"], 1.0)

    def test_all_rates_fail_is_right_censored(self):
        doc = document({"seed-1": {1.0: False, 2.0: False, 4.0: False}})
        result = evaluate_document(doc)
        self.assertEqual(result["minimum_fps"]["status"], "RIGHT_CENSORED")
        self.assertIsNone(result["minimum_fps"]["min_fps"])

    def test_non_monotone_success_without_passing_suffix_is_no_stable_rate(self):
        doc = document({"seed-1": {1.0: False, 2.0: True, 4.0: False}})
        result = evaluate_document(doc)
        self.assertEqual(result["minimum_fps"]["status"], "NO_STABLE_RATE")
        self.assertIsNone(result["minimum_fps"]["min_fps"])

    def test_lower_rate_anomaly_does_not_override_higher_stable_threshold(self):
        doc = document({"seed-1": {1.0: True, 2.0: False, 4.0: True}})
        result = evaluate_document(doc)
        self.assertEqual(result["minimum_fps"]["status"], "MIN_FPS_FOUND")
        self.assertEqual(result["minimum_fps"]["min_fps"], 4.0)

    def test_minimum_success_fraction_is_preregistered_across_seeds(self):
        successes = {
            "s1": {1.0: False, 2.0: True, 4.0: True},
            "s2": {1.0: False, 2.0: True, 4.0: True},
            "s3": {1.0: True, 2.0: False, 4.0: True},
        }
        result = evaluate_document(document(
            successes,
            seeds=("s1", "s2", "s3"),
            minimum_success_fraction=2 / 3,
            minimum_seed_count=3,
        ))
        self.assertEqual(result["minimum_fps"]["status"], "MIN_FPS_FOUND")
        self.assertEqual(result["minimum_fps"]["min_fps"], 2.0)

    def test_failed_attempts_remain_in_denominator(self):
        doc = document(
            {"seed-1": {1.0: True, 2.0: True, 4.0: True}},
            statuses={("seed-1", 2.0): "TIMEOUT"},
        )
        result = evaluate_document(doc)
        self.assertEqual(result["attempted_rate_runs"], 3)
        self.assertEqual(result["failed_rate_runs"], 1)
        self.assertEqual(result["rate_summaries"][1]["seeds"], 1)
        self.assertEqual(result["rate_summaries"][1]["grounded_success_fraction"], 0.0)

    def test_answer_correct_without_grounding_does_not_succeed(self):
        doc = document({"seed-1": {1.0: True, 2.0: True, 4.0: True}})
        doc["seeds"][0]["runs"][0]["grounded_success"] = False
        doc["seeds"][0]["runs"][0]["answer_correct"] = True
        doc["seeds"][0]["runs"][0]["evidence_sufficient"] = False
        result = evaluate_document(doc)
        self.assertFalse(result["rate_summaries"][0]["meets_success_threshold"])

    def test_frame_sets_must_be_nested(self):
        doc = document({"seed-1": {1.0: True, 2.0: True, 4.0: True}})
        doc["seeds"][0]["runs"][1]["frame_manifest"][0]["frame_id"] = "different-frame"
        runs = doc["seeds"][0]["runs"]
        runs[1]["frame_manifest_sha256"] = hashlib.sha256(
            json.dumps(runs[1]["frame_manifest"], sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode()
        ).hexdigest()
        with self.assertRaises(MinFpsEvaluationError):
            evaluate_document(doc)

    def test_frame_manifest_timestamp_order_is_enforced(self):
        doc = document({"seed-1": {1.0: True, 2.0: True, 4.0: True}})
        frames = doc["seeds"][0]["runs"][1]["frame_manifest"]
        frames[1]["timestamp_ms"] = frames[0]["timestamp_ms"]
        doc["seeds"][0]["runs"][1]["frame_manifest_sha256"] = hashlib.sha256(
            json.dumps(frames, sort_keys=True, separators=(",", ":"), ensure_ascii=False).encode()
        ).hexdigest()
        with self.assertRaises(MinFpsEvaluationError):
            evaluate_document(doc)

    def test_duplicate_seed_and_missing_rate_rows_are_rejected(self):
        doc = document({"seed-1": {1.0: True, 2.0: True, 4.0: True}})
        doc["seeds"].append(dict(doc["seeds"][0]))
        with self.assertRaises(MinFpsEvaluationError):
            evaluate_document(doc)
        doc = document({"seed-1": {1.0: True, 2.0: True, 4.0: True}})
        doc["seeds"][0]["runs"].pop()
        with self.assertRaises(MinFpsEvaluationError):
            evaluate_document(doc)

    def test_measured_energy_requires_method_and_evidence_digest(self):
        doc = document({"seed-1": {1.0: True, 2.0: True, 4.0: True}})
        resource_row = resource(
            1,
            energy=10.0,
            energy_provenance="MEASURED",
            energy_method="external-meter-v1",
            energy_digest=digest("metered-evidence"),
        )
        doc["seeds"][0]["runs"][0]["resource"] = resource_row
        result = evaluate_document(doc)
        self.assertEqual(result["rate_summaries"][0]["measured_energy_sample_count"], 1)
        self.assertEqual(result["rate_summaries"][0]["measured_energy_millijoules_mean"], 10.0)

    def test_not_measured_energy_cannot_be_zero(self):
        doc = document({"seed-1": {1.0: True, 2.0: True, 4.0: True}})
        doc["seeds"][0]["runs"][0]["resource"]["energy_millijoules"] = 0.0
        with self.assertRaises(MinFpsEvaluationError):
            evaluate_document(doc)


    def test_partial_failed_attempt_is_retained_without_fabricated_frames(self):
        doc = document({"seed-1": {1.0: True, 2.0: True, 4.0: True}})
        failed = doc["seeds"][0]["runs"][1]
        failed["run_status"] = "TIMEOUT"
        failed["frame_manifest"] = [
            {"frame_id": "frame-0", "timestamp_ms": 0.0}
        ]
        failed["frame_manifest_sha256"] = hashlib.sha256(
            json.dumps(
                failed["frame_manifest"],
                sort_keys=True,
                separators=(",", ":"),
                ensure_ascii=False,
            ).encode()
        ).hexdigest()
        failed["grounded_success"] = None
        failed["answer_correct"] = None
        failed["evidence_sufficient"] = None
        failed["decision_on_time"] = None
        failed["certificate_scorecard_sha256"] = None
        failed["resource"]["frames_delivered"] = 1
        result = evaluate_document(doc)
        self.assertEqual(result["failed_rate_runs"], 1)
        self.assertEqual(result["attempted_rate_runs"], 3)
        self.assertEqual(result["rate_summaries"][1]["grounded_success_fraction"], 0.0)

    def test_inconsistent_frames_delivered_count_is_rejected(self):
        doc = document({"seed-1": {1.0: True, 2.0: True, 4.0: True}})
        doc["seeds"][0]["runs"][1]["resource"]["frames_delivered"] = 99
        with self.assertRaises(MinFpsEvaluationError):
            evaluate_document(doc)


    def test_shared_frame_ids_must_keep_same_timestamps(self):
        doc = document({"seed-1": {1.0: True, 2.0: True, 4.0: True}})
        doc["sampling_tolerance_ms"] = 20.0
        low = doc["seeds"][0]["runs"][0]["frame_manifest"][0]
        high = doc["seeds"][0]["runs"][1]["frame_manifest"][0]
        high["timestamp_ms"] = 10.0
        doc["seeds"][0]["runs"][1]["frame_manifest_sha256"] = hashlib.sha256(
            json.dumps(
                doc["seeds"][0]["runs"][1]["frame_manifest"],
                sort_keys=True,
                separators=(",", ":"),
                ensure_ascii=False,
            ).encode()
        ).hexdigest()
        with self.assertRaises(MinFpsEvaluationError):
            evaluate_document(doc)

    def test_fixed_rate_cadence_must_match_declared_fps(self):
        doc = document({"seed-1": {1.0: True, 2.0: True, 4.0: True}})
        frames = doc["seeds"][0]["runs"][2]["frame_manifest"]
        frames[1]["timestamp_ms"] = 300.0
        doc["seeds"][0]["runs"][2]["frame_manifest_sha256"] = hashlib.sha256(
            json.dumps(
                frames,
                sort_keys=True,
                separators=(",", ":"),
                ensure_ascii=False,
            ).encode()
        ).hexdigest()
        with self.assertRaises(MinFpsEvaluationError):
            evaluate_document(doc)


    def test_official_independent_rate_grid_is_allowed_when_declared(self):
        doc = document({"seed-1": {1.0: False, 2.0: False, 4.0: True}})
        doc["fps_grid"] = [1.0, 2.0, 7.0]
        doc["sampling_design"] = "INDEPENDENT_FIXED_RATE"
        doc["seeds"][0]["runs"][2] = rate_run(7.0, 7, True)
        result = evaluate_document(doc)
        self.assertEqual(result["minimum_fps"]["status"], "MIN_FPS_FOUND")
        self.assertEqual(result["minimum_fps"]["min_fps"], 7.0)

    def test_non_nested_rate_grid_is_rejected_when_nested_design_declared(self):
        doc = document({"seed-1": {1.0: False, 2.0: False, 4.0: True}})
        doc["fps_grid"] = [1.0, 2.0, 7.0]
        doc["seeds"][0]["runs"][2] = rate_run(7.0, 7, True)
        with self.assertRaises(MinFpsEvaluationError):
            evaluate_document(doc)

    def test_invalid_fps_grid_is_rejected(self):
        doc = document({"seed-1": {1.0: True, 2.0: True, 4.0: True}})
        doc["fps_grid"] = [1.0, 1.0, 2.0]
        with self.assertRaises(MinFpsEvaluationError):
            evaluate_document(doc)

    def test_measured_energy_and_other_resource_metrics_stay_separate(self):
        doc = document({"seed-1": {1.0: True, 2.0: True, 4.0: True}})
        doc["seeds"][0]["runs"][0]["resource"] = resource(
            1,
            energy=10.0,
            energy_provenance="MEASURED",
            energy_method="meter-v1",
            energy_digest=digest("evidence"),
        )
        result = evaluate_document(doc)
        self.assertEqual(result["rate_summaries"][0]["measured_energy_sample_count"], 1)
        self.assertEqual(result["rate_summaries"][1]["measured_energy_sample_count"], 0)


if __name__ == "__main__":
    unittest.main()
