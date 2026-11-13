```tyrus
function print(label: string): void { // [!code error]
    console.log("custom:", label);
}
```
```text
error[E0003]: `print` is an ambient builtin in tyrus and cannot be redeclared
at /src/main.tyr:1:10
function print(label: string): void {
         ^^^^^
help: rename `print` to an identifier that does not collide with a builtin function
```
