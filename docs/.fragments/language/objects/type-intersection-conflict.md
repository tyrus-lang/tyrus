```tyrus
interface Metric {
    value: i32;
}

interface Label {
    value: string;
}

type Mixed = Metric & Label; // [!code error]
```
