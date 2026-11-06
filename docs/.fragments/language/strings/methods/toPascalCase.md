```tyrus
const phrase: string = "hello world example";
const pascal: string = phrase.toPascalCase(); // "HelloWorldExample"

const snake: string = "hello_world_example";
const result: string = snake.toPascalCase(); // "HelloWorldExample"
```

::: danger error
```tyrus
const flag: boolean = true;
const pascal: string = flag.toPascalCase(); // [!code error]
```

```text
error[E0599]: no method named `toPascalCase` for type `boolean`
at /src/main.tyr:2:24
const pascal: string = flag.toPascalCase();
                       ^^^^^^^^^^^^^^^^^^^
help: check the member name and the receiver type; hover the value to see its type
```
:::
