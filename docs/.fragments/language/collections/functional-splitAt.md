```tyrus
const letters: string[] = ["a", "b", "c", "d", "e"];
const parts: [string[], string[]] = letters.splitAt(2);
// [["a", "b"], ["c", "d", "e"]]
```

::: danger error
```tyrus
const letters: string[] = ["a", "b"];
const parts: [string[], string[]] = letters.splitAt(10); // [!code error]
```

```text
error[E0606]: splitAt index 10 is out of bounds: valid indices are 0 to 2 (length 2)
at /src/main.tyr:2:53
const parts: [string[], string[]] = letters.splitAt(10);
                                                    ^^
```
:::
