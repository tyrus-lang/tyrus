```tyrus
const encoded: string = "hello%20world";
const decoded: string = encoded.urlDecode(); // "hello world"

const special: string = "TitenQ%20%26%20friends";
const result: string = special.urlDecode(); // "TitenQ & friends"
```

::: danger error
```tyrus
const flag: boolean = true;
const decoded: string = flag.urlDecode(); // [!code error]
```

```text
error[E0599]: no method named `urlDecode` for type `boolean`
at /src/main.tyr:2:25
const decoded: string = flag.urlDecode();
                        ^^^^^^^^^^^^^^^^
help: check the member name and the receiver type; hover the value to see its type
```
:::
