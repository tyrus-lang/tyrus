```tyrus
const html: string = "<p>Hello <b>World</b></p>";
const text: string = html.stripHtml(); // Hello World

const link: string = "<a href=\"url\">click here</a>";
const plain: string = link.stripHtml(); // click here
```

::: danger error
```tyrus
const flag: boolean = true;
const text: string = flag.stripHtml(); // [!code error]
```

```text
error[E0599]: no method named `stripHtml` for type `boolean`
at /src/main.tyr:2:22
const text: string = flag.stripHtml();
                     ^^^^^^^^^^^^^^^^
help: check the member name and the receiver type; hover the value to see its type
```
:::
