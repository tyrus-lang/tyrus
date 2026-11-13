```json
{ "object_expand": "simple" }
```

```tyrus
const simple = { x: 1, y: 2 };
const complex = { x: foo(1), y: bar(2) };
```

With `"simple"`, the first object stays inline because values are literals, while the second expands because values are function calls.

```json
{ "object_expand": "expand-all" }
```

```tyrus
const simple = {
    x: 1,
    y: 2,
};
```

With `"expand-all"`, every multi-field object expands regardless of value complexity.

```json
{ "object_expand": "off" }
```

```tyrus
const options = { first: firstValue, second: secondValue };
```

With `"off"`, objects remain inline even when they exceed `line_width`.
