```tyrus
import { PI } from "tyrus:math";

enum Shape {
    Circle(f64),
    Rect(f64, f64),
    Empty,
}

const area = (shape: Shape): f64 => {
    match (shape) {
        Shape.Circle(r) => PI * r * r;
        Shape.Rect(w, h) => w * h;
        Shape.Empty => 0.0;
    }
};

print(area(Shape.Circle(2.0))); // 12.566370614359172
print(`${area(Shape.Circle(2.0)):.2f}`); // 12.57
print(area(Shape.Rect(3.0, 4.0))); // 12.0
print(area(Shape.Empty)); // 0.0
```
