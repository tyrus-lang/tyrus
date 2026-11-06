```tyrus
interface Example {
    @ipv6()
    ipv6Host: string;
}

const valid: Example = { ipv6Host: "::1" };
```

::: danger error
```tyrus
const invalid: Example = { ipv6Host: "not-ipv6" }; // [!code error]
```

```text
error[validator-decorations-failed]: `@ipv6()` failed for `ipv6Host`: value must be a valid IPv6 address
at /src/main.tyr:6:38
const invalid: Example = { ipv6Host: "not-ipv6" };
                                     ^^^^^^^^^^
```
:::
