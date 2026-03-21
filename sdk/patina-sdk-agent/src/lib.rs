pub const TOY_WIT_DIR: &str = env!("PATINA_SDK_AGENT_WIT_DIR");

pub trait QueryBackend {
    fn query(kind: &str, params_json: &str) -> Result<String, String>;
}

pub trait EmitBackend {
    fn emit(schema: &str, fact_type: &str, data: &str) -> Result<u64, String>;
}

pub trait SessionBackend {
    fn write(section: &str, content: &str) -> Result<(), String>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct QueryToy<B>(std::marker::PhantomData<B>);

impl<B> QueryToy<B> {
    pub fn new() -> Self {
        Self(std::marker::PhantomData)
    }
}

impl<B: QueryBackend> QueryToy<B> {
    pub fn query(&self, kind: &str, params_json: &str) -> Result<String, String> {
        B::query(kind, params_json)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct EmitToy<B>(std::marker::PhantomData<B>);

impl<B> EmitToy<B> {
    pub fn new() -> Self {
        Self(std::marker::PhantomData)
    }
}

impl<B: EmitBackend> EmitToy<B> {
    pub fn emit(&self, schema: &str, fact_type: &str, data: &str) -> Result<u64, String> {
        B::emit(schema, fact_type, data)
    }
}

#[derive(Debug, Clone, Copy, Default)]
pub struct SessionToy<B>(std::marker::PhantomData<B>);

impl<B> SessionToy<B> {
    pub fn new() -> Self {
        Self(std::marker::PhantomData)
    }
}

impl<B: SessionBackend> SessionToy<B> {
    pub fn write(&self, section: &str, content: &str) -> Result<(), String> {
        B::write(section, content)
    }
}
