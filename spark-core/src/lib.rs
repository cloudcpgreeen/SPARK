//! SPARK 领域核心：无 HTTP 纯库。领域逻辑随 idea 落地放这里（契约优先，见 CONTRACT.md）。
//! 当前只是骨架：契约版本与 `wit/core.wit` 的 package 版本对齐（见 DEVELOPMENT.md §5）。

pub const NAME: &str = "SPARK";

/// 契约版本，必须与 `wit/core.wit` 的 `spark:core@<version>` 一致。
pub fn contract_version() -> &'static str {
    "0.1.0"
}

#[cfg(test)]
mod tests {
    #[test]
    fn contract_version_is_semver() {
        let v = super::contract_version();
        let parts: Vec<&str> = v.split('.').collect();
        assert_eq!(parts.len(), 3, "contract version must be semver, got {v}");
        assert!(
            parts
                .iter()
                .all(|p| !p.is_empty() && p.parse::<u64>().is_ok()),
            "contract version parts must be numeric, got {v}"
        );
    }
}
