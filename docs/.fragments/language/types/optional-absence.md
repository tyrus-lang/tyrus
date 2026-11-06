```tyrus
const findAccount = (found: boolean): Account? => {
    if (found) {
        return { id: 1 };
    }

    return; // empty
};

let pending: i32?; // starts empty
const size: usize? = pending?.length;

print(size ?? 0); // 0
```
