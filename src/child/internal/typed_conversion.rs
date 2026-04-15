use anyhow::Result;
use serde_json::Value;
use wasmtime::component::types::{ComponentFunc, Type};
use wasmtime::component::Val;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ConversionErrorCode {
    InvalidArgsShape,
    UnsupportedType,
    InvalidChildJson,
}

impl ConversionErrorCode {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::InvalidArgsShape => "invalid-args-shape",
            Self::UnsupportedType => "unsupported-type",
            Self::InvalidChildJson => "invalid-child-json",
        }
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ConversionError {
    code: ConversionErrorCode,
    detail: String,
    path: String,
}

impl ConversionError {
    fn new(code: ConversionErrorCode, detail: impl Into<String>, path: impl Into<String>) -> Self {
        Self {
            code,
            detail: detail.into(),
            path: path.into(),
        }
    }

    pub(crate) fn code(&self) -> ConversionErrorCode {
        self.code
    }

    pub(crate) fn detail(&self) -> &str {
        &self.detail
    }

    pub(crate) fn path(&self) -> &str {
        &self.path
    }
}

impl std::fmt::Display for ConversionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "typed invocation {}: {} (path: {})",
            self.code.as_str(),
            self.detail,
            self.path
        )
    }
}

impl std::error::Error for ConversionError {}

type ConvResult<T> = std::result::Result<T, ConversionError>;

#[derive(Debug, Clone)]
enum ConversionShape {
    Bool,
    S8,
    U8,
    S16,
    U16,
    S32,
    U32,
    S64,
    U64,
    Float32,
    Float64,
    Char,
    String,
    List(Box<ConversionShape>),
    Record(Vec<RecordFieldShape>),
    Tuple(Vec<ConversionShape>),
    Variant(Vec<VariantCaseShape>),
    Enum(Vec<String>),
    Option(Box<ConversionShape>),
    Result {
        ok: Option<Box<ConversionShape>>,
        err: Option<Box<ConversionShape>>,
    },
    Flags(Vec<String>),
}

#[derive(Debug, Clone)]
struct RecordFieldShape {
    name: String,
    ty: ConversionShape,
}

#[derive(Debug, Clone)]
struct VariantCaseShape {
    name: String,
    ty: Option<ConversionShape>,
}

fn err(
    code: ConversionErrorCode,
    detail: impl Into<String>,
    path: impl Into<String>,
) -> ConversionError {
    ConversionError::new(code, detail, path)
}

fn child_path(parent: &str, segment: impl AsRef<str>) -> String {
    if parent == "$" {
        format!("$.{}", segment.as_ref())
    } else {
        format!("{}.{}", parent, segment.as_ref())
    }
}

fn index_path(parent: &str, idx: usize) -> String {
    format!("{}[{}]", parent, idx)
}

fn shape_from_type(ty: &Type, path: &str) -> ConvResult<ConversionShape> {
    Ok(match ty {
        Type::Bool => ConversionShape::Bool,
        Type::S8 => ConversionShape::S8,
        Type::U8 => ConversionShape::U8,
        Type::S16 => ConversionShape::S16,
        Type::U16 => ConversionShape::U16,
        Type::S32 => ConversionShape::S32,
        Type::U32 => ConversionShape::U32,
        Type::S64 => ConversionShape::S64,
        Type::U64 => ConversionShape::U64,
        Type::Float32 => ConversionShape::Float32,
        Type::Float64 => ConversionShape::Float64,
        Type::Char => ConversionShape::Char,
        Type::String => ConversionShape::String,
        Type::List(list_ty) => {
            ConversionShape::List(Box::new(shape_from_type(&list_ty.ty(), path)?))
        }
        Type::Record(record_ty) => {
            let mut fields = Vec::new();
            for field in record_ty.fields() {
                fields.push(RecordFieldShape {
                    name: field.name.to_string(),
                    ty: shape_from_type(&field.ty, &child_path(path, field.name))?,
                });
            }
            ConversionShape::Record(fields)
        }
        Type::Tuple(tuple_ty) => {
            let mut types = Vec::new();
            for (idx, tuple_item) in tuple_ty.types().enumerate() {
                types.push(shape_from_type(&tuple_item, &index_path(path, idx))?);
            }
            ConversionShape::Tuple(types)
        }
        Type::Variant(variant_ty) => {
            let mut cases = Vec::new();
            for case in variant_ty.cases() {
                cases.push(VariantCaseShape {
                    name: case.name.to_string(),
                    ty: case
                        .ty
                        .map(|ty| shape_from_type(&ty, &child_path(path, "value")))
                        .transpose()?,
                });
            }
            ConversionShape::Variant(cases)
        }
        Type::Enum(enum_ty) => {
            ConversionShape::Enum(enum_ty.names().map(|s| s.to_string()).collect())
        }
        Type::Option(option_ty) => {
            ConversionShape::Option(Box::new(shape_from_type(&option_ty.ty(), path)?))
        }
        Type::Result(result_ty) => ConversionShape::Result {
            ok: result_ty
                .ok()
                .map(|ty| shape_from_type(&ty, &child_path(path, "ok")))
                .transpose()?
                .map(Box::new),
            err: result_ty
                .err()
                .map(|ty| shape_from_type(&ty, &child_path(path, "err")))
                .transpose()?
                .map(Box::new),
        },
        Type::Flags(flags_ty) => {
            ConversionShape::Flags(flags_ty.names().map(|s| s.to_string()).collect())
        }
        Type::Own(_) | Type::Borrow(_) | Type::Future(_) | Type::Stream(_) | Type::ErrorContext => {
            return Err(err(
                ConversionErrorCode::UnsupportedType,
                format!("unsupported component type for JSON lowering: {:?}", ty),
                path,
            ));
        }
    })
}

fn lower_json_with_shape(value: &Value, shape: &ConversionShape, path: &str) -> ConvResult<Val> {
    Ok(match shape {
        ConversionShape::Bool => Val::Bool(
            value
                .as_bool()
                .ok_or_else(|| err(ConversionErrorCode::InvalidArgsShape, "expected bool", path))?,
        ),
        ConversionShape::S8 => {
            Val::S8(
                i8::try_from(value.as_i64().ok_or_else(|| {
                    err(ConversionErrorCode::InvalidArgsShape, "expected s8", path)
                })?)
                .map_err(|_| {
                    err(
                        ConversionErrorCode::InvalidArgsShape,
                        "s8 out of range",
                        path,
                    )
                })?,
            )
        }
        ConversionShape::U8 => {
            Val::U8(
                u8::try_from(value.as_u64().ok_or_else(|| {
                    err(ConversionErrorCode::InvalidArgsShape, "expected u8", path)
                })?)
                .map_err(|_| {
                    err(
                        ConversionErrorCode::InvalidArgsShape,
                        "u8 out of range",
                        path,
                    )
                })?,
            )
        }
        ConversionShape::S16 => {
            Val::S16(
                i16::try_from(value.as_i64().ok_or_else(|| {
                    err(ConversionErrorCode::InvalidArgsShape, "expected s16", path)
                })?)
                .map_err(|_| {
                    err(
                        ConversionErrorCode::InvalidArgsShape,
                        "s16 out of range",
                        path,
                    )
                })?,
            )
        }
        ConversionShape::U16 => {
            Val::U16(
                u16::try_from(value.as_u64().ok_or_else(|| {
                    err(ConversionErrorCode::InvalidArgsShape, "expected u16", path)
                })?)
                .map_err(|_| {
                    err(
                        ConversionErrorCode::InvalidArgsShape,
                        "u16 out of range",
                        path,
                    )
                })?,
            )
        }
        ConversionShape::S32 => {
            Val::S32(
                i32::try_from(value.as_i64().ok_or_else(|| {
                    err(ConversionErrorCode::InvalidArgsShape, "expected s32", path)
                })?)
                .map_err(|_| {
                    err(
                        ConversionErrorCode::InvalidArgsShape,
                        "s32 out of range",
                        path,
                    )
                })?,
            )
        }
        ConversionShape::U32 => {
            Val::U32(
                u32::try_from(value.as_u64().ok_or_else(|| {
                    err(ConversionErrorCode::InvalidArgsShape, "expected u32", path)
                })?)
                .map_err(|_| {
                    err(
                        ConversionErrorCode::InvalidArgsShape,
                        "u32 out of range",
                        path,
                    )
                })?,
            )
        }
        ConversionShape::S64 => Val::S64(
            value
                .as_i64()
                .ok_or_else(|| err(ConversionErrorCode::InvalidArgsShape, "expected s64", path))?,
        ),
        ConversionShape::U64 => Val::U64(
            value
                .as_u64()
                .ok_or_else(|| err(ConversionErrorCode::InvalidArgsShape, "expected u64", path))?,
        ),
        ConversionShape::Float32 => {
            let raw = value.as_f64().ok_or_else(|| {
                err(
                    ConversionErrorCode::InvalidArgsShape,
                    "expected float32",
                    path,
                )
            })?;
            if !raw.is_finite() {
                return Err(err(
                    ConversionErrorCode::InvalidArgsShape,
                    "float32 must be finite",
                    path,
                ));
            }
            Val::Float32(raw as f32)
        }
        ConversionShape::Float64 => {
            let raw = value.as_f64().ok_or_else(|| {
                err(
                    ConversionErrorCode::InvalidArgsShape,
                    "expected float64",
                    path,
                )
            })?;
            if !raw.is_finite() {
                return Err(err(
                    ConversionErrorCode::InvalidArgsShape,
                    "float64 must be finite",
                    path,
                ));
            }
            Val::Float64(raw)
        }
        ConversionShape::Char => {
            let as_str = value.as_str().ok_or_else(|| {
                err(
                    ConversionErrorCode::InvalidArgsShape,
                    "expected char string",
                    path,
                )
            })?;
            let mut chars = as_str.chars();
            let ch = chars.next().ok_or_else(|| {
                err(
                    ConversionErrorCode::InvalidArgsShape,
                    "expected non-empty char string",
                    path,
                )
            })?;
            if chars.next().is_some() {
                return Err(err(
                    ConversionErrorCode::InvalidArgsShape,
                    "char string must contain exactly one scalar",
                    path,
                ));
            }
            Val::Char(ch)
        }
        ConversionShape::String => Val::String(
            value
                .as_str()
                .ok_or_else(|| {
                    err(
                        ConversionErrorCode::InvalidArgsShape,
                        "expected string",
                        path,
                    )
                })?
                .to_string(),
        ),
        ConversionShape::List(element_shape) => {
            let values = value
                .as_array()
                .ok_or_else(|| err(ConversionErrorCode::InvalidArgsShape, "expected list", path))?;
            let mut lowered = Vec::with_capacity(values.len());
            for (idx, item) in values.iter().enumerate() {
                lowered.push(lower_json_with_shape(
                    item,
                    element_shape,
                    &index_path(path, idx),
                )?);
            }
            Val::List(lowered)
        }
        ConversionShape::Record(fields) => {
            let object = value.as_object().ok_or_else(|| {
                err(
                    ConversionErrorCode::InvalidArgsShape,
                    "expected record object",
                    path,
                )
            })?;
            let mut lowered = Vec::new();
            let mut seen = std::collections::HashSet::new();
            for field in fields {
                seen.insert(field.name.clone());
                let field_value = object.get(&field.name).ok_or_else(|| {
                    err(
                        ConversionErrorCode::InvalidArgsShape,
                        format!("missing record field '{}'", field.name),
                        path,
                    )
                })?;
                lowered.push((
                    field.name.clone(),
                    lower_json_with_shape(field_value, &field.ty, &child_path(path, &field.name))?,
                ));
            }
            for key in object.keys() {
                if !seen.contains(key) {
                    return Err(err(
                        ConversionErrorCode::InvalidArgsShape,
                        format!("unknown record field '{}'", key),
                        &child_path(path, key),
                    ));
                }
            }
            Val::Record(lowered)
        }
        ConversionShape::Tuple(tuple_shapes) => {
            let values = value.as_array().ok_or_else(|| {
                err(
                    ConversionErrorCode::InvalidArgsShape,
                    "expected tuple array",
                    path,
                )
            })?;
            if values.len() != tuple_shapes.len() {
                return Err(err(
                    ConversionErrorCode::InvalidArgsShape,
                    format!(
                        "tuple arity mismatch: expected {}, got {}",
                        tuple_shapes.len(),
                        values.len()
                    ),
                    path,
                ));
            }
            let mut lowered = Vec::with_capacity(values.len());
            for (idx, (item, item_shape)) in values.iter().zip(tuple_shapes.iter()).enumerate() {
                lowered.push(lower_json_with_shape(
                    item,
                    item_shape,
                    &index_path(path, idx),
                )?);
            }
            Val::Tuple(lowered)
        }
        ConversionShape::Variant(cases) => {
            let object = value.as_object().ok_or_else(|| {
                err(
                    ConversionErrorCode::InvalidArgsShape,
                    "expected variant object",
                    path,
                )
            })?;
            let case = object.get("case").and_then(|v| v.as_str()).ok_or_else(|| {
                err(
                    ConversionErrorCode::InvalidArgsShape,
                    "variant requires string field 'case'",
                    &child_path(path, "case"),
                )
            })?;
            let Some(case_meta) = cases.iter().find(|item| item.name == case) else {
                return Err(err(
                    ConversionErrorCode::InvalidArgsShape,
                    format!("unknown variant case '{}'", case),
                    &child_path(path, "case"),
                ));
            };

            for key in object.keys() {
                if key != "case" && key != "value" {
                    return Err(err(
                        ConversionErrorCode::InvalidArgsShape,
                        format!("unknown variant field '{}'", key),
                        &child_path(path, key),
                    ));
                }
            }

            let payload = match &case_meta.ty {
                Some(case_shape) => {
                    let raw_payload = object.get("value").ok_or_else(|| {
                        err(
                            ConversionErrorCode::InvalidArgsShape,
                            format!("variant case '{}' requires 'value'", case),
                            &child_path(path, "value"),
                        )
                    })?;
                    Some(Box::new(lower_json_with_shape(
                        raw_payload,
                        case_shape,
                        &child_path(path, "value"),
                    )?))
                }
                None => {
                    if let Some(raw_payload) = object.get("value") {
                        if !raw_payload.is_null() {
                            return Err(err(
                                ConversionErrorCode::InvalidArgsShape,
                                format!("variant case '{}' does not accept a payload value", case),
                                &child_path(path, "value"),
                            ));
                        }
                    }
                    None
                }
            };
            Val::Variant(case.to_string(), payload)
        }
        ConversionShape::Enum(allowed_cases) => {
            let case = value.as_str().ok_or_else(|| {
                err(
                    ConversionErrorCode::InvalidArgsShape,
                    "expected enum string",
                    path,
                )
            })?;
            if !allowed_cases.iter().any(|name| name == case) {
                return Err(err(
                    ConversionErrorCode::InvalidArgsShape,
                    format!("unknown enum case '{}'", case),
                    path,
                ));
            }
            Val::Enum(case.to_string())
        }
        ConversionShape::Option(inner_shape) => {
            if value.is_null() {
                Val::Option(None)
            } else {
                Val::Option(Some(Box::new(lower_json_with_shape(
                    value,
                    inner_shape,
                    path,
                )?)))
            }
        }
        ConversionShape::Result { ok, err: err_shape } => {
            let object = value.as_object().ok_or_else(|| {
                err(
                    ConversionErrorCode::InvalidArgsShape,
                    "expected result object",
                    path,
                )
            })?;
            if object.len() != 1 {
                return Err(err(
                    ConversionErrorCode::InvalidArgsShape,
                    "result object must contain exactly one of 'ok' or 'err'",
                    path,
                ));
            }
            if let Some(ok_value) = object.get("ok") {
                let lowered = match ok {
                    Some(ok_shape) => {
                        if ok_value.is_null() {
                            None
                        } else {
                            Some(Box::new(lower_json_with_shape(
                                ok_value,
                                ok_shape,
                                &child_path(path, "ok"),
                            )?))
                        }
                    }
                    None => {
                        if !ok_value.is_null() {
                            return Err(err(
                                ConversionErrorCode::InvalidArgsShape,
                                "result 'ok' payload not allowed for this signature",
                                &child_path(path, "ok"),
                            ));
                        }
                        None
                    }
                };
                Val::Result(Ok(lowered))
            } else if let Some(err_value) = object.get("err") {
                let lowered = match err_shape {
                    Some(err_shape) => {
                        if err_value.is_null() {
                            None
                        } else {
                            Some(Box::new(lower_json_with_shape(
                                err_value,
                                err_shape,
                                &child_path(path, "err"),
                            )?))
                        }
                    }
                    None => {
                        if !err_value.is_null() {
                            return Err(err(
                                ConversionErrorCode::InvalidArgsShape,
                                "result 'err' payload not allowed for this signature",
                                &child_path(path, "err"),
                            ));
                        }
                        None
                    }
                };
                Val::Result(Err(lowered))
            } else {
                return Err(err(
                    ConversionErrorCode::InvalidArgsShape,
                    "result object must contain either 'ok' or 'err'",
                    path,
                ));
            }
        }
        ConversionShape::Flags(allowed_flags) => {
            let values = value.as_array().ok_or_else(|| {
                err(
                    ConversionErrorCode::InvalidArgsShape,
                    "expected flags string array",
                    path,
                )
            })?;
            let mut names = Vec::new();
            let mut seen = std::collections::HashSet::new();
            for (idx, item) in values.iter().enumerate() {
                let item_path = index_path(path, idx);
                let name = item.as_str().ok_or_else(|| {
                    err(
                        ConversionErrorCode::InvalidArgsShape,
                        "flags values must be strings",
                        &item_path,
                    )
                })?;
                if !allowed_flags.iter().any(|allowed| allowed == name) {
                    return Err(err(
                        ConversionErrorCode::InvalidArgsShape,
                        format!("unknown flag '{}'", name),
                        &item_path,
                    ));
                }
                if !seen.insert(name.to_string()) {
                    return Err(err(
                        ConversionErrorCode::InvalidArgsShape,
                        format!("duplicate flag '{}'", name),
                        &item_path,
                    ));
                }
                names.push(name.to_string());
            }
            Val::Flags(names)
        }
    })
}

/// Lift a component value into JSON transport shape.
///
/// For variants without payload, the JSON form is normalized to
/// `{"case": "<name>", "value": null}`.
fn lift_component_val_to_json(value: &Val) -> ConvResult<Value> {
    Ok(match value {
        Val::Bool(v) => serde_json::json!(v),
        Val::S8(v) => serde_json::json!(v),
        Val::U8(v) => serde_json::json!(v),
        Val::S16(v) => serde_json::json!(v),
        Val::U16(v) => serde_json::json!(v),
        Val::S32(v) => serde_json::json!(v),
        Val::U32(v) => serde_json::json!(v),
        Val::S64(v) => serde_json::json!(v),
        Val::U64(v) => serde_json::json!(v),
        Val::Float32(v) => serde_json::json!(v),
        Val::Float64(v) => serde_json::json!(v),
        Val::Char(v) => serde_json::json!(v.to_string()),
        Val::String(v) => serde_json::json!(v),
        Val::List(values) => serde_json::Value::Array(
            values
                .iter()
                .map(lift_component_val_to_json)
                .collect::<ConvResult<Vec<_>>>()?,
        ),
        Val::Record(fields) => {
            let mut object = serde_json::Map::new();
            for (name, value) in fields {
                object.insert(name.clone(), lift_component_val_to_json(value)?);
            }
            serde_json::Value::Object(object)
        }
        Val::Tuple(values) => serde_json::Value::Array(
            values
                .iter()
                .map(lift_component_val_to_json)
                .collect::<ConvResult<Vec<_>>>()?,
        ),
        Val::Variant(case, payload) => {
            let mut object = serde_json::Map::new();
            object.insert("case".to_string(), serde_json::json!(case));
            object.insert(
                "value".to_string(),
                match payload {
                    Some(value) => lift_component_val_to_json(value)?,
                    None => serde_json::Value::Null,
                },
            );
            serde_json::Value::Object(object)
        }
        Val::Enum(case) => serde_json::json!(case),
        Val::Option(payload) => match payload {
            Some(value) => lift_component_val_to_json(value)?,
            None => serde_json::Value::Null,
        },
        Val::Result(result) => {
            let mut object = serde_json::Map::new();
            match result {
                Ok(value) => {
                    object.insert(
                        "ok".to_string(),
                        value
                            .as_ref()
                            .map(|v| lift_component_val_to_json(v))
                            .transpose()?
                            .unwrap_or(serde_json::Value::Null),
                    );
                }
                Err(value) => {
                    object.insert(
                        "err".to_string(),
                        value
                            .as_ref()
                            .map(|v| lift_component_val_to_json(v))
                            .transpose()?
                            .unwrap_or(serde_json::Value::Null),
                    );
                }
            }
            serde_json::Value::Object(object)
        }
        Val::Flags(flags) => serde_json::Value::Array(
            flags
                .iter()
                .cloned()
                .map(serde_json::Value::String)
                .collect(),
        ),
        Val::Resource(_) | Val::Future(_) | Val::Stream(_) | Val::ErrorContext(_) => {
            return Err(err(
                ConversionErrorCode::UnsupportedType,
                "unsupported component result type for JSON lift",
                "$",
            ));
        }
    })
}

fn lift_component_results_with_shapes(
    values: &[Val],
    result_shapes: &[ConversionShape],
) -> ConvResult<Value> {
    if values.len() != result_shapes.len() {
        return Err(err(
            ConversionErrorCode::InvalidChildJson,
            format!(
                "result arity mismatch: expected {}, got {}",
                result_shapes.len(),
                values.len()
            ),
            "$.results",
        ));
    }

    let mut lifted = Vec::with_capacity(values.len());
    for value in values {
        lifted.push(lift_component_val_to_json(value)?);
    }

    Ok(serde_json::json!({"results": lifted}))
}

fn map_conv_err(error: ConversionError) -> anyhow::Error {
    anyhow::Error::new(error)
}

pub(crate) fn lower_typed_args_for_component(args: &Value, ty: &ComponentFunc) -> Result<Vec<Val>> {
    let arg_values = args.as_array().ok_or_else(|| {
        map_conv_err(err(
            ConversionErrorCode::InvalidArgsShape,
            "typed call args must be a JSON array",
            "$",
        ))
    })?;

    let params = ty.params().collect::<Vec<_>>();
    if arg_values.len() != params.len() {
        return Err(map_conv_err(err(
            ConversionErrorCode::InvalidArgsShape,
            format!(
                "operation expects {} args, got {}",
                params.len(),
                arg_values.len()
            ),
            "$",
        )));
    }

    let mut lowered = Vec::with_capacity(params.len());
    for (idx, ((param_name, param_ty), arg)) in params.iter().zip(arg_values.iter()).enumerate() {
        let shape = shape_from_type(param_ty, &index_path("$", idx)).map_err(map_conv_err)?;
        let lowered_arg =
            lower_json_with_shape(arg, &shape, &index_path("$", idx)).map_err(|error| {
                map_conv_err(err(
                    error.code(),
                    format!("arg '{}' failed to lower: {}", param_name, error.detail()),
                    error.path(),
                ))
            })?;
        lowered.push(lowered_arg);
    }
    Ok(lowered)
}

pub(crate) fn lift_component_results_to_json(values: &[Val], ty: &ComponentFunc) -> Result<Value> {
    let result_shapes = ty
        .results()
        .enumerate()
        .map(|(idx, result_ty)| shape_from_type(&result_ty, &index_path("$.results", idx)))
        .collect::<ConvResult<Vec<_>>>()
        .map_err(map_conv_err)?;

    lift_component_results_with_shapes(values, &result_shapes).map_err(map_conv_err)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip_with_shape(shape: ConversionShape, input: Value) -> Value {
        let lowered = lower_json_with_shape(&input, &shape, "$").expect("lower");
        lift_component_val_to_json(&lowered).expect("lift")
    }

    #[test]
    fn scalar_roundtrip_matrix_is_identity() {
        let cases = vec![
            (ConversionShape::Bool, serde_json::json!(true)),
            (ConversionShape::S32, serde_json::json!(-7)),
            (ConversionShape::U64, serde_json::json!(9)),
            (ConversionShape::Float64, serde_json::json!(1.25)),
            (ConversionShape::Char, serde_json::json!("x")),
            (ConversionShape::String, serde_json::json!("hello")),
        ];

        for (shape, input) in cases {
            let output = roundtrip_with_shape(shape, input.clone());
            assert_eq!(output, input);
        }
    }

    #[test]
    fn record_tuple_list_roundtrip_matrix_is_identity() {
        let record_shape = ConversionShape::Record(vec![
            RecordFieldShape {
                name: "count".to_string(),
                ty: ConversionShape::U32,
            },
            RecordFieldShape {
                name: "name".to_string(),
                ty: ConversionShape::String,
            },
        ]);
        let tuple_shape =
            ConversionShape::Tuple(vec![ConversionShape::String, ConversionShape::Bool]);
        let list_shape = ConversionShape::List(Box::new(ConversionShape::S16));

        assert_eq!(
            roundtrip_with_shape(record_shape, serde_json::json!({"count": 2, "name": "n"})),
            serde_json::json!({"count": 2, "name": "n"})
        );
        assert_eq!(
            roundtrip_with_shape(tuple_shape, serde_json::json!(["x", true])),
            serde_json::json!(["x", true])
        );
        assert_eq!(
            roundtrip_with_shape(list_shape, serde_json::json!([1, 2, 3])),
            serde_json::json!([1, 2, 3])
        );
    }

    #[test]
    fn option_variant_enum_result_flags_matrix_behaves_as_documented() {
        let option_shape = ConversionShape::Option(Box::new(ConversionShape::String));
        assert_eq!(
            roundtrip_with_shape(option_shape.clone(), serde_json::json!(null)),
            serde_json::json!(null)
        );
        assert_eq!(
            roundtrip_with_shape(option_shape, serde_json::json!("x")),
            serde_json::json!("x")
        );

        let variant_shape = ConversionShape::Variant(vec![
            VariantCaseShape {
                name: "idle".to_string(),
                ty: None,
            },
            VariantCaseShape {
                name: "value".to_string(),
                ty: Some(ConversionShape::U32),
            },
        ]);
        assert_eq!(
            roundtrip_with_shape(variant_shape.clone(), serde_json::json!({"case": "idle"})),
            serde_json::json!({"case": "idle", "value": null})
        );
        assert_eq!(
            roundtrip_with_shape(
                variant_shape,
                serde_json::json!({"case": "value", "value": 5})
            ),
            serde_json::json!({"case": "value", "value": 5})
        );

        let enum_shape = ConversionShape::Enum(vec!["a".to_string(), "b".to_string()]);
        assert_eq!(
            roundtrip_with_shape(enum_shape, serde_json::json!("a")),
            serde_json::json!("a")
        );

        let result_shape = ConversionShape::Result {
            ok: Some(Box::new(ConversionShape::String)),
            err: Some(Box::new(ConversionShape::String)),
        };
        assert_eq!(
            roundtrip_with_shape(result_shape.clone(), serde_json::json!({"ok": "done"})),
            serde_json::json!({"ok": "done"})
        );
        assert_eq!(
            roundtrip_with_shape(result_shape, serde_json::json!({"err": "fail"})),
            serde_json::json!({"err": "fail"})
        );

        let flags_shape = ConversionShape::Flags(vec!["read".to_string(), "write".to_string()]);
        assert_eq!(
            roundtrip_with_shape(flags_shape, serde_json::json!(["read"])),
            serde_json::json!(["read"])
        );
    }

    #[test]
    fn strict_fail_closed_on_invalid_shapes_and_no_implicit_coercion() {
        let err = lower_json_with_shape(&serde_json::json!("123"), &ConversionShape::U32, "$")
            .expect_err("string->u32 coercion must fail");
        assert_eq!(err.code(), ConversionErrorCode::InvalidArgsShape);
        assert_eq!(err.path(), "$");

        let err = lower_json_with_shape(&serde_json::json!(""), &ConversionShape::Char, "$")
            .expect_err("empty char string must fail");
        assert_eq!(err.code(), ConversionErrorCode::InvalidArgsShape);
        assert_eq!(err.path(), "$");

        let err = lower_json_with_shape(&serde_json::json!("xx"), &ConversionShape::Char, "$")
            .expect_err("multi-char string must fail");
        assert_eq!(err.code(), ConversionErrorCode::InvalidArgsShape);
        assert_eq!(err.path(), "$");

        let record_shape = ConversionShape::Record(vec![RecordFieldShape {
            name: "count".to_string(),
            ty: ConversionShape::U32,
        }]);
        let err = lower_json_with_shape(
            &serde_json::json!({"count": 1, "extra": true}),
            &record_shape,
            "$",
        )
        .expect_err("unknown record field must fail");
        assert_eq!(err.code(), ConversionErrorCode::InvalidArgsShape);
        assert_eq!(err.path(), "$.extra");

        let err = lower_json_with_shape(
            &serde_json::json!("c"),
            &ConversionShape::Enum(vec!["a".to_string(), "b".to_string()]),
            "$",
        )
        .expect_err("unknown enum variant must fail");
        assert_eq!(err.code(), ConversionErrorCode::InvalidArgsShape);
        assert_eq!(err.path(), "$");

        let err = lower_json_with_shape(
            &serde_json::json!(["x"]),
            &ConversionShape::Tuple(vec![ConversionShape::String, ConversionShape::Bool]),
            "$",
        )
        .expect_err("tuple wrong arity must fail");
        assert_eq!(err.code(), ConversionErrorCode::InvalidArgsShape);
        assert_eq!(err.path(), "$");

        let err = lower_json_with_shape(
            &serde_json::json!(["x"]),
            &ConversionShape::List(Box::new(ConversionShape::U8)),
            "$",
        )
        .expect_err("list element type mismatch must fail");
        assert_eq!(err.code(), ConversionErrorCode::InvalidArgsShape);
        assert_eq!(err.path(), "$[0]");

        let err = lower_json_with_shape(
            &serde_json::json!("x"),
            &ConversionShape::Option(Box::new(ConversionShape::U32)),
            "$",
        )
        .expect_err("option inner type mismatch must fail");
        assert_eq!(err.code(), ConversionErrorCode::InvalidArgsShape);
        assert_eq!(err.path(), "$");

        let result_shape = ConversionShape::Result {
            ok: Some(Box::new(ConversionShape::U32)),
            err: Some(Box::new(ConversionShape::String)),
        };
        let err = lower_json_with_shape(
            &serde_json::json!({"ok": 1, "err": "x"}),
            &result_shape,
            "$",
        )
        .expect_err("result with both channels must fail");
        assert_eq!(err.code(), ConversionErrorCode::InvalidArgsShape);

        let flags_shape = ConversionShape::Flags(vec!["read".to_string()]);
        let err = lower_json_with_shape(&serde_json::json!(["read", "read"]), &flags_shape, "$")
            .expect_err("duplicate flags must fail");
        assert_eq!(err.code(), ConversionErrorCode::InvalidArgsShape);
    }

    #[test]
    fn result_lifting_uses_explicit_results_envelope_for_zero_one_many() {
        let zero = lift_component_results_with_shapes(&[], &[]).expect("zero results");
        assert_eq!(zero, serde_json::json!({"results": []}));

        let one = lift_component_results_with_shapes(&[Val::U32(7)], &[ConversionShape::U32])
            .expect("one result");
        assert_eq!(one, serde_json::json!({"results": [7]}));

        let many = lift_component_results_with_shapes(
            &[Val::String("x".to_string()), Val::Bool(true)],
            &[ConversionShape::String, ConversionShape::Bool],
        )
        .expect("many results");
        assert_eq!(many, serde_json::json!({"results": ["x", true]}));
    }

    #[test]
    fn structured_error_exposes_stable_code_detail_and_path() {
        let err = lower_json_with_shape(&serde_json::json!({}), &ConversionShape::Bool, "$")
            .expect_err("invalid bool shape should fail");
        assert_eq!(err.code(), ConversionErrorCode::InvalidArgsShape);
        assert_eq!(err.detail(), "expected bool");
        assert_eq!(err.path(), "$");
        assert!(
            err.to_string()
                .contains("typed invocation invalid-args-shape:"),
            "{}",
            err
        );
    }
}
