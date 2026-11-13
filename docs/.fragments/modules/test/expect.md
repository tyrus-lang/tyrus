```tyrus
import { it, expect } from "tyrus:test";

const rejectEmpty = (): void throws string => {
    throw "empty input";
};

const load = (): Promise<i32> => {
    return Promise.resolve<i32>(42);
};

const failToLoad = (): Promise<i32> => {
    return Promise.reject<i32>({ message: "not found" });
};

it("compares values", () => {
    expect(2 + 2).toBe(4);
    expect("tyrus").toHaveLength(5);
    expect(0.1 + 0.2).toBeCloseTo(0.3);
    expect(10).toBeGreaterThan(3);
    expect("native tests").toContain("tests");
});

it("inverts and checks errors", () => {
    expect(2 + 2).not.toBe(5);
    expect(rejectEmpty).toThrow("empty input");
});

it("settles promises", () => {
    expect(load()).resolves.toBe(42);
    expect(load()).resolves.toBeGreaterThan(40);
    expect(failToLoad()).rejects.toThrow("not found");
});
```
