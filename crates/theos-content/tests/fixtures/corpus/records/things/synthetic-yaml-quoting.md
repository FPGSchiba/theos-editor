---
# synthetic fixture (P02), not canon
id: synthetic-yaml-quoting
kind: thing
title: Synthetic YAML quoting
type: document
plain: a plain value with spaces
single: 'it''s a single-quoted value'
double: "a double-quoted value with \"quotes\" and a \\ backslash"
empty_double: ""
empty_single: ''
int_like: 42
zeros_like: 007
negative_like: -3
decimal_like: 2.99
exp_like: 1e5
bool_like: true
null_like: null
tilde_like: ~
yes_like: yes
hash_inside: b#c
url: https://example.com/p02/page#section
binary: x/y.png@sha256:0000000000000000000000000000000000000000000000000000000000000001
trailing: a value with trailing spaces   
flow_tight: [a,b]
flow_padded: [ a ]
flow_quoted: ['a', "b"]
flow_trailing_comma: [a, b,]
flow_empty: []
---
A synthetic body for a record whose front matter shows every way of writing a scalar.
