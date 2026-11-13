```tyrus
const csv: string = "rust,go,tyrus";
const languages: string[] = csv.split(",");
// ["rust", "go", "tyrus"]

const sentence: string = "hello world";
const words: string[] = sentence.split(" ");
// ["hello", "world"]
```

::: danger error
```tyrus
const csv: string = "rust,go,tyrus";
const languages: string[] = csv.split(42); // [!code error]
```

```text
error[E0308]: type mismatch for `split`: expected `string`, found `integer literal`
at /src/main.tyr:2:29
const languages: string[] = csv.split(42);
                            ^^^^^^^^^^^^^
help: change the value or the annotation so their types match; convert numbers explicitly with `value as <type>`
```
:::
