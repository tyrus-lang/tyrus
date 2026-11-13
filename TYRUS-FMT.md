# Tyrus Formatter Configuration

The formatter reads `tyrus-fmt.json` from the project root. All keys are optional; defaults are used when a key is absent.

- [`array_expand`](#array_expand)
- [`blank_lines`](#blank_lines)
- [`blank_lines_around_blocks`](#blank_lines_around_blocks)
- [`blank_lines_around_decorations`](#blank_lines_around_decorations)
- [`brace_spacing`](#brace_spacing)
- [`brace_style`](#brace_style)
- [`final_newline`](#final_newline)
- [`indent_style`](#indent_style)
- [`indent_width`](#indent_width)
- [`imports_sort`](#imports_sort)
- [`line_width`](#line_width)
- [`object_expand`](#object_expand)
- [`pipeline_chain_layout`](#pipeline_chain_layout)
- [`prefer_arrow_function`](#prefer_arrow_function)
- [`quote_style`](#quote_style)
- [`remove_unused_imports`](#remove_unused_imports)
- [`trailing_comma`](#trailing_comma)

---

## `array_expand`

Controls when an array literal is expanded to multi-line.

| Default    | Values                              |
|------------|-------------------------------------|
| `"simple"` | `"simple"`, `"expand-all"`, `"off"` |

- `"simple"`, expand only when at least one element is a complex expression (function call, method call, nested literal, operator, etc.) Example: `["a", "b", "c"]` stays inline; `[foo(1), bar(2)]` expands.
- `"expand-all"`, always expand if there are multiple elements.
- `"off"`: never expand an array, even when it exceeds `line_width`.

---

## `blank_lines`

Controls blank lines between top-level items (imports, declarations, statements).

| Default      | Values                   |
|--------------|--------------------------|
| `"collapse"` | `"collapse"`, `"off"`    |

- `"collapse"`, add blank lines between imports and items sections and around control flow blocks (if, while, for) inside function bodies. Original blank lines between other statements are preserved.
- `"off"`, the formatter does not add blank lines. Existing single blank lines are preserved, while repeated blank lines are collapsed.

---

## `blank_lines_around_blocks`

When enabled, adds a blank line before and after block statements (if, for, for...of, while, do...while, match, switch) and function declarations.

| Default | Values          |
|---------|-----------------|
| `true`  | `true`, `false` |

- `true`, insert blank lines around block statements and functions.
- `false`, no extra blank lines; blocks are adjacent to surrounding statements.

---

## `blank_lines_around_decorations`

When enabled, adds a blank line around `interface`/`type` fields that carry validator decorations, so each decorated field stands on its own. Decorators are always printed on their own line above the field regardless of this setting.

| Default | Values          |
|---------|-----------------|
| `true`  | `true`, `false` |

- `true`:

  ```typescript
  interface ProductFilter {
      @notEmpty()
      query: string;

      @min(5)
      pageSize: i32;
  }
  ```

- `false`:

  ```typescript
  interface ProductFilter {
      @notEmpty()
      query: string;
      @min(5)
      pageSize: i32;
  }
  ```

---

## `brace_spacing`

Whether to print a space between curly braces and their content.

| Default | Values          |
|---------|-----------------|
| `true`  | `true`, `false` |

- `true`, `{ x: 1 }`
- `false`, `{x: 1}`

---

## `brace_style`

Controls placement of the opening brace of code blocks: function bodies and control-flow statements (`if`/`else`, `while`, `do`/`while`, `for`, `for...of`, `for...in`, `switch`, `match`, `try`/`catch`/`finally`). It does not affect object literals or `interface`/`type`/`enum` declarations.

| Default        | Values                                 |
|----------------|----------------------------------------|
| `"same-line"`  | `"same-line"`, `"next-line"`, `"off"` |

- `"same-line"`, opening brace stays on the header line:

  ```typescript
  if (cond) {
      foo();
  }
  ```

- `"next-line"`, opening brace drops to its own line, and continuation keywords (`else`, `catch`, `finally`, `while`) start a new line too:

  ```typescript
  if (cond)
  {
      foo();
  }
  else
  {
      bar();
  }
  ```

- `"off"`: preserve the source layout of every block and continuation keyword.

---

## `final_newline`

Controls whether a trailing newline is added at the end of the file.

| Default | Values          |
|---------|-----------------|
| `true`  | `true`, `false` |

- `true`, ensure the file ends with a newline character.
- `false`, omit the trailing newline.

---

## `indent_style`

| Default    | Values                |
|------------|-----------------------|
| `"spaces"` | `"tab"`, `"spaces"`   |

---

## `indent_width`

Number of spaces per indentation level (used when `indent_style` is `"spaces"`).

| Default | Values |
|---------|--------|
| `4`     | integer ≥ 0 |

---

## `imports_sort`

Controls whether import declarations are sorted alphabetically and grouped by source type.

| Default | Values          |
|---------|-----------------|
| `true`  | `true`, `false` |

- `true`, sort imports alphabetically by module path, ignoring `./`, `../`, `../../` prefixes. Language module imports (`tyrus:…`) are always placed at the top, followed by a blank line, then code imports.
- `false`, preserve the original import order from the source file.

---

## `line_width`

Maximum line length in characters. Arrays, object literals, function-call arguments, and ternary expressions that exceed this limit are automatically expanded to multi-line.

| Default | Values |
|---------|--------|
| `90`    | integer ≥ 0 |

---

## `object_expand`

Controls when an object literal is expanded to multi-line.

| Default    | Values                              |
|------------|-------------------------------------|
| `"simple"` | `"simple"`, `"expand-all"`, `"off"` |

- `"simple"`, expand only when at least one value is a complex expression. Example: `{ x: 1, y: 2 }` stays inline; `{ x: foo(1) }` expands.
- `"expand-all"`, always expand if there are multiple fields.
- `"off"`: never expand an object, even when it exceeds `line_width`.

---

## `pipeline_chain_layout`

Controls how pipeline chains with more than one `|>` are printed.

| Default          | Values                                |
|------------------|---------------------------------------|
| `"line-by-line"` | `"line-by-line"`, `"inline"`, `"off"` |

- `"line-by-line"`, print each pipeline step on its own line:

  ```typescript
  const result = value
      |> first
      |> second(%);
  ```

- `"inline"`, keep the pipeline chain on one line:

  ```typescript
  const result = value |> first |> second(%);
  ```

- `"off"`: preserve whether each source chain was written inline or across multiple lines.

---

## `prefer_arrow_function`

Controls whether a top-level function declaration is forced to a `const` bound to an arrow function. `function name() {}` and `const name = () => {}` parse to the same declaration, so the formatter has to pick one when it needs to normalize at all.

| Default | Values          |
|---------|-----------------|
| `false` | `true`, `false` |

- `false`, leave each declaration in whichever form it was written, so a file can freely mix `function` declarations and arrow-bound `const`s:

  ```typescript
  function main(): void {
      hello();
  }

  const hello = (): void => {
      console.log("hello");
  };
  ```

- `true`, force every declaration to the arrow form, regardless of how it was written:

  ```typescript
  const main = (): void => {
      hello();
  };

  const hello = (): void => {
      console.log("hello");
  };
  ```

---

## `quote_style`

Preferred quotation mark for string literals.

| Default    | Values                            |
|------------|-----------------------------------|
| `"double"` | `"double"`, `"single"`, `"off"` |

- `"off"` preserves the quotation mark and escapes already written in each string literal.

---

## `remove_unused_imports`

Removes unused import specifiers when enabled. If all specifiers in an import declaration are unused, the whole import declaration is removed.

| Default | Values          |
|---------|-----------------|
| `true`  | `true`, `false` |

- `true`, remove imported names that are not used in values or type positions.
- `false`, keep imports exactly as parsed, only formatting their layout.

---

## `trailing_comma`

Controls whether a trailing comma is added after the last element in multi-line arrays and objects.

| Default | Values          |
|---------|-----------------|
| `true`  | `true`, `false` |

- `true`: add trailing commas to multi-line arrays and objects.
- `false`: do not add trailing commas.
