| Header | Value | What it does |
| --- | --- | --- |
| `x-content-type-options` | `nosniff` | Stops content-type sniffing, which is what turns a JSON response into executed script. |
| `referrer-policy` | `no-referrer` | Keeps request paths out of the `Referer` sent to whatever the response links to. |
| `x-frame-options` | `DENY` | Refuses framing, so the response cannot be the invisible layer of a clickjacking page. |
| `cross-origin-resource-policy` | `same-origin` | Refuses embedding by another origin: the modern form of the same refusal. |
