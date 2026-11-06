```tyrus
type Fn = (i32, string) => boolean;
type Params = Parameters<Fn>;
```

::: danger error
```tyrus
type NotFn = i32;
type Bad = Parameters<NotFn>; // [!code error]
```

```text
error[utility-non-function]: `Parameters` requires a function type, found `i32`
at /src/main.tyr:1:1
type NotFn = i32;
^
```
:::
