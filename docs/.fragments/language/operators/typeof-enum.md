```tyrus
enum Status: string {
    Active = "active",
    Inactive = "inactive",
}

const current: Status = Status.Active;
const kind: string = typeof current; // "string", the enum's backing type
```
