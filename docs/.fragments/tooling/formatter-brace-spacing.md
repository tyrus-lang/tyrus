```json
{ "brace_spacing": true }
```

```tyrus
const config = { host: "localhost", port: 8080 };
```

With `true`, a space separates the braces from their content: `{ host: "localhost" }`.

```json
{ "brace_spacing": false }
```

```tyrus
const config = {host: "localhost", port: 8080};
```

With `false`, no space is printed: `{host: "localhost"}`.
