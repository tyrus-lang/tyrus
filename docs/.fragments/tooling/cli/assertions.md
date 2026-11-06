```tyrus
import { it } from "tyrus:test";

const double = (n: i32): i32 => n * 2;

const alwaysRejects = (): void throws string => {
    throw "insufficient funds";
};

it("doubles", () => {
    assert(double(0) === 0, "doubling zero stays zero");
    assertEqual(double(21), 42);
});

it("throws", () => {
    assertThrows(alwaysRejects);
    assertThrows(alwaysRejects, "insufficient funds");
});
```
