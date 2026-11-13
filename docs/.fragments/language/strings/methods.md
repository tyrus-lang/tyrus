### Case conversion

#### toUpperCase

```tyrus
const language: string = "tyrus";
const loud: string = language.toUpperCase(); // "TYRUS"
```

```tyrus
const value: i32 = 42;
const upper: string = value.toUpperCase(); // [!code error]
```

```text
error[E0308]: mismatched types
  --> src/main.tyr:2:35
   |
 2 | const upper: string = value.toUpperCase();
   |                       ^^^^^^^^^^^^^^^^^^^ expected `string`, found `i32`
   |
help: call `.toString()` first to convert `i32` to `string`
```

#### toLowerCase

```tyrus
const shout: string = "TYRUS";
const quiet: string = shout.toLowerCase(); // "tyrus"
```

```tyrus
const flag: boolean = true;
const lower: string = flag.toLowerCase(); // [!code error]
```

```text
error[E0308]: mismatched types
  --> src/main.tyr:2:34
   |
 2 | const lower: string = flag.toLowerCase();
   |                       ^^^^^^^^^^^^^^^^^^ expected `string`, found `boolean`
   |
help: call `.toString()` first to convert `boolean` to `string`
```

### Trimming

#### trim

```tyrus
const padded: string = "  hello  ";
const clean: string = padded.trim(); // "hello"
```

```tyrus
const value: i32 = 42;
const clean: string = value.trim(); // [!code error]
```

```text
error[E0308]: mismatched types
  --> src/main.tyr:2:34
   |
 2 | const clean: string = value.trim();
   |                       ^^^^^^^^^^^^ expected `string`, found `i32`
   |
help: call `.toString()` first to convert `i32` to `string`
```

#### trimStart

```tyrus
const padded: string = "  hello  ";
const left: string = padded.trimStart(); // "hello  "
```

```tyrus
const value: i32 = 42;
const left: string = value.trimStart(); // [!code error]
```

```text
error[E0308]: mismatched types
  --> src/main.tyr:2:33
   |
 2 | const left: string = value.trimStart();
   |                      ^^^^^^^^^^^^^^^^^ expected `string`, found `i32`
   |
help: call `.toString()` first to convert `i32` to `string`
```

#### trimEnd

```tyrus
const padded: string = "  hello  ";
const right: string = padded.trimEnd(); // "  hello"
```

```tyrus
const value: i32 = 42;
const right: string = value.trimEnd(); // [!code error]
```

```text
error[E0308]: mismatched types
  --> src/main.tyr:2:34
   |
 2 | const right: string = value.trimEnd();
   |                       ^^^^^^^^^^^^^^^ expected `string`, found `i32`
   |
help: call `.toString()` first to convert `i32` to `string`
```

### Searching

#### includes

```tyrus
const sentence: string = "tyrus is native";
const found: boolean = sentence.includes("native"); // true
```

```tyrus
const sentence: string = "tyrus is native";
const found: boolean = sentence.includes(42); // [!code error]
```

```text
error[E0308]: mismatched types
  --> src/main.tyr:2:47
   |
 2 | const found: boolean = sentence.includes(42);
   |                                          ^^ expected `string`, found `i32`
   |
help: convert the argument to a string first
```

#### startsWith

```tyrus
const filename: string = "main.tyr";
const isSource: boolean = filename.startsWith("main."); // true
```

```tyrus
const filename: string = "main.tyr";
const isSource: boolean = filename.startsWith(true); // [!code error]
```

```text
error[E0308]: mismatched types
  --> src/main.tyr:2:50
   |
 2 | const isSource: boolean = filename.startsWith(true);
   |                                                  ^^^^ expected `string`, found `boolean`
   |
help: pass a string literal or variable
```

#### endsWith

```tyrus
const filename: string = "main.tyr";
const isTyrus: boolean = filename.endsWith(".tyr"); // true
```

```tyrus
const filename: string = "main.tyr";
const isTyrus: boolean = filename.endsWith(42); // [!code error]
```

```text
error[E0308]: mismatched types
  --> src/main.tyr:2:47
   |
 2 | const isTyrus: boolean = filename.endsWith(42);
   |                                            ^^ expected `string`, found `i32`
   |
help: pass a string literal or variable
```

#### indexOf

```tyrus
const text: string = "hello world";
const position: i64 = text.indexOf("world"); // 6
const missing: i64 = text.indexOf("rust"); // -1
```

```tyrus
const text: string = "hello world";
const position: i64 = text.indexOf(["hello"]); // [!code error]
```

```text
error[E0308]: mismatched types
  --> src/main.tyr:2:42
   |
 2 | const position: i64 = text.indexOf(["hello"]);
   |                                    ^^^^^^^^^ expected `string`, found `string[]`
   |
help: pass a single string, not an array
```

#### lastIndexOf

```tyrus
const path: string = "src/main.tyr";
const lastSlash: i64 = path.lastIndexOf("/"); // 3
```

```tyrus
const path: string = "src/main.tyr";
const lastSlash: i64 = path.lastIndexOf(42); // [!code error]
```

```text
error[E0308]: mismatched types
  --> src/main.tyr:2:45
   |
 2 | const lastSlash: i64 = path.lastIndexOf(42);
   |                                         ^^ expected `string`, found `i32`
   |
help: pass a string value
```

#### search

```tyrus
const email: string = "user@example.com";
const atSign: i32 = email.search(/@/); // 4
```

```tyrus
const email: string = "user@example.com";
const atSign: i64 = email.search("@"); // [!code error]
```

```text
error[E0308]: mismatched types
  --> src/main.tyr:2:37
   |
 2 | const atSign: i64 = email.search("@");
   |                                  ^^^ expected `RegExp`, found `string`
   |
help: use a `RegExp` literal (`/pattern/`) instead of a string
```

### Slicing and extracting

#### slice

```tyrus
const code: string = "tyrus-lang";
const lang: string = code.slice(6); // "lang"
const sub: string = code.slice(0, 5); // "tyrus"
```

```tyrus
const code: string = "tyrus-lang";
const result: string = code.slice("6"); // [!code error]
```

```text
error[E0308]: mismatched types
  --> src/main.tyr:2:37
   |
 2 | const result: string = code.slice("6");
   |                                   ^^^ expected `i64` or `usize`, found `string`
   |
help: pass a numeric index
```

#### substring

```tyrus
const version: string = "v0.1.0";
const numeric: string = version.substring(1); // "0.1.0"
```

```tyrus
const version: string = "v0.1.0";
const numeric: string = version.substring("1"); // [!code error]
```

```text
error[E0308]: mismatched types
  --> src/main.tyr:2:41
   |
 2 | const numeric: string = version.substring("1");
   |                                           ^^^ expected `i64` or `usize`, found `string`
   |
help: pass a numeric index
```

#### charAt

```tyrus
const word: string = "tyrus";
const letter: string = word.charAt(0); // "t"
```

```tyrus
const word: string = "tyrus";
const letter: string = word.charAt("0"); // [!code error]
```

```text
error[E0308]: mismatched types
  --> src/main.tyr:2:37
   |
 2 | const letter: string = word.charAt("0");
   |                                    ^^^ expected `i64` or `usize`, found `string`
   |
help: pass a numeric index
```

#### charCodeAt

```tyrus
const letter: string = "A";
const code: i32 = letter.charCodeAt(0); // 65
```

```tyrus
const letter: string = "A";
const code: u32 = letter.charCodeAt("0"); // [!code error]
```

```text
error[E0308]: mismatched types
  --> src/main.tyr:2:40
   |
 2 | const code: u32 = letter.charCodeAt("0");
   |                                     ^^^ expected `i64` or `usize`, found `string`
   |
help: pass a numeric index
```

### Transforming

#### replace

```tyrus
const path: string = "src/main.ts";
const updated: string = path.replace(".ts", ".tyr"); // "src/main.tyr"
```

```tyrus
const path: string = "src/main.ts";
const updated: string = path.replace(".ts", 42); // [!code error]
```

```text
error[E0308]: mismatched types
  --> src/main.tyr:2:44
   |
 2 | const updated: string = path.replace(".ts", 42);
   |                                             ^^ expected `string`, found `i32`
   |
help: pass a string replacement
```

#### repeat

```tyrus
const divider: string = "-".repeat(20); // "--------------------"
const pattern: string = "ab".repeat(3); // "ababab"
```

```tyrus
const divider: string = "-".repeat("20"); // [!code error]
```

```text
error[E0308]: mismatched types
  --> src/main.tyr:1:29
   |
 1 | const divider: string = "-".repeat("20");
   |                                    ^^^^ expected `i64` or `usize`, found `string`
   |
help: pass a numeric count
```

#### padStart

```tyrus
const id: string = "42";
const padded: string = id.padStart(5, "0"); // "00042"
```

```tyrus
const id: string = "42";
const padded: string = id.padStart("5"); // [!code error]
```

```text
error[E0308]: mismatched types
  --> src/main.tyr:2:39
   |
 2 | const padded: string = id.padStart("5");
   |                                    ^^^ expected `i64` or `usize`, found `string`
   |
help: pass a numeric target length
```

#### padEnd

```tyrus
const label: string = "Name";
const padded: string = label.padEnd(20, "."); // "Name................"
```

```tyrus
const label: string = "Name";
const padded: string = label.padEnd("20"); // [!code error]
```

```text
error[E0308]: mismatched types
  --> src/main.tyr:2:39
   |
 2 | const padded: string = label.padEnd("20");
   |                                     ^^^^ expected `i64` or `usize`, found `string`
   |
help: pass a numeric target length
```

#### concat

```tyrus
const first: string = "Hello";
const second: string = " World";
const combined: string = first.concat(second); // "Hello World"
```

```tyrus
const first: string = "Hello";
const combined: string = first.concat(42); // [!code error]
```

```text
error[E0308]: mismatched types
  --> src/main.tyr:2:38
   |
 2 | const combined: string = first.concat(42);
   |                                       ^^ expected `string`, found `i32`
   |
help: call `.toString()` first or pass a string
```

#### split

```tyrus
const csv: string = "rust,go,tyrus";
const languages: string[] = csv.split(",");
// ["rust", "go", "tyrus"]

const sentence: string = "hello world";
const words: string[] = sentence.split(" ");
// ["hello", "world"]
```

```tyrus
const csv: string = "rust,go,tyrus";
const languages: string[] = csv.split(42); // [!code error]
```

```text
error[E0308]: mismatched types
  --> src/main.tyr:2:44
   |
 2 | const languages: string[] = csv.split(42);
   |                                       ^^ expected `string`, found `i32`
   |
help: pass a string or `RegExp` separator
```

### Pattern matching

#### match

```tyrus
const text: string = "order-123";
const result: string[]? = text.match(/^order-(\d+)$/);
if (result) {
    const orderId: string = result[0]; // "order-123"
    const digits: string = result[1]; // "123"
}
```

```tyrus
const text: string = "order-123";
const result: string[]? = text.match("order"); // [!code error]
```

```text
error[E0308]: mismatched types
  --> src/main.tyr:2:42
   |
 2 | const result: string[]? = text.match("order");
   |                                      ^^^^^^^ expected `RegExp`, found `string`
   |
help: use a `RegExp` literal (`/pattern/`) instead of a string
```

#### matchAll

```tyrus
const text: string = "a1 b2 c3";
const matches: string[][]? = text.matchAll(/[a-z]\d/);
if (matches) {
    for (const found of matches) {
        print(found[0]); // "a1", "b2", "c3"
    }
}
```

```tyrus
const text: string = "a1 b2 c3";
const matches: string[][]? = text.matchAll("[a-z]"); // [!code error]
```

```text
error[E0308]: mismatched types
  --> src/main.tyr:2:45
   |
 2 | const matches: string[][]? = text.matchAll("[a-z]");
   |                                            ^^^^^^^ expected `RegExp`, found `string`
   |
help: use a `RegExp` literal (`/pattern/`) instead of a string
```

### Iteration

#### chars

```tyrus
const emoji: string = "hello";
for (const letter of emoji.chars()) {
    print(letter); // "h", "e", "l", "l", "o"
}
```

```tyrus
const value: i32 = 42;
for (const letter of value.chars()) { // [!code error]
}
```

```text
error[E0308]: mismatched types
  --> src/main.tyr:2:26
   |
 2 | for (const letter of value.chars()) {
   |                      ^^^^^^^^^^^^^ expected `string`, found `i32`
   |
help: call `.toString()` first to convert `i32` to `string`
```
