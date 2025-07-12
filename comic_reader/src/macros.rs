#[macro_export]
macro_rules! comic_exts {
    () => {{
        let mut v = vec!["cbz", "zip"];
        #[cfg(feature = "rar")]
        {
            v.push("cbr");
            v.push("rar");
        }
        #[cfg(feature = "7z")]
        {
            v.push("cb7");
            v.push("7z");
        }
        v
    }};
}

#[macro_export]
macro_rules! comic_filters {
    () => {{
        let mut dlg = rfd::FileDialog::new();
        let exts = $crate::comic_exts!();
        dlg = dlg.add_filter("Comic Book Archive", &exts);
        dlg = dlg.add_filter("Comic CBZ", &["cbz", "zip"]);
        #[cfg(feature = "rar")]
        {
            dlg = dlg.add_filter("Comic RAR", &["cbr", "rar"]);
        }
        #[cfg(feature = "7z")]
        {
            dlg = dlg.add_filter("Comic 7z", &["cb7", "7z"]);
        }
        dlg
    }};
}
