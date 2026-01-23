/// Apply a transform rule to get the target path for a resource
fn apply_transform_rule(&self, rule: &TransformRule, resource_path: &Path) -> PathBuf {
    let path_str = resource_path.to_string_lossy();

    // Build target path by substituting variables
    let mut target = rule.to.clone();

    // Handle {name} placeholder - extract filename without extension
    if target.contains("{name}") {
        if let Some(stem) = resource_path.file_stem() {
            target = target.replace("{name}", &stem.to_string_lossy());
        }
    }

    // Handle ** wildcard - preserve subdirectory structure
    // Must be done BEFORE extension transformation
    #[allow(clippy::needless_borrow)]
    if rule.from.contains("**") && rule.to.contains("**") {
        let prefix_len = rule.from.find("**").unwrap_or(0);
        let path_prefix = if prefix_len > 0 {
            &path_str[..prefix_len.min(path_str.len())]
        } else {
            ""
        };

        let relative_part = path_str.strip_prefix(path_prefix).unwrap_or(&path_str);
        let trimmed_part = relative_part.trim_start_matches('/');

        // Replace **/ with the relative part, being careful to handle slash correctly
        if target.contains("/**/") {
            // Pattern like ".cursor/rules/**/*.mdc" - has slashes around **
            target = target.replace("/**/", &format!("/{}/", trimmed_part));
        } else if target.contains("**") {
            // Pattern like ".cursor/rules/**" - no trailing slash
            target = target.replace("**", &trimmed_part);
        }
    }

    // Handle * wildcard (single file) - must be done BEFORE extension transformation
    if target.contains('*') && !target.contains("**") {
        if let Some(stem) = resource_path.file_stem() {
            target = target.replace('*', &stem.to_string_lossy());
        }
    }

    // Apply extension transformation after all wildcards are replaced
    if let Some(ref ext) = rule.extension {
        let without_ext = if let Some(pos) = target.rfind('.') {
            &target[..pos]
        } else {
            &target
        };
        target = format!("{}.{}", without_ext, ext);
    }

    self.workspace_root.join(&target)
}
