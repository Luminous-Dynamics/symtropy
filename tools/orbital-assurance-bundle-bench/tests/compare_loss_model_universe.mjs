import fs from "node:fs";
const [receiptPath,oraclePath]=process.argv.slice(2);
if(!receiptPath||!oraclePath)throw Error("usage: compare_loss_model_universe.mjs <receipt> <oracle>");
const receipt=JSON.parse(fs.readFileSync(receiptPath,"utf8")),oracle=JSON.parse(fs.readFileSync(oraclePath,"utf8"));
function canon(v){if(Array.isArray(v))return v.map(canon);if(v&&typeof v==="object")return Object.fromEntries(Object.keys(v).sort().map(k=>[k,canon(v[k])]));return v;}
if(JSON.stringify(canon(receipt.evaluation))!==JSON.stringify(canon(oracle)))throw Error("loss-model universe Rust/JS oracle mismatch");
if(receipt.result!=="pass")throw Error("loss-model universe receipt failed");
process.stdout.write("loss-model-universe-rust-oracle-match\n");
