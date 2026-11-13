```tyrus
type Shape =
    | { kind: "circle"; radius: f64 }
    | { kind: "rectangle"; width: f64; height: f64 };

const area = (shape: Shape): f64 => {
    match (shape) {
        ({ kind: "circle", radius }) => {
            return 3.14159 * radius * radius;
        }
        ({ kind: "rectangle", width, height }) => {
            return width * height;
        }
    }
};

const circle: Shape = { kind: "circle", radius: 5.0 };
print(area(circle)); // 78.53975
```
