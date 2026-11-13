```tyrus
import { EncodingError, base64Url } from "tyrus:encoding";

const bytes: u8[] = [0xfb, 0xff, 0x00, 0xde, 0xad];
const encoded: string = base64Url.encode(bytes);
print(encoded); // -_8A3q0

try {
    const decoded: u8[] = base64Url.decode(encoded);
    print(decoded); // [251, 255, 0, 222, 173]
} catch (error: EncodingError) {
    print(error.code);
}
```
