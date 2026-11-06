```tyrus
import { spawn, joinAll, sleepAsync } from "tyrus:task";

async function fetchShard(id: i32, delayMs: u64): Promise<i32> {
    // Stands in for a real per-shard request; `sleepAsync` simulates each
    // shard's own latency without needing real I/O for the example.
    await sleepAsync(delayMs);
    return id * 10;
}

async function main(): Promise<void> {
    const shardA = spawn(async (): Promise<i32> => await fetchShard(1, 20));
    const shardB = spawn(async (): Promise<i32> => await fetchShard(2, 10));
    // `joinAll` waits for every shard and keeps the results in the same
    // order the handles were given, regardless of which finishes first.
    const shards: i32[] = await joinAll([shardA, shardB]);
    print(shards[0] + shards[1]);
}
```
