```tyrus
interface Example {
    @ip()
    clientIp: string;
}

const valid: Example = { clientIp: "192.168.1.1" };
```

::: danger error
```tyrus
const invalid: Example = { clientIp: "999.999.999.999" }; // [!code error]
```

```text
error[validator-decorations-failed]: `@ip()` failed for `clientIp`: value must be a valid IP address
at /src/main.tyr:6:38
const invalid: Example = { clientIp: "999.999.999.999" };
                                     ^^^^^^^^^^^^^^^^^
```
:::
