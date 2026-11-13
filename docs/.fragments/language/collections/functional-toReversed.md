```tyrus
const days: string[] = ["Mon", "Tue", "Wed"];
const reversed: string[] = days.toReversed();
// ["Wed", "Tue", "Mon"]
// days is unchanged
```

::: danger error
```tyrus
const days: string[] = ["Mon", "Tue", "Wed"];
const reversed: i32[] = days.toReversed(); // [!code error]
```

```text
error[E0308]: type mismatch for `reversed`: expected `i32[]`, found `string[]`
at /src/main.tyr:2:7
const reversed: i32[] = days.toReversed();
      ^^^^^^^^
help: change the value or the annotation so their types match; convert numbers explicitly with `value as <type>`
```
:::
