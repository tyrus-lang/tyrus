```tyrus
interface SummaryUser {
    id: i32;
}

interface CompleteUser {
    id: i32;
    name: string;
}

const complete: CompleteUser = { id: 99, name: "TitenQ" };
const summary: SummaryUser = { id: complete.id };
```
