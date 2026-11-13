```tyrus
const phrase: string = "hello world example";
const camel: string = phrase.toCamelCase(); // "helloWorldExample"

const pascal: string = "HelloWorldExample";
const result: string = pascal.toCamelCase(); // "helloWorldExample"
```

::: danger error
```tyrus
const flag: boolean = true;
const camel: string = flag.toCamelCase(); // [!code error]
```

```text
error[E0599]: no method named `toCamelCase` for type `boolean`
at /src/main.tyr:2:23
const camel: string = flag.toCamelCase();
                      ^^^^^^^^^^^^^^^^^^
help: check the member name and the receiver type; hover the value to see its type
```
:::
