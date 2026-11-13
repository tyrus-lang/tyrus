```tyrus
const letters: string[] = ["a", "b", "c", "d", "e"];
const middle: string[] = letters.slice(1, 4);
// ["b", "c", "d"]
```

::: danger error
```tyrus
const letters: string[] = ["a", "b", "c", "d", "e"];
const middle: i32[] = letters.slice(1, 4); // [!code error]
```

```text
error[E0308]: type mismatch for `middle`: expected `i32[]`, found `string[]`
at /src/main.tyr:2:7
const middle: i32[] = letters.slice(1, 4);
      ^^^^^^
help: change the value or the annotation so their types match; convert numbers explicitly with `value as <type>`
```
:::
