pub const TOY_WIT_DIR: &str = env!("PATINA_SDK_AGENT_WIT_DIR");

pub trait QueryBackend {
    fn query(kind: &str, params_json: &str) -> Result<String, String>;
}

pub trait EmitBackend {
    fn emit(schema: &str, fact_type: &str, data: &str) -> Result<u64, String>;
}

pub trait SessionBackend {
    fn get_session_id() -> String;
    fn get_previous_session() -> Option<String>;
    fn get_previous_session_runtime_id() -> Option<String>;
    fn get_previous_session_handoff() -> Option<String>;
    fn write(section: &str, content: &str) -> Result<(), String>;
    fn set_parent_session(runtime_id: &str) -> Result<(), String>;
    fn create_tag(name: &str) -> Result<(), String>;
    fn set_status(status: &str) -> Result<(), String>;
    fn write_handoff(modified_files: &str, summary: &str) -> Result<(), String>;
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
    pub fn get_session_id(&self) -> String {
        B::get_session_id()
    }

    pub fn get_previous_session(&self) -> Option<String> {
        B::get_previous_session()
    }

    pub fn get_previous_session_runtime_id(&self) -> Option<String> {
        B::get_previous_session_runtime_id()
    }

    pub fn get_previous_session_handoff(&self) -> Option<String> {
        B::get_previous_session_handoff()
    }

    pub fn write(&self, section: &str, content: &str) -> Result<(), String> {
        B::write(section, content)
    }

    pub fn set_parent_session(&self, runtime_id: &str) -> Result<(), String> {
        B::set_parent_session(runtime_id)
    }

    pub fn create_tag(&self, name: &str) -> Result<(), String> {
        B::create_tag(name)
    }

    pub fn set_status(&self, status: &str) -> Result<(), String> {
        B::set_status(status)
    }

    pub fn write_handoff(&self, modified_files: &str, summary: &str) -> Result<(), String> {
        B::write_handoff(modified_files, summary)
    }
}
