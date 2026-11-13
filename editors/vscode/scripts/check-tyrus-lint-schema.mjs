import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const projectDir = resolve(dirname(fileURLToPath(import.meta.url)), "../../..");

const config = readFileSync(resolve(projectDir, "crates/tyrus-lint/src/config/config.rs"), "utf8");
const rules = parseRules(config);

const schema = JSON.parse(readFileSync(resolve(projectDir, "editors/vscode/schemas/tyrus-lint.schema.json"), "utf8"));
const properties = schema.properties ?? {};
const ruleKeys = rules.map((rule) => rule.name);
const actualKeys = Object.keys(properties);

assertEqual(schema.type, "object", "schema type");
assertEqual(schema.additionalProperties, false, "additionalProperties");
assertArrayEqual(actualKeys, ruleKeys, "rule order");

for (const key of ruleKeys) {
  const actual = properties[key];
  const rule = rules.find((candidate) => candidate.name === key);

  assertEqual(actual["$ref"], "#/$defs/severity", `${key}.$ref`);
  assertEqual(actual.default, rule.default, `${key}.default`);
  assertEqual(typeof actual.description, "string", `${key}.description`);
  assertEqual(actual.description.length > 0, true, `${key}.description not empty`);
}

const docs = readFileSync(resolve(projectDir, "TYRUS-LINT.md"), "utf8");
const documentedKeys = [...docs.matchAll(/^## `([^`]+)`$/gm)].map((match) => match[1]);
assertArrayEqual(documentedKeys, ruleKeys, "TYRUS-LINT.md section order");

console.log("tyrus-lint schema is in sync");

function parseRules(source) {
  const rules = [];
  const blockPattern = /RuleMeta\s*\{([\s\S]*?)\n\s*\},/g;
  for (const match of source.matchAll(blockPattern)) {
    const block = match[1];
    const name = block.match(/name: "([^"]+)"/)?.[1];
    const defaultSeverity = block.match(/default: Severity::(Off|Info|Warn|Error)/)?.[1];
    if (!name || !defaultSeverity) {
      continue;
    }
    rules.push({ name, default: severityLabel(defaultSeverity) });
  }
  return rules;
}

function severityLabel(name) {
  switch (name) {
    case "Off":
      return "off";
    case "Info":
      return "info";
    case "Warn":
      return "warn";
    case "Error":
      return "error";
    default:
      throw new Error(`unknown severity ${name}`);
  }
}

function assertEqual(actual, expectedValue, label) {
  if (actual !== expectedValue) {
    throw new Error(`${label}: expected ${JSON.stringify(expectedValue)}, got ${JSON.stringify(actual)}`);
  }
}

function assertArrayEqual(actual, expectedValue, label) {
  if (!Array.isArray(actual)) {
    throw new Error(`${label}: expected array, got ${JSON.stringify(actual)}`);
  }
  assertEqual(JSON.stringify(actual), JSON.stringify(expectedValue), label);
}
