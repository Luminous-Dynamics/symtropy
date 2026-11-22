// Dependency-free validator for the JSON Schema keywords used by this repository.
// This intentionally is not a general-purpose Draft 2020-12 implementation.
import fs from "node:fs";

function fail(message) {
  throw new Error(message);
}

function typeMatches(type, value) {
  switch (type) {
    case "object": return value !== null && typeof value === "object" && !Array.isArray(value);
    case "array": return Array.isArray(value);
    case "string": return typeof value === "string";
    case "number": return typeof value === "number" && Number.isFinite(value);
    case "integer": return Number.isSafeInteger(value);
    case "boolean": return typeof value === "boolean";
    case "null": return value === null;
    default: fail(`unsupported JSON Schema type: ${type}`);
  }
}

function resolvePointer(root, pointer) {
  if (!pointer.startsWith("#/")) fail(`unsupported schema reference: ${pointer}`);
  return pointer.slice(2).split("/").map(part => part.replaceAll("~1", "/").replaceAll("~0", "~"))
    .reduce((value, key) => {
      if (value === undefined || value === null || !(key in value)) fail(`unresolved schema reference: ${pointer}`);
      return value[key];
    }, root);
}

function validate(schema, value, path, root) {
  if (schema.$ref) return validate(resolvePointer(root, schema.$ref), value, path, root);

  if (schema.anyOf) {
    const errors = [];
    for (const candidate of schema.anyOf) {
      try {
        validate(candidate, value, path, root);
        return;
      } catch (error) {
        errors.push(error.message);
      }
    }
    fail(`${path}: failed anyOf (${errors.join(" | ")})`);
  }

  if (schema.type && !typeMatches(schema.type, value)) {
    fail(`${path}: expected ${schema.type}`);
  }
  if ("const" in schema && JSON.stringify(value) !== JSON.stringify(schema.const)) {
    fail(`${path}: expected constant ${JSON.stringify(schema.const)}`);
  }
  if (schema.enum && !schema.enum.some(candidate => JSON.stringify(candidate) === JSON.stringify(value))) {
    fail(`${path}: value is outside enum`);
  }

  if (typeof value === "number") {
    if (schema.minimum !== undefined && value < schema.minimum) fail(`${path}: below minimum`);
    if (schema.maximum !== undefined && value > schema.maximum) fail(`${path}: above maximum`);
    if (schema.exclusiveMinimum !== undefined && value <= schema.exclusiveMinimum) fail(`${path}: not above exclusive minimum`);
    if (schema.exclusiveMaximum !== undefined && value >= schema.exclusiveMaximum) fail(`${path}: not below exclusive maximum`);
  }

  if (typeof value === "string") {
    if (schema.minLength !== undefined && value.length < schema.minLength) fail(`${path}: shorter than minLength`);
    if (schema.maxLength !== undefined && value.length > schema.maxLength) fail(`${path}: longer than maxLength`);
    if (schema.pattern && !(new RegExp(schema.pattern)).test(value)) fail(`${path}: pattern mismatch`);
  }

  if (Array.isArray(value)) {
    if (schema.minItems !== undefined && value.length < schema.minItems) fail(`${path}: fewer than minItems`);
    if (schema.maxItems !== undefined && value.length > schema.maxItems) fail(`${path}: more than maxItems`);
    if (schema.items) value.forEach((item, index) => validate(schema.items, item, `${path}[${index}]`, root));
  }

  if (value !== null && typeof value === "object" && !Array.isArray(value)) {
    for (const key of schema.required ?? []) {
      if (!(key in value)) fail(`${path}: missing required property ${key}`);
    }
    if (schema.additionalProperties === false) {
      for (const key of Object.keys(value)) {
        if (!(key in (schema.properties ?? {}))) fail(`${path}: unexpected property ${key}`);
      }
    } else if (schema.additionalProperties && typeof schema.additionalProperties === "object") {
      for (const [key, childValue] of Object.entries(value)) {
        if (!(key in (schema.properties ?? {}))) {
          validate(schema.additionalProperties, childValue, `${path}.${key}`, root);
        }
      }
    }
    for (const [key, childSchema] of Object.entries(schema.properties ?? {})) {
      if (key in value) validate(childSchema, value[key], `${path}.${key}`, root);
    }
  }
}

try {
  const [schemaPath, dataPath] = process.argv.slice(2);
  if (!schemaPath || !dataPath) fail("usage: validate_schema.mjs <schema.json> <data.json>");
  const schema = JSON.parse(fs.readFileSync(schemaPath, "utf8"));
  const data = JSON.parse(fs.readFileSync(dataPath, "utf8"));
  if (schema.$schema !== "https://json-schema.org/draft/2020-12/schema") fail("unexpected JSON Schema dialect");
  validate(schema, data, "$", schema);
  process.stdout.write(`schema-valid: ${dataPath}\n`);
} catch (error) {
  process.stderr.write(String(error?.message ?? error) + "\n");
  process.exitCode = 1;
}
