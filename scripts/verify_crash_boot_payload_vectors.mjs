#!/usr/bin/env node
// Independent Node.js cross-check for the synthetic CRASH-BOOT payload vectors.
// Test tooling only: it does not authenticate issuers or establish physical truth.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';

const BOUND_FIELDS = [
  'evidence_id', 'issuer_id', 'asset_id', 'asset_revision', 'configuration_digest',
  'scope_id', 'observed_at', 'valid_until', 'lifecycle_state', 'method_id',
  'method_qualification', 'calibration_id', 'calibration_status',
  'acceptance_criterion_id', 'measured_value', 'unit', 'acceptance_min', 'acceptance_max',
];
const CONTROL_SPEC = {
  rehash_whitespace: ['insert_space_after_open_brace', true, 'REJECT_ARTIFACT_PAYLOAD_NONCANONICAL'],
  rehash_duplicate_member: ['duplicate_asset_id_member', true, 'REJECT_ARTIFACT_PAYLOAD_NONCANONICAL'],
  rehash_nonfinite_number: ['replace_measurement_with_nan', true, 'REJECT_ARTIFACT_PAYLOAD_INVALID'],
  rehash_negative_zero: ['replace_measurement_with_negative_zero', true, 'REJECT_ARTIFACT_PAYLOAD_NONCANONICAL'],
  invalid_utf8_string: ['set_payload_to_lone_surrogate', false, 'REJECT_ARTIFACT_PAYLOAD_INVALID'],
  invalid_json_syntax: ['replace_open_brace_with_x', true, 'REJECT_ARTIFACT_PAYLOAD_INVALID'],
  overflowing_number: ['replace_measurement_with_overflow_exponent', true, 'REJECT_ARTIFACT_PAYLOAD_INVALID'],
  excessive_nesting: ['replace_payload_with_deeply_nested_array', true, 'REJECT_ARTIFACT_PAYLOAD_INVALID'],
  oversized_claim_payload: ['pad_payload_beyond_declared_byte_limit', true, 'REJECT_ARTIFACT_PAYLOAD_TOO_LARGE'],
};
const JCS_SPEC = [
  {
    id: 'rfc8785_number_serialization',
    input_json: '{"numbers":[333333333.33333329,1E30,4.50,2e-3,0.000000000000000000000000001]}',
    expected_canonical_json: '{"numbers":[333333333.3333333,1e+30,4.5,0.002,1e-27]}',
  },
  {
    id: 'jcs_utf16_key_order',
    input_json: '{"\\ue000":1,"😀":2}',
    expected_canonical_json: '{"😀":2,"":1}',
  },
  {
    id: 'jcs_negative_zero',
    input_json: '-0',
    expected_canonical_json: '0',
  },
];
const MAX_DEPTH = 128;

function sha256(bytes) {
  return 'sha256:' + createHash('sha256').update(bytes).digest('hex');
}

function hasUnpairedSurrogate(text) {
  for (let i = 0; i < text.length; i += 1) {
    const code = text.charCodeAt(i);
    if (code >= 0xd800 && code <= 0xdbff) {
      if (i + 1 >= text.length) return true;
      const next = text.charCodeAt(i + 1);
      if (next < 0xdc00 || next > 0xdfff) return true;
      i += 1;
    } else if (code >= 0xdc00 && code <= 0xdfff) {
      return true;
    }
  }
  return false;
}

// RFC 8785 serialization primitives as implemented by ECMAScript, with explicit
// Unicode-scalar and depth guards. This helper is not presented as full JCS
// conformance; only the listed known-answer and fixture vectors are exercised.
function canonicalize(value, depth = 0) {
  if (depth > MAX_DEPTH) throw new Error('JSON nesting depth exceeded');
  if (value === null || typeof value === 'boolean') return JSON.stringify(value);
  if (typeof value === 'string') {
    if (hasUnpairedSurrogate(value)) throw new Error('invalid Unicode scalar sequence');
    return JSON.stringify(value);
  }
  if (typeof value === 'number') {
    if (!Number.isFinite(value)) throw new Error('non-finite number');
    return JSON.stringify(value); // ECMAScript JSON serialization renders -0 as 0.
  }
  if (Array.isArray(value)) {
    return '[' + value.map((item) => canonicalize(item, depth + 1)).join(',') + ']';
  }
  if (typeof value === 'object') {
    const keys = Object.keys(value);
    for (const key of keys) {
      if (hasUnpairedSurrogate(key)) throw new Error('invalid Unicode key');
    }
    // ECMAScript sort compares UTF-16 code units, as required by RFC 8785.
    keys.sort();
    return '{' + keys.map((key) => JSON.stringify(key) + ':' + canonicalize(value[key], depth + 1)).join(',') + '}';
  }
  throw new Error('unsupported JSON value');
}

function classifyPayload(payload, artifactDigest, record, byteLimit) {
  if (typeof payload !== 'string' || hasUnpairedSurrogate(payload)) {
    return 'REJECT_ARTIFACT_PAYLOAD_INVALID';
  }
  const bytes = Buffer.from(payload, 'utf8');
  if (!Number.isSafeInteger(byteLimit) || byteLimit <= 0) return 'REJECT_ARTIFACT_PAYLOAD_INVALID';
  if (bytes.byteLength > byteLimit) return 'REJECT_ARTIFACT_PAYLOAD_TOO_LARGE';
  if (sha256(bytes) !== artifactDigest) return 'REJECT_ARTIFACT_DIGEST_MISMATCH';

  let parsed;
  try {
    parsed = JSON.parse(payload);
  } catch {
    return 'REJECT_ARTIFACT_PAYLOAD_INVALID';
  }
  let canonical;
  try {
    canonical = canonicalize(parsed);
  } catch {
    return 'REJECT_ARTIFACT_PAYLOAD_INVALID';
  }
  if (canonical !== payload) return 'REJECT_ARTIFACT_PAYLOAD_NONCANONICAL';
  if (parsed === null || typeof parsed !== 'object' || Array.isArray(parsed)) {
    return 'REJECT_ARTIFACT_PAYLOAD_INVALID';
  }
  if (Object.keys(parsed).length !== BOUND_FIELDS.length) return 'REJECT_ARTIFACT_CLAIM_MISMATCH';
  for (const field of BOUND_FIELDS) {
    if (!Object.hasOwn(parsed, field) || parsed[field] !== record[field]) {
      return 'REJECT_ARTIFACT_CLAIM_MISMATCH';
    }
  }
  return 'VERIFIED_AVAILABLE_CANDIDATE';
}

function mutatePayload(mutation, base, limit) {
  switch (mutation) {
    case 'insert_space_after_open_brace': return base.replace('{', '{ ');
    case 'duplicate_asset_id_member': return base.replace(
      '"asset_id":"water-pump-7",',
      '"asset_id":"water-pump-7","asset_id":"water-pump-7",',
    );
    case 'replace_measurement_with_nan': return base.replace('"measured_value":1,', '"measured_value":NaN,');
    case 'replace_measurement_with_negative_zero': return base.replace('"measured_value":1,', '"measured_value":-0,');
    case 'set_payload_to_lone_surrogate': return '\ud800';
    case 'replace_open_brace_with_x': return base.replace('{', 'x');
    case 'replace_measurement_with_overflow_exponent': return base.replace('"measured_value":1,', '"measured_value":1e9999,');
    case 'replace_payload_with_deeply_nested_array': return '['.repeat(2000) + '0' + ']'.repeat(2000);
    case 'pad_payload_beyond_declared_byte_limit': {
      const length = Buffer.byteLength(base, 'utf8');
      return base + ' '.repeat(Math.max(1, limit + 1 - length));
    }
    default: throw new Error('unknown mutation operation: ' + mutation);
  }
}

function run() {
  const fixturePath = process.argv[2] ?? 'docs/settlement-bootstrap/fixtures/resource_flow_reference_v0.1.json';
  const fixture = JSON.parse(readFileSync(fixturePath, 'utf8'));
  assert.equal(fixture.authority, 'NON_AUTHORITATIVE_SYNTHETIC_REFERENCE');
  const evidence = fixture.service_evidence_reference;
  const binding = evidence.artifact_binding_rule;
  const byteLimit = evidence.target.max_artifact_payload_bytes;
  assert.equal(byteLimit, 65536, 'claim payload byte cap changed without a reviewed contract update');

  const integrity = fixture.artifact_integrity_reference;
  assert.equal(sha256(Buffer.from(integrity.payload_utf8, 'utf8')), 'sha256:' + integrity.expected_sha256);
  assert.equal(
    integrity.altered_payload_utf8,
    integrity.payload_utf8.replace('measurement=1.0\\n', 'measurement=1.1\\n'),
    'evidence canary mutation must change only the declared measurement field',
  );
  assert.notEqual(sha256(Buffer.from(integrity.altered_payload_utf8, 'utf8')), 'sha256:' + integrity.expected_sha256);
  const configurationHash = sha256(Buffer.from(integrity.configuration_payload_utf8, 'utf8'));
  assert.equal(configurationHash, 'sha256:' + integrity.expected_configuration_sha256);
  assert.equal(
    integrity.altered_configuration_payload_utf8,
    integrity.configuration_payload_utf8.replace(
      'scope=habitat-central-water-loop\\n',
      'scope=habitat-central-water-loop-mutated\\n',
    ),
    'configuration canary mutation must change only the declared scope field',
  );
  assert.notEqual(
    sha256(Buffer.from(integrity.altered_configuration_payload_utf8, 'utf8')),
    configurationHash,
    'one-field configuration mutation must change the pinned SHA-256 digest',
  );
  assert.ok(integrity.configuration_mutation_rule.toLowerCase().includes('scope bytes must change'));
  assert.equal(
    evidence.target.configuration_digest,
    configurationHash,
    'service-evidence target must use the digest of the pinned configuration bytes',
  );

  const jcsVectors = binding.jcs_known_answer_vectors;
  assert.deepEqual(jcsVectors, JCS_SPEC, 'JCS known-answer inputs/outputs changed without review');
  for (const vector of JCS_SPEC) {
    const result = canonicalize(JSON.parse(vector.input_json));
    assert.equal(result, vector.expected_canonical_json, vector.id + ': canonical bytes differ');
  }
  assert.throws(() => canonicalize(JSON.parse('"\\ud800"')), /Unicode/);

  const cases = new Map(evidence.cases.map((item) => [item.id, item]));
  const fresh = cases.get('fresh_qualified_evidence');
  assert.ok(fresh && fresh.records.length === 1, 'fresh content-bound control must be exactly one record');
  const record = fresh.records[0];
  assert.ok(record.artifact_payload_utf8, 'fresh content-bound payload missing');
  assert.equal(record.configuration_digest, evidence.target.configuration_digest);
  assert.equal(
    classifyPayload(record.artifact_payload_utf8, record.artifact_digest, record, byteLimit),
    'VERIFIED_AVAILABLE_CANDIDATE',
    'current content-bound claim failed independent Node.js validation',
  );

  const controls = binding.canonicalization_negative_controls;
  assert.equal(controls.length, Object.keys(CONTROL_SPEC).length, 'negative-control count changed');
  assert.deepEqual(controls.map((item) => item.id), Object.keys(CONTROL_SPEC), 'negative-control IDs/order changed');
  for (const control of controls) {
    const pinned = CONTROL_SPEC[control.id];
    assert.ok(pinned, control.id + ': unknown control');
    assert.equal(control.mutation, pinned[0], control.id + ': mutation changed');
    assert.equal(control.recompute_artifact_digest, pinned[1], control.id + ': digest policy changed');
    assert.equal(control.expected_status, pinned[2], control.id + ': verdict changed');
    const mutated = mutatePayload(control.mutation, record.artifact_payload_utf8, byteLimit);
    const digest = control.recompute_artifact_digest ? sha256(Buffer.from(mutated, 'utf8')) : record.artifact_digest;
    const observed = classifyPayload(mutated, digest, record, byteLimit);
    assert.equal(observed, control.expected_status, control.id + ': expected ' + control.expected_status + ', got ' + observed);
  }
  console.log('PASS: independent Node.js payload check; ' + jcsVectors.length + ' JCS known-answer vectors; ' + controls.length + ' hostile-input controls.');
  console.log('SCOPE: test tooling and synthetic fixture only; not full RFC 8785 certification or physical evidence authentication.');
}

try {
  run();
} catch (error) {
  console.error('FAIL: ' + (error instanceof Error ? error.message : String(error)));
  process.exitCode = 1;
}
