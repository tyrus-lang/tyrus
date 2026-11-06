```tyrus
import { RedBlackTree } from "tyrus:algorithm";

const main = (): void => {
    const t = RedBlackTree<i32>();
    print(t.isEmpty());  // true

    t.insert(5);
    t.insert(3);
    t.insert(8);
    t.insert(3);  // duplicate, ignored

    print(t.len());        // 3
    print(t.contains(3));  // true
    print(t.contains(4));  // false
};
```
