```tyrus{7}
interface User { name: string; }
interface OtherUser { name: string; }

const main = (): void => {
    const user: User = { name: "Ada" };
    const other: OtherUser = { name: "Ada" };
    const same: boolean = user.equals(other); // [!code error]
    print(same);
};
```

```text
error[E0308]: `OtherUser` is not assignable to `User`: tyrus interfaces are nominal, so distinct interface names are separate types even with the same fields; build a `User` value directly (its fields are available on `OtherUser`), or use one shared interface type for both
at /src/main.tyr:7:27
    const same: boolean = user.equals(other);
                          ^^^^^^^^^^^^^^^^^^
help: change the value or the annotation so their types match; convert numbers explicitly with `value as <type>`
```
