#!/usr/bin/env node
/**
 * scripts/validate_release_artifacts.js
 *
 * Stricter validator for the release artifact set declared in
 * data/release-manifest.json.  Runs as part of `npm run lint:db` and the
 * CI lint step (`make lint`).  Any drift between the manifest and the
 * on-disk artifact set fails the build before a release is cut.
 *
 * Checks performed:
 *   1. Manifest is valid JSON and the policy block is intact.
 *   2. Every declared artifact exists on disk.
 *   3. JSON artifacts parse cleanly.
 *   4. Schema files declare draft-07 ($schema field).
 *   5. data/vulnerability-db.json `version` and `last_updated` fields
 *      satisfy schemas/vulnerability-db.json.
 *   6. schemas/analysis-output.json describes a `schema_version` required
 *      field whose first example is a strict semver string.
 *   7. CHECKSUMS.txt (if present) covers every manifest artifact and no
 *      others — catches drift in either direction.
 *
 * Exit code 0 on success, 1 on validation failure, 2 on environment error.
 */

'use strict';

var fs = require('fs');
var path = require('path');

var REPO_ROOT = path.resolve(__dirname, '..');
var MANIFEST_PATH = path.join(REPO_ROOT, 'data', 'release-manifest.json');
var CHECKSUMS_PATH = path.join(REPO_ROOT, 'CHECKSUMS.txt');
var REQUIRED_SCHEMA_DRAFT = 'http://json-schema.org/draft-07/schema#';

var failed = false;
function fail(msg) {
  console.error('✗ ' + msg);
  failed = true;
}
function ok(msg) {
  console.log('✓ ' + msg);
}

function readJson(absPath, label) {
  try {
    var raw = fs.readFileSync(absPath, 'utf8');
    return JSON.parse(raw);
  } catch (err) {
    fail(label + ' (' + path.relative(REPO_ROOT, absPath) + '): ' + err.message);
    return null;
  }
}

function get(obj, prop) {
  return obj && Object.prototype.hasOwnProperty.call(obj, prop) ? obj[prop] : undefined;
}

if (!fs.existsSync(MANIFEST_PATH)) {
  console.error('Environment error: ' + MANIFEST_PATH + ' is missing.');
  process.exit(2);
}
var manifest = readJson(MANIFEST_PATH, 'release manifest');
if (!manifest) {
  process.exit(1);
}

if (!manifest.manifest_version || !manifest.policy || !manifest.artifacts) {
  fail('release-manifest.json must declare manifest_version, policy, and artifacts.');
}

var declaredDraft = get(manifest.policy, 'schemas_must_declare_draft');
if (declaredDraft !== REQUIRED_SCHEMA_DRAFT) {
  fail('policy.schemas_must_declare_draft must be "' + REQUIRED_SCHEMA_DRAFT + '".');
}

var dataArtifacts = get(manifest.artifacts, 'data') || [];
var schemaArtifacts = get(manifest.artifacts, 'schemas') || [];
var allArtifacts = dataArtifacts.concat(schemaArtifacts);

if (allArtifacts.length === 0) {
  fail('release manifest declares no artifacts.');
}

for (var i = 0; i < allArtifacts.length; i++) {
  var rel = allArtifacts[i];
  var abs = path.join(REPO_ROOT, rel);
  if (!fs.existsSync(abs)) {
    fail('manifest references missing file ' + rel + '.');
    continue;
  }
  if (rel.match(/\.json$/)) {
    readJson(abs, 'artifact JSON');
  }
}
ok(allArtifacts.length + ' artifacts present and parseable.');

for (var s = 0; s < schemaArtifacts.length; s++) {
  var srel = schemaArtifacts[s];
  var sabs = path.join(REPO_ROOT, srel);
  if (!fs.existsSync(sabs)) continue;
  var schemaObj = readJson(sabs, 'schema');
  if (!schemaObj) continue;
  if (schemaObj.$schema !== REQUIRED_SCHEMA_DRAFT) {
    fail(srel + ' must declare $schema = "' + REQUIRED_SCHEMA_DRAFT + '" (found ' + JSON.stringify(schemaObj.$schema) + ').');
  }
}
ok('schemas declare ' + REQUIRED_SCHEMA_DRAFT + '.');

var vulnDbPath = path.join(REPO_ROOT, 'data', 'vulnerability-db.json');
if (fs.existsSync(vulnDbPath)) {
  var db = readJson(vulnDbPath, 'vulnerability-db');
  if (db) {
    if (!/^[0-9]+\.[0-9]+\.[0-9]+$/.test(db.version || '')) {
      fail('data/vulnerability-db.json version "' + db.version + '" is not strict semver.');
    }
    if (!/^\d{4}-\d{2}-\d{2}$/.test(db.last_updated || '')) {
      fail('data/vulnerability-db.json last_updated "' + db.last_updated + '" is not YYYY-MM-DD.');
    }
  }
}

var analysisSchemaPath = path.join(REPO_ROOT, 'schemas', 'analysis-output.json');
if (fs.existsSync(analysisSchemaPath)) {
  var schema = readJson(analysisSchemaPath, 'analysis-output schema');
  if (schema) {
    var required = schema.required || [];
    if (required.indexOf('schema_version') < 0) {
      fail('schemas/analysis-output.json must require a `schema_version` field.');
    }
    var sv = get(schema.properties, 'schema_version');
    if (!sv || sv.type !== 'string') {
      fail('schemas/analysis-output.json `schema_version` must be a string property.');
    }
    var example = sv && sv.examples && sv.examples[0];
    if (!example || !/^[0-9]+\.[0-9]+\.[0-9]+$/.test(example)) {
      fail('schemas/analysis-output.json schema_version.examples[0] must be a strict semver.');
    }
  }
}

if (fs.existsSync(CHECKSUMS_PATH)) {
  var lines = fs.readFileSync(CHECKSUMS_PATH, 'utf8')
    .split('\n')
    .filter(function (l) { return l.trim() && l.charAt(0) !== '#'; });
  var covered = Object.create(null);
  var coveredCount = 0;
  for (var li = 0; li < lines.length; li++) {
    var m = lines[li].match(/^[0-9a-f]{64}\s+\*?(.+)$/i);
    if (m) {
      covered[m[1].trim()] = true;
      coveredCount += 1;
    }
  }
  var expected = Object.create(null);
  for (var ai = 0; ai < allArtifacts.length; ai++) {
    expected[allArtifacts[ai]] = true;
  }
  var coverageOk = true;
  for (var er in expected) {
    if (!covered[er]) {
      fail('CHECKSUMS.txt does not cover release artifact ' + er + '. Run scripts/generate-provenance.sh.');
      coverageOk = false;
    }
  }
  for (var cr in covered) {
    if (!expected[cr]) {
      fail('CHECKSUMS.txt covers ' + cr + ' but it is not in data/release-manifest.json.');
      coverageOk = false;
    }
  }
  if (coverageOk) {
    ok('CHECKSUMS.txt covers exactly the ' + allArtifacts.length + ' manifest artifacts.');
  }
} else {
  console.log('• CHECKSUMS.txt not present — skipping coverage check (release pipeline will generate it).');
}

if (failed) {
  console.error('\nRelease artifact validation FAILED.');
  process.exit(1);
}
console.log('\nRelease artifact validation passed.');
