```tyrus
import { CryptoError, digest } from "tyrus:crypto";
import { utf8 } from "tyrus:encoding";

const data: u8[] = utf8.encode("hello world");
const hash: Result<u8[], CryptoError> = digest.sha256(data);
match (hash) { Ok(bytes) => { print(bytes.length); } Err(error) => { printError(error.code); } }
```
