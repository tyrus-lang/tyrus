```tyrus
import { SortedMap } from "tyrus:algorithm";

const main = (): void => {
    const m = SortedMap<string, i32>();
    m.set("banana", 2);
    m.set("apple", 1);
    m.set("cherry", 3);

    print(m.min() ?? "none");      // "apple"
    print(m.max() ?? "none");      // "cherry"
    print(m.get("apple") ?? 0);    // 1
};
```
