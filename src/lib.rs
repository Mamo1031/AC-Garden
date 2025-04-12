//! AC-Garden - AtCoderの提出をアーカイブするためのライブラリ

/// AtCoderの提出結果を表す構造体
pub mod submission {
    use serde::{Deserialize, Serialize};

    /// AtCoderの提出結果
    #[derive(Debug, Clone, Serialize, Deserialize)]
    pub struct Submission {
        pub id: i64,
        pub epoch_second: i64,
        pub problem_id: String,
        pub contest_id: String,
        pub user_id: String,
        pub language: String,
        pub point: f64,
        pub length: i64,
        pub result: String,
        pub execution_time: Option<i64>,
    }
}

/// 設定ファイルの管理
pub mod config {
    use serde::{Deserialize, Serialize};
    use std::path::PathBuf;

    /// サービス設定
    #[derive(Debug, Serialize, Deserialize)]
    pub struct Service {
        pub repository_path: String,
        pub user_id: String,
        pub user_email: String,
    }

    /// アプリケーション設定
    #[derive(Debug, Serialize, Deserialize)]
    pub struct Config {
        pub atcoder: Service,
    }

    /// 設定ファイルのパスを取得
    pub fn get_config_path() -> PathBuf {
        let home = home::home_dir().expect("Failed to get home directory");
        home.join(".ac-garden").join("config.json")
    }
}

/// ファイル操作のユーティリティ
pub mod utils {
    use std::path::Path;

    /// ディレクトリが存在するか確認
    pub fn is_dir_exist<P: AsRef<Path>>(path: P) -> bool {
        path.as_ref().is_dir()
    }

    /// ファイルが存在するか確認
    pub fn is_file_exist<P: AsRef<Path>>(path: P) -> bool {
        path.as_ref().is_file()
    }

    /// 言語に基づいて適切なファイル名を決定
    pub fn language_to_file_name(language: &str) -> String {
        // プログラミング言語名から拡張子を判断する
        // 言語名には時にバージョン情報などが括弧付きで含まれる
        let language = if let Some(idx) = language.find('(') {
            &language[..idx].trim()
        } else {
            language
        };

        match language {
            "C++" | "C++14" | "C++17" | "C++20" => "Main.cpp",
            "Bash" => "Main.sh",
            "C" => "Main.c",
            // 他の言語の対応も同様...
            _ => {
                eprintln!("Unknown language: {}", language);
                "Main.txt"
            }
        }.to_string()
    }
}
