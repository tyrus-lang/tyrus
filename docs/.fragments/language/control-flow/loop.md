```tyrus
let index = 0;

loop {
    index += 1;
    
    if (index < 3) {
        continue;
    }

    if (index === 5) {
        break;
    }

    print(index); // 3, 4
}

const found: i32 = loop {
    break 42;
};

print(found); // 42
```
