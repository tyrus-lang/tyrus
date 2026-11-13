```tyrus
interface Example {
    @port()
    serverPort: u16;
}

const valid: Example = { serverPort: 8080 };
```

::: danger error
```tyrus
const invalid: Example = { serverPort: 80 }; // [!code error]
```

```text
error[validator-decorations-failed]: `@port()` failed for `serverPort`: value must be a registered port between 1024 and 49151
at /src/main.tyr:6:40
const invalid: Example = { serverPort: 80 };
                                       ^^
```
:::
