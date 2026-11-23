import fs from "node:fs";

const [schemaPath, corpusPath] = process.argv.slice(2);
if (!schemaPath || !corpusPath) {
  throw new Error("usage: validate_dependency_graph_contract.mjs <schema> <corpus>");
}
const schema = JSON.parse(fs.readFileSync(schemaPath, "utf8"));
const corpus = JSON.parse(fs.readFileSync(corpusPath, "utf8"));
if (schema.$schema !== "https://json-schema.org/draft/2020-12/schema"
  || schema.properties?.schema?.const !== "luminous.orbital-dependency-graph-adequacy.corpus.v0.1"
  || schema.properties?.evidence_class?.const !== "synthetic"
  || schema.properties?.claim_ceiling?.const !== "synthetic-dependency-graph-adequacy-only") {
  throw new Error("dependency graph corpus schema contract is malformed");
}
if (corpus.schema !== schema.properties.schema.const
  || corpus.evidence_class !== schema.properties.evidence_class.const
  || corpus.claim_ceiling !== schema.properties.claim_ceiling.const
  || !Array.isArray(corpus.cases) || corpus.cases.length !== 7) {
  throw new Error("dependency graph corpus does not match its v0.1 contract");
}
const ids = new Set();
for (const item of corpus.cases) {
  if (typeof item.case_id !== "string" || !item.case_id || ids.has(item.case_id)
    || !Array.isArray(item.producers) || !item.producers.length
    || !Array.isArray(item.consumers) || !item.consumers.length
    || !Array.isArray(item.declared_edges) || !item.expected) {
    throw new Error("invalid dependency graph case structure or duplicate ID");
  }
  ids.add(item.case_id);
  const artifacts = new Set();
  for (const producer of item.producers) {
    if (!/^[0-9a-f]{64}$/.test(producer.content_sha256)
      || !/^[0-9a-f]{64}$/.test(producer.scope_sha256)
      || !producer.node_id || !producer.artifact_id || artifacts.has(producer.artifact_id)) {
      throw new Error("invalid producer artifact binding in " + item.case_id);
    }
    artifacts.add(producer.artifact_id);
  }
  for (const consumer of item.consumers) {
    if (!/^[0-9a-f]{64}$/.test(consumer.scope_sha256)
      || !Array.isArray(consumer.input_artifact_ids)) {
      throw new Error("invalid consumer manifest in " + item.case_id);
    }
  }
}
process.stdout.write("dependency-graph-adequacy-corpus-valid: " + corpus.cases.length + " cases\n");
