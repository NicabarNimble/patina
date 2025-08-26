; Cyclomatic complexity nodes
; Each of these adds a branch to the control flow

; If expressions/statements
(if_expression) @complexity.branch

; Match expressions and arms
(match_expression) @complexity.match
(match_arm) @complexity.branch

; Loops
(while_expression) @complexity.branch
(for_expression) @complexity.branch
(loop_expression) @complexity.branch

; Early returns and breaks
(return_expression) @complexity.early_exit
(break_expression) @complexity.early_exit
(continue_expression) @complexity.early_exit

; Closures (can add complexity)
(closure_expression) @complexity.closure

; Boolean operators (short-circuit)
(binary_expression
  operator: ["&&" "||"] @complexity.logical)