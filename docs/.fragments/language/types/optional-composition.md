```tyrus
const findFirst = <T>(items: T[], predicate: (item: T) => boolean): T? => {
    for (const item of items) {
        if (predicate(item)) {
            return item;
        }
    }

    return;
};

const numbers: i32[] = [1, 2, 3, 4, 5];
const first: i32? = findFirst(numbers, n => n > 3);

print(first ?? -1); // 4

interface Company {
    ceo?: User;
}

const company: Company = {};
const ceoName: string? = company.ceo?.name;

print(ceoName ?? "no CEO"); // no CEO
```
