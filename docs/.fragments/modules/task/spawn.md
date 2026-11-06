```tyrus
import { NetError, resolve } from "tyrus:net";
import { spawn } from "tyrus:task";

async function lookup(host: string): Promise<string[]> {
    try {
        const addresses: string[] = await resolve(host, 2_000);
        return addresses;
    } catch (error: NetError) {
        return [];
    }
}

async function main(): Promise<void> {
    // Two independent, real network round trips, each one could take
    // hundreds of milliseconds. Spawning both means the second lookup starts
    // before the first one has replied, instead of waiting its turn.
    const first = spawn(async (): Promise<string[]> => {
        const addresses: string[] = await lookup("example.com");
        return addresses;
    });
    const second = spawn(async (): Promise<string[]> => {
        const addresses: string[] = await lookup("localhost");
        return addresses;
    });

    const exampleAddresses: string[] = await first.join();
    const localAddresses: string[] = await second.join();

    print(exampleAddresses.length > 0);
    print(localAddresses.length > 0);
}
```
