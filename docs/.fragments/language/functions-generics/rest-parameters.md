```tyrus
const sum = (...values: i32[]): i32 => {
    let total: i32 = 0;

    for (const value of values) {
        total += value;
    }

    return total;
};

const numbers: i32[] = [1, 2, 3];
print(sum(1, 2, 3));   // individual arguments
print(sum(...numbers)); // spread of an array
print(sum());          // no arguments
```
