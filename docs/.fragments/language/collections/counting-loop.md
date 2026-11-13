```tyrus
function sweep(values: i32[]): i32 {
    let sum = 0;
    
    for (let i: usize = 0; i < values.length; i += 1) {
        sum += values[i];
    }

    return sum;
}
```
