parse_git_branch() {
    git branch 2> /dev/null | sed -e '/^[^*]/d' -e 's/* \(.*\)/ (\1)/'
}

export PS1="\[\e]0;\w\a\]\n\[\e[32m\]\u\[\e[0;90m\]:\[\e[1;33m\]\w\[\e[31m\]\[\e[0;90m\]\[\e[0m\]\[\e[32m\]\$(parse_git_branch)\[\e[0;90m\]\n\[\033[33m\] $ \[\033[00m\]"

alias c='clear'
alias ..='cd ..'
alias ...='cd ..; cd ..'
alias ....='cd ..; cd ..; cd ..'

alias proj='cd ~/Dropbox/proj'
alias open='cd ~/Dropbox/open'
alias sym='cd /c/Sympli'
alias docser='cd /c/Sympli/DocumentService'
alias tui='cd /c/Sympli/tickle-ui'
alias tapi='cd /c/Sympli/TickleApi'
alias lrsqld='cd /c/Sympli/LRS.QLD'
alias lrsvic='cd /c/Sympli/LRS.VIC'
alias lrsnsw='cd /c/Sympli/LRS.NSW'
alias lrscore='cd /c/Sympli/LRS.Core'

alias vba='vim ~/.bashrc'
alias sba='source ~/.bashrc'
alias add='git add .'
alias com='git commit -m'
alias qco='git add . && git commit -m "quick update"'
alias gpu='git push'
alias bra='git branch'
alias new='git checkout -b'
alias rhd='git checkout develop && git fetch && git reset --hard origin/develop'
alias rhm='git checkout master && git fetch && git reset --hard origin/master'
alias cre='git config credential.helper store'
