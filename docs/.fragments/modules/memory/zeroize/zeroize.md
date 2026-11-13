```tyrus
import { MemoryError, zeroize } from "tyrus:memory";

function eraseToken(): void throws MemoryError {
    let token: string = readToken();
    zeroize(token);
}

function main(): void {
    try {
        eraseToken();
    } catch (error: MemoryError) {
        print(error.code); // ERR_MEMORY_NOT_OWNED or ERR_MEMORY_SHARED_VALUE
    }
}
```
