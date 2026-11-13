```tyrus
const fruits: string[] = ["apple", "banana", "cherry"];
const hasBanana: boolean = fruits.includes("banana");
// true
```

::: danger error
```tyrus
const fruits: string[] = ["apple", "banana", "cherry"];
const hasBanana: boolean = fruits.includes(42); // [!code error]
```

```text
error[E0308]: type mismatch for `includes`: expected `string`, found `integer literal`
at /src/main.tyr:2:28
const hasBanana: boolean = fruits.includes(42);
                           ^^^^^^^^^^^^^^^^^^^
help: change the value or the annotation so their types match; convert numbers explicitly with `value as <type>`
```
:::
