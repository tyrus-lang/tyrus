---
title: Keywords
description: Active keywords, reserved keywords, and ambient builtins in Tyrus.
---

# Keywords

Keywords are reserved tokens in the Tyrus grammar that have special syntactic meanings. They cannot be used as identifier names for variables, constants, functions, interfaces, types, parameters, or other bindings.

## Active Keywords

Active keywords are recognized and parsed by the Tyrus compiler and lexer. They are organized into five primary functional categories.

### Declarations and Modifiers

These keywords introduce declarations, scope visibility, immutability, and compile-time evaluations.

| Keyword | Description |
|---|---|
| `async` | Declares asynchronous functions returning promises |
| `comptime` | Evaluates expressions or blocks at compile time |
| `const` | Declares deeply immutable bindings |
| `enum` | Defines an enumerated type with fixed variants |
| `export` | Publishes a symbol across all files in the project |
| `from` | Specifies the module specifier in import statements |
| `function` | Declares a named or anonymous function |
| `import` | Loads declarations from local files or native modules |
| `interface` | Defines a structural object shape or contract |
| `let` | Declares a mutable binding |
| `protect` | Restricts export reach to the declaring folder and subfolders |
| `readonly` | Enforces deep immutability on properties and types |
| `type` | Declares a type alias |
| `validator` | Declares custom validation logic for decorator validation |

### Control Flow and Error Handling

These keywords control branches, loops, function returns, deferred cleanups, and exception handling.

| Keyword | Description |
|---|---|
| `break` | Terminates execution of the nearest loop or switch |
| `case` | Defines a branch arm within a switch statement |
| `catch` | Captures errors thrown inside a try block |
| `continue` | Advances execution to the next loop iteration |
| `default` | Fallback branch in a switch statement |
| `defer` | Executes cleanup logic unconditionally upon exiting scope |
| `do` | Initiates a do-while loop block |
| `else` | Alternative branch in conditional statements |
| `errdefer` | Executes cleanup logic only when exiting scope with an error |
| `finally` | Executes cleanup logic regardless of success or exception |
| `for` | Loops over collections, ranges, or numeric steps |
| `if` | Conditional execution branch |
| `loop` | Unconditional infinite loop block |
| `match` | Pattern matching expression with destructuring |
| `onfail` | Attaches a recovery handler on block failure |
| `return` | Returns a value and terminates the current function |
| `switch` | Dispatches control based on literal values or enum tags |
| `throw` | Throws an error or exception |
| `throws` | Declares typed recoverable error contracts on functions |
| `try` | Executes a block with exception capture support |
| `while` | Loops while a condition evaluates to true |

### Expressions and Operators

These keywords act as operators, type casts, or clauses within expressions.

| Keyword | Description |
|---|---|
| `and` | Logical conjunction operator |
| `as` | Type assertion or import alias clause |
| `at` | Positional indexing or decorator target clause |
| `await` | Pauses execution awaiting asynchronous completion |
| `extends` | Type parameter constraint clause |
| `in` | Membership check in loops and expressions |
| `is` | Type checking operator |
| `not` | Logical negation operator |
| `of` | Value iteration clause in for-of loops |
| `or` | Logical disjunction operator |
| `typeof` | Inspects type metadata at compile time or runtime |

### Primitive Types

These keywords represent the built-in primitive scalar and numeric types of the language.

| Keyword | Description |
|---|---|
| `boolean` | Logical true or false value |
| `char` | 32-bit Unicode scalar character |
| `decimal` | Exact fixed-precision decimal arithmetic type |
| `f128` | 128-bit IEEE 754 floating-point number |
| `f32` | 32-bit single-precision floating-point number |
| `f64` | 64-bit double-precision floating-point number |
| `i128` | 128-bit signed integer |
| `i16` | 16-bit signed integer |
| `i32` | 32-bit signed integer |
| `i64` | 64-bit signed integer |
| `i8` | 8-bit signed integer |
| `isize` | Target architecture pointer-sized signed integer |
| `never` | Bottom type representing unreachable code or values |
| `string` | Immutable UTF-8 encoded string |
| `u128` | 128-bit unsigned integer |
| `u16` | 16-bit unsigned integer |
| `u32` | 32-bit unsigned integer |
| `u64` | 64-bit unsigned integer |
| `u8` | 8-bit unsigned integer |
| `unknown` | Top type requiring narrowing before member access |
| `usize` | Target architecture pointer-sized unsigned integer |
| `void` | Return type of functions yielding no value |

### Literal Values

These keywords denote primitive literals and absence constants.

| Keyword | Description |
|---|---|
| `false` | Boolean false literal |
| `Infinity` | Floating-point positive infinity literal |
| `NaN` | Floating-point not-a-number literal |
| `None` | Optional absence pattern sentinel |
| `null` | Reserved absence literal (prohibited as a runtime value) |
| `true` | Boolean true literal |

## Reserved for Future Use

These keywords are reserved for future language syntax and cannot be used as user-defined identifiers.

| Category | Keywords | Description |
|---|---|---|
| Classes and OOP | `class`, `constructor`, `implements`, `instanceof`, `new`, `override`, `super`, `this` | Reserved for potential object-oriented and class constructs |
| Visibility and Modifiers | `abstract`, `private`, `protected`, `public`, `static` | Reserved for member visibility and scope qualifiers |
| Modules and Scopes | `declare`, `module`, `namespace`, `package`, `require`, `var` | Reserved for ambient declarations and module system extensions |
| Type System | `asserts`, `infer`, `intrinsic`, `keyof`, `satisfies`, `unique` | Reserved for advanced type operators and narrowing |
| Accessors and Parameters | `get`, `out`, `set` | Reserved for property accessors and parameter passing modes |
| Execution and Control | `debugger`, `delete`, `when`, `with`, `yield` | Reserved for debugging, generator execution, and pattern conditions |

## Ambient Built-ins

These built-in identifiers are ambiently provided in the global scope without requiring an import statement.

| Identifier | Description |
|---|---|
| `assert` | Runtime assertion verifying a boolean condition |
| `assertEqual` | Runtime equality verification between two values |
| `assertThrows` | Runtime verification that a closure throws an expected error |
| `console` | Standard console logging namespace |
| `isNaN` | Checks whether a numeric value is NaN |
| `Map` | Built-in associative key-value map collection |
| `panic` | Aborts execution immediately with an error message |
| `print` | Standard output printing function |
| `printCR` | Carriage return output helper for terminal progress |
| `printError` | Standard error stream printing function |
| `printRaw` | Raw unformatted printing function |
| `printWarn` | Standard error warning printing function |
| `Set` | Built-in unique value set collection |

