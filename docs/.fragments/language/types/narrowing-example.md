```tyrus
function takesI32(value: i32): void {
    print(value);
}

const large: i64 = 9000;

takesI32(large); // [!code error]
takesI32(large as i32);
```
