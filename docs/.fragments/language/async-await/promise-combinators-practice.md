```tyrus
import { sleepAsync } from "tyrus:task";

interface TimeoutError extends Error {
    code: string;
}

async function fetchWithTimeout(url: string, timeoutMs: u64): Promise<string> throws TimeoutError {
    const fetchOp = async (): Promise<string> => {
        // Simulated fetch
        return `data from ${url}`;
    };

    const timeoutOp = async (): Promise<string> throws TimeoutError => {
        await sleepAsync(timeoutMs);
        const error: TimeoutError = { message: `timeout after ${timeoutMs}ms`, code: "ERR_TIMEOUT" };
        throw error;
    };

    return await Promise.race<string>([fetchOp(), timeoutOp()]);
}

async function main(): Promise<void> {
    try {
        const data: string = await fetchWithTimeout("api.example.com", 5000);
        print(data); // "data from api.example.com"
    } catch (error) {
        printError(error.message);
    }
}
```
