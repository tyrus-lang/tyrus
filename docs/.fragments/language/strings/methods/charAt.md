```tyrus
const word: string = "tyrus";
const letter: string = word.charAt(0); // "t"
```

::: danger error
```tyrus
const word: string = "tyrus";
const letter: string = word.charAt("0"); // [!code error]
```

```text
error[E0308]: type mismatch for `charAt`: expected `usize`, found `string`
at /src/main.tyr:2:24
const letter: string = word.charAt("0");
                       ^^^^^^^^^^^^^^^^
help: change the value or the annotation so their types match; convert numbers explicitly with `value as <type>`
```
:::
