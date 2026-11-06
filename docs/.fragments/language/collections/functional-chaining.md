```tyrus
interface Product {
    name: string;
    price: f64;
    inStock: boolean;
}

const products: Product[] = [
    { name: "Keyboard", price: 79.99, inStock: true },
    { name: "Mouse", price: 29.99, inStock: false },
    { name: "Monitor", price: 349.99, inStock: true },
    { name: "Headset", price: 59.99, inStock: true },
];

const availableNames: string[] = products
    .filter(product => product.inStock)
    .map(product => product.name);
// ["Keyboard", "Monitor", "Headset"]

const totalRevenue: f64 = products
    .filter(product => product.inStock)
    .reduce((sum, product) => sum + product.price, 0.0);
// 489.97
```