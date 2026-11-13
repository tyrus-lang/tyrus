```tyrus
const masked: string = "ada@example.com".replace(/@.+$/, "@example.test");
print(masked); // ada@example.test
```
