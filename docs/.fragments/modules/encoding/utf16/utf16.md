```tyrus
import { EncodingError, utf16 } from "tyrus:encoding";

const units: u16[] = utf16.encode("A😀");
const littleEndian: u8[] = utf16.encodeLE("A😀");
const bigEndian: u8[] = utf16.encodeBE("A😀");
const malformed: u16[] = [55296];

try {
    print(utf16.decode(units)); // A😀
    print(utf16.decodeLE(littleEndian)); // A😀
    print(utf16.decodeBE(bigEndian)); // A😀
} catch (error: EncodingError) {
    print(error.code);
}
print(utf16.decodeLossy(malformed)); // �
```
