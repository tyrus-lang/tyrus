```tyrus
interface Example {
    @notIn(["admin", "superadmin"])
    role: string;
}

const valid: Example = { role: "user" };
```

::: danger error
```tyrus
const invalid: Example = { role: "admin" }; // [!code error]
```

```text
error[validator-decorations-failed]: `@notIn(["admin", "superadmin"])` failed for `role`: value must not be one of ["admin", "superadmin"]
at /src/main.tyr:6:34
const invalid: Example = { role: "admin" };
                                 ^^^^^^^
```
:::
