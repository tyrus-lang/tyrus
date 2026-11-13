```tyrus
const fruits: string[] = ["apple", "banana", "cherry"];

for (const [index, fruit] of fruits.entries()) {
    print(`${index}: ${fruit}`);
    // 0: apple
    // 1: banana
    // 2: cherry
}
```

::: danger error
```tyrus
const fruits: string[] = ["apple", "banana", "cherry"];
const entries: [i64, string][] = fruits.entries(); // [!code error]
```

```text
error[E0308]: type mismatch for `entries`: expected `[i64, string][]`, found `[usize, string][]`
at /src/main.tyr:2:7
const entries: [i64, string][] = fruits.entries();
      ^^^^^^^
help: change the value or the annotation so their types match; convert numbers explicitly with `value as <type>`
```
:::
