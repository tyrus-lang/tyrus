```tyrus
const describe = (value: unknown): string => {
    if (typeof value === "string") {
        return `text with ${value.length} characters`;
    }
    if (typeof value === "i32") {
        return `the integer after ${value + 1}`;
    }
    return typeof value;
};

const main = (): void => {
    print(describe("Tyrus")); // text with 5 characters
    print(describe(41)); // the integer after 42
    print(describe(true)); // boolean
};
```
