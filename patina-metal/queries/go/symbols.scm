; Functions
(function_declaration
  name: (identifier) @function.name) @function

; Methods
(method_declaration
  receiver: (parameter_list
    (parameter_declaration
      type: (_) @method.receiver))
  name: (field_identifier) @method.name) @method

; Structs
(type_declaration
  (type_spec
    name: (type_identifier) @struct.name
    type: (struct_type))) @struct

; Interfaces
(type_declaration
  (type_spec
    name: (type_identifier) @interface.name
    type: (interface_type))) @interface

; Type aliases
(type_declaration
  (type_spec
    name: (type_identifier) @type_alias.name
    type: (_))) @type_alias

; Constants
(const_declaration
  (const_spec
    name: (identifier) @constant.name)) @constant

; Variables
(var_declaration
  (var_spec
    name: (identifier) @variable.name)) @variable

; Package declaration
(package_clause
  (package_identifier) @package.name) @package

; Import declarations
(import_declaration
  (import_spec
    path: (interpreted_string_literal) @import.path)) @import