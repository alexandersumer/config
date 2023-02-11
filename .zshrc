export EDITOR=vim

 export PYENV_ROOT="$HOME/.pyenv"
 command -v pyenv >/dev/null || export PATH="$PYENV_ROOT/bin:$PATH"
 eval "$(pyenv init -)"

 export ZSH="$HOME/.oh-my-zsh"
 export PATH=/usr/local/opt/util-linux/bin:$PATH
 export TF_VAR_signalfx_auth_token="bcbbe3d9-2bdb-4c60-b6ae-cc9e627be066"

 ZSH_THEME=""

 autoload -U promptinit; promptinit
 prompt pure

 plugins=(git sdk colored-man-pages colorize pip python brew macos zsh-autosuggestions zsh-syntax-highlighting)

 source $ZSH/oh-my-zsh.sh

 JAVA8="8.0.332-tem"
 JAVA11="11.0.15-tem"
 JAVA17="17.0.3-tem"

 export SDKMAN_DIR="$HOME/.sdkman"
 [[ -s "$HOME/.sdkman/bin/sdkman-init.sh" ]] && source "$HOME/.sdkman/bin/sdkman-init.sh"
 alias java8="sdk use java $JAVA8"
 alias java11="sdk use java $JAVA11"
 alias java17="sdk use java $JAVA17"

 alias src="cd $HOME/src"
 alias jira="cd $HOME/src/jira && sdk use java $JAVA8"
 alias jis="cd $HOME/src/jira-issue-service && sdk use java $JAVA11"
 alias jii="cd $HOME/src/jira-issue-investigation && sdk use java $JAVA17"
 alias tag="cd $HOME/src/tagging-service && sdk use java $JAVA17"
 alias sfx="pyenv shell sfm && cd $HOME/src/jis-signalfx-resources"
 alias proto="cd $HOME/src/jira-issue-protos && sdk use java $JAVA11"
 alias agg="cd $HOME/src/graphql-central-schema && sdk use java $JAVA11"
