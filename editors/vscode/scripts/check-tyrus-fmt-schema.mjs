import { readFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const projectDir = resolve(dirname(fileURLToPath(import.meta.url)), "../../..");

const expected = {
  array_expand: {
    type: "string",
    default: "simple",
    enum: ["simple", "expand-all", "off"],
  },
  blank_lines: {
    type: "string",
    default: "collapse",
    enum: ["collapse", "off"],
  },
  blank_lines_around_blocks: {
    type: "boolean",
    default: true,
  },
  blank_lines_around_decorations: {
    type: "boolean",
    default: true,
  },
  brace_spacing: {
    type: "boolean",
    default: true,
  },
  brace_style: {
    type: "string",
    default: "same-line",
    enum: ["same-line", "next-line", "off"],
  },
  final_newline: {
    type: "boolean",
    default: true,
  },
  indent_style: {
    type: "string",
    default: "spaces",
    enum: ["spaces", "tab"],
  },
  indent_width: {
    type: "integer",
    default: 4,
    minimum: 0,
  },
  imports_sort: {
    type: "boolean",
    default: true,
  },
  line_width: {
    type: "integer",
    default: 80,
    minimum: 0,
  },
  object_expand: {
    type: "string",
    default: "simple",
    enum: ["simple", "expand-all", "off"],
  },
  pipeline_chain_layout: {
    type: "string",
    default: "line-by-line",
    enum: ["line-by-line", "inline", "off"],
  },
  prefer_arrow_function: {
    type: "boolean",
    default: false,
  },
  quote_style: {
    type: "string",
    default: "double",
    enum: ["double", "single", "off"],
  },
  remove_unused_imports: {
    type: "boolean",
    default: true,
  },
  trailing_comma: {
    type: "boolean",
    default: true,
  },
};

const schema = JSON.parse(readFileSync(resolve(projectDir, "editors/vscode/schemas/tyrus-fmt.schema.json"), "utf8"));
const properties = schema.properties ?? {};
const expectedKeys = Object.keys(expected);
const actualKeys = Object.keys(properties);

assertEqual(schema.type, "object", "schema type");
assertEqual(schema.additionalProperties, false, "additionalProperties");
assertArrayEqual(actualKeys, expectedKeys, "property order");

for (const key of expectedKeys) {
  const actual = properties[key];
  const config = expected[key];

  assertEqual(actual.type, config.type, `${key}.type`);
  assertEqual(actual.default, config.default, `${key}.default`);

  if ("enum" in config) {
    assertArrayEqual(actual.enum, config.enum, `${key}.enum`);
  } else if ("enum" in actual) {
    throw new Error(`${key}.enum must be omitted`);
  }

  if ("minimum" in config) {
    assertEqual(actual.minimum, config.minimum, `${key}.minimum`);
  } else if ("minimum" in actual) {
    throw new Error(`${key}.minimum must be omitted`);
  }
}

const docs = readFileSync(resolve(projectDir, "TYRUS-FMT.md"), "utf8");
const documentedKeys = [...docs.matchAll(/^## `([^`]+)`$/gm)].map((match) => match[1]);
assertArrayEqual(documentedKeys, expectedKeys, "TYRUS-FMT.md section order");

const config = readFileSync(resolve(projectDir, "crates/tyrus-fmt/src/config/config.rs"), "utf8");
const configFields = extractConfigFields(config);
assertArrayEqual(configFields, expectedKeys, "Config field order");

console.log("tyrus-fmt schema is in sync");

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

function extractConfigFields(source) {
  const match = source.match(/pub struct Config \{([\s\S]*?)\n\}/);
  if (!match) {
    throw new Error("Could not find Config struct in crates/tyrus-fmt/src/config/config.rs");
  }
  return [...match[1].matchAll(/pub ([a-z_]+):/g)].map((field) => field[1]);
}
