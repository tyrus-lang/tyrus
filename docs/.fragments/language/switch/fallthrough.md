```tyrus
switch (role) {
    case "admin": {
        print("privileged");
    }
    case "operator": {
        print("authenticated");
        break;
    }
    default: {
        print("anonymous");
    }
}
```
