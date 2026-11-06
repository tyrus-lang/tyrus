```tyrus
const letters: string[] = ["a", "b", "c", "d"];
const reversed: string = letters.reduceRight(
  (acc, letter) => acc + letter, "",
);
// "dcba"
```

::: danger error
```tyrus
const letters: string[] = ["a", "b", "c", "d"];
const reversed: i64 = letters.reduceRight((acc, letter) => acc + letter, ""); // [!code error]
```

```text
error[E0308]: type mismatch for `reversed`: expected `i64`, found `string`
at /src/main.tyr:2:7
const reversed: i64 = letters.reduceRight((acc, letter) => acc + letter, "");
      ^^^^^^^^
help: change the value or the annotation so their types match; convert numbers explicitly with `value as <type>`
```
:::
