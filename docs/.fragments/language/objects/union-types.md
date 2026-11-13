```tyrus
type Shape =
    | { kind: "circle"; radius: f64 }
    | { kind: "square"; size: f64 }
    | { kind: "rect"; width: f64; height: f64 };

const printShape = (shape: Shape): void => {
    if (shape.kind === "circle") {
        print(shape.radius); // radius value
    } else if (shape.kind === "square") {
        print(shape.size); // size value
    } else {
        print(shape.width); // width value
        print(shape.height); // height value
    }
};
```
