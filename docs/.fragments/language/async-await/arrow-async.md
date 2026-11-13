```tyrus
const fetchData = async (): Promise<i32> => {
    return 42;
};

async function main(): Promise<void> {
    const value: i32 = await fetchData();
    print(value); // 42
}
```
