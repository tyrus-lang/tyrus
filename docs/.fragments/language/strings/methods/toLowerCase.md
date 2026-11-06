```tyrus
const shout: string = "TYRUS";
const quiet: string = shout.toLowerCase(); // "tyrus"
```

::: danger error
```tyrus
const flag: boolean = true;
const lower: string = flag.toLowerCase(); // [!code error]
```

```text
error[E0599]: no method named `toLowerCase` for type `boolean`
at /src/main.tyr:2:23
const lower: string = flag.toLowerCase();
                      ^^^^^^^^^^^^^^^^^^
help: check the member name and the receiver type; hover the value to see its type
```
:::
