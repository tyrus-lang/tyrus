```tyrus
import { CryptoError, hmac } from "tyrus:crypto";
import { utf8 } from "tyrus:encoding";

const data: u8[] = utf8.encode("message");
const key: u8[] = utf8.encode("secret-key");
const mac: Result<u8[], CryptoError> = hmac.sha256(data, key);
match (mac) { Ok(bytes) => { print(bytes.length); } Err(error) => { printError(error.code); } }
```
