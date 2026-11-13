```tyrus
interface Example {
    @required()
    mandatoryParam: string;
}

const valid: Example = { mandatoryParam: "present" };
```

::: danger error
```tyrus
const invalid: Example = { /* missing mandatoryParam */ }; // [!code error]
```

```text
error[E2741]: property `mandatoryParam` is missing in object literal for type `Example`
at /src/main.tyr:6:7
const invalid: Example = { /* missing mandatoryParam */ };
      ^^^^^^^
```
:::
