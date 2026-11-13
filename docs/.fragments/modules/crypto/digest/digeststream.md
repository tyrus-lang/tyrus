```tyrus
import { digest } from "tyrus:crypto";
import { utf8 } from "tyrus:encoding";

const stream: DigestStream = digest.create("sha256");
stream.update(utf8.encode("hello "));
stream.update(utf8.encode("world"));
const hash: u8[] = stream.digest();
```
