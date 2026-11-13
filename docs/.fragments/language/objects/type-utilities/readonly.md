```tyrus
interface Endpoint {
    port: i32;
}

interface Config {
    endpoint: Endpoint;
    name: string;
}

type Frozen = Readonly<Config>;

export function main(): void {
    let config: Frozen = { endpoint: { port: 8080 }, name: "api" };
    print(config.endpoint.port);
}
```

::: danger error
```tyrus
let config: Frozen = { endpoint: { port: 8080 }, name: "api" };
config.name = "web"; // [!code error]
```

```text
error[E2542]: cannot assign to readonly property `name`
at /src/main.tyr:18:1
config.name = "web";
^^^^^^^^^^^^^^^^^^^^
```
:::

The guarantee reaches every level, not only the surface. A nested object is frozen too:

::: danger error
```tyrus
let config: Frozen = { endpoint: { port: 8080 }, name: "api" };
config.endpoint.port = 9090; // [!code error]
```

```text
error[E2541]: cannot assign to `port` through a readonly value
at /src/main.tyr:18:1
config.endpoint.port = 9090;
^^^^^^^^^^^^^^^^^^^^^^^^^^^^
```
:::

An array field becomes a readonly array, so its mutating methods are refused as well:

::: danger error
```tyrus
interface Tagged {
    tags: string[];
}

let tagged: Readonly<Tagged> = { tags: ["a"] };
tagged.tags.push("b"); // [!code error]
```

```text
error[E2540]: cannot call mutating method `push` on readonly array
at /src/main.tyr:6:1
tagged.tags.push("b");
^^^^^^^^^^^
```
:::

::: warning Deeply recursive
`Readonly<T>` applies recursively to nested objects and arrays, so values cannot be mutated through a nested property.
:::

The examples above use `let`. With `const` the binding is already deeply immutable on its own, so the rejection would come from the binding rather than from the type, and would not show what `Readonly<T>` adds.
