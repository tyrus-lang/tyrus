```tyrus
interface Snapshot {
    count: i32;
    label: string;
    tags: string[];
    note?: string;
}

async function refresh(): Promise<void> {
    let snapshot: Snapshot = { count: 1, label: "first", tags: ["static"] };
    await Promise.resolve();
    snapshot.count = 2;
    snapshot.label = snapshot.label.toUpperCase();
    snapshot.tags = "direct,field".split(",");
    snapshot.tags.push("tail");
    snapshot.note = snapshot.label;
    await Promise.resolve();
    snapshot = {
        count: 3,
        label: "second".toUpperCase(),
        tags: "owned,array".split(","),
        note: "present".toUpperCase(),
    };
    await Promise.resolve();
    print(snapshot.label);
}
```
