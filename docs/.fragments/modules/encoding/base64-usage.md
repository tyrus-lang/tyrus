```tyrus
import { EncodingError, base64 } from "tyrus:encoding";

const bytes: u8[] = [0xde, 0xad, 0xbe, 0xef, 0x61, 0x62];
const encoded: string = base64.encode(bytes);
print(encoded); // 3q2+72Fi

try {
    const decoded: u8[] = base64.decode(encoded);
    print(decoded); // [222, 173, 190, 239, 97, 98]
} catch (error: EncodingError) {
    print(error.code);
}
```
