```tyrus
const emoji: string = "hello";
for (const letter of emoji.chars()) {
    print(letter); // "h", "e", "l", "l", "o"
}
```

::: danger error
```tyrus
const value: i32 = 42;
for (const letter of value.chars()) { // [!code error]
}
```

```text
error[E0599]: no method named `chars` for type `i32`
at /src/main.tyr:2:22
for (const letter of value.chars()) {
                     ^^^^^^^^^^^^^
help: check the member name and the receiver type; hover the value to see its type
```
:::
