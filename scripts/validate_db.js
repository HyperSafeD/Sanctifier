const fs = require('fs');
const path = require('path');
const Ajv = require('ajv');
const yaml = require('yaml');
const addFormats = require('ajv-formats');

const ajv = new Ajv({ strict: false });
addFormats(ajv);

const MAPPINGS = [
  { schema: 'schemas/vulnerability-db.json', file: 'data/vulnerability-db.json' },
  { schema: 'schemas/sarif-rule-metadata.schema.json', file: 'data/sarif/rule-metadata.yaml' },
  { schema: 'schemas/severity-taxonomy.schema.json', file: 'data/sarif/severity-map.yaml' },
  { schema: 'schemas/security-review.schema.json', file: 'data/security-review/defaults.yaml' },
];

let hasErrors = false;

for (const { schema, file } of MAPPINGS) {
  if (!fs.existsSync(schema)) {
    console.error(`Missing schema: ${schema}`);
    hasErrors = true;
    continue;
  }
  if (!fs.existsSync(file)) {
    console.error(`Missing file: ${file}`);
    hasErrors = true;
    continue;
  }

  const schemaObj = JSON.parse(fs.readFileSync(schema, 'utf8'));
  const validate = ajv.compile(schemaObj);
  
  const content = fs.readFileSync(file, 'utf8');
  const dataObj = file.endsWith('.yaml') || file.endsWith('.yml') ? yaml.parse(content) : JSON.parse(content);
  
  const valid = validate(dataObj);
  if (!valid) {
    console.error(`Validation failed for ${file} against ${schema}:`);
    console.error(validate.errors);
    hasErrors = true;
  } else {
    console.log(`Validated ${file} successfully.`);
  }
}

if (hasErrors) {
  process.exit(1);
} else {
  console.log("All DB files validated successfully.");
}
