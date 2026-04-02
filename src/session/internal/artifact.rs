use anyhow::Result;
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

use crate::session::{InterfaceKind, SessionParticipant};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionGitEnvelope {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_uid: Option<String>,
    pub branch: String,
    pub starting_commit: String,
    pub start_tag: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_tag: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactParticipant {
    pub id: String,
    pub role: String,
    pub interface: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub adapter: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    pub joined_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub left_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionFrontmatter {
    pub r#type: String,
    pub id: String,
    pub runtime_id: String,
    pub title: String,
    pub status: String,
    pub llm: String,
    pub interface: String,
    pub created: String,
    pub updated: String,
    pub start_timestamp: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub persona: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub participants: Vec<ArtifactParticipant>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub interfaces: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_session: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub handoff_from: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub handoff_to: Vec<String>,
    pub git: SessionGitEnvelope,
}

#[derive(Debug, Clone)]
pub struct SessionDocument {
    pub frontmatter: SessionFrontmatter,
    pub body: String,
}

#[derive(Debug, Clone)]
pub struct NewDocument {
    pub file_id: String,
    pub runtime_id: String,
    pub title: String,
    pub adapter_name: String,
    pub interface_kind: InterfaceKind,
    pub persona_uid: Option<String>,
    pub project_uid: String,
    pub branch: String,
    pub starting_commit: String,
    pub start_tag: String,
    pub parent_session: Option<String>,
    pub handoff_from: Option<String>,
    pub participants: Vec<SessionParticipant>,
    pub created_at: String,
    pub created_local: DateTime<Local>,
}

impl SessionDocument {
    pub fn render(&self) -> Result<String> {
        let yaml = serde_yaml::to_string(&self.frontmatter)?;
        Ok(format!("---\n{yaml}---\n\n{}", self.body))
    }
}

pub fn initial_document(request: NewDocument) -> Result<SessionDocument> {
    let time_str = request.created_local.format("%H:%M").to_string();
    let participants = request
        .participants
        .into_iter()
        .map(|participant| ArtifactParticipant {
            id: participant.participant_id,
            role: participant.role,
            interface: participant.interface_kind.as_str().to_string(),
            adapter: participant.adapter_name,
            display_name: participant.display_name,
            joined_at: request.created_at.clone(),
            left_at: None,
        })
        .collect::<Vec<_>>();

    let frontmatter = SessionFrontmatter {
        r#type: "session".to_string(),
        id: request.file_id,
        runtime_id: request.runtime_id,
        title: request.title.clone(),
        status: "active".to_string(),
        llm: request.adapter_name.clone(),
        interface: request.interface_kind.as_str().to_string(),
        created: request.created_at.clone(),
        updated: request.created_at.clone(),
        start_timestamp: request.created_local.timestamp_millis(),
        persona: request.persona_uid,
        interfaces: vec![request.interface_kind.as_str().to_string()],
        participants,
        parent_session: request.parent_session,
        handoff_from: request.handoff_from,
        handoff_to: Vec::new(),
        git: SessionGitEnvelope {
            project_uid: Some(request.project_uid),
            branch: request.branch.clone(),
            starting_commit: request.starting_commit.clone(),
            start_tag: request.start_tag.clone(),
            end_tag: None,
        },
    };

    let body = format!(
        "## Previous Session Context\n\
         <!-- AI: Summarize the last session from last-session.md -->\n\n\
         ## Goals\n\
         - [ ] {title}\n\n\
         ## Activity Log\n\
         ### {time_str} - Session Start\n\
         Session initialized with goal: {title}\n\
         Working on branch: {branch}\n\
         Tagged as: {start_tag}\n\n\
         ## Decisions\n\n\
         ## Evidence\n\n\
         ## Handoff\n\n\
         ## Outcome\n",
        title = request.title,
        branch = request.branch,
        start_tag = request.start_tag
    );

    Ok(SessionDocument { frontmatter, body })
}

pub fn parse_document(content: &str) -> Option<SessionDocument> {
    let rest = content.strip_prefix("---")?;
    let end = rest.find("\n---")?;
    let yaml_str = &rest[..end];
    let body = rest[end + 4..].trim_start_matches('\n').to_string();
    let frontmatter = serde_yaml::from_str::<SessionFrontmatter>(yaml_str).ok()?;
    Some(SessionDocument { frontmatter, body })
}

pub fn parse_session_ids(content: &str) -> Option<(String, String)> {
    let doc = parse_document(content)?;
    Some((doc.frontmatter.runtime_id, doc.frontmatter.id))
}

pub fn rewrite_document_status(
    content: &str,
    status: &str,
    updated_at: &str,
    end_tag: Option<&str>,
) -> Result<String> {
    let mut doc = parse_document(content)
        .ok_or_else(|| anyhow::anyhow!("session document is missing YAML frontmatter"))?;
    doc.frontmatter.status = status.to_string();
    doc.frontmatter.updated = updated_at.to_string();
    if let Some(tag) = end_tag {
        doc.frontmatter.git.end_tag = Some(tag.to_string());
    }
    doc.render()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Local, TimeZone};

    #[test]
    fn test_parse_and_rewrite_status() {
        let created_local = Local.with_ymd_and_hms(2026, 3, 11, 9, 45, 0).unwrap();
        let doc = initial_document(NewDocument {
            file_id: "20260311-094500-ABCD".to_string(),
            runtime_id: uuid::Uuid::new_v4().to_string(),
            title: "Build session layer".to_string(),
            adapter_name: "opencode".to_string(),
            interface_kind: InterfaceKind::OpenCode,
            persona_uid: None,
            project_uid: "proj1234".to_string(),
            branch: "patina".to_string(),
            starting_commit: "abc123".to_string(),
            start_tag: "session-20260311-094500-ABCD-opencode-start".to_string(),
            parent_session: None,
            handoff_from: None,
            participants: Vec::new(),
            created_at: "2026-03-11T13:45:00Z".to_string(),
            created_local,
        })
        .unwrap();

        let rendered = doc.render().unwrap();
        let rewritten = rewrite_document_status(
            &rendered,
            "archived",
            "2026-03-11T14:00:00Z",
            Some("session-20260311-094500-ABCD-opencode-end"),
        )
        .unwrap();
        let parsed = parse_document(&rewritten).unwrap();
        assert_eq!(parsed.frontmatter.status, "archived");
        assert_eq!(
            parsed.frontmatter.git.end_tag.as_deref(),
            Some("session-20260311-094500-ABCD-opencode-end")
        );
    }
}
