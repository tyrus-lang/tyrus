```tyrus
interface Ticket {
    title: string;
    completed: boolean;
    category: string;
}

const tickets: Ticket[] = [
    { title: "Plan", completed: true, category: "work" },
    { title: "Read", completed: false, category: "learning" },
    { title: "Review", completed: true, category: "work" },
    { title: "Archive", completed: false, category: "personal" }
];

const [completed, pending] = tickets.partition("completed");
print(completed); // [{ title: "Plan", completed: true, category: "work" }, { title: "Review", completed: true, category: "work" }]
print(pending); // [{ title: "Read", completed: false, category: "learning" }, { title: "Archive", completed: false, category: "personal" }]

const [focused, deferred] = tickets.partition("category", "work", "learning");
print(focused); // [{ title: "Plan", completed: true, category: "work" }, { title: "Read", completed: false, category: "learning" }, { title: "Review", completed: true, category: "work" }]
print(deferred); // [{ title: "Archive", completed: false, category: "personal" }]
```
