```tyrus
interface Example {
    @notEqual("deleted")
    accountType: string;
}

const valid: Example = { accountType: "active" };
```

::: danger error
```tyrus
const invalid: Example = { accountType: "deleted" }; // [!code error]
```

```text
error[validator-decorations-failed]: `@notEqual("deleted")` failed for `accountType`: value must be different from "deleted"
at /src/main.tyr:6:41
const invalid: Example = { accountType: "deleted" };
                                        ^^^^^^^^^
```
:::
