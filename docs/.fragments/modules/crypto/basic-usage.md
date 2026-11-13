```tyrus
import { CryptoError, digest, hmac, timingSafeEqual } from "tyrus:crypto";
import { utf8 } from "tyrus:encoding";

const data: u8[] = utf8.encode("hello world");
const hash: Result<u8[], CryptoError> = digest.sha256(data);
match (hash) { Ok(bytes) => { print(bytes.length); } Err(error) => { printError(error.code); } }
const mac: Result<u8[], CryptoError> = hmac.sha256(data, utf8.encode("webhook-secret"));
match (mac) { Ok(bytes) => { print(bytes.length); } Err(error) => { printError(error.code); } }
```
