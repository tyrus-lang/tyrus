```tyrus
import { describe, it, test, expect } from "tyrus:test";

describe("parser", () => {
    it("runs", () => {
        expect(1).toBe(1);
    });

    test.skip("waits for the new grammar", () => {
        expect(1).toBe(2);
    });

    test.todo("reports unicode columns");
});

describe.skip("legacy format", () => {
    it("is not run", () => {
        expect(1).toBe(2);
    });
});
```
