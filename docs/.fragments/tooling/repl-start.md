```text
tyrus repl
  ______                         ____  __________  __ 
 /_  __/_  _________  _______   / __ \/ ____/ __ \/ / 
  / / / / / / ___/ / / / ___/  / /_/ / __/ / /_/ / /  
 / / / /_/ / /  / /_/ (__  )  / _, _/ /___/ ____/ /___
/_/  \__, /_/   \__,_/____/  /_/ |_/_____/_/   /_____/
    /____/                                            

Type :help for available commands.

tyrus> :help
Commands (prefix with :):
  :help                              Show this help
  :clear                             Clear screen and session buffer
  :load <file>                       Load and evaluate a .tyr file
  :type <expr>                       Show the inferred type of an expression
  :doc <symbol>                      Show documentation for a symbol
  :edit                              Edit session buffer in external editor ($EDITOR)
  :save <name.tyr>                   Save session buffer to ~/.tyrus/repl/<name>.tyr
  :restore <name|file> [--merge|-m]  Restore a saved session (resets first; --merge keeps the current buffer)
  :delete <name>                     Delete a saved session from ~/.tyrus/repl
  :list                              List sessions saved via :save (~/.tyrus/repl)
  :reset                             Reset the entire session
  :exit                              Exit the REPL
  :quit                              Alias for :exit
  :q                                 Alias for :exit

Multi-line input is auto-detected by bracket matching.
tyrus> 
```
