```tyrus
let sum = 0;

for (const i of 0..5) {
    sum += i;
}

print(sum); // 10 (0..5 excludes 5)
```

```tyrus
const main = (): void => {
    for (const i of 0..=3) {
        print(i); // 0, 1, 2, 3 (0..=3 includes 3)
    }
};
```
