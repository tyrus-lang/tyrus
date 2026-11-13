```tyrus
type Handler = (message: string) => void;

const handlers: Record<string, Handler[]> = {};

const on = (event: string, handler: Handler): void => {
    if (handlers[event] === undefined) {
        handlers[event] = [];
    }
    
    handlers[event].push(handler);
};

on("error", (msg) => printError(msg));
on("error", (msg) => logToFile(msg));
```
