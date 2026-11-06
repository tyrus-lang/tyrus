```tyrus
const role: string = "admin";

switch (role) {
    case "admin": {
        print("full access");
        break;
    }
    case "operator": {
        print("limited access");
        break;
    }
    default: {
        print("read-only access");
    }
}
```
