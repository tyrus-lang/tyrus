```tyrus
const match: string[]? = /(c)(at)/.exec("a cat");

if (match) {
    print(match[0]); // cat
    print(match[1]); // c
}
```
