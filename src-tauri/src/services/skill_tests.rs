use super::*;
use tempfile::tempdir;

fn write_skill(dir: &Path, name: &str) {
    fs::create_dir_all(dir).expect("create skill dir");
    fs::write(
        dir.join("SKILL.md"),
        format!("---\nname: {name}\ndescription: Test skill\n---\n"),
    )
    .expect("write SKILL.md");
}

#[test]
fn resolve_skill_source_dir_returns_repo_root_for_root_level_skill() {
    let temp = tempdir().expect("tempdir");
    write_skill(temp.path(), "Root Skill");

    let resolved = SkillService::resolve_skill_source_dir(temp.path(), "last30days-skill-cn")
        .expect("root-level skill should resolve to the extracted repo root");

    assert_eq!(resolved, temp.path());
}

#[test]
fn resolve_skill_source_dir_returns_direct_nested_directory_when_present() {
    let temp = tempdir().expect("tempdir");
    let nested = temp.path().join("skills").join("nested-skill");
    write_skill(&nested, "Nested Skill");

    let resolved = SkillService::resolve_skill_source_dir(temp.path(), "skills/nested-skill")
        .expect("nested skill should resolve from its relative source path");

    assert_eq!(resolved, nested);
}

#[test]
fn resolve_skill_source_dir_falls_back_to_matching_install_name() {
    let temp = tempdir().expect("tempdir");
    let nested = temp.path().join("skills").join("nested-skill");
    write_skill(&nested, "Nested Skill");

    let resolved = SkillService::resolve_skill_source_dir(temp.path(), "nested-skill")
        .expect("install name should fall back to the matching discovered skill directory");

    assert_eq!(resolved, nested);
}

#[test]
fn replace_dest_with_copy_rejects_empty_source_without_touching_existing_dest() {
    let temp = tempdir().expect("tempdir");
    let source = temp.path().join("source-skill");
    let dest = temp.path().join("app-skills").join("source-skill");
    fs::create_dir_all(&source).expect("create empty source");
    write_skill(&dest, "Existing Skill");

    let err = SkillService::replace_dest_with_copy(&source, &dest, "source-skill")
        .expect_err("empty source should not replace existing app skill");

    assert!(
        err.to_string().contains("SKILL.md"),
        "unexpected error: {err:#}"
    );
    assert!(
        dest.join("SKILL.md").is_file(),
        "existing destination skill should be preserved"
    );
}
