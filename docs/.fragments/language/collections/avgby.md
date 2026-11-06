```tyrus
const items: Item[] = [
    { name: "Widget", quantity: 3, price: 25.0 },
    { name: "Gadget", quantity: 1, price: 99.99 }
];

const avgPrice = items.avgBy("price"); // 62.495 (f64?)
```
