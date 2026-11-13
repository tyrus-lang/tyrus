```tyrus
type Fn = (i32) => string;
type Result = ReturnType<Fn>; // string
```

::: danger error
```tyrus
type NotFn = i32;
type Bad = ReturnType<NotFn>; // [!code error]
```

```text
error[utility-non-function]: `ReturnType` requires a function type, found `i32`
at /src/main.tyr:1:1
type NotFn = i32;
^
```
:::
