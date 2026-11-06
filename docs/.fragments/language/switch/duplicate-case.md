```tyrus
switch (role) {
    case "admin": {
        print("first");
    }
    case "admin": { // [!code error]
        print("duplicate");
    }
}
```
