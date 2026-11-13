```tyrus
const colors: string[] = ["red", "green", "blue"];
colors.forEach(color => print(color));
// red
// green
// blue
```

::: danger error
```tyrus
const colors: string[] = ["red", "green", "blue"];
colors.forEach((color: i32) => print(color)); // [!code error]
```

```text
error[E0308]: type mismatch for callback parameter `color` of `forEach`: expected `string`, found `i32`
at /src/main.tyr:2:24
colors.forEach((color: i32) => print(color));
                       ^^^
help: change the callback parameter type to `string`
```
:::
