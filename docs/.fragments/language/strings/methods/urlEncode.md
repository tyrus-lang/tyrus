```tyrus
const query: string = "hello world & friends";
const encoded: string = query.urlEncode(); // "hello%20world%20%26%20friends"

const path: string = "a/b?c=d";
const safe: string = path.urlEncode(); // "a%2Fb%3Fc%3Dd"
```

::: danger error
```tyrus
const flag: boolean = true;
const encoded: string = flag.urlEncode(); // [!code error]
```

```text
error[E0599]: no method named `urlEncode` for type `boolean`
at /src/main.tyr:2:25
const encoded: string = flag.urlEncode();
                        ^^^^^^^^^^^^^^^^
help: check the member name and the receiver type; hover the value to see its type
```
:::
