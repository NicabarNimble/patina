use super::{AuthPlan, ConnectionRecord, ResolvedCredential};

pub fn require_auth_plan_domain(
    connection_name: &str,
    auth_plan: &AuthPlan,
    domain: &str,
) -> Result<ResolvedCredential, String> {
    let credential = auth_plan
        .credential
        .clone()
        .ok_or_else(|| format!("connection '{}' has no credential", connection_name))?;
    if !auth_plan
        .allowed_domains
        .iter()
        .any(|allowed| allowed == domain)
    {
        return Err(format!(
            "connection '{}' is not allowed for {}",
            connection_name, domain
        ));
    }
    Ok(credential)
}

pub fn resolve_credential_for_domain(
    record: &ConnectionRecord,
    domain: &str,
) -> Result<ResolvedCredential, String> {
    let auth_plan = crate::connect::resolve_auth(record)
        .map_err(|e| format!("connection '{}': {}", record.identity.name, e))?;
    require_auth_plan_domain(&record.identity.name, &auth_plan, domain)
}
