```tyrus
import { SortedSet } from "tyrus:algorithm";

const main = (): void => {
    const words = SortedSet<string>();
    words.insert("banana");
    words.insert("apple");
    words.insert("cherry");

    print(words.min() ?? "none");  // "apple"
    print(words.max() ?? "none");  // "cherry"
};
```
