```tyrus
interface Item {
    name: string;
    quantity: i32;
    price: f64;
}

const items: Item[] = [
    { name: "Widget", quantity: 3, price: 25.0 },
    { name: "Gadget", quantity: 1, price: 99.99 }
];

const totalQuantity = items.sumBy("quantity"); // 4 (i32)
const totalPrice = items.sumBy("price");       // 124.99 (f64)
```
