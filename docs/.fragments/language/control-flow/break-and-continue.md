```tyrus
for (let index: u32 = 0; index < 10; index += 1) {
    if (index === 2) {
        continue;
    }

    if (index === 5) {
        break;
    }

    print(index); // 0, 1, 3, 4
}
```
