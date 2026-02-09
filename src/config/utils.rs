//! Utility functions and traits for configuration

pub trait BundleContainer<B> {
    fn bundles(&self) -> &[B];

    fn name(bundle: &B) -> &str;

    fn find_bundle(&self, name: &str) -> Option<&B> {
        self.bundles().iter().find(|b| Self::name(b) == name)
    }
}

fn add_blank_lines_between_bundles(lines: Vec<&str>) -> Vec<String> {
    let mut formatted = Vec::new();
    let mut in_bundles_section = false;

    for line in lines {
        if line.trim_start().starts_with("bundles:") {
            in_bundles_section = true;
            formatted.push(line.to_string());
        } else if in_bundles_section && line.trim_start().starts_with("- name:") {
            if let Some(last) = formatted.last() {
                if !last.is_empty() && last.starts_with(' ') {
                    formatted.push(String::new());
                }
            }
            formatted.push(line.to_string());
        } else {
            formatted.push(line.to_string());
        }
    }
    formatted
}

/// Format YAML output with workspace name
pub fn format_yaml_with_workspace_name(yaml: &str, workspace_name: &str) -> String {
    let yaml = yaml.replace("name: ''", &format!("name: '{}'", workspace_name));

    let parts: Vec<&str> = yaml.splitn(2, '\n').collect();
    if parts.len() != 2 {
        return format!("{}\n", yaml);
    }

    let result = format!("{}\n\n{}", parts[0], parts[1]);
    let lines = result.lines().collect::<Vec<_>>();
    let formatted = add_blank_lines_between_bundles(lines);

    format!("{}\n", formatted.join("\n"))
}
