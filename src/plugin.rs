//! Agent host plugin discovery (OpenClaw SKILL.md, Hermes JSON).

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgentPluginKind {
    OpenClawSkill,
    Hermes,
    RsGbrainBuiltin,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentPluginSpec {
    pub id: String,
    pub kind: AgentPluginKind,
    pub path: PathBuf,
    pub name: Option<String>,
    pub description: Option<String>,
}

pub fn discover_agent_plugins(roots: &[PathBuf]) -> Vec<AgentPluginSpec> {
    let mut out = Vec::new();
    for root in roots {
        scan_root(root, &mut out);
    }
    out.sort_by(|a, b| a.id.cmp(&b.id));
    out.dedup_by(|a, b| a.path == b.path);
    out
}

fn scan_root(root: &Path, out: &mut Vec<AgentPluginSpec>) {
    if !root.is_dir() {
        return;
    }
    let openclaw = root.join("plugins/openclaw/SKILL.md");
    if openclaw.is_file() {
        out.push(parse_openclaw_skill(&openclaw));
    }
    let hermes = root.join("plugins/hermes/plugin.json");
    if hermes.is_file() {
        if let Ok(spec) = parse_hermes(&hermes) {
            out.push(spec);
        }
    }
    if let Ok(entries) = std::fs::read_dir(root.join("plugins")) {
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                let skill = p.join("SKILL.md");
                if skill.is_file() {
                    out.push(parse_openclaw_skill(&skill));
                }
                let hj = p.join("plugin.json");
                if hj.is_file() {
                    if let Ok(spec) = parse_hermes(&hj) {
                        out.push(spec);
                    }
                }
            }
        }
    }
}

fn parse_openclaw_skill(path: &Path) -> AgentPluginSpec {
    let (name, description) = read_skill_frontmatter(path);
    AgentPluginSpec {
        id: format!("openclaw:{}", path.display()),
        kind: AgentPluginKind::OpenClawSkill,
        path: path.to_path_buf(),
        name,
        description,
    }
}

fn read_skill_frontmatter(path: &Path) -> (Option<String>, Option<String>) {
    let Ok(body) = std::fs::read_to_string(path) else {
        return (None, None);
    };
    if !body.starts_with("---") {
        return (None, None);
    }
    let mut name = None;
    let mut desc = None;
    for line in body.lines().skip(1) {
        if line.trim() == "---" {
            break;
        }
        if let Some(v) = line.strip_prefix("name:") {
            name = Some(v.trim().to_string());
        }
        if let Some(v) = line.strip_prefix("description:") {
            desc = Some(v.trim().to_string());
        }
    }
    (name, desc)
}

#[derive(Debug, Deserialize)]
struct HermesManifest {
    id: String,
    name: Option<String>,
    description: Option<String>,
}

fn parse_hermes(path: &Path) -> Result<AgentPluginSpec> {
    let raw = std::fs::read_to_string(path)?;
    let m: HermesManifest = serde_json::from_str(&raw)?;
    Ok(AgentPluginSpec {
        id: format!("hermes:{}", m.id),
        kind: AgentPluginKind::Hermes,
        path: path.to_path_buf(),
        name: m.name,
        description: m.description,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    #[test]
    fn discovers_builtin_openclaw_skill() {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let plugins = discover_agent_plugins(&[manifest_dir]);
        assert!(plugins
            .iter()
            .any(|p| p.kind == AgentPluginKind::OpenClawSkill));
    }
}
