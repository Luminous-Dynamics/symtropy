use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{collections::{BTreeMap, BTreeSet}, env, fs, path::{Path, PathBuf}};

const PROGRAMME: &str = "WATER-MIXING-EXPERIMENT-DESIGN-QUAL-001";
const CASE_COUNT: usize = 6;
const EVIDENCE_CEILING: &str = "synthetic candidate-design information arithmetic only";

type Matrix = Vec<Vec<f64>>;

fn field<'a>(value: &'a Value, key: &str) -> Result<&'a Value, String> {
    value.get(key).ok_or_else(|| format!("missing field: {key}"))
}
fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    field(value, key)?.as_str().ok_or_else(|| format!("{key} must be text"))
}
fn number(value: &Value, key: &str) -> Result<f64, String> {
    let result = field(value, key)?.as_f64().ok_or_else(|| format!("{key} must be numeric"))?;
    if !result.is_finite() { return Err(format!("{key} must be finite")); }
    Ok(result)
}
fn usize_value(value: &Value, key: &str) -> Result<usize, String> {
    field(value, key)?.as_u64().and_then(|v| usize::try_from(v).ok())
        .ok_or_else(|| format!("{key} must be a non-negative integer"))
}
fn array<'a>(value: &'a Value, key: &str) -> Result<&'a Vec<Value>, String> {
    field(value, key)?.as_array().ok_or_else(|| format!("{key} must be an array"))
}
fn vector(value: &Value, key: &str) -> Result<Vec<f64>, String> {
    field(value, key)?.as_array().ok_or_else(|| format!("{key} must be an array"))?
        .iter().map(|x| x.as_f64().filter(|v| v.is_finite())
            .ok_or_else(|| format!("{key} contains a non-finite/non-numeric value")))
        .collect()
}
fn matrix(value: &Value, key: &str, rows: usize, cols: usize) -> Result<Matrix, String> {
    let raw = field(value, key)?.as_array().ok_or_else(|| format!("{key} must be a matrix"))?;
    if raw.len() != rows { return Err(format!("{key} row count mismatch")); }
    let mut out = Vec::with_capacity(rows);
    for row in raw {
        let values = row.as_array().ok_or_else(|| format!("{key} row must be an array"))?;
        if values.len() != cols { return Err(format!("{key} column count mismatch")); }
        out.push(values.iter().map(|x| x.as_f64().filter(|v| v.is_finite())
            .ok_or_else(|| format!("{key} contains a non-finite/non-numeric value")))
            .collect::<Result<Vec<_>, _>>()?);
    }
    Ok(out)
}
fn dot(left: &[f64], right: &[f64]) -> f64 {
    left.iter().zip(right).map(|(a,b)| a*b).sum()
}
fn cholesky(a: &Matrix) -> Result<Matrix, String> {
    let n = a.len();
    if n == 0 || a.iter().any(|row| row.len() != n) { return Err("covariance must be non-empty and square".into()); }
    let scale = a.iter().enumerate().map(|(i,row)| row[i].abs()).fold(0.0_f64, f64::max);
    if scale <= 0.0 { return Err("measurement covariance has no positive diagonal scale".into()); }
    for i in 0..n { for j in 0..n {
        if (a[i][j]-a[j][i]).abs() > 1.0e-10*scale {
            return Err("measurement covariance is not symmetric".into());
        }
    }}
    let mut l = vec![vec![0.0; n]; n];
    for i in 0..n {
        for j in 0..=i {
            let mut sum = a[i][j];
            for k in 0..j { sum -= l[i][k]*l[j][k]; }
            if i == j {
                if !sum.is_finite() || sum <= 1.0e-12*scale {
                    return Err("measurement covariance is not positive definite at declared tolerance".into());
                }
                l[i][j] = sum.sqrt();
            } else {
                l[i][j] = sum/l[j][j];
            }
        }
    }
    Ok(l)
}
fn solve_spd(l: &Matrix, b: &[f64]) -> Result<Vec<f64>, String> {
    let n = l.len();
    if b.len()!=n { return Err("linear solve dimension mismatch".into()); }
    let mut y = vec![0.0; n];
    for i in 0..n {
        let mut value=b[i];
        for j in 0..i { value -= l[i][j]*y[j]; }
        y[i]=value/l[i][i];
    }
    let mut x=vec![0.0;n];
    for i in (0..n).rev() {
        let mut value=y[i];
        for j in (i+1)..n { value -= l[j][i]*x[j]; }
        x[i]=value/l[i][i];
    }
    if x.iter().any(|v| !v.is_finite()) { return Err("linear solve produced non-finite output".into()); }
    Ok(x)
}
fn fisher_information(j: &Matrix, covariance: &Matrix) -> Result<Matrix, String> {
    let m=j.len();
    if m==0 || j.iter().any(|row| row.len()!=j[0].len()) { return Err("sensitivity matrix must be non-empty and rectangular".into()); }
    if covariance.len()!=m || covariance.iter().any(|row|row.len()!=m) { return Err("covariance/sensitivity observation dimensions differ".into()); }
    let l=cholesky(covariance)?;
    let p=j[0].len();
    let mut solved=vec![vec![0.0;p];m];
    for col in 0..p {
        let rhs=(0..m).map(|row|j[row][col]).collect::<Vec<_>>();
        let result=solve_spd(&l,&rhs)?;
        for row in 0..m { solved[row][col]=result[row]; }
    }
    let mut f=vec![vec![0.0;p];p];
    for a in 0..p { for b in 0..p {
        for row in 0..m { f[a][b] += j[row][a]*solved[row][b]; }
    }}
    for a in 0..p { for b in 0..a {
        let mean=0.5*(f[a][b]+f[b][a]); f[a][b]=mean; f[b][a]=mean;
    }}
    Ok(f)
}
fn symmetric_eigenvalues(mut a: Matrix) -> Result<Vec<f64>, String> {
    let n=a.len();
    if n==0 || a.iter().any(|row|row.len()!=n) { return Err("eigenvalue matrix must be square".into()); }
    if n==1 { return Ok(vec![a[0][0]]); }
    let scale=a.iter().flatten().map(|v|v.abs()).fold(0.0_f64,f64::max).max(1.0e-300);
    let max_iter=100*n*n;
    let mut converged=false;
    for _ in 0..max_iter {
        let mut p=0usize; let mut q=1usize; let mut largest=0.0_f64;
        for i in 0..n { for j in (i+1)..n {
            if a[i][j].abs()>largest { largest=a[i][j].abs(); p=i; q=j; }
        }}
        if largest <= 1.0e-13*scale { converged=true; break; }
        let app=a[p][p]; let aqq=a[q][q]; let apq=a[p][q];
        let tau=(aqq-app)/(2.0*apq);
        let sign=if tau>=0.0 {1.0} else {-1.0};
        let t=sign/(tau.abs()+(1.0+tau*tau).sqrt());
        let c=1.0/(1.0+t*t).sqrt(); let s=t*c;
        for k in 0..n {
            if k!=p && k!=q {
                let akp=a[k][p]; let akq=a[k][q];
                a[k][p]=c*akp-s*akq; a[p][k]=a[k][p];
                a[k][q]=s*akp+c*akq; a[q][k]=a[k][q];
            }
        }
        a[p][p]=app-t*apq; a[q][q]=aqq+t*apq;
        a[p][q]=0.0; a[q][p]=0.0;
    }
    if !converged { return Err("symmetric Jacobi eigensolver did not converge".into()); }
    let mut values=(0..n).map(|i|a[i][i]).collect::<Vec<_>>();
    let magnitude=values.iter().map(|v|v.abs()).fold(0.0_f64,f64::max).max(1.0);
    for value in &mut values {
        if *value < -1.0e-9*magnitude { return Err("Fisher information has materially negative eigenvalue".into()); }
        if *value<0.0 { *value=0.0; }
    }
    values.sort_by(|a,b|b.total_cmp(a));
    Ok(values)
}
#[derive(Clone)]
struct Spectrum {
    singular: Vec<f64>,
    rank: usize,
    rank_tenth: usize,
    rank_ten: usize,
    condition: Option<f64>,
    logdet: Option<f64>,
    minimum: f64,
}
fn spectrum(info: &Matrix, tolerance: f64) -> Result<Spectrum,String> {
    let eigen=symmetric_eigenvalues(info.clone())?;
    let singular=eigen.iter().map(|v|v.sqrt()).collect::<Vec<_>>();
    let maximum=singular.first().copied().unwrap_or(0.0);
    let rank_at=|relative:f64| -> usize {
        if maximum<=0.0 {0} else {singular.iter().filter(|s|**s>maximum*relative).count()}
    };
    let rank=rank_at(tolerance);
    let full=rank==singular.len() && singular.last().copied().unwrap_or(0.0)>0.0;
    let condition=if full {Some(maximum/singular[singular.len()-1])} else {None};
    let logdet=if full {Some(eigen.iter().map(|v|v.ln()).sum())} else {None};
    Ok(Spectrum {
        minimum:if full {singular[singular.len()-1]} else {0.0},
        singular, rank, rank_tenth:rank_at(tolerance*0.1),
        rank_ten:rank_at((tolerance*10.0).min(0.999999)),
        condition, logdet
    })
}
fn submatrix(a:&Matrix, row_start:usize,row_end:usize,col_start:usize,col_end:usize)->Matrix {
    (row_start..row_end).map(|i|(col_start..col_end).map(|j|a[i][j]).collect()).collect()
}
fn nuisance_adjusted(f:&Matrix,target:usize)->Result<Option<Matrix>,String> {
    let p=f.len();
    if target==p { return Ok(Some(f.clone())); }
    if target==0 || target>p { return Err("target parameter count is out of range".into()); }
    let ftt=submatrix(f,0,target,0,target);
    let ftn=submatrix(f,0,target,target,p);
    let fnn=submatrix(f,target,p,target,p);
    let fnt=submatrix(f,target,p,0,target);
    let l=match cholesky(&fnn) {Ok(v)=>v,Err(_)=>return Ok(None)};
    let q=p-target;
    let mut adjusted=ftt;
    for col in 0..target {
        let rhs=(0..q).map(|r|fnt[r][col]).collect::<Vec<_>>();
        let solved=solve_spd(&l,&rhs)?;
        for row in 0..target {
            adjusted[row][col]-=dot(&ftn[row],&solved);
        }
    }
    for i in 0..target { for j in 0..i {
        let mean=0.5*(adjusted[i][j]+adjusted[j][i]); adjusted[i][j]=mean; adjusted[j][i]=mean;
    }}
    Ok(Some(adjusted))
}
fn model_separation(case:&Value,covariance:&Matrix)->Result<(Vec<Value>,f64),String> {
    let raw=field(case,"model_predictions")?.as_object()
        .ok_or_else(||"model_predictions must be an object".to_owned())?;
    if raw.len()<2 { return Err("at least two competing model predictions are required".into()); }
    let l=cholesky(covariance)?;
    let models=raw.iter().map(|(name,v)| {
        let values=v.as_array().ok_or_else(||format!("model prediction {name} must be an array"))?
            .iter().map(|x|x.as_f64().filter(|n|n.is_finite())
                .ok_or_else(||format!("model prediction {name} contains invalid value")))
            .collect::<Result<Vec<_>,_>>()?;
        Ok((name.clone(),values))
    }).collect::<Result<Vec<_>,String>>()?;
    let mut pairs=Vec::new(); let mut minimum=f64::INFINITY;
    for i in 0..models.len() { for j in (i+1)..models.len() {
        if models[i].1.len()!=covariance.len() || models[j].1.len()!=covariance.len() {
            return Err("model prediction observation dimension mismatch".into());
        }
        let delta=models[i].1.iter().zip(&models[j].1).map(|(a,b)|a-b).collect::<Vec<_>>();
        let solved=solve_spd(&l,&delta)?;
        let distance=dot(&delta,&solved).max(0.0).sqrt();
        minimum=minimum.min(distance);
        pairs.push(json!({"model_a":models[i].0.clone(),"model_b":models[j].0.clone(),"mahalanobis_separation":distance}));
    }}
    Ok((pairs,minimum))
}
fn metric_json(s:&Spectrum)->Value {
    json!({
        "singular_values":s.singular,"rank":s.rank,
        "rank_at_one_tenth_tolerance":s.rank_tenth,
        "rank_at_ten_times_tolerance":s.rank_ten,
        "condition_number":s.condition,
        "log_determinant":s.logdet,
        "minimum_singular_value":s.minimum
    })
}
fn compute_stratum(case:&Value,parameter_scales:&[f64],target_count:usize,tolerance:f64)->Result<(Value,Spectrum,f64,bool),String> {
    let raw=field(case,"sensitivity_matrix")?.as_array()
        .ok_or_else(||"sensitivity_matrix must be an array".to_owned())?;
    if raw.is_empty() { return Err("sensitivity_matrix cannot be empty".into()); }
    let observations=raw.len();
    let columns=raw[0].as_array().ok_or_else(||"sensitivity row must be an array".to_owned())?.len();
    if columns!=parameter_scales.len() || columns<target_count { return Err("sensitivity parameter dimension mismatch".into()); }
    let mut j=matrix(case,"sensitivity_matrix",observations,columns)?;
    for row in &mut j { for (col,value) in row.iter_mut().enumerate() { *value*=parameter_scales[col]; }}
    let covariance=matrix(case,"measurement_covariance",observations,observations)?;
    let f=fisher_information(&j,&covariance)?;
    let raw_target=submatrix(&f,0,target_count,0,target_count);
    let raw_metrics=spectrum(&raw_target,tolerance)?;
    let adjusted=nuisance_adjusted(&f,target_count)?;
    let nuisance_ok=adjusted.is_some();
    let target_info=adjusted.unwrap_or_else(||vec![vec![0.0;target_count];target_count]);
    let target_metrics=spectrum(&target_info,tolerance)?;
    let (pairs,min_separation)=model_separation(case,&covariance)?;
    let state=if !nuisance_ok {"NUISANCE_UNRESOLVED"} else if target_metrics.rank<target_count {"UNIDENTIFIABLE_AFTER_NUISANCE_ADJUSTMENT"} else {"ANALYTICALLY_IDENTIFIABLE_T0"};
    let result=json!({
        "stratum_id":text(case,"stratum_id")?,
        "raw_target_information":metric_json(&raw_metrics),
        "nuisance_adjustment_state":if parameter_scales.len()==target_count {"NO_NUISANCE_PARAMETERS"} else if nuisance_ok {"SCHUR_COMPLEMENT_APPLIED"} else {"NUISANCE_UNRESOLVED"},
        "target_information":metric_json(&target_metrics),
        "pairwise_model_separations":pairs,
        "minimum_model_separation":min_separation,
        "stratum_state":state
    });
    Ok((result,target_metrics,min_separation,nuisance_ok))
}
fn compute_design(case:&Value,parameter_scales:&[f64],target_count:usize,tolerance:f64,
                  budget:f64,max_condition:f64,min_separation:f64)->Result<Value,String> {
    let id=text(case,"design_id")?;
    let cost=number(case,"cost_units")?;
    if cost<0.0 { return Err("cost_units cannot be negative".into()); }
    let sensors=usize_value(case,"sensor_count")?;
    let duration=number(case,"duration_minutes")?;
    if duration<0.0 { return Err("duration_minutes cannot be negative".into()); }
    let strata=array(case,"strata")?;
    if strata.is_empty() { return Err("design must include at least one operating stratum".into()); }
    let mut rows=Vec::new(); let mut metrics=Vec::new(); let mut separations=Vec::new(); let mut nuisance_ok=true;
    for stratum in strata {
        let (row,metric,separation,nuisance)=compute_stratum(stratum,parameter_scales,target_count,tolerance)?;
        rows.push(row); metrics.push(metric); separations.push(separation); nuisance_ok &= nuisance;
    }
    let worst_rank=metrics.iter().map(|m|m.rank).min().unwrap_or(0);
    let min_singular=metrics.iter().map(|m|m.minimum).fold(f64::INFINITY,f64::min);
    let worst_logdet=if metrics.iter().all(|m|m.logdet.is_some()) {
        metrics.iter().filter_map(|m|m.logdet).fold(f64::INFINITY,f64::min)
    } else {f64::NEG_INFINITY};
    let worst_condition=if metrics.iter().all(|m|m.condition.is_some()) {
        Some(metrics.iter().filter_map(|m|m.condition).fold(0.0_f64,f64::max))
    } else {None};
    let worst_separation=separations.iter().copied().fold(f64::INFINITY,f64::min);
    let within_budget=cost<=budget;
    let state=if !within_budget {"OVER_BUDGET"}
        else if !nuisance_ok {"NUISANCE_UNRESOLVED"}
        else if worst_rank<target_count {"UNIDENTIFIABLE"}
        else if worst_condition.is_none_or(|v|v>max_condition) {"ILL_CONDITIONED"}
        else if worst_separation<min_separation {"MODEL_DISCRIMINATION_INSUFFICIENT"}
        else {"ELIGIBLE_DESIGN_CANDIDATE"};
    Ok(json!({
        "design_id":id,"cost_units":cost,"sensor_count":sensors,"duration_minutes":duration,
        "within_budget":within_budget,"design_state":state,
        "worst_case_target_rank":worst_rank,
        "worst_case_minimum_singular_value":min_singular,
        "worst_case_condition_number":worst_condition,
        "worst_case_log_determinant":if worst_logdet.is_finite(){Some(worst_logdet)}else{None},
        "worst_case_minimum_model_separation":worst_separation,
        "per_stratum":rows,
        "physical_qualification_established":false,
        "potability_claim_permitted":false,"release_authorized":false
    }))
}
fn metric_value(row:&Value,key:&str,missing:f64)->f64 {
    row.get(key).and_then(Value::as_f64).unwrap_or(missing)
}
fn dominates(a:&Value,b:&Value)->bool {
    let al=metric_value(a,"worst_case_log_determinant",f64::NEG_INFINITY);
    let bl=metric_value(b,"worst_case_log_determinant",f64::NEG_INFINITY);
    let asg=metric_value(a,"worst_case_minimum_singular_value",0.0);
    let bsg=metric_value(b,"worst_case_minimum_singular_value",0.0);
    let am=metric_value(a,"worst_case_minimum_model_separation",0.0);
    let bm=metric_value(b,"worst_case_minimum_model_separation",0.0);
    let ac=metric_value(a,"cost_units",f64::INFINITY);
    let bc=metric_value(b,"cost_units",f64::INFINITY);
    let no_worse=al>=bl && asg>=bsg && am>=bm && ac<=bc;
    let strictly=al>bl || asg>bsg || am>bm || ac<bc;
    no_worse && strictly
}
fn validate(input:&Value,oracle:&Value,manifest:&Value)->Result<(),String> {
    if text(input,"schema_version")? != "mixing-design-t0-input-v0.1" {return Err("unexpected input schema".into());}
    if text(input,"programme_id")?!=PROGRAMME || text(oracle,"programme_id")?!=PROGRAMME || text(manifest,"programme_id")?!=PROGRAMME {
        return Err("programme identity mismatch".into());
    }
    if text(oracle,"schema_version")? != "mixing-design-t0-oracle-v0.1" {return Err("unexpected oracle schema".into());}
    if field(oracle,"physical_qualification_established")?.as_bool()!=Some(false)
        || field(oracle,"potability_claim_permitted")?.as_bool()!=Some(false)
        || field(oracle,"release_authorized")?.as_bool()!=Some(false) {
        return Err("oracle may not claim physical qualification, potability or release".into());
    }
    let candidates=array(input,"candidates")?;
    let expected=array(oracle,"designs")?;
    let manifest_ids=array(manifest,"design_ids")?;
    if candidates.len()!=CASE_COUNT || expected.len()!=CASE_COUNT
        || usize_value(manifest,"scenario_count")?!=CASE_COUNT {
        return Err("frozen design count mismatch".into());
    }
    let to_ids=|values:&[Value],key:&str| -> Result<BTreeSet<String>,String> {
        values.iter().map(|v|v.as_str().map(str::to_owned).ok_or_else(||format!("{key} entries must be text"))).collect()
    };
    let input_ids=candidates.iter().map(|v|Ok(text(v,"design_id")?.to_owned())).collect::<Result<BTreeSet<_>,String>>()?;
    let oracle_ids=expected.iter().map(|v|Ok(text(v,"design_id")?.to_owned())).collect::<Result<BTreeSet<_>,String>>()?;
    let manifest_set=to_ids(manifest_ids,"manifest")?;
    if input_ids.len()!=candidates.len() || oracle_ids.len()!=expected.len() {return Err("duplicate design identity".into());}
    if input_ids!=oracle_ids || input_ids!=manifest_set {return Err("input/oracle/manifest design identity mismatch".into());}
    Ok(())
}
fn compare_json(actual:&Value,expected:&Value)->bool {
    match (actual,expected) {
        (Value::Number(a),Value::Number(b)) => matches!((a.as_f64(),b.as_f64()),(Some(x),Some(y)) if (x-y).abs()<=1.0e-8*x.abs().max(y.abs()).max(1.0)),
        (Value::Array(a),Value::Array(b)) => a.len()==b.len() && a.iter().zip(b).all(|(x,y)|compare_json(x,y)),
        (Value::Object(a),Value::Object(b)) => a.len()==b.len() && a.iter().all(|(k,v)|b.get(k).is_some_and(|w|compare_json(v,w))),
        _=>actual==expected
    }
}
pub fn evaluate(input:&Value,oracle:&Value,manifest:&Value)->Result<Value,String> {
    validate(input,oracle,manifest)?;
    let parameter_count=usize_value(input,"parameter_count")?;
    let target_count=usize_value(input,"target_parameter_count")?;
    if target_count==0 || target_count>parameter_count {return Err("target_parameter_count out of range".into());}
    let scales=vector(input,"parameter_scales")?;
    if scales.len()!=parameter_count || scales.iter().any(|v|*v<=0.0) {return Err("parameter scales must be positive and match parameter_count".into());}
    let tolerance=number(input,"rank_relative_tolerance")?;
    if tolerance<=0.0 || tolerance>=0.1 {return Err("rank_relative_tolerance must be in (0, 0.1)".into());}
    let budget=number(input,"budget_limit_units")?;
    let max_condition=number(input,"maximum_condition_number")?;
    let min_sep=number(input,"minimum_model_separation")?;
    if budget<0.0 || max_condition<=1.0 || min_sep<0.0 {return Err("budget/condition/separation threshold invalid".into());}
    let mut results=Vec::new(); let mut mismatches=Vec::new();
    let expected=field(oracle,"designs")?.as_array().unwrap();
    let expected_by_id: BTreeMap<String,Value>=expected.iter().map(|row|Ok((text(row,"design_id")?.to_owned(),field(row,"expected")?.clone())))
        .collect::<Result<_,String>>()?;
    for candidate in array(input,"candidates")? {
        let id=text(candidate,"design_id")?;
        let computed=match compute_design(candidate,&scales,target_count,tolerance,budget,max_condition,min_sep) {
            Ok(v)=>v,
            Err(reason)=>json!({"design_id":id,"design_state":"REJECTED_INPUT","reason":reason,
                "physical_qualification_established":false,"potability_claim_permitted":false,"release_authorized":false})
        };
        let expected_row=expected_by_id.get(id).ok_or_else(||format!("oracle missing design {id}"))?;
        let comparable=json!({
            "design_state":computed["design_state"],
            "worst_case_target_rank":computed["worst_case_target_rank"],
            "worst_case_minimum_singular_value":computed["worst_case_minimum_singular_value"],
            "worst_case_condition_number":computed["worst_case_condition_number"],
            "worst_case_log_determinant":computed["worst_case_log_determinant"],
            "worst_case_minimum_model_separation":computed["worst_case_minimum_model_separation"]
        });
        let matched=compare_json(&comparable,expected_row);
        if !matched {mismatches.push(json!({"design_id":id,"expected":expected_row,"computed":comparable}));}
        results.push(computed);
    }
    results.sort_by(|a,b|a["design_id"].as_str().cmp(&b["design_id"].as_str()));
    let mut pareto=Vec::new();
    for i in 0..results.len() {
        if results[i]["within_budget"].as_bool()!=Some(true) {continue;}
        let dominated=(0..results.len()).any(|j|i!=j && results[j]["within_budget"].as_bool()==Some(true) && dominates(&results[j],&results[i]));
        if !dominated {pareto.push(results[i]["design_id"].as_str().unwrap().to_owned());}
    }
    pareto.sort();
    Ok(json!({
        "schema_version":"mixing-design-t0-evaluation-v0.1",
        "verification_state":if mismatches.is_empty() {"MatchedFrozenOracle"} else {"OracleMismatch"},
        "evidence_class":"synthetic_only",
        "evidence_ceiling":EVIDENCE_CEILING,
        "target_parameter_count":target_count,
        "candidate_count":results.len(),
        "matched_candidate_count":results.len()-mismatches.len(),
        "mismatches":mismatches,"pareto_frontier_design_ids":pareto,"results":results,
        "nonclaims":[
            "This evaluator compares pre-registered candidate design bundles; it does not synthesize arbitrary sensor layouts.",
            "Fisher information is a local linear/Gaussian approximation and does not prove nonlinear posterior calibration.",
            "Mahalanobis separation is a deterministic model-discrimination proxy, not expected information gain or posterior model probability.",
            "No result establishes physical mixing, sensor calibration, tracer safety, potability, contamination or release authority."
        ]
    }))
}
fn sha256(bytes:&[u8])->String {Sha256::digest(bytes).iter().map(|b|format!("{b:02x}")).collect()}
fn read_json(path:&Path)->Result<(Vec<u8>,Value),String> {
    let bytes=fs::read(path).map_err(|e|format!("read {}: {e}",path.display()))?;
    let value=serde_json::from_slice(&bytes).map_err(|e|format!("parse {}: {e}",path.display()))?;
    Ok((bytes,value))
}
fn valid_revision(value:&str)->bool {
    value.len()==40 && value.bytes().all(|b|b.is_ascii_digit()||(b'a'..=b'f').contains(&b))
}
fn run()->Result<i32,String> {
    let args:Vec<String>=env::args().skip(1).collect();
    if args.len()>4 {return Err("usage: mixing-design-t0 [INPUTS] [ORACLE] [MANIFEST] [RECEIPT]".into());}
    let input_path=PathBuf::from(args.first().map(String::as_str).unwrap_or("research/water-intrusion-coupling/mixing-design-t0-cases-v0.1.json"));
    let oracle_path=PathBuf::from(args.get(1).map(String::as_str).unwrap_or("research/water-intrusion-coupling/mixing-design-t0-oracle-v0.1.json"));
    let manifest_path=PathBuf::from(args.get(2).map(String::as_str).unwrap_or("research/water-intrusion-coupling/mixing-design-t0-manifest-v0.1.json"));
    let receipt_path=PathBuf::from(args.get(3).map(String::as_str).unwrap_or("artifacts/water-mixing-design-t0/receipt.json"));
    let (ib,iv)=read_json(&input_path)?; let (ob,ov)=read_json(&oracle_path)?; let (mb,mv)=read_json(&manifest_path)?;
    let evaluation=evaluate(&iv,&ov,&mv)?;
    let count=evaluation["candidate_count"].as_u64().unwrap_or(0);
    let matched=evaluation["matched_candidate_count"].as_u64().unwrap_or(0);
    let success=count==CASE_COUNT as u64 && matched==CASE_COUNT as u64 && evaluation["mismatches"].as_array().is_some_and(Vec::is_empty);
    let revision=env::var("WATER_MIXING_DESIGN_T0_REVISION").or_else(|_|env::var("GITHUB_SHA")).ok().filter(|v|valid_revision(v));
    let lock_path=PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.lock");
    let lock=fs::read(&lock_path).map_err(|e|format!("read {}: {e}",lock_path.display()))?;
    let receipt=json!({
        "schema_version":"mixing-design-t0-run-v0.1",
        "verification_state":if !success {"OracleMismatch"} else if revision.is_some() {"PassedExactRevision"} else {"PassedLocalUnpinned"},
        "qualification_status":"NotRun","implementation_revision":revision,
        "input_sha256":sha256(&ib),"oracle_sha256":sha256(&ob),"manifest_sha256":sha256(&mb),
        "cargo_lock_sha256":sha256(&lock),
        "candidate_count":count,"matched_candidate_count":matched,
        "pareto_frontier_design_ids":evaluation["pareto_frontier_design_ids"],
        "mismatches":evaluation["mismatches"],"results":evaluation["results"],
        "evidence_class":"synthetic_only","evidence_ceiling":EVIDENCE_CEILING,
        "nonclaims":evaluation["nonclaims"]
    });
    if let Some(parent)=receipt_path.parent() {if !parent.as_os_str().is_empty() {
        fs::create_dir_all(parent).map_err(|e|format!("create {}: {e}",parent.display()))?;
    }}
    let serialized=serde_json::to_string_pretty(&receipt).map_err(|e|format!("serialize receipt: {e}"))?+"\n";
    fs::write(&receipt_path,serialized.as_bytes()).map_err(|e|format!("write {}: {e}",receipt_path.display()))?;
    println!("{serialized}");
    Ok(if success {0}else{2})
}
fn main() {
    match run() {
        Ok(code)=>std::process::exit(code),
        Err(error)=>{eprintln!("WATER_MIXING_DESIGN_T0_ERROR: {error}");std::process::exit(1);}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn load(name:&str)->Value {
        let source=match name {
            "input"=>include_str!("../../../../research/water-intrusion-coupling/mixing-design-t0-cases-v0.1.json"),
            "oracle"=>include_str!("../../../../research/water-intrusion-coupling/mixing-design-t0-oracle-v0.1.json"),
            "manifest"=>include_str!("../../../../research/water-intrusion-coupling/mixing-design-t0-manifest-v0.1.json"),
            _=>panic!("unknown fixture")
        };
        serde_json::from_str(source).expect("frozen design fixture must be valid JSON")
    }
    fn evaluated()->Value {evaluate(&load("input"),&load("oracle"),&load("manifest")).expect("design evaluator runs")}
    #[test] fn all_six_designs_match_the_frozen_oracle() {
        let v=evaluated(); assert_eq!(v["candidate_count"],CASE_COUNT); assert_eq!(v["matched_candidate_count"],CASE_COUNT);
        assert_eq!(v["verification_state"],"MatchedFrozenOracle"); assert!(v["mismatches"].as_array().unwrap().is_empty());
    }
    #[test] fn full_rank_but_ill_conditioned_design_is_not_eligible() {
        let row=evaluated()["results"].as_array().unwrap().iter().find(|r|r["design_id"]=="MDT-T0-002-ill-conditioned").unwrap().clone();
        assert_eq!(row["worst_case_target_rank"],2); assert_eq!(row["design_state"],"ILL_CONDITIONED");
        assert!((row["worst_case_condition_number"].as_f64().unwrap()-1000.0).abs()<1.0e-8);
    }
    #[test] fn repeated_proportional_excitation_is_unidentifiable() {
        let row=evaluated()["results"].as_array().unwrap().iter().find(|r|r["design_id"]=="MDT-T0-003-rank-deficient").unwrap().clone();
        assert_eq!(row["worst_case_target_rank"],1); assert_eq!(row["design_state"],"UNIDENTIFIABLE");
        assert!(row["worst_case_log_determinant"].is_null());
    }
    #[test] fn robust_design_reports_the_worst_operating_stratum() {
        let row=evaluated()["results"].as_array().unwrap().iter().find(|r|r["design_id"]=="MDT-T0-004-robust-two-strata").unwrap().clone();
        assert_eq!(row["per_stratum"].as_array().unwrap().len(),2); assert_eq!(row["design_state"],"ELIGIBLE_DESIGN_CANDIDATE");
        assert!((row["worst_case_minimum_singular_value"].as_f64().unwrap()-0.4).abs()<1.0e-8);
    }
    #[test] fn nuisance_confounding_reduces_target_rank_after_schur_adjustment() {
        let row=evaluated()["results"].as_array().unwrap().iter().find(|r|r["design_id"]=="MDT-T0-005-nuisance-confounded").unwrap().clone();
        assert_eq!(row["per_stratum"][0]["raw_target_information"]["rank"],2);
        assert_eq!(row["worst_case_target_rank"],1); assert_eq!(row["design_state"],"UNIDENTIFIABLE");
    }
    #[test] fn singular_nuisance_block_causes_explicit_abstention() {
        let row=evaluated()["results"].as_array().unwrap().iter().find(|r|r["design_id"]=="MDT-T0-006-nuisance-unresolved").unwrap().clone();
        assert_eq!(row["design_state"],"NUISANCE_UNRESOLVED");
    }
    #[test] fn identical_model_predictions_have_zero_separation() {
        let row=evaluated()["results"].as_array().unwrap().iter().find(|r|r["design_id"]=="MDT-T0-003-rank-deficient").unwrap().clone();
        assert_eq!(row["worst_case_minimum_model_separation"],0.0);
    }
    #[test] fn non_positive_definite_covariance_is_rejected() {
        let mut candidate=load("input")["candidates"][0].clone();
        candidate["strata"][0]["measurement_covariance"]=json!([[1.0,2.0],[2.0,1.0]]);
        assert!(compute_design(&candidate,&[1.0,1.0],2,1.0e-8,8.0,100.0,0.1).is_err());
    }
    #[test] fn row_permutation_with_covariance_and_predictions_is_invariant() {
        let original=load("input")["candidates"][0]["strata"][0].clone();
        let mut permuted=original.clone();
        permuted["sensitivity_matrix"]=json!([[0.0,1.0],[1.0,0.0]]);
        permuted["measurement_covariance"]=json!([[1.0,0.0],[0.0,1.0]]);
        permuted["model_predictions"]=json!({"complete_mixing":[0.5,0.5],"geometry_model":[0.2,0.8]});
        let a=compute_stratum(&original,&[1.0,1.0],2,1.0e-8).unwrap();
        let b=compute_stratum(&permuted,&[1.0,1.0],2,1.0e-8).unwrap();
        assert!((a.1.minimum-b.1.minimum).abs()<1.0e-10); assert!((a.2-b.2).abs()<1.0e-10);
    }
    #[test] fn pareto_frontier_is_deterministic_and_budget_bounded() {
        let value=evaluated(); let ids=value["pareto_frontier_design_ids"].as_array().unwrap();
        assert!(!ids.is_empty());
        for result in value["results"].as_array().unwrap() {
            if result["within_budget"]==true { continue; }
            assert!(!ids.iter().any(|id|id==&result["design_id"]));
        }
    }
}
