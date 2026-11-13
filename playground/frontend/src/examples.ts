import { getEditor } from './editor.js';

interface Example {
    name: string;
    code: string;
}

const examples: Example[] = [
    {
        name: 'Hello World',
        code: `const main = (): void => {
    print("Hello, World!");
};
`,
    },
    {
        name: 'Fibonacci',
        code: `function fibonacci(n: i32): i32 {
    if (n <= 1) {
        return n;
    }

    return fibonacci(n - 1) + fibonacci(n - 2);
}

const main = (): void => {
    print(\`fibonacci(10) = \${fibonacci(10)}\`);
};
`,
    },
    {
        name: 'Match with Option',
        code: `// tyrus has no \`null\`. Absence of a \`T?\` is produced explicitly by a bare
// \`return;\`, and read back with \`match\` against \`None\` / a binding arm.
const findUser = (id: i32): string? => {
    if (id === 1) {
        return "Ada";
    }

    return; // absence
};

const describe = (id: i32): void => {
    const found = findUser(id);

    match (found) {
        value => { print(\`user \${id}: \${value}\`); }
        None => { print(\`user \${id}: not found\`); }
    }
};

const main = (): void => {
    describe(1);
    describe(2);
};
`
    },
    {
        name: 'Loops',
        code: `const main = (): void => {
    for (const i of 0..5) {
        print(\`Iteration \${i}\`);
    }
        
    let count = 0;
    
    while (count < 3) {
        print(\`Count \${count}\`);
        
        count++;
    }
};
`
    },
    {
        name: 'Enums',
        code: `enum Status {
    Success,
    Error,
    Pending,
}

const describeStatus = (status: Status): void => {
    if (status === Status.Success) {
        print("status: success");
    } else if (status === Status.Error) {
        print("status: error");
    } else {
        print("status: pending");
    }
};

const main = (): void => {
    describeStatus(Status.Success);
    describeStatus(Status.Error);
    describeStatus(Status.Pending);
};
`
    },
    {
        name: 'Pipe Operator',
        code: `const double = (n: i32): i32 => n * 2;
const increment = (n: i32): i32 => n + 1;
const square = (n: i32): i32 => n * n;
const divide = (a: f64, b: f64): f64 => a / b;

const main = (): void => {
    const result = 5 |> double |> increment |> square;
    print(\`result = \${ result }\`);
    
    const a = 100.0 |> divide(%, 4.0);
    print(\`100 / 4 = \${ a }\`);
    
    const b = 100.0 |> divide(4.0, %);
    print(\`4 / 100 = \${ b }\`);
};
`
    },
    {
        name: 'Arrays',
        code: `const main = (): void => {
    let numbers: i32[] = [10, 20, 30];
    
    for (const n of numbers) {
        print(n);
    }
    
    print("Length:", numbers.length);
};
`
    }
];

export function load(selectId: string): void {
    const select = document.getElementById(selectId) as HTMLSelectElement | null;
    if (!select) return;

    examples.forEach((example, index) => {
        const option = document.createElement('option');
        option.value = String(index);
        option.textContent = example.name;
        select.appendChild(option);
    });

    select.addEventListener('change', (e) => {
        const index = parseInt((e.target as HTMLSelectElement).value, 10);
        const editor = getEditor();
        const example = examples[index];
        if (editor && example) {
            editor.setValue(example.code);
        }
    });

    const first = examples[0];
    if (first) {
        const editor = getEditor();
        if (editor) {
            editor.setValue(first.code);
        }
    }
}

export function getAll(): Example[] {
    return examples;
}
