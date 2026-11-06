```tyrus
import { EncodingError, radix } from "tyrus:encoding";

print(radix.format(255, 16)); // ff
print(radix.format(255, 2)); // 11111111
print(radix.format(-255, 8)); // -377

try {
    print(radix.parseI64("-ff", 16)); // -255
    print(radix.parseI64("255", 10)); // 255
    radix.parseU64("-1", 10);
} catch (error: EncodingError) {
    print(error.code); // ERR_ENCODING_INVALID_RADIX
}
```
