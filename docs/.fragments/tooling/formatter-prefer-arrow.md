```json
{ "prefer_arrow_function": false }
```

```tyrus
function main(): void {
    hello();
}

const hello = (): void => {
    print("hello");
};
```

With `false`, the formatter preserves each declaration in its original form, so `function` and arrow styles can coexist.

```json
{ "prefer_arrow_function": true }
```

```tyrus
const main = (): void => {
    hello();
};

const hello = (): void => {
    print("hello");
};
```

With `true`, every top-level function declaration is forced to the arrow form.
