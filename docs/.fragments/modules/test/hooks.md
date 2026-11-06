```tyrus
import { describe, it, expect, beforeAll, afterAll, beforeEach, afterEach } from "tyrus:test";

describe("hooks", () => {
    beforeAll(() => {
        print("once, before the first test");
    });

    beforeEach(() => {
        print("before every test");
    });

    afterEach(() => {
        print("after every test");
    });

    afterAll(() => {
        print("once, after the last test");
    });

    it("runs between the hooks", () => {
        expect(1 + 1).toBe(2);
    });
});
```
