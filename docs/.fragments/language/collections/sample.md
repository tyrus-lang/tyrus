```tyrus
const teams = ["Arsenal", "Barcelona", "Bayern Munich", "Inter Milan"];
const distinct = teams.sample(2);
const independent = teams.sample(2, { repeat: true });
print(distinct); // ["Bayern Munich", "Barcelona"]
print(independent); // ["Inter Milan", "Inter Milan"]
```
