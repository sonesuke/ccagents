/// Resolve capture group references in a template string
pub fn resolve_capture_groups(template: &str, captured_groups: &[String]) -> String {
    let mut result = template.to_string();
    for (i, group) in captured_groups.iter().enumerate() {
        let placeholder = format!("${{{}}}", i + 1);
        result = result.replace(&placeholder, group);
    }
    result
}

/// Resolve capture groups in a vector of strings
pub fn resolve_capture_groups_in_vec(
    templates: &[String],
    captured_groups: &[String],
) -> Vec<String> {
    templates
        .iter()
        .map(|template| resolve_capture_groups(template, captured_groups))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_capture_groups_single() {
        let template = "echo ${1}";
        let groups = vec!["hello".to_string()];
        let result = resolve_capture_groups(template, &groups);
        assert_eq!(result, "echo hello");
    }

    #[test]
    fn test_resolve_capture_groups_multiple() {
        let template = "cp ${1} ${2}";
        let groups = vec!["file1.txt".to_string(), "/tmp/file2.txt".to_string()];
        let result = resolve_capture_groups(template, &groups);
        assert_eq!(result, "cp file1.txt /tmp/file2.txt");
    }

    #[test]
    fn test_resolve_capture_groups_no_placeholder() {
        let template = "echo hello";
        let groups = vec!["world".to_string()];
        let result = resolve_capture_groups(template, &groups);
        assert_eq!(result, "echo hello");
    }

    #[test]
    fn test_resolve_capture_groups_in_vec() {
        let templates = vec!["echo ${1}".to_string(), "cd ${2}".to_string()];
        let groups = vec!["hello".to_string(), "/tmp".to_string()];
        let result = resolve_capture_groups_in_vec(&templates, &groups);
        assert_eq!(result, vec!["echo hello", "cd /tmp"]);
    }
}