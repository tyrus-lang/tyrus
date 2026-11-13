```tyrus
const title: string = "Como criar uma API em Tyrus!";
const slug: string = title.toSlugCase(); // "como-criar-uma-api-em-tyrus"

const greeting: string = "Hällo Wörld";
const clean: string = greeting.toSlugCase(); // "hallo-world"
```

::: danger error
```tyrus
const flag: boolean = true;
const slug: string = flag.toSlugCase(); // [!code error]
```

```text
error[E0599]: no method named `toSlugCase` for type `boolean`
at /src/main.tyr:2:22
const slug: string = flag.toSlugCase();
                     ^^^^^^^^^^^^^^^^^
help: check the member name and the receiver type; hover the value to see its type
```
:::
