```tyrus
import { rand } from "tyrus:crypto";

const bytes: u8[] = rand.randomBytes(32);
const num: i64 = rand.randomInt(1, 100);
const id: string = rand.uuidv7();
```
