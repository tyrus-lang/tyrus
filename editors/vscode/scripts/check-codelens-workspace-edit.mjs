import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { transformSync } from "esbuild";

const here = dirname(fileURLToPath(import.meta.url));
const source = readFileSync(join(here, "../src/lsp-workspace-edit.ts"), "utf8");
const compiled = transformSync(source, {
  loader: "ts",
  format: "cjs",
  target: "node18",
});
const module = { exports: {} };
new Function("module", "exports", compiled.code)(module, module.exports);
const { lspTextEditsByUri } = module.exports;

const first = {
  range: {
    start: { line: 1, character: 2 },
    end: { line: 1, character: 5 },
  },
  newText: "const",
};
const second = {
  range: {
    start: { line: 3, character: 0 },
    end: { line: 3, character: 1 },
  },
  newText: "x",
};

assert.deepEqual(
  lspTextEditsByUri({ changes: { "file:///workspace/main.tyr": [first] } }),
  [{ uri: "file:///workspace/main.tyr", edits: [first] }],
);
assert.deepEqual(
  lspTextEditsByUri({
    documentChanges: [
      { textDocument: { uri: "file:///workspace/lib.tyr", version: 4 }, edits: [second] },
    ],
  }),
  [{ uri: "file:///workspace/lib.tyr", edits: [second] }],
);
assert.deepEqual(
  lspTextEditsByUri({ changes: { "file:///workspace/main.tyr": [{ ...first, range: {} }] } }),
  [{ uri: "file:///workspace/main.tyr", edits: [] }],
);
assert.equal(lspTextEditsByUri(undefined), undefined);

console.log("Quick Fix CodeLens WorkspaceEdit conversion handles changes and documentChanges");
