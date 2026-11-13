```tyrus
const front: i32[] = [1, 2];
const back: i32[] = [3, 4];
const merged: i32[] = front.concat(back);
// [1, 2, 3, 4]
```

::: danger error
```tyrus
const front: string[] = ["a", "b"];
const merged: string[] = front.concat(42); // [!code error]
```

```text
error[E0308]: type mismatch for `concat`: expected `string[]`, found `integer literal`
at /src/main.tyr:2:26
const merged: string[] = front.concat(42);
                         ^^^^^^^^^^^^^^^^
help: change the value or the annotation so their types match; convert numbers explicitly with `value as <type>`
```
:::
