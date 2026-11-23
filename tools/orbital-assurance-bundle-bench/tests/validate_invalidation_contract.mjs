import fs from "node:fs";

const [schemaPath, corpusPath] = process.argv.slice(2);
if (!schemaPath || !corpusPath) {
  throw new Error("usage: validate_invalidation_contract.mjs <schema.json> <corpus.json>");
}
const schema = JSON.parse(fs.readFileSync(schemaPath, "utf8"));
const corpus = JSON.parse(fs.readFileSync(corpusPath, "utf8"));
if (schema.$schema !== "https://json-schema.org/draft/2020-12/schema"
  || schema.properties?.schema?.const !== "luminous.orbital-assurance-invalidation.corpus.v0.1"
  || schema.properties?.evidence_class?.const !== "synthetic"
  || schema.properties?.claim_ceiling?.const !== "synthetic-dependency-invalidation-only") {
  throw new Error("invalidation corpus schema contract is malformed");
}
if (corpus.schema !== schema.properties.schema.const
  || corpus.evidence_class !== schema.properties.evidence_class.const
  || corpus.claim_ceiling !== schema.properties.claim_ceiling.const
  || !Array.isArray(corpus.cases) || corpus.cases.length !== 7) {
  throw new Error("invalidation corpus does not match its v0.1 contract");
}
const ids = new Set();
for (const item of corpus.cases) {
  if (typeof item.case_id !== "string" || !item.case_id || ids.has(item.case_id)
    || !Array.isArray(item.nodes) || !item.nodes.length
    || !Array.isArray(item.required_root_node_ids) || !item.required_root_node_ids.length
    || !Array.isArray(item.expected_invalidated_node_ids)) {
    throw new Error("invalid invalidation case structure or duplicate case ID");
  }
  ids.add(item.case_id);
  for (const node of item.nodes) {
    if (!/^[0-9a-f]{64}$/.test(node.content_sha256)
      || !/^[0-9a-f]{64}$/.test(node.current_content_sha256)
      || !/^[0-9a-f]{64}$/.test(node.scope_sha256)
      || !/^[0-9a-f]{64}$/.test(node.current_scope_sha256)
      || !Array.isArray(node.dependencies)) {
      throw new Error("invalid node version binding in " + item.case_id);
    }
  }
}
process.stdout.write("dependency-invalidation-corpus-contract-valid: " + corpus.cases.length + " cases\n");
