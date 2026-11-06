```tyrus
interface Example {
    @endsWith(".md")
    fileName: string;
}

const valid: Example = { fileName: "readme.md" };
```

::: danger error
```tyrus
const invalid: Example = { fileName: "readme.txt" }; // [!code error]
```

```text
error[validator-decorations-failed]: `@endsWith(".md")` failed for `fileName`: value must end with ".md"
at /src/main.tyr:6:38
const invalid: Example = { fileName: "readme.txt" };
                                     ^^^^^^^^^^^^
```
:::
