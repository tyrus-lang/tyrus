```tyrus
import { SortedMap } from "tyrus:algorithm";

const main = (): void => {
    const m = SortedMap<i32, string>();
    m.set(3, "c");
    m.set(1, "a");
    m.set(2, "b");
    m.set(2, "B");

    print(m.len());            // 3 (the entry for key 2 was replaced)
    print(m.has(2));           // true
    print(m.get(2) ?? "none"); // "B"
    print(m.min() ?? 0);       // 1
    print(m.max() ?? 0);       // 3

    m.delete(2);
    print(m.has(2));           // false
    print(m.isEmpty());        // false

    m.clear();
    print(m.isEmpty());        // true
    print(m.len());            // 0
};
```
