```tyrus
const shout: string = "tyrus";
const loud: string = shout.toUpperCase(); // TYRUS
```

::: danger error
```tyrus
const flag: boolean = true;
const upper: string = flag.toUpperCase(); // [!code error]
```

```text
error[E0599]: no method named `toUpperCase` for type `boolean`
at /src/main.tyr:2:23
const upper: string = flag.toUpperCase();
                      ^^^^^^^^^^^^^^^^^^
help: check the member name and the receiver type; hover the value to see its type
```
:::
