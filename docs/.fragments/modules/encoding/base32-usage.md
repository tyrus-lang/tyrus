```tyrus
import { EncodingError, base32 } from "tyrus:encoding";

const bytes: u8[] = [0x66, 0x6f, 0x6f, 0x62, 0x61, 0x72];
const encoded: string = base32.encode(bytes);
print(encoded); // MZXW6YTBOI======

try {
    const decoded: u8[] = base32.decode(encoded);
    print(decoded); // [102, 111, 111, 98, 97, 114]
} catch (error: EncodingError) {
    print(error.code);
}
```
