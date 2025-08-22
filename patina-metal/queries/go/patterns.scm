; Common Go patterns

; Constructor/Factory pattern
(function_declaration
  name: (identifier) @factory.name
  (#match? @factory.name "^(New|Create)")
  result: (parameter_list
    (parameter_declaration
      type: (pointer_type)))) @factory.pattern

; Error handling pattern - simplified for compatibility
(if_statement
  condition: (binary_expression
    left: (identifier) @error.check
    operator: "!="
    right: (nil))) @error.handling

; Defer cleanup pattern
(defer_statement
  (call_expression
    function: (selector_expression
      field: (field_identifier) @defer.cleanup
      (#match? @defer.cleanup "^(Close|Unlock|Done)")))) @defer.pattern

; Interface implementation check (simplified)
(type_switch_statement) @interface.check

; Goroutine with WaitGroup
(go_statement
  (call_expression)) @goroutine.spawn

; Channel patterns (simplified)
(for_statement) @channel.loop

; Context usage
(call_expression
  function: (selector_expression
    operand: (identifier) @context.var
    (#eq? @context.var "ctx")
    field: (field_identifier))) @context.usage

; Embedded struct pattern
(struct_type
  (field_declaration_list
    (field_declaration
      type: (type_identifier) @embedded.type
      (#not-match? @embedded.type "^[a-z]")))) @embedded.struct

; Method chaining
(call_expression
  function: (selector_expression
    operand: (call_expression) @chain.call)) @method.chain

; Singleton pattern (package-level var)
(var_declaration
  (var_spec
    name: (identifier) @singleton.instance
    (#match? @singleton.instance "^(instance|singleton)")
    type: (pointer_type))) @singleton.pattern