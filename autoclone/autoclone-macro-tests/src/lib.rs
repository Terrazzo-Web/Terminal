#[cfg(test)]
mod tests {
    use autoclone_macro::autoclone;

    #[test]
    #[autoclone]
    fn autoclone() {
        let string = "hello".to_owned();
        do_with_owned(move || {
            autoclone!(string);
            drop(string)
        });
        let _ = string.clone();
    }

    #[test]
    #[autoclone]
    fn autoclone2() {
        let string1 = "hello".to_owned();
        let string2 = "world".to_owned();
        do_with_owned(move || {
            autoclone!(string1);
            autoclone!(string2);
            drop(string1);
            drop(string2);
        });
        let _ = string1.clone();
        let _ = string2.clone();
    }

    #[test]
    #[autoclone]
    fn debug() {
        let string1 = "hello".to_owned();
        let string2 = "world".to_owned();
        do_with_owned(move || {
            autoclone!(string1);
            autoclone!(string2);
            drop(string1);
            drop(string2);
        });
        let _ = string1.clone();
        let _ = string2.clone();
    }

    #[test]
    #[autoclone]
    fn nested() {
        let string1 = "hello".to_owned();
        let string2 = "world".to_owned();
        do_with_owned(move || {
            autoclone!(string1);
            autoclone!(string2);
            do_with_owned(move || {
                autoclone!(string1);
                autoclone!(string2);
                drop(string1);
                drop(string2);
            });
            drop(string1);
            drop(string2);
        });
        let _ = string1.clone();
        let _ = string2.clone();
    }

    fn do_with_owned(callback: impl FnOnce()) {
        callback()
    }
}
