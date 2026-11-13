```tyrus
import { kmpSearch } from "tyrus:algorithm";

const matches: usize[] = kmpSearch("ababa", "aba");
print(matches[0], matches[1]); // 0 2
```
