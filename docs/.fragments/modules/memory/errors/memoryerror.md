```tyrus
import { MemoryError, zeroize } from "tyrus:memory";

function main(): void {
    const literal = "api-key";
    match (zeroize(literal)) {
        Ok(_) => { print("erased"); }
        Err(error) => { print(error.code); // ERR_MEMORY_NOT_OWNED
        }
    }
}
```
