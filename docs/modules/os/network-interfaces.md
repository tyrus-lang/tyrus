---
title: networkInterfaces
description: List active network interfaces and their local IP addresses through the native tyrus:os module.
---

# networkInterfaces

`networkInterfaces` returns one `NetworkInterface` per local IP address found on every active network adapter, an adapter with both an IPv4 and an IPv6 address contributes two entries. Enumeration failures return `Err(OsError)` with code `ERR_OS_NETWORK_INTERFACES_FAILED`.

<SignatureBlock label="Signature">networkInterfaces(): Result&lt;NetworkInterface[], OsError&gt;</SignatureBlock>



<!-- @include: ../../.fragments/modules/os/usage-network-interfaces.md -->
### NetworkInterface Fields

| Field | Type | Meaning |
|-------|------|---------|
| `name` | `string` | The adapter's name (e.g. `"eth0"`, `"en0"`, `"Wi-Fi"`). |
| `address` | `string` | The local IP address. |
| `netmask` | `string` | The subnet mask for `address`. |
| `family` | `string` | Either `"IPv4"` or `"IPv6"`. |
