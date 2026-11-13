```tyrus
interface Coordinate {
    x: i32;
    y: i32;
}

const points: Coordinate[] = [
    { x: 3, y: 9 },
    { x: 1, y: 4 },
    { x: 2, y: 7 },
];
const byX: Coordinate[] = points.sortedBy("x"); // [{ x: 1, y: 4 }, { x: 2, y: 7 }, { x: 3, y: 9 }]
const byYDesc: Coordinate[] = points.sortedBy("y", "desc"); // [{ x: 3, y: 9 }, { x: 2, y: 7 }, { x: 1, y: 4 }]
```
