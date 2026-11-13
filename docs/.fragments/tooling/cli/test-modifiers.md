```tyrus
import { describe, it, test, expect } from "tyrus:test";

const rejectEmpty = (): void throws string => {
    throw "empty input";
};

describe("input", () => {
    it("reports the error", () => {
        expect(rejectEmpty).toThrow("empty input");
    });

    test.skip("waits for the new parser", () => {
        expect(1).toBe(2);
    });

    test.todo("handles unicode input");
});
```
