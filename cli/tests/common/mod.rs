//! 統合テストの共通部品: PlantUML（Java + plantuml.jar）の有無。
//!
//! jar は `DDQ_PLANTUML_JAR` か、無ければ `cli/vendor/plantuml.jar`（git 管理外。cli/vendor/README.md）。
//! ddq 本体は exe の隣の plantuml.jar も探すが、テストの exe は target/ にあるので env で渡す。

use std::{path::PathBuf, process::Command};

pub fn plantuml_jar() -> Option<PathBuf> {
    if let Some(p) = std::env::var_os("DDQ_PLANTUML_JAR").filter(|v| !v.is_empty()) {
        return Some(PathBuf::from(p)).filter(|p| p.is_file());
    }
    let vendored = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("vendor/plantuml.jar");
    vendored.is_file().then_some(vendored)
}

pub fn java_available() -> bool {
    let java = std::env::var_os("DDQ_JAVA")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("JAVA_HOME").map(|h| PathBuf::from(h).join("bin/java")))
        .unwrap_or_else(|| PathBuf::from("java"));
    Command::new(java)
        .arg("-version")
        .output()
        .is_ok_and(|o| o.status.success())
}

pub fn plantuml_available() -> bool {
    plantuml_jar().is_some() && java_available()
}

/// ddq の起動に jar の場所を渡す
pub fn with_plantuml_jar(cmd: &mut Command) {
    if let Some(jar) = plantuml_jar() {
        cmd.env("DDQ_PLANTUML_JAR", jar);
    }
}
