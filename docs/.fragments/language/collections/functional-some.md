```tyrus
const ages: i32[] = [15, 22, 17, 30];
const hasMinor: boolean = ages.some(age => age < 18);
// true
```

::: danger error
```tyrus
const ages: i32[] = [15, 22, 17, 30];
const hasMinor: boolean = ages.some(age => age * 2); // [!code error]
```

```text
error[E0308]: type mismatch for `return`: expected `boolean`, found `i32`
at /src/main.tyr:2:44
const hasMinor: boolean = ages.some(age => age * 2);
                                           ^^^^^^^
help: change the value or the annotation so their types match; convert numbers explicitly with `value as <type>`
```
:::
