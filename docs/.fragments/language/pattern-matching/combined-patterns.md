```tyrus
type Input =
    | string
    | number
    | boolean;

const process = (input: Input): string => {
    match (input) {
        (s: string) if (s.length === 0) => {
            return "Empty string";
        }
        (s: string) => {
            return `String: ${s}`;
        }
        (n: number) if (n < 0) => {
            return "Negative number";
        }
        (n: number) if (n === 0) => {
            return "Zero";
        }
        (n: number) => {
            return `Positive number: ${n}`;
        }
        (true) => {
            return "Boolean true";
        }
        (false) => {
            return "Boolean false";
        }
    }
};

print(process("hello")); // "String: hello"
print(process("")); // "Empty string"
print(process(-5)); // "Negative number"
print(process(0)); // "Zero"
print(process(42)); // "Positive number: 42"
print(process(true)); // "Boolean true"
```
