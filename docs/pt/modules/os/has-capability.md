---
title: hasCapability
description: Verifica se uma capacidade do sistema está disponível através do módulo nativo tyrus:os.
---

# hasCapability

`hasCapability` verifica se uma `SystemCapability` está disponível neste sistema.

<SignatureBlock label="Assinatura">hasCapability(cap: SystemCapability): boolean</SignatureBlock>



<!-- @include: ../../../.fragments/modules/os/usage-has-capability.md -->
Responde "dá pra descobrir?"/"isso funcionaria?" sem disparar o efeito real, verificar `Notification` não mostra uma notificação, verificar `SleepPrevention` não inicia um processo inibidor.

### Membros de SystemCapability

| Membro | Significado |
|--------|-------------|
| `Notification` | Se uma notificação de desktop poderia plausivelmente ser mostrada. |
| `Clipboard` | Se a área de transferência do sistema pode ser alcançada. |
| `Battery` | Se esta máquina reporta pelo menos uma bateria. |
| `DarkMode` | Se a configuração de modo escuro do desktop é consultável. |
| `ScreenLock` | Se o estado de proteção de tela/tela de bloqueio é consultável (só Linux). |
| `SessionLock` | Se o estado de bloqueio de sessão é consultável (só Linux). |
| `SleepPrevention` | Se `preventSleep()`/`allowSleep()` realmente funcionariam. |
| `Fonts` | Se `systemFonts()` retornaria alguma coisa. |

::: tip Não é a mesma pergunta da função que espelha
`hasCapability(SystemCapability.ScreenLock)` pergunta se o estado da proteção de tela é consultável, não se a tela está bloqueada agora, isso é o próprio valor de retorno de [`isScreenLocked()`](./is-screen-locked.md). A mesma distinção vale para toda outra capacidade que espelha uma função real (`Battery` vs. [`battery()`](./battery.md), `DarkMode` vs. [`isDarkMode()`](./is-dark-mode.md), e assim por diante).
:::
