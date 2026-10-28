#!/usr/bin/env python3
"""Compare MAR adjustment and MNAR tipping-point diagnostics on synthetic family outcomes."""
from __future__ import annotations
import argparse
from functools import lru_cache
import hashlib
import json
import math
import random
import statistics
import sys
from pathlib import Path
from typing import Any
import plan_independent_scene_families as planner_module
from plan_independent_scene_families import _t_critical
from mnar_sensitivity_audit import (DISTRIBUTIONS, MISSINGNESS, MissingnessAuditError,
    draw_noise, missing_probability, wilson_interval, bounded_hoeffding_lower_bound)

@lru_cache(maxsize=512)
def _critical_t(alpha: float, df: int) -> float:
    return _t_critical(alpha, df)

def complete_case(values: list[float], alpha: float, margin: float) -> tuple[float, float, bool]:
    if not values:
        return math.nan, -math.inf, False
    est = statistics.fmean(values)
    if len(values) < 2:
        return est, -math.inf, False
    sd = statistics.stdev(values)
    if not math.isfinite(sd) or sd <= 0:
        return est, -math.inf, False
    lower = est - _critical_t(alpha, len(values)-1) * sd / math.sqrt(len(values))
    return est, lower, lower > margin

def weighted_ht(values: list[float | None], probabilities: list[float], alpha: float, margin: float) -> tuple[float, float, bool]:
    """Horvitz-Thompson mean and t lower bound using known pre-outcome response probabilities."""
    n=len(values)
    if n < 2 or len(probabilities) != n or any(not (0 < p <= 1) for p in probabilities):
        return math.nan, -math.inf, False
    contributions=[(value / p) if value is not None else 0.0 for value,p in zip(values,probabilities)]
    est=statistics.fmean(contributions)
    sd=statistics.stdev(contributions)
    if not math.isfinite(sd) or sd <= 0:
        return est, -math.inf, False
    lower=est-_critical_t(alpha,n-1)*sd/math.sqrt(n)
    return est, lower, lower > margin

def mar_regression(values: list[float | None], difficulty: list[float], alpha: float, margin: float) -> tuple[float,float,bool]:
    """OLS intercept at mean difficulty=.5; valid only under the declared linear MAR DGP."""
    rows=[(0.5-d, y) for y,d in zip(values,difficulty) if y is not None]
    n=len(rows)
    if n < 3:
        return math.nan,-math.inf,False
    xbar=statistics.fmean(x for x,_ in rows)
    ybar=statistics.fmean(y for _,y in rows)
    sxx=math.fsum((x-xbar)**2 for x,_ in rows)
    if not math.isfinite(sxx) or sxx <= 1e-14:
        return ybar,-math.inf,False
    slope=math.fsum((x-xbar)*(y-ybar) for x,y in rows)/sxx
    intercept=ybar-slope*xbar
    residuals=[y-(intercept+slope*x) for x,y in rows]
    residual_ss=math.fsum(r*r for r in residuals)
    mse=residual_ss/(n-2)
    se=math.sqrt(max(0.0,mse*(1.0/n+xbar*xbar/sxx)))
    if not math.isfinite(se) or se <= 0:
        return intercept,-math.inf,False
    lower=intercept-_critical_t(alpha,n-2)*se
    return intercept,lower,lower > margin

def tipping_delta(observed: list[float], n_attempted: int, margin: float) -> float | None:
    """Delta for assumed missing mean = observed mean + delta that tips the full mean to margin."""
    missing=n_attempted-len(observed)
    if not observed or missing <= 0:
        return None
    return (margin-statistics.fmean(observed))/(missing/n_attempted)

def quantile(values: list[float], q: float) -> float | None:
    if not values: return None
    v=sorted(values); p=(len(v)-1)*q; lo=int(p); frac=p-lo; hi=min(lo+1,len(v)-1)
    return v[lo]*(1-frac)+v[hi]*frac

def simulate(*, replicates: int=1000, seed: int=20261028, alpha: float=.05, margin_pct: float=5.0,
             family_sd_pp: float=3.0, lower_bound_pct: float=-15.0, upper_bound_pct: float=25.0,
             difficulty_slope_pp: float=3.0, sample_sizes: tuple[int,...]=(10,20,40)) -> dict[str,Any]:
    if replicates < 200 or replicates > 1_000_000: raise MissingnessAuditError("replicates must be in [200,1000000]")
    if not isinstance(seed,int) or isinstance(seed,bool) or seed<0: raise MissingnessAuditError("seed must be nonnegative integer")
    if not (0<alpha<.5): raise MissingnessAuditError("alpha must be in (0,.5)")
    if not all(math.isfinite(v) for v in (margin_pct,family_sd_pp,lower_bound_pct,upper_bound_pct,difficulty_slope_pp)):
        raise MissingnessAuditError("configuration values must be finite")
    if family_sd_pp<=0 or lower_bound_pct>=upper_bound_pct or difficulty_slope_pp<0: raise MissingnessAuditError("invalid SD, outcome bounds or difficulty effect")
    max_noise=max(4.0,math.sqrt(10.0))*family_sd_pp+difficulty_slope_pp/2
    if margin_pct-max_noise<lower_bound_pct or margin_pct+2*family_sd_pp+max_noise>upper_bound_pct: raise MissingnessAuditError("bounds do not contain stress effects")
    if not sample_sizes or any(not isinstance(n,int) or isinstance(n,bool) or n<5 for n in sample_sizes): raise MissingnessAuditError("sample sizes must be >=5")
    conditions=(("null_at_margin",0.0),("target_two_sd_above_margin",2.0))
    results=[]; index=0
    for n in sample_sizes:
      for dist in DISTRIBUTIONS:
       for mechanism in MISSINGNESS:
        for cond,mult in conditions:
         center=margin_pct+mult*family_sd_pp; rng=random.Random(seed+index*1009)
         reject={k:0 for k in ("complete_case_student_t","mar_regression","oracle_ipw","bounded_hoeffding")}
         estimates={k:[] for k in reject}; tip=[]; miss_fracs=[]
         for _ in range(replicates):
          observed=[]; values=[]; difficulties=[]; response_probs=[]
          for _family in range(n):
           d=rng.random(); y=center+family_sd_pp*draw_noise(rng,dist)+difficulty_slope_pp*(.5-d)
           if y<lower_bound_pct-1e-12 or y>upper_bound_pct+1e-12: raise MissingnessAuditError("DGP outside frozen effect bounds")
           p_miss=missing_probability(mechanism,y,d,margin_pct); is_missing=rng.random()<p_miss
           difficulties.append(d); values.append(None if is_missing else y)
           if not is_missing: observed.append(y)
           if mechanism=="complete": p_response=1.0
           elif mechanism=="mcar_30pct": p_response=.7
           elif mechanism=="mar_difficulty": p_response=1.0-(.05+.65*d)
           else: p_response=math.nan
           response_probs.append(p_response)
          cc_est,cc_lcb,cc_rej=complete_case(observed,alpha,margin_pct)
          reg_est,reg_lcb,reg_rej=mar_regression(values,difficulties,alpha,margin_pct)
          # Known-propensity IPW is available only for MCAR or MAR, never oracle-conditioned on an MNAR outcome.
          if mechanism=="mnar_low_outcome": ipw_est,ipw_lcb,ipw_rej=math.nan,-math.inf,False
          else: ipw_est,ipw_lcb,ipw_rej=weighted_ht(values,response_probs,alpha,margin_pct)
          completed=[lower_bound_pct if y is None else y for y in values]
          robust_est=statistics.fmean(completed)
          robust_lcb=bounded_hoeffding_lower_bound(completed,lower_bound_pct,upper_bound_pct,alpha)
          values_this={"complete_case_student_t":(cc_est,cc_rej),"mar_regression":(reg_est,reg_rej),
            "oracle_ipw":(ipw_est,ipw_rej),"bounded_hoeffding":(robust_est,robust_lcb>margin_pct)}
          for method,(est,rejected) in values_this.items():
           if math.isfinite(est): estimates[method].append(est)
           reject[method]+=int(rejected)
          delta=tipping_delta(observed,n,margin_pct)
          if delta is not None: tip.append(delta)
          miss_fracs.append((n-len(observed))/n)
         method_report={}
         for method in reject:
          method_report[method]={"rejection_rate":reject[method]/replicates,
            "rejection_rate_monte_carlo_95pct_wilson_ci":wilson_interval(reject[method],replicates),
            "mean_estimate_pct":statistics.fmean(estimates[method]) if estimates[method] else None,
            "mean_estimate_bias_pp":statistics.fmean(estimates[method])-center if estimates[method] else None}
         method_report["complete_case_student_t"]["validity_scope"]="observed-case mean; not a general all-family mean under informative missingness"
         method_report["mar_regression"]["validity_scope"]="linear conditional-mean model with fully observed pre-outcome difficulty and MAR selection; invalid as a general MNAR correction"
         method_report["oracle_ipw"]["validity_scope"]="known pre-outcome response propensity for complete/MCAR/MAR only; not evaluated under MNAR because propensity depends on unobserved effect"
         method_report["bounded_hoeffding"]["validity_scope"]="all-family mean under valid preregistered finite outcome bounds and independent families; conservative"
         results.append({"scenario_id":f"n{n}__{dist}__{mechanism}__{cond}","n_independent_families":n,
           "distribution":dist,"missingness_mechanism":mechanism,"effect_condition":cond,
           "true_mean_effect_location_pct":center,"superiority_margin_pct":margin_pct,"family_sd_pp":family_sd_pp,
           "outcome_bounds_pct":{"lower":lower_bound_pct,"upper":upper_bound_pct},"replicates":replicates,
           "seed":seed+index*1009,"mean_missing_fraction":statistics.fmean(miss_fracs),"methods":method_report,
           "missing_mean_tipping_delta_pp":{"meaning":"assumed missing-outcome mean minus observed-outcome mean needed for the full-sample point estimate to equal the margin; not a confidence bound",
             "median":quantile(tip,.5),"p05":quantile(tip,.05),"p95":quantile(tip,.95),"defined_replicates":len(tip)}})
         index+=1
    null=[r for r in results if r["effect_condition"]=="null_at_margin"]
    method_names=("complete_case_student_t","mar_regression","oracle_ipw","bounded_hoeffding")
    return {"schema_version":"0.1.0","analysis_kind":"mar_adjustment_and_mnar_tipping_point_sensitivity",
      "decision":"synthetic_missingness_adjustment_diagnostics_no_empirical_claim","claim_scope":"synthetic method diagnostics only; no real lighting performance claim",
      "sampling_unit":"independent_scene_family","primary_estimand":"all-family mean latent energy-saving percentage",
      "methods_not_selected_as_winners":True,"configuration":{"replicates_per_scenario":replicates,"seed":seed,
       "alpha_one_sided":alpha,"margin_pct":margin_pct,"family_sd_pp":family_sd_pp,"difficulty_slope_pp":difficulty_slope_pp,
       "outcome_bounds_pct":{"lower":lower_bound_pct,"upper":upper_bound_pct},"distributions":list(DISTRIBUTIONS),
       "missingness_mechanisms":list(MISSINGNESS),"sample_sizes":list(sample_sizes),"effect_conditions":[{"name":k,"shift_sd":v} for k,v in conditions],
       "monte_carlo_intervals":"individual 95% Wilson intervals; not simultaneous"},
      "summary":{"scenario_count":len(results),"null_scenario_count":len(null),
       "max_null_rejection_by_method":{m:max(r["methods"][m]["rejection_rate"] for r in null) for m in method_names},
       "null_scenario_flags_by_method":{m:[r["scenario_id"] for r in null if r["methods"][m]["rejection_rate_monte_carlo_95pct_wilson_ci"]["lower"]>alpha] for m in method_names}},
      "scenario_results":results,"limitations":[
       "The regression and IPW procedures are oracle-style demonstrations using the correct DGP model/propensity; model estimation and propensity uncertainty are not included.",
       "The MAR regression relies on a correct linear conditional mean and fully observed pre-outcome difficulty; it is not generally valid under MNAR.",
       "MNAR tipping points are point-estimate sensitivity summaries, not confidence intervals or significance tests.",
       "The bounded Hoeffding procedure can be uninformative when bounds are wide or many outcomes are missing; bounds need external justification.",
       "All effects and missingness patterns are synthetic and do not estimate actual lighting run failure rates.",
       "Per-scenario Monte Carlo intervals are not simultaneous. A separately preregistered confirmation suite is required before method selection."]}

def main(argv: list[str]|None=None)->int:
    p=argparse.ArgumentParser(description=__doc__);p.add_argument("--replicates",type=int,default=1000);p.add_argument("--seed",type=int,default=20261028)
    p.add_argument("--alpha",type=float,default=.05);p.add_argument("--margin-pct",type=float,default=5.0);p.add_argument("--family-sd-pp",type=float,default=3.0)
    p.add_argument("--lower-bound-pct",type=float,default=-15.0);p.add_argument("--upper-bound-pct",type=float,default=25.0)
    p.add_argument("--difficulty-slope-pp",type=float,default=3.0);p.add_argument("--sample-sizes",default="10,20,40");p.add_argument("--output",type=Path)
    a=p.parse_args(argv)
    try:
      sizes=tuple(int(v.strip()) for v in a.sample_sizes.split(",") if v.strip())
      report=simulate(replicates=a.replicates,seed=a.seed,alpha=a.alpha,margin_pct=a.margin_pct,family_sd_pp=a.family_sd_pp,
        lower_bound_pct=a.lower_bound_pct,upper_bound_pct=a.upper_bound_pct,difficulty_slope_pp=a.difficulty_slope_pp,sample_sizes=sizes)
      report["source_sha256"]=hashlib.sha256(Path(__file__).read_bytes()).hexdigest()
      report["t_critical_helper_source_sha256"]=hashlib.sha256(Path(planner_module.__file__).read_bytes()).hexdigest()
    except (MissingnessAuditError,ValueError,OverflowError) as exc:
      print(f"MAR/MNAR sensitivity audit: FAIL: {exc}",file=sys.stderr);return 2
    rendered=json.dumps(report,sort_keys=True,indent=2,allow_nan=False)+"\n"
    if a.output:a.output.parent.mkdir(parents=True,exist_ok=True);a.output.write_text(rendered,encoding="utf-8")
    print(f"MAR/MNAR sensitivity audit: {report['decision']}");print(f"scenarios={report['summary']['scenario_count']} reps={a.replicates}")
    print("maximum null rejection rates: "+json.dumps(report["summary"]["max_null_rejection_by_method"],sort_keys=True));return 0

if __name__=="__main__":raise SystemExit(main())
