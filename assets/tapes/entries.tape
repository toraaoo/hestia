# Provision a server, start it, drive its console, then detach.

Source common.tape
Output ../demo/cli-server.gif

Hide
Type "export PS1='\033[38;5;208m❯\033[0m '" Enter
Type "export HESTIA_HOME=~/.hestia-demo" Enter
Type "export PATH=$PWD/../../target/debug:$PATH" Enter
Type "clear" Enter
Show

Sleep 800ms
Type "hestia server create vanilla 1.21.4 --name smp --eula"
Sleep 400ms
Enter
Sleep 11s

Type "hestia start smp"
Sleep 300ms
Enter
Sleep 12s

Type "say welcome to the server"
Sleep 400ms
Enter
Sleep 3s

Escape
Sleep 2s

Type "hestia server list"
Sleep 300ms
Enter
Sleep 4s
