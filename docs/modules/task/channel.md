---
title: channel
description: Create a typed channel and move values between concurrent tasks through its Sender and Receiver endpoints.
---

# channel

`channel` creates a typed channel and returns its two owned endpoints as a tuple.

<SignatureBlock label="Signature">channel&lt;T&gt;(capacity?: usize): [Sender&lt;T&gt;, Receiver&lt;T&gt;]</SignatureBlock>

`capacity` is optional: omit it for an unbounded channel, or pass a `usize` to bound the number of buffered values, `Sender<T>.send`/`trySend` wait for (or reject on) room once a bounded channel is full. `Sender<T>.send(value: T): Promise<void>` waits for room and delivers `value`, rejecting with `channel is closed` once every `Receiver<T>` has been dropped; `Sender<T>.trySend(value: T): boolean` never waits, returning `true` once accepted or `false` if the channel is full or closed. `Receiver<T>.recv(): Promise<T?>` waits for the next message, resolving to `None` once every `Sender<T>` has been dropped and nothing remains buffered; `Receiver<T>.tryRecv(): T?` never waits, returning the next buffered message or `None` if there isn't one yet. `Receiver<T>.close(): void` closes the channel, `send`/`trySend` start failing, while `recv`/`tryRecv` still drain whatever was already buffered before settling to `None`. Like `Promise<T>` and `JoinHandle<T>`, `Sender<T>`/`Receiver<T>` are native types and never need to be imported.

Channel payloads are restricted to the same subset `Promise<T>` itself supports: `string`, arrays of `Copy` elements, and `Copy` scalars, not arbitrary interfaces.

A `Sender<T>`/`Receiver<T>` moved into a `spawn` task is consumed there, the same way any other owned value is: capturing one twice, say, into two separate tasks, is a compile-time move error, so a single producer or consumer per endpoint is the shape to reach for. The example below moves `tx` into one spawned worker that streams a result back for each host as soon as it resolves, letting `main` process results as they arrive instead of waiting for every lookup to finish the way `Promise.all` would, see the Playground page under Tooling for why this example does not run there:

<!-- @include: ../../.fragments/modules/task/channel.md -->

A capacity of `0` is a compile error (`E3035`): a channel with no room holds nothing, so every `send` would wait for a `recv` that has nowhere to take the value from. Omit the argument to use the default capacity.
