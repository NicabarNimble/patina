use patina_sdk::toys;

wit_bindgen::generate!({
    path: "wit",
    world: "{{ child_name }}",
    generate_all,
});

struct Child;

impl exports::patina::records::transform::Guest for Child {
    fn transform(
        records: Vec<patina::records::types::RecordEnvelope>,
    ) -> Result<patina::records::types::TransformResult, String> {
        toys::log::info("{{ child_name }}", "processing record batch");
        Ok(patina::records::types::TransformResult {
            accepted: records,
            rejected: Vec::new(),
        })
    }
}

export!(Child);
