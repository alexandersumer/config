colorscheme desert
set mouse=a
set autoindent
set smartindent

set cindent

" enable syntax
syntax on

autocmd BufRead,BufNewFile *.s :set syntax=mips 

" show line numbers
set number

" tabs are spaces
set expandtab
set smarttab

set shiftwidth=4

" number of visual spaces per TAB
set tabstop=4

" number of spaces in tab when editing
set softtabstop=4

" visual autocomplete for command menu
set wildmenu

" search as characters are entered
set incsearch
" highlight matches
set hlsearch
" turn off search highlight (\ )
nnoremap <leader><space> :nohlsearch<CR>

" move to beginning / end of line
nnoremap B ^
nnoremap E $

" highlight last inserted text
nnoremap gV `[v`]

"Enable folding by syntax
"set foldmethod=syntax
"autocmd BufRead * :normal zR

function! s:CloseBracket()
  let line = getline('.')
  if line =~# '^\s*struct\|class\|enumstruct∥class∥enum '
    return "{\<Enter>};\<Esc>O"
  elseif searchpair('(', '', ')', 'bmn', '', line('.'))
    " Probably inside a function call. Close it off.
    return "{\<Enter>});\<Esc>O"
  else
    return "{\<Enter>}\<Esc>O"
  endif
endfunction

inoremap <expr> {<Enter>  <SID>CloseBracket()

set undofile

set laststatus=2
set statusline+=%Fm
