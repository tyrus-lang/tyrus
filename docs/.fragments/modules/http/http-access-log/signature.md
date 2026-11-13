```tyrus
interface HttpAccessLog {
    readonly requestId: string;
    readonly method: string;
    readonly path: string;
    readonly status: HttpStatus;
    readonly duration: Duration;
}
```
