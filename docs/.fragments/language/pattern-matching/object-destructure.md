```tyrus
interface Point {
    x: f64;
    y: f64;
}

const describe = (point: Point): string => {
    match (point) {
        ({ x: 0, y: 0 }) => {
            return "Origin";
        }
        ({ x: 0, y }) => {
            return `On Y-axis at ${y}`;
        }
        ({ x, y: 0 }) => {
            return `On X-axis at ${x}`;
        }
        ({ x, y }) => {
            return `Point at (${x}, ${y})`;
        }
    }
};

print(describe({ x: 0, y: 5 })); // "On Y-axis at 5"
print(describe({ x: 3, y: 0 })); // "On X-axis at 3"
print(describe({ x: 1, y: 2 })); // "Point at (1, 2)"
```
