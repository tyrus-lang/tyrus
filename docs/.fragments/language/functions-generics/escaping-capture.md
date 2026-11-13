::: danger error
```tyrus
function make(): () => i32 {
    let counter: i32 = 0;
    return (): i32 => {
        counter = counter + 1; // [!code error]
        return counter;
    };
}
```

```text
error[E0436]: cannot assign to `counter` from inside a closure that outlives its scope
at /src/main.tyr:4:9
        counter = counter + 1;
        ^^^^^^^^^^^^^^^^^^^^^^
help: this closure escapes the function that declared `counter`, so it captures a copy; the write lands in that copy and nothing can observe it
help: return the value from the closure instead of writing `counter`, or hold the state in a `Mutex` both sides can reach
```
:::
