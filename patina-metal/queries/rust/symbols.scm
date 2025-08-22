; Functions
(function_item
  name: (identifier) @function.name) @function

; Methods
(impl_item
  type: (_) @impl.type
  body: (declaration_list
    (function_item
      name: (identifier) @method.name) @method))

; Structs
(struct_item
  name: (type_identifier) @struct.name) @struct

; Enums
(enum_item
  name: (type_identifier) @enum.name) @enum

; Traits
(trait_item
  name: (type_identifier) @trait.name) @trait

; Type aliases (type_item in this version)
(type_item
  name: (type_identifier) @type_alias.name) @type_alias

; Constants
(const_item
  name: (identifier) @constant.name) @constant

; Statics
(static_item
  name: (identifier) @static.name) @static

; Modules
(mod_item
  name: (identifier) @module.name) @module

; Macro definitions
(macro_definition
  name: (identifier) @macro.name) @macro