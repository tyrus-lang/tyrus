---
title: networkInterfaces
description: Liste as interfaces de rede ativas e seus endereços IP locais através do módulo nativo tyrus:os.
---

# networkInterfaces

`networkInterfaces` devolve um `NetworkInterface` para cada endereço IP local encontrado em cada placa de rede ativa, uma placa com endereço IPv4 e IPv6 contribui com duas entradas. Falhas ao enumerar retornam `Err(OsError)` com o código `ERR_OS_NETWORK_INTERFACES_FAILED`.

<SignatureBlock label="Assinatura">networkInterfaces(): Result&lt;NetworkInterface[], OsError&gt;</SignatureBlock>



<!-- @include: ../../../.fragments/modules/os/usage-network-interfaces.md -->
### Campos de NetworkInterface

| Campo | Tipo | Significado |
|-------|------|-------------|
| `name` | `string` | O nome da placa (ex.: `"eth0"`, `"en0"`, `"Wi-Fi"`). |
| `address` | `string` | O endereço IP local. |
| `netmask` | `string` | A máscara de sub-rede de `address`. |
| `family` | `string` | `"IPv4"` ou `"IPv6"`. |
