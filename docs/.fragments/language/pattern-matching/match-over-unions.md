```tyrus
type Result =
    | { tag: "success"; data: string }
    | { tag: "error"; code: i32; message: string };

const res: Result = { tag: "error", code: 404, message: "Not Found" };

match (res) {
    { tag: "success", data } => {
        print(`Data: ${data}`); // "Data: ..."
    }
    ({ tag: "error", code, message }) if (code === 404) => {
        print(`Error specific 404: ${message}`); // "Error specific 404: Not Found"
    }
    { tag: "error", message } => {
        print(`Other error: ${message}`); // "Other error: ..."
    }
    _ => {
        print("Unknown result");
    }
}
```
