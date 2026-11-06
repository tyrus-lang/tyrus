```tyrus
import { createRouter, HttpError, HttpReply, HttpRouter, HttpServer, HttpStatus } from "tyrus:http";

interface TodoItem {
    id: i64;
    title: string;
    done: boolean;
}

interface TodoListResponse {
    items: TodoItem[];
    total: i32;
}

interface CreateTodoInput {
    @minLength(1)
    title: string;
}

const app: HttpRouter = createRouter();
let nextId: i64 = 1;
const todos: TodoItem[] = [];

app.get("/todos", {
    response: { 200: TodoListResponse },
    handler: (request, reply) => {
        reply.status(HttpStatus.Ok).send({
            items: todos,
            total: todos.length as i32,
        });
    },
});

app.post("/todos", {
    response: { 201: TodoItem },
    handler: (request, reply) => {
        const input: CreateTodoInput = request.body;
        const todo: TodoItem = { id: nextId, title: input.title, done: false };
        nextId += 1;
        todos.push(todo);
        reply.status(HttpStatus.Created).send(todo);
    },
});

const main = (): void => {
    const server = app.listen(3000) onfail {
        printError(`${server.code}: ${server.message}`);
    };
    print(`Server listening on port ${server.port()}`);
};
```
