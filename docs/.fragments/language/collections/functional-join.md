```tyrus
const parts: string[] = ["tyrus", "is", "native"];
const sentence: string = parts.join(" ");
// "tyrus is native"
```

::: danger error
```tyrus
const numbers: i32[] = [1, 2, 3];
const text: string = numbers.join(", "); // [!code error]
```

```text
error[E0277]: `join` currently requires a `string[]` receiver
at /src/main.tyr:2:22
const text: string = numbers.join(", ");
                     ^^^^^^^^^^^^^^^^^^
```
:::
