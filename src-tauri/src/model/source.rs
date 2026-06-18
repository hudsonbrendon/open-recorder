#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceKind {
    Display,
    Window,
    Region,
}

impl SourceKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            SourceKind::Display => "display",
            SourceKind::Window => "window",
            SourceKind::Region => "region",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CaptureSource {
    pub kind: SourceKind,
    pub id: String,
    pub rect: [i64; 4],
}
