" Vim syntax file
" Language: Jez

if exists("b:current_syntax")
  finish
endif

syn keyword jezDirective def globals version track
syn match jezComment ";.*$"

syntax region jezString start=/"/ end=/"/

syntax match jezNumber "\v<\d+>"
syntax match jezNumber "\v<\d+\.\d+>"

syntax match jezOpertator "\v\~"
syntax match jezOpertator "\v\="

syntax match jezDelimiter "\v\["
syntax match jezDelimiter "\v\]"
syntax match jezDelimiter "\v\("
syntax match jezDelimiter "\v\)"

highlight default link jezComment Comment
highlight default link jezDirective Keyword
highlight default link jezString String
highlight default link jezNumber Number
highlight default link jezOpertator Operator
highlight default link jezDelimiter Delimiter

let b:current_syntax = "jez"
