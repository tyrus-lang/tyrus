```tyrus
import { sleep } from "tyrus:task";

function main(): void {
    sleep(-1); // [!code error]
}
```

```text
error[E0308]: type mismatch for `argument 1 of `sleep``: expected `u64`, found `negative integer literal`
at /src/main.tyr:4:5
    sleep(-1);
    ^^^^^
help: change the value or the annotation so their types match; convert numbers explicitly with `value as <type>`
error[E0706]: integer literal `-1` does not fit in `u64`, which holds values from 0 to 18446744073709551615
at /src/main.tyr:4:11
    sleep(-1);
          ^^
help: the narrowest type that holds this value is `i8`; widen the type or change the value to fit
```
