use crate::converter::api::Language;

pub fn add_unescape(input: &str, add: &mut impl super::AddConversionFn) -> bool {
    if !input.contains('\\') {
        return false;
    }
    let Ok(unescaped) = unescaper::unescape(input) else {
        return false;
    };
    add(Language::new("Unescaped"), unescaped);
    return true;
}

#[cfg(test)]
mod tests {
    use super::super::tests::GetConversionForTest as _;

    static UNESCAPED: &str = "Unescaped";

    #[tokio::test]
    async fn nothing_to_unescape() {
        let input = r#"A  B"#;
        let conversion = input.get_conversion(UNESCAPED).await;
        assert_eq!("Not found", conversion);
        assert_eq!(vec!["JSON", "YAML"], input.get_languages().await);
    }

    #[tokio::test]
    async fn invalid_escape() {
        let input = r#"\A  \B"#;
        let conversion = input.get_conversion(UNESCAPED).await;
        assert_eq!("Not found", conversion);
        assert_eq!(vec!["JSON", "YAML"], input.get_languages().await);
    }

    #[tokio::test]
    async fn unescaped() {
        let input = r#"A\n\tB"#;
        let conversion = input.get_conversion(UNESCAPED).await;
        assert_eq!("A\n\tB", conversion);
        assert_eq!(vec!["JSON", UNESCAPED, "YAML"], input.get_languages().await);
    }
}
