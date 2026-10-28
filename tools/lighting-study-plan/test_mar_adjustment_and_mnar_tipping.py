from __future__ import annotations
import json
import math
import unittest
import mar_adjustment_and_mnar_tipping as audit

class MarMnarAdjustmentTests(unittest.TestCase):
    def test_tipping_delta_reconstructs_margin(self):
        observed=[8.0,8.0,8.0,8.0,8.0]
        delta=audit.tipping_delta(observed,10,5.0)
        self.assertAlmostEqual(delta,-6.0)
        full_mean=(len(observed)/10)*8.0+(5/10)*(8.0+delta)
        self.assertAlmostEqual(full_mean,5.0)

    def test_tipping_delta_undefined_without_missing_or_observed(self):
        self.assertIsNone(audit.tipping_delta([6,7],2,5))
        self.assertIsNone(audit.tipping_delta([],4,5))

    def test_complete_case_and_ipw_agree_when_all_units_observed(self):
        values=[2.0,4.0,5.0,7.0,9.0]
        p=[1.0]*len(values)
        cc_est,cc_lcb,_=audit.complete_case(values,.05,0.0)
        ipw_est,ipw_lcb,_=audit.weighted_ht(values,p,.05,0.0)
        self.assertAlmostEqual(cc_est,ipw_est)
        self.assertAlmostEqual(cc_lcb,ipw_lcb)

    def test_regression_recovers_intercept_on_exact_linear_data(self):
        difficulties=[0.0,.1,.3,.6,.8,1.0]
        values=[8.0+3.0*(.5-d) for d in difficulties]
        est,_,_=audit.mar_regression(values,difficulties,.05,5.0)
        self.assertAlmostEqual(est,8.0,places=10)

    def test_report_is_synthetic_and_scopes_methods(self):
        report=audit.simulate(replicates=200,seed=10,sample_sizes=(10,20))
        self.assertEqual(report['decision'],'synthetic_missingness_adjustment_diagnostics_no_empirical_claim')
        self.assertEqual(report['summary']['scenario_count'],64)
        self.assertTrue(report['methods_not_selected_as_winners'])
        mnar=next(r for r in report['scenario_results'] if r['missingness_mechanism']=='mnar_low_outcome')
        self.assertIsNone(mnar['methods']['oracle_ipw']['mean_estimate_pct'])
        self.assertIn('invalid as a general MNAR correction',mnar['methods']['mar_regression']['validity_scope'])

    def test_mar_adjustment_reduces_complete_case_bias_in_linear_fixture(self):
        report=audit.simulate(replicates=1500,seed=20261028,sample_sizes=(20,))
        row=next(r for r in report['scenario_results'] if r['distribution']=='clipped_normal_4sd' and
                 r['missingness_mechanism']=='mar_difficulty' and r['effect_condition']=='null_at_margin')
        cc=abs(row['methods']['complete_case_student_t']['mean_estimate_bias_pp'])
        reg=abs(row['methods']['mar_regression']['mean_estimate_bias_pp'])
        ipw=abs(row['methods']['oracle_ipw']['mean_estimate_bias_pp'])
        self.assertLess(reg,cc)
        self.assertLess(ipw,cc)

    def test_tipping_point_becomes_more_negative_when_observed_mean_higher(self):
        a=audit.tipping_delta([6.0]*6,10,5.0)
        b=audit.tipping_delta([8.0]*6,10,5.0)
        self.assertLess(b,a)

    def test_methods_fail_closed_on_small_samples(self):
        est,lower,reject=audit.complete_case([5.0],.05,5.0)
        self.assertEqual(est,5.0)
        self.assertEqual(lower,float('-inf'))
        self.assertFalse(reject)
        est,lower,reject=audit.mar_regression([5.0,6.0],[.1,.9],.05,5.0)
        self.assertTrue(math.isnan(est))
        self.assertEqual(lower,float('-inf'))

    def test_deterministic_report_for_fixed_seed(self):
        a=audit.simulate(replicates=200,seed=444,sample_sizes=(10,))
        b=audit.simulate(replicates=200,seed=444,sample_sizes=(10,))
        self.assertEqual(json.dumps(a,sort_keys=True),json.dumps(b,sort_keys=True))

    def test_invalid_inputs_fail_closed(self):
        with self.assertRaises(audit.MissingnessAuditError): audit.simulate(replicates=99)
        with self.assertRaises(audit.MissingnessAuditError): audit.simulate(alpha=0)
        with self.assertRaises(audit.MissingnessAuditError): audit.simulate(sample_sizes=(4,))
        with self.assertRaises(audit.MissingnessAuditError): audit.simulate(lower_bound_pct=10,upper_bound_pct=-10)

if __name__=='__main__': unittest.main(verbosity=2)
