```tyrus
import { it, expect } from "tyrus:test";

it("is left out while another test is focused", () => {
    expect(1).toBe(1);
});

it.only("is the only test that runs", () => {
    expect(2).toBe(2);
});
```
