```tyrus
readonly const PRIMES: i32[] = [2, 3, 5];

export function main(): void {
    let ps: i32[] = PRIMES; // [!code error]
    print(ps[0]);
}
```
