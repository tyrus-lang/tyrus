" Detect .tyr files as the `tyrus` filetype so syntax/tyrus.vim and the language
" server attach.
autocmd BufRead,BufNewFile *.tyr setfiletype tyrus
