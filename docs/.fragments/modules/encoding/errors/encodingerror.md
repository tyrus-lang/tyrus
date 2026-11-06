```tyrus
import { EncodingError, utf8 } from "tyrus:encoding";

const invalid: u8[] = [255];

try {
    print(utf8.decode(invalid));
} catch (error: EncodingError) {
    print(error.code); // ERR_ENCODING_INVALID_UTF8
}
```
