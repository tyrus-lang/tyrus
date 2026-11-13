# Tyrus Linter Configuration

The linter reads `tyrus-lint.json` from the project root. The file is a flat JSON object mapping rule names to severities. There is no `extends`, no `presets`, and no `rules` key. Rules omitted from the file use their built-in default severity.

## Commands

Run the linter for the current project:

```sh
tyrus lint
```

By default, the command lints every `.tyr` file under `src`. It reads `tyrus-lint.json` from the project root when the file exists; otherwise it uses the built-in default severities.

Create or update `tyrus-lint.json` without changing existing rule severities:

```sh
tyrus lint --init
```

If the file does not exist, this writes the default lint configuration. If the file already exists, it adds any missing implemented rules and keeps the severities already configured by the project.

Create or reset `tyrus-lint.json` to the default severities:

```sh
tyrus lint --default
```

This overwrites `tyrus-lint.json` with the built-in defaults for implemented rules. Use it when the project should discard its current lint severity overrides.

Severities:

| Value   | Meaning                                      |
|---------|----------------------------------------------|
| `off`   | Rule disabled                                |
| `info`  | Informative diagnostic, non-blocking         |
| `warn`  | Warning, does not break CI by default        |
| `error` | Error, causes the command to exit with code 1 |

Example:

```json
{
  "no-unused-vars": "warn",
  "no-unused-imports": "warn",
  "prefer-const": "warn",
  "no-unreachable-code": "error",
  "no-empty-block": "warn",
  "no-force-unwrap": "error",
  "no-null-literal": "error",
  "exhaustive-match": "error",
  "no-duplicate-case": "error",
  "no-shadow": "warn"
}
```

---

- [`api-no-verb-in-path`](#api-no-verb-in-path)
- [`api-prefer-resource-nouns`](#api-prefer-resource-nouns)
- [`controller-must-send-response`](#controller-must-send-response)
- [`controller-no-db-access`](#controller-no-db-access)
- [`controller-no-try-catch`](#controller-no-try-catch)
- [`db-explicit-select-columns`](#db-explicit-select-columns)
- [`db-no-extra-select-columns`](#db-no-extra-select-columns)
- [`exhaustive-match`](#exhaustive-match)
- [`export-protect-no-effect`](#export-protect-no-effect)
- [`http-no-raw-status-code`](#http-no-raw-status-code)
- [`no-duplicate-case`](#no-duplicate-case)
- [`no-duplicate-imports`](#no-duplicate-imports)
- [`no-empty-block`](#no-empty-block)
- [`no-empty-catch`](#no-empty-catch)
- [`no-floating-enum`](#no-floating-enum)
- [`no-force-unwrap`](#no-force-unwrap)
- [`no-heterogeneous-enum`](#no-heterogeneous-enum)
- [`no-implicit-enum-primitive-cast`](#no-implicit-enum-primitive-cast)
- [`no-implicit-heavy-move`](#no-implicit-heavy-move)
- [`no-layer-skip`](#no-layer-skip)
- [`no-null-literal`](#no-null-literal)
- [`no-shadow`](#no-shadow)
- [`no-string-concat-in-loop`](#no-string-concat-in-loop)
- [`no-uninitialized-variable`](#no-uninitialized-variable)
- [`no-unnecessary-await`](#no-unnecessary-await)
- [`no-unnecessary-allocation`](#no-unnecessary-allocation)
- [`no-unnecessary-clone`](#no-unnecessary-clone)
- [`no-upward-imports`](#no-upward-imports)
- [`no-unreachable-code`](#no-unreachable-code)
- [`no-unsafe-finally`](#no-unsafe-finally)
- [`no-useless-null-check`](#no-useless-null-check)
- [`no-unused-functions`](#no-unused-functions)
- [`no-unused-imports`](#no-unused-imports)
- [`no-unused-vars`](#no-unused-vars)
- [`prefer-borrow`](#prefer-borrow)
- [`prefer-const`](#prefer-const)
- [`prefer-nullish-coalescing`](#prefer-nullish-coalescing)
- [`repository-no-http-error`](#repository-no-http-error)
- [`repository-only-db-access`](#repository-only-db-access)
- [`route-no-business-logic`](#route-no-business-logic)
- [`service-no-reply`](#service-no-reply)
- [`use-caught-error`](#use-caught-error)

---

## `api-no-verb-in-path`

Warns when an HTTP route path uses action verbs instead of resource-oriented paths.

| Default | Category      | Autofix |
|---------|---------------|---------|
| `warn`  | style         | no      |

---

## `api-prefer-resource-nouns`

Warns when HTTP route paths do not look like resource nouns.

| Default | Category      | Autofix |
|---------|---------------|---------|
| `warn`  | style         | no      |

---

## `controller-must-send-response`

Requires HTTP controllers to complete every success path with `reply.status(...).send(...)`.

| Default | Category      | Autofix |
|---------|---------------|---------|
| `error` | correctness   | no      |

---

## `controller-no-db-access`

Forbids direct database access from HTTP controllers.

| Default | Category      | Autofix |
|---------|---------------|---------|
| `error` | safety        | no      |

---

## `controller-no-try-catch`

Warns when HTTP controllers catch expected errors instead of letting services map them.

| Default | Category      | Autofix |
|---------|---------------|---------|
| `warn`  | safety        | no      |

---

## `db-explicit-select-columns`

Encourages explicit `SELECT` projections and can expand them from the local result interface.

| Default | Category      | Autofix |
|---------|---------------|---------|
| `info`  | style         | yes     |

---

## `db-no-extra-select-columns`

Warns when a SQL `SELECT` projects columns that the result interface does not use.

| Default | Category      | Autofix |
|---------|---------------|---------|
| `info`  | performance   | no      |

---

## `exhaustive-match`

Requires `match` over a closed type (enum, union, literal union, boolean) to cover every case. Enforced by the compiler (E0004).

| Default | Category      | Autofix |
|---------|---------------|---------|
| `error` | correctness   | no      |

**Bad** (missing `Error` case):

```tyrus
enum Status { Success, Error }

match (status) {
    (Status.Success) => {}
}
```

**Good**:

```tyrus
match (status) {
    (Status.Success) => {}
    (Status.Error) => {}
}
```

---

## `export-protect-no-effect`

Warns when `export protect` sits in the source root, where its folder scope already covers the whole project and narrows nothing.

| Default | Category      | Autofix |
|---------|---------------|---------|
| `warn`  | style         | no      |

**Bad** (`src/main.tyr`, whose folder is the whole project):

```tyrus
export protect const helper = (): i32 => 1;
```

**Good** (`src/auth/token.tyr`, whose folder is a real boundary):

```tyrus
export protect const helper = (): i32 => 1;
```

---

## `http-no-raw-status-code`

Warns when HTTP responses use raw numeric status codes instead of `HttpStatus` members.

| Default | Category      | Autofix |
|---------|---------------|---------|
| `warn`  | safety        | no      |

---

## `no-duplicate-case`

Detects duplicate cases in `switch` or `match`.

| Default | Category      | Autofix |
|---------|---------------|---------|
| `error` | correctness   | no      |

**Bad**:

```tyrus
match (status) {
    (Status.Success) => {}
    (Status.Success) => {}
}
```

---

## `no-duplicate-imports`

Detects multiple import statements from the same module.

| Default | Category      | Autofix |
|---------|---------------|---------|
| `error` | correctness   | no      |

**Bad**:

```tyrus
import { User } from "./user";
import { Role } from "./user";
```

**Good**:

```tyrus
import { Role, User } from "./user";
```

---

## `no-empty-block`

Detects empty blocks without an explanatory comment.

| Default | Category      | Autofix |
|---------|---------------|---------|
| `warn`  | style         | no      |

**Bad**:

```tyrus
if (ok) {
}
```

**Allowed**:

```tyrus
if (ok) {
    // intentionally empty
}
```

---

## `no-empty-catch`

Detects an empty `catch` block without an explanatory comment.

| Default | Category      | Autofix |
|---------|---------------|---------|
| `warn`  | safety        | no      |

**Bad**:

```tyrus
try {
    readFileSync("config.json");
} catch (error) {
}
```

**Allowed**:

```tyrus
try {
    readFileSync("config.json");
} catch (error) {
    // intentionally ignored
}
```

---

## `no-floating-enum`

Forbids `f64`/`f128` values in an enum. Enforced by the compiler (E0701).

| Default | Category      | Autofix |
|---------|---------------|---------|
| `error` | correctness   | no      |

**Bad**:

```tyrus
enum Ratio {
    Half = 0.5,
}
```

---

## `no-force-unwrap`

Forbids forced unwrap (`expr!`) on optional values. Enforced by the compiler (`E0716`).

| Default | Category      | Autofix |
|---------|---------------|---------|
| `error` | safety        | no      |

**Bad**:

```tyrus
const name: string? = getName();
console.log(name!);
```

**Better**:

```tyrus
const name: string? = getName();
if (name) {
    console.log(name);
}
```

---

## `no-heterogeneous-enum`

Forbids mixing value types within a primitive enum. Enforced by the compiler (E0702).

| Default | Category      | Autofix |
|---------|---------------|---------|
| `error` | correctness   | no      |

**Bad**:

```tyrus
enum Mixed {
    Ok = 200,
    Error = "error",
}
```

---

## `no-implicit-enum-primitive-cast`

Forbids implicit conversion of an enum to its underlying primitive. Enforced by the compiler (E0308).

| Default | Category      | Autofix |
|---------|---------------|---------|
| `error` | safety        | no      |

**Bad**:

```tyrus
const code: i32 = HttpStatus.Ok;
```

**Good**:

```tyrus
const code: i32 = HttpStatus.Ok as i32;
```

---

## `no-implicit-heavy-move`

Warns when a large or non-Copy value is moved in a potentially surprising way.

| Default | Category      | Autofix |
|---------|---------------|---------|
| `warn`  | performance   | no      |

---

## `no-layer-skip`

Warns when API code skips the route-controller-service-repository layering.

| Default | Category      | Autofix |
|---------|---------------|---------|
| `warn`  | safety        | no      |

---

## `no-null-literal`

Forbids the `null` keyword. Nullable values use `T?`; absence is produced by optional fields, optional parameters, and external boundaries.

| Default | Category      | Autofix |
|---------|---------------|---------|
| `error` | correctness   | no      |

**Bad**:

```tyrus
const message = null;
```

**Good**:

```tyrus
interface Message {
    text?: string;
}

const message: Message = {};
const text: string = message.text ?? "default";
```

---

## `no-shadow`

Detects a binding that shadows another binding from an enclosing scope.

| Default | Category      | Autofix |
|---------|---------------|---------|
| `warn`  | style         | no      |

**Bad**:

```tyrus
const name: string = "Ada";

if (ok) {
    const name: string = "Grace";
}
```

**Good**:

```tyrus
const name: string = "Ada";

if (ok) {
    const inner: string = "Grace";
}
```

---

## `no-string-concat-in-loop`

Detects repeated string concatenation inside a loop.

| Default | Category      | Autofix |
|---------|---------------|---------|
| `warn`  | performance   | no      |

**Bad**:

```tyrus
let output: string = "";
for (let i: i32 = 0; i < items.length; i++) {
    output += items[i];
}
```

**Better**: use an array buffer and join when available.

---

## `no-uninitialized-variable`

Surfaces the compiler's definite-assignment diagnostics for deferred `let x: T;` declarations: a variable used before it is assigned (`E0381`) or declared and never assigned (`E0382`). The same diagnostics appear in the editor and, for `E0381`, fail `tyrus build`.

| Default | Category    | Autofix |
|---------|-------------|---------|
| `warn`  | correctness | no      |

**Bad**:

```tyrus
let message: string;
console.log(message);   // used before being assigned

let other: string;      // declared but never assigned
```

**Good**:

```tyrus
let message: string;
message = loadMessage();
console.log(message);
```

---

## `no-unnecessary-await`

Reports `return await promise` inside async functions. Tyrus adopts a directly returned native Promise, preserving fulfillment, rejection, cancellation, payload typing, and generated cleanup without an additional suspension in the current async frame.

| Default | Category | Autofix |
|---------|----------|---------|
| `warn` | style | yes |

**Bad**:

```tyrus
const load = async (): Promise<i32> => {
    return await fetchValue();
};
```

**Good**:

```tyrus
const load = async (): Promise<i32> => {
    return fetchValue();
};
```

---

## `no-unnecessary-allocation`

Detects clearly unnecessary allocations.

| Default | Category      | Autofix |
|---------|---------------|---------|
| `info`  | performance   | no      |

**Bad**:

```tyrus
const copy: string = name.slice(0);
```

---

## `no-unnecessary-clone`

Detects a clone where the value could be borrowed instead.

| Default | Category      | Autofix |
|---------|---------------|---------|
| `warn`  | performance   | no      |

**Bad**:

```tyrus
print(user.clone());
```

**Better**: pass `&user` if `print` only needs to read it.

---

## `no-upward-imports`

Forbids imports from lower API layers back into higher layers.

| Default | Category      | Autofix |
|---------|---------------|---------|
| `error` | safety        | no      |

---

## `no-unreachable-code`

Detects code that can never execute because it follows `return`, `break`, `continue`, or `throw`.

| Default | Category      | Autofix |
|---------|---------------|---------|
| `error` | correctness   | no      |

**Bad**:

```tyrus
return;
console.log("never");
```

---

## `no-unsafe-finally`

Detects `return`, `throw`, `break`, or `continue` inside a `finally` block, which can override the try/catch result.

| Default | Category      | Autofix |
|---------|---------------|---------|
| `error` | safety        | no      |

**Bad**:

```tyrus
try {
    return readConfig();
} finally {
    return defaultConfig();
}
```

---

## `no-useless-null-check`

Detects a null check on a value that can never be null. This rule is kept for legacy configurations; new code is blocked earlier by `no-null-literal`.

| Default | Category      | Autofix |
|---------|---------------|---------|
| `warn`  | correctness   | no      |

---

## `no-unused-functions`

Detects non-exported functions that are never called. Exported functions are treated as public API.

| Default | Category      | Autofix |
|---------|---------------|---------|
| `warn`  | correctness   | no      |

---

## `no-unused-imports`

Detects imported names that are never used.

| Default | Category      | Autofix |
|---------|---------------|---------|
| `warn`  | correctness   | yes     |

---

## `no-unused-vars`

Detects declared variables that are never used. Prefix a name with `_` to mark it intentionally unused.

| Default | Category      | Autofix |
|---------|---------------|---------|
| `warn`  | correctness   | no      |

**Bad**:

```tyrus
const name: string = "Ada";
```

**Help**: remove the declaration or prefix it with `_name`.

---

## `prefer-borrow`

Suggests passing a reference when a function does not need ownership.

| Default | Category      | Autofix |
|---------|---------------|---------|
| `info`  | performance   | no      |

**Bad**:

```tyrus
process(user);
```

**Better**: pass `&user` if `process` only needs to read the value.

---

## `prefer-const`

Suggests `const` when a `let` binding is never reassigned.

| Default | Category      | Autofix |
|---------|---------------|---------|
| `warn`  | style         | yes     |

**Bad**:

```tyrus
let name: string = "Ada";
console.log(name);
```

**Good**:

```tyrus
const name: string = "Ada";
console.log(name);
```

---

## `prefer-nullish-coalescing`

Suggests `??` instead of a ternary that falls back on null. This rule is kept for legacy configurations; new code is blocked earlier by `no-null-literal`.

| Default | Category      | Autofix |
|---------|---------------|---------|
| `info`  | style         | no      |

**Good**:

```tyrus
const value: string = name ?? "default";
```

---

## `repository-no-http-error`

Warns when repositories throw HTTP-layer errors instead of database/domain errors.

| Default | Category      | Autofix |
|---------|---------------|---------|
| `warn`  | safety        | no      |

---

## `repository-only-db-access`

Restricts direct database access to repository modules.

| Default | Category      | Autofix |
|---------|---------------|---------|
| `error` | safety        | no      |

---

## `route-no-business-logic`

Warns when route modules contain business logic instead of declarations.

| Default | Category      | Autofix |
|---------|---------------|---------|
| `warn`  | complexity    | no      |

---

## `service-no-reply`

Forbids services from depending on `HttpReply` or sending HTTP responses directly.

| Default | Category      | Autofix |
|---------|---------------|---------|
| `error` | safety        | no      |

---

## `use-caught-error`

Detects a caught error that is never used. Rename it to `_error` to mark it intentionally ignored.

| Default | Category      | Autofix |
|---------|---------------|---------|
| `warn`  | safety        | no      |

**Bad**:

```tyrus
try {
    readFileSync("config.json");
} catch (error) {
    console.error("failed");
}
```

**Good**:

```tyrus
try {
    readFileSync("config.json");
} catch (_error) {
    console.error("failed");
}
```
