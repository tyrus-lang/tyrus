```tyrus
interface Example {
    @powerOfTwo()
    bufferSize: u32;
}

const valid: Example = { bufferSize: 8 };
```

::: danger error
```tyrus
const invalid: Example = { bufferSize: 3 }; // [!code error]
```

```text
error[validator-decorations-failed]: `@powerOfTwo()` failed for `bufferSize`: value must be a power of two
at /src/main.tyr:6:40
const invalid: Example = { bufferSize: 3 };
                                       ^
```
:::
