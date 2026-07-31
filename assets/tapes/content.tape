# Install content into an instance, then list what it carries.

Source common.tape
Output ../demo/cli-content.gif

Hide
Type "export PS1='\033[38;5;208m❯\033[0m '" Enter
Type "export HESTIA_HOME=~/.hestia-demo" Enter
Type "export PATH=$PWD/../../target/debug:$PATH" Enter
Type "clear" Enter
Show

Sleep 800ms
Type "hestia instance list"
Sleep 300ms
Enter
Sleep 3s

Type "hestia instance Skyblock mod add sodium"
Sleep 400ms
Enter
Sleep 8s

Type "hestia instance Skyblock mod add iris"
Sleep 400ms
Enter
Sleep 8s

Type "hestia instance Skyblock shader add complementary-reimagined"
Sleep 400ms
Enter
Sleep 8s

Type "hestia instance Skyblock mod list"
Sleep 300ms
Enter
Sleep 5s
