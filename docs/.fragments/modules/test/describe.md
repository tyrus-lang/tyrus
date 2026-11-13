```tyrus
import { describe, it, expect } from "tyrus:test";

const discount = (total: i32): i32 => {
    if (total >= 100) {
        return total / 10;
    }
    return 0;
};

describe("discount", () => {
    it("is zero below 100", () => {
        expect(discount(99)).toBe(0);
    });

    describe("from 100 on", () => {
        it("is ten percent", () => {
            expect(discount(250)).toBe(25);
        });
    });
});
```
