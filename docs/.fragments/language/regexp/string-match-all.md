```tyrus
const matches: string[][]? = "one two".matchAll(/\w+/);

if (matches) {
    print(matches[1][0]); // two
}
```
