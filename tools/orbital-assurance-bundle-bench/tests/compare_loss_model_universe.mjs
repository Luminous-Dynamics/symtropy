import fs from "node:fs";
const [receiptPath,oraclePath]=process.argv.slice(2);
if(!receiptPath||!oraclePath)throw Error("usage: compare_loss_model_universe.mjs <receipt> <oracle>");
const receipt=JSON.parse(fs.readFileSync(receiptPath,"utf8")),oracle=JSON.parse(fs.readFileSync(oraclePath,"utf8"));
function canon(v){if(Array.isArray(v))return v.map(canon);if(v&&typeof v==="object")return Object.fromEntries(Object.keys(v).sort().map(k=>[k,canon(v[k])]));return v;}
function firstDifference(left,right,path="$"){
  if(Array.isArray(left)||Array.isArray(right)){
    if(!Array.isArray(left)||!Array.isArray(right))return {path,rust:left,oracle:right};
    if(left.length!==right.length)return {path:path+".length",rust:left.length,oracle:right.length};
    for(let i=0;i<left.length;i++){const d=firstDifference(left[i],right[i],path+"["+i+"]");if(d)return d;}
    return null;
  }
  const lo=left!==null&&typeof left==="object",ro=right!==null&&typeof right==="object";
  if(lo||ro){
    if(!lo||!ro)return {path,rust:left,oracle:right};
    const keys=[...new Set([...Object.keys(left),...Object.keys(right)])].sort();
    for(const key of keys){
      if(!(key in left)||!(key in right))return {path:path+"."+key,rust:left[key],oracle:right[key]};
      const d=firstDifference(left[key],right[key],path+"."+key);if(d)return d;
    }
    return null;
  }
  return Object.is(left,right)?null:{path,rust:left,oracle:right};
}
const difference=firstDifference(canon(receipt.evaluation),canon(oracle));
if(difference)throw Error("loss-model universe Rust/JS oracle mismatch: "+JSON.stringify(difference));
if(receipt.result!=="pass")throw Error("loss-model universe receipt failed");
process.stdout.write("loss-model-universe-rust-oracle-match\n");
