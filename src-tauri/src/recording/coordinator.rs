pub fn make_filenames(timestamp: &str) -> (String, String) {
    (format!("REC-{timestamp}.mp4"), format!("REC-{timestamp}.metadata.json"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filenames_share_timestamp() {
        let (v, m) = make_filenames("20260618-153000");
        assert_eq!(v, "REC-20260618-153000.mp4");
        assert_eq!(m, "REC-20260618-153000.metadata.json");
    }
}
