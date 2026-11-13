```tyrus
import { SortedSet } from "tyrus:algorithm";

const main = (): void => {
    const s = SortedSet<i32>();
    s.insert(3);
    s.insert(1);
    s.insert(2);
    s.insert(2);

    print(s.len());        // 3 (duplicates are ignored)
    print(s.contains(2));  // true
    print(s.min() ?? 0);   // 1
    print(s.max() ?? 0);   // 3

    s.remove(2);
    print(s.contains(2));  // false
    print(s.isEmpty());    // false

    s.clear();
    print(s.isEmpty());    // true
    print(s.len());        // 0
};
```
