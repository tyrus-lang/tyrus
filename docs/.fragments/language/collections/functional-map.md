```tyrus
const prices: f64[] = [9.99, 19.99, 29.99];
const withTax: f64[] = prices.map(price => price * 1.08);
// [10.7892, 21.5892, 32.3892]
```

::: danger error
```tyrus
const prices: f64[] = [9.99, 19.99, 29.99];
const withTax: string[] = prices.map(price => price * 1.08); // [!code error]
```

```text
error[E0308]: type mismatch for `withTax`: expected `string[]`, found `f64[]`
at /src/main.tyr:2:7
const withTax: string[] = prices.map(price => price * 1.08);
      ^^^^^^^
help: change the value or the annotation so their types match; convert numbers explicitly with `value as <type>`
```
:::
