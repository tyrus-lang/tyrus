```tyrus
import { test, it, expect } from "tyrus:test";

const greet = (name: string): string => {
    return `Hello, ${name}!`;
};

test("greets by name", () => {
    expect(greet("Ada")).toBe("Hello, Ada!");
});

it("keeps the punctuation", () => {
    expect(greet("Grace")).toContain("!");
});
```
