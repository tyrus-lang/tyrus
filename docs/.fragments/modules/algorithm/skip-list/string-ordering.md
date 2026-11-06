```tyrus
import { SkipList } from "tyrus:algorithm";

const main = (): void => {
    const words = SkipList<string>();
    words.insert("banana");
    words.insert("apple");
    words.insert("cherry");

    print(words.min() ?? "none");  // "apple"
    print(words.max() ?? "none");  // "cherry"
};
```
