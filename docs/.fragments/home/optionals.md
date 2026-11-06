```tyrus
const findAccount = (id: i32): Account? => {
    if (id > 0) {
        return { id, nickname: "ada" };
    }
    
    return;  // absence
};

const account: Account? = findAccount(1);
print(account?.nickname ?? "anon");

// Uninitialized optional starts empty
let pending: i32?;
print(pending ?? 0);  // 0
```
