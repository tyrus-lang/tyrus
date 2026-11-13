```tyrus
const phrase: string = "hello world example";
const dashed: string = phrase.toDashCase(); // "hello-world-example"

const camel: string = "HelloWorldExample";
const result: string = camel.toDashCase(); // "hello-world-example"
```

::: danger error
```tyrus
const flag: boolean = true;
const dashed: string = flag.toDashCase(); // [!code error]
```

```text
error[E0599]: no method named `toDashCase` for type `boolean`
at /src/main.tyr:2:24
const dashed: string = flag.toDashCase();
                       ^^^^^^^^^^^^^^^^^
help: check the member name and the receiver type; hover the value to see its type
```
:::
