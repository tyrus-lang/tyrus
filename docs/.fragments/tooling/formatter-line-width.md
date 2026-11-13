```json
{ "line_width": 80 }
```

```tyrus
const user = { name: "Alice", age: 30, email: "alice@example.com" };
```

If the line exceeds 80 characters, the formatter expands it:

```tyrus
const user = {
    name: "Alice",
    age: 30,
    email: "alice@example.com",
};
```

The same expansion applies to arrays, function-call arguments, and ternary expressions.
