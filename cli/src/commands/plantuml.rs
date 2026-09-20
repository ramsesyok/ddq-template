//! `ddq plantuml serve` — ローカルの PlantUML サーバ（jar 内蔵の PicoWeb）を上げたままにする。
//!
//! 執筆者が VSCode の Quarto 拡張や素の `quarto preview` で ```plantuml を見るためのもの。
//! design-doc.lua は LAN のサーバ（_quarto.yml の plantuml-server）に届かなければ、
//! 既定の http://127.0.0.1:18080 を探すので、ポートを変えなければ設定は要らない。
//! `ddq pdf` / `ddq html` / `ddq diagrams` は内部で同じサーバを空きポートに上げるので、
//! 発行者がこれを起動しておく必要はない（cli/DESIGN.md §13）。

use anyhow::Result;

use crate::plantuml;

pub fn serve(port: u16, bind: &str) -> Result<()> {
    let java = plantuml::find_java().ok_or_else(|| {
        anyhow::anyhow!(
            "Java が見つかりません。JAVA_HOME を設定するか java を PATH に置くか、DDQ_JAVA に java.exe のパスを指定してください。"
        )
    })?;
    let jar = plantuml::find_jar().ok_or_else(|| {
        anyhow::anyhow!(
            "plantuml.jar が見つかりません。ddq.exe と同じフォルダに置くか、DDQ_PLANTUML_JAR / PLANTUML_JAR にパスを指定してください。"
        )
    })?;
    println!("java: {}", java.display());
    println!("jar : {}", jar.display());
    let mut server = plantuml::LocalServer::start(&java, &jar, port, bind)?;
    println!(
        "PlantUML サーバを起動しました: {}{}",
        server.url(),
        server
            .version()
            .map(|v| format!("（PlantUML {v}）"))
            .unwrap_or_default()
    );
    if port == plantuml::DEFAULT_PORT && bind == plantuml::LOCAL_BIND {
        println!("  quarto preview / VSCode の Quarto 拡張はこのまま図を描けます（設定不要）。");
    } else {
        println!(
            "  既定と違うので、環境変数 PLANTUML_SERVER={} を設定してから quarto preview を起動してください。",
            server.url()
        );
    }
    println!("  止めるには Ctrl-C。");
    server.wait()
}
