```tyrus
import { BloomFilter } from "tyrus:algorithm";

const bf = BloomFilter(3, 256);
bf.insert("apple");
bf.insert("banana");
bf.insert("cherry");
bf.insert("date");

```
