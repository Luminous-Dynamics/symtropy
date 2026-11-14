// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Synthetic known-truth oracle only; not an electromagnetic model or safety tool.
use std::{collections::BTreeSet, env, fs, path::Path, process::ExitCode};

const HEADER: [&str; 15] = [
 "design_id","useful_service_index","off_target_field_index","noise_sigma_feature_0","noise_sigma_feature_1",
 "pose_m100_feature_0","pose_m100_feature_1","pose_m50_feature_0","pose_m50_feature_1",
 "pose_0_feature_0","pose_0_feature_1","pose_p50_feature_0","pose_p50_feature_1",
 "pose_p100_feature_0","pose_p100_feature_1",
];
const X:[f64;5]=[-100.0,-50.0,0.0,50.0,100.0];
const DELTA_MM:f64=100.0;
const ALIAS_SIGMA:f64=1.5;
const SERVICE_FLOOR:f64=300.0;
const OFF_TARGET_CEILING:f64=40.0;

#[derive(Debug,Clone,PartialEq)]
pub struct Design { pub id:String, pub service:f64, pub off_target:f64, pub sigma:[f64;2], pub y:[[f64;2];5] }
#[derive(Debug,Clone,PartialEq)]
pub struct Alias { pub x0_mm:i32, pub x1_mm:i32, pub distance_sigma:f64 }
#[derive(Debug,Clone,PartialEq)]
pub struct Score {
 pub id:String, pub disposition:String, pub service:f64, pub off_target:f64,
 pub min_fisher_per_mm2:f64, pub min_global_separation_sigma:f64,
 pub unique_fraction:f64, pub aliases:Vec<Alias>,
}

fn number(s:&str,where_:&str)->Result<f64,String>{
 let n=s.parse::<f64>().map_err(|_|format!("{where_}: expected finite number"))?;
 if !n.is_finite(){return Err(format!("{where_}: non-finite number"))} Ok(n)
}

pub fn parse_designs(input:&str)->Result<Vec<Design>,Vec<String>>{
 let mut lines=input.split_terminator('\n');
 let Some(h)=lines.next() else{return Err(vec!["empty input".into()])};
 let h=h.strip_suffix('\r').unwrap_or(h);
 if h.split('\t').collect::<Vec<_>>().as_slice()!=HEADER.as_slice(){
  return Err(vec![format!("header must contain exactly {} columns",HEADER.len())])
 }
 let mut ids=BTreeSet::new();let mut out=Vec::new();let mut errors=Vec::new();
 for (i,raw) in lines.enumerate(){
  let line=raw.strip_suffix('\r').unwrap_or(raw);if line.is_empty(){continue}
  let f:Vec<_>=line.split('\t').collect();let row=i+2;
  if f.len()!=HEADER.len(){errors.push(format!("line {row}: expected {} columns, found {}",HEADER.len(),f.len()));continue}
  if f[0].trim().is_empty()||f[0].trim()!=f[0]{errors.push(format!("line {row}: invalid design_id"));continue}
  if !ids.insert(f[0].to_string()){errors.push(format!("line {row}: duplicate design_id {}",f[0]));continue}
  let parsed:Result<Vec<f64>,String>=f[1..].iter().enumerate().map(|(j,v)|number(v,&format!("line {row} col {}",j+2))).collect();
  let v=match parsed{Ok(v)=>v,Err(e)=>{errors.push(e);continue}};
  let sigma=[v[2],v[3]];
  if v[0]<0.0||v[1]<0.0||sigma.iter().any(|n|*n<=0.0){errors.push(format!("line {row}: indices must be non-negative and sigma positive"));continue}
  let mut y=[[0.0;2];5];for k in 0..5{y[k]=[v[4+2*k],v[5+2*k]]}
  out.push(Design{id:f[0].into(),service:v[0],off_target:v[1],sigma,y});
 }
 if out.is_empty()&&errors.is_empty(){errors.push("no designs".into())}
 if errors.is_empty(){Ok(out)}else{Err(errors)}
}

fn deriv(y:&[[f64;2];5],i:usize,k:usize)->f64{
 let(a,b)=if i==0{(0,1)}else if i==4{(3,4)}else{(i-1,i+1)};
 (y[b][k]-y[a][k])/(X[b]-X[a])
}
fn dist(a:[f64;2],b:[f64;2],s:[f64;2])->f64{
 let u=(a[0]-b[0])/s[0];let v=(a[1]-b[1])/s[1];(u*u+v*v).sqrt()
}

pub fn score(d:&Design)->Score{
 let mut min_fisher=f64::INFINITY;
 for i in 0..5{let a=deriv(&d.y,i,0)/d.sigma[0];let b=deriv(&d.y,i,1)/d.sigma[1];min_fisher=min_fisher.min(a*a+b*b)}
 let mut aliases=Vec::new();let mut ambiguous=BTreeSet::new();let mut min_sep=f64::INFINITY;
 for i in 0..5{for j in i+1..5{
  if X[j]-X[i]<DELTA_MM{continue}
  let q=dist(d.y[i],d.y[j],d.sigma);min_sep=min_sep.min(q);
  if q<=ALIAS_SIGMA{aliases.push(Alias{x0_mm:X[i] as i32,x1_mm:X[j] as i32,distance_sigma:q});ambiguous.insert(i);ambiguous.insert(j);}
 }}
 let unique=(5-ambiguous.len()) as f64/5.0;
 let disposition=if d.service<SERVICE_FLOOR||d.off_target>OFF_TARGET_CEILING{"HARD_CONSTRAINT_REJECTED"}else if !aliases.is_empty(){"FEASIBLE_BUT_AMBIGUOUS"}else{"FEASIBLE_SAMPLE_SET_UNIQUE"};
 Score{id:d.id.clone(),disposition:disposition.into(),service:d.service,off_target:d.off_target,min_fisher_per_mm2:min_fisher,min_global_separation_sigma:min_sep,unique_fraction:unique,aliases}
}
fn dominates(a:&Score,b:&Score)->bool{
 let weak=a.service>=b.service&&a.min_fisher_per_mm2>=b.min_fisher_per_mm2&&a.min_global_separation_sigma>=b.min_global_separation_sigma&&a.unique_fraction>=b.unique_fraction&&a.off_target<=b.off_target;
 let strict=a.service>b.service||a.min_fisher_per_mm2>b.min_fisher_per_mm2||a.min_global_separation_sigma>b.min_global_separation_sigma||a.unique_fraction>b.unique_fraction||a.off_target<b.off_target;
 weak&&strict
}
pub fn pareto_front(scores:&[Score])->Vec<String>{
 let f:Vec<_>=scores.iter().filter(|s|s.disposition!="HARD_CONSTRAINT_REJECTED").collect();
 f.iter().filter(|x|!f.iter().any(|y|y.id!=x.id&&dominates(y,x))).map(|x|x.id.clone()).collect()
}
pub fn evaluate(input:&str)->Result<(Vec<Score>,Vec<String>),Vec<String>>{
 let mut d=parse_designs(input)?;
 d.sort_by(|a,b|a.id.cmp(&b.id));
 let s:Vec<_>=d.iter().map(score).collect();
 let p=pareto_front(&s);
 Ok((s,p))
}
fn run(path:&Path)->Result<(),String>{
 let text=fs::read_to_string(path).map_err(|e|format!("cannot read {}: {e}",path.display()))?;
 let(s,p)=evaluate(&text).map_err(|e|e.join("\n"))?;
 println!("WPT field observability score oracle v0 — SYNTHETIC_KNOWN_TRUTH_ONLY");
 println!("lateral pose grid: -100..100 mm; material separation: {DELTA_MM} mm; alias threshold: {ALIAS_SIGMA} sigma");
 for x in &s{println!("{}: {} | service-index={:.1} | off-target-index={:.1} | min-FIM/mm2={:.6} | min-global-separation={:.3} sigma | unique-fraction={:.3} | aliases={}",x.id,x.disposition,x.service,x.off_target,x.min_fisher_per_mm2,x.min_global_separation_sigma,x.unique_fraction,x.aliases.len());}
 println!("NON-DOMINATED FEASIBLE SET: {}",p.join(", "));
 println!("CLAIM CEILING: finite synthetic signature map only; no EM, hardware, efficiency, exposure or safety claim");Ok(())
}
fn main()->ExitCode{
 let p=env::args().nth(1).unwrap_or_else(||"research/wpt-observability-field-v0/synthetic-designs.tsv".into());
 match run(Path::new(&p)){Ok(())=>ExitCode::SUCCESS,Err(e)=>{eprintln!("NOT_CONFORMS: {e}");ExitCode::FAILURE}}
}
#[cfg(test)]
mod tests{
 use super::*;const DATA:&str=include_str!("../../../research/wpt-observability-field-v0/synthetic-designs.tsv");
 fn all()->Vec<Score>{evaluate(DATA).unwrap().0}
 #[test]fn symmetric_lut_analogue_is_high_service_but_ambiguous(){let s=all().into_iter().find(|x|x.id=="position_lut_analogue").unwrap();assert_eq!(s.disposition,"FEASIBLE_BUT_AMBIGUOUS");assert_eq!(s.aliases.len(),2);assert_eq!(s.unique_fraction,0.2);assert_eq!(s.min_fisher_per_mm2,0.0);}
 #[test]fn informative_candidate_is_unique_on_the_declared_sample_set(){let s=all().into_iter().find(|x|x.id=="information_bearing_candidate").unwrap();assert_eq!(s.disposition,"FEASIBLE_SAMPLE_SET_UNIQUE");assert!(s.min_fisher_per_mm2>0.0);assert!(s.min_global_separation_sigma>ALIAS_SIGMA);assert_eq!(s.unique_fraction,1.0);}
 #[test]fn hard_off_target_limit_rejects_even_high_information(){let s=all().into_iter().find(|x|x.id=="off_target_violation_control").unwrap();assert_eq!(s.disposition,"HARD_CONSTRAINT_REJECTED");assert!(s.min_fisher_per_mm2>0.0);}
 #[test]fn pareto_front_preserves_energy_information_tradeoff(){let(_,p)=evaluate(DATA).unwrap();assert!(p.contains(&"position_lut_analogue".into()));assert!(p.contains(&"information_bearing_candidate".into()));assert!(!p.contains(&"off_target_violation_control".into()));}
 #[test]fn duplicate_ids_fail_closed(){let row=DATA.lines().nth(1).unwrap();let mut d=DATA.to_string();d.push_str(row);d.push('\n');assert!(parse_designs(&d).unwrap_err().iter().any(|e|e.contains("duplicate design_id")));}
 #[test]fn zero_noise_sigma_fails_closed(){let d=DATA.replacen("\t0.1\t0.1\t","\t0\t0.1\t",1);assert!(parse_designs(&d).is_err());}
 #[test]fn invalid_header_and_row_width_fail_closed(){assert!(parse_designs(&DATA.replacen("design_id","design_identifier",1)).is_err());assert!(parse_designs(&DATA.replacen("\t350\t35\t","\t350\t35\t1\t",1)).is_err());}
 #[test]fn output_is_deterministic_under_candidate_permutation(){let a=evaluate(DATA).unwrap();let mut r=DATA.lines().collect::<Vec<_>>();r[1..].reverse();let b=format!("{}\n{}\n",r[0],r[1..].join("\n"));assert_eq!(a,evaluate(&b).unwrap());}
}
