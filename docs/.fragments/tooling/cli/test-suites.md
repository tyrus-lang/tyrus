```tyrus
import { describe, it, expect, beforeEach } from "tyrus:test";

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
    beforeEach(() => {
        print("runs before every test in this suite");
    });

    it("keeps a value inside the range", () => {
        expect(clamp(5, 0, 10)).toBe(5);
    });

    it("lowers a value above the range", () => {
        expect(clamp(15, 0, 10)).toBe(10);
        expect(clamp(15, 0, 10)).not.toBe(15);
    });
});
```
