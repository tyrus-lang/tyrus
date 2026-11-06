```tyrus
const text: string = "a1 b2 c3";
const matches: string[][]? = text.matchAll(/[a-z]\d/g);
if (matches) {
    for (const match of matches) {
        print(match[0]); // "a1", "b2", "c3"
    }
}
```

::: danger error
```tyrus
const text: string = "a1 b2 c3";
const matches: string[][]? = text.matchAll("[a-z]"); // [!code error]
```

```text
error[E0308]: type mismatch for `matchAll`: expected `Regex`, found `string`
at /src/main.tyr:2:30
const matches: string[][]? = text.matchAll("[a-z]");
                             ^^^^^^^^^^^^^^^^^^^^^^
help: change the value or the annotation so their types match; convert numbers explicitly with `value as <type>`
```
:::
