```tyrus
const temperatures: f64[] = [36.5, 37.0, 36.8];
const allNormal: boolean = temperatures.every(t => t >= 36.0 && t <= 37.5);
// true
```

::: danger error
```tyrus
const temperatures: f64[] = [36.5, 37.0, 36.8];
const allNormal: boolean = temperatures.every(t => t + 1); // [!code error]
```

```text
error[E0369]: operator `+` cannot be applied to `f64` and `integer literal`
at /src/main.tyr:2:52
const allNormal: boolean = temperatures.every(t => t + 1);
                                                   ^^^^^
```
:::
