```tyrus
interface Example {
    @maxItems(10)
    selectedIds: i32[];
}

const valid: Example = { selectedIds: [1, 2, 3] };
```

::: danger error
```tyrus
const invalid: Example = { selectedIds: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11] }; // [!code error]
```

```text
error[validator-decorations-failed]: `@maxItems(10)` failed for `selectedIds`: array must contain at most 10 items
at /src/main.tyr:6:41
const invalid: Example = { selectedIds: [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11] };
                                        ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
```
:::
