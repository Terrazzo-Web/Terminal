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
        let conversion = r#"A  B"#.get_conversion(UNESCAPED).await;
        assert_eq!("Not found", conversion);
    }

    #[tokio::test]
    async fn invalid_escape() {
        let conversion = r#"\A  \B"#.get_conversion(UNESCAPED).await;
        assert_eq!("Not found", conversion);
    }

    #[tokio::test]
    async fn unescaped() {
        let conversion = r#"A\n\tB"#.get_conversion(UNESCAPED).await;
        assert_eq!("A\n\tB", conversion);
    }
}
