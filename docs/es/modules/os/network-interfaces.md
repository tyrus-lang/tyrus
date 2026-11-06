---
title: networkInterfaces
description: Lista las interfaces de red activas y sus direcciones IP locales mediante el módulo nativo tyrus:os.
---

# networkInterfaces

`networkInterfaces` devuelve un `NetworkInterface` por cada dirección IP local encontrada en cada adaptador de red activo, un adaptador con dirección IPv4 e IPv6 aporta dos entradas. Los errores de enumeración devuelven `Err(OsError)` con el código `ERR_OS_NETWORK_INTERFACES_FAILED`.

<SignatureBlock label="Firma">networkInterfaces(): Result&lt;NetworkInterface[], OsError&gt;</SignatureBlock>



<!-- @include: ../../../.fragments/modules/os/usage-network-interfaces.md -->
### Campos de NetworkInterface

| Campo | Tipo | Significado |
|-------|------|-------------|
| `name` | `string` | El nombre del adaptador (ej.: `"eth0"`, `"en0"`, `"Wi-Fi"`). |
| `address` | `string` | La dirección IP local. |
| `netmask` | `string` | La máscara de subred de `address`. |
| `family` | `string` | `"IPv4"` o `"IPv6"`. |
