use std::fmt::Display;
use std::ops::Deref;
use std::ops::DerefMut;

pub struct Writer {
    result: String,
    indent: String,
    newline: bool,
}

impl Writer {
    pub fn new() -> Self {
        Self {
            result: String::new(),
            indent: "\n".into(),
            newline: false,
        }
    }

    #[allow(unused)]
    pub fn write(&mut self, txt: &str) -> &mut Self {
        if self.newline {
            self.newline = false;
            if !self.result.ends_with(&self.indent) {
                self.write("\n");
            }
        }
        self.result += &txt.replace('\n', &self.indent);
        self
    }

    #[allow(unused)]
    pub fn print(&mut self, txt: impl ToString) -> &mut Self {
        self.write(&txt.to_string())
    }

    #[allow(unused)]
    pub fn debug(&mut self, txt: impl std::fmt::Debug) -> &mut Self {
        self.write(&format!("{txt:#?}"))
    }

    #[allow(unused)]
    pub fn writeln(&mut self) -> &mut Self {
        self.newline = true;
        self
    }

    #[allow(unused)]
    #[must_use]
    pub fn indent(&mut self) -> Indented<'_> {
        Indented::new(self.writeln())
    }
}

impl Display for Writer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        Display::fmt(&self.result, f)
    }
}

pub struct Indented<'t>(&'t mut Writer);

static INDENTATION: &str = "  ";

impl<'t> Indented<'t> {
    pub fn new(writer: &'t mut Writer) -> Self {
        writer.indent += INDENTATION;
        Self(writer)
    }
}

impl<'t> Deref for Indented<'t> {
    type Target = Writer;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

impl<'t> DerefMut for Indented<'t> {
    fn deref_mut(&mut self) -> &mut Self::Target {
         self.0
    }
}

impl<'t> Drop for Indented<'t> {
    fn drop(&mut self) {
        self.0
            .indent
            .truncate(self.0.indent.len() - INDENTATION.len());
        self.writeln();
    }
}

#[cfg(test)]
#[test]
fn write() {
    let mut writer = Writer::new();
    writer.write("a").writeln().writeln();
    writer.write("b");
    let mut w2 = writer.indent();
    w2.write("b1").writeln();
    w2.write("b2").writeln();
    w2.indent().write("[]");
    drop(w2);
    writer.write("c").writeln();
    assert_eq!(
        r#"a
b
  b1
  b2
    []
c"#,
        writer.to_string()
    )
}
