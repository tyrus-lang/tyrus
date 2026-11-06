```tyrus
import { NetError, resolve } from "tyrus:net";
import { Sender, channel, spawn } from "tyrus:task";

async function lookupAllInto(tx: Sender<string>, hosts: string[]): Promise<void> {
    for (let i: usize = 0; i < hosts.length; i = i + 1) {
        try {
            const addresses: string[] = await resolve(hosts[i], 2_000);
            await tx.send(`${hosts[i]}: ${addresses.length} address(es)`);
        } catch (error: NetError) {
            await tx.send(`${hosts[i]}: lookup failed`);
        }
    }
}

async function main(): Promise<void> {
    const [tx, rx] = channel<string>();
    // The worker streams each host's result back through the channel as
    // soon as it resolves, instead of making the caller wait for every
    // lookup to finish the way `Promise.all` would.
    const worker = spawn(async (): Promise<void> => await lookupAllInto(tx, ["example.com", "localhost"]));

    for (let i = 0; i < 2; i = i + 1) {
        const result = await rx.recv();
        print(result ?? "no result");
    }
    rx.close();
    await worker.join();
}
```
