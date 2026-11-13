```tyrus
interface ValueError extends Error {
    code: string;
}

async function validate(value: i32): Promise<i32> throws ValueError {
    if (value < 0) {
        const error: ValueError = { message: "value must be non-negative", code: "ERR_VALUE_MUST_BE_NON_NEGATIVE" };
        throw error;
    }
    return value;
}

async function main(): Promise<void> {
    try {
        const result: i32 = await validate(-1);
    } catch (error: Error) {
        printError(error.message); // "value must be non-negative"
    }
}
```
