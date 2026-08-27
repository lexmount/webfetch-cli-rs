use std::{
    collections::HashMap,
    env,
    ffi::OsString,
    fs,
    path::{Component, Path, PathBuf},
};

use serde::Serialize;

pub const SKILL_NAME: &str = "lexmount-webfetch";

/// The exact file manifest shipped in the SkillHub ZIP.
///
/// Platform binaries are downloaded after installation and deliberately do not
/// belong in the ZIP manifest.
pub const SKILL_ARCHIVE_FILES: [&str; 8] = [
    "SKILL.md",
    "references/authentication.md",
    "references/commands.md",
    "references/troubleshooting.md",
    "scripts/bootstrap.ps1",
    "scripts/bootstrap.sh",
    "scripts/doctor.ps1",
    "scripts/doctor.sh",
];

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DiscoveryInputs {
    pub home_dir: Option<PathBuf>,
    pub codex_home: Option<PathBuf>,
    pub claude_skill_dir: Option<PathBuf>,
    pub claude_config_dir: Option<PathBuf>,
    pub codebuddy_skill_dir: Option<PathBuf>,
    pub current_exe: Option<PathBuf>,
}

impl DiscoveryInputs {
    pub fn from_process() -> Self {
        Self {
            home_dir: dirs::home_dir(),
            codex_home: env_path("CODEX_HOME"),
            claude_skill_dir: env_path("CLAUDE_SKILL_DIR"),
            claude_config_dir: env_path("CLAUDE_CONFIG_DIR"),
            codebuddy_skill_dir: env_path("CODEBUDDY_SKILL_DIR"),
            current_exe: env::current_exe().ok(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SkillRootSource {
    CodexAgentsHome,
    ClaudeSkillDir,
    ClaudeConfigDir,
    ClaudeDefaultHome,
    WorkBuddySkillDir,
    CodexHome,
    CodexLegacyHome,
    CurrentExecutable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SkillManifestStatus {
    pub complete: bool,
    pub present_files: Vec<String>,
    pub missing_files: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SkillRootCandidate {
    pub root: PathBuf,
    pub sources: Vec<SkillRootSource>,
    pub manifest: SkillManifestStatus,
}

impl SkillRootCandidate {
    pub fn is_installed(&self) -> bool {
        self.manifest.complete
    }
}

/// Discover every supported Agent Skill candidate from the current process.
pub fn discover_skill_roots_from_process() -> Vec<SkillRootCandidate> {
    discover_skill_roots(&DiscoveryInputs::from_process())
}

/// Discover supported Agent Skill roots while keeping all process state
/// injectable for deterministic tests.
///
/// Candidates retain their first-seen order. If multiple hosts or compatibility
/// paths resolve to the same root, the root is returned once with every source
/// recorded in `sources`.
pub fn discover_skill_roots(inputs: &DiscoveryInputs) -> Vec<SkillRootCandidate> {
    let mut candidates = Vec::new();
    let mut indices = HashMap::<PathBuf, usize>::new();

    if let Some(home) = non_empty_path(inputs.home_dir.as_ref()) {
        push_candidate(
            &mut candidates,
            &mut indices,
            home.join(".agents/skills").join(SKILL_NAME),
            SkillRootSource::CodexAgentsHome,
        );
    }

    if let Some(root) = non_empty_path(inputs.claude_skill_dir.as_ref()) {
        push_candidate(
            &mut candidates,
            &mut indices,
            root.to_path_buf(),
            SkillRootSource::ClaudeSkillDir,
        );
    }

    let claude_config = non_empty_path(inputs.claude_config_dir.as_ref())
        .map(|path| (path.to_path_buf(), SkillRootSource::ClaudeConfigDir))
        .or_else(|| {
            non_empty_path(inputs.home_dir.as_ref())
                .map(|home| (home.join(".claude"), SkillRootSource::ClaudeDefaultHome))
        });
    if let Some((config_dir, source)) = claude_config {
        push_candidate(
            &mut candidates,
            &mut indices,
            config_dir.join("skills").join(SKILL_NAME),
            source,
        );
    }

    if let Some(root) = non_empty_path(inputs.codebuddy_skill_dir.as_ref()) {
        push_candidate(
            &mut candidates,
            &mut indices,
            root.to_path_buf(),
            SkillRootSource::WorkBuddySkillDir,
        );
    }

    if let Some(codex_home) = non_empty_path(inputs.codex_home.as_ref()) {
        push_candidate(
            &mut candidates,
            &mut indices,
            codex_home.join("skills").join(SKILL_NAME),
            SkillRootSource::CodexHome,
        );
    }

    if let Some(home) = non_empty_path(inputs.home_dir.as_ref()) {
        push_candidate(
            &mut candidates,
            &mut indices,
            home.join(".codex/skills").join(SKILL_NAME),
            SkillRootSource::CodexLegacyHome,
        );
    }

    if let Some(executable) = non_empty_path(inputs.current_exe.as_ref())
        && let Some(root) = skill_root_from_executable(executable)
    {
        push_candidate(
            &mut candidates,
            &mut indices,
            root,
            SkillRootSource::CurrentExecutable,
        );
    }

    candidates
}

/// Infer a Skill root only from a Skill-local executable layout:
/// `<skill-root>/bin/webfetch-cli[.exe]`. A root is recognized when it uses the
/// canonical Skill directory name or contains `SKILL.md`; this avoids treating
/// a global `/usr/local/bin`-style install as active while allowing hosts to
/// rename an installed Skill directory.
pub fn skill_root_from_executable(executable: &Path) -> Option<PathBuf> {
    let file_name = executable.file_name()?.to_string_lossy();
    if file_name != "webfetch-cli" && !file_name.eq_ignore_ascii_case("webfetch-cli.exe") {
        return None;
    }

    let bin_dir = executable.parent()?;
    if !bin_dir
        .file_name()
        .is_some_and(|name| name.to_string_lossy().eq_ignore_ascii_case("bin"))
    {
        return None;
    }

    let root = bin_dir.parent()?;
    let canonical_name = root
        .file_name()
        .is_some_and(|name| name.to_string_lossy().eq_ignore_ascii_case(SKILL_NAME));
    (canonical_name || root.join("SKILL.md").is_file()).then(|| root.to_path_buf())
}

/// Validate the required ZIP payload files in an installed Skill root.
///
/// Extra installed files are intentionally ignored because bootstrap adds
/// `bin/webfetch-cli[.exe]` after the ZIP is extracted.
pub fn validate_skill_root(root: &Path) -> SkillManifestStatus {
    let mut present_files = Vec::new();
    let mut missing_files = Vec::new();

    for relative in SKILL_ARCHIVE_FILES {
        if root.join(relative).is_file() {
            present_files.push(relative.to_owned());
        } else {
            missing_files.push(relative.to_owned());
        }
    }

    SkillManifestStatus {
        complete: missing_files.is_empty(),
        present_files,
        missing_files,
    }
}

fn env_path(name: &str) -> Option<PathBuf> {
    env::var_os(name)
        .and_then(non_empty_os_string)
        .map(PathBuf::from)
}

fn non_empty_os_string(value: OsString) -> Option<OsString> {
    (!value.is_empty()).then_some(value)
}

fn non_empty_path(path: Option<&PathBuf>) -> Option<&Path> {
    path.map(PathBuf::as_path)
        .filter(|path| !path.as_os_str().is_empty())
}

fn push_candidate(
    candidates: &mut Vec<SkillRootCandidate>,
    indices: &mut HashMap<PathBuf, usize>,
    root: PathBuf,
    source: SkillRootSource,
) {
    let key = deduplication_key(&root);
    if let Some(index) = indices.get(&key).copied() {
        let sources = &mut candidates[index].sources;
        if !sources.contains(&source) {
            sources.push(source);
        }
        return;
    }

    let index = candidates.len();
    let manifest = validate_skill_root(&root);
    candidates.push(SkillRootCandidate {
        root,
        sources: vec![source],
        manifest,
    });
    indices.insert(key, index);
}

fn deduplication_key(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| lexically_normalize(path))
}

fn lexically_normalize(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if !normalized.pop() && !path.is_absolute() {
                    normalized.push(component.as_os_str());
                }
            }
            _ => normalized.push(component.as_os_str()),
        }
    }
    normalized
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn candidate<'a>(candidates: &'a [SkillRootCandidate], root: &Path) -> &'a SkillRootCandidate {
        candidates
            .iter()
            .find(|candidate| candidate.root == root)
            .unwrap_or_else(|| panic!("missing candidate {}", root.display()))
    }

    fn write_manifest(root: &Path) {
        for relative in SKILL_ARCHIVE_FILES {
            let path = root.join(relative);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, relative).unwrap();
        }
    }

    #[test]
    fn discovers_codex_personal_and_compatibility_roots() {
        let home = PathBuf::from("/home/alice");
        let codex_home = PathBuf::from("/opt/codex-profile");
        let roots = discover_skill_roots(&DiscoveryInputs {
            home_dir: Some(home.clone()),
            codex_home: Some(codex_home.clone()),
            ..DiscoveryInputs::default()
        });

        let agents = home.join(".agents/skills").join(SKILL_NAME);
        assert_eq!(
            candidate(&roots, &agents).sources,
            vec![SkillRootSource::CodexAgentsHome]
        );
        assert_eq!(
            candidate(&roots, &codex_home.join("skills").join(SKILL_NAME)).sources,
            vec![SkillRootSource::CodexHome]
        );
        assert_eq!(
            candidate(&roots, &home.join(".codex/skills").join(SKILL_NAME)).sources,
            vec![SkillRootSource::CodexLegacyHome]
        );
        assert_eq!(
            candidate(&roots, &home.join(".claude/skills").join(SKILL_NAME)).sources,
            vec![SkillRootSource::ClaudeDefaultHome]
        );
    }

    #[test]
    fn discovers_claude_direct_and_config_roots() {
        let direct = PathBuf::from("/skills/lexmount-webfetch");
        let config = PathBuf::from("/profiles/claude");
        let roots = discover_skill_roots(&DiscoveryInputs {
            claude_skill_dir: Some(direct.clone()),
            claude_config_dir: Some(config.clone()),
            ..DiscoveryInputs::default()
        });

        assert_eq!(
            candidate(&roots, &direct).sources,
            vec![SkillRootSource::ClaudeSkillDir]
        );
        assert_eq!(
            candidate(&roots, &config.join("skills").join(SKILL_NAME)).sources,
            vec![SkillRootSource::ClaudeConfigDir]
        );
    }

    #[test]
    fn discovers_workbuddy_root() {
        let root = PathBuf::from("/workbuddy/skills/lexmount-webfetch");
        let roots = discover_skill_roots(&DiscoveryInputs {
            codebuddy_skill_dir: Some(root.clone()),
            ..DiscoveryInputs::default()
        });

        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0].root, root);
        assert_eq!(roots[0].sources, vec![SkillRootSource::WorkBuddySkillDir]);
    }

    #[test]
    fn discovers_skill_root_from_current_executable() {
        let root = PathBuf::from("/agent/skills/lexmount-webfetch");
        let executable = root.join("bin/webfetch-cli.exe");
        let roots = discover_skill_roots(&DiscoveryInputs {
            current_exe: Some(executable.clone()),
            ..DiscoveryInputs::default()
        });

        assert_eq!(skill_root_from_executable(&executable), Some(root.clone()));
        assert_eq!(roots.len(), 1);
        assert_eq!(roots[0].root, root);
        assert_eq!(roots[0].sources, vec![SkillRootSource::CurrentExecutable]);
        assert_eq!(
            skill_root_from_executable(Path::new("/tmp/debug/webfetch-cli")),
            None
        );
        assert_eq!(
            skill_root_from_executable(Path::new("/usr/local/bin/webfetch-cli")),
            None
        );

        let temp = tempdir().unwrap();
        let renamed_root = temp.path().join("renamed-skill");
        fs::create_dir_all(renamed_root.join("bin")).unwrap();
        fs::write(renamed_root.join("SKILL.md"), "---\nname: test\n---\n").unwrap();
        assert_eq!(
            skill_root_from_executable(&renamed_root.join("bin/webfetch-cli")),
            Some(renamed_root)
        );
    }

    #[test]
    fn deduplicates_roots_and_preserves_all_sources() {
        let home = PathBuf::from("/home/alice");
        let root = home.join(".agents/skills").join(SKILL_NAME);
        let roots = discover_skill_roots(&DiscoveryInputs {
            home_dir: Some(home),
            claude_skill_dir: Some(root.clone()),
            codebuddy_skill_dir: Some(root.clone()),
            current_exe: Some(root.join("bin/webfetch-cli")),
            ..DiscoveryInputs::default()
        });

        let matches = roots
            .iter()
            .filter(|candidate| candidate.root == root)
            .collect::<Vec<_>>();
        assert_eq!(matches.len(), 1);
        assert_eq!(
            matches[0].sources,
            vec![
                SkillRootSource::CodexAgentsHome,
                SkillRootSource::ClaudeSkillDir,
                SkillRootSource::WorkBuddySkillDir,
                SkillRootSource::CurrentExecutable,
            ]
        );
    }

    #[test]
    fn root_validation_reports_a_missing_manifest_file() {
        let temp = tempdir().unwrap();
        write_manifest(temp.path());
        fs::remove_file(temp.path().join("scripts/doctor.ps1")).unwrap();

        let status = validate_skill_root(temp.path());
        assert!(!status.complete);
        assert_eq!(status.missing_files, vec!["scripts/doctor.ps1"]);
        assert_eq!(status.present_files.len(), SKILL_ARCHIVE_FILES.len() - 1);
    }
}
