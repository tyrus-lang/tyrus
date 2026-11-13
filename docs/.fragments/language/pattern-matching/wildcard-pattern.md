```tyrus
const status: i32 = 200;

match (status) {
    (200) => {
        print("OK");
    }
    404 => {
        print("Not Found");
    }
    (500) => {
        print("Server Error");
    }
    _ => {
        print(`Other status: ${status}`);
    }
}
// "OK"
```
