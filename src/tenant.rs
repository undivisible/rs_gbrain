//! Toy multi-tenant scope (not Postgres RLS — filter by `tenant_id` column).

use std::env;

pub const DEFAULT_TENANT: &str = "default";

pub fn tenant_from_env() -> String {
    env::var("RS_GBRAIN_TENANT").unwrap_or_else(|_| DEFAULT_TENANT.to_string())
}

pub fn normalize_tenant(id: &str) -> String {
    let t = id.trim();
    if t.is_empty() {
        DEFAULT_TENANT.to_string()
    } else {
        t.to_string()
    }
}

#[derive(Debug, Clone)]
pub struct TenantScope {
    pub tenant_id: String,
}

impl TenantScope {
    pub fn new(tenant_id: impl Into<String>) -> Self {
        Self {
            tenant_id: normalize_tenant(&tenant_id.into()),
        }
    }

    pub fn from_env() -> Self {
        Self::new(tenant_from_env())
    }
}
