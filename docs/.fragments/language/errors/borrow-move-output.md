```text
error[E0382]: use of moved value: `name`
at /src/main.tyr:3:13
print(name);
            ^^^^
at /src/main.tyr:2:23
const moved: string = name;
                      ^^^^ value moved here; borrow `name` instead if it must remain usable
help: borrow `name` at the move site if it must remain usable later
help: for example: `const moved: &string = &name;`
```
