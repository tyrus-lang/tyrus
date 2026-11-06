```tyrus
interface Example {
    @ipv4()
    localhost: string;
}

const valid: Example = { localhost: "127.0.0.1" };
```

::: danger error
```tyrus
const invalid: Example = { localhost: "999.999.999.999" }; // [!code error]
```

```text
error[validator-decorations-failed]: `@ipv4()` failed for `localhost`: value must be a valid IPv4 address
at /src/main.tyr:6:39
const invalid: Example = { localhost: "999.999.999.999" };
                                      ^^^^^^^^^^^^^^^^^
```
:::
