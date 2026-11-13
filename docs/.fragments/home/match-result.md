```tyrus
type Result =
    | { tag: "success"; data: string }
    | { tag: "error"; code: i32; message: string };

const handle = (res: Result): void => {
    match (res) {
        ({ tag: "success", data }) => {
            print(`Data: ${data}`);
        }
        ({ tag: "error", code, message }) if (code === 404) => {
            print(`Not found: ${message}`);
        }
        ({ tag: "error", message }) => {
            print(`Error: ${message}`);
        }
    }
};
```
