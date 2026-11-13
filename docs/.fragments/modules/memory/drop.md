```tyrus
import { drop } from "tyrus:memory";

function discard<T>(value: T[]): void {
    drop(value);
}

function main(): void {
    let bytes: u8[] = [1, 2, 3];
    print(bytes); // [1, 2, 3]
    discard(bytes);

    let entries: Map<string, i32> = Map<string, i32>();
    entries.set("sessions", 1);
    drop(entries);
}
```

::: danger error
```tyrus
import { drop } from "tyrus:memory";

function main(): void {
    let bytes: u8[] = [1, 2, 3];
    drop(bytes);
    print(bytes); // [!code error]
}
```

```text
error[E0382]: use of dropped value: `bytes`
at /src/main.tyr:6:11
    print(bytes);
          ^^^^^
at /src/main.tyr:5:10
    drop(bytes);
         ^^^^^ value dropped here; `bytes` cannot be used afterward
help: remove the later use of `bytes`, or move it before `drop(bytes)`
```
:::
