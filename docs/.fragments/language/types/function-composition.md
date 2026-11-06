```tyrus
type Callback = (result: string) => void;
type AsyncOp = (url: string) => Promise<string>;
type Predicate<T> = (value: T) => boolean;

const process = (items: i32[], predicate: Predicate<i32>): i32[] => {
    return items.filter(predicate);
};

const isEven = (n: i32): boolean => n % 2 === 0;
const evens: i32[] = process([1, 2, 3, 4, 5, 6], isEven);
```
