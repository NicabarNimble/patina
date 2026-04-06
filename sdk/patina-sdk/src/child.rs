use crate::toys;
use crate::toys::{PendingEvent as ToyPendingEvent, TaskIntent as ToyTaskIntent};

#[cfg(target_arch = "wasm32")]
#[used]
#[link_section = ".patina_api_version"]
static API_VERSION: [u8; 3] = [0, 1, 0];

wit_bindgen::generate!({
    path: "wit/child",
    world: "child",
    skip: ["init"],
    generate_all,
});

pub use patina::child::runtime_types::HealthStatus;

use crate::wasm_cell::WasmCell;

pub mod granted {
    use super::host::GuestHost;
    use super::toys;

    #[cfg(feature = "toy-log")]
    pub type Log = toys::LogToy<GuestHost>;
    #[cfg(feature = "toy-measure")]
    pub type Measure = toys::MeasureToy<GuestHost>;
    #[cfg(feature = "toy-query")]
    pub type Query = toys::QueryToy<GuestHost>;
    #[cfg(feature = "toy-fetch")]
    pub type Fetch = toys::FetchToy<GuestHost>;
    #[cfg(feature = "toy-emit")]
    pub type Emit = toys::EmitToy<GuestHost>;
    #[cfg(feature = "toy-emit")]
    pub type Messaging = toys::MessagingToy<GuestHost>;
    #[cfg(feature = "toy-state")]
    pub type State = toys::StateToy<GuestHost>;
    #[cfg(feature = "toy-layer-fs")]
    pub type LayerFs = toys::LayerFsToy<GuestHost>;
    #[cfg(feature = "toy-checkpoint")]
    pub type Checkpoint = toys::CheckpointToy<GuestHost>;
    #[cfg(feature = "toy-lake")]
    pub type Lakes = toys::LakeCatalog<GuestHost>;
    #[cfg(feature = "toy-lake")]
    pub type Lake = toys::LakeToy<GuestHost>;
    #[cfg(feature = "toy-ingress")]
    pub type IngressSources = toys::IngressCatalog<GuestHost>;
    #[cfg(feature = "toy-ingress")]
    pub type Ingress = toys::IngressToy<GuestHost>;
    #[cfg(feature = "toy-connector")]
    pub type Connectors = toys::ConnectorCatalog<GuestHost>;
    #[cfg(feature = "toy-connector")]
    pub type Connector = toys::ConnectorBinding<GuestHost>;
    #[cfg(feature = "toy-github")]
    pub type Github = toys::GithubToy<GuestHost>;
    #[cfg(feature = "toy-events")]
    pub type Events = toys::EventToy<GuestHost>;
    #[cfg(feature = "toy-peer")]
    pub type Peer = toys::PeerToy<GuestHost>;
    #[cfg(feature = "toy-graph")]
    pub type Graph = toys::GraphToy<GuestHost>;
    #[cfg(feature = "toy-belief")]
    pub type Belief = toys::BeliefToy<GuestHost>;
    #[cfg(feature = "toy-session")]
    pub type Session = toys::SessionToy<GuestHost>;

    pub trait Bundle {
        fn granted() -> Self;
    }

    #[cfg(feature = "toy-log")]
    pub fn log() -> Log {
        Log::new()
    }

    #[cfg(feature = "toy-measure")]
    pub fn measure() -> Measure {
        Measure::new()
    }

    #[cfg(feature = "toy-query")]
    pub fn query() -> Query {
        Query::new()
    }

    #[cfg(feature = "toy-fetch")]
    pub fn fetch() -> Fetch {
        Fetch::new()
    }

    #[cfg(feature = "toy-emit")]
    pub fn emit() -> Emit {
        Emit::new()
    }

    #[cfg(feature = "toy-emit")]
    pub fn messaging() -> Messaging {
        Messaging::new()
    }

    #[cfg(feature = "toy-state")]
    pub fn state() -> State {
        State::new()
    }

    #[cfg(feature = "toy-layer-fs")]
    pub fn layer_fs() -> LayerFs {
        LayerFs::new()
    }

    #[cfg(feature = "toy-checkpoint")]
    pub fn checkpoint() -> Checkpoint {
        Checkpoint::new()
    }

    #[cfg(feature = "toy-lake")]
    pub fn lakes() -> Lakes {
        Lakes::new()
    }

    #[cfg(feature = "toy-lake")]
    pub fn lake(name: &str) -> Lake {
        lakes()
            .require(name)
            .unwrap_or_else(|error| panic!("missing granted lake '{}': {}", name, error))
    }

    #[cfg(feature = "toy-ingress")]
    pub fn ingress_sources() -> IngressSources {
        IngressSources::new()
    }

    #[cfg(feature = "toy-ingress")]
    pub fn ingress(name: &str) -> Ingress {
        ingress_sources()
            .require(name)
            .unwrap_or_else(|error| panic!("missing granted ingress source '{}': {}", name, error))
    }

    #[cfg(feature = "toy-connector")]
    pub fn connectors() -> Connectors {
        Connectors::new()
    }

    #[cfg(feature = "toy-connector")]
    pub fn connector(binding_id: &str) -> Connector {
        connectors().require(binding_id).unwrap_or_else(|error| {
            panic!(
                "missing granted connector binding '{}': {}",
                binding_id, error
            )
        })
    }

    #[cfg(feature = "toy-github")]
    pub fn github() -> Github {
        Github::new()
    }

    #[cfg(feature = "toy-events")]
    pub fn events() -> Events {
        Events::new()
    }

    #[cfg(feature = "toy-peer")]
    pub fn peer() -> Peer {
        Peer::new()
    }

    #[cfg(feature = "toy-graph")]
    pub fn graph() -> Graph {
        Graph::new()
    }

    #[cfg(feature = "toy-belief")]
    pub fn belief() -> Belief {
        Belief::new()
    }

    #[cfg(feature = "toy-session")]
    pub fn session() -> Session {
        Session::new()
    }
}

pub mod substrate {
    pub use crate::toys::{PendingEvent, TaskIntent, TaskIntentKind};

    pub fn enqueue(intent: &TaskIntent) -> Result<String, String> {
        super::host::GuestHost::enqueue_task(intent)
    }
}

pub trait Child {
    fn name(&self) -> String;

    fn on_load(&mut self) -> Result<(), String> {
        Ok(())
    }

    fn on_unload(&mut self) {}

    fn health(&self) -> ChildHealth {
        ChildHealth {
            status: HealthStatus::Healthy,
            reason: None,
        }
    }

    fn handle(&mut self, action: &str, payload: &str) -> Result<String, String>;

    fn drain(&mut self, _limit: u32) -> Result<Vec<ToyPendingEvent>, String> {
        Ok(vec![])
    }

    fn tick(&mut self) -> Vec<ToyTaskIntent> {
        vec![]
    }
}

#[doc(hidden)]
pub mod host {
    use super::patina;
    use crate::toys::{
        BeliefBackend, CheckpointBackend, ConnectorBackend, EmitBackend, EventBackend,
        FetchBackend, GithubBackend, GraphBackend, HttpMethod, HttpRequest, HttpResponse,
        IngressBackend, LakeBackend, LogBackend, LogLevel, MeasureBackend, MessagingBackend,
        MessagingMessage, PendingEvent, QueryBackend, SessionBackend, StateBackend,
        StateBucketBackend, TaskBackend, TaskIntent, TaskIntentKind,
    };
    #[cfg(feature = "toy-layer-fs")]
    use crate::toys::{
        LayerFsBackend, LayerFsDescriptorBackend, LayerFsDescriptorType, LayerFsDirectoryEntry,
        LayerFsDirectoryEntryStreamBackend,
    };
    #[cfg(feature = "toy-peer")]
    use crate::toys::{PeerBackend, PeerEvent};

    #[derive(Debug, Clone, Copy, Default)]
    pub struct GuestHost;

    #[derive(Debug)]
    pub struct GuestStateBucket(super::wasi::keyvalue::store::Bucket);
    #[derive(Debug)]
    pub struct GuestMessagingClient(super::wasi::messaging::producer::Client);
    #[cfg(feature = "toy-layer-fs")]
    #[derive(Debug)]
    pub struct GuestFsDescriptor(super::wasi::filesystem::types::Descriptor);
    #[cfg(feature = "toy-layer-fs")]
    #[derive(Debug)]
    pub struct GuestFsDirectoryStream(super::wasi::filesystem::types::DirectoryEntryStream);

    impl GuestHost {
        pub fn log() -> crate::toys::LogToy<Self> {
            crate::toys::LogToy::new()
        }
        pub fn measure() -> crate::toys::MeasureToy<Self> {
            crate::toys::MeasureToy::new()
        }
        pub fn query() -> crate::toys::QueryToy<Self> {
            crate::toys::QueryToy::new()
        }
        pub fn fetch() -> crate::toys::FetchToy<Self> {
            crate::toys::FetchToy::new()
        }
        pub fn emit() -> crate::toys::EmitToy<Self> {
            crate::toys::EmitToy::new()
        }
        pub fn messaging() -> crate::toys::MessagingToy<Self> {
            crate::toys::MessagingToy::new()
        }
        pub fn state() -> crate::toys::StateToy<Self> {
            crate::toys::StateToy::new()
        }
        #[cfg(feature = "toy-layer-fs")]
        pub fn layer_fs() -> crate::toys::LayerFsToy<Self> {
            crate::toys::LayerFsToy::new()
        }
        pub fn checkpoint() -> crate::toys::CheckpointToy<Self> {
            crate::toys::CheckpointToy::new()
        }
        pub fn lake(name: &str) -> crate::toys::LakeToy<Self> {
            crate::toys::LakeCatalog::<Self>::new()
                .require(name)
                .unwrap_or_else(|error| panic!("missing granted lake '{}': {}", name, error))
        }
        pub fn ingress(name: &str) -> crate::toys::IngressToy<Self> {
            crate::toys::IngressCatalog::<Self>::new()
                .require(name)
                .unwrap_or_else(|error| {
                    panic!("missing granted ingress source '{}': {}", name, error)
                })
        }
        pub fn events() -> crate::toys::EventToy<Self> {
            crate::toys::EventToy::new()
        }
        #[cfg(feature = "toy-peer")]
        pub fn peer() -> crate::toys::PeerToy<Self> {
            crate::toys::PeerToy::new()
        }
        pub fn github() -> crate::toys::GithubToy<Self> {
            crate::toys::GithubToy::new()
        }
        pub fn graph() -> crate::toys::GraphToy<Self> {
            crate::toys::GraphToy::new()
        }
        pub fn belief() -> crate::toys::BeliefToy<Self> {
            crate::toys::BeliefToy::new()
        }
        pub fn session() -> crate::toys::SessionToy<Self> {
            crate::toys::SessionToy::new()
        }
    }

    fn state_bucket(identifier: &str) -> Result<super::wasi::keyvalue::store::Bucket, String> {
        super::wasi::keyvalue::store::open(identifier)
    }

    fn state_get_string(key: &str) -> Option<String> {
        let bucket = state_bucket("default").ok()?;
        match bucket.get(key) {
            Ok(Some(bytes)) => String::from_utf8(bytes).ok(),
            Ok(None) => None,
            Err(_) => None,
        }
    }

    fn state_set_string(key: &str, value: &str) -> Result<(), String> {
        let bucket = state_bucket("default")?;
        bucket.set(key, value.as_bytes())
    }

    fn messaging_publish(
        stream_name: &str,
        event_type: &str,
        payload: &str,
    ) -> Result<u64, String> {
        let client = super::wasi::messaging::producer::connect(stream_name)?;
        let message = super::wasi::messaging::types::Message {
            topic: event_type.to_string(),
            content_type: Some("application/json".to_string()),
            data: payload.as_bytes().to_vec(),
            metadata: Vec::new(),
        };
        super::wasi::messaging::producer::send(&client, &message)
    }

    fn sql_query_string(connection: &str, query: &str) -> Result<String, String> {
        let conn = super::wasi::sql::readwrite::open(connection)?;
        let stmt = super::wasi::sql::readwrite::prepare(query, &[])?;
        let rows = super::wasi::sql::readwrite::query(&conn, &stmt)?;

        if let Some(first_row) = rows.first() {
            if let Some(super::wasi::sql::readwrite::DataType::Text(value)) =
                first_row.values.first()
            {
                return Ok(value.clone());
            }
        }

        let json_rows = rows
            .into_iter()
            .map(|row| {
                serde_json::Value::Array(
                    row.values
                        .into_iter()
                        .map(|value| match value {
                            super::wasi::sql::readwrite::DataType::Int32(value) => {
                                serde_json::json!(value)
                            }
                            super::wasi::sql::readwrite::DataType::Int64(value) => {
                                serde_json::json!(value)
                            }
                            super::wasi::sql::readwrite::DataType::Float64(value) => {
                                serde_json::json!(value)
                            }
                            super::wasi::sql::readwrite::DataType::Text(value) => {
                                serde_json::json!(value)
                            }
                            super::wasi::sql::readwrite::DataType::Flag(value) => {
                                serde_json::json!(value)
                            }
                            super::wasi::sql::readwrite::DataType::Binary(value) => {
                                serde_json::json!(value)
                            }
                            super::wasi::sql::readwrite::DataType::Null => serde_json::Value::Null,
                        })
                        .collect(),
                )
            })
            .collect::<Vec<_>>();

        serde_json::to_string(&json_rows).map_err(|error| error.to_string())
    }

    fn sql_exec_action(connection: &str, action: &str, payload: &str) -> Result<u32, String> {
        let conn = super::wasi::sql::readwrite::open(connection)?;
        let stmt = super::wasi::sql::readwrite::prepare(
            "__patina_mutate__",
            &[action.to_string(), payload.to_string()],
        )?;
        super::wasi::sql::readwrite::exec(&conn, &stmt)
    }

    fn default_authority_for_source(source: &str) -> String {
        match source {
            "github" => "api.github.com".to_string(),
            other => other.to_string(),
        }
    }

    fn http_method_to_wasi(method: &HttpMethod) -> super::wasi::http::types::Method {
        match method {
            HttpMethod::Get => super::wasi::http::types::Method::Get,
            HttpMethod::Head => super::wasi::http::types::Method::Head,
            HttpMethod::Post => super::wasi::http::types::Method::Post,
            HttpMethod::Put => super::wasi::http::types::Method::Put,
            HttpMethod::Delete => super::wasi::http::types::Method::Delete,
            HttpMethod::Connect => super::wasi::http::types::Method::Connect,
            HttpMethod::Options => super::wasi::http::types::Method::Options,
            HttpMethod::Trace => super::wasi::http::types::Method::Trace,
            HttpMethod::Patch => super::wasi::http::types::Method::Patch,
            HttpMethod::Other(value) => super::wasi::http::types::Method::Other(value.clone()),
        }
    }

    fn wasi_http_send_with_binding(
        source: &str,
        request: &HttpRequest,
    ) -> Result<HttpResponse, String> {
        let _binding = patina::connect::connect::resolve(source)?;

        let headers = super::wasi::http::types::Fields::from_list(
            &request
                .headers
                .iter()
                .map(|(name, value)| (name.clone(), value.clone()))
                .collect::<Vec<_>>(),
        )
        .map_err(|_| "failed to construct outgoing HTTP headers".to_string())?;

        let outgoing = super::wasi::http::types::OutgoingRequest::new(headers);
        outgoing
            .set_method(&http_method_to_wasi(&request.method))
            .map_err(|_| "failed to set HTTP method".to_string())?;
        let scheme = match request.scheme.as_deref().unwrap_or("https") {
            "http" => super::wasi::http::types::Scheme::Http,
            "https" => super::wasi::http::types::Scheme::Https,
            other => super::wasi::http::types::Scheme::Other(other.to_string()),
        };
        outgoing
            .set_scheme(Some(&scheme))
            .map_err(|_| "failed to set HTTP scheme".to_string())?;
        outgoing
            .set_authority(request.authority.as_deref())
            .map_err(|_| "failed to set HTTP authority".to_string())?;
        outgoing
            .set_path_with_query(request.path_with_query.as_deref())
            .map_err(|_| "failed to set HTTP path".to_string())?;

        let body = outgoing
            .body()
            .map_err(|_| "failed to open outgoing HTTP body".to_string())?;
        let stream = body
            .write()
            .map_err(|_| "failed to open outgoing HTTP stream".to_string())?;
        if !request.body.is_empty() {
            stream
                .blocking_write_and_flush(&request.body)
                .map_err(|error| format!("failed to write outgoing HTTP body: {:?}", error))?;
        }
        drop(stream);
        super::wasi::http::types::OutgoingBody::finish(body, None)
            .map_err(|error| format!("failed to finalize outgoing HTTP body: {:?}", error))?;

        let future = super::wasi::http::outgoing_handler::handle(outgoing, None)
            .map_err(|error| format!("HTTP request failed to start: {:?}", error))?;
        let pollable = future.subscribe();

        let response = loop {
            if let Some(result) = future.get() {
                let result = result.map_err(|_| "response future already consumed".to_string())?;
                break result.map_err(|error| format!("HTTP request failed: {:?}", error));
            }
            super::wasi::io::poll::poll(&[&pollable]);
        }?;

        let status = response.status();
        let incoming_body = response
            .consume()
            .map_err(|_| "incoming response body already consumed".to_string())?;
        let input_stream = incoming_body
            .stream()
            .map_err(|_| "failed to open incoming response stream".to_string())?;
        let mut bytes = Vec::new();

        loop {
            let chunk = input_stream
                .blocking_read(64 * 1024)
                .map_err(|error| format!("failed to read HTTP response body: {:?}", error))?;
            if chunk.is_empty() {
                break;
            }
            bytes.extend_from_slice(&chunk);
        }

        let _ = super::wasi::http::types::IncomingBody::finish(incoming_body);
        Ok(HttpResponse {
            status,
            headers: Vec::new(),
            body: bytes,
        })
    }

    fn wasi_http_get_with_binding(
        source: &str,
        authority: &str,
        path_with_query: &str,
    ) -> Result<String, String> {
        let response = wasi_http_send_with_binding(
            source,
            &HttpRequest {
                method: HttpMethod::Get,
                scheme: Some("https".to_string()),
                authority: Some(authority.to_string()),
                path_with_query: Some(path_with_query.to_string()),
                headers: Vec::new(),
                body: Vec::new(),
            },
        )?;
        String::from_utf8(response.body).map_err(|error| error.to_string())
    }

    impl LogBackend for GuestHost {
        fn log(level: LogLevel, context: &str, message: &str) {
            let level = match level {
                LogLevel::Trace => super::wasi::logging::logging::Level::Trace,
                LogLevel::Debug => super::wasi::logging::logging::Level::Debug,
                LogLevel::Info => super::wasi::logging::logging::Level::Info,
                LogLevel::Warn => super::wasi::logging::logging::Level::Warn,
                LogLevel::Error => super::wasi::logging::logging::Level::Error,
                LogLevel::Critical => super::wasi::logging::logging::Level::Critical,
            };
            super::wasi::logging::logging::log(level, context, message);
        }
    }

    impl MeasureBackend for GuestHost {
        fn record(verb: &str, tool: &str, mode: &str, metrics_json: &str) -> Result<(), String> {
            let payload = serde_json::json!({
                "verb": verb,
                "tool": tool,
                "mode": mode,
                "metrics": serde_json::from_str::<serde_json::Value>(metrics_json)
                    .unwrap_or(serde_json::Value::String(metrics_json.to_string())),
            })
            .to_string();
            let _ = messaging_publish("measure", "measurement", &payload)?;
            Ok(())
        }
    }

    impl QueryBackend for GuestHost {
        fn query(kind: &str, params_json: &str) -> Result<String, String> {
            sql_query_string("default", &format!("{kind}:{params_json}"))
        }
    }

    impl FetchBackend for GuestHost {
        fn send(request: &HttpRequest) -> Result<HttpResponse, String> {
            let authority = request
                .authority
                .as_deref()
                .ok_or_else(|| "http authority is required".to_string())?;
            let source = authority.split(':').next().unwrap_or(authority);
            wasi_http_send_with_binding(source, request)
        }
    }

    impl EmitBackend for GuestHost {
        fn emit(schema: &str, fact_type: &str, data: &str) -> Result<u64, String> {
            let payload = serde_json::json!({
                "schema": schema,
                "fact_type": fact_type,
                "data": serde_json::from_str::<serde_json::Value>(data)
                    .unwrap_or(serde_json::Value::String(data.to_string())),
            })
            .to_string();
            messaging_publish("fact.ingested", fact_type, &payload)
        }
    }

    impl MessagingBackend for GuestHost {
        type Client = GuestMessagingClient;

        fn connect(name: &str) -> Result<Self::Client, String> {
            let client = super::wasi::messaging::producer::connect(name)?;
            Ok(GuestMessagingClient(client))
        }

        fn send(client: &Self::Client, message: &MessagingMessage) -> Result<u64, String> {
            let msg = super::wasi::messaging::types::Message {
                topic: message.topic.clone(),
                content_type: message.content_type.clone(),
                data: message.data.clone(),
                metadata: message.metadata.clone(),
            };
            super::wasi::messaging::producer::send(&client.0, &msg)
        }
    }

    impl StateBackend for GuestHost {
        type Bucket = GuestStateBucket;

        fn open(identifier: &str) -> Result<Self::Bucket, String> {
            Ok(GuestStateBucket(state_bucket(identifier)?))
        }
    }

    impl StateBucketBackend for GuestStateBucket {
        fn get(&self, key: &str) -> Result<Option<Vec<u8>>, String> {
            self.0.get(key)
        }

        fn set(&self, key: &str, value: &[u8]) -> Result<(), String> {
            self.0.set(key, value)
        }

        fn delete(&self, key: &str) -> Result<(), String> {
            self.0.delete(key)
        }

        fn exists(&self, key: &str) -> Result<bool, String> {
            self.0.exists(key)
        }

        fn list_keys(&self, cursor: Option<&str>) -> Result<crate::toys::KeyResponse, String> {
            let response = self.0.list_keys(cursor)?;
            Ok(crate::toys::KeyResponse {
                keys: response.keys,
                cursor: response.cursor,
            })
        }
    }

    #[cfg(feature = "toy-layer-fs")]
    fn fs_error(error: super::wasi::filesystem::types::ErrorCode) -> String {
        format!("{:?}", error)
    }

    #[cfg(feature = "toy-layer-fs")]
    fn fs_descriptor_type(
        ty: super::wasi::filesystem::types::DescriptorType,
    ) -> LayerFsDescriptorType {
        use super::wasi::filesystem::types::DescriptorType;
        match ty {
            DescriptorType::Unknown => LayerFsDescriptorType::Unknown,
            DescriptorType::BlockDevice => LayerFsDescriptorType::BlockDevice,
            DescriptorType::CharacterDevice => LayerFsDescriptorType::CharacterDevice,
            DescriptorType::Directory => LayerFsDescriptorType::Directory,
            DescriptorType::Fifo => LayerFsDescriptorType::Fifo,
            DescriptorType::SymbolicLink => LayerFsDescriptorType::SymbolicLink,
            DescriptorType::RegularFile => LayerFsDescriptorType::RegularFile,
            DescriptorType::Socket => LayerFsDescriptorType::Socket,
        }
    }

    #[cfg(feature = "toy-layer-fs")]
    impl LayerFsBackend for GuestHost {
        type Descriptor = GuestFsDescriptor;

        fn get_directories() -> Result<Vec<(Self::Descriptor, String)>, String> {
            Ok(super::wasi::filesystem::preopens::get_directories()
                .into_iter()
                .map(|(descriptor, path)| (GuestFsDescriptor(descriptor), path))
                .collect())
        }
    }

    #[cfg(feature = "toy-layer-fs")]
    impl LayerFsDirectoryEntryStreamBackend for GuestFsDirectoryStream {
        fn read_directory_entry(&self) -> Result<Option<LayerFsDirectoryEntry>, String> {
            self.0
                .read_directory_entry()
                .map_err(fs_error)
                .map(|entry| {
                    entry.map(|entry| LayerFsDirectoryEntry {
                        name: entry.name,
                        descriptor_type: fs_descriptor_type(entry.type_),
                    })
                })
        }
    }

    #[cfg(feature = "toy-layer-fs")]
    impl LayerFsDescriptorBackend for GuestFsDescriptor {
        type DirectoryStream = GuestFsDirectoryStream;

        fn read(&self, length: u64, offset: u64) -> Result<(Vec<u8>, bool), String> {
            self.0.read(length, offset).map_err(fs_error)
        }

        fn write(&self, buffer: &[u8], offset: u64) -> Result<u64, String> {
            self.0.write(buffer, offset).map_err(fs_error)
        }

        fn read_directory(&self) -> Result<Self::DirectoryStream, String> {
            self.0
                .read_directory()
                .map(GuestFsDirectoryStream)
                .map_err(fs_error)
        }

        fn create_directory_at(&self, path: &str) -> Result<(), String> {
            self.0.create_directory_at(path).map_err(fs_error)
        }

        fn open_at(
            &self,
            path: &str,
            create: bool,
            directory: bool,
            truncate: bool,
            read: bool,
            write: bool,
        ) -> Result<Self, String> {
            let mut open_flags = super::wasi::filesystem::types::OpenFlags::empty();
            if create {
                open_flags |= super::wasi::filesystem::types::OpenFlags::CREATE;
            }
            if directory {
                open_flags |= super::wasi::filesystem::types::OpenFlags::DIRECTORY;
            }
            if truncate {
                open_flags |= super::wasi::filesystem::types::OpenFlags::TRUNCATE;
            }

            let mut descriptor_flags = super::wasi::filesystem::types::DescriptorFlags::empty();
            if read {
                descriptor_flags |= super::wasi::filesystem::types::DescriptorFlags::READ;
            }
            if write {
                descriptor_flags |= super::wasi::filesystem::types::DescriptorFlags::WRITE;
                descriptor_flags |=
                    super::wasi::filesystem::types::DescriptorFlags::MUTATE_DIRECTORY;
            }

            self.0
                .open_at(
                    super::wasi::filesystem::types::PathFlags::empty(),
                    path,
                    open_flags,
                    descriptor_flags,
                )
                .map(GuestFsDescriptor)
                .map_err(fs_error)
        }

        fn unlink_file_at(&self, path: &str) -> Result<(), String> {
            self.0.unlink_file_at(path).map_err(fs_error)
        }

        fn remove_directory_at(&self, path: &str) -> Result<(), String> {
            self.0.remove_directory_at(path).map_err(fs_error)
        }

        fn rename_at(
            &self,
            old_path: &str,
            new_descriptor: &Self,
            new_path: &str,
        ) -> Result<(), String> {
            self.0
                .rename_at(old_path, &new_descriptor.0, new_path)
                .map_err(fs_error)
        }
    }

    impl CheckpointBackend for GuestHost {
        fn load(stream: &str) -> Option<String> {
            state_get_string(&format!("checkpoint:{stream}"))
        }
        fn save(stream: &str, checkpoint_json: &str) -> Result<(), String> {
            state_set_string(&format!("checkpoint:{stream}"), checkpoint_json)
        }
    }

    impl LakeBackend for GuestHost {
        fn list_granted_lakes() -> Result<Vec<crate::toys::GrantedLake>, String> {
            Ok(vec![crate::toys::GrantedLake {
                name: "default".to_string(),
                path: "state://default".to_string(),
            }])
        }
        fn load_cursor(lake: &str, source: &str, data_type: &str) -> Option<String> {
            let key = format!("lake-cursor:{lake}:{source}:{data_type}");
            state_get_string(&key)
        }
        fn save_cursor(
            lake: &str,
            source: &str,
            data_type: &str,
            cursor: Option<&str>,
            written: u64,
            status: &str,
            last_error: Option<&str>,
        ) -> Result<(), String> {
            let key = format!("lake-cursor:{lake}:{source}:{data_type}");
            state_set_string(
                &key,
                &serde_json::json!({
                    "cursor": cursor,
                    "written": written,
                    "status": status,
                    "last_error": last_error,
                })
                .to_string(),
            )
        }
        fn ensure_table(lake: &str, table: &str) -> Result<(), String> {
            let payload = serde_json::json!({"table": table}).to_string();
            let _ = sql_exec_action(lake, "ensure-table", &payload)?;
            Ok(())
        }
        fn append_json_batch(
            lake: &str,
            table: &str,
            source: &str,
            rows_json: &[String],
        ) -> Result<u64, String> {
            let payload = serde_json::json!({
                "table": table,
                "source": source,
                "rows_json": rows_json,
            })
            .to_string();
            let inserted = sql_exec_action(lake, "append-json-batch", &payload)?;
            Ok(inserted as u64)
        }
        fn query_json(lake: &str, sql: &str) -> Result<String, String> {
            sql_query_string(lake, sql)
        }
    }

    impl IngressBackend for GuestHost {
        fn list_granted_sources() -> Vec<crate::toys::GrantedIngressSource> {
            match patina::connect::connect::resolve("github") {
                Ok(_) => vec![crate::toys::GrantedIngressSource {
                    name: "github".to_string(),
                    endpoint: "https://api.github.com".to_string(),
                }],
                Err(_) => vec![],
            }
        }

        fn fetch(source: &str) -> Result<String, String> {
            let authority = default_authority_for_source(source);
            wasi_http_get_with_binding(source, &authority, "/")
        }
    }

    impl ConnectorBackend for GuestHost {
        fn list_bindings() -> Result<Vec<crate::toys::GrantedConnectorBinding>, String> {
            Ok(vec![])
        }

        fn upsert_binding(
            binding: &crate::toys::GrantedConnectorBinding,
        ) -> Result<crate::toys::GrantedConnectorBinding, String> {
            Ok(binding.clone())
        }

        fn remove_binding(binding_id: &str) -> Result<(), String> {
            let _ = binding_id;
            Ok(())
        }

        fn sync_binding(
            binding_id: &str,
            data_type: &str,
            since: Option<&str>,
        ) -> Result<crate::toys::ConnectorSyncResult, String> {
            let _ = since;
            Ok(crate::toys::ConnectorSyncResult {
                binding_id: binding_id.to_string(),
                data_type: data_type.to_string(),
                cursor: None,
                rows_json: vec![],
            })
        }
    }

    impl GithubBackend for GuestHost {
        fn list_issues(
            owner: &str,
            repo: &str,
            params: &crate::toys::GithubListParams,
        ) -> Result<crate::toys::GithubPage, String> {
            let mut path = format!(
                "/repos/{owner}/{repo}/issues?state={}",
                params.state.clone().unwrap_or_else(|| "open".to_string())
            );
            if let Some(since) = &params.since {
                path.push_str(&format!("&since={since}"));
            }
            if let Some(page) = params.page {
                path.push_str(&format!("&page={page}"));
            }
            if let Some(per_page) = params.per_page {
                path.push_str(&format!("&per_page={per_page}"));
            }
            let items = wasi_http_get_with_binding("github", "api.github.com", &path)?;
            Ok(crate::toys::GithubPage {
                items,
                has_next: false,
                next_page: None,
                rate_remaining: 0,
            })
        }

        fn list_pulls(
            owner: &str,
            repo: &str,
            params: &crate::toys::GithubListParams,
        ) -> Result<crate::toys::GithubPage, String> {
            let mut path = format!(
                "/repos/{owner}/{repo}/pulls?state={}",
                params.state.clone().unwrap_or_else(|| "open".to_string())
            );
            if let Some(since) = &params.since {
                path.push_str(&format!("&since={since}"));
            }
            if let Some(page) = params.page {
                path.push_str(&format!("&page={page}"));
            }
            if let Some(per_page) = params.per_page {
                path.push_str(&format!("&per_page={per_page}"));
            }
            let items = wasi_http_get_with_binding("github", "api.github.com", &path)?;
            Ok(crate::toys::GithubPage {
                items,
                has_next: false,
                next_page: None,
                rate_remaining: 0,
            })
        }

        fn list_issue_comments(
            owner: &str,
            repo: &str,
            issue_number: u32,
        ) -> Result<crate::toys::GithubPage, String> {
            let path = format!("/repos/{owner}/{repo}/issues/{issue_number}/comments");
            Ok(crate::toys::GithubPage {
                items: wasi_http_get_with_binding("github", "api.github.com", &path)?,
                has_next: false,
                next_page: None,
                rate_remaining: 0,
            })
        }

        fn list_issue_events(
            owner: &str,
            repo: &str,
            issue_number: u32,
        ) -> Result<crate::toys::GithubPage, String> {
            let path = format!("/repos/{owner}/{repo}/issues/{issue_number}/events");
            Ok(crate::toys::GithubPage {
                items: wasi_http_get_with_binding("github", "api.github.com", &path)?,
                has_next: false,
                next_page: None,
                rate_remaining: 0,
            })
        }

        fn list_pull_comments(
            owner: &str,
            repo: &str,
            pull_number: u32,
        ) -> Result<crate::toys::GithubPage, String> {
            let path = format!("/repos/{owner}/{repo}/pulls/{pull_number}/comments");
            Ok(crate::toys::GithubPage {
                items: wasi_http_get_with_binding("github", "api.github.com", &path)?,
                has_next: false,
                next_page: None,
                rate_remaining: 0,
            })
        }

        fn list_reviews(
            owner: &str,
            repo: &str,
            pull_number: u32,
        ) -> Result<crate::toys::GithubPage, String> {
            let path = format!("/repos/{owner}/{repo}/pulls/{pull_number}/reviews");
            Ok(crate::toys::GithubPage {
                items: wasi_http_get_with_binding("github", "api.github.com", &path)?,
                has_next: false,
                next_page: None,
                rate_remaining: 0,
            })
        }

        fn list_review_comments(
            owner: &str,
            repo: &str,
            pull_number: u32,
            review_id: u64,
        ) -> Result<crate::toys::GithubPage, String> {
            let path =
                format!("/repos/{owner}/{repo}/pulls/{pull_number}/reviews/{review_id}/comments");
            Ok(crate::toys::GithubPage {
                items: wasi_http_get_with_binding("github", "api.github.com", &path)?,
                has_next: false,
                next_page: None,
                rate_remaining: 0,
            })
        }
    }

    impl SessionBackend for GuestHost {
        fn get_session_id() -> String {
            state_get_string("session-id").unwrap_or_else(|| "unknown".to_string())
        }

        fn get_previous_session() -> Option<String> {
            state_get_string("previous-session")
        }

        fn get_previous_session_runtime_id() -> Option<String> {
            state_get_string("parent-session-runtime")
        }

        fn get_previous_session_handoff() -> Option<String> {
            state_get_string("parent-handoff")
        }

        fn write(section: &str, content: &str) -> Result<(), String> {
            state_set_string(&format!("session:{section}"), content)
        }

        fn set_parent_session(runtime_id: &str) -> Result<(), String> {
            state_set_string("parent-session-runtime", runtime_id)
        }

        fn create_tag(name: &str) -> Result<(), String> {
            patina::git::git::create_tag(name)
        }

        fn set_status(status: &str) -> Result<(), String> {
            state_set_string("session-status", status)
        }

        fn write_handoff(modified_files: &str, summary: &str) -> Result<(), String> {
            state_set_string(
                "session-handoff",
                &serde_json::json!({"modified_files": modified_files, "summary": summary})
                    .to_string(),
            )
        }
    }

    impl EventBackend for GuestHost {
        fn pull(
            stream: &str,
            after_offset: Option<u64>,
            limit: u32,
        ) -> Result<Vec<PendingEvent>, String> {
            patina::events_stream::events_stream::subscribe(stream, after_offset, limit).map(
                |events| {
                    events
                        .into_iter()
                        .map(|event| PendingEvent {
                            stream_name: event.stream_name,
                            offset: event.offset,
                            event_type: event.event_type,
                            payload_json: event.payload,
                            occurred_at: event.occurred_at,
                        })
                        .collect()
                },
            )
        }
        fn ack_through(stream: &str, offset: u64) -> Result<(), String> {
            patina::events_stream::events_stream::ack(stream, offset)
        }
        fn list_streams() -> Vec<String> {
            vec![
                "belief.changed".to_string(),
                "graph.changed".to_string(),
                "fact.ingested".to_string(),
                "session.completed".to_string(),
                "repo.synced".to_string(),
                "file.found".to_string(),
                "file.written".to_string(),
            ]
        }
    }

    #[cfg(feature = "toy-peer")]
    impl PeerBackend for GuestHost {
        fn emit_event(event_type: &str, payload_json: &str) -> Result<(), String> {
            let _ = messaging_publish(event_type, event_type, payload_json)?;
            Ok(())
        }

        fn on_event(
            stream_name: &str,
            after_offset: Option<u64>,
            limit: u32,
        ) -> Result<Vec<PeerEvent>, String> {
            patina::events_stream::events_stream::subscribe(stream_name, after_offset, limit).map(
                |events| {
                    events
                        .into_iter()
                        .map(|event| PeerEvent {
                            stream_name: event.stream_name,
                            offset: event.offset,
                            event_type: event.event_type,
                            payload_json: event.payload,
                            occurred_at: event.occurred_at,
                        })
                        .collect()
                },
            )
        }
    }

    impl TaskBackend for GuestHost {
        fn enqueue(intent: &TaskIntent) -> Result<String, String> {
            let kind = match intent.kind {
                TaskIntentKind::FetchSource => "fetch-source",
                TaskIntentKind::RunQuery => "run-query",
                TaskIntentKind::EmitFacts => "emit-facts",
                TaskIntentKind::MaterializeIndex => "materialize-index",
                TaskIntentKind::VerifyBelief => "verify-belief",
                TaskIntentKind::SyncGraph => "sync-graph",
                TaskIntentKind::RefreshCredential => "refresh-credential",
                TaskIntentKind::NativeJob => "native-job",
            };
            patina::task::task::enqueue(kind, &intent.payload_json, intent.dedupe_key.as_deref())
        }
    }

    impl GuestHost {
        pub fn enqueue_task(intent: &TaskIntent) -> Result<String, String> {
            <Self as TaskBackend>::enqueue(intent)
        }
    }

    impl GraphBackend for GuestHost {
        fn query(kind: &str, params_json: &str) -> Result<String, String> {
            sql_query_string("graph", &format!("{kind}:{params_json}"))
        }
        fn mutate(action: &str, payload_json: &str) -> Result<(), String> {
            let _ = sql_exec_action("graph", action, payload_json)?;
            Ok(())
        }
    }

    impl BeliefBackend for GuestHost {
        fn query(kind: &str, params_json: &str) -> Result<String, String> {
            sql_query_string("beliefs", &format!("{kind}:{params_json}"))
        }
        fn mutate(action: &str, payload_json: &str) -> Result<(), String> {
            let _ = sql_exec_action("beliefs", action, payload_json)?;
            Ok(())
        }
    }
}

static PLUGIN: WasmCell<Option<Box<dyn Child>>> = WasmCell(std::cell::UnsafeCell::new(None));

#[doc(hidden)]
pub fn __register_child(child: Box<dyn Child>) {
    unsafe {
        *PLUGIN.0.get() = Some(child);
    }
}

#[doc(hidden)]
pub fn __register_plugin(plugin: Box<dyn Child>) {
    __register_child(plugin);
}

#[cfg(target_arch = "wasm32")]
mod __wasm {
    use super::*;

    fn child() -> &'static mut dyn Child {
        unsafe {
            (*PLUGIN.0.get())
                .as_deref_mut()
                .expect("child not initialized — host must call init first")
        }
    }

    struct Component;

    impl Guest for Component {
        fn name() -> String {
            child().name()
        }

        fn on_load() -> Result<(), String> {
            child().on_load()
        }

        fn on_unload() {
            child().on_unload()
        }

        fn health() -> ChildHealth {
            child().health()
        }

        fn handle(action: String, payload: String) -> Result<String, String> {
            child().handle(&action, &payload)
        }

        fn drain(limit: u32) -> Result<Vec<PendingEvent>, String> {
            child().drain(limit).map(|events| {
                events
                    .into_iter()
                    .map(|event| patina::child::runtime_types::PendingEvent {
                        stream_name: event.stream_name,
                        offset: event.offset,
                        event_type: event.event_type,
                        payload_json: event.payload_json,
                        occurred_at: event.occurred_at,
                    })
                    .collect()
            })
        }

        fn tick() -> Vec<patina::child::runtime_types::TaskIntent> {
            child()
                .tick()
                .into_iter()
                .map(|intent| patina::child::runtime_types::TaskIntent {
                    kind: match intent.kind {
                        crate::toys::TaskIntentKind::FetchSource => {
                            patina::child::runtime_types::TaskIntentKind::FetchSource
                        }
                        crate::toys::TaskIntentKind::RunQuery => {
                            patina::child::runtime_types::TaskIntentKind::RunQuery
                        }
                        crate::toys::TaskIntentKind::EmitFacts => {
                            patina::child::runtime_types::TaskIntentKind::EmitFacts
                        }
                        crate::toys::TaskIntentKind::MaterializeIndex => {
                            patina::child::runtime_types::TaskIntentKind::MaterializeIndex
                        }
                        crate::toys::TaskIntentKind::VerifyBelief => {
                            patina::child::runtime_types::TaskIntentKind::VerifyBelief
                        }
                        crate::toys::TaskIntentKind::SyncGraph => {
                            patina::child::runtime_types::TaskIntentKind::SyncGraph
                        }
                        crate::toys::TaskIntentKind::RefreshCredential => {
                            patina::child::runtime_types::TaskIntentKind::RefreshCredential
                        }
                        crate::toys::TaskIntentKind::NativeJob => {
                            patina::child::runtime_types::TaskIntentKind::NativeJob
                        }
                    },
                    payload_json: intent.payload_json,
                    dedupe_key: intent.dedupe_key,
                })
                .collect()
        }
    }

    export!(Component);
}

#[macro_export]
macro_rules! register_child {
    ($plugin_type:ty) => {
        #[export_name = "init"]
        extern "C" fn __patina_plugin_init() {
            $crate::child::__register_child(Box::new(<$plugin_type>::default()));
        }
    };
}

#[macro_export]
// MIGRATION-SHIM: remove in v0.47.0
#[deprecated(since = "0.46.0", note = "use register_child!")]
macro_rules! register_knowledge_child {
    ($plugin_type:ty) => {
        $crate::register_child!($plugin_type);
    };
}

pub use Child as ChildPlugin;
// MIGRATION-SHIM: remove in v0.47.0
#[deprecated(since = "0.46.0", note = "use Child")]
pub use Child as KnowledgeChild;
// MIGRATION-SHIM: remove in v0.47.0
#[deprecated(since = "0.46.0", note = "use ChildPlugin")]
pub use ChildPlugin as KnowledgeChildPlugin;
