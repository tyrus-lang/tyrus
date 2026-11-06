```tyrus
type Result = { tag: "success"; data: string } | { tag: "error"; code: i32 };

const handle = (result: Result): void => {
    match (result) {
        ({ tag: "success", data }) => {
            print(data); // "data value"
        }
        ({ tag: "error", code }) => {
            print(code); // error code
        }
    }
};
```
