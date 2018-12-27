parse_git_branch() {
    git branch 2> /dev/null | sed -e '/^[^*]/d' -e 's/* \(.*\)/ (\1)/'
}

export PS1="\[\e]0;\w\a\]\n\[\e[32m\]\u\[\e[0;90m\]:\[\e[1;33m\]\w\[\e[31m\]\[\e[0;90m\]\[\e[0m\]\[\e[32m\]\$(parse_git_branch)\[\e[0;90m\]\n\[\033[33m\] $ \[\033[00m\]"

alias c='clear'
alias ..='cd ..'
alias ...='cd ..; cd ..'
alias ....='cd ..; cd ..; cd ..'
alias gdr='cd ~/Google\ Drive'
alias prj='cd ~/Google\ Drive/projects'
alias sym='cd /c/Sympli'
alias doc='cd /c/Sympli/DocumentService'
alias sui='cd /c/Sympli/tickle-ui'
alias vba='vim ~/.bashrc'
alias sba='source ~/.bashrc'
alias com='git add . && git commit -m "update"'
alias dev='git checkout develop && git fetch && git reset --hard origin/develop'
alias cre='git config credential.helper store'
