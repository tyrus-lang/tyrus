```tyrus
const name: string? = getName();
const label: string = name ?? "anonymous";

const retries: Optional<i32> = 3;
const remaining: i32 = retries ?? 0;
```
