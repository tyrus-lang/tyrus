```tyrus
const code: string = "<script>alert('hack')</script>";
const safe: string = code.escapeHtml(); // "&lt;script&gt;alert(&#39;hack&#39;)&lt;/script&gt;"
```

::: danger error
```tyrus
const flag: boolean = true;
const safe: string = flag.escapeHtml(); // [!code error]
```

```text
error[E0599]: no method named `escapeHtml` for type `boolean`
at /src/main.tyr:2:22
const safe: string = flag.escapeHtml();
                     ^^^^^^^^^^^^^^^^^
help: check the member name and the receiver type; hover the value to see its type
```
:::
