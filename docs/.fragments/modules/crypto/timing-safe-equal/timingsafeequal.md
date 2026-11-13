```tyrus
import { timingSafeEqual } from "tyrus:crypto";
import { utf8 } from "tyrus:encoding";

const a: u8[] = utf8.encode("secret");
const b: u8[] = utf8.encode("secret");
const equal: boolean = timingSafeEqual(a, b);
```
