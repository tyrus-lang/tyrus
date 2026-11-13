```tyrus
import { describe, it, expect, beforeEach } from "tyrus:test";
import { sleepAsync } from "tyrus:task";

async function fetchCount(): Promise<i32> {
    await sleepAsync(2);
    return 3;
}

describe("inventory", () => {
    beforeEach(async () => {
        await sleepAsync(1);
    });

    it("awaits the count", async () => {
        const count = await fetchCount();
        expect(count).toBe(3);
    });
});
```
