```tyrus
interface Example {
    @notBlank()
    bio: string;
}

const valid: Example = { bio: "Hello" };
```

::: danger error
```tyrus
const invalid: Example = { bio: "   " }; // [!code error]
```

```text
error[validator-decorations-failed]: `@notBlank()` failed for `bio`: value must not be blank
at /src/main.tyr:6:33
const invalid: Example = { bio: "   " };
                                ^^^^^
```
:::
