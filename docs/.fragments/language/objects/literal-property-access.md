```tyrus
interface Person {
    name: string;
    age: i32;
}

let person: Person = { name: "Ada", age: 36 };

print(person.name);
print(person["name"]);

person["age"] = 42;
```
