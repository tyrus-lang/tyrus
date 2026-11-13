```tyrus
import { cipher, rand } from "tyrus:crypto";
import { utf8 } from "tyrus:encoding";

try {
    const key: u8[] = rand.randomBytes(32);
    const aad: u8[] = utf8.encode("record:42");
    const encrypted: u8[] = cipher.aes.encryptWithAad("sensitive data", key, aad);
    const decrypted: string = cipher.aes.decryptTextWithAad(encrypted, key, aad);
    print(decrypted);
} catch (error: Error) {
    printError(error.message);
}
```
