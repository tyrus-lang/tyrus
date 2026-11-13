```tyrus
const matches: string[][]? = /\w+/.execAll("one two");

if (matches) {
    print(matches[0][0]); // one
    print(matches[1][0]); // two
}
```
