use anyhow::Result;

use crate::state::{
    InterfaceKindId, MotherSessionParticipant, MotherSessionRecord, MotherSessionStatus,
    PersonaUid, ProjectUid,
};
use crate::KnowledgeRuntimeStore;

#[derive(Clone)]
pub struct SessionStateService {
    store: KnowledgeRuntimeStore,
}

impl SessionStateService {
    pub fn new(store: KnowledgeRuntimeStore) -> Self {
        Self { store }
    }

    pub fn create(
        &self,
        record: &MotherSessionRecord,
        participants: &[MotherSessionParticipant],
    ) -> Result<()> {
        self.store.create_mother_session(record, participants)
    }

    pub fn get(&self, runtime_id: &str) -> Result<Option<MotherSessionRecord>> {
        self.store.get_mother_session(runtime_id)
    }

    pub fn get_by_file_id(&self, file_id: &str) -> Result<Option<MotherSessionRecord>> {
        self.store.get_mother_session_by_file_id(file_id)
    }

    pub fn list_active(&self, project_uid: &ProjectUid) -> Result<Vec<MotherSessionRecord>> {
        self.store.list_active_mother_sessions(project_uid)
    }

    pub fn find_active_for_interface(
        &self,
        project_uid: &ProjectUid,
        interface_name: &str,
        interface_kind: &InterfaceKindId,
        persona_uid: Option<&PersonaUid>,
    ) -> Result<Option<MotherSessionRecord>> {
        self.store.find_active_mother_session_for_interface(
            project_uid,
            interface_name,
            interface_kind,
            persona_uid,
        )
    }

    pub fn touch(&self, runtime_id: &str, updated_at: &str) -> Result<()> {
        self.store.touch_mother_session(runtime_id, updated_at)
    }

    pub fn finish(
        &self,
        runtime_id: &str,
        status: MotherSessionStatus,
        end_tag: Option<&str>,
        updated_at: &str,
    ) -> Result<()> {
        self.store
            .finish_mother_session(runtime_id, status, end_tag, updated_at)
    }
}
