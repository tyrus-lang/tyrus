```tyrus
interface Example {
    @lengthItems(3)
    coordinates: f64[];
}

const valid: Example = { coordinates: [1.0, 2.0, 3.0] };
```

::: danger error
```tyrus
const invalid: Example = { coordinates: [1.0, 2.0] }; // [!code error]
```

```text
error[validator-decorations-failed]: `@lengthItems(3)` failed for `coordinates`: array must contain exactly 3 items
at /src/main.tyr:6:41
const invalid: Example = { coordinates: [1.0, 2.0] };
                                        ^^^^^^^^^^
```
:::
