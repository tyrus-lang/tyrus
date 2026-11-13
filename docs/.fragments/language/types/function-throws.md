```tyrus
const parseDigit = (text: string): i32 throws string => {
    if (text === "") {
        throw "empty input";
    }
    return 1;
};

const parseOrZero = (parse: (string) => i32 throws string, text: string): i32 => {
    try {
        return parse(text);
    } catch (error: string) {
        console.error(error);
        return 0;
    }
};

const main = (): void => {
    console.log(parseOrZero(parseDigit, ""));
};
```
