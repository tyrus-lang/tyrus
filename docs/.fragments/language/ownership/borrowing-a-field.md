```tyrus
interface Basket {
    label: string;
    items: i32[];
}

const describe = (value: &string): void => {
    console.log(value);
};

const basket: Basket = { label: "fruit", items: [1, 2, 3] };

describe(&basket.label);
// fruit
```
