```tyrus
import { replace } from "tyrus:memory";

function main(): void {
    let source: string = "new value";
    let destination: string = "old value";
    const oldText: string = replace(destination, source);
    print(source); // [!code error]
    print(destination);
    print(oldText);
}
```

```text
error[E0382]: use of moved value: `source`
at /src/main.tyr:7:11
    print(source);
          ^^^^^^
at /src/main.tyr:6:50
    const oldText: string = replace(destination, source);
                                                 ^^^^^^ value moved here; borrow `source` instead if it must remain usable
help: borrow `source` at the move site if it must remain usable later
help: for example: `const oldText: &string = &source;`
```
