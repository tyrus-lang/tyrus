```tyrus
const readings: i32[] = [1, 2, 3, 4, 5, 6, 7, 8];
const [selected, remaining] = readings.partition((reading, index) => {
    return reading > 1 && index < 3;
});
print(selected); // [2, 3]
print(remaining); // [1, 4, 5, 6, 7, 8]
print(readings); // [1, 2, 3, 4, 5, 6, 7, 8]
```
