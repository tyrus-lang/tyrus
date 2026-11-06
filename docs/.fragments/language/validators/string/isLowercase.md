```tyrus
interface Example {
    @isLowercase()
    slug: string;
}

const valid: Example = { slug: "valid-slug" };
```

::: danger error
```tyrus
const invalid: Example = { slug: "Invalid-Slug" }; // [!code error]
```

```text
error[validator-decorations-failed]: `@isLowercase()` failed for `slug`: value must already be lowercase
at /src/main.tyr:6:34
const invalid: Example = { slug: "Invalid-Slug" };
                                 ^^^^^^^^^^^^^^
```
:::
