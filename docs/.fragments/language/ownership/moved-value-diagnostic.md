```text
error[E0382]: use of moved value: `greeting`
at /src/main.tyr:4:7
print(greeting);
      ^^^^^^^^
at /src/main.tyr:2:22
const copy: string = greeting;
                     ^^^^^^^^ value moved here; borrow `greeting` instead if it must remain usable
help: borrow `greeting` at the move site if it must remain usable later
help: for example: `const copy: &string = &greeting;`
```
