```tyrus
interface Score {
    name: string;
    points: i32;
}

let scores: Score[] = [
    { name: "Ada", points: 30 },
    { name: "Grace", points: 10 },
    { name: "Linus", points: 20 },
];
scores.sortBy("points"); // [{ name: "Grace", points: 10 }, { name: "Linus", points: 20 }, { name: "Ada", points: 30 }]
scores.sortBy("points", "desc"); // [{ name: "Ada", points: 30 }, { name: "Linus", points: 20 }, { name: "Grace", points: 10 }]
```
