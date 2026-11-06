```tyrus
const sanitize = (input: string): string => input.trim().toLowerCase();
const slugify = (text: string): string => text.replace(" ", "-");
const validate = (slug: string): string => (slug.length > 0 ? slug : "untitled");

const title: string = "  Hello World  ";
const slug: string = title |> sanitize |> slugify |> validate;

print(slug); // "hello-world"
```
