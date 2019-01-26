parse_git_branch() {
    git branch 2> /dev/null | sed -e '/^[^*]/d' -e 's/* \(.*\)/ (\1)/'
}

export PS1="\[\e]0;\w\a\]\n\[\e[32m\]\u\[\e[0;90m\]:\[\e[1;33m\]\w\[\e[31m\]\[\e[0;90m\]\[\e[0m\]\[\e[32m\]\$(parse_git_branch)\[\e[0;90m\]\n\[\033[33m\] $ \[\033[00m\]"

export UUSER='aj'
export WUSER='alexander.jones'
export UHOME='/home/aj'
export WHOME='/mnt/c/Users/alexander.jones'

alias c='clear'
alias ..='cd ..'
alias ...='cd ..; cd ..'
alias ....='cd ..; cd ..; cd ..'

alias proj='cd $WHOME/Dropbox/proj'
alias open='cd $WHOME/Dropbox/open'
alias sym='cd /mnt/c/Sympli'
alias docser='cd /mnt/c/Sympli/DocumentService'
alias tui='cd /mnt/c/Sympli/tickle-ui'
alias tapi='cd /mnt/c/Sympli/TickleApi'
alias tcore='cd /mnt/c/Sympli/Tickle.Core'
alias lrsqld='cd /mnt/c/Sympli/LRS.QLD'
alias lrsvic='cd /mnt/c/Sympli/LRS.VIC'
alias lrsnsw='cd /mntc/Sympli/LRS.NSW'
alias lrscore='cd /mnt/c/Sympli/LRS.Core'

alias vba='vim $UHOME/.bashrc'
alias sba='source $UHOME/.bashrc'
alias add='git add .'
alias com='git commit -m'
alias gpu='git push'
alias bra='git branch'
alias new='git checkout -b'
alias rhd='git checkout develop && git fetch && git reset --hard origin/develop'
alias rhm='git checkout master && git fetch && git reset --hard origin/master'
 alias cre='git config credential.helper store'

# Install Ruby Gems to ~/gems
export GEM_HOME="$UHOME/gems"
export PATH="$UHOME/gems/bin:$PATH"
