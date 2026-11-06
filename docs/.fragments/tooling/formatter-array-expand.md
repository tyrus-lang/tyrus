```json
{ "array_expand": "simple" }
```

```tyrus
const simple = ["a", "b", "c"];
const complex = [foo(1), bar(2)];
```

With `"simple"`, the first array stays inline because all elements are literals, while the second expands because it contains function calls.

```json
{ "array_expand": "expand-all" }
```

```tyrus
const simple = [
    "a",
    "b",
    "c",
];
```

With `"expand-all"`, every multi-element array expands regardless of complexity.

```json
{ "array_expand": "off" }
```

```tyrus
const values = [firstValue, secondValue];
```

With `"off"`, arrays remain inline even when they exceed `line_width`.
