```tyrus
const permissions: string[] = ["admin", "editor", "dev"];
print("admin" in permissions);
// true
print("guest" not in permissions);
// true

const status: i32 = 204;
print(status in 200..299);
// true
print(status not in 400..=599);
// true

const httpOk = 200..299;
print(httpOk.contains(204));
// true
```
