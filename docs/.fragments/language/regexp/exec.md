```tyrus
const email: Regex = /^([a-z]+)@([a-z]+)\.([a-z]{2,})$/;
const result: string[]? = email.exec("hello@example.com");

if (result) {
    print(result[0]); // "hello@example.com"
    print(result[1]); // "hello"
}
```
