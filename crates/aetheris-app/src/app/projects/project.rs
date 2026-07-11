use super::*;

impl Project {
    pub(crate) fn custom_namespaces_for_context(&self, context: Option<&str>) -> Vec<String> {
        let Some(context) = context else {
            return Vec::new();
        };
        self.custom_namespaces_by_context
            .iter()
            .find(|entry| entry.context == context)
            .map(|entry| entry.namespaces.clone())
            .unwrap_or_default()
    }

    pub(crate) fn has_custom_namespace(&self, context: Option<&str>, namespace: &str) -> bool {
        self.custom_namespaces_for_context(context)
            .iter()
            .any(|known| known == namespace)
    }

    pub(crate) fn add_custom_namespace(&mut self, context: &str, namespace: &str) -> bool {
        if context.is_empty() || namespace.is_empty() {
            return false;
        }

        if let Some(entry) = self
            .custom_namespaces_by_context
            .iter_mut()
            .find(|entry| entry.context == context)
        {
            if entry.namespaces.iter().any(|known| known == namespace) {
                return false;
            }
            entry.namespaces.push(namespace.to_owned());
            entry.namespaces.sort();
            entry.namespaces.dedup();
            return true;
        }

        self.custom_namespaces_by_context.push(ContextNamespaces {
            context: context.to_owned(),
            namespaces: vec![namespace.to_owned()],
        });
        self.normalize_custom_namespaces();
        true
    }

    pub(crate) fn remove_custom_namespace(&mut self, context: &str, namespace: &str) -> bool {
        let Some(entry) = self
            .custom_namespaces_by_context
            .iter_mut()
            .find(|entry| entry.context == context)
        else {
            return false;
        };

        let before = entry.namespaces.len();
        entry.namespaces.retain(|known| known != namespace);
        let removed = entry.namespaces.len() != before;

        self.custom_namespaces_by_context
            .retain(|entry| !entry.namespaces.is_empty());

        removed
    }

    pub(crate) fn rename_custom_namespace(&mut self, context: &str, old: &str, new: &str) -> bool {
        let new = new.trim();
        if new.is_empty() || new == old {
            return false;
        }

        let Some(entry) = self
            .custom_namespaces_by_context
            .iter_mut()
            .find(|entry| entry.context == context)
        else {
            return false;
        };

        if entry.namespaces.iter().any(|known| known == new) {
            return false;
        }

        let Some(slot) = entry.namespaces.iter_mut().find(|known| *known == old) else {
            return false;
        };
        *slot = new.to_owned();
        entry.namespaces.sort();
        entry.namespaces.dedup();

        true
    }

    pub(super) fn normalize_custom_namespaces(&mut self) {
        for entry in &mut self.custom_namespaces_by_context {
            entry.context = entry.context.trim().to_owned();
            entry.namespaces.retain(|namespace| {
                let namespace = namespace.trim();
                !namespace.is_empty() && namespace != "all"
            });
            for namespace in &mut entry.namespaces {
                *namespace = namespace.trim().to_owned();
            }
            entry.namespaces.sort();
            entry.namespaces.dedup();
        }
        self.custom_namespaces_by_context
            .retain(|entry| !entry.context.is_empty() && !entry.namespaces.is_empty());
        self.custom_namespaces_by_context
            .sort_by(|left, right| left.context.cmp(&right.context));
        self.custom_namespaces_by_context.dedup_by(|left, right| {
            if left.context == right.context {
                left.namespaces.extend(right.namespaces.clone());
                left.namespaces.sort();
                left.namespaces.dedup();
                true
            } else {
                false
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn custom_namespaces_are_scoped_by_context() {
        let mut project = Project {
            name: String::from("Work"),
            contexts: vec![String::from("prod"), String::from("stage")],
            custom_namespaces_by_context: Vec::new(),
        };

        assert!(project.add_custom_namespace("prod", "billing"));

        assert_eq!(
            project.custom_namespaces_for_context(Some("prod")),
            vec![String::from("billing")]
        );
        assert!(
            project
                .custom_namespaces_for_context(Some("stage"))
                .is_empty()
        );
        assert!(!project.has_custom_namespace(Some("stage"), "billing"));
    }

    #[test]
    fn remove_custom_namespace_drops_empty_context_entry() {
        let mut project = Project {
            name: String::from("Work"),
            contexts: vec![String::from("prod")],
            custom_namespaces_by_context: Vec::new(),
        };
        project.add_custom_namespace("prod", "billing");

        assert!(project.remove_custom_namespace("prod", "billing"));
        assert!(
            project
                .custom_namespaces_for_context(Some("prod"))
                .is_empty()
        );
        assert!(project.custom_namespaces_by_context.is_empty());
    }

    #[test]
    fn remove_custom_namespace_returns_false_when_not_found() {
        let mut project = Project {
            name: String::from("Work"),
            contexts: vec![String::from("prod")],
            custom_namespaces_by_context: Vec::new(),
        };
        project.add_custom_namespace("prod", "billing");

        assert!(!project.remove_custom_namespace("prod", "not-there"));
        assert!(!project.remove_custom_namespace("other-context", "billing"));
        assert_eq!(
            project.custom_namespaces_for_context(Some("prod")),
            vec![String::from("billing")]
        );
    }

    #[test]
    fn rename_custom_namespace_replaces_entry_in_place() {
        let mut project = Project {
            name: String::from("Work"),
            contexts: vec![String::from("prod")],
            custom_namespaces_by_context: Vec::new(),
        };
        project.add_custom_namespace("prod", "billing");

        assert!(project.rename_custom_namespace("prod", "billing", "payments"));
        assert_eq!(
            project.custom_namespaces_for_context(Some("prod")),
            vec![String::from("payments")]
        );
        assert!(!project.has_custom_namespace(Some("prod"), "billing"));
    }

    #[test]
    fn rename_custom_namespace_no_ops_when_target_name_already_exists() {
        let mut project = Project {
            name: String::from("Work"),
            contexts: vec![String::from("prod")],
            custom_namespaces_by_context: Vec::new(),
        };
        project.add_custom_namespace("prod", "billing");
        project.add_custom_namespace("prod", "payments");

        assert!(!project.rename_custom_namespace("prod", "billing", "payments"));
        assert_eq!(
            project.custom_namespaces_for_context(Some("prod")),
            vec![String::from("billing"), String::from("payments")]
        );
    }

    #[test]
    fn rename_custom_namespace_returns_false_when_source_missing() {
        let mut project = Project {
            name: String::from("Work"),
            contexts: vec![String::from("prod")],
            custom_namespaces_by_context: Vec::new(),
        };

        assert!(!project.rename_custom_namespace("prod", "billing", "payments"));
    }
}
