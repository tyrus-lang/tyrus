```tyrus
interface Example {
    @regex(/^[a-z]+$/)
    alphaName: string;
}

const valid: Example = { alphaName: "hello" };
```

::: danger error
```tyrus
const invalid: Example = { alphaName: "user123" }; // [!code error]
```

```text
error[validator-decorations-failed]: `@regex(/^[a-z]+$/)` failed for `alphaName`: value must match /^[a-z]+$/
at /src/main.tyr:6:39
const invalid: Example = { alphaName: "user123" };
                                      ^^^^^^^^^
```
:::
