export ZSH="/Users/super/.oh-my-zsh"

ZSH_THEME="robbyrussell"
 
plugins=(git zsh-autosuggestions)
 
source $ZSH/oh-my-zsh.sh
 
alias py='python3'
 
alias c='clear'
alias ..='cd ..'
alias ...='cd ..; cd ..'
alias ....='cd ..; cd ..; cd ..'
 
alias proj='cd ~/Dropbox/proj'
alias curr='cd ~/Dropbox/curr'
 
alias add='git add .'
alias com='git commit -m'
alias qc='git add . && git commit -m "checkpoint"'
alias gpu='git push'
alias bra='git branch'
alias new='git checkout -b'
alias rhd='git checkout develop && git fetch && git reset --hard origin/develop'
alias rhm='git checkout master && git fetch && git reset --hard origin/master'
