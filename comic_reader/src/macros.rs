#[macro_export]
macro_rules! comic_exts {
    () => {{
        let mut v = vec!["cbz", "zip"];
        #[cfg(feature = "rar")]
        {
            v.push("cbr");
            v.push("rar");
        }
        v
    }};
}

#[macro_export]
macro_rules! comic_filters {
    () => {{
        let dlg = rfd::FileDialog::new();
        let exts = $crate::comic_exts!();
        let dlg = dlg.add_filter("Comic Book Archive", &exts);
        #[cfg(feature = "rar")]
        let dlg = dlg
            .add_filter("Comic ZIP", &["cbz", "zip"])
            .add_filter("Comic RAR", &["cbr", "rar"]);
        dlg
    }};
}
