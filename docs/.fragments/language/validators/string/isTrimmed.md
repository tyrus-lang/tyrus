```tyrus
interface Example {
    @isTrimmed()
    title: string;
}

const valid: Example = { title: "no spaces" };
```

::: danger error
```tyrus
const invalid: Example = { title: " spaces " }; // [!code error]
```

```text
error[validator-decorations-failed]: `@isTrimmed()` failed for `title`: value must already be trimmed
at /src/main.tyr:6:35
const invalid: Example = { title: " spaces " };
                                  ^^^^^^^^^^
```
:::
