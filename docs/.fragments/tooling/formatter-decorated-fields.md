```tyrus
interface ProductFilter {
    @notEmpty()
    query: string;

    @min(5)
    pageSize: i32;
}
```
