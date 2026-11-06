```tyrus
interface Job {
    id: i32;
    state: { kind: "done"; value: i32 } | { kind: "failed"; reason: string };
}

const done: Job = { id: 1, state: { kind: "done", value: 42 } };
console.log(done.toJson());
// {"id": 1,"state": {"kind": "done","value": 42}}

const failed: Job = { id: 2, state: { kind: "failed", reason: "timeout" } };
console.log(failed.toJson());
// {"id": 2,"state": {"kind": "failed","reason": "timeout"}}
```
