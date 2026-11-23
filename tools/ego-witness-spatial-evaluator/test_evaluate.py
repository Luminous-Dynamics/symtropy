import copy
import importlib.util
import json
import unittest
from pathlib import Path

MODULE_PATH=Path(__file__).with_name("evaluate.py")
spec=importlib.util.spec_from_file_location("ego_witness_evaluator",MODULE_PATH)
evaluator=importlib.util.module_from_spec(spec)
assert spec.loader is not None
spec.loader.exec_module(evaluator)

ROOT=Path(__file__).resolve().parents[2]
BASE=ROOT/"research"/"ego-witness-spatial-memory-v0"/"fixtures"
WORKLOAD=json.loads((BASE/"agent"/"workload-v1.json").read_text(encoding="utf-8"))
TRUTH=json.loads((BASE/"evaluator-only"/"truth-v1.json").read_text(encoding="utf-8"))
SUBMISSION=json.loads((BASE/"agent"/"reference-submission-v1.json").read_text(encoding="utf-8"))


class EgoWitnessSpatialEvaluatorTests(unittest.TestCase):
    def setUp(self):
        self.workload=copy.deepcopy(WORKLOAD)
        self.truth=copy.deepcopy(TRUTH)
        self.submission=copy.deepcopy(SUBMISSION)

    def prediction(self,episode_id):
        return next(row for row in self.submission["predictions"] if row["episode_id"]==episode_id)

    def result(self,score,episode_id):
        return next(row for row in score["per_episode"] if row["episode_id"]==episode_id)

    def test_reference_policy_matches_all_synthetic_known_answers(self):
        score=evaluator.evaluate(self.workload,self.truth,self.submission)
        totals=score["totals"]
        self.assertEqual(totals["episode_count"],4)
        self.assertEqual(totals["answer_correct_count"],4)
        self.assertEqual(totals["decision_correct_count"],4)
        self.assertEqual(totals["instance_pair_count"],12)
        self.assertEqual(totals["instance_pair_correct_count"],12)
        self.assertEqual(totals["false_merge_count"],0)
        self.assertEqual(totals["false_split_count"],0)
        self.assertEqual(totals["room_pair_accuracy"],1.0)
        self.assertEqual(totals["last_seen_accuracy"],1.0)
        self.assertEqual(totals["evidence_qualified_answer_accuracy"],1.0)
        self.assertEqual(totals["hard_violation_count"],0)

    def test_same_category_lookalike_is_not_same_instance(self):
        row=self.prediction("ew-001-cross-day-instance-reidentification")
        pair=next(x for x in row["instance_pair_predictions"]
                  if {x["left_observation_ref"],x["right_observation_ref"]}==
                  {"obs-ew1-anchor","obs-ew1-lookalike"})
        pair["same_instance"]=True
        score=evaluator.evaluate(self.workload,self.truth,self.submission)
        self.assertEqual(score["totals"]["false_merge_count"],1)
        self.assertLess(score["totals"]["instance_pair_precision"],1.0)

    def test_same_instance_across_days_must_not_be_split(self):
        row=self.prediction("ew-001-cross-day-instance-reidentification")
        pair=next(x for x in row["instance_pair_predictions"]
                  if {x["left_observation_ref"],x["right_observation_ref"]}==
                  {"obs-ew1-anchor","obs-ew1-same-instance"})
        pair["same_instance"]=False
        score=evaluator.evaluate(self.workload,self.truth,self.submission)
        self.assertEqual(score["totals"]["false_split_count"],1)
        self.assertLess(score["totals"]["instance_pair_recall"],1.0)

    def test_last_seen_must_select_latest_observation_of_same_instance(self):
        self.prediction("ew-002-last-seen-wearer-attribution")["last_seen_prediction"]["selected_observation_ref"]="obs-ew2-lookalike-ada"
        score=evaluator.evaluate(self.workload,self.truth,self.submission)
        result=self.result(score,"ew-002-last-seen-wearer-attribution")
        self.assertIn("last_seen_not_latest_eligible_instance_observation",result["violations"])
        self.assertFalse(result["last_seen_correct"])

    def test_last_seen_wearer_must_match_selected_observation(self):
        self.prediction("ew-002-last-seen-wearer-attribution")["last_seen_prediction"]["wearer_ref"]="wearer-bob"
        score=evaluator.evaluate(self.workload,self.truth,self.submission)
        result=self.result(score,"ew-002-last-seen-wearer-attribution")
        self.assertIn("last_seen_wearer_attribution_mismatch",result["violations"])
        self.assertEqual(score["totals"]["false_wearer_attribution_count"],1)

    def test_distinct_room_views_may_resolve_to_same_room(self):
        score=evaluator.evaluate(self.workload,self.truth,self.submission)
        self.assertEqual(score["totals"]["room_pair_accuracy"],1.0)

    def test_same_category_different_toolboxes_do_not_merge(self):
        row=self.prediction("ew-004-same-category-distinct-instances")
        pair=next(x for x in row["instance_pair_predictions"]
                  if {x["left_observation_ref"],x["right_observation_ref"]}==
                  {"obs-ew4-toolbox-a","obs-ew4-toolbox-b"})
        self.assertFalse(pair["same_instance"])
        score=evaluator.evaluate(self.workload,self.truth,self.submission)
        self.assertEqual(score["totals"]["false_merge_count"],0)

    def test_unknown_observation_citation_is_rejected(self):
        row=self.prediction("ew-001-cross-day-instance-reidentification")
        row["cited_observation_refs"].append("obs-not-in-this-episode")
        score=evaluator.evaluate(self.workload,self.truth,self.submission)
        result=self.result(score,"ew-001-cross-day-instance-reidentification")
        self.assertIn("citation_not_in_episode_or_not_admitted",result["violations"])

    def test_as_known_cutoff_must_equal_query_tick(self):
        self.prediction("ew-001-cross-day-instance-reidentification")["evidence_as_of_tick"]=0
        score=evaluator.evaluate(self.workload,self.truth,self.submission)
        result=self.result(score,"ew-001-cross-day-instance-reidentification")
        self.assertIn("evidence_cutoff_must_equal_query_tick",result["violations"])

    def test_missing_required_observation_reduces_provenance(self):
        row=self.prediction("ew-001-cross-day-instance-reidentification")
        row["cited_observation_refs"].remove("obs-ew1-lookalike")
        score=evaluator.evaluate(self.workload,self.truth,self.submission)
        result=self.result(score,"ew-001-cross-day-instance-reidentification")
        self.assertIn("required_source_citation_missing",result["violations"])
        self.assertFalse(result["provenance_complete"])

    def test_room_alias_collision_is_a_false_room_merge(self):
        row=self.prediction("ew-003-room-anchor-identity")
        pair=next(x for x in row["room_pair_predictions"]
                  if {x["left_observation_ref"],x["right_observation_ref"]}==
                  {"obs-ew3-room-living-a","obs-ew3-room-kitchen"})
        pair["same_room"]=True
        score=evaluator.evaluate(self.workload,self.truth,self.submission)
        self.assertEqual(score["totals"]["room_pair_accuracy"],2/3)

    def test_candidate_workload_contains_no_canonical_truth_fields(self):
        forbidden={"instance_identity_by_observation","room_identity_by_observation",
                   "same_instance","same_room","expected_observation_ref","expected_wearer_ref"}
        def scan(value):
            if isinstance(value,dict):
                self.assertFalse(forbidden & set(value))
                for child in value.values(): scan(child)
            elif isinstance(value,list):
                for child in value: scan(child)
        scan(self.workload)


if __name__=="__main__":
    unittest.main()
