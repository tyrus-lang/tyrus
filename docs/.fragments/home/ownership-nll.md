```tyrus
const greeting: string = "hello";
const copy: string = greeting;

print(copy); // hello
print(greeting); // error: `greeting` was moved to `copy` [!code error]
```
