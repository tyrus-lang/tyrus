```tyrus
interface Product {
    id: i32;
    name: string;
    price: f64;
    inStock: boolean;
}

type NonNumeric = OmitByType<Product, i32>;
const listing: NonNumeric = { name: "Widget", price: 29.99, inStock: true };
```
