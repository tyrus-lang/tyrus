```tyrus
interface User {
    age: u32;
}

interface UserMethods at User {
    isAdult(user: User): boolean {
        return user.age >= 18;
    }
}

const user: User = { age: 21 };
const adult: boolean = user.isAdult(); // true
```

```tyrus
// model.tyr
export interface User { age: u32; }
export const ageText = (user: User): string => user.age.toString();
export interface UserMethods at User { ageText; }

// main.tyr
import { User, UserMethods } from "./model";
const user: User = { age: 21 };
const text: string = user.ageText(); // "21"
```

```tyrus
interface NumberMethods at i32 {
    isPositive(value: i32): boolean {
        return value > 0;
    }
}

interface StringMethods at string {
    isBlank(value: string): boolean {
        return value.length === 0;
    }
}

interface BooleanMethods at boolean {
    inverted(value: boolean): boolean {
        return !value;
    }
}

interface CharMethods at char {
    isAscii(value: char): boolean {
        return value.isAlphabetic();
    }
}

const number: i32 = 4;
const text: string = "";
const enabled: boolean = true;
const letter: char = 'A';
const positive: boolean = number.isPositive(); // true
const blank: boolean = text.isBlank(); // true
const inverted: boolean = enabled.inverted(); // false
const ascii: boolean = letter.isAscii(); // true
```

```tyrus
interface PositiveNumbers at i32, u32 {
    isPositive(value): boolean {
        return value > 0;
    }
}

const signed: i32 = 7;
const unsigned: u32 = 9;
const signedPositive: boolean = signed.isPositive(); // true
const unsignedPositive: boolean = unsigned.isPositive(); // true
```
