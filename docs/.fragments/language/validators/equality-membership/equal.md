```tyrus
interface Example {
    @equal("active")
    status: string;
}

const valid: Example = { status: "active" };
```

::: danger error
```tyrus
const invalid: Example = { status: "inactive" }; // [!code error]
```

```text
error[validator-decorations-failed]: `@equal("active")` failed for `status`: value must be equal to "active"
at /src/main.tyr:6:36
const invalid: Example = { status: "inactive" };
                                   ^^^^^^^^^^
```
:::
