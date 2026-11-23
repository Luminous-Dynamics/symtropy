// Independent BigInt oracle for fixed-scope assurance-budget composition.
import fs from "node:fs";
import { createHash } from "node:crypto";
import assert from "node:assert/strict";

const MAX_SAFE=9_007_199_254_740_991n;
const REQUIRED=["OBSERVATION_WORLD_COVERAGE","LOSS_MODEL_COVERAGE","REGRET_SCORE_CALIBRATION"];
function fail(message){throw new Error(message);}
function cmpString(a,b){
  const x=Array.from(a,v=>v.codePointAt(0)),y=Array.from(b,v=>v.codePointAt(0));
  for(let i=0;i<Math.min(x.length,y.length);i+=1) if(x[i]!==y[i]) return x[i]-y[i];
  return x.length-y.length;
}
function canonical(value){
  if(Array.isArray(value)) return "["+value.map(canonical).join(",")+"]";
  if(value!==null&&typeof value==="object")
    return "{"+Object.keys(value).sort(cmpString).map(k=>JSON.stringify(k)+":"+canonical(value[k])).join(",")+"}";
  return JSON.stringify(value);
}
function sha(value){return createHash("sha256").update(value,"utf8").digest("hex");}
function id(value,path){
  if(typeof value!=="string"||!value.trim()||!/^[\x00-\x7F]+$/.test(value)) fail(path+": expected nonempty ASCII ID");
}
function rational(value,path){
  if(!Number.isSafeInteger(value?.numerator)||!Number.isSafeInteger(value?.denominator)||
     value.numerator<0||value.denominator<=0||
     BigInt(value.numerator)>MAX_SAFE||BigInt(value.denominator)>MAX_SAFE||
     value.numerator>value.denominator) fail(path+": invalid rational");
  let n=BigInt(value.numerator),d=BigInt(value.denominator);
  const gcd=(a,b)=>{while(b!==0n){const r=a%b;a=b;b=r;}return a>0n?a:1n;};
  const g=gcd(n,d);return {n:n/g,d:d/g};
}
function cmpR(a,b){const x=a.n*b.d,y=b.n*a.d;return x<y?-1:x>y?1:0;}
function add(a,b){
  const n=a.n*b.d+b.n*a.d,d=a.d*b.d;
  const gcd=(x,y)=>{while(y!==0n){const r=x%y;x=y;y=r;}return x>0n?x:1n;};
  const g=gcd(n,d);return {n:n/g,d:d/g};
}
function sub(a,b){
  const n=a.n*b.d-b.n*a.d,d=a.d*b.d;
  if(n<0n) fail("rational subtraction underflow");
  const gcd=(x,y)=>{while(y!==0n){const r=x%y;x=y;y=r;}return x>0n?x:1n;};
  const g=gcd(n,d);return {n:n/g,d:d/g};
}
function jsonR(a){return {numerator:Number(a.n),denominator:Number(a.d)};}
function idsDigest(ids){return sha(JSON.stringify([...ids].sort(cmpString)));}
function validate(c){
  if(c?.schema!=="luminous.orbital-assurance-budget.corpus.v0.1"||
     c.evidence_class!=="synthetic"||
     c.claim_ceiling!=="exact-union-bound-assurance-budget-algebra-only"||
     typeof c.assurance_budget_id!=="string"||
     !["FIXED_EVALUATION","ANYTIME"].includes(c.requested_scope)||
     !Array.isArray(c.claims)||!c.claims.length||c.claims.length>128) fail("unsupported assurance budget corpus");
  id(c.assurance_budget_id,"assurance_budget_id");id(c.requested_scope_id,"requested_scope_id");
  const total=rational(c.total_alpha,"total_alpha");
  if(total.n===0n||total.n>=total.d) fail("total alpha must be strictly between zero and one");
  const planned=[...c.expected_claim_ids].sort(cmpString);
  if(!planned.length||new Set(planned).size!==planned.length) fail("invalid planned claim IDs");
  for(const claimId of planned) id(claimId,"expected_claim_ids");
  if(c.expected_claim_ids_sha256!==idsDigest(planned)) fail("planned claim ID digest mismatch");
  const observed=new Set(),receipts=new Set();
  for(const claim of c.claims){
    id(claim.claim_id,"claim_id");id(claim.scope_id,"scope_id");
    if(observed.has(claim.claim_id)) fail("duplicate claim ID");
    observed.add(claim.claim_id);
    if(!/^[0-9a-f]{64}$/.test(claim.source_contract_sha256??"")) fail("invalid source contract SHA-256");
    if(!["OBSERVATION_WORLD_COVERAGE","LOSS_MODEL_COVERAGE","REGRET_SCORE_CALIBRATION","TRANSPORTABILITY","MULTIPLICITY_FAMILY"].includes(claim.claim_class)) fail("invalid claim class");
    if(!["STATISTICAL_GUARANTEE","STRUCTURAL_ONLY"].includes(claim.guarantee_kind)) fail("invalid guarantee kind");
    if(!["FIXED_EVALUATION","ANYTIME"].includes(claim.claim_scope)) fail("invalid claim scope");
    if(!["admissible","indeterminate","out_of_scope"].includes(claim.evidence_status)) fail("invalid evidence status");
    if(claim.guarantee_kind==="STATISTICAL_GUARANTEE"){
      const a=rational(claim.alpha,"claim alpha");
      if(a.n===0n||a.n>=a.d) fail("claim alpha must be in (0,1)");
      for(const [v,k] of [[claim.evidence_receipt_id,"evidence"],[claim.claim_receipt_id,"claim receipt"],
        [claim.multiplicity_family_id,"multiplicity family"],[claim.multiplicity_allocation_receipt_id,"allocation receipt"]]) id(v,k);
      if(receipts.has(claim.claim_receipt_id)) fail("duplicate statistical claim receipt");
      receipts.add(claim.claim_receipt_id);
    }else{
      if(claim.alpha!==null||claim.claim_receipt_id!==null||claim.multiplicity_allocation_receipt_id!==null)
        fail("structural-only claim must not carry statistical allocation");
      id(claim.evidence_receipt_id,"structural evidence receipt");
    }
  }
  if(observed.size!==planned.length||JSON.stringify([...observed].sort(cmpString))!==JSON.stringify(planned))
    fail("observed claim IDs do not match planned inventory");
  if(c.claims_sha256!==sha(canonical([...c.claims].sort((a,b)=>cmpString(a.claim_id,b.claim_id)))))
    fail("claims digest mismatch");
  return total;
}
function evaluate(c){
  const total=validate(c);
  const claims=[...c.claims].sort((a,b)=>cmpString(a.claim_id,b.claim_id));
  let allocated={n:0n,d:1n};
  for(const claim of claims) if(claim.guarantee_kind==="STATISTICAL_GUARANTEE") allocated=add(allocated,rational(claim.alpha,"alpha"));
  const reports=[],noncomp=[],classes=new Set();
  let scopeMismatch=false,evidenceBad=false;
  for(const claim of claims){
    let reason=null;
    if(claim.guarantee_kind==="STRUCTURAL_ONLY") reason="STRUCTURAL_ONLY_NOT_A_STATISTICAL_GUARANTEE";
    else if(claim.evidence_status!=="admissible") reason="EVIDENCE_"+claim.evidence_status.toUpperCase();
    else if(claim.scope_id!==c.requested_scope_id) reason="SCOPE_ID_MISMATCH";
    else if(c.requested_scope==="ANYTIME"&&claim.claim_scope!=="ANYTIME") reason="FIXED_TIME_CLAIM_CANNOT_SUPPORT_ANYTIME_REQUEST";
    const composable=reason===null;
    if(!composable){
      noncomp.push(claim.claim_id);
      if(reason==="SCOPE_ID_MISMATCH"||reason==="FIXED_TIME_CLAIM_CANNOT_SUPPORT_ANYTIME_REQUEST") scopeMismatch=true;
      if(reason?.startsWith("EVIDENCE_")) evidenceBad=true;
    }else classes.add(claim.claim_class);
    reports.push({
      claim_id:claim.claim_id,claim_class:claim.claim_class,guarantee_kind:claim.guarantee_kind,
      claim_scope:claim.claim_scope,evidence_status:claim.evidence_status,
      composable,exclusion_reason:reason,alpha:claim.alpha===null?null:jsonR(rational(claim.alpha,"alpha"))
    });
  }
  const missing=REQUIRED.filter(v=>!classes.has(v));
  let status,used=null,remaining=null,lower=null;
  if(scopeMismatch) status="ASSURANCE_SCOPE_MISMATCH";
  else if(evidenceBad||missing.length) status="ASSURANCE_INDETERMINATE";
  else if(cmpR(allocated,total)>0){status="ASSURANCE_BUDGET_EXCEEDED";used=jsonR(allocated);}
  else{
    status=c.requested_scope==="ANYTIME"?"ASSURANCE_COMPOSED_ANYTIME":"ASSURANCE_COMPOSED_FIXED_EVALUATION";
    used=jsonR(allocated);remaining=jsonR(sub(total,allocated));lower=jsonR(sub({n:1n,d:1n},allocated));
  }
  return {
    result:"pass",evidence_class:c.evidence_class,claim_ceiling:c.claim_ceiling,
    assurance_budget_id:c.assurance_budget_id,requested_scope:c.requested_scope,
    requested_scope_id:c.requested_scope_id,status,total_alpha:jsonR(total),
    alpha_allocated:jsonR(allocated),alpha_used:used,alpha_remaining:remaining,
    joint_lower_confidence:lower,missing_required_classes:missing,
    noncomposable_claim_ids:noncomp,claim_reports:reports,
    independence_assumed:false,physical_qualification_granted:false,authority_granted:false
  };
}
try{
 const [path]=process.argv.slice(2); if(!path) fail("usage: assurance_budget_oracle.mjs <corpus.json>");
 const input=JSON.parse(fs.readFileSync(path,"utf8"));
 const output=evaluate(input);
 const reversed=structuredClone(input);reversed.claims.reverse();reversed.expected_claim_ids.reverse();
 assert.deepStrictEqual(evaluate(reversed),output,"assurance budget result changed under claim ordering");
 process.stdout.write(JSON.stringify(output,null,2)+"\n");
}catch(error){process.stderr.write(String(error?.message??error)+"\n");process.exitCode=1;}
