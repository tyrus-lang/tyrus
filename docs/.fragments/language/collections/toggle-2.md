```tyrus
interface Pet {
    name: string;
    age: i32;
}

const pets: Pet[] = [
    { name: "rex", age: 2 },
    { name: "mia", age: 5 },
];
const adopted: Pet[] = pets.toggle({ name: "rex", age: 99 }, "name");
print(adopted.length); // 1
```
