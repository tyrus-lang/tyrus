```tyrus
const pairs: [i32, i32][] = [[1, 2], [3, 4]];

for (const [left, right] of pairs) {
    print(left + right); // 3, 7
}

const scores = Map<string, i32>();
scores.set("Smith", 1);

for (const [name, score] of scores.entries()) {
    print(name, score); // "Smith" 1
}
```
