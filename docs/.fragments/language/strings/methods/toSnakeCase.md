```tyrus
const phrase: string = "hello world example";
const snake: string = phrase.toSnakeCase(); // "hello_world_example"

const camel: string = "HelloWorldExample";
const result: string = camel.toSnakeCase(); // "hello_world_example"
```

::: danger error
```tyrus
const flag: boolean = true;
const snake: string = flag.toSnakeCase(); // [!code error]
```

```text
error[E0599]: no method named `toSnakeCase` for type `boolean`
at /src/main.tyr:2:23
const snake: string = flag.toSnakeCase();
                      ^^^^^^^^^^^^^^^^^^
help: check the member name and the receiver type; hover the value to see its type
```
:::
