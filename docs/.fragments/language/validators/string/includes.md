```tyrus
interface Example {
    @includes("@")
    handle: string;
}

const valid: Example = { handle: "user@domain" };
```

::: danger error
```tyrus
const invalid: Example = { handle: "no-at-sign" }; // [!code error]
```

```text
error[validator-decorations-failed]: `@includes("@")` failed for `handle`: value must include "@"
at /src/main.tyr:6:36
const invalid: Example = { handle: "no-at-sign" };
                                   ^^^^^^^^^^^^
```
:::
