```tyrus
import { describe, test, expect } from "tyrus:test";

const clamp = (value: i32, low: i32, high: i32): i32 => {
    if (value < low) {
        return low;
    }
    if (value > high) {
        return high;
    }
    return value;
};

describe("clamp", () => {
    test.each("keeps a value in range", [0, 5, 10], (value: i32) => {
        expect(clamp(value, 0, 10)).toBe(value);
    });
});
```
