```tyrus
import { spawn, race, sleepAsync } from "tyrus:task";

async function fetchReplica(id: i32, delayMs: u64): Promise<i32> {
    await sleepAsync(delayMs);
    return id;
}

async function main(): Promise<void> {
    // Two redundant replicas answer the same request; `race` returns
    // whichever answers first and cancels the other one, no need to wait
    // for a reply nothing will use.
    const replicaA = spawn(async (): Promise<i32> => await fetchReplica(1, 50));
    const replicaB = spawn(async (): Promise<i32> => await fetchReplica(2, 5));
    const winner: i32 = await race([replicaA, replicaB]);
    print(winner);
}
```
