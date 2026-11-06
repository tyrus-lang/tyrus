```tyrus
const numbers: i32[] = [1, 2, 3, 4, 5, 6];
print(numbers.count());               // 6
print(numbers.count(3));              // 1
print(numbers.count(value % 2 === 0)); // 3
print(numbers.count(value > 3));       // 3
```

::: danger error
```tyrus
const numbers: i32[] = [1, 2, 3, 4, 5, 6];
const total: i64 = numbers.count(value => value.toString()); // [!code error]
```

```text
error[E0282]: cannot infer type of parameter `value`
at /src/main.tyr:2:43
const total: i64 = numbers.count(value => value.toString());
                                          ^^^^^
error[E0599]: no method named `toString` for type `<inferred>`
at /src/main.tyr:2:43
const total: i64 = numbers.count(value => value.toString());
                                          ^^^^^^^^^^^^^^^^
help: check the member name and the receiver type; hover the value to see its type
error[E0308]: `count` expects a value assignable to `i32`, found `(<inferred>) => void`
at /src/main.tyr:2:34
const total: i64 = numbers.count(value => value.toString());
                                 ^^^^^^^^^^^^^^^^^^^^^^^^^
help: change the value or the annotation so their types match; convert numbers explicitly with `value as <type>`
error[E0308]: type mismatch for `total`: expected `i64`, found `usize`
at /src/main.tyr:2:7
const total: i64 = numbers.count(value => value.toString());
      ^^^^^
help: `usize` does not convert to `i64` on its own; write the conversion explicitly with `as i64`
```
:::
