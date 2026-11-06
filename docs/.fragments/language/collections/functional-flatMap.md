```tyrus
const sentences: string[] = ["hello world", "goodbye moon"];
const words: string[] = sentences.flatMap(sentence => sentence.split(" "));
// ["hello", "world", "goodbye", "moon"]
```

::: danger error
```tyrus
const sentences: string[] = ["hello world", "goodbye moon"];
const words: string[] = sentences.flatMap(sentence => sentence); // [!code error]
```

```text
error[E0308]: `flatMap` callback must return an array, found `string`
at /src/main.tyr:2:25
const words: string[] = sentences.flatMap(sentence => sentence);
                        ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
help: change the value or the annotation so their types match; convert numbers explicitly with `value as <type>`
```
:::
