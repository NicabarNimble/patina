; Common Rust patterns

; Builder pattern - simplified for compatibility
(impl_item
  type: (type_identifier) @builder.struct
  body: (declaration_list
    (function_item
      name: (identifier) @builder.method
      (#match? @builder.method "^(with_|set_)")))) @builder.pattern

; Factory pattern
(function_item
  name: (identifier) @factory.method
  (#match? @factory.method "^(new|create|from_)")
  return_type: (_)) @factory.pattern

; Result/Option handling patterns
(match_expression
  body: (match_block
    (match_arm
      pattern: (tuple_struct_pattern
        type: (identifier) @result.variant
        (#match? @result.variant "^(Ok|Some)")))))

; Iterator chains
(method_call_expression
  receiver: (method_call_expression) @iterator.chain
  method: (field_expression
    field: (field_identifier) @iterator.method
    (#match? @iterator.method "^(map|filter|fold|collect|zip)")))

; Unsafe blocks
(unsafe_block) @unsafe.usage

; Lifetime annotations
(lifetime
  (identifier) @lifetime.name) @lifetime.annotation

; Async patterns
(async_block) @async.block
(await_expression) @async.await