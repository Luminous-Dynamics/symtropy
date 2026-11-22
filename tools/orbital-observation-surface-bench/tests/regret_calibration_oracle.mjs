// Independent BigInt oracle for per-regime finite-sample regret calibration.
import fs from "node:fs";
import { createHash } from "node:crypto";
import assert from "node:assert/strict";

const MAX_SAFE = 9_007_199_254_740_991n;
function fail(message) { throw new Error(message); }
function codePointCompare(left, right) {
  const a=Array.from(left,value=>value.codePointAt(0));
  const b=Array.from(right,value=>value.codePointAt(0));
  for(let i=0;i<Math.min(a.length,b.length);i+=1) if(a[i]!==b[i]) return a[i]-b[i];
  return a.length-b.length;
}
function canonical(value) {
  if(Array.isArray(value)) return "["+value.map(canonical).join(",")+"]";
  if(value!==null && typeof value==="object") {
    return "{"+Object.keys(value).sort(codePointCompare).map(key=>JSON.stringify(key)+":"+canonical(value[key])).join(",")+"}";
  }
  return JSON.stringify(value);
}
function sha256(value) { return createHash("sha256").update(value,"utf8").digest("hex"); }
function gcd(a,b) { while(b!==0n){const r=a%b;a=b;b=r;} return a>0n?a:1n; }
function rational(value,path) {
  if(!Number.isSafeInteger(value?.numerator)||!Number.isSafeInteger(value?.denominator)||
     value.numerator<0||value.denominator<=0||
     BigInt(value.numerator)>MAX_SAFE||BigInt(value.denominator)>MAX_SAFE||
     value.numerator>value.denominator) fail(path+": invalid exact [0,1] rational");
  const n=BigInt(value.numerator),d=BigInt(value.denominator),g=gcd(n,d);
  return {n:n/g,d:d/g};
}
function compare(a,b) {
  const x=a.n*b.d,y=b.n*a.d;
  return x<y?-1:x>y?1:0;
}
function asJson(value){return {numerator:Number(value.n),denominator:Number(value.d)};}
function id(value,path){
  if(typeof value!=="string"||!value.trim()||!/^[\x00-\x7F]+$/.test(value)) fail(path+": expected nonempty ASCII ID");
}
function digestIds(ids){return sha256(JSON.stringify([...ids].sort(codePointCompare)));}
function rank(n,alpha){
  const numerator=BigInt(n+1)*(alpha.d-alpha.n);
  return Number((numerator+alpha.d-1n)/alpha.d);
}
function validate(corpus){
  if(corpus?.schema!=="luminous.orbital-regret-calibration.corpus.v0.1"||
     corpus?.evidence_class!=="synthetic"||
     corpus?.claim_ceiling!=="finite-split-conformal-regret-rank-algebra-only"||
     !["predeclared"].includes(corpus.selection_status)||
     corpus.stopping_rule!=="fixed"||
     !Array.isArray(corpus.regimes)||corpus.regimes.length<1||corpus.regimes.length>64) fail("unsupported calibration corpus or claim ceiling");
  for(const key of ["calibration_split_id","score_contract_id","score_contract_receipt_id"]) id(corpus[key],"corpus."+key);
  const alpha=rational(corpus.alpha,"alpha");
  if(alpha.n===0n||alpha.n===alpha.d) fail("alpha must be strictly between zero and one");
  const hasBound=corpus.score_support_upper_bound!==null;
  if(hasBound){
    rational(corpus.score_support_upper_bound,"score_support_upper_bound");
    id(corpus.score_support_receipt_id,"score_support_receipt_id");
  } else if(corpus.score_support_receipt_id!==null) fail("support receipt supplied without bound");
  const expected=[...corpus.expected_regime_ids].sort(codePointCompare);
  if(!expected.length||new Set(expected).size!==expected.length) fail("invalid expected regime inventory");
  for(const regimeId of expected) id(regimeId,"expected_regime_ids");
  if(corpus.expected_regime_ids_sha256!==digestIds(expected)) fail("expected regime inventory digest mismatch");
  const observed=new Set(),globalSamples=new Set(),globalUnits=new Set();
  for(const [ri,regime] of corpus.regimes.entries()){
    id(regime.regime_id,"regime["+ri+"].regime_id");
    if(observed.has(regime.regime_id)) fail("duplicate regime ID");
    observed.add(regime.regime_id);
    if(!Array.isArray(regime.samples)||regime.samples.length<1||regime.samples.length>100000||
       !Array.isArray(regime.expected_sample_ids)||regime.expected_sample_ids.length!==regime.samples.length) fail("invalid calibration sample count");
    const expectedSamples=[...regime.expected_sample_ids].sort(codePointCompare);
    for(const sampleId of expectedSamples) id(sampleId,"expected sample ID");
    if(new Set(expectedSamples).size!==expectedSamples.length) fail("duplicate expected sample ID");
    if(regime.expected_sample_ids_sha256!==digestIds(expectedSamples)) fail("expected sample inventory digest mismatch");
    const sampleIds=new Set(),units=new Set();
    for(const sample of regime.samples){
      id(sample.sample_id,"sample_id");id(sample.replication_unit_id,"replication_unit_id");
      if(sampleIds.has(sample.sample_id)||units.has(sample.replication_unit_id)||
         globalSamples.has(sample.sample_id)||globalUnits.has(sample.replication_unit_id)) fail("sample or replication unit duplicated");
      sampleIds.add(sample.sample_id);units.add(sample.replication_unit_id);
      globalSamples.add(sample.sample_id);globalUnits.add(sample.replication_unit_id);
      const score=rational(sample.regret_upper_bound_score,"regret_upper_bound_score");
      if(hasBound && compare(score,rational(corpus.score_support_upper_bound,"support bound"))>0) fail("score exceeds declared support bound");
    }
    if(JSON.stringify([...sampleIds].sort(codePointCompare))!==JSON.stringify(expectedSamples)) fail("observed sample IDs differ from planned inventory");
    if(regime.samples_sha256!==sha256(canonical([...regime.samples].sort((a,b)=>codePointCompare(a.sample_id,b.sample_id))))) fail("sample record digest mismatch");
  }
  if(JSON.stringify([...observed].sort(codePointCompare))!==JSON.stringify(expected)) fail("observed regime IDs differ from planned inventory");
  return alpha;
}
function evaluateRegime(regime,alpha,supportBound){
  const k=rank(regime.samples.length,alpha);
  const sorted=[...regime.samples].sort((a,b)=>
    compare(rational(a.regret_upper_bound_score,"score"),rational(b.regret_upper_bound_score,"score"))||
    codePointCompare(a.sample_id,b.sample_id));
  if(k<=sorted.length){
    const chosen=sorted[k-1];
    return {regime_id:regime.regime_id,sample_count:sorted.length,rank:k,
      threshold_status:"EMPIRICAL_ORDER_STATISTIC",
      threshold:asJson(rational(chosen.regret_upper_bound_score,"selected score")),
      selected_sample_id:chosen.sample_id};
  }
  if(supportBound!==null){
    return {regime_id:regime.regime_id,sample_count:sorted.length,rank:k,
      threshold_status:"SUPPORT_BOUND_ONLY",threshold:asJson(rational(supportBound,"support bound")),
      selected_sample_id:null};
  }
  return {regime_id:regime.regime_id,sample_count:sorted.length,rank:k,
    threshold_status:"NO_FINITE_NONTRIVIAL_THRESHOLD",threshold:null,selected_sample_id:null};
}
function evaluate(corpus){
  const alpha=validate(corpus);
  const regimes=corpus.regimes.map(regime=>evaluateRegime(regime,alpha,corpus.score_support_upper_bound))
    .sort((a,b)=>codePointCompare(a.regime_id,b.regime_id));
  const without=regimes.filter(item=>item.threshold===null).map(item=>item.regime_id);
  let globalStatus,threshold,worst;
  if(without.length){globalStatus="NO_FINITE_NONTRIVIAL_THRESHOLD";threshold=null;worst=[];}
  else{
    let best=rational(regimes[0].threshold,"threshold");
    for(const item of regimes.slice(1)){
      const x=rational(item.threshold,"threshold");
      if(compare(x,best)>0) best=x;
    }
    threshold=asJson(best);
    worst=regimes.filter(item=>compare(rational(item.threshold,"threshold"),best)===0).map(item=>item.regime_id);
    globalStatus=regimes.some(item=>item.threshold_status==="SUPPORT_BOUND_ONLY")
      ?"WORST_REGIME_SUPPORT_BOUND_ONLY":"WORST_REGIME_EMPIRICAL_THRESHOLD";
  }
  return {
    result:"pass",
    evidence_class:corpus.evidence_class,
    claim_ceiling:corpus.claim_ceiling,
    calibration_split_id:corpus.calibration_split_id,
    score_contract_id:corpus.score_contract_id,
    alpha:asJson(alpha),
    regime_count:regimes.length,
    total_sample_count:regimes.reduce((sum,item)=>sum+item.sample_count,0),
    global_status:globalStatus,
    worst_regime_threshold:threshold,
    worst_regime_ids:worst,
    regimes_without_finite_nontrivial_threshold:without,
    regime_results:regimes,
    coverage_claim:"marginal_per_declared_regime_under_within_regime_exchangeability_only",
    physical_qualification_granted:false,
    authority_granted:false
  };
}
try{
  const [corpusPath]=process.argv.slice(2);
  if(!corpusPath) fail("usage: regret_calibration_oracle.mjs <corpus.json>");
  const corpus=JSON.parse(fs.readFileSync(corpusPath,"utf8"));
  const output=evaluate(corpus);
  const reversed=structuredClone(corpus);
  reversed.expected_regime_ids.reverse();
  reversed.regimes.reverse();
  for(const regime of reversed.regimes){
    regime.expected_sample_ids.reverse();
    regime.samples.reverse();
  }
  assert.deepStrictEqual(evaluate(reversed),output,"calibration output changed under regime/sample ordering");
  process.stdout.write(JSON.stringify(output,null,2)+"\n");
}catch(error){
  process.stderr.write(String(error?.message??error)+"\n");
  process.exitCode=1;
}
