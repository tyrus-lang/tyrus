```tyrus
import { freeMem } from "tyrus:os";

const free: u64 = freeMem();
print(`${free} bytes free`);
```
