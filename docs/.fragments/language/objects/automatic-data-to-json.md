```tyrus
interface User { 
  name: string; 
  scores: i32[]; 
}

const user: User = { name: "Ada", scores: [10, 20] };
const json: string = user.toJson();
const pretty: string = user.toJson({
    pretty: true,
});

print(json); // {"name": "Ada","scores": [10,20]}
print(pretty);
/*
{
    "name": "Ada",
    "scores": [
        10,
        20
    ]
}
*/
```
