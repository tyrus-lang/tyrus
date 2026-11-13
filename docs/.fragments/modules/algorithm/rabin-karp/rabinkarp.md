```tyrus
import { rabinKarp } from "tyrus:algorithm";

const matches: usize[] = rabinKarp("ababa", "ba");
print(matches[0], matches[1]); // 1 3
```
