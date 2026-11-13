```tyrus
import { EncodingError, utf8 } from "tyrus:encoding";

const bytes: u8[] = utf8.encode("hello 🌍");
const malformed: u8[] = [255];

try {
    print(utf8.decode(bytes)); // hello 🌍
} catch (error: EncodingError) {
    print(error.code);
}
print(utf8.decodeLossy(malformed)); // �
```
