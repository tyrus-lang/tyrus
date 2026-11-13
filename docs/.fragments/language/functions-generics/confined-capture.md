```tyrus
function main(): void {
    let counter: i32 = 0;
    const bump = (): i32 => {
        counter = counter + 1;
        return counter;
    };

    print(bump()); // 1
    print(bump()); // 2
    print(counter); // 2
}
```
