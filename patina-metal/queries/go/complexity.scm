; Cyclomatic complexity nodes for Go

; If statements
(if_statement) @complexity.branch

; Switch statements and cases (expression_switch_statement in this version)
(expression_switch_statement) @complexity.switch
(expression_case) @complexity.branch
(default_case) @complexity.branch

; Loops
(for_statement) @complexity.branch
(range_clause) @complexity.branch

; Early returns and control flow
(return_statement) @complexity.early_exit
(break_statement) @complexity.early_exit
(continue_statement) @complexity.early_exit
(goto_statement) @complexity.goto
(fallthrough_statement) @complexity.fallthrough

; Error handling
(if_statement
  condition: (binary_expression
    left: (_)
    operator: "!="
    right: (nil))) @complexity.error_check

; Defer statements (can add complexity)
(defer_statement) @complexity.defer

; Go routines (concurrent complexity)
(go_statement) @complexity.goroutine

; Channel operations
(send_statement) @complexity.channel

; Select statements
(select_statement) @complexity.select
(communication_clause) @complexity.branch

; Boolean operators
(binary_expression
  operator: ["&&" "||"] @complexity.logical)