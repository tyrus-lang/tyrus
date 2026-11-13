// Loads the TextMate grammar through the same Oniguruma engine VS Code uses and
// tokenizes a handful of lines, asserting the scopes that matter. This exists
// because an invalid regex (e.g. a variable-length lookbehind, which Oniguruma
// rejects but many other engines accept) makes VS Code silently drop the whole
// grammar (every token renders unstyled) with no error anywhere. A JSON-valid
// grammar is not enough; the regexes must actually compile in Oniguruma.

import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { createRequire } from "node:module";

const require = createRequire(import.meta.url);
// Loaded through CommonJS `require`, not `import`: both packages ship a CJS
// entry whose named exports (loadWASM, Registry, ...) sit on the module object
// itself, not under an ESM `default`.
const oniguruma = require("vscode-oniguruma");
const vsctm = require("vscode-textmate");
const here = dirname(fileURLToPath(import.meta.url));
const GRAMMAR = join(here, "../syntaxes/tyrus.tmLanguage.json");

const onigWasm = readFileSync(
  join(dirname(require.resolve("vscode-oniguruma")), "onig.wasm")
).buffer;

const onigLib = oniguruma.loadWASM(onigWasm).then(() => ({
  createOnigScanner: (patterns) => new oniguruma.OnigScanner(patterns),
  createOnigString: (s) => new oniguruma.OnigString(s),
}));

const registry = new vsctm.Registry({
  onigLib,
  loadGrammar: () =>
    Promise.resolve(
      vsctm.parseRawGrammar(readFileSync(GRAMMAR, "utf8"), GRAMMAR)
    ),
});

// Each case: a line, then substrings that MUST carry a scope containing the
// given fragment (`want`), and substrings that MUST NOT be scoped as that
// fragment (`reject`). Kept small and behavioral, not a full snapshot.
const cases = [
  {
    line: 'const charac: char = "A";',
    want: [["char", "storage.type"]],
  },
  {
    line: "const nu8: u8 = 2;",
    want: [["u8", "storage.type"]],
  },
  {
    line: "const ni16: i16 = 2;",
    want: [["i16", "storage.type"]],
  },
  {
    line: "console.log(charac, nu8);",
    // A call argument must never be colored as a function parameter.
    reject: [["charac", "variable.parameter"]],
  },
  {
    line: "const add = (x: i32): i32 => x;",
    want: [["x", "variable.parameter"]],
  },
  {
    line: 'return db.query<UserResponse>("SELECT id, name FROM users ORDER BY id", []);',
    want: [["UserResponse", "entity.name.type.generic.tyr"]],
    all: [
      ["<", "entity.name.type.generic.tyr"],
      [">", "entity.name.type.generic.tyr"],
    ],
  },
  {
    line: "const findUsers = (): Promise<Result<UserResponse[], DbError>> => loadUsers();",
    want: [
      ["Promise", "entity.name.type.tyr"],
      ["UserResponse", "entity.name.type.tyr"],
    ],
    all: [
      ["<", "entity.name.type.generic.tyr"],
      [">", "entity.name.type.generic.tyr"],
    ],
  },
  {
    line: "type TextFlag = IsText<string>; // comment after a generic type",
    want: [
      ["string", "storage.type"],
      ["// comment after a generic type", "comment.line.double-slash"],
    ],
    all: [[">", "entity.name.type.generic.tyr"]],
  },
  {
    line: "const result = parse<Result<string, DbError>>();",
    want: [["parse", "entity.name.function.tyr"]],
    all: [[">", "entity.name.type.generic.tyr"]],
  },
  {
    line: 'return db.query<UserResponse>("SELECT id, name FROM users WHERE name LIKE ? ORDER BY id", []);',
    want: [
      ["query", "entity.name.function.tyr"],
      ["UserResponse", "entity.name.type.generic.tyr"],
      ["SELECT", "keyword.control.import.tyr"],
      ["FROM", "keyword.control.import.tyr"],
      ["LIKE", "keyword.control.import.tyr"],
      ["ORDER", "keyword.control.import.tyr"],
    ],
    all: [
      ["<", "entity.name.type.generic.tyr"],
      [">", "entity.name.type.generic.tyr"],
    ],
  },
  {
    line: 'return db.query<UserResponse>(\n    "SELECT id, name FROM users WHERE name LIKE ? ORDER BY id",\n    []\n);',
    want: [
      ["SELECT", "keyword.control.import.tyr"],
      ["FROM", "keyword.control.import.tyr"],
      ["LIKE", "keyword.control.import.tyr"],
      ["ORDER", "keyword.control.import.tyr"],
    ],
  },
  {
    line: "const byte = 10u8; const count = 1000u64; const ratio = 3.14f32; const money = 5.0d;",
    want: [
      ["10", "constant.numeric.integer"],
      ["u8", "storage.type"],
      ["1000", "constant.numeric.integer"],
      ["u64", "storage.type"],
      ["3.14", "constant.numeric.float"],
      ["f32", "storage.type"],
      ["5.0", "constant.numeric.float"],
      ["d", "storage.type"],
    ],
  },
];

registry.loadGrammar("source.tyr").then((grammar) => {
  if (!grammar) {
    console.error(
      "tyrus grammar failed to load, an invalid regex likely broke it in Oniguruma"
    );
    process.exit(1);
  }

  const tokensFor = (source) => {
    let state = vsctm.INITIAL;
    return source.split(/\r?\n/).flatMap((line) => {
      const { tokens, ruleStack } = grammar.tokenizeLine(line, state);
      state = ruleStack;
      return tokens.map((t) => ({
        text: line.substring(t.startIndex, t.endIndex),
        scopes: t.scopes,
      }));
    });
  };

  const scopeOf = (tokens, fragment) =>
    tokens.filter((t) => t.text.includes(fragment));

  const failures = [];
  for (const { line, want = [], reject = [], all = [] } of cases) {
    const tokens = tokensFor(line);
    for (const [fragment, scope] of want) {
      const matched = scopeOf(tokens, fragment).some((t) =>
        t.scopes.some((s) => s.includes(scope))
      );
      if (!matched) {
        failures.push(`"${line}": expected "${fragment}" to have scope *${scope}*`);
      }
    }
    for (const [fragment, scope] of reject) {
      const matched = scopeOf(tokens, fragment).some((t) =>
        t.scopes.some((s) => s.includes(scope))
      );
      if (matched) {
        failures.push(`"${line}": "${fragment}" must NOT have scope *${scope}*`);
      }
    }
    for (const [fragment, scope] of all) {
      const matches = tokens.filter((token) => token.text === fragment);
      if (matches.length === 0 || matches.some((token) =>
        !token.scopes.some((candidate) => candidate.includes(scope))
      )) {
        failures.push(`"${line}": every "${fragment}" must have scope *${scope}*`);
      }
    }
  }

  if (failures.length > 0) {
    console.error("tyrus grammar tokenization check failed:");
    for (const f of failures) console.error(`  - ${f}`);
    process.exit(1);
  }

  console.log("tyrus grammar loads in Oniguruma and tokenizes as expected");
}).catch((error) => {
  console.error("tyrus grammar check errored:", error.message);
  process.exit(1);
});
