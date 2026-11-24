import copy
import unittest
from fractions import Fraction
from pathlib import Path

from cvar_regret_oracle import cvar_upper_tail, evaluate_case, fraction_json

FIXTURE = Path(__file__).resolve().parents[1] / "fixtures" / "v0.1" / "cvar-regret-cases.json"


def load_case():
    import json
    corpus = json.loads(FIXTURE.read_text(encoding="utf-8"))
    return corpus, corpus["cases"][0]


class CvarArithmeticTests(unittest.TestCase):
    def test_tail_smaller_than_one_atom_uses_fraction_of_var_atom(self):
        result = cvar_upper_tail([
            (Fraction(0), Fraction(9, 10)),
            (Fraction(10), Fraction(1, 10)),
        ], Fraction(19, 20))
        self.assertEqual(result["var"], Fraction(10))
        self.assertEqual(result["cvar"], Fraction(10))
        self.assertEqual(result["tail_mass"], Fraction(1, 20))
        self.assertEqual(result["var_atom_fraction"], Fraction(1, 2))

    def test_partial_var_atom_is_integrated_exactly(self):
        result = cvar_upper_tail([
            (Fraction(0), Fraction(7, 10)),
            (Fraction(3), Fraction(1, 5)),
            (Fraction(8), Fraction(1, 10)),
        ], Fraction(9, 10))
        self.assertEqual(result["var"], Fraction(3))
        self.assertEqual(result["cvar"], Fraction(8))
        self.assertEqual(result["var_atom_fraction"], Fraction(1, 2))

    def test_zero_mass_scenario_does_not_change_tail_risk(self):
        result = cvar_upper_tail([
            (Fraction(2), Fraction(1)),
            (Fraction(1000), Fraction(0)),
        ], Fraction(9, 10))
        self.assertEqual(result["var"], Fraction(2))
        self.assertEqual(result["cvar"], Fraction(2))

    def test_tied_losses_preserve_exact_tail_mass(self):
        result = cvar_upper_tail([
            (Fraction(4), Fraction(1, 4)),
            (Fraction(4), Fraction(3, 4)),
        ], Fraction(99, 100))
        self.assertEqual(result["var"], Fraction(4))
        self.assertEqual(result["cvar"], Fraction(4))
        self.assertEqual(result["var_atom_fraction"], Fraction(1, 100))

    def test_non_normalized_weights_are_rejected(self):
        with self.assertRaisesRegex(ValueError, "normalized"):
            cvar_upper_tail([
                (Fraction(1), Fraction(1, 2)),
                (Fraction(2), Fraction(1, 4)),
            ], Fraction(9, 10))

    def test_alpha_must_be_strictly_between_zero_and_one(self):
        with self.assertRaisesRegex(ValueError, "strictly between"):
            cvar_upper_tail([(Fraction(1), Fraction(1))], Fraction(1))


class ProbabilityReceiptTests(unittest.TestCase):
    def test_qualified_receipt_computes_exact_expected_cvars(self):
        _, case = load_case()
        result = evaluate_case(case, [Fraction(9, 10), Fraction(19, 20), Fraction(99, 100)])
        self.assertEqual(result["status"], "PASS")
        self.assertFalse(result["authority_granted"])
        by_id = {item["policy_id"]: item for item in result["policies"]}
        incumbent = by_id["incumbent-static"]["metrics"]["wait_and_see_regret"]
        candidate = by_id["candidate-observable"]["metrics"]["wait_and_see_regret"]
        self.assertEqual(incumbent["expected_regret"], fraction_json(Fraction(16, 25)))
        self.assertEqual(incumbent["worst_case_regret"], fraction_json(Fraction(10)))
        self.assertEqual(
            [item["cvar"] for item in incumbent["cvar"]],
            [fraction_json(Fraction(22, 5)), fraction_json(Fraction(29, 5)), fraction_json(Fraction(10))]
        )
        self.assertEqual(candidate["expected_regret"], fraction_json(Fraction(19, 50)))
        self.assertEqual(
            [item["cvar"] for item in candidate["cvar"]],
            [fraction_json(Fraction(14, 5)), fraction_json(Fraction(18, 5)), fraction_json(Fraction(6))]
        )

    def test_missing_probability_receipt_abstains_instead_of_inventing_weights(self):
        _, case = load_case()
        case["probability_model"] = None
        result = evaluate_case(case, [Fraction(9, 10)])
        self.assertEqual(result["status"], "ABSTAIN")
        self.assertEqual(result["reason"], "PROBABILITY_MODEL_RECEIPT_ABSENT")
        self.assertFalse(result["authority_granted"])

    def test_stale_probability_model_abstains(self):
        _, case = load_case()
        case["probability_model"]["state"] = "STALE"
        result = evaluate_case(case, [Fraction(9, 10)])
        self.assertEqual(result["status"], "ABSTAIN")
        self.assertEqual(result["reason"], "PROBABILITY_MODEL_NOT_CURRENT")

    def test_unqualified_calibration_abstains(self):
        _, case = load_case()
        case["probability_model"]["calibration_state"] = "UNQUALIFIED"
        result = evaluate_case(case, [Fraction(9, 10)])
        self.assertEqual(result["status"], "ABSTAIN")
        self.assertEqual(result["reason"], "PROBABILITY_CALIBRATION_NOT_QUALIFIED")

    def test_support_mismatch_is_invalid(self):
        _, case = load_case()
        case["probability_model"]["weights"].pop()
        result = evaluate_case(case, [Fraction(9, 10)])
        self.assertEqual(result["status"], "INVALID_INPUT")
        self.assertIn("exact joint scenario inventory", result["reason"])

    def test_non_normalized_weight_receipt_is_invalid(self):
        _, case = load_case()
        case["probability_model"]["weights"][0]["weight"] = {"numerator": 1, "denominator": 2}
        result = evaluate_case(case, [Fraction(9, 10)])
        self.assertEqual(result["status"], "INVALID_INPUT")
        self.assertIn("sum exactly to one", result["reason"])

    def test_modified_joint_scenario_mapping_is_invalid(self):
        _, case = load_case()
        case["joint_scenarios"][0]["loss_model_id"] = "substituted-model"
        result = evaluate_case(case, [Fraction(9, 10)])
        self.assertEqual(result["status"], "INVALID_INPUT")
        self.assertIn("joint scenario identity records", result["reason"])


if __name__ == "__main__":
    unittest.main()
